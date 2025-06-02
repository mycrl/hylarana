use super::{
    MediaAudioStreamDescription, MediaStreamDescription, MediaStreamObserver, MediaStreamSink,
    MediaVideoStreamDescription,
};

#[cfg(target_os = "windows")]
use super::util::get_direct3d;

use std::{
    net::SocketAddr,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

use capture::{
    AudioCaptureSourceDescription, Capture, CaptureOptions, FrameConsumer, Source,
    SourceCaptureOptions, VideoCaptureSourceDescription,
};

use common::{
    Size,
    codec::VideoEncoderType,
    frame::{AudioFrame, VideoFormat, VideoFrame},
};

use codec::{
    AudioEncoder, AudioEncoderSettings, CodecType, VideoEncoder, VideoEncoderSettings,
    create_opus_identification_header,
};

use thiserror::Error;
use transport::{Buffer, BufferType, StreamType, TransportOptions, TransportSender};

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

// Encoding is a relatively complex task. If you add encoding tasks to the
// pipeline that pushes frames, it will slow down the entire pipeline.
//
// Here, the tasks are separated, and the encoding tasks are separated into
// independent threads. The encoding thread is notified of task updates through
// the optional lock.
struct VideoSender<S> {
    callback: Arc<dyn Fn() + Send + Sync + 'static>,
    transport: Weak<TransportSender>,
    encoder: VideoEncoder,
    sink: Arc<S>,
}

impl<S> VideoSender<S> {
    fn new(
        options: &VideoOptions,
        transport: &Arc<TransportSender>,
        sink: Arc<S>,
        callback: Arc<dyn Fn() + Send + Sync + 'static>,
    ) -> Result<Self, HylaranaSenderError> {
        Ok(VideoSender {
            encoder: VideoEncoder::new(VideoEncoderSettings {
                codec: options.codec,
                key_frame_interval: options.key_frame_interval,
                frame_rate: options.frame_rate,
                width: options.width,
                height: options.height,
                bit_rate: options.bit_rate,
                #[cfg(target_os = "windows")]
                direct3d: Some(get_direct3d()),
            })?,
            transport: Arc::downgrade(&transport),
            callback,
            sink,
        })
    }
}

impl<S> FrameConsumer for VideoSender<S>
where
    S: MediaStreamSink + 'static,
{
    type Frame = VideoFrame;

    fn sink(&mut self, frame: &Self::Frame) -> bool {
        if let Some(transport) = self.transport.upgrade() {
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
                        if let Err(e) = transport.send(Buffer {
                            data: Buffer::<()>::copy_from_slice(buffer),
                            ty: BufferType::try_from(flags as u8).unwrap(),
                            stream: StreamType::Video,
                            timestamp,
                        }) {
                            log::warn!("video send packet to transport failed, err={:?}", e);

                            return false;
                        }
                    }
                }
            } else {
                log::warn!("video encoder update frame failed");

                return false;
            }

            if self.sink.video(frame) {
                true
            } else {
                log::warn!("video sink on frame return false");

                false
            }
        } else {
            log::warn!("transport weak upgrade failed, maybe is drop");

            false
        }
    }

    fn close(&mut self) {
        log::info!("video sender is closed");

        (self.callback)();
    }
}

// Encoding is a relatively complex task. If you add encoding tasks to the
// pipeline that pushes frames, it will slow down the entire pipeline.
//
// Here, the tasks are separated, and the encoding tasks are separated into
// independent threads. The encoding thread is notified of task updates through
// the optional lock.
struct AudioSender<S> {
    callback: Arc<dyn Fn() + Send + Sync + 'static>,
    transport: Weak<TransportSender>,
    encoder: AudioEncoder,
    sink: Arc<S>,
}

