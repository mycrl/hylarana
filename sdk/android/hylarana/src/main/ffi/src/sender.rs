use std::sync::Arc;

use anyhow::{anyhow, Result};
use bytes::BytesMut;
use transport::{
    create_sender, with_capacity as package_with_capacity, StreamBufferInfo, StreamKind,
    StreamSenderAdapter, TransportSender,
};

use jni::{
    objects::{JByteArray, JObject, JString, JValueGen},
    JNIEnv,
};

pub struct Sender {
    sender: TransportSender,
    adapter: Arc<StreamSenderAdapter>,
}

impl Sender {
    pub fn new(env: &mut JNIEnv, options: &JString) -> Result<Self> {
        let options: String = env.get_string(options)?.into();
        let sender = create_sender(serde_json::from_str(&options)?)?;
        Ok(Self {
            adapter: sender.get_adapter(),
            sender,
        })
    }

    pub fn get_id(&self) -> &str {
        self.sender.get_id()
    }

    pub fn sink(&self, env: &mut JNIEnv, info: JObject, array: JByteArray) -> Result<bool> {
        let bytes = copy_from_byte_array(env, &array)?;
        let info = {
            let kind = if let JValueGen::Int(it) = env.get_field(&info, "type", "I")? {
                StreamKind::try_from(it as u8)?
            } else {
                return Err(anyhow!("StreamBufferInfo type not a int"));
            };

            let flags = if let JValueGen::Int(it) = env.get_field(&info, "flags", "I")? {
                it
            } else {
                return Err(anyhow!("StreamBufferInfo flags not a int"));
            };

            let timestamp = if let JValueGen::Long(it) = env.get_field(&info, "timestamp", "J")? {
                it as u64
            } else {
                return Err(anyhow!("StreamBufferInfo timestamp not a long"));
            };

            match kind {
                StreamKind::Video => StreamBufferInfo::Video(flags, timestamp),
                StreamKind::Audio => StreamBufferInfo::Audio(flags, timestamp),
            }
        };

        Ok(self.adapter.send(bytes, info))
    }
}

fn copy_from_byte_array(env: &JNIEnv, array: &JByteArray) -> Result<BytesMut> {
    let size = env.get_array_length(array)? as usize;
    let mut bytes = package_with_capacity(size);
    let start = bytes.len() - size;

    env.get_byte_array_region(array, 0, unsafe {
        std::mem::transmute::<&mut [u8], &mut [i8]>(&mut bytes[start..])
    })?;

    Ok(bytes)
}
