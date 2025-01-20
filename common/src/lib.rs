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
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize)]
pub struct Size {
    #[serde(rename = "w")]
    pub width: u32,
    #[serde(rename = "h")]
    pub height: u32,
}

/// Transport layer strategies.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
    #[serde(rename = "d")]
    Direct(SocketAddr),
    /// Forwarding mode, where the sender and receiver pass data through a relay
    /// server.
    ///
    /// The network address is the address of the transit server.
    #[serde(rename = "r")]
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
    #[serde(rename = "m")]
    Multicast(SocketAddr),
}

/// Transport configuration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TransportOptions {
    #[serde(rename = "s")]
    pub strategy: TransportStrategy,
    /// see: [Maximum_transmission_unit](https://en.wikipedia.org/wiki/Maximum_transmission_unit)
    #[serde(rename = "m")]
    pub mtu: usize,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct MediaVideoStreamDescription {
    #[serde(rename = "f")]
    pub format: VideoFormat,
    #[serde(rename = "s")]
    pub size: Size,
    pub fps: u8,
    #[serde(rename = "br")]
    pub bit_rate: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct MediaAudioStreamDescription {
    #[serde(rename = "sr")]
    pub sample_rate: u64,
    #[serde(rename = "cs")]
    pub channels: u8,
    #[serde(rename = "br")]
    pub bit_rate: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaStreamDescription {
    #[serde(rename = "i")]
    pub id: String,
    #[serde(rename = "t")]
    pub transport: TransportOptions,
    #[serde(rename = "v")]
    pub video: Option<MediaVideoStreamDescription>,
    #[serde(rename = "a")]
    pub audio: Option<MediaAudioStreamDescription>,
}
