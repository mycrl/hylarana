use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Result};
use common::MediaStreamDescription;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use hylarana::{DiscoveryObserver, DiscoveryService};
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    runtime::Handle,
    sync::{oneshot, watch, RwLock},
    time::timeout,
};

use tokio_tungstenite::{
    accept_async, connect_async,
    tungstenite::{client::IntoClientRequest, http::StatusCode, Message},
    MaybeTlsStream, WebSocketStream,
};

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

pub struct Device {
    _hook: Arc<()>,
    sender: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    description: Arc<RwLock<Option<MediaStreamDescription>>>,
    addrs: Vec<Ipv4Addr>,
    kind: DeviceType,
    port: u16,
}

impl Device {
    async fn new(
        name: String,
        kind: DeviceType,
        addrs: Vec<Ipv4Addr>,
        port: u16,
    ) -> Result<(Self, oneshot::Receiver<()>)> {
        let (socket, response) =
            connect_async(format!("ws://{}:{}", addrs[0], port).into_client_request()?).await?;

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

        let _hook: Arc<()> = Default::default();
        let hook_ = Arc::downgrade(&_hook);

        let (tx, rx) = oneshot::channel();
        let (sender, mut receiver) = socket.split();
        tokio::spawn(async move {
            loop {
                if hook_.upgrade().is_none() {
                    break;
                } else {
                    if let Ok(it) = timeout(Duration::from_secs(1), receiver.next()).await {
                        match it {
                            None | Some(Err(_)) => break,
                            _ => (),
                        }
                    }
                }
            }

            let _ = tx.send(());

            log::warn!("remote device disconnection, name={}", name);
        });

        Ok((
            Self {
                _hook,
                description: Default::default(),
                sender,
                addrs,
                port,
                kind,
            },
            rx,
        ))
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn get_kind(&self) -> DeviceType {
        self.kind
    }

    pub fn get_addrs(&self) -> Vec<Ipv4Addr> {
        self.addrs.clone()
    }

    pub async fn get_description(&self) -> Option<MediaStreamDescription> {
        self.description.read().await.clone()
    }

    async fn send_description(&mut self, description: &MediaStreamDescription) {
        log::info!(
            "send description to remote device, description={:?}",
            description
        );

        if let Err(e) = self
            .sender
            .send(Message::text(serde_json::to_string(description).unwrap()))
            .await
        {
            log::error!("failed to send description, err={}", e);
        }
    }

    async fn update_description(&self, description: MediaStreamDescription) {
        self.description.write().await.replace(description);
    }
}

#[derive(Default)]
struct Devices {
    table: RwLock<HashMap<String, Device>>,
    /// addr name mapping
    anm: RwLock<HashMap<Ipv4Addr, String>>,
}

impl Devices {
    async fn set(&self, name: &str, device: Device) {
        let mut anm = self.anm.write().await;
        for it in &device.addrs {
            anm.insert(*it, name.to_string());
        }

        self.table.write().await.insert(name.to_string(), device);

        log::info!("add a new device for devices, name={}", name);
    }

    async fn remove(&self, name: &str) {
        if let Some(it) = self.table.write().await.remove(name) {
            let mut anm = self.anm.write().await;
            for it in it.addrs {
                anm.remove(&it);
            }
        }

        log::info!("remove a device for devices, name={}", name);
    }

    async fn remove_from_addr(&self, addr: Ipv4Addr) {
        if let Some(it) = self.anm.write().await.remove(&addr) {
            self.remove(&it).await;
        }
    }

