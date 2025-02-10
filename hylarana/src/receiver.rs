use crate::{MediaStreamDescription, MediaStreamObserver, MediaStreamSink};

use std::{
    sync::{atomic::AtomicBool, Arc},
    thread,
};

use codec::{AudioDecoder, VideoDecoder, VideoDecoderSettings};
use common::{atomic::EasyAtomic, codec::VideoDecoderType};
use serde::{Deserialize, Serialize};
use transport::{StreamKind, StreamMultiReceiverAdapter, TransportReceiver};

use thiserror::Error;

#[cfg(target_os = "windows")]
use common::win32::MediaThreadClass;

#[cfg(target_os = "windows")]
use crate::util::get_direct3d;

#[derive(Debug, Error)]
pub enum HylaranaReceiverError {
    #[error(transparent)]
    CreateThreadError(#[from] std::io::Error),
    #[error(transparent)]
    VideoDecoderError(#[from] codec::VideoDecoderError),
    #[error(transparent)]
    AudioDecoderError(#[from] codec::AudioDecoderError),
}

/// Receiver configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HylaranaReceiverOptions {
    pub video_decoder: VideoDecoderType,
}

fn create_video_decoder<S, O>(
    transport: &TransportReceiver<StreamMultiReceiverAdapter>,
    status: Arc<AtomicBool>,
    settings: VideoDecoderSettings,
    sink: &Arc<S>,
    observer: &Arc<O>,
) -> Result<(), HylaranaReceiverError>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    let sink_ = Arc::downgrade(sink);
    let observer_ = Arc::downgrade(observer);
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
            if let Some(observer) = observer_.upgrade() {
                if !status.get() {
                    status.update(true);
                    observer.close();
                }
            }

            #[cfg(target_os = "windows")]
            if let Some(guard) = thread_class_guard {
                drop(guard)
            }
        })?;

    Ok(())
}

fn create_audio_decoder<S, O>(
    transport: &TransportReceiver<StreamMultiReceiverAdapter>,
    status: Arc<AtomicBool>,
    sink: &Arc<S>,
    observer: &Arc<O>,
) -> Result<(), HylaranaReceiverError>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    let sink_ = Arc::downgrade(sink);
    let observer_ = Arc::downgrade(observer);
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
            if let Some(observer) = observer_.upgrade() {
                if !status.get() {
                    status.update(true);
                    observer.close();
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
pub struct HylaranaReceiver<S, O>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    description: MediaStreamDescription,
    #[allow(unused)]
    transport: TransportReceiver<StreamMultiReceiverAdapter>,
    status: Arc<AtomicBool>,
    #[allow(unused)]
    sink: Arc<S>,
    observer: Arc<O>,
}

impl<S, O> HylaranaReceiver<S, O>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    /// Create a receiving end. The receiving end is much simpler to implement.
    /// You only need to decode the data in the queue and call it back to the
    /// sink.
    pub(crate) fn new(
        description: &MediaStreamDescription,
        options: &HylaranaReceiverOptions,
        sink: S,
        observer: O,
    ) -> Result<Self, HylaranaReceiverError> {
        log::info!("create receiver");

        let transport = transport::create_split_receiver(&description.id, description.transport)?;
        let status = Arc::new(AtomicBool::new(false));
        let observer = Arc::new(observer);
        let sink = Arc::new(sink);

        create_audio_decoder(&transport, status.clone(), &sink, &observer)?;
        create_video_decoder(
            &transport,
            status.clone(),
            VideoDecoderSettings {
                codec: options.video_decoder,
                #[cfg(target_os = "windows")]
                direct3d: Some(get_direct3d()),
            },
            &sink,
            &observer,
        )?;

        Ok(Self {
            description: description.clone(),
            transport,
            observer,
            status,
            sink,
        })
    }

    /// Get the media description information of the current receiver.
    pub fn get_description(&self) -> &MediaStreamDescription {
        &self.description
    }
}

impl<S, O> Drop for HylaranaReceiver<S, O>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    fn drop(&mut self) {
        log::info!("receiver drop");

        if !self.status.get() {
            self.status.update(true);
            self.observer.close();
        }
    }
}
