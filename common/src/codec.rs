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
    H264,
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
            Self::H264 => "h264",
            Self::D3D11 => "d3d11va",
            Self::Qsv => "h264_qsv",
            Self::VideoToolBox => "h264_videotoolbox",
        }
        .to_string()
    }
}

impl FromStr for VideoDecoderType {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "h264" => Self::H264,
            "d3d11va" => Self::D3D11,
            "h264_qsv" => Self::Qsv,
            "h264_videotoolbox" => Self::VideoToolBox,
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
    X264,
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
            Self::X264 => "libx264",
            Self::Qsv => "h264_qsv",
            Self::VideoToolBox => "h264_videotoolbox",
        }
        .to_string()
    }
}

impl FromStr for VideoEncoderType {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "libx264" => Self::X264,
            "h264_qsv" => Self::Qsv,
            "h264_videotoolbox" => Self::VideoToolBox,
            _ => return Err(Error::new(ErrorKind::InvalidInput, value)),
        })
    }
}
