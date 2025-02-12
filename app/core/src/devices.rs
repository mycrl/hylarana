use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};

use anyhow::{anyhow, Result};
use common::MediaStreamDescription;
use crossbeam::channel::{unbounded, Receiver, Sender};
use futures_util::{SinkExt, StreamExt};
use hylarana::{DiscoveryObserver, DiscoveryService};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::{
    net::TcpListener,
    sync::mpsc::{unbounded_channel, UnboundedSender},
};

use tokio_tungstenite::{
    accept_async, connect_async,
    tungstenite::{client::IntoClientRequest, http::StatusCode, Message},
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
    port: u16,
}

impl Device {
    fn new<T>(
        name: String,
        kind: DeviceType,
        addrs: Vec<Ipv4Addr>,
        port: u16,
        observer: T,
    ) -> Result<Self>
    where
        T: FnOnce(&str) + Send + 'static,
    {
        let (mut socket, response) = RUNTIME.block_on(connect_async(
            format!("ws://{}:{}", addrs[0], port).into_client_request()?,
        ))?;

        if response.status() != StatusCode::SWITCHING_PROTOCOLS {
            return Err(anyhow!(
                "websocket connect status code={}",
                response.status()
            ));
        }

        log::info!(
            "connection to remote device success, name={}, url = ws://{}:{}",
            name,
            addrs[0],
            port
        );

        let (tx, mut rx) = unbounded_channel::<String>();
        RUNTIME.spawn(async move {
            'a: loop {
                tokio::select! {
                    Some(message) = rx.recv() => {
                        if socket.send(Message::text(message)).await.is_err() {
                            break 'a;
                        }
                    },
                    Some(_) = socket.next() => (),
                    else => {
                        break;
                    }
                };
            }

            observer(&name);

            log::warn!("remote device disconnection, name={}", name);
        });

        Ok(Self {
            description: Default::default(),
            addrs,
            port,
            kind,
            tx,
        })
    }

    fn get_port(&self) -> u16 {
        self.port
    }

    fn get_kind(&self) -> DeviceType {
        self.kind
    }

    fn get_addrs(&self) -> Vec<Ipv4Addr> {
        self.addrs.clone()
    }

    fn get_description(&self) -> Option<MediaStreamDescription> {
        self.description.read().clone()
    }

    fn update_description(&self, description: MediaStreamDescription) {
        self.description.write().replace(description);
    }

    fn send_description(&mut self, description: &MediaStreamDescription) -> Result<()> {
        log::info!(
            "send description to remote device, description={:?}",
            description
        );

        self.tx.send(serde_json::to_string(description)?)?;

        Ok(())
    }
}

#[derive(Default)]
struct Devices {
    /// addr name mapping
    anm: RwLock<HashMap<Ipv4Addr, String>>,
    table: RwLock<HashMap<String, Device>>,
}

impl Devices {
    fn set(&self, name: &str, device: Device) {
        let mut anm = self.anm.write();
        for it in &device.addrs {
            anm.insert(*it, name.to_string());
        }

        self.table.write().insert(name.to_string(), device);

        log::info!("add a new device for devices, name={}", name);
    }

    fn remove(&self, name: &str) {
        if let Some(it) = self.table.write().remove(name) {
            let mut anm = self.anm.write();
            for it in it.addrs {
                anm.remove(&it);
            }
        }

        log::info!("remove a device for devices, name={}", name);
    }

    fn remove_from_addr(&self, addr: Ipv4Addr) {
        if let Some(it) = self.anm.write().remove(&addr) {
            self.remove(&it);
        }
    }

    fn update_description_from_addr(&self, addr: Ipv4Addr, description: MediaStreamDescription) {
        if let Some(it) = self.anm.read().get(&addr) {
            if let Some(device) = self.table.read().get(it) {
                device.update_description(description);
            }
        }
    }
}

struct DiscoveryServiceObserver {
    description: Arc<RwLock<Option<MediaStreamDescription>>>,
    devices: Arc<Devices>,
    tx: Sender<()>,
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

        let tx = self.tx.clone();
        let devices = self.devices.clone();
        match Device::new(name.to_string(), info.kind, addrs, info.port, move |name| {
            devices.remove(name);

            if let Err(e) = tx.send(()) {
                log::error!("devices send change notify error={:?}", e);
            }
        }) {
            Ok(mut device) => {
                if let Some(description) = self.description.read().as_ref() {
                    if let Err(e) = device.send_description(description) {
                        log::error!("failed to send description to remote device, error={}", e);
                    } else {
                        self.devices.set(&name, device);

                        if let Err(e) = self.tx.send(()) {
                            log::error!("devices send change notify error={:?}", e);
                        }
                    }
                }
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
        let devices: Arc<Devices> = Arc::new(Devices::default());

        let listener = RUNTIME.block_on(TcpListener::bind("0.0.0.0:0"))?;
        let local_addr = listener.local_addr()?;

        log::info!("devices manager server listener addr={}", local_addr);

        let (tx, rx) = unbounded::<()>();

        let devices_ = Arc::downgrade(&devices);
        RUNTIME.spawn(async move {
            while let Ok((socket, addr)) = listener.accept().await {
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
                                    if let Some(devices) = devices_.upgrade() {
                                        if let Ok(it) = serde_json::from_str(text.as_str()) {
                                            devices.update_description_from_addr(ip, it);
                                        }
                                    } else {
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
                        devices.remove_from_addr(ip);
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
                tx,
            })?,
        );

        Ok(Self {
            rx,
            devices,
            discoverys,
            description,
        })
    }

    pub fn set_description(
        &self,
        name_list: Vec<String>,
        description: MediaStreamDescription,
    ) -> Result<()> {
        let mut devices = self.devices.table.write();

        if name_list.is_empty() {
            self.description.write().replace(description.clone());
        } else {
            let _ = self.description.write().take();
        }

        if name_list.is_empty() {
            for (_, device) in devices.iter_mut() {
                device.send_description(&description)?;
            }
        } else {
            for name in name_list {
                if let Some(device) = devices.get_mut(&name) {
                    device.send_description(&description)?;
                }
            }
        }

        Ok(())
    }

    pub fn get_devices(&self) -> Vec<DeviceInfo> {
        let mut devices = Vec::with_capacity(100);

        for (k, v) in self.devices.table.read().iter() {
            devices.push(DeviceInfo {
                description: v.get_description(),
                addrs: v.get_addrs(),
                port: v.get_port(),
                kind: v.get_kind(),
                name: k.clone(),
            });
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
