use std::{fmt::Debug, net::Ipv4Addr, sync::atomic::{AtomicBool, Ordering}, thread};

use mdns_sd::{IfKind, ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Server,
    Client,
}

#[derive(Debug, Error)]
pub enum DiscoveryError {
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
}

impl DiscoveryService {
    /// Register the service, the service type is fixed, you can customize the
    /// port number, in properties you can add
    /// customized data to the published service.
    pub fn register<P: Serialize + Debug>(
        port: u16,
        name: &str,
        properties: &P,
    ) -> Result<Self, DiscoveryError> {
        let service = ServiceDaemon::new()?;
        service.disable_interface(IfKind::IPv6)?;

        let id = Uuid::new_v4().to_string();
        service.register(
            ServiceInfo::new(
                "_hylarana._udp.local.",
                name,
                &format!("{}._hylarana._udp.local.", id),
                "",
                port,
                &[("p", &serde_json::to_string(properties)?)][..],
            )?
            .enable_addr_auto(),
        )?;

        log::info!(
            "discovery service register, port={}, id={}, properties={:?}",
            port,
            id,
            properties
        );

        Ok(Self {
            runing: AtomicBool::new(true),
            kind: Kind::Server, 
            service,
        })
    }

    /// Query the registered service, the service type is fixed, when the query
    /// is published the callback function will call back all the network
    /// addresses of the service publisher as well as the attribute information.
    pub fn query<P: DeserializeOwned + Debug, T: Fn(&str, Vec<Ipv4Addr>, P) + Send + 'static>(
        func: T,
    ) -> Result<Self, DiscoveryError> {
        let service = ServiceDaemon::new()?;
        service.disable_interface(IfKind::IPv6)?;

        let receiver = service.browse("_hylarana._udp.local.")?;
        thread::spawn(move || {
            let process = |info: ServiceInfo| {
                if let Some(properties) = info.get_property("p") {
                    let properties = serde_json::from_str(properties.val_str())?;
                    let addrs = info
                        .get_addresses_v4()
                        .into_iter()
                        .map(|it| *it)
                        .collect::<Vec<_>>();

                    log::info!(
                        "discovery service query, host={}, address={:?}, properties={:?}",
                        info.get_hostname(),
                        addrs,
                        properties,
                    );

                    if let Some((name, _)) = info.get_fullname().split_once('.') {
                        func(name, addrs, properties);
                    }
                }

                Ok::<(), DiscoveryError>(())
            };

            loop {
                match receiver.recv() {
                    Ok(ServiceEvent::ServiceResolved(info)) => {
                        if let Err(e) = process(info) {
                            log::warn!("discovery service resolved error={:?}", e);
                        }
                    }
                    Err(e) => {
                        log::warn!("discovery service query error={:?}", e);

                        break;
                    }
                    Ok(event) => {
                        log::info!("discovery service query event={:?}", event);
                    }
                }
            }
        });

        Ok(Self {
            runing: AtomicBool::new(true),
            kind: Kind::Client, 
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
    }
}
