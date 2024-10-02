mod receiver;
mod render;

#[cfg(any(target_os = "windows", target_os = "linux"))]
mod sender;

use graphics::SurfaceTarget;
use parking_lot::Mutex;

#[cfg(target_os = "windows")]
use parking_lot::RwLock;

pub use self::receiver::{MirrorReceiver, MirrorReceiverDescriptor};
pub use self::render::Backend as VideoRenderBackend;

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub use self::sender::{AudioDescriptor, MirrorSender, MirrorSenderDescriptor, VideoDescriptor};

use self::render::{AudioRender, VideoRender};

use anyhow::Result;
use transport::Transport;
use utils::Size;

pub use capture::{Capture, Source, SourceType};
pub use codec::{VideoDecoderType, VideoEncoderType};
pub use frame::{AudioFrame, VideoFormat, VideoFrame};
pub use graphics::raw_window_handle;
pub use transport::TransportDescriptor;

#[cfg(target_os = "windows")]
use utils::win32::{
    set_process_priority, shutdown as win32_shutdown, startup as win32_startup,
    windows::Win32::Foundation::HWND, Direct3DDevice, ProcessPriority,
};

#[cfg(target_os = "windows")]
pub(crate) static DIRECT_3D_DEVICE: RwLock<Option<Direct3DDevice>> = RwLock::new(None);

/// Initialize the environment, which must be initialized before using the SDK.
pub fn startup() -> Result<()> {
    log::info!("mirror startup");

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

/// Cleans up the environment when the SDK exits, and is recommended to be
/// called when the application exits.
pub fn shutdown() -> Result<()> {
    log::info!("mirror shutdown");

    codec::shutdown();
    transport::shutdown();

    #[cfg(target_os = "windows")]
    if let Err(e) = win32_shutdown() {
        log::warn!("{:?}", e);
    }

    Ok(())
}

pub trait Close: Sync + Send {
    /// Callback when the sender is closed. This may be because the external
    /// side actively calls the close, or the audio and video packets cannot be
    /// sent (the network is disconnected), etc.
    fn close(&self);
}

pub trait AVFrameSink: Sync + Send {
    /// Callback occurs when the video frame is updated. The video frame format
    /// is fixed to NV12. Be careful not to call blocking methods inside the
    /// callback, which will seriously slow down the encoding and decoding
    /// pipeline.
    ///
    /// YCbCr (NV12)
    ///
    /// YCbCr, Y′CbCr, or Y Pb/Cb Pr/Cr, also written as YCBCR or Y′CBCR, is a
    /// family of color spaces used as a part of the color image pipeline in
    /// video and digital photography systems. Y′ is the luma component and
    /// CB and CR are the blue-difference and red-difference chroma
    /// components. Y′ (with prime) is distinguished from Y, which is
    /// luminance, meaning that light intensity is nonlinearly encoded based
    /// on gamma corrected RGB primaries.
    ///
    /// Y′CbCr color spaces are defined by a mathematical coordinate
    /// transformation from an associated RGB primaries and white point. If
    /// the underlying RGB color space is absolute, the Y′CbCr color space
    /// is an absolute color space as well; conversely, if the RGB space is
    /// ill-defined, so is Y′CbCr. The transformation is defined in
    /// equations 32, 33 in ITU-T H.273. Nevertheless that rule does not
    /// apply to P3-D65 primaries used by Netflix with BT.2020-NCL matrix,
    /// so that means matrix was not derived from primaries, but now Netflix
    /// allows BT.2020 primaries (since 2021). The same happens with
    /// JPEG: it has BT.601 matrix derived from System M primaries, yet the
    /// primaries of most images are BT.709.
    #[allow(unused_variables)]
    fn video(&self, frame: &VideoFrame) -> bool {
        true
    }

    /// Callback is called when the audio frame is updated. The audio frame
    /// format is fixed to PCM. Be careful not to call blocking methods inside
    /// the callback, which will seriously slow down the encoding and decoding
    /// pipeline.
    ///
    /// Pulse-code modulation
    ///
    /// Pulse-code modulation (PCM) is a method used to digitally represent
    /// analog signals. It is the standard form of digital audio in
    /// computers, compact discs, digital telephony and other digital audio
    /// applications. In a PCM stream, the amplitude of the analog signal is
    /// sampled at uniform intervals, and each sample is quantized to the
    /// nearest value within a range of digital steps.
    ///
    /// Linear pulse-code modulation (LPCM) is a specific type of PCM in which
    /// the quantization levels are linearly uniform. This is in contrast to
    /// PCM encodings in which quantization levels vary as a function of
    /// amplitude (as with the A-law algorithm or the μ-law algorithm).
    /// Though PCM is a more general term, it is often used to describe data
    /// encoded as LPCM.
    ///
    /// A PCM stream has two basic properties that determine the stream's
    /// fidelity to the original analog signal: the sampling rate, which is
    /// the number of times per second that samples are taken; and the bit
    /// depth, which determines the number of possible digital values that
    /// can be used to represent each sample.
    #[allow(unused_variables)]
    fn audio(&self, frame: &AudioFrame) -> bool {
        true
    }
}

pub trait AVFrameStream: AVFrameSink + Close {}

pub struct Mirror(Transport);

impl Mirror {
    pub fn new(options: TransportDescriptor) -> Result<Self> {
        log::info!("create mirror: options={:?}", options);

        // Check if the D3D device has been created. If not, create a global one.
        #[cfg(target_os = "windows")]
        {
            if DIRECT_3D_DEVICE.read().is_none() {
                DIRECT_3D_DEVICE.write().replace(Direct3DDevice::new()?);
            }
        }

        Ok(Self(Transport::new(options)?))
    }

    /// Create a sender, specify a bound NIC address, you can pass callback to
    /// get the device screen or sound callback, callback can be null, if it is
    /// null then it means no callback data is needed.
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    pub fn create_sender<T: AVFrameStream + 'static>(
        &self,
        id: u32,
        options: MirrorSenderDescriptor,
        sink: T,
    ) -> Result<MirrorSender> {
        log::info!("create sender: id={}, options={:?}", id, options);

        let sender = MirrorSender::new(options, sink)?;
        self.0.create_sender(id, &sender.adapter)?;
        Ok(sender)
    }

    /// Create a receiver, specify a bound NIC address, you can pass callback to
    /// get the sender's screen or sound callback, callback can not be null.
    pub fn create_receiver<T: AVFrameStream + 'static>(
        &self,
        id: u32,
        options: MirrorReceiverDescriptor,
        sink: T,
    ) -> Result<MirrorReceiver> {
        log::info!("create receiver: id={}, options={:?}", id, options);

        let receiver = MirrorReceiver::new(options, sink)?;
        self.0.create_receiver(id, &receiver.adapter)?;
        Ok(receiver)
    }
}

