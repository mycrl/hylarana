use std::net::{Ipv4Addr, SocketAddr};

use anyhow::{anyhow, Result};
use common::{
    MediaAudioStreamDescription, MediaStreamDescription, MediaVideoStreamDescription, Size,
    TransportOptions, TransportStrategy,
};

use jni::{
    objects::{JObject, JObjectArray, JString, JValueGen},
    JNIEnv,
};

use transport::{StreamBufferInfo, StreamKind};

#[allow(unused)]
pub trait TransformObject: Sized {
    fn from_object(env: &mut JNIEnv, object: &JObject) -> Result<Self> {
        unimplemented!()
    }

    fn to_object<'a>(&self, env: &mut JNIEnv<'a>) -> Result<JObject<'a>> {
        unimplemented!()
    }
}

#[allow(unused)]
pub trait TransformArray: Sized {
    fn to_array<'a>(&self, env: &mut JNIEnv<'a>) -> Result<JObjectArray<'a>> {
        unimplemented!()
    }
}

#[allow(unused)]
pub trait EasyObject<'a>
where
    Self: AsRef<JObject<'a>>,
{
    fn get_string(&self, env: &mut JNIEnv, key: &str) -> Result<String> {
        if let JValueGen::Object(value) = env.get_field(&self, key, "Ljava/lang/String;")? {
            Ok(env.get_string(&JString::from(value))?.into())
        } else {
            Err(anyhow!("[{}] not a string", key))
        }
    }

    fn set_string(&self, env: &mut JNIEnv, key: &str, value: &str) -> Result<()> {
        let value = env.new_string(value)?;
        self.set_object(env, "Ljava/lang/String;", key, value.as_ref())
    }

    fn get_int(&self, env: &mut JNIEnv, key: &str) -> Result<i32> {
        if let JValueGen::Int(value) = env.get_field(&self, key, "I")? {
            Ok(value)
        } else {
            Err(anyhow!("[{}] not a int", key))
        }
    }

    fn set_int(&self, env: &mut JNIEnv, key: &str, value: i32) -> Result<()> {
        env.set_field(&self, key, "I", JValueGen::Int(value))?;
        Ok(())
    }

    fn get_long(&self, env: &mut JNIEnv, key: &str) -> Result<i64> {
        if let JValueGen::Long(value) = env.get_field(&self, key, "J")? {
            Ok(value)
        } else {
            Err(anyhow!("[{}] not a long", key))
        }
    }

    fn set_long(&self, env: &mut JNIEnv, key: &str, value: i64) -> Result<()> {
        env.set_field(&self, key, "J", JValueGen::Long(value))?;
        Ok(())
    }

    fn get_object<'b>(&self, env: &mut JNIEnv<'b>, ty: &str, key: &str) -> Result<JObject<'b>> {
        if let JValueGen::Object(value) = env.get_field(&self, key, ty)? {
            Ok(value)
        } else {
            Err(anyhow!("[{}] not a object", key))
        }
    }

    fn set_object<'b>(
        &self,
        env: &mut JNIEnv<'b>,
        ty: &str,
        key: &str,
        value: &JObject,
    ) -> Result<()> {
        env.set_field(&self, key, ty, JValueGen::Object(value))?;
        Ok(())
    }
}

impl<'a> EasyObject<'a> for JObject<'a> {}

// ```kt
// /**
//  * transport strategy
//  */
// data class TransportStrategy(
//     /**
//      * STRATEGY_DIRECT | STRATEGY_RELAY | STRATEGY_MULTICAST
//      */
//     val type: Int,
//     /**
//      * socket address
//      */
//     val addr: String
// )
// ```
impl TransformObject for TransportStrategy {
    fn from_object(env: &mut JNIEnv, object: &JObject) -> Result<Self> {
        let addr: SocketAddr = object.get_string(env, "addr")?.parse()?;

        Ok(match object.get_int(env, "type")? {
            0 => Self::Direct(addr),
            1 => Self::Relay(addr),
            2 => Self::Multicast(addr),
            _ => return Err(anyhow!("type of invalidity")),
        })
    }

