use anyhow::Result;
use transport::{Buffer, BufferType, StreamType, TransportSender};

use jni::{
    JNIEnv,
    objects::{JByteArray, JString},
};

pub struct Sender(TransportSender);

impl Sender {
    pub fn new(env: &mut JNIEnv, bind: &JString, options: &JString) -> Result<Self> {
        let bind: String = env.get_string(bind)?.into();
        let options: String = env.get_string(options)?.into();

        Ok(Self(TransportSender::new(
            bind.parse()?,
            serde_json::from_str(&options)?,
        )?))
    }

    pub fn sink(
        &self,
        env: &mut JNIEnv,
        ty: i32,
        flags: i32,
        timestamp: i64,
        array: JByteArray,
    ) -> Result<bool> {
        Ok(self
            .0
            .send(Buffer {
                data: {
                    let size = env.get_array_length(&array)? as usize;
                    let mut bytes = Buffer::<()>::with_capacity(size);
                    let start = bytes.len() - size;

                    env.get_byte_array_region(array, 0, unsafe {
                        std::mem::transmute::<&mut [u8], &mut [i8]>(&mut bytes[start..])
                    })?;

                    bytes
                },
                stream: StreamType::try_from(ty as u8)?,
                ty: BufferType::try_from(flags as u8)?,
                timestamp: timestamp as u64,
            })
            .is_ok())
    }

    pub fn get_pkt_lose_rate(&self) -> f64 {
        self.0.get_pkt_lose_rate()
    }

    pub fn get_port(&self) -> u16 {
        self.0.local_addr().port()
    }
}
