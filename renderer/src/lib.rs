mod generator;
mod transform;
mod vertex;

use std::sync::Arc;

use self::vertex::Vertex;

pub use self::generator::{
    GeneratorError, Texture, Texture2DBuffer, Texture2DRaw, Texture2DResource,
};

use common::{
    Size,
    frame::{VideoFormat, VideoSubFormat},
    runtime::get_runtime_handle,
};

use generator::{Generator, GeneratorOptions};
use thiserror::Error;
use wgpu::{
    Backends, Buffer, BufferUsages, Color, CommandEncoderDescriptor, CompositeAlphaMode, Device,
    DeviceDescriptor, IndexFormat, Instance, InstanceDescriptor, LoadOp, MemoryHints, Operations,
    PowerPreference, PresentMode, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    RequestAdapterOptions, StoreOp, Surface, TextureFormat, TextureUsages, TextureViewDescriptor,
    util::{BufferInitDescriptor, DeviceExt},
};

pub use wgpu::{SurfaceTarget, rwh as raw_window_handle};

#[derive(Debug, Error)]
pub enum GraphicsError {
    #[error("not found graphics adaper")]
    NotFoundAdapter,
    #[error("not found graphics surface default config")]
    NotFoundSurfaceDefaultConfig,
    #[error(transparent)]
    RequestDeviceError(#[from] wgpu::RequestDeviceError),
    #[error(transparent)]
    SurfaceGetTextureFailed(#[from] wgpu::SurfaceError),
    #[error(transparent)]
    CreateSurfaceError(#[from] wgpu::CreateSurfaceError),
    #[error(transparent)]
    GeneratorError(#[from] GeneratorError),
}

#[derive(Debug)]
pub struct RendererSurfaceOptions<T> {
    pub window: T,
    pub size: Size,
}

#[derive(Debug)]
pub struct RendererSourceOptions {
    pub size: Size,
    pub format: VideoFormat,
    pub sub_format: VideoSubFormat,
}

#[derive(Debug)]
pub struct RendererOptions<T> {
    #[cfg(target_os = "windows")]
    pub direct3d: common::win32::Direct3DDevice,
    pub surface: RendererSurfaceOptions<T>,
    pub source: RendererSourceOptions,
}

/// Window Renderer.
///
/// Supports rendering RGBA or NV12 hardware or software textures to system
/// native windows.
///
/// Note that the renderer uses a hardware implementation by default, i.e. it
/// uses the underlying GPU device, and the use of software devices is not
/// currently supported.
pub struct Renderer<'a> {
    surface: Surface<'a>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    generator: Generator,
}

impl<'a> Renderer<'a> {
    pub fn new<T: Into<SurfaceTarget<'a>>>(
        RendererOptions {
            #[cfg(target_os = "windows")]
            direct3d,
            surface: RendererSurfaceOptions { window, size },
            source,
        }: RendererOptions<T>,
    ) -> Result<Self, GraphicsError> {
        log::info!("create renderer, options={:?}", source);

        let instance = Instance::new(InstanceDescriptor {
            backends: if cfg!(target_os = "windows") {
                Backends::DX12
            } else if cfg!(target_os = "linux") {
                Backends::VULKAN
            } else {
                Backends::METAL
            },
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;
        let adapter = get_runtime_handle()
            .block_on(instance.request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::LowPower,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                ..Default::default()
            }))
            .ok_or_else(|| GraphicsError::NotFoundAdapter)?;

        let (device, queue) = get_runtime_handle().block_on(adapter.request_device(
            &DeviceDescriptor {
                label: None,
                memory_hints: MemoryHints::MemoryUsage,
                required_features: adapter.features(),
                required_limits: adapter.limits(),
            },
            None,
        ))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Configure surface as BGRA, BGRA this format compatibility is the best, in
        // order to unnecessary trouble, directly fixed to BGRA is the best.
        {
            let mut config = surface
                .get_default_config(&adapter, size.width, size.height)
                .ok_or_else(|| GraphicsError::NotFoundSurfaceDefaultConfig)?;

            config.present_mode = if cfg!(target_os = "windows") {
                PresentMode::Mailbox
            } else if cfg!(target_os = "linux") {
                PresentMode::Fifo
            } else {
                PresentMode::Immediate
            };

            config.format = TextureFormat::Bgra8Unorm;
            config.alpha_mode = CompositeAlphaMode::Opaque;
            config.usage = TextureUsages::RENDER_ATTACHMENT;
            surface.configure(&device, &config);
        };

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(Vertex::VERTICES),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(Vertex::INDICES),
            usage: BufferUsages::INDEX,
        });

        Ok(Self {
            generator: Generator::new(GeneratorOptions {
                device: device.clone(),
                queue: queue.clone(),
                size: source.size,
                format: source.format,
                sub_format: source.sub_format,
                #[cfg(target_os = "windows")]
                direct3d,
            })?,
            vertex_buffer,
            index_buffer,
            surface,
            device,
            queue,
        })
    }

