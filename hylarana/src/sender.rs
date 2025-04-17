use super::{
    MediaAudioStreamDescription, MediaStreamDescription, MediaStreamObserver, MediaStreamSink,
    MediaVideoStreamDescription,
};

#[cfg(target_os = "windows")]
use super::util::get_direct3d;

use std::{
    mem::size_of,
    sync::{Arc, Weak, atomic::AtomicBool},
};

use bytes::BytesMut;
use capture::{
    AudioCaptureSourceDescription, Capture, CaptureOptions, FrameConsumer, Source,
    SourceCaptureOptions, VideoCaptureSourceDescription,
};

use common::{
    Size, TransportOptions,
    atomic::EasyAtomic,
    codec::VideoEncoderType,
    frame::{AudioFrame, VideoFormat, VideoFrame},
};

use codec::{
    AudioEncoder, AudioEncoderSettings, CodecType, VideoEncoder, VideoEncoderSettings,
    create_opus_identification_header,
};

use thiserror::Error;
use transport::{
    BufferFlag, StreamBufferInfo, StreamSenderAdapter, TransportSender,
    copy_from_slice as package_copy_from_slice,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Error)]
pub enum HylaranaSenderError {
    #[error(transparent)]
    TransportError(#[from] std::io::Error),
    #[error(transparent)]
    CaptureError(#[from] capture::CaptureError),
    #[error(transparent)]
    VideoEncoderError(#[from] codec::VideoEncoderError),
    #[error(transparent)]
    AudioEncoderError(#[from] codec::AudioEncoderError),
}

/// Description of video coding.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct VideoOptions {
    pub codec: VideoEncoderType,
    pub frame_rate: u8,
    pub width: u32,
    pub height: u32,
    pub bit_rate: u64,
    pub key_frame_interval: u32,
}

/// Description of the audio encoding.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct AudioOptions {
    pub sample_rate: u64,
    pub bit_rate: u64,
}

/// Options of the media track.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct HylaranaSenderTrackOptions<T> {
    pub source: Source,
    pub options: T,
}

/// Options of the media stream.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct HylaranaSenderMediaOptions {
    pub video: Option<HylaranaSenderTrackOptions<VideoOptions>>,
    pub audio: Option<HylaranaSenderTrackOptions<AudioOptions>>,
}

/// Sender configuration.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct HylaranaSenderOptions {
    pub media: HylaranaSenderMediaOptions,
    pub transport: TransportOptions,
}

struct VideoSender<S, O> {
    adapter: Arc<StreamSenderAdapter>,
    status: Arc<AtomicBool>,
    encoder: VideoEncoder,
    sink: Weak<S>,
    observer: Weak<O>,
}

// Encoding is a relatively complex task. If you add encoding tasks to the
// pipeline that pushes frames, it will slow down the entire pipeline.
//
// Here, the tasks are separated, and the encoding tasks are separated into
// independent threads. The encoding thread is notified of task updates through
// the optional lock.
impl<S, O> VideoSender<S, O>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    fn new(
        status: Arc<AtomicBool>,
        transport: &TransportSender,
        settings: VideoEncoderSettings,
        sink: &Arc<S>,
        observer: &Arc<O>,
    ) -> Result<Self, HylaranaSenderError> {
        Ok(Self {
            adapter: transport.get_adapter(),
            sink: Arc::downgrade(sink),
            observer: Arc::downgrade(observer),
            encoder: VideoEncoder::new(settings)?,
            status,
        })
    }

    fn send(&mut self, frame: &VideoFrame) -> bool {
        // Push the audio and video frames into the encoder.
        if self.encoder.update(frame) {
            // Try to get the encoded data packets. The audio and video frames do not
            // correspond to the data packets one by one, so you need to try to get
            // multiple packets until they are empty.
            if let Err(e) = self.encoder.encode() {
                log::error!("video encode error={:?}", e);

                return false;
            } else {
                while let Some((buffer, flags, timestamp)) = self.encoder.read() {
                    if !self.adapter.send(
                        package_copy_from_slice(buffer),
                        StreamBufferInfo::Video(flags, timestamp),
                    ) {
                        log::warn!("video send packet to adapter failed");

                        return false;
                    }
                }
            }
        } else {
            log::warn!("video encoder update frame failed");

            return false;
        }

        if let Some(sink) = self.sink.upgrade() {
            if sink.video(frame) {
                true
            } else {
                log::warn!("video sink on frame return false");

                false
            }
        } else {
            log::warn!("video sink weak upgrade failed, maybe is drop");

            false
        }
    }
}

