pub mod atomic;
pub mod codec;
pub mod frame;
pub mod logger;
pub mod runtime;
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
    pub width: u32,
    pub height: u32,
}

/// Transport layer strategies.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "ty", content = "address"))]
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
    Direct(SocketAddr),
    /// Forwarding mode, where the sender and receiver pass data through a relay
    /// server.
    ///
    /// The network address is the address of the transit server.
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
    Multicast(SocketAddr),
}

/// Transport configuration.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct TransportOptions {
    pub strategy: TransportStrategy,
    /// see: [Maximum_transmission_unit](https://en.wikipedia.org/wiki/Maximum_transmission_unit)
    pub mtu: usize,
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
    pub id: String,
    pub transport: TransportOptions,
    pub video: Option<MediaVideoStreamDescription>,
    pub audio: Option<MediaAudioStreamDescription>,
}
