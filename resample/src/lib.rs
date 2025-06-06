use std::{
    marker::PhantomData,
    ptr::null_mut,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Sender, channel},
    },
    thread,
};

use ffmpeg::*;
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub enum AudioSampleFormat {
    I16,
    I32,
    F32,
}

impl Into<AVSampleFormat> for AudioSampleFormat {
    fn into(self) -> AVSampleFormat {
        match self {
            Self::I16 => AVSampleFormat::AV_SAMPLE_FMT_S16,
            Self::I32 => AVSampleFormat::AV_SAMPLE_FMT_S32,
            Self::F32 => AVSampleFormat::AV_SAMPLE_FMT_FLT,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AudioSampleDescription {
    pub sample_bits: AudioSampleFormat,
    pub sample_rate: u32,
    pub channels: u8,
}

impl AudioSampleDescription {
    fn channel_layout(&self) -> AVChannelLayout {
        AVChannelLayout {
            order: AVChannelOrder::AV_CHANNEL_ORDER_NATIVE,
            nb_channels: self.channels as i32,
            u: AVChannelLayout__bindgen_ty_1 {
                mask: match self.channels {
                    1 => AV_CH_LAYOUT_MONO,
                    2 => AV_CH_LAYOUT_STEREO,
                    _ => unimplemented!("unsupports audio channels={}", self.channels),
                },
            },
            opaque: null_mut(),
        }
    }
}

pub trait AudioResamplerOutput<T>: Send {
    fn output(&mut self, buffer: &[T], frames: u32) -> bool;
}

#[derive(Debug, Error)]
pub enum AudioResamplerError {
    #[error("failed to send buffer to queue")]
    SendBufferError,
    #[error("failed to create swresample")]
    CreateSwresampleError,
    #[error("queue is closed")]
    QueueClosed,
}

/// Audio resampler, quickly resample input to a single channel count and
/// different sampling rates.
///
/// Note that due to the fast sampling, the quality may be reduced.
pub struct AudioResampler<I, O> {
    _p: PhantomData<O>,
    tx: Sender<Vec<I>>,
    status: Arc<AtomicBool>,
}

impl<I, O> AudioResampler<I, O>
where
    I: Copy + Send + 'static,
    O: Copy + Default,
{
    pub fn new<T: AudioResamplerOutput<O> + 'static>(
        input: AudioSampleDescription,
        output: AudioSampleDescription,
        mut sink: T,
    ) -> Result<Self, AudioResamplerError> {
        let (tx, rx) = channel::<Vec<I>>();

        let status = Arc::new(AtomicBool::new(true));
        let mut swresample = Swresample::new(&input, &output)
            .ok_or_else(|| AudioResamplerError::CreateSwresampleError)?;

        let status_ = status.clone();
        thread::spawn(move || {
            let mut output: Vec<O> =
                vec![O::default(); output.sample_rate as usize * output.channels as usize];

            while let Ok(buffer) = rx.recv() {
                let frames = buffer.len() / input.channels as usize;
                if swresample.convert(&buffer, &mut output, frames as i32) {
                    if !sink.output(&output, frames as u32) {
                        break;
                    }
                } else {
                    break;
                }
            }

            status_.store(false, Ordering::Relaxed);
        });

        Ok(Self {
            _p: PhantomData::default(),
            status,
            tx,
        })
    }

    pub fn resample<'a>(&'a mut self, buffer: &'a [I]) -> Result<(), AudioResamplerError> {
        if !self.status.load(Ordering::Relaxed) {
            return Err(AudioResamplerError::QueueClosed);
        }

        self.tx
            .send(buffer.to_vec())
            .map_err(|_| AudioResamplerError::SendBufferError)?;
        Ok(())
    }
}

struct Swresample(*mut SwrContext);

unsafe impl Send for Swresample {}
unsafe impl Sync for Swresample {}

impl Swresample {
    fn new(input: &AudioSampleDescription, output: &AudioSampleDescription) -> Option<Self> {
        let mut ctx = null_mut();
        if unsafe {
            swr_alloc_set_opts2(
                &mut ctx,
                &output.channel_layout(),
                output.sample_bits.into(),
                output.sample_rate as i32,
                &input.channel_layout(),
                input.sample_bits.into(),
                output.sample_rate as i32,
                0,
                null_mut(),
            )
        } != 0
        {
            return None;
        }

        if unsafe { swr_init(ctx) } != 0 {
            return None;
        }

        Some(Self(ctx))
    }