    fn to_object<'a>(&self, env: &mut JNIEnv<'a>) -> Result<JObject<'a>> {
        let object = env.alloc_object("Lcom/github/mycrl/hylarana/TransportStrategy;")?;
        let (addr, kind) = match self {
            Self::Direct(addr) => (addr, 0),
            Self::Relay(addr) => (addr, 1),
            Self::Multicast(addr) => (addr, 2),
        };

        object.set_string(env, "addr", &addr.to_string())?;
        object.set_int(env, "type", kind)?;
        Ok(object)
    }
}

// ```kt
// data class TransportOptions(
//     val strategy: TransportStrategy,
//     /**
//      * see: [Maximum_transmission_unit](https://en.wikipedia.org/wiki/Maximum_transmission_unit)
//      */
//     val mtu: Int
// )
// ```
impl TransformObject for TransportOptions {
    fn from_object(env: &mut JNIEnv, object: &JObject) -> Result<Self> {
        let strategy = object.get_object(
            env,
            "Lcom/github/mycrl/hylarana/TransportStrategy;",
            "strategy",
        )?;

        Ok(Self {
            strategy: TransportStrategy::from_object(env, &strategy)?,
            mtu: object.get_int(env, "mtu")? as usize,
        })
    }

    fn to_object<'a>(&self, env: &mut JNIEnv<'a>) -> Result<JObject<'a>> {
        let object = env.alloc_object("Lcom/github/mycrl/hylarana/TransportOptions;")?;
        object.set_int(env, "mtu", self.mtu as i32)?;

        let strategy = self.strategy.to_object(env)?;
        object.set_object(
            env,
            "Lcom/github/mycrl/hylarana/TransportStrategy;",
            "strategy",
            &strategy,
        )?;

        Ok(object)
    }
}

// ```kt
// /**
//  * STREAM_TYPE_VIDEO | STREAM_TYPE_AUDIO
//  */
// data class StreamBufferInfo(val type: Int) {
//     var flags: Int = 0
//     var timestamp: Long = 0
// }
// ```
impl TransformObject for StreamBufferInfo {
    fn from_object(env: &mut JNIEnv, object: &JObject) -> Result<Self> {
        let flags = object.get_int(env, "flags")?;
        let timestamp = object.get_long(env, "timestamp")? as u64;

        Ok(
            match StreamKind::try_from(object.get_int(env, "type")? as u8)
                .map_err(|_| anyhow!("type unreachable"))?
            {
                StreamKind::Video => Self::Video(flags, timestamp),
                StreamKind::Audio => Self::Audio(flags, timestamp),
            },
        )
    }
}

impl TransformArray for Vec<Ipv4Addr> {
    fn to_array<'a>(&self, env: &mut JNIEnv<'a>) -> Result<JObjectArray<'a>> {
        let array =
            env.new_object_array(self.len() as i32, "java/lang/String", JString::default())?;

        for (i, item) in self.iter().enumerate() {
            env.set_object_array_element(&array, i as i32, env.new_string(&item.to_string())?)?;
        }

        Ok(array)
    }
}

// ```kt
// data class MediaVideoStreamDescription(
//     val format: Int,
//     val width: Int,
//     val height: Int,
//     val fps: Int,
//     val bitRate: Int,
// )
// ```
impl TransformObject for MediaVideoStreamDescription {
    fn from_object(env: &mut JNIEnv, object: &JObject) -> Result<Self> {
        Ok(Self {
            fps: object.get_int(env, "fps")? as u8,
            bit_rate: object.get_int(env, "bitRate")? as u64,
            format: unsafe { std::mem::transmute(object.get_int(env, "format")?) },
            size: Size {
                width: object.get_int(env, "width")? as u32,
                height: object.get_int(env, "height")? as u32,
            },
        })
    }

