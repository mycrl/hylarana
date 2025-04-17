mod player;
mod receiver;
mod sender;
mod util;

use thiserror::Error;

pub use self::{player::*, receiver::*, sender::*};

pub use capture::{Capture, Source, SourceType};
pub use common::{
    MediaAudioStreamDescription, MediaStreamDescription, MediaVideoStreamDescription, Size,
    TransportOptions, TransportStrategy, codec::*, frame::*, runtime::*,
};

pub use discovery::{DiscoveryContext, DiscoveryError, DiscoveryObserver, DiscoveryService};
pub use renderer::{SurfaceTarget, raw_window_handle};

#[cfg(target_os = "windows")]
use common::win32::{
    ProcessPriority, set_process_priority, shutdown as win32_shutdown, startup as win32_startup,
};

#[derive(Debug, Error)]
pub enum HylaranaError {
    #[error(transparent)]
    #[cfg(target_os = "windows")]
    Win32Error(#[from] common::win32::windows::core::Error),
    #[error(transparent)]
    TransportError(#[from] std::io::Error),
}

/// Initialize the environment, which must be initialized before using the sdk.
pub fn startup() -> Result<(), HylaranaError> {
    log::info!("hylarana startup");

    #[cfg(target_os = "windows")]
    if let Err(e) = win32_startup() {
        log::warn!("{:?}", e);
    }

    // In order to prevent other programs from affecting the delay performance of
    // the current program, set the priority of the current process to high.
    #[cfg(target_os = "windows")]
    if set_process_priority(ProcessPriority::High).is_err() {
        log::error!(
            "failed to set current process priority, Maybe it's \
            because you didn't run it with administrator privileges."
        );
    }

    codec::startup();
    log::info!("codec initialized");

    transport::startup();
    log::info!("transport initialized");

    log::info!("all initialized");
    Ok(())
}

/// Cleans up the environment when the sdk exits, and is recommended to be
/// called when the application exits.
pub fn shutdown() -> Result<(), HylaranaError> {
    log::info!("hylarana shutdown");

    codec::shutdown();
    transport::shutdown();

    #[cfg(target_os = "windows")]
    if let Err(e) = win32_shutdown() {
        log::warn!("{:?}", e);
    }

    Ok(())
}

/// Audio and video streaming events observer.
pub trait MediaStreamObserver: Sync + Send {
    /// Callback when the sender is closed. This may be because the external
    /// side actively calls the close, or the audio and video packets cannot be
    /// sent (the network is disconnected), etc.
    fn close(&self) {}
}

// impl empty type for default
impl MediaStreamObserver for () {}

/// Streaming sink for audio and video frames.
pub trait MediaStreamSink: Sync + Send {
    /// Callback occurs when the video frame is updated. The video frame format
    /// is fixed to NV12. Be careful not to call blocking methods inside the
    /// callback, which will seriously slow down the encoding and decoding
    /// pipeline.
    ///
    /// Returning `false` causes the stream to close.
    #[allow(unused_variables)]
    fn video(&self, frame: &VideoFrame) -> bool {
        true
    }

    /// Callback is called when the audio frame is updated. The audio frame
    /// format is fixed to PCM. Be careful not to call blocking methods inside
    /// the callback, which will seriously slow down the encoding and decoding
    /// pipeline.
    ///
    /// Returning `false` causes the stream to close.
    #[allow(unused_variables)]
    fn audio(&self, frame: &AudioFrame) -> bool {
        true
    }
}

// impl empty type for default
impl MediaStreamSink for () {}

/// Creates a sender that can specify the audio source or video source to be
/// captured.
pub fn create_sender<S, O>(
    options: &HylaranaSenderOptions,
    sink: S,
    observer: O,
) -> Result<HylaranaSender<S, O>, HylaranaSenderError>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    log::info!("create sender: options={:?}", options);

    HylaranaSender::new(options, sink, observer)
}

/// To create a receiver, you need to specify the sender's ID to associate
/// with it.
pub fn create_receiver<S, O>(
    description: &MediaStreamDescription,
    options: &HylaranaReceiverOptions,
    sink: S,
    observer: O,
) -> Result<HylaranaReceiver<S, O>, HylaranaReceiverError>
where
    S: MediaStreamSink + 'static,
    O: MediaStreamObserver + 'static,
{
    log::info!(
        "create receiver: description={:?}, options={:?}",
        description,
        options
    );

    HylaranaReceiver::new(description, options, sink, observer)
}