    fn convert<I, O>(&mut self, input: &[I], output: &mut [O], frames: i32) -> bool {
        unsafe {
            swr_convert(
                self.0,
                [output.as_mut_ptr() as _].as_ptr(),
                frames,
                [input.as_ptr() as _].as_ptr(),
                frames,
            ) >= 0
        }
    }
}

impl Drop for Swresample {
    fn drop(&mut self) {
        unsafe {
            swr_free(&mut self.0);
        }
    }
}

#[cfg(target_os = "windows")]
pub mod win32 {
    use std::mem::ManuallyDrop;

    use common::{
        Size,
        frame::VideoFormat,
        win32::{
            Direct3DDevice,
            windows::{
                Win32::{
                    Foundation::RECT,
                    Graphics::{
                        Direct3D11::{
                            D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE,
                            D3D11_CPU_ACCESS_READ, D3D11_CPU_ACCESS_WRITE, D3D11_MAP_READ,
                            D3D11_MAP_WRITE_DISCARD, D3D11_MAPPED_SUBRESOURCE,
                            D3D11_RESOURCE_MISC_SHARED, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
                            D3D11_USAGE_DYNAMIC, D3D11_USAGE_STAGING,
                            D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
                            D3D11_VIDEO_PROCESSOR_COLOR_SPACE, D3D11_VIDEO_PROCESSOR_CONTENT_DESC,
                            D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC,
                            D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC, D3D11_VIDEO_PROCESSOR_STREAM,
                            D3D11_VIDEO_USAGE_PLAYBACK_NORMAL, D3D11_VPIV_DIMENSION_TEXTURE2D,
                            D3D11_VPOV_DIMENSION_TEXTURE2D, ID3D11Device, ID3D11DeviceContext,
                            ID3D11Texture2D, ID3D11VideoContext, ID3D11VideoDevice,
                            ID3D11VideoProcessor, ID3D11VideoProcessorEnumerator,
                            ID3D11VideoProcessorInputView, ID3D11VideoProcessorOutputView,
                        },
                        Dxgi::Common::{
                            DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_NV12,
                            DXGI_FORMAT_R8G8B8A8_UNORM,
                        },
                    },
                },
                core::{Error, Interface},
            },
        },
    };

    #[derive(Clone)]
    pub enum Resource {
        Default(VideoFormat, Size),
        Texture(ID3D11Texture2D),
    }

    pub struct VideoResamplerOptions {
        pub direct3d: Direct3DDevice,
        pub input: Resource,
        pub output: Resource,
    }

    /// Used to convert video frames using hardware accelerators, including
    /// color space conversion and scaling. Note that the output is fixed to
    /// NV12, but the input is optional and is RGBA by default. However, if
    /// you use the `process` method, you can let the external texture
    /// decide what format to use, because this method does not copy the
    /// texture.
    #[allow(unused)]
    pub struct VideoResampler {
        d3d_device: ID3D11Device,
        d3d_context: ID3D11DeviceContext,
        video_device: ID3D11VideoDevice,
        video_context: ID3D11VideoContext,
        input_texture: ID3D11Texture2D,
        input_sw_texture: Option<ID3D11Texture2D>,
        output_texture: ID3D11Texture2D,
        output_sw_texture: Option<ID3D11Texture2D>,
        video_enumerator: ID3D11VideoProcessorEnumerator,
        video_processor: ID3D11VideoProcessor,
        input_view: ID3D11VideoProcessorInputView,
        output_view: ID3D11VideoProcessorOutputView,
    }

    unsafe impl Send for VideoResampler {}
    unsafe impl Sync for VideoResampler {}

