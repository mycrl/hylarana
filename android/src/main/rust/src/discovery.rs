use std::net::IpAddr;

use anyhow::Error;
use jni::objects::{GlobalRef, JValue};

pub use discovery::{DiscoveryObserver, DiscoveryService};

use super::get_current_env;

pub struct DiscoveryServiceObserver(pub GlobalRef);

unsafe impl Send for DiscoveryServiceObserver {}
unsafe impl Sync for DiscoveryServiceObserver {}

impl DiscoveryObserver for DiscoveryServiceObserver {
    async fn online(&self, local_id: &str, id: &str, ip: IpAddr) {
        log::info!("devices manager device online, id={}, ip={}", id, ip);

        let mut env = get_current_env();
        if let Err(e) = (|| {
            env.call_method(
                self.0.as_obj(),
                "onLine",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
                &[
                    JValue::Object(&env.new_string(local_id)?.into()),
                    JValue::Object(&env.new_string(id)?.into()),
                    JValue::Object(&env.new_string(ip.to_string())?.into()),
                ],
            )?;

            Ok::<(), Error>(())
        })() {
            log::error!("DiscoveryObserver on line error={:?}", e);
        }
    }

    async fn offline(&self, local_id: &str, id: &str, ip: IpAddr) {
        log::info!("devices manager device offline, id={}, ip={}", id, ip);

        let mut env = get_current_env();
        if let Err(e) = (|| {
            env.call_method(
                self.0.as_obj(),
                "offLine",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
                &[
                    JValue::Object(&env.new_string(local_id)?.into()),
                    JValue::Object(&env.new_string(id)?.into()),
                    JValue::Object(&env.new_string(ip.to_string())?.into()),
                ],
            )?;

            Ok::<(), Error>(())
        })() {
            log::error!("DiscoveryObserver off line error={:?}", e);
        }
    }

    async fn on_metadata(&self, local_id: &str, id: &str, ip: IpAddr, metadata: Vec<u8>) {
        log::info!(
            "devices manager device on metadata, id={}, ip={} metadata={:?}",
            id,
            ip,
            std::str::from_utf8(&metadata)
        );

        let mut env = get_current_env();
        if let Err(e) = (|| {
            env.call_method(
                self.0.as_obj(),
                "onMetadata",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;[B)V",
                &[
                    JValue::Object(&env.new_string(local_id)?.into()),
                    JValue::Object(&env.new_string(id)?.into()),
                    JValue::Object(&env.new_string(ip.to_string())?.into()),
                    JValue::Object(&env.byte_array_from_slice(&metadata)?.into()),
                ],
            )?;

            Ok::<(), Error>(())
        })() {
            log::error!("DiscoveryObserver on metadata error={:?}", e);
        }
    }
}
