use std::net::SocketAddr;

use super::{MediaStreamDescription, MediaStreamObserver, MediaStreamSink};

use bytes::Bytes;
use codec::{AudioDecoder, VideoDecoder, VideoDecoderSettings};
use common::codec::VideoDecoderType;
use thiserror::Error;
use transport::{Buffer, StreamType, TransportOptions, TransportReceiver, TransportReceiverSink};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(target_os = "windows")]
use super::util::get_direct3d;

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
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct HylaranaReceiverOptions {
    pub codec: VideoDecoderType,
    pub transport: TransportOptions,
}

struct ReceiverSinker<S, O> {
    audio_decoder: AudioDecoder,
    video_decoder: VideoDecoder,
    observer: O,
    sink: S,
}

impl<S, O> TransportReceiverSink for ReceiverSinker<S, O>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    fn sink(&mut self, buffer: Buffer<Bytes>) -> bool {
        match buffer.stream {
            StreamType::Video => {
                if let Err(e) = self.video_decoder.decode(&buffer.data, buffer.timestamp) {
                    log::error!("video decode error={:?}", e);

                    return false;
                } else {
                    while let Some(frame) = self.video_decoder.read() {
                        if !self.sink.video(frame) {
                            log::warn!("video sink return false!");

                            return false;
                        }
                    }
                }
            }
            StreamType::Audio => {
                if let Err(e) = self.audio_decoder.decode(&buffer.data, buffer.timestamp) {
                    log::error!("audio decode error={:?}", e);

                    return false;
                } else {
                    while let Some(frame) = self.audio_decoder.read() {
                        if !self.sink.audio(frame) {
                            log::warn!("audio sink return false!");

                            return false;
                        }
                    }
                }
            }
        }

        true
    }

    fn close(&mut self) {
        log::info!("receiver is closed");

        self.observer.close();
    }
}

/// Screen casting receiver.
pub struct HylaranaReceiver {
    description: MediaStreamDescription,
    #[allow(unused)]
    transport: TransportReceiver,
}

impl HylaranaReceiver {
    /// Create a receiving end. The receiving end is much simpler to implement.
    /// You only need to decode the data in the queue and call it back to the
    /// sink.
    pub(crate) fn new<S, O>(
        addr: SocketAddr,
        options: &HylaranaReceiverOptions,
        description: &MediaStreamDescription,
        sink: S,
        observer: O,
    ) -> Result<Self, HylaranaReceiverError>
    where
        S: MediaStreamSink + 'static,
        O: MediaStreamObserver + 'static,
    {
        log::info!("create receiver");

        Ok(Self {
            description: description.clone(),
            transport: TransportReceiver::new(
                addr,
                options.transport.clone(),
                ReceiverSinker {
                    video_decoder: VideoDecoder::new(VideoDecoderSettings {
                        codec: options.codec,
                        #[cfg(target_os = "windows")]
                        direct3d: Some(get_direct3d()),
                    })?,
                    audio_decoder: AudioDecoder::new()?,
                    observer,
                    sink,
                },
            )?,
        })
    }

    /// Get the media description information of the current receiver.
    pub fn get_description(&self) -> &MediaStreamDescription {
        &self.description
    }
}