    impl VideoResampler {
        /// Create `VideoResampler`, the default_device parameter is used to
        /// directly use the device when it has been created externally, so
        /// there is no need to copy across devices, which improves
        /// processing performance.
        pub fn new(options: VideoResamplerOptions) -> Result<Self, Error> {
            let (d3d_device, d3d_context) = (options.direct3d.device, options.direct3d.context);
            let video_device = d3d_device.cast::<ID3D11VideoDevice>()?;
            let video_context = d3d_context.cast::<ID3D11VideoContext>()?;

            let input_texture = match options.input.clone() {
                Resource::Texture(texture) => texture,
                Resource::Default(format, size) => unsafe {
                    let mut desc = D3D11_TEXTURE2D_DESC::default();
                    desc.Width = size.width;
                    desc.Height = size.height;
                    desc.MipLevels = 1;
                    desc.ArraySize = 1;
                    desc.SampleDesc.Count = 1;
                    desc.SampleDesc.Quality = 0;
                    desc.Usage = D3D11_USAGE_DEFAULT;
                    desc.BindFlags = D3D11_BIND_RENDER_TARGET.0 as u32;
                    desc.CPUAccessFlags = 0;
                    desc.MiscFlags = 0;
                    desc.Format = video_fmt_to_dxgi_fmt(format);

                    let mut texture = None;
                    d3d_device.CreateTexture2D(&desc, None, Some(&mut texture))?;
                    texture.unwrap()
                },
            };

            let input_sw_texture = match options.input {
                Resource::Default(format, size)
                    if format == VideoFormat::NV12 || format == VideoFormat::I420 =>
                {
                    let mut desc = D3D11_TEXTURE2D_DESC::default();
                    desc.Width = size.width;
                    desc.Height = size.height;
                    desc.MipLevels = 1;
                    desc.ArraySize = 1;
                    desc.SampleDesc.Count = 1;
                    desc.SampleDesc.Quality = 0;
                    desc.Usage = D3D11_USAGE_DYNAMIC;
                    desc.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE.0 as u32;
                    desc.BindFlags = D3D11_BIND_SHADER_RESOURCE.0 as u32;
                    desc.MiscFlags = 0;
                    desc.Format = video_fmt_to_dxgi_fmt(format);

                    let mut texture = None;
                    unsafe {
                        d3d_device.CreateTexture2D(&desc, None, Some(&mut texture))?;
                    }

                    Some(texture.unwrap())
                }
                _ => None,
            };

            let output_texture = match options.output {
                Resource::Texture(texture) => texture,
                Resource::Default(format, size) => unsafe {
                    let mut desc = D3D11_TEXTURE2D_DESC::default();
                    desc.Width = size.width;
                    desc.Height = size.height;
                    desc.MipLevels = 1;
                    desc.ArraySize = 1;
                    desc.SampleDesc.Count = 1;
                    desc.SampleDesc.Quality = 0;
                    desc.Usage = D3D11_USAGE_DEFAULT;
                    desc.BindFlags = D3D11_BIND_RENDER_TARGET.0 as u32;
                    desc.CPUAccessFlags = 0;
                    desc.MiscFlags = D3D11_RESOURCE_MISC_SHARED.0 as u32;
                    desc.Format = video_fmt_to_dxgi_fmt(format);

                    let mut texture = None;
                    d3d_device.CreateTexture2D(&desc, None, Some(&mut texture))?;
                    texture.unwrap()
                },
            };

            let mut input_desc = D3D11_TEXTURE2D_DESC::default();
            unsafe {
                input_texture.GetDesc(&mut input_desc);
            }

            let mut output_desc = D3D11_TEXTURE2D_DESC::default();
            unsafe {
                output_texture.GetDesc(&mut output_desc);
            }

            let (video_enumerator, video_processor) = unsafe {
                let mut desc = D3D11_VIDEO_PROCESSOR_CONTENT_DESC::default();
                desc.InputFrameFormat = D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE;
                desc.InputWidth = input_desc.Width;
                desc.InputHeight = input_desc.Height;
                desc.OutputWidth = output_desc.Width;
                desc.OutputHeight = output_desc.Height;
                desc.Usage = D3D11_VIDEO_USAGE_PLAYBACK_NORMAL;

                let enumerator = video_device.CreateVideoProcessorEnumerator(&desc)?;
                let processor = video_device.CreateVideoProcessor(&enumerator, 0)?;
                (enumerator, processor)
            };

            let input_view = unsafe {
                let mut desc = D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC::default();
                desc.FourCC = 0;
                desc.ViewDimension = D3D11_VPIV_DIMENSION_TEXTURE2D;
                desc.Anonymous.Texture2D.MipSlice = 0;

                let mut view = None;
                video_device.CreateVideoProcessorInputView(
                    &input_texture,
                    &video_enumerator,
                    &desc,
                    Some(&mut view),
                )?;

                view.unwrap()
            };

            let output_view = unsafe {
                let mut desc = D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC::default();
                desc.ViewDimension = D3D11_VPOV_DIMENSION_TEXTURE2D;

                let mut view = None;
                video_device.CreateVideoProcessorOutputView(
                    &output_texture,
                    &video_enumerator,
                    &desc,
                    Some(&mut view),
                )?;

                view.unwrap()
            };

            unsafe {
                video_context.VideoProcessorSetStreamSourceRect(
                    &video_processor,
                    0,
                    true,
                    Some(&RECT {
                        left: 0,
                        top: 0,
                        right: input_desc.Width as i32,
                        bottom: input_desc.Height as i32,
                    }),
                );
            }

            unsafe {
                video_context.VideoProcessorSetStreamDestRect(
                    &video_processor,
                    0,
                    true,
                    Some(&RECT {
                        left: 0,
                        top: 0,
                        right: output_desc.Width as i32,
                        bottom: output_desc.Height as i32,
                    }),
                );
            }

            unsafe {
                let color_space = D3D11_VIDEO_PROCESSOR_COLOR_SPACE::default();
                video_context.VideoProcessorSetStreamColorSpace(&video_processor, 0, &color_space);
            }

            Ok(Self {
                output_sw_texture: None,
                input_sw_texture,
                d3d_device,
                d3d_context,
                video_device,
                video_context,
                video_enumerator,
                video_processor,
                input_texture,
                output_texture,
                input_view,
                output_view,
            })
        }

