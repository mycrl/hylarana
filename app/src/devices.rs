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

use crate::env::Env;

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceInfo {
    pub description: Option<MediaStreamDescription>,
    pub addrs: Vec<Ipv4Addr>,
    pub name: String,
    pub port: u16,
}

pub struct Device {
    _hook: Arc<()>,
    sender: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    description: Arc<RwLock<Option<MediaStreamDescription>>>,
    addrs: Vec<Ipv4Addr>,
    port: u16,
}

impl Device {
    async fn new(addrs: Vec<Ipv4Addr>, port: u16) -> Result<(Self, oneshot::Receiver<()>)> {
        let (socket, response) =
            connect_async(format!("ws://{}:{}", addrs[0], port).into_client_request()?).await?;

        if response.status() == StatusCode::SWITCHING_PROTOCOLS {
            return Err(anyhow!(
                "websocket connect status code={}",
                response.status()
            ));
        }

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
        });

        Ok((
            Self {
                _hook,
                description: Default::default(),
                sender,
                addrs,
                port,
            },
            rx,
        ))
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn get_addrs(&self) -> Vec<Ipv4Addr> {
        self.addrs.clone()
    }

    pub async fn get_description(&self) -> Option<MediaStreamDescription> {
        self.description.read().await.clone()
    }

    async fn send_description(&mut self, description: &MediaStreamDescription) {
        if let Err(e) = self
            .sender
            .send(Message::text(serde_json::to_string(description).unwrap()))
            .await
        {
            log::error!("{}", e);
        }
    }

    async fn update_description(&self, description: MediaStreamDescription) {
        self.description.write().await.replace(description);
    }
}

#[derive(Default)]
struct Devices {
    table: RwLock<HashMap<String, Device>>,
    names: RwLock<HashMap<Ipv4Addr, String>>,
}

impl Devices {
    async fn set(&self, name: &str, device: Device) {
        let mut names = self.names.write().await;
        for it in &device.addrs {
            names.insert(*it, name.to_string());
        }

        self.table.write().await.insert(name.to_string(), device);
    }

    async fn remove(&self, name: &str) {
        if let Some(it) = self.table.write().await.remove(name) {
            let mut names = self.names.write().await;
            for it in it.addrs {
                names.remove(&it);
            }
        }
    }

    async fn remove_from_addr(&self, addr: Ipv4Addr) {
        if let Some(it) = self.names.write().await.remove(&addr) {
            self.remove(&it).await;
        }
    }

    async fn update_description_from_addr(
        &self,
        addr: Ipv4Addr,
        description: MediaStreamDescription,
    ) {
        if let Some(it) = self.names.read().await.get(&addr) {
            if let Some(device) = self.table.read().await.get(it) {
                device.update_description(description).await;
            }
        }
    }
}

struct DiscoveryServiceObserver {
    tx: Arc<watch::Sender<()>>,
    env: Arc<RwLock<Env>>,
    devices: Arc<Devices>,
    runtime: Arc<Handle>,
}

impl DiscoveryObserver<u16> for DiscoveryServiceObserver {
    fn resolve(&self, name: &str, addrs: Vec<Ipv4Addr>, port: u16) {
        if name == &self.env.blocking_read().settings.name {
            return;
        }

        let name = name.to_string();
        let devices = self.devices.clone();
        let notify = self.tx.clone();
        self.runtime.spawn(async move {
            match Device::new(addrs, port).await {
                Ok((it, disconnection_notify)) => {
                    {
                        devices.set(&name, it).await;

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
                    log::error!("{}", e);
                }
            }
        });
    }

    fn remove(&self, name: &str) {
        if name == &self.env.blocking_read().settings.name {
            return;
        }

        let name = name.to_string();
        let devices = self.devices.clone();
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            devices.remove(&name).await;

            if let Err(e) = tx.send(()) {
                log::error!("devices send change notify error={:?}", e);
            }
        });
    }
}

pub struct DevicesManager {
    rx: watch::Receiver<()>,
    devices: Arc<Devices>,
    #[allow(dead_code)]
    discoverys: (DiscoveryService, DiscoveryService),
}

impl DevicesManager {
    pub async fn new(env: Arc<RwLock<Env>>) -> Result<Self> {
        let devices: Arc<Devices> = Arc::new(Devices::default());

        let listener = TcpListener::bind("0.0.0.0:0").await?;
        let port = listener.local_addr()?.port();

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
                            log::error!("{}", e);
                        }
                    }

                    if let Some(devices) = devices_.upgrade() {
                        devices.remove_from_addr(ip).await;
                    }
                });
            }
        });

        let discoverys = (
            DiscoveryService::register(&env.read().await.settings.name, &port)?,
            DiscoveryService::query(DiscoveryServiceObserver {
                runtime: Arc::new(Handle::current()),
                devices: devices.clone(),
                env: env.clone(),
                tx,
            })?,
        );

        Ok(Self {
            rx,
            devices,
            discoverys,
        })
    }

    pub async fn send_description(&self, names: Vec<String>, description: MediaStreamDescription) {
        let mut devices = self.devices.table.write().await;
        for name in names {
            if let Some(it) = devices.get_mut(&name) {
                it.send_description(&description).await;
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
