use std::sync::Arc;

use anyhow::{Result, anyhow};
use bytes::Bytes;
use transport::{StreamKind, StreamReceiverAdapter, TransportReceiver, create_mix_receiver};

use jni::{
    JNIEnv,
    objects::{GlobalRef, JObject, JString, JValue, JValueGen},
};

use super::get_current_env;

pub struct Receiver {
    observer: GlobalRef,
    receiver: TransportReceiver<StreamReceiverAdapter>,
}

impl Receiver {
    pub fn new(
        env: &mut JNIEnv,
        id: &JString,
        options: &JString,
        observer: &JObject,
    ) -> Result<Self> {
        let id: String = env.get_string(id)?.into();
        let options: String = env.get_string(options)?.into();

        Ok(Self {
            receiver: create_mix_receiver(&id, serde_json::from_str(&options)?)?,
            observer: env.new_global_ref(observer)?,
        })
    }

    pub fn sink(&self, buf: Bytes, kind: StreamKind, flags: i32, timestamp: u64) -> Result<()> {
        let mut env = get_current_env();
        let buf = env.byte_array_from_slice(&buf)?.into();
        let ret = env.call_method(
            self.observer.as_obj(),
            "sink",
            "(IIJ[B)Z",
            &[
                JValue::Int(kind as i32),
                JValue::Int(flags),
                JValue::Long(timestamp as i64),
                JValue::Object(&buf),
            ],
        );

        let _ = env.delete_local_ref(buf);
        if let JValueGen::Bool(ret) = ret? {
            if ret == 0 {
                return Err(anyhow!("sink return false."));
            }
        } else {
            return Err(anyhow!("connect return result type missing."));
        };

        Ok(())
    }

    pub fn close(&self) -> Result<()> {
        let mut env = get_current_env();
        env.call_method(self.observer.as_obj(), "close", "()V", &[])?;

        Ok(())
    }

    pub fn get_adapter(&self) -> Arc<StreamReceiverAdapter> {
        self.receiver.get_adapter()
    }
}
