use std::{
    fmt::Debug,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    num::ParseIntError,
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use bytes::BytesMut;
use common::runtime::get_runtime_handle;
use mdns_sd::{IfKind, ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc::{channel, Sender},
    time::{self, timeout},
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Server,
    Client,
}

pub trait DiscoveryObserver<T>: Send + Sync {
    #[allow(unused_variables)]
    fn resolve(&self, name: &str, addrs: Vec<Ipv4Addr>, properties: T) {}

    #[allow(unused_variables)]
    fn remove(&self, name: &str) {}
}

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    MdnsError(#[from] mdns_sd::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

/// LAN service discovery.
///
/// which exposes its services through the MDNS protocol
/// and can allow other nodes or clients to discover the current service.
pub struct DiscoveryService {
    kind: Kind,
    runing: AtomicBool,
    service: ServiceDaemon,
    close_sender: Option<Sender<()>>,
}

impl DiscoveryService {
    /// Register the service, the service type is fixed, you can customize the
    /// port number, in properties you can add
    /// customized data to the published service.
    pub fn register<P: Serialize + Debug>(
        name: &str,
        description: &P,
    ) -> Result<Self, DiscoveryError> {
        let service = ServiceDaemon::new()?;
        service.disable_interface(IfKind::IPv6)?;

        let id = Uuid::new_v4().to_string();
        let description = serde_json::to_string(description)?;

        #[allow(unused_assignments)]
        let mut properties = Vec::new();

        let mut close_sender = None;
        if description.len() >= 255 {
            let handle = get_runtime_handle();
            let listener = handle.block_on(TcpListener::bind("0.0.0.0:0"))?;
            let port = listener.local_addr()?.port();

            let (tx, mut rx) = channel(1);
            close_sender.replace(tx);

            handle.spawn(async move {
                loop {
                    tokio::select! {
                        Ok((mut socket, _)) = listener.accept() => {
                            if socket.write_all(description.as_bytes()).await.is_ok() {
                                let _ = socket.flush().await;
                            }

                            drop(socket);
                        }
                        _ = rx.recv() => {
                            break;
                        }
                        else => {
                            break;
                        }
                    }
                }
            });

            properties = [("p", port.to_string())].to_vec();
        } else {
            properties = [("d", description)].to_vec();
        }

        service.register(
            ServiceInfo::new(
                "_hylarana._udp.local.",
                name,
                &format!("{}._hylarana._udp.local.", id),
                "",
                0,
                &properties[..],
            )?
            .enable_addr_auto(),
        )?;

        log::info!(
            "discovery service register, id={}, properties={:?}",
            id,
            properties
        );

        Ok(Self {
            runing: AtomicBool::new(true),
            kind: Kind::Server,
            close_sender,
            service,
        })
    }

    /// Query the registered service, the service type is fixed, when the query
    /// is published the callback function will call back all the network
    /// addresses of the service publisher as well as the attribute information.
    pub fn query<P, T>(observer: T) -> Result<Self, DiscoveryError>
    where
        P: DeserializeOwned + Send + Debug + 'static,
        T: DiscoveryObserver<P> + 'static,
    {
        let service = ServiceDaemon::new()?;
        service.disable_interface(IfKind::IPv6)?;

        let receiver = service.browse("_hylarana._udp.local.")?;
        get_runtime_handle().spawn(async move {
            loop {
                match receiver.recv() {
                    Ok(event) => match event {
                        ServiceEvent::ServiceResolved(info) => {
                            if let Err(e) = resolve(&observer, &info).await {
                                log::warn!("discovery service resolved error={:?}", e);
                            }
                        }
                        ServiceEvent::ServiceRemoved(_, full_name) => {
                            if let Some((name, _)) = full_name.split_once('.') {
                                observer.remove(name);
                            }
                        }
                        _ => log::info!("discovery service query event={:?}", event),
                    },
                    Err(e) => {
                        log::warn!("discovery service query error={:?}", e);

                        break;
                    }
                }
            }
        });

        Ok(Self {
            runing: AtomicBool::new(true),
            kind: Kind::Client,
            close_sender: None,
            service,
        })
    }

    pub fn stop(&self) -> Result<(), DiscoveryError> {
        if self.runing.load(Ordering::Relaxed) {
            self.runing.store(false, Ordering::Relaxed);
        } else {
            return Ok(());
        }

        if self.kind == Kind::Server {
            drop(self.service.unregister("_hylarana._udp.local.")?.recv());
        } else {
            self.service.stop_browse("_hylarana._udp.local.")?;
        }

        Ok(())
    }
}

impl Drop for DiscoveryService {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            log::error!("discovery service drop error={:?}", e);
        }

        if let Some(tx) = self.close_sender.take() {
            let _ = tx.send(());
        }
    }
}

async fn resolve<P, T>(observer: &T, info: &ServiceInfo) -> Result<(), DiscoveryError>
where
    P: DeserializeOwned + Send + Debug + 'static,
    T: DiscoveryObserver<P> + 'static,
{
    let addrs = info
        .get_addresses_v4()
        .into_iter()
        .map(|it| *it)
        .collect::<Vec<_>>();

    if let Some((name, _)) = info.get_fullname().split_once('.') {
        if let Some(properties) = info.get_property("d") {
            let properties = serde_json::from_str(properties.val_str())?;

            log::info!(
                "discovery service query, host={}, address={:?}, properties={:?}",
                info.get_hostname(),
                addrs,
                properties,
            );

            observer.resolve(name, addrs, properties);
        } else if let Some(port) = info.get_property("p") {
            let port: u16 = u16::from_str(port.val_str())?;

            let mut socket =
                TcpStream::connect(SocketAddr::new(IpAddr::V4(addrs[0]), port)).await?;

            let (tx, mut rx) = channel::<P>(1);
            tokio::spawn(async move {
                let mut buf = BytesMut::with_capacity(4096);

                let sleep = time::sleep(Duration::from_secs(5));
                tokio::pin!(sleep);

                loop {
                    tokio::select! {
                        Ok(size) = socket.read_buf(&mut buf) => {
                            if size == 0 {
                                break;
                            }
                        },
                        _ = &mut sleep => {
                            break;
                        }
                        else => {
                            break;
                        }
                    }
                }

                if let Ok(it) = serde_json::from_slice(&buf) {
                    let _ = tx.send(it);
                }
            });

            if let Some(properties) = timeout(Duration::from_secs(5), rx.recv())
                .await
                .ok()
                .flatten()
            {
                observer.resolve(name, addrs, properties);
            }
        }
    }

    Ok(())
}
