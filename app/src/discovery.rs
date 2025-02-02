use anyhow::Result;
use hylarana::{DiscoveryService, MediaStreamDescription};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum DiscoveryDescription {
    MediaStreamDescription(MediaStreamDescription),
}

pub struct Discovery {
    query: DiscoveryService,
}

impl Discovery {
    pub fn new() -> Result<Self> {
        let query = DiscoveryService::query(move |name, addrs, info: DiscoveryDescription| {

        })?;

        Ok(Self {
            query,
        })
    }
}
