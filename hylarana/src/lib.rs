#![doc = include_str!("../README.md")]

mod receiver;
mod sender;

use std::{slice::from_raw_parts, str::FromStr};

pub use self::{
    receiver::{
        HylaranaReceiver, HylaranaReceiverCodecOptions, HylaranaReceiverError,
        HylaranaReceiverOptions,
    },
    sender::{
        AudioOptions, HylaranaSender, HylaranaSenderError, HylaranaSenderMediaOptions,
        HylaranaSenderOptions, HylaranaSenderTrackOptions, VideoOptions,
    },
};

pub use capture::{Capture, Source, SourceType};
pub use codec::{VideoDecoderType, VideoEncoderType};
pub use common::{
    frame::{AudioFrame, VideoFormat, VideoFrame, VideoSubFormat},
    Size,
};

pub use discovery::{DiscoveryError, DiscoveryService};
pub use renderer::{raw_window_handle, SurfaceTarget};
pub use transport::{TransportOptions, TransportStrategy};

#[cfg(target_os = "windows")]
use common::win32::{
    d3d_texture_borrowed_raw, set_process_priority, shutdown as win32_shutdown,
    startup as win32_startup, windows::Win32::Foundation::HWND, Direct3DDevice, ProcessPriority,
};

#[cfg(target_os = "macos")]
use common::macos::CVPixelBufferRef;

use parking_lot::Mutex;

#[cfg(target_os = "windows")]
use parking_lot::RwLock;

#[cfg(target_os = "windows")]
use renderer::win32::D3D11Renderer;

#[cfg(target_os = "macos")]
use renderer::Texture2DRaw;

use renderer::{
    Renderer as WgpuRenderer, RendererOptions as WgpuRendererOptions, Texture, Texture2DBuffer,
    Texture2DResource,
};

use rodio::{OutputStream, OutputStreamHandle, Sink};
use thiserror::Error;

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

    #[cfg(target_os = "linux")]
    capture::startup();

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

/// Creates entries for the sender and receiver.
pub struct Hylarana;

impl Hylarana {
    /// Creates a sender that can specify the audio source or video source to be
    /// captured.
    pub fn create_sender<T: AVFrameStream + 'static>(
        options: HylaranaSenderOptions,
        sink: T,
    ) -> Result<HylaranaSender<T>, HylaranaSenderError> {
        log::info!("create sender: options={:?}", options);

        let sender = HylaranaSender::new(options.clone(), sink)?;
        log::info!("create sender done: id={:?}", sender.get_id());

        Ok(sender)
    }

    /// To create a receiver, you need to specify the sender's ID to associate
    /// with it.
    pub fn create_receiver<T: AVFrameStream + 'static>(
        id: String,
        options: HylaranaReceiverOptions,
        sink: T,
    ) -> Result<HylaranaReceiver<T>, HylaranaReceiverError> {
        log::info!("create receiver: id={:?}, options={:?}", id, options);

        HylaranaReceiver::new(id, options.clone(), sink)
    }
}

#[cfg(target_os = "windows")]
static DIRECT_3D_DEVICE: RwLock<Option<Direct3DDevice>> = RwLock::new(None);

// Check if the D3D device has been created. If not, create a global one.
#[cfg(target_os = "windows")]
pub(crate) fn get_direct3d() -> Direct3DDevice {
    if DIRECT_3D_DEVICE.read().is_none() {
        DIRECT_3D_DEVICE
            .write()
            .replace(Direct3DDevice::new().expect("D3D device was not initialized successfully!"));
    }

    DIRECT_3D_DEVICE.read().as_ref().unwrap().clone()
}

