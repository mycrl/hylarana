use std::{collections::HashMap, net::Ipv4Addr, sync::Arc};

use anyhow::Result;
use hylarana::{
    DiscoveryContext, DiscoveryObserver, DiscoveryService, MediaStreamDescription,
    get_runtime_handle,
};
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
pub struct Device {
    pub name: String,
    pub ip: Ipv4Addr,
    pub kind: DeviceType,
    pub description: Option<MediaStreamDescription>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ServiceMessage {
    targets: Vec<String>,
    name: String,
    kind: DeviceType,
    description: Option<MediaStreamDescription>,
}

enum Event {
    OnLine,
    OffLine,
    NewDevice,
}

struct ServiceObserver {
    tx: UnboundedSender<Event>,
    context: Arc<Context>,
}

impl DiscoveryObserver for ServiceObserver {
    async fn online(&self, ctx: DiscoveryContext<'_>) {
        log::info!(
            "devices manager device online, id={}, ip={}",
            ctx.id,
            ctx.ip
        );

        let _ = self.tx.send(Event::OnLine);
    }

    async fn offline(&self, ctx: DiscoveryContext<'_>) {
        log::info!("devices manager device offline, id={}", ctx.id);

        self.context.devices.write().remove(&ctx.id);

        let _ = self.tx.send(Event::OffLine);
    }

    async fn on_message(&self, ctx: DiscoveryContext<'_>, message: Vec<u8>) {
        log::info!(
            "devices manager device onmessage, id={}, message={:?}",
            ctx.id,
            std::str::from_utf8(&message)
        );

        if let Ok(ServiceMessage {
            targets,
            name,
            kind,
            description,
            ..
        }) = serde_json::from_slice(&message)
        {
            if targets.is_empty()
                || targets
                    .iter()
                    .find(|it| it.as_str() == ctx.local_id)
                    .is_some()
            {
                log::info!(
                    "devices manager update device, id={}, targets={:?}, name={}, kind={:?}",
                    ctx.id,
                    targets,
                    name,
                    kind
                );

                self.context.devices.write().insert(
                    ctx.id,
                    Device {
                        description,
                        ip: ctx.ip,
                        name,
                        kind,
                    },
                );

                let _ = self.tx.send(Event::NewDevice);
            }
        }
    }
}

#[derive(Default)]
struct Context {
    devices: RwLock<HashMap<String, Device>>,
    name: RwLock<String>,
    // This is where you store your own information so that the newly connected device can directly
    // synchronise the current information about itself to the newly connected device.
    description: RwLock<Option<MediaStreamDescription>>,
    targets: RwLock<Vec<String>>,
}

pub struct Discovery {
    context: Arc<Context>,
    service: Arc<DiscoveryService>,
    receivers: Arc<RwLock<HashMap<usize, UnboundedSender<()>>>>,
}

impl Discovery {
    pub fn new(name: String) -> Result<Arc<Self>> {
        let context = Arc::new(Context {
            name: RwLock::new(name),
            ..Default::default()
        });

        let (tx, mut rx) = unbounded_channel::<Event>();
        let receivers: Arc<RwLock<HashMap<usize, UnboundedSender<()>>>> = Default::default();

        let service = Arc::new(get_runtime_handle().block_on(DiscoveryService::new(
            "hylarana-app-core".to_string(),
            ServiceObserver {
                context: context.clone(),
                tx,
            },
        ))?);

        let service_ = service.clone();
        let context_ = context.clone();
        let receivers_ = receivers.clone();
        get_runtime_handle().spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    Event::OnLine => {
                        let payload = ServiceMessage {
                            description: context_.description.read().clone(),
                            targets: context_.targets.read().clone(),
                            name: context_.name.read().clone(),
                            kind: DEVICE_TYPE,
                        };

                        log::info!("devices manager send message={:?}", payload);

                        if let Ok(it) = serde_json::to_vec(&payload) {
                            if let Err(e) = service_.broadcast(it).await {
                                log::warn!(
                                    "discovery service gossipsub publish is failed, error={}",
                                    e
                                );
                            }
                        }
                    }
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
            context,
            service,
        }))
    }

    pub fn set_name(&self, name: String) {
        *self.context.name.write() = name;
    }

    pub fn send(&self, targets: Vec<String>, description: Option<MediaStreamDescription>) {
        let payload = ServiceMessage {
            name: self.context.name.read().clone(),
            description: description.clone(),
            targets: targets.clone(),
            kind: DEVICE_TYPE,
        };

        log::info!("devices manager send message={:?}", payload);

        if let Err(e) = get_runtime_handle().block_on(
            self.service
                .broadcast(serde_json::to_vec(&payload).unwrap()),
        ) {
            log::error!("devices manager send message failed, error={:?}", e);
        }

        *self.context.description.write() = description;
        *self.context.targets.write() = targets;
    }

    pub fn get_devices(&self) -> Vec<Device> {
        self.context
            .devices
            .read()
            .iter()
            .map(|(_, v)| v.clone())
            .collect()
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
