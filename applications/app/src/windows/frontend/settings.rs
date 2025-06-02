use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use anyhow::Result;
use hylarana::{TransportOptions, VideoDecoderType, VideoEncoderType};
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
            name: crate::APP_CONFIG.username.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Network {
    pub bind: SocketAddr,
    /// Maximum Transmission Unit size
    pub mtu: u32,
    // Maximum bandwidth in bytes per second
    pub max_bandwidth: i64,
    // Latency in milliseconds
    pub latency: u32,
    // Connection timeout in milliseconds
    pub timeout: u32,
    // Forward Error Correction configuration
    pub fec: String,
    // Flow control window size
    pub fc: u32,
}

impl Default for Network {
    fn default() -> Self {
        let opt = TransportOptions::default();

        Self {
            bind: "0.0.0.0:43165".parse().unwrap(),
            max_bandwidth: opt.max_bandwidth,
            latency: opt.latency,
            timeout: opt.timeout,
            fec: opt.fec,
            mtu: opt.mtu,
            fc: opt.fc,
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
                VideoEncoderType::X265
            },
            decoder: if cfg!(target_os = "macos") {
                VideoDecoderType::VideoToolBox
            } else {
                VideoDecoderType::HEVC
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
            bit_rate: 5_000_000,
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
            bit_rate: 64_000,
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