#[derive(Debug, Error)]
pub enum AVFrameStreamPlayerError {
    #[error(transparent)]
    VideoRenderError(#[from] VideoRenderError),
    #[error(transparent)]
    AudioRenderError(#[from] AudioRenderError),
}

/// Configuration of the audio and video streaming player.
pub enum AVFrameStreamPlayerOptions<T> {
    /// Play video only.
    OnlyVideo(VideoRenderOptions<T>),
    /// Both audio and video will play.
    All(VideoRenderOptions<T>),
    /// Play audio only.
    OnlyAudio,
    /// Nothing plays.
    Quiet,
}

/// Player for audio and video streaming.
///
/// This player is used to quickly and easily create a player that implements
/// AVFrameStream, you only need to focus on the stream observer, the rest of
/// the player will be automatically hosted.
pub struct AVFrameStreamPlayer<'a, O> {
    video: Option<Mutex<VideoRender<'a>>>,
    audio: Option<AudioRender>,
    observer: O,
}

impl<'a, O> AVFrameStreamPlayer<'a, O>
where
    O: AVFrameObserver,
{
    pub fn new<T>(
        options: AVFrameStreamPlayerOptions<T>,
        observer: O,
    ) -> Result<Self, AVFrameStreamPlayerError>
    where
        T: Into<SurfaceTarget<'a>>,
    {
        Ok(Self {
            observer,
            audio: match options {
                AVFrameStreamPlayerOptions::All(_) | AVFrameStreamPlayerOptions::OnlyAudio => {
                    Some(AudioRender::new()?)
                }
                _ => None,
            },
            video: match options {
                AVFrameStreamPlayerOptions::All(options)
                | AVFrameStreamPlayerOptions::OnlyVideo(options) => {
                    Some(Mutex::new(VideoRender::new(options)?))
                }
                _ => None,
            },
        })
    }
}

impl<'a, O> AVFrameStream for AVFrameStreamPlayer<'a, O> where O: AVFrameObserver {}

impl<'a, O> AVFrameObserver for AVFrameStreamPlayer<'a, O>
where
    O: AVFrameObserver,
{
    fn close(&self) {
        self.observer.close();
    }
}

impl<'a, O> AVFrameSink for AVFrameStreamPlayer<'a, O>
where
    O: AVFrameObserver,
{
    fn audio(&self, frame: &AudioFrame) -> bool {
        if let Some(player) = &self.audio {
            if let Err(e) = player.send(frame) {
                log::error!("AVFrameStreamPlayer sink audio error={:?}", e);

                false
            } else {
                true
            }
        } else {
            true
        }
    }

    fn video(&self, frame: &VideoFrame) -> bool {
        if let Some(player) = &self.video {
            if let Err(e) = player.lock().send(frame) {
                log::error!("AVFrameStreamPlayer sink video error={:?}", e);

                false
            } else {
                true
            }
        } else {
            true
        }
    }
}

#[derive(Debug, Error)]
pub enum VideoRenderError {
    #[error(transparent)]
    #[cfg(target_os = "windows")]
    D3D11RendererError(#[from] renderer::win32::D3D11RendererError),
    #[error(transparent)]
    GraphicsError(#[from] renderer::GraphicsError),
    #[error("invalid d3d11texture2d texture")]
    #[cfg(target_os = "windows")]
    InvalidD3D11Texture,
    #[error("invalid backend")]
    InvalidBackend,
}

#[derive(Debug, Error)]
pub enum AudioRenderError {
    #[error("no output device available")]
    NotFoundOutputDevice,
    #[error(transparent)]
    StreamError(#[from] rodio::StreamError),
    #[error(transparent)]
    PlayError(#[from] rodio::PlayError),
    #[error("send audio queue error")]
    SendQueueError,
}

struct AudioSamples {
    sample_rate: u32,
    buffer: Vec<i16>,
    index: usize,
    frames: usize,
}

impl rodio::Source for AudioSamples {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.frames)
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl Iterator for AudioSamples {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.buffer.get(self.index).map(|it| *it);
        self.index += 1;
        item
    }
}

impl From<&AudioFrame> for AudioSamples {
    fn from(frame: &AudioFrame) -> Self {
        Self {
            buffer: unsafe { from_raw_parts(frame.data, frame.frames as usize) }.to_vec(),
            sample_rate: frame.sample_rate,
            frames: frame.frames as usize,
            index: 0,
        }
    }
}

/// Audio player that plays the original audio frames directly.
pub struct AudioRender {
    #[allow(dead_code)]
    stream: OutputStream,
    #[allow(dead_code)]
    stream_handle: OutputStreamHandle,
    sink: Sink,
}

unsafe impl Send for AudioRender {}
unsafe impl Sync for AudioRender {}

impl AudioRender {
    /// Create a audio player.
    pub fn new() -> Result<Self, AudioRenderError> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;

        sink.play();
        Ok(Self {
            stream_handle,
            stream,
            sink,
        })
    }

    /// Push an audio clip to the queue.
    pub fn send(&self, frame: &AudioFrame) -> Result<(), AudioRenderError> {
        self.sink.append(AudioSamples::from(frame));
        Ok(())
    }
}

impl Drop for AudioRender {
    fn drop(&mut self) {
        self.sink.pause();
    }
}

/// Back-end implementation of graphics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoRenderBackend {
    /// Backend implemented using D3D11, which is supported on an older device
    /// and platform and has better performance performance and memory
    /// footprint, but only on windows.
    Direct3D11,
    /// Cross-platform graphics backends implemented using WebGPUs are supported
    /// on a number of common platforms or devices.
    WebGPU,
}

impl ToString for VideoRenderBackend {
    fn to_string(&self) -> String {
        match self {
            Self::Direct3D11 => "d3d11",
            Self::WebGPU => "webgpu",
        }
        .to_string()
    }
}

impl FromStr for VideoRenderBackend {
    type Err = VideoRenderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "d3d11" => Self::Direct3D11,
            "webgpu" => Self::WebGPU,
            _ => return Err(VideoRenderError::InvalidBackend),
        })
    }
}