impl<S, O> FrameConsumer for VideoSender<S, O>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    type Frame = VideoFrame;

    fn sink(&mut self, frame: &Self::Frame) -> bool {
        if self.send(frame) {
            true
        } else {
            if let Some(observer) = self.observer.upgrade() {
                if !self.status.get() {
                    self.status.set(true);
                    observer.close();
                }
            }

            false
        }
    }
}

struct AudioSender<S, O> {
    adapter: Arc<StreamSenderAdapter>,
    status: Arc<AtomicBool>,
    encoder: AudioEncoder,
    chunk_count: usize,
    buffer: BytesMut,
    sink: Weak<S>,
    observer: Weak<O>,
}

// Encoding is a relatively complex task. If you add encoding tasks to the
// pipeline that pushes frames, it will slow down the entire pipeline.
//
// Here, the tasks are separated, and the encoding tasks are separated into
// independent threads. The encoding thread is notified of task updates through
// the optional lock.
impl<S, O> AudioSender<S, O>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    fn new(
        status: Arc<AtomicBool>,
        transport: &TransportSender,
        settings: AudioEncoderSettings,
        sink: &Arc<S>,
        observer: &Arc<O>,
    ) -> Result<Self, HylaranaSenderError> {
        let adapter = transport.get_adapter();

        // Create an opus header data. The opus decoder needs this data to obtain audio
        // information. Here, actively add an opus header information to the queue, and
        // the adapter layer will automatically cache it.
        adapter.send(
            package_copy_from_slice(&create_opus_identification_header(
                2,
                settings.sample_rate as u32,
            )),
            StreamBufferInfo::Audio(BufferFlag::Config as i32, 0),
        );

        Ok(AudioSender {
            chunk_count: settings.sample_rate as usize / 1000 * 100 * 2,
            encoder: AudioEncoder::new(settings)?,
            buffer: BytesMut::with_capacity(48000 * 2),
            observer: Arc::downgrade(observer),
            sink: Arc::downgrade(sink),
            adapter,
            status,
        })
    }

    fn send(&mut self, frame: &AudioFrame) -> bool {
        self.buffer.extend_from_slice(unsafe {
            std::slice::from_raw_parts(
                frame.data as *const _,
                frame.frames as usize * 2 * size_of::<i16>(),
            )
        });

        if self.buffer.len() >= self.chunk_count * size_of::<i16>() {
            let payload = self.buffer.split_to(self.chunk_count * size_of::<i16>());
            let frame = AudioFrame {
                data: payload.as_ptr() as *const _,
                frames: self.chunk_count as u32 / 2,
                sample_rate: 0,
            };

            if self.encoder.update(&frame) {
                // Push the audio and video frames into the encoder.
                if let Err(e) = self.encoder.encode() {
                    log::error!("audio encode error={:?}", e);

                    return false;
                } else {
                    // Try to get the encoded data packets. The audio and video frames
                    // do not correspond to the data
                    // packets one by one, so you need to try to get
                    // multiple packets until they are empty.
                    while let Some((buffer, flags, timestamp)) = self.encoder.read() {
                        if !self.adapter.send(
                            package_copy_from_slice(buffer),
                            StreamBufferInfo::Audio(flags, timestamp),
                        ) {
                            log::warn!("audio send packet to adapter failed");

                            return false;
                        }
                    }
                }
            } else {
                log::warn!("audio encoder update frame failed");

                return false;
            }
        }

        if let Some(sink) = self.sink.upgrade() {
            if sink.audio(frame) {
                true
            } else {
                log::warn!("audio sink on frame return false");

                false
            }
        } else {
            log::warn!("audio sink weak upgrade failed, maybe is drop");

            false
        }
    }
}

impl<S, O> FrameConsumer for AudioSender<S, O>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    type Frame = AudioFrame;

    fn sink(&mut self, frame: &Self::Frame) -> bool {
        if self.send(frame) {
            true
        } else {
            if let Some(observer) = self.observer.upgrade() {
                if !self.status.get() {
                    self.status.set(true);
                    observer.close();
                }
            }

            false
        }
    }
}

