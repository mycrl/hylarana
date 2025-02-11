pub mod atomic;
pub mod codec;
pub mod frame;
pub mod logger;
pub mod strings;

#[cfg(target_os = "windows")]
pub mod win32;

#[cfg(target_os = "macos")]
pub mod macos;

use std::net::SocketAddr;

use frame::VideoFormat;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Size {
    #[cfg_attr(feature = "serde-short", serde(rename = "w"))]
    pub width: u32,
    #[cfg_attr(feature = "serde-short", serde(rename = "h"))]
    pub height: u32,
}

/// Transport layer strategies.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde-short", serde(tag = "t", content = "v"))]
pub enum TransportStrategy {
    /// In straight-through mode, the sender creates an SRT server and the
    /// receiver connects directly to the sender via the SRT protocol.
    ///
    /// For the sender, the network address is the address to which the SRT
    /// server binds and listens.
    ///
    /// ```text
    /// example: 0.0.0.0:8080
    /// ```
    ///
    /// For the receiving end, the network address is the address of the SRT
    /// server on the sending end.
    ///
    /// ```text
    /// example: 192.168.1.100:8080
    /// ```
    #[cfg_attr(feature = "serde-short", serde(rename = "d"))]
    Direct(SocketAddr),
    /// Forwarding mode, where the sender and receiver pass data through a relay
    /// server.
    ///
    /// The network address is the address of the transit server.
    #[cfg_attr(feature = "serde-short", serde(rename = "r"))]
    Relay(SocketAddr),
    /// UDP multicast mode, where the sender sends multicast packets into the
    /// current network and the receiver processes the multicast packets.
    ///
    /// The sender and receiver use the same address, which is a combination of
    /// multicast address + port.
    ///
    /// ```text
    /// example: 239.0.0.1:8080
    /// ```
    #[cfg_attr(feature = "serde-short", serde(rename = "m"))]
    Multicast(SocketAddr),
}

/// Transport configuration.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct TransportOptions {
    #[cfg_attr(feature = "serde-short", serde(rename = "s"))]
    pub strategy: TransportStrategy,
    /// see: [Maximum_transmission_unit](https://en.wikipedia.org/wiki/Maximum_transmission_unit)
    #[cfg_attr(feature = "serde-short", serde(rename = "m"))]
    pub mtu: usize,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct MediaVideoStreamDescription {
    #[cfg_attr(feature = "serde-short", serde(rename = "f"))]
    pub format: VideoFormat,
    #[cfg_attr(feature = "serde-short", serde(rename = "s"))]
    pub size: Size,
    #[cfg_attr(feature = "serde-short", serde(rename = "p"))]
    pub fps: u8,
    #[cfg_attr(feature = "serde-short", serde(rename = "b"))]
    pub bit_rate: u64,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct MediaAudioStreamDescription {
    #[cfg_attr(feature = "serde-short", serde(rename = "s"))]
    pub sample_rate: u64,
    #[cfg_attr(feature = "serde-short", serde(rename = "c"))]
    pub channels: u8,
    #[cfg_attr(feature = "serde-short", serde(rename = "b"))]
    pub bit_rate: u64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct MediaStreamDescription {
    #[cfg_attr(feature = "serde-short", serde(rename = "i"))]
    pub id: String,
    #[cfg_attr(feature = "serde-short", serde(rename = "t"))]
    pub transport: TransportOptions,
    #[cfg_attr(feature = "serde-short", serde(rename = "v"))]
    pub video: Option<MediaVideoStreamDescription>,
    #[cfg_attr(feature = "serde-short", serde(rename = "a"))]
    pub audio: Option<MediaAudioStreamDescription>,
}
