use std::{thread::sleep, time::Duration};

use hylarana_discovery::{DiscoveryObserver, DiscoveryService};

struct Observer;

impl DiscoveryObserver<String> for Observer {
    fn remove(&self, _name: &str) {
        println!("====================== 000");
    }

    fn resolve(&self, _name: &str, _addrs: Vec<std::net::Ipv4Addr>, _properties: String) {
        println!("====================== 111");
    }
}

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let it = DiscoveryService::query(Observer).unwrap();

    sleep(Duration::from_secs(9999));
    drop(it);
}
