use std::net::Ipv4Addr;

use anyhow::Result;
use common::MediaStreamDescription;
use jni::objects::{GlobalRef, JValue};

pub use discovery::DiscoveryService;

use super::get_current_env;

pub struct DiscoveryServiceObserver(pub GlobalRef);

unsafe impl Send for DiscoveryServiceObserver {}
unsafe impl Sync for DiscoveryServiceObserver {}

impl DiscoveryServiceObserver {
    pub fn resolve(
        &self,
        addrs: &Vec<Ipv4Addr>,
        description: &MediaStreamDescription,
    ) -> Result<()> {
        let mut env = get_current_env();
        env.call_method(
            self.0.as_obj(),
            "resolve",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            &[
                JValue::Object(&env.new_string(serde_json::to_string(addrs)?)?.into()),
                JValue::Object(&env.new_string(serde_json::to_string(description)?)?.into()),
            ],
        )?;

        Ok(())
    }
}