impl<S> AudioSender<S> {
    fn new(
        options: &AudioOptions,
        transport: &Arc<TransportSender>,
        sink: Arc<S>,
        callback: Arc<dyn Fn() + Send + Sync + 'static>,
    ) -> Result<Self, HylaranaSenderError> {
        // Create an opus header data. The opus decoder needs this data to obtain audio
        // information. Here, actively add an opus header information to the queue, and
        // the adapter layer will automatically cache it.
        transport.send(Buffer {
            stream: StreamType::Audio,
            ty: BufferType::Config,
            timestamp: 0,
            data: Buffer::<()>::copy_from_slice(&create_opus_identification_header(
                2,
                options.sample_rate as u32,
            )),
        })?;

        Ok(Self {
            encoder: AudioEncoder::new(AudioEncoderSettings {
                sample_rate: options.sample_rate,
                bit_rate: options.bit_rate,
            })?,
            transport: Arc::downgrade(&transport),
            callback,
            sink,
        })
    }
}

impl<S> FrameConsumer for AudioSender<S>
where
    S: MediaStreamSink + 'static,
{
    type Frame = AudioFrame;

    fn sink(&mut self, frame: &Self::Frame) -> bool {
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
                while let Some((buffer, _, timestamp)) = self.encoder.read() {
                    if let Some(transport) = self.transport.upgrade() {
                        if let Err(e) = transport.send(Buffer {
                            data: Buffer::<()>::copy_from_slice(buffer),
                            ty: BufferType::Partial,
                            stream: StreamType::Audio,
                            timestamp,
                        }) {
                            log::warn!("audio send packet to transport failed, err={:?}", e);

                            return false;
                        }
                    } else {
                        log::warn!("transport weak upgrade failed, maybe is drop");

                        return false;
                    }
                }
            }
        } else {
            log::warn!("audio encoder update frame failed");

            return false;
        }

        if self.sink.audio(frame) {
            true
        } else {
            log::warn!("audio sink on frame return false");

            false
        }
    }

    fn close(&mut self) {
        log::info!("audio sender is closed");

        (self.callback)();
    }
}

/// Screen casting sender.
pub struct HylaranaSender {
    callback: Arc<dyn Fn() + Send + Sync + 'static>,
    description: MediaStreamDescription,
    transport: Arc<TransportSender>,
    #[allow(unused)]
    capture: Capture,
}

impl HylaranaSender {
    // Create a sender. The capture of the sender is started following the sender,
    // but both video capture and audio capture can be empty, which means you can
    // create a sender that captures nothing.
    pub(crate) fn new<S, O>(
        bind: SocketAddr,
        options: &HylaranaSenderOptions,
        sink: S,
        observer: O,
    ) -> Result<Self, HylaranaSenderError>
    where
        S: MediaStreamSink + 'static,
        O: MediaStreamObserver + 'static,
    {
        log::info!("create sender");

        let transport = Arc::new(TransportSender::new(bind, options.transport.clone())?);

        let callback = {
            let working = AtomicBool::new(true);

            Arc::new(move || {
                if working.load(Ordering::Relaxed) {
                    working.store(false, Ordering::Relaxed);
                    observer.close();

                    log::info!("sender is closed");
                }
            })
        };

        let capture_options = {
            let sink = Arc::new(sink);
            let mut opt = CaptureOptions::default();

            if let Some(HylaranaSenderTrackOptions { source, options }) = &options.media.audio {
                opt.audio = Some(SourceCaptureOptions {
                    consumer: AudioSender::new(
                        &options,
                        &transport,
                        sink.clone(),
                        callback.clone(),
                    )?,
                    description: AudioCaptureSourceDescription {
                        sample_rate: options.sample_rate as u32,
                        source: source.clone(),
                    },
                });
            }

            if let Some(HylaranaSenderTrackOptions { source, options }) = &options.media.video {
                opt.video = Some(SourceCaptureOptions {
                    consumer: VideoSender::new(
                        options,
                        &transport,
                        sink.clone(),
                        callback.clone(),
                    )?,
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
                });
            }

            opt
        };

        let description = MediaStreamDescription {
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
            callback,
        })
    }

    /// Get the media description information of the current sender. The media
    /// description is the information needed to create the receiver.
    pub fn get_description(&self) -> &MediaStreamDescription {
        &self.description
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.transport.local_addr()
    }
}

impl Drop for HylaranaSender {
    fn drop(&mut self) {
        (self.callback)();
    }
}
