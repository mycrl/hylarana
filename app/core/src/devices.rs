use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Result};
use common::MediaStreamDescription;
use crossbeam::channel::{unbounded, Receiver, Sender};
use futures_util::{SinkExt, StreamExt};
use hylarana::{DiscoveryObserver, DiscoveryService};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tokio::{
    net::TcpListener,
    sync::mpsc::{unbounded_channel, UnboundedSender},
    time::{sleep, timeout},
};

use tokio_tungstenite::{
    accept_async, connect_async,
    tungstenite::{client::IntoClientRequest, http::StatusCode, Bytes, Message},
};

use crate::RUNTIME;

#[cfg(target_os = "windows")]
pub static DEVICE_TYPE: DeviceType = DeviceType::Windows;

#[cfg(target_os = "macos")]
pub static DEVICE_TYPE: DeviceType = DeviceType::Apple;

#[derive(Debug, Deserialize, Serialize)]
struct ServiceInfo {
    port: u16,
    kind: DeviceType,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum DeviceType {
    Windows,
    Android,
    Apple,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceInfo {
    pub description: Option<MediaStreamDescription>,
    pub addrs: Vec<Ipv4Addr>,
    pub kind: DeviceType,
    pub name: String,
    pub port: u16,
}

struct Device {
    tx: UnboundedSender<String>,
    description: Arc<RwLock<Option<MediaStreamDescription>>>,
    addrs: Vec<Ipv4Addr>,
    kind: DeviceType,
    name: String,
    port: u16,
}

impl Device {
    fn new<T>(
        name: &str,
        kind: DeviceType,
        addrs: Vec<Ipv4Addr>,
        port: u16,
        observer: T,
    ) -> Result<Self>
    where
        T: FnOnce(String) + Send + 'static,
    {
        let name = name.to_string();
        let url = format!("ws://{}:{}", addrs[0], port);
        log::info!(
            "connectioning to remote device, name={}, url = {}",
            name,
            url
        );

        let (mut socket, response) = RUNTIME.block_on(async move {
            Ok::<_, anyhow::Error>(
                timeout(
                    Duration::from_secs(5),
                    connect_async(url.into_client_request()?),
                )
                .await??,
            )
        })?;

        if response.status() != StatusCode::SWITCHING_PROTOCOLS {
            return Err(anyhow!(
                "websocket connect status code={}",
                response.status()
            ));
        }

        log::info!("connection to remote device success, name={}", name);

        let name_ = name.clone();
        let (tx, mut rx) = unbounded_channel::<String>();
        RUNTIME.spawn(async move {
            let timeout = sleep(Duration::from_secs(2));
            tokio::pin!(timeout);

            'a: loop {
                tokio::select! {
                    Some(message) = rx.recv() => {
                        if socket.send(Message::text(message)).await.is_err() {
                            break 'a;
                        }
                    },
                    Some(_) = socket.next() => (),
                    _ = &mut timeout =>  {
                        if socket.send(Message::Ping(Bytes::new())).await.is_err() {
                            break 'a;
                        }
                    },
                    else => {
                        break;
                    }
                };
            }

            log::warn!("remote device disconnection, name={}", name_);

            observer(name_);
        });

        Ok(Self {
            description: Default::default(),
            addrs,
            port,
            kind,
            name,
            tx,
        })
    }

    fn get_info(&self) -> DeviceInfo {
        DeviceInfo {
            description: self.description.read().clone(),
            addrs: self.addrs.clone(),
            name: self.name.clone(),
            port: self.port,
            kind: self.kind,
        }
    }

    fn update_description(&self, description: Option<MediaStreamDescription>) {
        log::info!("update device description from remote, name={}", self.name);

        *self.description.write() = description;
    }

    fn send_description(&mut self, description: Option<&MediaStreamDescription>) -> Result<()> {
        log::info!("send device description to remote, name={:?}", self.name);

        self.tx.send(serde_json::to_string(&description)?)?;

        Ok(())
    }
}

enum DevicesRemoveParams<'a> {
    Name(String),
    IpAddr(Ipv4Addr),
    Names(&'a [String]),
}

struct Devices {
    notify: Sender<()>,
    /// addr name mapping
    anm: RwLock<HashMap<Ipv4Addr, String>>,
    table: RwLock<HashMap<String, Device>>,
}

impl Devices {
    fn new(notify: Sender<()>) -> Self {
        Self {
            anm: Default::default(),
            table: Default::default(),
            notify,
        }
    }

