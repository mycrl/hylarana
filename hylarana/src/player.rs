use std::{slice::from_raw_parts, sync::Arc};

use super::{
    HylaranaReceiverOptions, MediaStreamDescription, MediaStreamSink, sender::HylaranaSenderOptions,
};

#[cfg(target_os = "windows")]
use super::util::get_direct3d;

#[cfg(target_os = "windows")]
use common::win32::d3d_texture_borrowed_raw;

#[cfg(target_os = "macos")]
use common::macos::{CVPixelBufferRef, PixelMomeryBuffer};

#[cfg(not(target_os = "linux"))]
use renderer::Texture2DRaw;

use common::{
    Size,
    codec::{VideoDecoderType, VideoEncoderType},
    frame::{AudioFrame, VideoFormat, VideoFrame, VideoSubFormat},
};

use renderer::{
    Renderer, RendererOptions, RendererSourceOptions, RendererSurfaceOptions, SurfaceTarget,
    Texture, Texture2DBuffer, Texture2DResource,
};

use parking_lot::Mutex;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VideoRenderError {
    #[error(transparent)]
    GraphicsError(#[from] renderer::GraphicsError),
    #[error("invalid d3d11texture2d texture")]
    #[cfg(target_os = "windows")]
    InvalidD3D11Texture,
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

#[derive(Debug)]
pub struct VideoRenderSurfaceOptions<T> {
    pub window: T,
    pub size: Size,
}

#[derive(Debug)]
pub struct VideoRenderSourceOptions {
    pub size: Size,
    pub format: VideoFormat,
    pub sub_format: VideoSubFormat,
}

/// Video renderer configuration.
pub struct VideoRenderOptions<T> {
    pub surface: VideoRenderSurfaceOptions<T>,
    pub source: VideoRenderSourceOptions,
}

pub struct VideoRenderOptionsBuilder<T>(VideoRenderOptions<T>);

impl<T> VideoRenderOptionsBuilder<T> {
    pub fn new(surface: VideoRenderSurfaceOptions<T>) -> Self {
        Self(VideoRenderOptions {
            source: VideoRenderSourceOptions {
                size: Size::default(),
                sub_format: VideoSubFormat::SW,
                format: if cfg!(target_os = "macos") {
                    VideoFormat::BGRA
                } else {
                    VideoFormat::NV12
                },
            },
            surface,
        })
    }

    pub fn from_sender(mut self, options: &HylaranaSenderOptions) -> Self {
        if let Some(it) = &options.media.video {
            self.0.source.sub_format = match it.options.codec {
                VideoEncoderType::X265 => VideoSubFormat::SW,
                VideoEncoderType::Qsv => VideoSubFormat::D3D11,
                VideoEncoderType::VideoToolBox => VideoSubFormat::CvPixelBufferRef,
            };

            self.0.source.size = Size {
                width: it.options.width,
                height: it.options.height,
            };
        }

        self
    }

    pub fn from_receiver(
        mut self,
        description: &MediaStreamDescription,
        options: &HylaranaReceiverOptions,
    ) -> Self {
        if let Some(it) = description.video {
            self.0.source.format = it.format;
            self.0.source.size = it.size;
            self.0.source.sub_format = match options.codec {
                VideoDecoderType::HEVC => VideoSubFormat::SW,
                VideoDecoderType::Qsv | VideoDecoderType::D3D11 => {
                    if it.format == VideoFormat::I420 {
                        VideoSubFormat::SW
                    } else {
                        VideoSubFormat::D3D11
                    }
                }
                VideoDecoderType::VideoToolBox => {
                    if it.format == VideoFormat::BGRA || it.format == VideoFormat::RGBA {
                        VideoSubFormat::CvPixelBufferRef
                    } else {
                        VideoSubFormat::SW
                    }
                }
            };
        }

        self
    }

    pub fn build(self) -> VideoRenderOptions<T> {
        self.0
    }
}

/// Player for audio and video streaming.
///
/// This player is used to quickly and easily create a player that implements
/// AVFrameStream, you only need to focus on the stream observer, the rest of
/// the player will be automatically hosted.
pub struct AVFrameStreamPlayer<'a> {
    video: Option<Mutex<VideoRender<'a>>>,
    audio: Option<AudioRender>,
}

impl<'a> AVFrameStreamPlayer<'a> {
    pub fn new<T>(
        options: AVFrameStreamPlayerOptions<T>,
    ) -> Result<Arc<Self>, AVFrameStreamPlayerError>
    where
        T: Into<SurfaceTarget<'a>>,
    {
        Ok(Arc::new(Self {
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
        }))
    }

    pub fn resize(&self, size: Size) {
        if let Some(player) = &self.video {
            player.lock().resize(size);
        }
    }
}

impl<'a> MediaStreamSink for Arc<AVFrameStreamPlayer<'a>> {
    fn audio(&self, frame: &AudioFrame) -> bool {
        if let Some(player) = &self.audio {
            if let Err(e) = player.send(frame) {
                log::error!("AVFrameStreamPlayer sink audio error={:?}", e);

                return false;
            }
        }

        true
    }

    fn video(&self, frame: &VideoFrame) -> bool {
        if let Some(player) = &self.video {
            if let Some(mut player) = player.try_lock() {
                if let Err(e) = player.send(frame) {
                    log::error!("AVFrameStreamPlayer sink video error={:?}", e);

                    return false;
                }
            }
        }

        true
    }
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
        2
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
            index: 0,
            frames: frame.frames as usize,
            sample_rate: frame.sample_rate,
            buffer: unsafe { from_raw_parts(frame.data as *const i16, frame.frames as usize * 2) }
                .to_vec(),
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

/// Video player that can render video frames to window.
pub struct VideoRender<'a>(Renderer<'a>);

impl<'a> VideoRender<'a> {
    /// Create a video player.
    pub fn new<T>(
        VideoRenderOptions { surface, source }: VideoRenderOptions<T>,
    ) -> Result<Self, VideoRenderError>
    where
        T: Into<SurfaceTarget<'a>>,
    {
        log::info!("create video render, size={:?}", surface.size);

        #[cfg(target_os = "windows")]
        let direct3d = get_direct3d();

        let options = RendererOptions {
            #[cfg(target_os = "windows")]
            direct3d,
            surface: RendererSurfaceOptions {
                window: surface.window,
                size: surface.size,
            },
            source: RendererSourceOptions {
                size: source.size,
                format: source.format,
                sub_format: source.sub_format,
            },
        };

        Ok(Self(Renderer::new(options)?))
    }

    pub fn resize(&mut self, size: Size) {
        self.0.resize(size);
    }

    /// Push video frames to the queue and the player will render them as
    /// quickly as possible, basically in real time.
    pub fn send(&mut self, frame: &VideoFrame) -> Result<(), VideoRenderError> {
        match frame.sub_format {
            #[cfg(target_os = "windows")]
            VideoSubFormat::D3D11 => {
                let texture = Texture2DResource::Texture(Texture2DRaw::ID3D11Texture2D(
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

                self.0.submit(texture)?;
            }
            #[cfg(target_os = "macos")]
            VideoSubFormat::CvPixelBufferRef => match frame.format {
                VideoFormat::BGRA => {
                    self.0.submit(Texture::Bgra(Texture2DResource::Texture(
                        Texture2DRaw::CVPixelBufferRef(frame.data[0] as CVPixelBufferRef),
                    )))?;
                }
                VideoFormat::RGBA => {
                    self.0.submit(Texture::Rgba(Texture2DResource::Texture(
                        Texture2DRaw::CVPixelBufferRef(frame.data[0] as CVPixelBufferRef),
                    )))?;
                }
                _ => {
                    let pixel_buffer = PixelMomeryBuffer::from((
                        frame.data[0] as CVPixelBufferRef,
                        frame.format,
                        Size {
                            width: frame.width,
                            height: frame.height,
                        },
                    ));

                    let buffer = Texture2DBuffer {
                        buffers: &pixel_buffer.data,
                        linesize: &frame.linesize,
                    };

                    self.0.submit(match frame.format {
                        VideoFormat::NV12 => Texture::Nv12(Texture2DResource::Buffer(buffer)),
                        VideoFormat::I420 => Texture::I420(buffer),
                        _ => unreachable!(),
                    })?;
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
                                frame.linesize[0] as usize * frame.height as usize,
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
                                frame.linesize[0] as usize * frame.height as usize,
                            )
                        },
                        unsafe {
                            from_raw_parts(
                                frame.data[1] as *const _,
                                frame.linesize[1] as usize * frame.height as usize,
                            )
                        },
                        &[],
                    ],
                    VideoFormat::I420 => [
                        unsafe {
                            from_raw_parts(
                                frame.data[0] as *const _,
                                frame.linesize[0] as usize * frame.height as usize,
                            )
                        },
                        unsafe {
                            from_raw_parts(
                                frame.data[1] as *const _,
                                frame.linesize[1] as usize * (frame.height as usize / 2),
                            )
                        },
                        unsafe {
                            from_raw_parts(
                                frame.data[2] as *const _,
                                frame.linesize[2] as usize * (frame.height as usize / 2),
                            )
                        },
                    ],
                };

                let texture = Texture2DBuffer {
                    buffers: &buffers,
                    linesize: &frame.linesize,
                };

                let texture = match frame.format {
                    VideoFormat::BGRA => Texture::Bgra(Texture2DResource::Buffer(texture)),
                    VideoFormat::RGBA => Texture::Rgba(Texture2DResource::Buffer(texture)),
                    VideoFormat::NV12 => Texture::Nv12(Texture2DResource::Buffer(texture)),
                    VideoFormat::I420 => Texture::I420(texture),
                };

                self.0.submit(texture)?;
            }
            #[allow(unreachable_patterns)]
            _ => unimplemented!("not suppports the frame format = {:?}", frame.sub_format),
        }

        Ok(())
    }
}
