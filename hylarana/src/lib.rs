mod player;
mod receiver;
mod sender;
mod util;

use thiserror::Error;

pub use self::{player::*, receiver::*, sender::*};

pub use capture::{Capture, Source, SourceType};
pub use common::{
    codec::*, frame::*, MediaAudioStreamDescription, MediaStreamDescription,
    MediaVideoStreamDescription, Size, TransportOptions, TransportStrategy,
};

pub use discovery::{DiscoveryError, DiscoveryService};
pub use renderer::{raw_window_handle, SurfaceTarget};

#[cfg(target_os = "windows")]
use common::win32::{
    set_process_priority, shutdown as win32_shutdown, startup as win32_startup, ProcessPriority,
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
pub trait AVFrameObserver: Sync + Send {
    /// Callback when the sender is closed. This may be because the external
    /// side actively calls the close, or the audio and video packets cannot be
    /// sent (the network is disconnected), etc.
    fn close(&self) {}
}

/// Streaming sink for audio and video frames.
pub trait AVFrameSink: Sync + Send {
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

/// Abstraction of audio and video streams.
pub trait AVFrameStream: AVFrameSink + AVFrameObserver {}

/// Creates a sender that can specify the audio source or video source to be
/// captured.
pub fn create_sender<T: AVFrameStream + 'static>(
    options: &HylaranaSenderOptions,
    sink: T,
) -> Result<HylaranaSender<T>, HylaranaSenderError> {
    log::info!("create sender: options={:?}", options);

    HylaranaSender::new(options, sink)
}

/// To create a receiver, you need to specify the sender's ID to associate
/// with it.
pub fn create_receiver<T: AVFrameStream + 'static>(
    description: &MediaStreamDescription,
    options: &HylaranaReceiverOptions,
    sink: T,
) -> Result<HylaranaReceiver<T>, HylaranaReceiverError> {
    log::info!(
        "create receiver: description={:?}, options={:?}",
        description,
        options
    );

    HylaranaReceiver::new(description, options, sink)
}