/// Screen casting sender.
pub struct HylaranaSender<S, O>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    description: MediaStreamDescription,
    #[allow(unused)]
    transport: TransportSender,
    status: Arc<AtomicBool>,
    capture: Capture,
    #[allow(unused)]
    sink: Arc<S>,
    observer: Arc<O>,
}

impl<S, O> HylaranaSender<S, O>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    // Create a sender. The capture of the sender is started following the sender,
    // but both video capture and audio capture can be empty, which means you can
    // create a sender that captures nothing.
    pub(crate) fn new(
        options: &HylaranaSenderOptions,
        sink: S,
        observer: O,
    ) -> Result<Self, HylaranaSenderError> {
        log::info!("create sender");

        let mut capture_options = CaptureOptions::default();
        let transport = transport::create_sender(options.transport)?;
        let status = Arc::new(AtomicBool::new(false));
        let observer = Arc::new(observer);
        let sink = Arc::new(sink);

        if let Some(HylaranaSenderTrackOptions { source, options }) = &options.media.audio {
            capture_options.audio = Some(SourceCaptureOptions {
                consumer: AudioSender::new(
                    status.clone(),
                    &transport,
                    AudioEncoderSettings {
                        sample_rate: options.sample_rate,
                        bit_rate: options.bit_rate,
                    },
                    &sink,
                    &observer,
                )?,
                description: AudioCaptureSourceDescription {
                    sample_rate: options.sample_rate as u32,
                    source: source.clone(),
                },
            });
        }

        if let Some(HylaranaSenderTrackOptions { source, options }) = &options.media.video {
            capture_options.video = Some(SourceCaptureOptions {
                description: VideoCaptureSourceDescription {
                    hardware: CodecType::from(options.codec).is_hardware(),
                    fps: options.frame_rate,
                    size: Size {
                        width: options.width,
                        height: options.height,
                    },
                    source: source.clone(),
                    #[cfg(target_os = "windows")]
                    direct3d: get_direct3d(),
                },
                consumer: VideoSender::new(
                    status.clone(),
                    &transport,
                    VideoEncoderSettings {
                        codec: options.codec,
                        key_frame_interval: options.key_frame_interval,
                        frame_rate: options.frame_rate,
                        width: options.width,
                        height: options.height,
                        bit_rate: options.bit_rate,
                        #[cfg(target_os = "windows")]
                        direct3d: Some(get_direct3d()),
                    },
                    &sink,
                    &observer,
                )?,
            });
        }

        let description = MediaStreamDescription {
            id: transport.get_id().to_string(),
            transport: options.transport,
            video: options
                .media
                .video
                .clone()
                .map(|it| MediaVideoStreamDescription {
                    format: VideoFormat::NV12,
                    fps: it.options.frame_rate,
                    bit_rate: it.options.bit_rate,
                    size: Size {
                        width: it.options.width,
                        height: it.options.height,
                    },
                }),
            audio: options
                .media
                .audio
                .clone()
                .map(|it| MediaAudioStreamDescription {
                    sample_rate: it.options.sample_rate,
                    bit_rate: it.options.bit_rate,
                    channels: 2,
                }),
        };

        log::info!("sender description={:?}", description);

        Ok(Self {
            capture: Capture::start(capture_options)?,
            description,
            transport,
            observer,
            status,
            sink,
        })
    }

    /// Get the media description information of the current sender. The media
    /// description is the information needed to create the receiver.
    pub fn get_description(&self) -> &MediaStreamDescription {
        &self.description
    }
}

impl<S, O> Drop for HylaranaSender<S, O>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    fn drop(&mut self) {
        log::info!("sender drop");

        if !self.status.get() {
            self.status.set(true);

            // When the sender releases, the cleanup work should be done, but there is a
            // more troublesome point here. If it is actively released by the outside, it
            // will also call back to the external closing event. It stands to reason that
            // it should be distinguished whether it is an active closure, but in order to
            // make it simpler to implement, let's do it this way first.
            if let Err(e) = self.capture.close() {
                log::warn!("hylarana sender capture close error={:?}", e);
            }

            self.observer.close();
        }
    }
}
