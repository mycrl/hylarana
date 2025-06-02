use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use anyhow::Result;
use hylarana::{DiscoveryObserver, DiscoveryService, MediaStreamDescription, get_runtime_handle};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

#[cfg(target_os = "windows")]
pub static DEVICE_TYPE: DeviceType = DeviceType::Windows;

#[cfg(target_os = "macos")]
pub static DEVICE_TYPE: DeviceType = DeviceType::Apple;

#[cfg(target_os = "linux")]
pub static DEVICE_TYPE: DeviceType = DeviceType::Linux;

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum DeviceType {
    Windows,
    Android,
    Apple,
    Linux,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DeviceMetadata {
    pub port: u16,
    pub description: MediaStreamDescription,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Device {
    pub name: String,
    pub ip: IpAddr,
    pub kind: DeviceType,
    pub metadata: Option<DeviceMetadata>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ServiceMessage {
    targets: Vec<String>,
    name: String,
    kind: DeviceType,
    metadata: Option<DeviceMetadata>,
}

enum Event {
    OffLine,
    NewDevice,
}

struct ServiceObserver {
    tx: UnboundedSender<Event>,
    devices: Arc<RwLock<HashMap<String, Device>>>,
}

impl DiscoveryObserver for ServiceObserver {
    async fn offline(&self, _local_id: &str, id: &str, ip: IpAddr) {
        log::info!("devices manager device offline, id={}, ip={}", id, ip);

        self.devices.write().remove(id);

        let _ = self.tx.send(Event::OffLine);
    }

    async fn on_metadata(&self, local_id: &str, id: &str, ip: IpAddr, metadata: Vec<u8>) {
        log::info!(
            "devices manager device on metadata, id={}, ip={} metadata={:?}",
            id,
            ip,
            std::str::from_utf8(&metadata)
        );

        if let Ok(ServiceMessage {
            targets,
            name,
            kind,
            metadata,
            ..
        }) = serde_json::from_slice(&metadata)
        {
            if targets.is_empty() || targets.iter().find(|it| it.as_str() == local_id).is_some() {
                log::info!(
                    "devices manager update device, id={}, targets={:?}, name={}, kind={:?}",
                    id,
                    targets,
                    name,
                    kind
                );

                self.devices.write().insert(
                    id.to_string(),
                    Device {
                        metadata,
                        ip,
                        name,
                        kind,
                    },
                );

                let _ = self.tx.send(Event::NewDevice);
            }
        }
    }
}

pub struct Discovery {
    service: Arc<DiscoveryService>,
    devices: Arc<RwLock<HashMap<String, Device>>>,
    receivers: Arc<RwLock<HashMap<usize, UnboundedSender<()>>>>,
}

impl Discovery {
    pub fn new(addr: SocketAddr) -> Result<Arc<Self>> {
        let devices: Arc<RwLock<HashMap<String, Device>>> = Default::default();

        let (tx, mut rx) = unbounded_channel::<Event>();
        let receivers: Arc<RwLock<HashMap<usize, UnboundedSender<()>>>> = Default::default();

        let service = Arc::new(get_runtime_handle().block_on(DiscoveryService::new(
            addr,
            ServiceObserver {
                devices: devices.clone(),
                tx,
            },
        ))?);

        let receivers_ = receivers.clone();
        get_runtime_handle().spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    _ => {
                        let mut list = Vec::with_capacity(5);

                        {
                            for (index, tx) in receivers_.read().iter() {
                                if tx.send(()).is_err() {
                                    list.push(*index);
                                }
                            }
                        }

                        {
                            let mut receivers = receivers_.write();
                            for item in list {
                                receivers.remove(&item);
                            }
                        }
                    }
                }
            }
        });

        log::info!("service register initialization completed");

        Ok(Arc::new(Self {
            receivers,
            devices,
            service,
        }))
    }

    pub fn set_metadata(
        &self,
        name: String,
        targets: Vec<String>,
        metadata: Option<DeviceMetadata>,
    ) {
        let payload = ServiceMessage {
            kind: DEVICE_TYPE,
            metadata,
            targets,
            name,
        };

        log::info!("devices manager set metadata={:?}", payload);

        get_runtime_handle().block_on(
            self.service
                .set_metadata(serde_json::to_vec(&payload).unwrap()),
        );
    }

    pub fn get_devices(&self) -> Vec<Device> {
        self.devices.read().iter().map(|(_, v)| v.clone()).collect()
    }

    pub async fn get_watcher(&self) -> Watcher {
        let (tx, rx) = unbounded_channel::<()>();

        let mut receivers = self.receivers.write();
        let index = receivers.len();
        receivers.insert(index, tx);

        Watcher(rx)
    }
}

pub struct Watcher(UnboundedReceiver<()>);

impl Watcher {
    pub async fn change(&mut self) -> bool {
        self.0.recv().await.is_some()
    }
}
