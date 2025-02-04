use anyhow::Result;
use hylarana::DiscoveryService;

pub struct Devices {
    discoverys: (DiscoveryService, DiscoveryService),
}

impl Devices {
    pub async fn new(name: String) -> Result<Self> {
        let discoverys = (
            DiscoveryService::register(3456, "node", &name)?,
            DiscoveryService::query(move |_, addrs, name: String| {

            })?,
        );

        Ok(Self {
            discoverys,
        })
    }
}
