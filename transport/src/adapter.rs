use std::{
    fmt,
    sync::{
        atomic::{AtomicBool, AtomicU8},
        mpsc::{Receiver, Sender, channel},
    },
};

use bytes::{Bytes, BytesMut};
use common::atomic::{AtomicOption, EasyAtomic};
use parking_lot::Mutex;

struct Channel<T>(Sender<Option<T>>, Mutex<Receiver<Option<T>>>);

impl<T> Default for Channel<T> {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self(tx, Mutex::new(rx))
    }
}

impl<T> Channel<T> {
    fn send(&self, item: Option<T>) -> bool {
        self.0.send(item).is_ok()
    }

    fn recv(&self) -> Option<T> {
        self.1.lock().recv().ok().flatten()
    }
}

#[derive(Default)]
struct PacketFilter {
    initialized: AtomicBool,
    readable: AtomicBool,
}

impl PacketFilter {
    fn filter(&self, flag: i32, keyframe: bool) -> bool {
        // First check whether the decoder has been initialized. Here, it is judged
        // whether the configuration information has consumer. If the configuration
        // information has consumer, the decoder initialization is marked as completed.
        if !self.initialized.get() {
            if flag != BufferFlag::Config as i32 {
                return false;
            }

            self.initialized.set(true);
            return true;
        }

        // The configuration information only needs to be filled into the decoder once.
        // If it has been initialized, it means that the configuration information has
        // been received. It is meaningless to receive it again later. Here, duplicate
        // configuration information is filtered out.
        if flag == BufferFlag::Config as i32 {
            return false;
        }

        // The audio does not have keyframes
        if keyframe {
            // Check whether the current stream is in a readable state. When packet loss
            // occurs, the entire stream should be paused and wait for the next key frame to
            // arrive.
            if !self.readable.get() {
                if flag == BufferFlag::KeyFrame as i32 {
                    self.readable.set(true);
                } else {
                    return false;
                }
            }
        }

        true
    }

    fn loss(&self) {
        self.readable.set(false);
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferFlag {
    KeyFrame = 1,
    Config = 2,
    EndOfStream = 4,
    Partial = 8,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    Video = 0,
    Audio = 1,
}

#[derive(Debug, Clone, Copy)]
pub struct StreamKindTryFromError;

impl std::error::Error for StreamKindTryFromError {}

impl fmt::Display for StreamKindTryFromError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StreamKindTryFromError")
    }
}

impl TryFrom<u8> for StreamKind {
    type Error = StreamKindTryFromError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Video,
            1 => Self::Audio,
            _ => return Err(StreamKindTryFromError),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamBufferInfo {
    Video(i32, u64),
    Audio(i32, u64),
}

#[derive(Default)]
struct ConfigCache {
    video: AtomicOption<BytesMut>,
    audio: AtomicOption<BytesMut>,
}

#[derive(Default)]
struct AutoInsertOfConfigInfo {
    audio: AtomicU8,
}

impl AutoInsertOfConfigInfo {
    const AUDIO_INTERVAL: u8 = 30;
}

/// Video Audio Streaming Send Processing
///
/// Because the receiver will normally join the stream in the middle of the
/// stream, and in the face of this situation, it is necessary to process the
/// sps and pps as well as the key frame information.
#[derive(Default)]
pub struct StreamSenderAdapter {
    channel: Channel<(BytesMut, StreamKind, i32, u64)>,
    aioci: AutoInsertOfConfigInfo,
    config: ConfigCache,
}

impl StreamSenderAdapter {
    pub(crate) fn close(&self) {
        self.channel.send(None);
    }

    // h264 decoding any p-frames and i-frames requires sps and pps
    // frames, so the configuration frames are saved here, although it
    // should be noted that the configuration frames will only be
    // generated once.
    pub fn send(&self, buf: BytesMut, info: StreamBufferInfo) -> bool {
        if buf.is_empty() {
            return true;
        }

        match info {
            StreamBufferInfo::Video(flags, timestamp) => {
                if flags == BufferFlag::Config as i32 {
                    self.config.video.swap(Some(buf.clone()));
                }

                // Add SPS and PPS units in front of each keyframe (only use android)
                if flags == BufferFlag::KeyFrame as i32 {
                    if let Some(config) = self.config.video.get() {
                        if !self.channel.send(Some((
                            config.clone(),
                            StreamKind::Video,
                            BufferFlag::Config as i32,
                            timestamp,
                        ))) {
                            return false;
                        }
                    }
                }

                self.channel
                    .send(Some((buf, StreamKind::Video, flags, timestamp)))
            }
            StreamBufferInfo::Audio(flags, timestamp) => {
                if flags == BufferFlag::Config as i32 {
                    self.config.audio.swap(Some(buf.clone()));
                }

                // Insert a configuration package into every 30 audio packages.
                let count = self.aioci.audio.get();
                self.aioci
                    .audio
                    .set(if count == AutoInsertOfConfigInfo::AUDIO_INTERVAL {
                        if let Some(config) = self.config.audio.get() {
                            if !self.channel.send(Some((
                                config.clone(),
                                StreamKind::Audio,
                                BufferFlag::Config as i32,
                                timestamp,
                            ))) {
                                return false;
                            }
                        }

                        0
                    } else {
                        count + 1
                    });

                self.channel
                    .send(Some((buf, StreamKind::Audio, flags, timestamp)))
            }
        }
    }