    fn add(&self, name: &str, device: Device) {
        let mut anm = self.anm.write();
        for it in &device.addrs {
            anm.insert(*it, name.to_string());
        }

        self.table.write().insert(name.to_string(), device);

        log::info!("add a new device for devices, name={}", name);

        if let Err(e) = self.notify.send(()) {
            log::error!("devices send change notify error={:?}", e);
        }
    }

    fn remove(&self, params: DevicesRemoveParams) {
        let mut table = self.table.write();
        let mut anm = self.anm.write();

        let mut items: SmallVec<[String; 5]> = SmallVec::with_capacity(5);
        match params {
            DevicesRemoveParams::Name(it) => items.push(it),
            DevicesRemoveParams::Names(list) => {
                for it in list {
                    items.push(it.clone());
                }
            }
            DevicesRemoveParams::IpAddr(ip) => {
                if let Some(it) = anm.get(&ip) {
                    items.push(it.clone());
                }
            }
        }

        for it in items {
            if let Some(device) = table.remove(&it) {
                for ip in device.addrs {
                    anm.remove(&ip);
                }

                log::info!("remove a device for devices, name={}", device.name);
            }
        }

        if let Err(e) = self.notify.send(()) {
            log::error!("devices send change notify error={:?}", e);
        }
    }

