use std::{thread::sleep, time::Duration};

use hylarana_discovery::DiscoveryService;

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let it = DiscoveryService::query(|_, _, _: String| {
        
    }).unwrap();

    sleep(Duration::from_secs(9999));
    drop(it);
}
