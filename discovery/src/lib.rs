use std::{
    collections::HashMap,
    io::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::{Duration, Instant},
};

use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, UdpSocket},
    sync::{
        Mutex, RwLock,
        broadcast::{Sender, channel},
    },
    time::sleep,
};

use uuid::Uuid;

pub trait DiscoveryObserver {
    #[allow(unused_variables)]
    fn online(&self, local_id: &str, id: &str, ip: IpAddr) -> impl Future<Output = ()> + Send {
        async {}
    }

    #[allow(unused_variables)]
    fn offline(&self, local_id: &str, id: &str, ip: IpAddr) -> impl Future<Output = ()> + Send {
        async {}
    }

    #[allow(unused_variables)]
    fn on_metadata(
        &self,
        local_id: &str,
        id: &str,
        ip: IpAddr,
        metadata: Vec<u8>,
    ) -> impl Future<Output = ()> + Send {
        async {}
    }
}

pub struct DiscoveryService {
    _t: Sender<()>,
    metadata: Arc<RwLock<Option<Vec<u8>>>>,
    sequence: Arc<AtomicU32>,
    local_id: String,
}

impl DiscoveryService {
    pub async fn new<O>(bind: SocketAddr, observer: O) -> Result<Self, Error>
    where
        O: DiscoveryObserver + Send + Sync + 'static,
    {
        let observer = Arc::new(observer);
        let local_id = Uuid::new_v4().to_string();
        let sequence: Arc<AtomicU32> = Default::default();
        let metadata: Arc<RwLock<Option<Vec<u8>>>> = Default::default();
        let service: Arc<Mutex<HashMap<String, Service>>> = Default::default();

        let (tx, mut rx) = channel::<()>(2);

        // Create a TCP listener and a UDP socket for discovery
        // The TCP listener is used for HTTP requests, and the UDP socket is used for
        // broadcasting pings
        let listener = TcpListener::bind(bind).await?;
        let socket = Arc::new(UdpSocket::bind(bind).await?);
        socket.set_broadcast(true)?;

        {
            let app = Router::new()
                .route(
                    "/metadata",
                    get(
                        |State(metadata): State<Arc<RwLock<Option<Vec<u8>>>>>| async move {
                            if let Some(data) = metadata.read().await.as_ref() {
                                data.clone().into_response()
                            } else {
                                StatusCode::NOT_FOUND.into_response()
                            }
                        },
                    ),
                )
                .with_state(metadata.clone());

            let mut rx = tx.subscribe();
            tokio::spawn(async move {
                axum::serve(listener, app)
                    .with_graceful_shutdown(async move {
                        let _ = rx.recv().await;
                    })
                    .await
                    .unwrap();
            });
        }

        let to_addr = {
            let mut addr = bind.clone();
            addr.set_ip(IpAddr::V4(Ipv4Addr::BROADCAST));

            addr
        };

        {
            let mut rx = tx.subscribe();
            let local_id = local_id.clone();
            let services = service.clone();
            let socket = socket.clone();
            let observer = observer.clone();
            tokio::spawn(async move {
                let mut buffer = [0u8; 1024];

                loop {
                    tokio::select! {
                        Ok((size, addr)) = socket.recv_from(&mut buffer) => {
                            if size == 0 {
                                break;
                            }

                            if let Ok(ping) = serde_json::from_slice::<Ping>(&buffer[..size]) {
                                // ignore ping from self
                                if ping.id == local_id {
                                    continue;
                                }

                                let mut services = services.lock().await;
                                if let Some(service) = services.get_mut(ping.id) {
                                    if service.sequence != ping.sequence {
                                        if let Some(metadata) = request_metadata(addr.ip(), to_addr.port()).await {
                                            observer.on_metadata(&local_id, ping.id, addr.ip(), metadata).await;
                                        }
                                    }

                                    service.update_at = Instant::now();
                                    service.sequence = ping.sequence;
                                } else {
                                    services.insert(ping.id.to_string(), Service {
                                        update_at: Instant::now(),
                                        sequence: ping.sequence,
                                        ip: addr.ip(),
                                    });

                                    observer.online(&local_id, ping.id, addr.ip()).await;
                                    if let Some(metadata) = request_metadata(addr.ip(), to_addr.port()).await {
                                        observer.on_metadata(&local_id, ping.id, addr.ip(), metadata).await;
                                    }
                                }
                            }
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
        }

        {
            let local_id = local_id.clone();
            let sequence = sequence.clone();
            let services = service.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = sleep(Duration::from_secs(1)) => {
                            if let Err(e) = socket.send_to(&serde_json::to_vec(&Ping {
                                sequence: sequence.load(Ordering::Relaxed),
                                id: &local_id,
                            }).unwrap(), to_addr).await {
                                log::error!("discovery service send ping failed, err={:?}", e);
                            }

                            {
                                let mut offlines = Vec::new();
                                let mut services = services.lock().await;

                                for (k, v) in services.iter() {
                                    if v.update_at.elapsed().as_secs() >= 3 {
                                        offlines.push(k.clone());

                                        observer.offline(&local_id, k, v.ip).await;
                                    }
                                }

                                for k in &offlines {
                                    services.remove(k);
                                }
                            }
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
        }

        Ok(Self {
            _t: tx,
            metadata,
            sequence,
            local_id,
        })
    }

    pub fn local_id(&self) -> &str {
        &self.local_id
    }

    pub async fn set_metadata(&self, metadata: Vec<u8>) {
        log::info!("discovery service set metadata");

        self.sequence.fetch_add(1, Ordering::Relaxed);
        self.metadata.write().await.replace(metadata);
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Ping<'a> {
    id: &'a str,
    sequence: u32,
}

struct Service {
    ip: IpAddr,
    sequence: u32,
    update_at: Instant,
}

async fn request_metadata(ip: IpAddr, port: u16) -> Option<Vec<u8>> {
    let res = reqwest::get(format!("http://{}:{}/metadata", ip, port))
        .await
        .ok()?;

    if res.status() != StatusCode::OK {
        return None;
    }

    Some(res.bytes().await.ok()?.to_vec())
}