    // Submit the texture to the renderer, it should be noted that the renderer will
    // not render this texture immediately, the processing flow will enter the
    // render queue and wait for the queue to automatically schedule the rendering
    // to the surface.
    pub fn submit(&mut self, texture: Texture) -> Result<(), GraphicsError> {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        let (pipeline, bind_group) = self.generator.get_view(&mut encoder, texture)?;
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, Some(&bind_group), &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..Vertex::INDICES.len() as u32, 0, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub mod win32 {
    use common::{
        frame::VideoFormat,
        win32::{
            Direct3DDevice,
            windows::Win32::{
                Foundation::HWND,
                Graphics::{
                    Direct3D11::{D3D11_VIEWPORT, ID3D11RenderTargetView, ID3D11Texture2D},
                    Dxgi::{
                        Common::DXGI_FORMAT_R8G8B8A8_UNORM, CreateDXGIFactory, DXGI_PRESENT,
                        DXGI_SWAP_CHAIN_DESC, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIFactory,
                        IDXGISwapChain,
                    },
                },
            },
        },
    };

    use resample::win32::{Resource, VideoResampler, VideoResamplerOptions};
    use thiserror::Error;
    use wgpu::SurfaceTarget;

    use crate::{RendererOptions, Texture, Texture2DRaw, Texture2DResource};

    #[derive(Debug, Error)]
    pub enum D3D11RendererError {
        #[error(transparent)]
        WindowsError(#[from] common::win32::windows::core::Error),
    }

    pub struct D3D11Renderer {
        direct3d: Direct3DDevice,
        swap_chain: IDXGISwapChain,
        render_target_view: ID3D11RenderTargetView,
        video_processor: VideoResampler,
    }

    unsafe impl Send for D3D11Renderer {}
    unsafe impl Sync for D3D11Renderer {}

    impl D3D11Renderer {
        pub fn new<'a, T: Into<SurfaceTarget<'a>>>(
            RendererOptions {
                direct3d,
                surface,
                source,
            }: RendererOptions<T>,
        ) -> Result<Self, D3D11RendererError> {
            let swap_chain = unsafe {
                let dxgi_factory = CreateDXGIFactory::<IDXGIFactory>()?;

                let mut desc = DXGI_SWAP_CHAIN_DESC::default();
                desc.BufferCount = 1;
                desc.BufferDesc.Width = surface.size.width;
                desc.BufferDesc.Height = surface.size.height;
                desc.BufferDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
                desc.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
                desc.SampleDesc.Count = 1;
                desc.Windowed = true.into();
                desc.OutputWindow = match surface.window.into() {
                    SurfaceTarget::Window(window) => match window.window_handle().unwrap().as_raw()
                    {
                        crate::raw_window_handle::RawWindowHandle::Win32(window) => {
                            HWND(window.hwnd.get() as _)
                        }
                        _ => unimplemented!(
                            "what happened? why is the dx11 renderer enabled on linux?"
                        ),
                    },
                    _ => {
                        unimplemented!("the renderer does not support non-windowed render targets")
                    }
                };

                let mut swap_chain = None;
                dxgi_factory
                    .CreateSwapChain(&direct3d.device, &desc, &mut swap_chain)
                    .ok()?;

                swap_chain.unwrap()
            };

            let back_buffer = unsafe { swap_chain.GetBuffer::<ID3D11Texture2D>(0)? };
            let render_target_view = unsafe {
                let mut render_target_view = None;
                direct3d.device.CreateRenderTargetView(
                    &back_buffer,
                    None,
                    Some(&mut render_target_view),
                )?;

                render_target_view.unwrap()
            };

            unsafe {
                direct3d
                    .context
                    .OMSetRenderTargets(Some(&[Some(render_target_view.clone())]), None);
            }

            unsafe {
                let mut vp = D3D11_VIEWPORT::default();
                vp.Width = surface.size.width as f32;
                vp.Height = surface.size.height as f32;
                vp.MinDepth = 0.0;
                vp.MaxDepth = 1.0;

                direct3d.context.RSSetViewports(Some(&[vp]));
            }

            let video_processor = VideoResampler::new(VideoResamplerOptions {
                direct3d: direct3d.clone(),
                input: Resource::Default(source.format, source.size),
                output: Resource::Texture(unsafe { swap_chain.GetBuffer::<ID3D11Texture2D>(0)? }),
            })?;

            Ok(Self {
                video_processor,
                render_target_view,
                swap_chain,
                direct3d,
            })
        }

        /// Draw this pixel buffer to the configured SurfaceTexture.
        pub fn submit(&mut self, texture: Texture) -> Result<(), D3D11RendererError> {
            unsafe {
                self.direct3d
                    .context
                    .ClearRenderTargetView(&self.render_target_view, &[0.0, 0.0, 0.0, 1.0]);
            }

            let format = match texture {
                Texture::Nv12(_) => VideoFormat::NV12,
                Texture::Rgba(_) => VideoFormat::RGBA,
                Texture::Bgra(_) => VideoFormat::BGRA,
                Texture::I420(_) => VideoFormat::I420,
            };

            let view = match texture {
                Texture::Nv12(resource) | Texture::Rgba(resource) | Texture::Bgra(resource) => {
                    match resource {
                        Texture2DResource::Texture(texture) => match texture {
                            Texture2DRaw::ID3D11Texture2D(texture, index) => {
                                Some(self.video_processor.create_input_view(&texture, index)?)
                            }
                        },
                        Texture2DResource::Buffer(texture) => {
                            self.video_processor.update_input_from_buffer(
                                format,
                                texture.buffers,
                                texture.linesize,
                            )?;

                            None
                        }
                    }
                }
                Texture::I420(texture) => {
                    self.video_processor.update_input_from_buffer(
                        format,
                        texture.buffers,
                        texture.linesize,
                    )?;

                    None
                }
            };

            self.video_processor.process(view)?;

            unsafe {
                self.swap_chain.Present(0, DXGI_PRESENT(0)).ok()?;
            }

            Ok(())
        }
    }
}
