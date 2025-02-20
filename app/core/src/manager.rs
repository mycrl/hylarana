use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use anyhow::Result;
use bytes::{Buf, BufMut, BytesMut};
use common::{runtime::get_runtime_handle, MediaStreamDescription};
use crossbeam::channel::{unbounded, Receiver};
use hylarana::{DiscoveryObserver, DiscoveryService};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::broadcast,
};

use uuid::Uuid;

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
pub struct DeviceInfo {
    pub name: String,
    pub ip: Ipv4Addr,
    pub kind: DeviceType,
    pub description: Option<MediaStreamDescription>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Node {
    name: String,
    kind: DeviceType,
    description: Option<MediaStreamDescription>,
}

impl Node {
    fn as_bytes(&self) -> BytesMut {
        let mut bytes = BytesMut::with_capacity(2000);
        let payload = serde_json::to_vec(self).unwrap();

        bytes.put_u16(payload.len() as u16);
        bytes.extend_from_slice(&payload);

        bytes
    }
}

impl TryFrom<BytesMut> for Node {
    type Error = anyhow::Error;

    fn try_from(value: BytesMut) -> Result<Self, Self::Error> {
        Ok(serde_json::from_slice::<Node>(&value)?)
    }
}

struct ServiceObserver {
    service_name: String,
    update_receiver: broadcast::Receiver<()>,
    local_info: Arc<RwLock<Option<(Vec<Ipv4Addr>, Node)>>>,
}

impl ServiceObserver {
    async fn change(
        ip: Ipv4Addr,
        socket: &mut TcpStream,
        local_info: Arc<RwLock<Option<(Vec<Ipv4Addr>, Node)>>>,
    ) {
        let bytes = local_info
            .read()
            .as_ref()
            .map(|(targets, node)| {
                // No target is specified, or the current ip is in the list are required to
                // synchronise the information.
                //
                // Not specifying a target is equivalent to broadcast mode, which can be
                // received by any device.
                if targets.is_empty() || targets.iter().find(|it| **it == ip).is_some() {
                    Some(node.as_bytes())
                } else {
                    None
                }
            })
            .flatten();

        if let Some(bytes) = bytes {
            if socket.write_all(&bytes).await.is_err() {
                return;
            }
        }
    }
}

impl DiscoveryObserver<u16> for ServiceObserver {
    fn resolve(&self, name: &str, addrs: Vec<Ipv4Addr>, port: u16) {
        // It may receive its own registration service, which is filtered out here.
        if name == self.service_name {
            return;
        }

        // Even if multiple IP addresses exist, the first one is forced to be used.
        let ip = addrs[0];
        let addr = SocketAddr::new(IpAddr::V4(ip), port);

        let local_info = self.local_info.clone();
        let mut update_receiver = self.update_receiver.resubscribe();
        get_runtime_handle().spawn(async move {
            if let Ok(mut socket) = TcpStream::connect(addr).await {
                // You need to synchronise the information once when you first connect.
                Self::change(ip, &mut socket, local_info.clone()).await;

                // An external notification that the information on the current device has been
                // updated, here the information is synchronised across again.
                while let Ok(_) = update_receiver.recv().await {
                    Self::change(ip, &mut socket, local_info.clone()).await;
                }
            }
        });
    }
}

pub struct DeviceManager {
    // This is where you store your own information so that the newly connected device can directly
    // synchronise the current information about itself to the newly connected device.
    local_info: Arc<RwLock<Option<(Vec<Ipv4Addr>, Node)>>>,
    nodes: Arc<RwLock<HashMap<Ipv4Addr, Node>>>,
    update_sender: broadcast::Sender<()>,
    change_receiver: Receiver<()>,
    _register: DiscoveryService,
    _query: DiscoveryService,
}

impl DeviceManager {
    pub fn new() -> Result<Self> {
        let handle = get_runtime_handle();

        let listener = handle.block_on(TcpListener::bind("0.0.0.0:0"))?;
        let local_addr = listener.local_addr()?;

        log::info!("device manager tcp listener bind={}", local_addr);

        let service_name = format!("hylarana-core={}", Uuid::new_v4());
        let _register = DiscoveryService::register(&service_name, &local_addr.port())?;

        log::info!(
            "service register initialization completed, service name={}",
            service_name
        );

        let (change_sender, change_receiver) = unbounded::<()>();
        let (update_sender, update_receiver) = broadcast::channel::<()>(1);

        let local_info: Arc<RwLock<Option<(Vec<Ipv4Addr>, Node)>>> = Default::default();
        let _query = DiscoveryService::query(ServiceObserver {
            update_receiver: update_receiver.resubscribe(),
            local_info: local_info.clone(),
            service_name,
        })?;

        log::info!("service query initialization completed");

        let nodes: Arc<RwLock<HashMap<Ipv4Addr, Node>>> = Default::default();
        let nodes_ = nodes.clone();

        handle.spawn(async move {
            while let Ok((mut socket, addr)) = listener.accept().await {
                // Only ipv4 is supported, if it is not in the supported range, the connection
                // is rejected directly.
                let ip = match addr.ip() {
                    IpAddr::V4(ip) => ip,
                    _ => continue,
                };

                log::info!("device manager tcp listener accept a socket, ip={}", ip);

                let change_sender = change_sender.clone();
                let nodes_ = nodes_.clone();
                tokio::spawn(async move {
                    let mut bytes = BytesMut::with_capacity(1024);

                    while let Ok(size) = socket.read_buf(&mut bytes).await {
                        if size == 0 {
                            break;
                        }

                        // The header of a message is 2 bytes of length data, so a message is a
                        // minimum of two bytes.
                        if bytes.len() <= 2 {
                            continue;
                        }

                        // Peek at the length of the current message.
                        let size = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;

                        // Checks if the current buffer has received at least one complete message.
                        if size + 2 < bytes.len() {
                            continue;
                        }

                        // The length is no longer needed, discarding 2 bytes of length data.
                        bytes.advance(2);

                        if let Ok(node) = Node::try_from(bytes.split_to(size)) {
                            log::info!(
                                "device manager tcp socket recv a info, ip={}, node={:?}",
                                ip,
                                node
                            );

                            // It's the easiest way to handle this by overwriting the current ip
                            // regardless of whether it already exists or not.
                            nodes_.write().insert(ip, node);

                            // Notifies that the list of external devices has changed.
                            if change_sender.send(()).is_err() {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    // The current device has been taken offline, deleting the device information
                    // and notifying the external device list that it has been updated.
                    if nodes_.write().remove(&ip).is_some() {
                        let _ = change_sender.send(());
                    }

                    log::info!("device manager tcp socket closed, ip={}", ip);
                });
            }

            log::info!("device manager tcp listener closed");
        });

        Ok(Self {
            change_receiver,
            update_sender,
            local_info,
            _register,
            _query,
            nodes,
        })
    }

    pub fn send_info(
        &self,
        targets: Vec<Ipv4Addr>,
        name: String,
        description: Option<MediaStreamDescription>,
    ) {
        self.local_info.write().replace((
            targets,
            Node {
                kind: DEVICE_TYPE,
                description,
                name,
            },
        ));

        // Updates the self information while notifying all connected devices that the
        // current self information has been updated.
        let _ = self.update_sender.send(());
    }

    pub fn get_devices(&self) -> Vec<DeviceInfo> {
        self.nodes
            .read()
            .iter()
            .map(|(k, v)| DeviceInfo {
                description: v.description.clone(),
                name: v.name.clone(),
                kind: v.kind,
                ip: *k,
            })
            .collect()
    }

    pub fn get_watcher(&self) -> Watcher {
        Watcher(self.change_receiver.clone())
    }
}

pub struct Watcher(Receiver<()>);

impl Watcher {
    pub fn change(&mut self) -> bool {
        self.0.recv().is_ok()
    }
}