        /// To update the internal texture, simply copy it to the internal
        /// texture.
        pub fn update_input(&mut self, texture: &ID3D11Texture2D) {
            unsafe {
                self.d3d_context.CopyResource(&self.input_texture, texture);
            }
        }

        /// Perform the conversion. This method will copy the texture array to
        /// the internal texture, so there are restrictions on the
        /// format of the incoming texture. Because the internal one is
        /// fixed to RGBA, the external texture can only be RGBA.
        pub fn update_input_from_buffer(
            &mut self,
            format: VideoFormat,
            data: &[&[u8]],
            linesize: &[u32],
        ) -> Result<(), Error> {
            match format {
                VideoFormat::BGRA | VideoFormat::RGBA => unsafe {
                    self.d3d_context.UpdateSubresource(
                        &self.input_texture,
                        0,
                        None,
                        data[0].as_ptr() as *const _,
                        linesize[0],
                        0,
                    );
                },
                // Although NV12 separates the two planes, usually memory is contiguous and
                // is treated uniformly here, but of course there are contingencies, and
                // this is not a good implementation here, but in most cases there will be
                // one less copy step.
                VideoFormat::NV12 => {
                    if is_single_allocation(&data[0..2]) {
                        unsafe {
                            self.d3d_context.UpdateSubresource(
                                &self.input_texture,
                                0,
                                None,
                                data[0].as_ptr() as *const _,
                                linesize[0],
                                0,
                            );
                        }
                    } else {
                        if let Some(texture) = &self.input_sw_texture {
                            let mut mappend = D3D11_MAPPED_SUBRESOURCE::default();
                            unsafe {
                                self.d3d_context.Map(
                                    texture,
                                    0,
                                    D3D11_MAP_WRITE_DISCARD,
                                    0,
                                    Some(&mut mappend),
                                )?;
                            }

                            unsafe {
                                std::slice::from_raw_parts_mut(
                                    mappend.pData as *mut u8,
                                    data[0].len(),
                                )
                            }
                            .copy_from_slice(data[0]);

                            unsafe {
                                std::slice::from_raw_parts_mut(
                                    mappend.pData.add(data[0].len()) as *mut u8,
                                    data[1].len(),
                                )
                            }
                            .copy_from_slice(data[1]);

                            unsafe {
                                self.d3d_context.Unmap(texture, 0);
                                self.d3d_context.CopyResource(&self.input_texture, texture);
                            }
                        }
                    }
                }
                VideoFormat::I420 => {
                    if let Some(texture) = &self.input_sw_texture {
                        let mut mappend = D3D11_MAPPED_SUBRESOURCE::default();
                        unsafe {
                            self.d3d_context.Map(
                                texture,
                                0,
                                D3D11_MAP_WRITE_DISCARD,
                                0,
                                Some(&mut mappend),
                            )?;
                        }

                        unsafe {
                            std::slice::from_raw_parts_mut(mappend.pData as *mut u8, data[0].len())
                        }
                        .copy_from_slice(data[0]);

                        {
                            let buffer = unsafe {
                                std::slice::from_raw_parts_mut(
                                    mappend.pData.add(data[0].len()) as *mut u8,
                                    data[1].len() + data[2].len(),
                                )
                            };

                            let mut index = 0;
                            for i in 0..data[1].len() {
                                buffer[index] = data[1][i];
                                buffer[index + 1] = data[2][i];
                                index += 2;
                            }
                        }

                        unsafe {
                            self.d3d_context.Unmap(texture, 0);
                            self.d3d_context.CopyResource(&self.input_texture, texture);
                        }
                    }
                }
            };

            Ok(())
        }