    fn update_description(&self, addr: Ipv4Addr, description: Option<MediaStreamDescription>) {
        if let Some(it) = self.anm.read().get(&addr) {
            if let Some(device) = self.table.read().get(it) {
                device.update_description(description);
            }
        }

        log::info!("update remote description for address, ip={}", addr);

        if let Err(e) = self.notify.send(()) {
            log::error!("devices send change notify error={:?}", e);
        }
    }
}

struct DiscoveryServiceObserver {
    description: Arc<RwLock<Option<MediaStreamDescription>>>,
    devices: Arc<Devices>,
    name: String,
}

impl DiscoveryObserver<ServiceInfo> for DiscoveryServiceObserver {
    fn resolve(&self, name: &str, addrs: Vec<Ipv4Addr>, info: ServiceInfo) {
        if name == &self.name {
            log::warn!(
                "discovery service resolve myself, ignore this, name={}",
                name
            );

            return;
        }

        log::info!(
            "discovery service resolve, name={}, addrs={:?}, info={:?}",
            name,
            addrs,
            info
        );

        let devices = self.devices.clone();
        match Device::new(name, info.kind, addrs, info.port, move |name| {
            devices.remove(DevicesRemoveParams::Name(name));

            log::info!("device is drop, clean device table and send notify events");
        }) {
            Ok(mut device) => {
                log::info!("new device connected, name={}", name);

                if let Some(description) = self.description.read().as_ref() {
                    if let Err(e) = device.send_description(Some(description)) {
                        log::error!("failed to send description to remote device, error={}", e);

                        return;
                    }

                    log::info!("broadcast mode has been enabled, sending the current sender description to the remote device");
                } else {
                    log::info!("the broadcast mode is not enabled and the device is treated as a normal receiving device");
                }

                self.devices.add(&name, device);
            }
            Err(e) => {
                log::error!("failed to create device, error={}", e);
            }
        }
    }
}

pub struct DevicesManager {
    rx: Receiver<()>,
    devices: Arc<Devices>,
    #[allow(dead_code)]
    discoverys: (DiscoveryService, DiscoveryService),
    description: Arc<RwLock<Option<MediaStreamDescription>>>,
}

impl DevicesManager {
    pub fn new(name: String) -> Result<Self> {
        let (tx, rx) = unbounded::<()>();
        let devices: Arc<Devices> = Arc::new(Devices::new(tx));

        let listener = RUNTIME.block_on(TcpListener::bind("0.0.0.0:0"))?;
        let local_addr = listener.local_addr()?;

        log::info!("devices manager server listener addr={}", local_addr);

        let devices_ = Arc::downgrade(&devices);
        RUNTIME.spawn(async move {
            while let Ok((socket, addr)) = listener.accept().await {
                log::info!("accept a new tcp socket, address={}", addr);

                let devices_ = devices_.clone();
                let ip = match addr.ip() {
                    IpAddr::V4(it) => it,
                    _ => unimplemented!(),
                };

                RUNTIME.spawn(async move {
                    match accept_async(socket).await {
                        Ok(mut stream) => {
                            while let Some(Ok(message)) = stream.next().await {
                                if let Message::Text(text) = message {
                                    log::info!("recv a new text message, address={}, content={}", addr, text);

                                    if let Some(devices) = devices_.upgrade() {
                                        if let Ok(it) = serde_json::from_str(text.as_str()) {
                                            devices.update_description(ip, it);
                                        }
                                    } else {
                                        log::error!("device ref is droped! close the recv thread, address={}", addr);

                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("websocket server upgrade error={}", e);
                        }
                    }

                    if let Some(devices) = devices_.upgrade() {
                        devices.remove(DevicesRemoveParams::IpAddr(ip));
                    }
                });
            }
        });

        let description: Arc<RwLock<Option<MediaStreamDescription>>> = Default::default();
        let discoverys = (
            DiscoveryService::register(
                &name,
                &ServiceInfo {
                    port: local_addr.port(),
                    kind: DEVICE_TYPE,
                },
            )?,
            DiscoveryService::query(DiscoveryServiceObserver {
                description: description.clone(),
                devices: devices.clone(),
                name,
            })?,
        );

        Ok(Self {
            rx,
            devices,
            discoverys,
            description,
        })
    }

    pub fn set_description(&self, name_list: Vec<String>, description: MediaStreamDescription) {
        log::info!("set description, names={:?}", name_list);

        let mut devices = self.devices.table.write();

        if name_list.is_empty() {
            log::info!("name list is empty, store description");

            self.description.write().replace(description.clone());
        } else {
            let _ = self.description.write().take();
        }

        let mut closeds: SmallVec<[String; 5]> = SmallVec::with_capacity(5);

        if name_list.is_empty() {
            for (name, device) in devices.iter_mut() {
                if device.send_description(Some(&description)).is_err() {
                    closeds.push(name.clone());
                }
            }
        } else {
            for name in name_list {
                if let Some(device) = devices.get_mut(&name) {
                    if device.send_description(Some(&description)).is_err() {
                        closeds.push(name.clone());
                    }
                }
            }
        }

        if !closeds.is_empty() {
            self.devices.remove(DevicesRemoveParams::Names(&closeds));
        }
    }

    pub fn remove_description(&self) {
        let _ = self.description.write().take();

        let mut closeds: SmallVec<[String; 5]> = SmallVec::with_capacity(5);

        let mut devices = self.devices.table.write();
        for (name, device) in devices.iter_mut() {
            if device.send_description(None).is_err() {
                closeds.push(name.clone());
            }
        }

        if !closeds.is_empty() {
            self.devices.remove(DevicesRemoveParams::Names(&closeds));
        }
    }

    pub fn get_devices(&self) -> Vec<DeviceInfo> {
        let mut devices = Vec::with_capacity(100);

        for it in self.devices.table.read().values() {
            devices.push(it.get_info());
        }

        devices
    }

    pub fn get_watcher(&self) -> DevicesWatcher {
        DevicesWatcher(self.rx.clone())
    }
}

pub struct DevicesWatcher(Receiver<()>);

impl DevicesWatcher {
    pub fn change(&mut self) -> bool {
        self.0.recv().is_ok()
    }
}
