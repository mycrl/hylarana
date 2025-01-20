use std::net::Ipv4Addr;

use anyhow::Result;
use common::MediaStreamDescription;
use jni::objects::{GlobalRef, JValue};

pub use discovery::DiscoveryService;

use super::{get_current_env, TransformArray};
use crate::object::TransformObject;

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
        let addrs = addrs.to_array(&mut env)?;
        let description = description.to_object(&mut env).unwrap();
        env.call_method(
            self.0.as_obj(),
            "resolve",
            "([Ljava/lang/String;Ljava/util/Map;)V",
            &[
                JValue::Object(addrs.as_ref()),
                JValue::Object(description.as_ref()),
            ],
        )?;

        Ok(())
    }
}
