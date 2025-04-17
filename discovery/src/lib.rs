use std::{
    collections::HashMap,
    io::{Error as IoError, ErrorKind as IoErrorKind},
    net::Ipv4Addr,
    time::Duration,
};

use libp2p::{
    BehaviourBuilderError, TransportError,
    futures::StreamExt,
    gossipsub::{self, MessageId, PublishError},
    mdns,
    multiaddr::{self, Protocol},
    noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};

use thiserror::Error;
use tokio::sync::mpsc::{Sender, channel, error::SendError};
use uuid::Uuid;

#[derive(NetworkBehaviour)]
pub struct DiscoveryBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub struct DiscoveryContext<'a> {
    behaviour: &'a mut DiscoveryBehaviour,
    topic: &'a gossipsub::IdentTopic,
    pub local_id: &'a str,
    pub id: String,
    pub ip: Ipv4Addr,
}

impl<'a> DiscoveryContext<'a> {
    pub fn broadcast(&mut self, message: Vec<u8>) -> Result<(), DiscoveryError> {
        self.behaviour
            .gossipsub
            .publish(self.topic.clone(), message)?;

        Ok(())
    }
}

pub trait DiscoveryObserver {
    #[allow(unused_variables)]
    fn online(&self, ctx: DiscoveryContext) -> impl Future<Output = ()> + Send {
        async {}
    }

    #[allow(unused_variables)]
    fn offline(&self, ctx: DiscoveryContext) -> impl Future<Output = ()> + Send {
        async {}
    }

    #[allow(unused_variables)]
    fn on_message(
        &self,
        ctx: DiscoveryContext,
        message: Vec<u8>,
    ) -> impl Future<Output = ()> + Send {
        async {}
    }
}

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error(transparent)]
    IoError(#[from] IoError),
    #[error(transparent)]
    NoiseError(#[from] noise::Error),
    #[error(transparent)]
    BehaviourBuilderError(#[from] BehaviourBuilderError),
    #[error(transparent)]
    SubscriptionError(#[from] gossipsub::SubscriptionError),
    #[error(transparent)]
    MultiaddrError(#[from] multiaddr::Error),
    #[error(transparent)]
    TransportError(#[from] TransportError<IoError>),
    #[error(transparent)]
    PublishError(#[from] PublishError),
}

pub struct DiscoveryService {
    channel: Sender<Vec<u8>>,
    local_id: String,
}

impl DiscoveryService {
    pub async fn new<O>(topic: String, observer: O) -> Result<Self, DiscoveryError>
    where
        O: DiscoveryObserver + Send + 'static,
    {
        let mut swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Anonymous,
                    gossipsub::ConfigBuilder::default()
                        .duplicate_cache_time(Duration::from_secs(10))
                        .heartbeat_interval(Duration::from_secs(10))
                        .validation_mode(gossipsub::ValidationMode::None)
                        .message_id_fn(move |_| MessageId::new(Uuid::new_v4().as_bytes()))
                        .build()
                        .map_err(|msg| IoError::new(IoErrorKind::Other, msg))?,
                )?;

                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?;

                Ok(DiscoveryBehaviour { gossipsub, mdns })
            })?
            .build();

        log::info!("discovery service swarm is created");

        let topic = gossipsub::IdentTopic::new(topic);

        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        swarm.listen_on("/ip4/0.0.0.0/tcp/60342".parse()?)?;

        let local_id = swarm.local_peer_id().to_string();

        log::info!(
            "discovery service gossipsub subscribe topic={topic}, local_id={}",
            local_id
        );

        let (tx, mut rx) = channel(1);
        let local_id_ = local_id.clone();
        tokio::spawn(async move {
            let mut address: HashMap<String, Ipv4Addr> = Default::default();

            loop {
                tokio::select! {
                    Some(message) = rx.recv() => {
                        log::info!("discovery service loop recv a message");

                        if !address.is_empty() {
                            if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), message) {
                                log::warn!("discovery service gossipsub publish is failed, error={}", e);
                            }
                        } else {
                            log::info!("discovery service device table is empty, ignore message");
                        }
                    }
                    event = swarm.select_next_some() => {
                        log::info!("discovery service swarm event={:?}", event);

                        match event {
                            SwarmEvent::Behaviour(DiscoveryBehaviourEvent::Mdns(mdns::Event::Discovered(
                                peers,
                            ))) => {
                                let behaviour = swarm.behaviour_mut();

                                for (peer_id, _) in peers {
                                    behaviour.gossipsub.add_explicit_peer(&peer_id);
                                }
                            }
                            SwarmEvent::Behaviour(DiscoveryBehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
                                let behaviour = swarm.behaviour_mut();

                                for (peer_id, _) in peers {
                                    behaviour.gossipsub.remove_explicit_peer(&peer_id);
                                }
                            }
                            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                                let mut addr = endpoint.get_remote_address().iter();

                                while let Some(it) = addr.next() {
                                    if let Protocol::Ip4(ip) = it {
                                        address.insert(peer_id.to_string(), ip);

                                        break;
                                    }
                                }
                            }
                            SwarmEvent::Behaviour(DiscoveryBehaviourEvent::Gossipsub(
                                gossipsub::Event::Subscribed { peer_id, .. },
                            )) => {
                                let id = peer_id.to_string();

                                if let Some(ip) = address.get(&id).copied() {
                                    observer.online(DiscoveryContext {
                                        behaviour: swarm.behaviour_mut(),
                                        local_id: &local_id_,
                                        topic: &topic,
                                        id,
                                        ip,
                                    }).await;
                                }
                            }
                            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                                let id = peer_id.to_string();

                                let behaviour = swarm.behaviour_mut();
                                if let Some(ip) = address.remove(&id) {
                                    observer.offline(DiscoveryContext {
                                        local_id: &local_id_,
                                        topic: &topic,
                                        behaviour,
                                        id,
                                        ip,
                                    }).await;
                                }

                                behaviour.gossipsub.remove_explicit_peer(&peer_id);
                            }
                            SwarmEvent::Behaviour(DiscoveryBehaviourEvent::Gossipsub(
                                gossipsub::Event::Message { propagation_source, message, .. },
                            )) => {
                                let id = propagation_source.to_string();

                                if let Some(ip) = address.get(&id).copied() {
                                    observer.on_message(DiscoveryContext {
                                        behaviour: swarm.behaviour_mut(),
                                        local_id: &local_id_,
                                        topic: &topic,
                                        id,
                                        ip,
                                    }, message.data).await;
                                }
                            }
                            _ => (),
                        }
                    }
                    else => {
                        swarm.behaviour_mut().gossipsub.unsubscribe(&topic);

                        break;
                    }
                }
            }

            log::info!("discovery service message loop is exited");
        });

        Ok(Self {
            channel: tx,
            local_id,
        })
    }

    pub fn local_id(&self) -> &str {
        &self.local_id
    }

    pub async fn broadcast(&self, message: Vec<u8>) -> Result<(), SendError<Vec<u8>>> {
        log::info!("discovery service broadcast message before");

        self.channel.send(message).await
    }
}