    fn to_object<'a>(&self, env: &mut JNIEnv<'a>) -> Result<JObject<'a>> {
        let object = env.alloc_object("Lcom/github/mycrl/hylarana/MediaVideoStreamDescription;")?;
        object.set_int(env, "fps", self.fps as i32)?;
        object.set_int(env, "format", self.format as i32)?;
        object.set_int(env, "width", self.size.width as i32)?;
        object.set_int(env, "height", self.size.height as i32)?;
        object.set_int(env, "bitRate", self.bit_rate as i32)?;

        Ok(object)
    }
}

// ```kt
// data class MediaAudioStreamDescription(
//     val sampleRate: Int,
//     val channels: Int,
//     val bitRate: Int,
// )
// ```
impl TransformObject for MediaAudioStreamDescription {
    fn from_object(env: &mut JNIEnv, object: &JObject) -> Result<Self> {
        Ok(Self {
            sample_rate: object.get_int(env, "sampleRate")? as u64,
            bit_rate: object.get_int(env, "bitRate")? as u64,
            channels: object.get_int(env, "channels")? as u8,
        })
    }

    fn to_object<'a>(&self, env: &mut JNIEnv<'a>) -> Result<JObject<'a>> {
        let object = env.alloc_object("Lcom/github/mycrl/hylarana/MediaAudioStreamDescription;")?;
        object.set_int(env, "sampleRate", self.sample_rate as i32)?;
        object.set_int(env, "channels", self.channels as i32)?;
        object.set_int(env, "bitRate", self.bit_rate as i32)?;

        Ok(object)
    }
}

// ```kt
// data class MediaStreamDescription(
//     val id: String,
//     val transport: TransportOptions,
//     val video: MediaVideoStreamDescription?,
//     val audio: MediaAudioStreamDescription?,
// )
// ```
impl TransformObject for MediaStreamDescription {
    fn from_object(env: &mut JNIEnv, object: &JObject) -> Result<Self> {
        let transport = object.get_object(
            env,
            "Lcom/github/mycrl/hylarana/TransportOptions;",
            "transport",
        )?;

        Ok(Self {
            id: object.get_string(env, "id")?,
            transport: TransportOptions::from_object(env, &transport)?,
            video: object
                .get_object(
                    env,
                    "Lcom/github/mycrl/hylarana/MediaVideoStreamDescription;",
                    "video",
                )
                .ok()
                .map(|it| MediaVideoStreamDescription::from_object(env, &it).ok())
                .flatten(),
            audio: object
                .get_object(
                    env,
                    "Lcom/github/mycrl/hylarana/MediaAudioStreamDescription;",
                    "audio",
                )
                .ok()
                .map(|it| MediaAudioStreamDescription::from_object(env, &it).ok())
                .flatten(),
        })
    }

    fn to_object<'a>(&self, env: &mut JNIEnv<'a>) -> Result<JObject<'a>> {
        let object = env.alloc_object("Lcom/github/mycrl/hylarana/MediaStreamDescription;")?;
        object.set_string(env, "id", &self.id)?;

        let transport = self.transport.to_object(env)?;
        object.set_object(
            env,
            "Lcom/github/mycrl/hylarana/TransportOptions;",
            "transport",
            &transport,
        )?;

        if let Some(it) = self.video {
            let video = it.to_object(env)?;
            object.set_object(
                env,
                "Lcom/github/mycrl/hylarana/MediaVideoStreamDescription;",
                "video",
                &video,
            )?;
        }

        if let Some(it) = self.audio {
            let audio = it.to_object(env)?;
            object.set_object(
                env,
                "Lcom/github/mycrl/hylarana/MediaAudioStreamDescription;",
                "audio",
                &audio,
            )?;
        }

        Ok(object)
    }
}
