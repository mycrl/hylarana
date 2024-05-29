use std::{collections::HashMap, sync::RwLock};

use tokio::sync::broadcast::{channel, Receiver, Sender};

use crate::signal::Signal;

pub struct Route {
    nodes: RwLock<HashMap<u32, u16>>,
    tx: Sender<Signal>,
    rx: Receiver<Signal>,
}

impl Default for Route {
    fn default() -> Self {
        let (tx, rx) = channel(20);
        Self {
            nodes: RwLock::new(HashMap::with_capacity(255)),
            tx,
            rx,
        }
    }
}

impl Route {
    /// Add a channel to the route, where the port number is the multicast port
    /// on the sender side
    ///
    /// This will trigger an event update, which will broadcast a channel
    /// release event
    pub fn add(&self, id: u32, port: u16) {
        self.nodes.write().unwrap().insert(id, port);
        self.tx.send(Signal::Start { id, port }).unwrap();
    }

    /// Delete a published channel
    ///
    /// This will trigger an event update, which will broadcast a channel closed
    /// event
    pub fn remove(&self, id: u32) {
        if self.nodes.write().unwrap().remove(&id).is_some() {
            self.tx.send(Signal::Stop { id }).unwrap();
        }
    }

    /// Get all channels that are publishing
    pub fn get_channels(&self) -> Vec<(u32, u16)> {
        self.nodes
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (*k, *v))
            .collect()
    }

    /// Get the event update listener, which can listen to all subsequent events
    /// triggered from the current listener
    pub fn get_changer(&self) -> Changer {
        Changer(self.rx.resubscribe())
    }
}

pub struct Changer(Receiver<Signal>);

impl Changer {
    /// Callback when the event is updated
    pub async fn change(&mut self) -> Option<Signal> {
        self.0.recv().await.ok()
    }
}
