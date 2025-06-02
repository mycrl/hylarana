use std::sync::{
    Arc,
    atomic::{AtomicU8, AtomicU32, Ordering},
};

use arc_swap::ArcSwapOption;
use bytes::{Bytes, BytesMut};
use smallvec::SmallVec;

use crate::{Buffer, BufferType, StreamType};

#[derive(Default)]
struct Configs {
    video: ArcSwapOption<BytesMut>,
    audio: ArcSwapOption<BytesMut>,
}

/// Video Audio Streaming Send Processing
///
/// Because the receiver will normally join the stream in the middle of the
/// stream, and in the face of this situation, it is necessary to process the
/// sps and pps as well as the key frame information.
#[derive(Default)]
pub struct StreamProducer {
    audio_count: AtomicU8,
    sequence: AtomicU32,
    configs: Configs,
}

impl StreamProducer {
    const AUDIO_INTERVAL: u8 = 50;

    // h264 decoding any p-frames and i-frames requires sps and pps
    // frames, so the configuration frames are saved here, although it
    // should be noted that the configuration frames will only be
    // generated once.
    pub fn filter(&self, buffer: Buffer<BytesMut>) -> SmallVec<[Bytes; 2]> {
        let mut pkts: SmallVec<[Bytes; 2]> = SmallVec::with_capacity(5);

        match buffer.stream {
            StreamType::Video => {
                if buffer.ty == BufferType::Config {
                    self.configs
                        .video
                        .store(Some(Arc::new(buffer.data.clone())));
                }

                // Add SPS and PPS units in front of each keyframe (only use android)
                if buffer.ty == BufferType::KeyFrame {
                    if let Some(cfg) = self.configs.video.load().as_ref() {
                        pkts.push(
                            Buffer {
                                data: cfg.as_ref().clone(),
                                stream: StreamType::Video,
                                ty: BufferType::Config,
                                timestamp: buffer.timestamp,
                            }
                            .encode(self.sequence.fetch_add(1, Ordering::Relaxed)),
                        );
                    }
                }

                pkts.push(buffer.encode(self.sequence.fetch_add(1, Ordering::Relaxed)));
            }
            StreamType::Audio => {
                if buffer.ty == BufferType::Config {
                    self.configs
                        .audio
                        .store(Some(Arc::new(buffer.data.clone())));
                }

                // Insert a configuration package into every 30 audio packages.
                if self.audio_count.fetch_add(1, Ordering::Relaxed) == Self::AUDIO_INTERVAL {
                    self.audio_count.store(0, Ordering::Relaxed);

                    if let Some(cfg) = self.configs.audio.load().as_ref() {
                        pkts.push(
                            Buffer {
                                data: cfg.as_ref().clone(),
                                stream: StreamType::Audio,
                                ty: BufferType::Config,
                                timestamp: buffer.timestamp,
                            }
                            .encode(0),
                        );
                    }
                }

                pkts.push(buffer.encode(0));
            }
        }

        pkts
    }
}

struct PacketFilter {
    ty: StreamType,
    initialized: bool,
    readable: bool,
}

impl PacketFilter {
    fn new(ty: StreamType) -> Self {
        Self {
            initialized: false,
            readable: false,
            ty,
        }
    }

    fn filter(&mut self, ty: BufferType) -> bool {
        // First check whether the decoder has been initialized. Here, it is judged
        // whether the configuration information has consumer. If the configuration
        // information has consumer, the decoder initialization is marked as completed.
        if !self.initialized {
            if ty != BufferType::Config {
                return false;
            }

            self.initialized = true;
            return true;
        }

        // The configuration information only needs to be filled into the decoder once.
        // If it has been initialized, it means that the configuration information has
        // been received. It is meaningless to receive it again later. Here, duplicate
        // configuration information is filtered out.
        if self.ty == StreamType::Audio && ty == BufferType::Config {
            return false;
        }

        // The audio does not have keyframes
        if self.ty == StreamType::Video {
            // Check whether the current stream is in a readable state. When packet loss
            // occurs, the entire stream should be paused and wait for the next key frame to
            // arrive.
            if !self.readable {
                if ty == BufferType::KeyFrame {
                    self.readable = true;
                } else {
                    return false;
                }
            }
        }

        true
    }

    fn pkt_loss(&mut self) {
        self.readable = false;
    }
}

/// Video Audio Streaming Receiver Processing
///
/// The main purpose is to deal with cases where packet loss occurs at the
/// receiver side, since the SRT communication protocol does not completely
/// guarantee no packet loss.
pub struct StreamConsumer {
    last_sequence: Option<u32>,
    video: PacketFilter,
    audio: PacketFilter,
}

impl Default for StreamConsumer {
    fn default() -> Self {
        Self {
            video: PacketFilter::new(StreamType::Video),
            audio: PacketFilter::new(StreamType::Audio),
            last_sequence: None,
        }
    }
}

impl StreamConsumer {
    /// As soon as a keyframe is received, the keyframe is cached, and when a
    /// packet loss occurs, the previous keyframe is retransmitted directly into
    /// the decoder.
    pub fn filter(&mut self, bytes: Bytes) -> Option<Buffer<Bytes>> {
        // Decode the data packet to get sequence number and buffer information
        let (sequence, buffer) = Buffer::<Bytes>::decode(bytes).ok()?;

        match buffer.stream {
            StreamType::Video => {
                // If there is a previous sequence number, perform packet loss detection
                if let Some(last) = self.last_sequence.replace(sequence) {
                    // Check if sequence numbers are consecutive, if not, packet loss is detected
                    if sequence != last.wrapping_add(1) {
                        // Mark video stream as unreadable and wait for next keyframe
                        self.video.pkt_loss();

                        log::warn!("packet loss occurs at the transport layer");

                        return None;
                    }
                }

                // Filter packets based on their type
                if self.video.filter(buffer.ty) {
                    return Some(buffer);
                }
            }
            StreamType::Audio => {
                // Audio stream only needs type-based filtering
                if self.audio.filter(buffer.ty) {
                    return Some(buffer);
                }
            }
        }

        None
    }
}
