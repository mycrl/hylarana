#[cfg(target_os = "windows")]
mod win32 {
    pub mod audio;
    pub mod screen;
}

#[cfg(target_os = "linux")]
mod linux {
    pub mod audio;
    pub mod screen;
}

#[cfg(target_os = "macos")]
mod macos {
    pub mod audio;
    pub mod screen;
}

#[cfg(target_os = "windows")]
pub use self::win32::{
    audio::{AudioCapture, AudioCaptureError},
    screen::{ScreenCapture, ScreenCaptureError},
};

#[cfg(target_os = "linux")]
pub use self::linux::{
    audio::{AudioCapture, AudioCaptureError},
    screen::{ScreenCapture, ScreenCaptureError},
};

#[cfg(target_os = "macos")]
pub use self::macos::{
    audio::{AudioCapture, AudioCaptureError},
    screen::{ScreenCapture, ScreenCaptureError},
};

#[cfg(target_os = "windows")]
use common::win32::Direct3DDevice;

use common::{
    Size,
    frame::{AudioFrame, VideoFrame},
};

use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error(transparent)]
    AudioCaptureError(#[from] AudioCaptureError),
    #[error(transparent)]
    ScreenCaptureError(#[from] ScreenCaptureError),
}

pub trait FrameConsumer: Sync + Send {
    /// The type of data captured, such as video frames.
    type Frame;

    /// This method is called when the capture source captures new data. If it
    /// returns false, the source stops capturing.
    fn sink(&mut self, frame: &Self::Frame) -> bool;

    fn close(&mut self);
}

pub trait CaptureHandler: Sync + Send {
    type Error;

    /// The type of data captured, such as video frames.
    type Frame;

    /// Start capturing configuration information, which may be different for
    /// each source.
    type CaptureOptions;

    /// Get a list of sources, such as multiple screens in a display source.
    fn get_sources() -> Result<Vec<Source>, Self::Error>;

    /// Stop capturing the current source.
    fn stop(&self) -> Result<(), Self::Error>;

    /// Start capturing. This function will not block until capturing is
    /// stopped, and it maintains its own capture thread internally.
    fn start<S: FrameConsumer<Frame = Self::Frame> + 'static>(
        &self,
        options: Self::CaptureOptions,
        consumer: S,
    ) -> Result<(), Self::Error>;
}

/// Video source type or Audio source type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum SourceType {
    /// Camera or video capture card and other devices (and support virtual
    /// camera)
    Camera,
    /// The desktop or monitor corresponds to the desktop in the operating
    /// system.
    Screen,
    /// Audio input and output devices.
    Audio,
}

/// Video source or Audio source.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Source {
    /// Device ID, usually the symbolic link to the device or the address of the
    /// device file handle.
    pub id: String,
    pub name: String,
    /// Sequence number, which can normally be ignored, in most cases this field
    /// has no real meaning and simply indicates the order in which the device
    /// was acquired internally.
    pub index: usize,
    pub kind: SourceType,
    /// Whether or not it is the default device, normally used to indicate
    /// whether or not it is the master device.
    pub is_default: bool,
}

#[derive(Debug, Clone)]
pub struct VideoCaptureSourceDescription {
    #[cfg(target_os = "windows")]
    pub direct3d: Direct3DDevice,
    /// Indicates whether the capturer internally outputs hardware frames or
    /// not, it should be noted that internally it will just output hardware
    /// frames to the best of its ability and may also output software frames.
    pub hardware: bool,
    pub source: Source,
    pub size: Size,
    pub fps: u8,
}

#[derive(Debug, Clone)]
pub struct AudioCaptureSourceDescription {
    pub source: Source,
    pub sample_rate: u32,
}

pub struct SourceCaptureOptions<T, P> {
    pub description: P,
    pub consumer: T,
}

pub struct CaptureOptions<V, A>
where
    V: FrameConsumer<Frame = VideoFrame>,
    A: FrameConsumer<Frame = AudioFrame>,
{
    pub video: Option<SourceCaptureOptions<V, VideoCaptureSourceDescription>>,
    pub audio: Option<SourceCaptureOptions<A, AudioCaptureSourceDescription>>,
}

impl<V, A> Default for CaptureOptions<V, A>
where
    V: FrameConsumer<Frame = VideoFrame>,
    A: FrameConsumer<Frame = AudioFrame>,
{
    fn default() -> Self {
        Self {
            video: None,
            audio: None,
        }
    }
}

enum CaptureImplement {
    Screen(ScreenCapture),
    Audio(AudioCapture),
}

/// Capture implementations for audio devices and video devices.
#[derive(Default)]
pub struct Capture(Vec<CaptureImplement>);

impl Capture {
    /// Get all sources that can be used for capture by specifying the type,
    /// which is usually an audio or video device.
    #[allow(unreachable_patterns)]
    pub fn get_sources(kind: SourceType) -> Result<Vec<Source>, CaptureError> {
        log::info!("capture get sources, kind={:?}", kind);

        Ok(match kind {
            SourceType::Screen => ScreenCapture::get_sources()?,
            SourceType::Audio => AudioCapture::get_sources()?,
            _ => Vec::new(),
        })
    }

    /// Create a capture and start capturing audio and video frames by
    /// specifying the source to be captured.
    pub fn start<V, A>(
        CaptureOptions { video, audio }: CaptureOptions<V, A>,
    ) -> Result<Self, CaptureError>
    where
        V: FrameConsumer<Frame = VideoFrame> + 'static,
        A: FrameConsumer<Frame = AudioFrame> + 'static,
    {
        let mut devices = Vec::with_capacity(3);

        if let Some(SourceCaptureOptions {
            description,
            consumer,
        }) = video
        {
            let screen = ScreenCapture::default();
            screen.start(description, consumer)?;
            devices.push(CaptureImplement::Screen(screen));
        }

        if let Some(SourceCaptureOptions {
            description,
            consumer,
        }) = audio
        {
            let audio = AudioCapture::default();
            audio.start(description, consumer)?;
            devices.push(CaptureImplement::Audio(audio));
        }

        Ok(Self(devices))
    }

    /// Stop capturing and turn off internal audio/video frame pushing.
    pub fn close(&self) -> Result<(), CaptureError> {
        for item in self.0.iter() {
            match item {
                CaptureImplement::Screen(it) => it.stop()?,
                CaptureImplement::Audio(it) => it.stop()?,
            };
        }

        log::info!("close capture");

        Ok(())
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        log::info!("capture drop");

        drop(self.close());
    }
}
