use std::{
    fs,
    net::{Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

use anyhow::Result;
use hylarana::{VideoDecoderType, VideoEncoderType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct System {
    pub name: String,
    pub language: String,
}

impl Default for System {
    fn default() -> Self {
        Self {
            language: "english".to_string(),
            name: {
                dirs::home_dir()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .split("/")
                    .last()
                    .unwrap()
                    .to_string()
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Network {
    pub interface: Ipv4Addr,
    pub multicast: Ipv4Addr,
    pub server: Option<SocketAddr>,
    pub port: u32,
    pub mtu: u32,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            interface: "0.0.0.0".parse().unwrap(),
            multicast: "239.0.0.1".parse().unwrap(),
            server: None,
            port: 8080,
            mtu: 1400,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Codec {
    pub encoder: VideoEncoderType,
    pub decoder: VideoDecoderType,
}

impl Default for Codec {
    fn default() -> Self {
        Self {
            encoder: if cfg!(target_os = "macos") {
                VideoEncoderType::VideoToolBox
            } else {
                VideoEncoderType::X264
            },
            decoder: if cfg!(target_os = "macos") {
                VideoDecoderType::VideoToolBox
            } else {
                VideoDecoderType::H264
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Video {
    pub width: u32,
    pub height: u32,
    pub frame_rate: u8,
    pub bit_rate: usize,
    pub key_frame_interval: u8,
}

impl Default for Video {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            frame_rate: 30,
            bit_rate: 10000000,
            key_frame_interval: 30,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Audio {
    pub sample_rate: u32,
    pub bit_rate: usize,
}

impl Default for Audio {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            bit_rate: 64000,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Configure {
    pub network: Network,
    pub system: System,
    pub codec: Codec,
    pub video: Video,
    pub audio: Audio,
}

pub struct Settings {
    path: PathBuf,
    value: Configure,
}

impl Settings {
    pub fn new() -> Result<Self> {
        let path = Path::new(&crate::APP_CONFIG.cache_path).join("./settings.json");
        Ok(Self {
            value: if fs::exists(&path).unwrap_or(false) {
                serde_json::from_str(&fs::read_to_string(&path)?)?
            } else {
                let value = Configure::default();
                fs::write(&path, serde_json::to_string(&value)?)?;
                value
            },
            path,
        })
    }

    pub fn get(&self) -> &Configure {
        &self.value
    }

    pub fn set(&mut self, value: Configure) -> Result<()> {
        fs::write(&self.path, serde_json::to_string(&value)?)?;
        self.value = value;
        Ok(())
    }
}