    pub fn next(&self) -> Option<(BytesMut, StreamKind, i32, u64)> {
        self.channel.recv()
    }
}

pub trait StreamReceiverAdapterAbstract: Sync + Send {
    fn send(&self, buf: Bytes, kind: StreamKind, flags: i32, timestamp: u64) -> bool;
    fn close(&self);
    fn lose(&self);
}

#[derive(Default)]
struct Filter {
    video: PacketFilter,
    audio: PacketFilter,
}

/// Video Audio Streaming Receiver Processing
///
/// The main purpose is to deal with cases where packet loss occurs at the
/// receiver side, since the SRT communication protocol does not completely
/// guarantee no packet loss.
#[derive(Default)]
pub struct StreamReceiverAdapter {
    channel: Channel<(Bytes, StreamKind, i32, u64)>,
    filter: Filter,
}

impl StreamReceiverAdapter {
    pub fn next(&self) -> Option<(Bytes, StreamKind, i32, u64)> {
        self.channel.recv()
    }
}

impl StreamReceiverAdapterAbstract for StreamReceiverAdapter {
    fn close(&self) {
        self.channel.send(None);
    }

    fn lose(&self) {
        self.filter.video.loss();

        log::warn!(
            "Packet loss has occurred and the data stream is currently paused, waiting for the key frame to arrive.",
        );
    }

    /// As soon as a keyframe is received, the keyframe is cached, and when a
    /// packet loss occurs, the previous keyframe is retransmitted directly into
    /// the decoder.
    fn send(&self, buf: Bytes, kind: StreamKind, flags: i32, timestamp: u64) -> bool {
        if buf.is_empty() {
            return true;
        }

        if match kind {
            StreamKind::Video => self.filter.video.filter(flags, true),
            StreamKind::Audio => self.filter.audio.filter(flags, false),
        } {
            return self.channel.send(Some((buf, kind, flags, timestamp)));
        }

        true
    }
}

#[derive(Default)]
struct MultiChannels {
    video: Channel<(Bytes, i32, u64)>,
    audio: Channel<(Bytes, i32, u64)>,
}

/// Video Audio Streaming Receiver Processing
///
/// The main purpose is to deal with cases where packet loss occurs at the
/// receiver side, since the SRT communication protocol does not completely
/// guarantee no packet loss.
#[derive(Default)]
pub struct StreamMultiReceiverAdapter {
    channel: MultiChannels,
    filter: Filter,
}

impl StreamMultiReceiverAdapter {
    pub fn next(&self, kind: StreamKind) -> Option<(Bytes, i32, u64)> {
        match kind {
            StreamKind::Video => self.channel.video.recv(),
            StreamKind::Audio => self.channel.audio.recv(),
        }
    }
}

impl StreamReceiverAdapterAbstract for StreamMultiReceiverAdapter {
    fn close(&self) {
        self.channel.video.send(None);
        self.channel.audio.send(None);
    }

    fn lose(&self) {
        self.filter.video.loss();

        log::warn!(
            "Packet loss has occurred and the data stream is currently paused, waiting for the key frame to arrive.",
        );
    }

    /// As soon as a keyframe is received, the keyframe is cached, and when a
    /// packet loss occurs, the previous keyframe is retransmitted directly into
    /// the decoder.
    fn send(&self, buf: Bytes, kind: StreamKind, flags: i32, timestamp: u64) -> bool {
        if buf.is_empty() {
            return true;
        }

        match kind {
            StreamKind::Video => {
                if self.filter.video.filter(flags, true) {
                    return self.channel.video.send(Some((buf, flags, timestamp)));
                }
            }
            StreamKind::Audio => {
                if self.filter.audio.filter(flags, false) {
                    return self.channel.audio.send(Some((buf, flags, timestamp)));
                }
            }
        }

        true
    }
}
