use std::{
    io::{Error, ErrorKind},
    str::FromStr,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Video decoder type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum VideoDecoderType {
    /// [Open H264](https://www.openh264.org/)
    ///
    /// OpenH264 is a codec library which supports H.264 encoding and decoding.
    HEVC,
    /// [D3D11VA](https://learn.microsoft.com/en-us/windows/win32/medfound/direct3d-11-video-apis)
    ///
    /// Accelerated video decoding using Direct3D 11 Video APIs.
    D3D11,
    /// [H264 QSV](https://en.wikipedia.org/wiki/Intel_Quick_Sync_Video)
    ///
    /// Intel Quick Sync Video is Intel's brand for its dedicated video encoding
    /// and decoding hardware core.
    Qsv,
    /// [Video Toolbox](https://developer.apple.com/documentation/videotoolbox)
    ///
    /// VideoToolbox is a low-level framework that provides direct access to
    /// hardware encoders and decoders.
    VideoToolBox,
}

impl ToString for VideoDecoderType {
    fn to_string(&self) -> String {
        match self {
            Self::HEVC => "hevc",
            Self::D3D11 => "d3d11va",
            Self::Qsv => "hevc_qsv",
            Self::VideoToolBox => "hevc_videotoolbox",
        }
        .to_string()
    }
}

impl FromStr for VideoDecoderType {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "hevc" => Self::HEVC,
            "d3d11va" => Self::D3D11,
            "hevc_qsv" => Self::Qsv,
            "hevc_videotoolbox" => Self::VideoToolBox,
            _ => return Err(Error::new(ErrorKind::InvalidInput, value)),
        })
    }
}

/// Video encoder type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum VideoEncoderType {
    /// [X264](https://www.videolan.org/developers/x264.html)
    ///
    /// x264 is a free software library and application for encoding video
    /// streams into the H.264/MPEG-4 AVC compression format, and is released
    /// under the terms of the GNU GPL.
    X265,
    /// [H264 QSV](https://en.wikipedia.org/wiki/Intel_Quick_Sync_Video)
    ///
    /// Intel Quick Sync Video is Intel's brand for its dedicated video encoding
    /// and decoding hardware core.
    Qsv,
    /// [Video Toolbox](https://developer.apple.com/documentation/videotoolbox)
    ///
    /// VideoToolbox is a low-level framework that provides direct access to
    /// hardware encoders and decoders.
    VideoToolBox,
}

impl ToString for VideoEncoderType {
    fn to_string(&self) -> String {
        match self {
            Self::X265 => "libx265",
            Self::Qsv => "hevc_qsv",
            Self::VideoToolBox => "hevc_videotoolbox",
        }
        .to_string()
    }
}

impl FromStr for VideoEncoderType {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "libx265" => Self::X265,
            "hevc_qsv" => Self::Qsv,
            "hevc_videotoolbox" => Self::VideoToolBox,
            _ => return Err(Error::new(ErrorKind::InvalidInput, value)),
        })
    }
}