/// Video renderer configuration.
pub struct VideoRenderOptions<T> {
    /// The graphics backend used by the video renderer.
    pub backend: VideoRenderBackend,
    /// The size of the target window.
    pub size: Size,
    /// Renders the target's window.
    pub target: T,
}

/// Video player that can render video frames to window.
pub enum VideoRender<'a> {
    WebGPU(WgpuRenderer<'a>),
    #[cfg(target_os = "windows")]
    Direct3D11(D3D11Renderer),
}

impl<'a> VideoRender<'a> {
    /// Create a video player.
    pub fn new<T>(
        VideoRenderOptions {
            backend,
            size,
            target,
        }: VideoRenderOptions<T>,
    ) -> Result<Self, VideoRenderError>
    where
        T: Into<SurfaceTarget<'a>>,
    {
        log::info!(
            "create video render, backend={:?}, size={:?}",
            backend,
            size
        );

        #[cfg(target_os = "windows")]
        let direct3d = get_direct3d();

        Ok(match backend {
            #[cfg(target_os = "windows")]
            VideoRenderBackend::Direct3D11 => Self::Direct3D11(D3D11Renderer::new(
                match target.into() {
                    SurfaceTarget::Window(window) => match window.window_handle().unwrap().as_raw()
                    {
                        raw_window_handle::RawWindowHandle::Win32(window) => {
                            HWND(window.hwnd.get() as _)
                        }
                        _ => unimplemented!(
                            "what happened? why is the dx11 renderer enabled on linux?"
                        ),
                    },
                    _ => {
                        unimplemented!("the renderer does not support non-windowed render targets")
                    }
                },
                size,
                direct3d,
            )?),
            VideoRenderBackend::WebGPU => Self::WebGPU(WgpuRenderer::new(WgpuRendererOptions {
                window: target,
                #[cfg(target_os = "windows")]
                direct3d,
                size,
            })?),
            #[allow(unreachable_patterns)]
            _ => unimplemented!("not supports the {:?} backend", backend),
        })
    }