/// Renderer for video frames and audio frames.
///
/// Typically, the player underpinnings for audio and video are implementedin
/// hardware, but not always, the underpinnings automatically select the adapter
/// and fall back to the software adapter if the hardware adapter is
/// unavailable, for video this can be done by enabling the dx11 feature tobe
/// implemented with Direct3D 11 Graphics, which works fine on some very old
/// devices.
pub struct Render<'a>(Mutex<VideoRender<'a>>, Mutex<AudioRender>);

impl<'a> Render<'a> {
    pub fn new<T: Into<SurfaceTarget<'a>>>(
        backend: VideoRenderBackend,
        window: T,
        size: Size,
    ) -> Result<Self> {
        Ok(Self(
            Mutex::new(VideoRender::new(backend, window, size)?),
            Mutex::new(AudioRender::new()?),
        ))
    }
}

impl<'a> AVFrameSink for Render<'a> {
    /// Renders the audio frame, note that a queue is maintained internally,
    /// here it just pushes the audio to the playback queue, and if the queue is
    /// empty, it fills the mute data to the player by default, so you need to
    /// pay attention to the push rate.
    fn audio(&self, frame: &AudioFrame) -> bool {
        if let Err(e) = self.1.lock().send(frame) {
            log::error!("{:?}", e);

            return false;
        }

        true
    }

    /// Renders video frames and can automatically handle rendering of hardware
    /// textures and rendering textures.
    fn video(&self, frame: &VideoFrame) -> bool {
        if let Err(e) = self.0.lock().send(frame) {
            log::error!("{:?}", e);

            return false;
        }

        true
    }
}
