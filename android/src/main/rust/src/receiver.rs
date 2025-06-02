use anyhow::Result;
use bytes::Bytes;
use transport::{Buffer, TransportReceiver, TransportReceiverSink};

use jni::{
    JNIEnv,
    objects::{GlobalRef, JObject, JString, JValue, JValueGen},
};

use super::get_current_env;

struct ReceiverSink(GlobalRef);

impl TransportReceiverSink for ReceiverSink {
    fn sink(&mut self, buffer: Buffer<Bytes>) -> bool {
        let mut env = get_current_env();
        let bytes = if let Ok(it) = env.byte_array_from_slice(&buffer.data) {
            it.into()
        } else {
            return false;
        };

        let ret = env.call_method(
            self.0.as_obj(),
            "sink",
            "(IIJ[B)Z",
            &[
                JValue::Int(buffer.stream as i32),
                JValue::Int(buffer.ty as i32),
                JValue::Long(buffer.timestamp as i64),
                JValue::Object(&bytes),
            ],
        );

        let _ = env.delete_local_ref(bytes);
        if let Ok(JValueGen::Bool(size)) = ret {
            size > 0
        } else {
            false
        }
    }

    fn close(&mut self) {
        let mut env = get_current_env();

        let _ = env.call_method(self.0.as_obj(), "close", "()V", &[]);
    }
}

#[allow(unused)]
pub struct Receiver(TransportReceiver);

impl Receiver {
    pub fn new(
        env: &mut JNIEnv,
        addr: &JString,
        options: &JString,
        observer: &JObject,
    ) -> Result<Self> {
        let addr: String = env.get_string(addr)?.into();
        let options: String = env.get_string(options)?.into();

        Ok(Self(TransportReceiver::new(
            addr.parse()?,
            serde_json::from_str(&options)?,
            ReceiverSink(env.new_global_ref(observer)?),
        )?))
    }
}
