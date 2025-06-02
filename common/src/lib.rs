pub mod codec;
pub mod frame;
pub mod logger;
pub mod runtime;
pub mod strings;

#[cfg(target_os = "windows")]
pub mod win32;

#[cfg(target_os = "macos")]
pub mod macos;

use frame::VideoFormat;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct MediaVideoStreamDescription {
    pub format: VideoFormat,
    pub size: Size,
    pub fps: u8,
    pub bit_rate: u64,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct MediaAudioStreamDescription {
    pub sample_rate: u64,
    pub channels: u8,
    pub bit_rate: u64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct MediaStreamDescription {
    pub video: Option<MediaVideoStreamDescription>,
    pub audio: Option<MediaAudioStreamDescription>,
}