    async fn update_description_from_addr(
        &self,
        addr: Ipv4Addr,
        description: MediaStreamDescription,
    ) {
        if let Some(it) = self.anm.read().await.get(&addr) {
            if let Some(device) = self.table.read().await.get(it) {
                device.update_description(description).await;
            }
        }
    }
}

struct DiscoveryServiceObserver {
    description: Arc<RwLock<Option<MediaStreamDescription>>>,
    tx: Arc<watch::Sender<()>>,
    devices: Arc<Devices>,
    runtime: Arc<Handle>,
    name: String,
}

impl DiscoveryObserver<ServiceInfo> for DiscoveryServiceObserver {
    fn resolve(&self, name: &str, addrs: Vec<Ipv4Addr>, info: ServiceInfo) {
        if name == &self.name {
            log::warn!("discovery service resolve myself, ignore this, name={}", name);

            return;
        }

        log::info!(
            "discovery service resolve, name={}, addrs={:?}, info={:?}",
            name,
            addrs,
            info
        );

        let name = name.to_string();
        let notify = self.tx.clone();
        let devices = self.devices.clone();
        let description = self.description.clone();
        self.runtime.spawn(async move {
            match Device::new(name.clone(), info.kind, addrs, info.port).await {
                Ok((mut device, disconnection_notify)) => {
                    {
                        if let Some(description) = description.read().await.as_ref() {
                            device.send_description(description).await;
                        }

                        devices.set(&name, device).await;

                        if let Err(e) = notify.send(()) {
                            log::error!("devices send change notify error={:?}", e);
                        }
                    }

                    if disconnection_notify.await.is_ok() {
                        devices.remove(&name).await;

                        if let Err(e) = notify.send(()) {
                            log::error!("devices send change notify error={:?}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("failed to create device, error={}", e);
                }
            }
        });
    }
}

pub struct DevicesManager {
    rx: watch::Receiver<()>,
    devices: Arc<Devices>,
    #[allow(dead_code)]
    discoverys: (DiscoveryService, DiscoveryService),
    description: Arc<RwLock<Option<MediaStreamDescription>>>,
}

impl DevicesManager {
    pub async fn new(name: String) -> Result<Self> {
        let devices: Arc<Devices> = Arc::new(Devices::default());

        let listener = TcpListener::bind("0.0.0.0:0").await?;
        let local_addr = listener.local_addr()?;

        log::info!("devices manager server listener addr={}", local_addr);

        let (tx, rx) = watch::channel::<()>(());
        let tx = Arc::new(tx);

        let devices_ = Arc::downgrade(&devices);
        tokio::spawn(async move {
            while let Ok((socket, addr)) = listener.accept().await {
                let devices_ = devices_.clone();
                let ip = match addr.ip() {
                    IpAddr::V4(it) => it,
                    _ => unimplemented!(),
                };

                tokio::spawn(async move {
                    match accept_async(socket).await {
                        Ok(mut stream) => {
                            while let Some(Ok(message)) = stream.next().await {
                                if let Message::Text(text) = message {
                                    if let Some(devices) = devices_.upgrade() {
                                        if let Ok(it) = serde_json::from_str(text.as_str()) {
                                            devices.update_description_from_addr(ip, it).await;
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
                        devices.remove_from_addr(ip).await;
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
                runtime: Arc::new(Handle::current()),
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

    pub async fn set_description(
        &self,
        name_list: Vec<String>,
        description: MediaStreamDescription,
    ) {
        let mut devices = self.devices.table.write().await;

        if name_list.is_empty() {
            self.description.write().await.replace(description.clone());
        } else {
            self.description.write().await.take();
        }

        if name_list.is_empty() {
            for (_, device) in devices.iter_mut() {
                device.send_description(&description).await;
            }
        } else {
            for name in name_list {
                if let Some(device) = devices.get_mut(&name) {
                    device.send_description(&description).await;
                }
            }
        }
    }

    pub async fn get_devices(&self) -> Vec<DeviceInfo> {
        let mut devices = Vec::with_capacity(100);

        for (k, v) in self.devices.table.read().await.iter() {
            devices.push(DeviceInfo {
                description: v.get_description().await,
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

pub struct DevicesWatcher(watch::Receiver<()>);

impl DevicesWatcher {
    pub async fn change(&mut self) -> bool {
        self.0.changed().await.is_ok()
    }
}
