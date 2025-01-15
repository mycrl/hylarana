use crate::AVFrameStream;

use std::{
    sync::{atomic::AtomicBool, Arc},
    thread,
};

use codec::{AudioDecoder, VideoDecoder, VideoDecoderSettings, VideoDecoderType};
use common::atomic::EasyAtomic;
use transport::{StreamKind, StreamMultiReceiverAdapter, TransportOptions, TransportReceiver};

use thiserror::Error;

#[cfg(target_os = "windows")]
use common::win32::MediaThreadClass;

#[derive(Debug, Error)]
pub enum HylaranaReceiverError {
    #[error(transparent)]
    CreateThreadError(#[from] std::io::Error),
    #[error(transparent)]
    VideoDecoderError(#[from] codec::VideoDecoderError),
    #[error(transparent)]
    AudioDecoderError(#[from] codec::AudioDecoderError),
}

/// Receiver media codec configuration.
#[derive(Debug, Clone)]
pub struct HylaranaReceiverCodecOptions {
    pub video: VideoDecoderType,
}

/// Receiver configuration.
#[derive(Debug, Clone)]
pub struct HylaranaReceiverOptions {
    pub transport: TransportOptions,
    pub codec: HylaranaReceiverCodecOptions,
}

fn create_video_decoder<T: AVFrameStream + 'static>(
    transport: &TransportReceiver<StreamMultiReceiverAdapter>,
    status: Arc<AtomicBool>,
    sink: &Arc<T>,
    settings: VideoDecoderSettings,
) -> Result<(), HylaranaReceiverError> {
    let sink_ = Arc::downgrade(sink);
    let adapter = transport.get_adapter();
    let mut codec = VideoDecoder::new(settings)?;

    thread::Builder::new()
        .name("VideoDecoderThread".to_string())
        .spawn(move || {
            #[cfg(target_os = "windows")]
            let thread_class_guard = MediaThreadClass::Playback.join().ok();

            'a: while let Some(sink) = sink_.upgrade() {
                if let Some((packet, _, timestamp)) = adapter.next(StreamKind::Video) {
                    if let Err(e) = codec.decode(&packet, timestamp) {
                        log::error!("video decode error={:?}", e);

                        break;
                    } else {
                        while let Some(frame) = codec.read() {
                            if !sink.video(frame) {
                                log::warn!("video sink return false!");

                                break 'a;
                            }
                        }
                    }
                } else {
                    log::warn!("video adapter next is none!");

                    break;
                }
            }

            log::warn!("video decoder thread is closed!");
            if let Some(sink) = sink_.upgrade() {
                if !status.get() {
                    status.update(true);
                    sink.close();
                }
            }

            #[cfg(target_os = "windows")]
            if let Some(guard) = thread_class_guard {
                drop(guard)
            }
        })?;

    Ok(())
}

fn create_audio_decoder<T: AVFrameStream + 'static>(
    transport: &TransportReceiver<StreamMultiReceiverAdapter>,
    status: Arc<AtomicBool>,
    sink: &Arc<T>,
) -> Result<(), HylaranaReceiverError> {
    let sink_ = Arc::downgrade(sink);
    let adapter = transport.get_adapter();
    let mut codec = AudioDecoder::new()?;

    thread::Builder::new()
        .name("AudioDecoderThread".to_string())
        .spawn(move || {
            #[cfg(target_os = "windows")]
            let thread_class_guard = MediaThreadClass::ProAudio.join().ok();

            'a: while let Some(sink) = sink_.upgrade() {
                if let Some((packet, _, timestamp)) = adapter.next(StreamKind::Audio) {
                    if let Err(e) = codec.decode(&packet, timestamp) {
                        log::error!("audio decode error={:?}", e);

                        break;
                    } else {
                        while let Some(frame) = codec.read() {
                            if !sink.audio(frame) {
                                log::warn!("audio sink return false!");

                                break 'a;
                            }
                        }
                    }
                } else {
                    log::warn!("audio adapter next is none!");

                    break;
                }
            }

            log::warn!("audio decoder thread is closed!");
            if let Some(sink) = sink_.upgrade() {
                if !status.get() {
                    status.update(true);
                    sink.close();
                }
            }

            #[cfg(target_os = "windows")]
            if let Some(guard) = thread_class_guard {
                drop(guard)
            }
        })?;

    Ok(())
}

/// Screen casting receiver.
pub struct HylaranaReceiver<T: AVFrameStream + 'static> {
    #[allow(unused)]
    transport: TransportReceiver<StreamMultiReceiverAdapter>,
    status: Arc<AtomicBool>,
    sink: Arc<T>,
}

impl<T: AVFrameStream + 'static> HylaranaReceiver<T> {
    /// Create a receiving end. The receiving end is much simpler to implement.
    /// You only need to decode the data in the queue and call it back to the
    /// sink.
    pub(crate) fn new(
        id: String,
        options: HylaranaReceiverOptions,
        sink: T,
    ) -> Result<Self, HylaranaReceiverError> {
        log::info!("create receiver");

        let transport = transport::create_split_receiver(id, options.transport)?;
        let status = Arc::new(AtomicBool::new(false));
        let sink = Arc::new(sink);

        create_audio_decoder(&transport, status.clone(), &sink)?;
        create_video_decoder(
            &transport,
            status.clone(),
            &sink,
            VideoDecoderSettings {
                codec: options.codec.video,
                #[cfg(target_os = "windows")]
                direct3d: Some(crate::get_direct3d()),
            },
        )?;

        Ok(Self {
            transport,
            status,
            sink,
        })
    }
}

impl<T: AVFrameStream + 'static> Drop for HylaranaReceiver<T> {
    fn drop(&mut self) {
        log::info!("receiver drop");

        if !self.status.get() {
            self.status.update(true);
            self.sink.close();
        }
    }
}