    /// Push video frames to the queue and the player will render them as
    /// quickly as possible, basically in real time.
    pub fn send(&mut self, frame: &VideoFrame) -> Result<(), VideoRenderError> {
        match frame.sub_format {
            #[cfg(target_os = "windows")]
            VideoSubFormat::D3D11 => {
                let texture = Texture2DResource::Texture(renderer::Texture2DRaw::ID3D11Texture2D(
                    d3d_texture_borrowed_raw(&(frame.data[0] as *mut _))
                        .ok_or_else(|| VideoRenderError::InvalidD3D11Texture)?
                        .clone(),
                    frame.data[1] as u32,
                ));

                let texture = match frame.format {
                    VideoFormat::BGRA => Texture::Bgra(texture),
                    VideoFormat::RGBA => Texture::Rgba(texture),
                    VideoFormat::NV12 => Texture::Nv12(texture),
                    VideoFormat::I420 => unimplemented!("no hardware texture for I420"),
                };

                match self {
                    Self::Direct3D11(render) => render.submit(texture)?,
                    Self::WebGPU(render) => render.submit(texture)?,
                }
            }
            #[cfg(target_os = "macos")]
            VideoSubFormat::CvPixelBufferRef => match self {
                Self::WebGPU(render) => {
                    render.submit(Texture::Nv12(Texture2DResource::Texture(
                        Texture2DRaw::CVPixelBufferRef(frame.data[0] as CVPixelBufferRef),
                    )))?
                }
            },
            VideoSubFormat::SW => {
                let buffers = match frame.format {
                    // RGBA stands for red green blue alpha. While it is sometimes described as a
                    // color space, it is actually a three-channel RGB color model supplemented
                    // with a fourth alpha channel. Alpha indicates how opaque each pixel is and
                    // allows an image to be combined over others using alpha compositing, with
                    // transparent areas and anti-aliasing of the edges of opaque regions. Each
                    // pixel is a 4D vector.
                    //
                    // The term does not define what RGB color space is being used. It also does
                    // not state whether or not the colors are premultiplied by the alpha value,
                    // and if they are it does not state what color space that premultiplication
                    // was done in. This means more information than just "RGBA" is needed to
                    // determine how to handle an image.
                    //
                    // In some contexts the abbreviation "RGBA" means a specific memory layout
                    // (called RGBA8888 below), with other terms such as "BGRA" used for
                    // alternatives. In other contexts "RGBA" means any layout.
                    VideoFormat::BGRA | VideoFormat::RGBA => [
                        unsafe {
                            from_raw_parts(
                                frame.data[0] as *const _,
                                frame.linesize[0] * frame.height as usize,
                            )
                        },
                        &[],
                        &[],
                    ],
                    // YCbCr, Y′CbCr, or Y Pb/Cb Pr/Cr, also written as YCBCR or Y′CBCR, is a
                    // family of color spaces used as a part of the color image pipeline in video
                    // and digital photography systems. Y′ is the luma component and CB and CR are
                    // the blue-difference and red-difference chroma components. Y′ (with prime) is
                    // distinguished from Y, which is luminance, meaning that light intensity is
                    // nonlinearly encoded based on gamma corrected RGB primaries.
                    //
                    // Y′CbCr color spaces are defined by a mathematical coordinate transformation
                    // from an associated RGB primaries and white point. If the underlying RGB
                    // color space is absolute, the Y′CbCr color space is an absolute color space
                    // as well; conversely, if the RGB space is ill-defined, so is Y′CbCr. The
                    // transformation is defined in equations 32, 33 in ITU-T H.273. Nevertheless
                    // that rule does not apply to P3-D65 primaries used by Netflix with
                    // BT.2020-NCL matrix, so that means matrix was not derived from primaries, but
                    // now Netflix allows BT.2020 primaries (since 2021).[1] The same happens with
                    // JPEG: it has BT.601 matrix derived from System M primaries, yet the
                    // primaries of most images are BT.709.
                    VideoFormat::NV12 => [
                        unsafe {
                            from_raw_parts(
                                frame.data[0] as *const _,
                                frame.linesize[0] * frame.height as usize,
                            )
                        },
                        unsafe {
                            from_raw_parts(
                                frame.data[1] as *const _,
                                frame.linesize[1] * frame.height as usize,
                            )
                        },
                        &[],
                    ],
                    VideoFormat::I420 => [
                        unsafe {
                            from_raw_parts(
                                frame.data[0] as *const _,
                                frame.linesize[0] * frame.height as usize,
                            )
                        },
                        unsafe {
                            from_raw_parts(
                                frame.data[1] as *const _,
                                frame.linesize[1] * (frame.height as usize / 2),
                            )
                        },
                        unsafe {
                            from_raw_parts(
                                frame.data[2] as *const _,
                                frame.linesize[2] * (frame.height as usize / 2),
                            )
                        },
                    ],
                };

                let texture = Texture2DBuffer {
                    buffers: &buffers,
                    size: Size {
                        width: frame.width,
                        height: frame.height,
                    },
                };

                let texture = match frame.format {
                    VideoFormat::BGRA => Texture::Bgra(Texture2DResource::Buffer(texture)),
                    VideoFormat::RGBA => Texture::Rgba(Texture2DResource::Buffer(texture)),
                    VideoFormat::NV12 => Texture::Nv12(Texture2DResource::Buffer(texture)),
                    VideoFormat::I420 => Texture::I420(texture),
                };

                match self {
                    #[cfg(target_os = "windows")]
                    Self::Direct3D11(render) => render.submit(texture)?,
                    Self::WebGPU(render) => render.submit(texture)?,
                }
            }
            #[allow(unreachable_patterns)]
            _ => unimplemented!("not suppports the frame format = {:?}", frame.sub_format),
        }

        Ok(())
    }
}