        /// Perform the conversion. This method will not copy the passed
        /// texture, but will use the texture directly, which can save a
        /// copy step and improve performance.
        pub fn create_input_view(
            &mut self,
            texture: &ID3D11Texture2D,
            index: u32,
        ) -> Result<ID3D11VideoProcessorInputView, Error> {
            let input_view = unsafe {
                let mut desc = D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC::default();
                desc.FourCC = 0;
                desc.ViewDimension = D3D11_VPIV_DIMENSION_TEXTURE2D;
                desc.Anonymous.Texture2D.MipSlice = 0;
                desc.Anonymous.Texture2D.ArraySlice = index;

                let mut view = None;
                self.video_device.CreateVideoProcessorInputView(
                    texture,
                    &self.video_enumerator,
                    &desc,
                    Some(&mut view),
                )?;

                view.unwrap()
            };

            Ok(input_view)
        }

        pub fn get_output(&self) -> &ID3D11Texture2D {
            &self.output_texture
        }

        pub fn get_output_buffer(&mut self) -> Result<TextureBuffer, Error> {
            if self.output_sw_texture.is_none() {
                unsafe {
                    let mut desc = D3D11_TEXTURE2D_DESC::default();
                    self.output_texture.GetDesc(&mut desc);

                    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
                    desc.Usage = D3D11_USAGE_STAGING;
                    desc.BindFlags = 0;
                    desc.MiscFlags = 0;

                    let mut texture = None;
                    self.d3d_device
                        .CreateTexture2D(&desc, None, Some(&mut texture))?;

                    self.output_sw_texture = Some(texture.unwrap());
                };
            }

            let texture = self.output_sw_texture.as_ref().unwrap();
            unsafe {
                self.d3d_context.CopyResource(texture, &self.output_texture);
            }

            Ok(TextureBuffer::new(&self.d3d_context, texture)?)
        }

        pub fn process(
            &mut self,
            input_view: Option<ID3D11VideoProcessorInputView>,
        ) -> Result<(), Error> {
            unsafe {
                let mut streams = [D3D11_VIDEO_PROCESSOR_STREAM::default()];
                streams[0].Enable = true.into();
                streams[0].OutputIndex = 0;
                streams[0].InputFrameOrField = 0;
                streams[0].pInputSurface =
                    ManuallyDrop::new(Some(input_view.unwrap_or_else(|| self.input_view.clone())));

                self.video_context.VideoProcessorBlt(
                    &self.video_processor,
                    &self.output_view,
                    0,
                    &streams,
                )?;

                ManuallyDrop::drop(&mut streams[0].pInputSurface);
            }

            Ok(())
        }
    }

    pub struct TextureBuffer<'a> {
        d3d_context: &'a ID3D11DeviceContext,
        texture: &'a ID3D11Texture2D,
        resource: D3D11_MAPPED_SUBRESOURCE,
    }

    unsafe impl Send for TextureBuffer<'_> {}
    unsafe impl Sync for TextureBuffer<'_> {}

    impl<'a> TextureBuffer<'a> {
        pub fn new(
            d3d_context: &'a ID3D11DeviceContext,
            texture: &'a ID3D11Texture2D,
        ) -> Result<Self, Error> {
            let mut resource = D3D11_MAPPED_SUBRESOURCE::default();
            unsafe {
                d3d_context.Map(texture, 0, D3D11_MAP_READ, 0, Some(&mut resource))?;
            }

            Ok(Self {
                d3d_context,
                resource,
                texture,
            })
        }

        /// Represents a pointer to texture data. Internally, the texture is
        /// copied to the CPU first, and then the internal data is
        /// mapped.
        pub fn buffer(&self) -> *const u8 {
            self.resource.pData as *const _
        }

        /// The stride of the texture data
        pub fn stride(&self) -> u32 {
            self.resource.RowPitch
        }
    }

    impl Drop for TextureBuffer<'_> {
        fn drop(&mut self) {
            unsafe {
                self.d3d_context.Unmap(self.texture, 0);
            }
        }
    }

    fn is_single_allocation<T>(source: &[&[T]]) -> bool {
        let mut size = 0;
        let mut offset = 0;

        for it in source {
            if size > 0 {
                if offset + size != it.as_ptr() as usize {
                    return false;
                }
            }

            size = it.len();
            offset = it.as_ptr() as usize;
        }

        true
    }

    fn video_fmt_to_dxgi_fmt(format: VideoFormat) -> DXGI_FORMAT {
        match format {
            VideoFormat::NV12 | VideoFormat::I420 => DXGI_FORMAT_NV12,
            VideoFormat::RGBA => DXGI_FORMAT_R8G8B8A8_UNORM,
            VideoFormat::BGRA => DXGI_FORMAT_B8G8R8A8_UNORM,
        }
    }
}
