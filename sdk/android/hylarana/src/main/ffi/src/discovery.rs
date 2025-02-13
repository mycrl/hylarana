use std::net::Ipv4Addr;

use anyhow::Error;
use jni::objects::{GlobalRef, JValue};

pub use discovery::{DiscoveryObserver, DiscoveryService};

use super::get_current_env;

pub struct DiscoveryServiceObserver(pub GlobalRef);

unsafe impl Send for DiscoveryServiceObserver {}
unsafe impl Sync for DiscoveryServiceObserver {}

impl DiscoveryObserver<String> for DiscoveryServiceObserver {
    fn resolve(&self, name: &str, addrs: Vec<Ipv4Addr>, description: String) {
        let mut env = get_current_env();
        if let Err(e) = (|| {
            env.call_method(
                self.0.as_obj(),
                "resolve",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
                &[
                    JValue::Object(&env.new_string(name)?.into()),
                    JValue::Object(&env.new_string(serde_json::to_string(&addrs)?)?.into()),
                    JValue::Object(&env.new_string(description)?.into()),
                ],
            )?;

            Ok::<(), Error>(())
        })() {
            log::error!("DiscoveryObserver resolve error={:?}", e);
        }
    }

    fn remove(&self, name: &str) {
        let mut env = get_current_env();
        if let Err(e) = (|| {
            env.call_method(
                self.0.as_obj(),
                "remove",
                "(Ljava/lang/String)V",
                &[JValue::Object(&env.new_string(name)?.into())],
            )?;

            Ok::<(), Error>(())
        })() {
            log::error!("DiscoveryObserver remove error={:?}", e);
        }
    }
}
