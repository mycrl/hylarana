use std::{thread::sleep, time::Duration};

use hylarana_discovery::DiscoveryService;

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let name = "panda".to_string();
    let it = DiscoveryService::register("node", &name).unwrap();

    sleep(Duration::from_secs(20));
    it.stop().unwrap();

    sleep(Duration::from_secs(10));
    drop(it);
}
