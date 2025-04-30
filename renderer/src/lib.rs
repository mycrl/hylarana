mod backbuffer;
mod transform;
mod vertex;

use std::sync::Arc;

use self::vertex::Vertex;

pub use self::backbuffer::{
    BackBufferError, Texture, Texture2DBuffer, Texture2DRaw, Texture2DResource,
};

use common::{
    Size,
    frame::{VideoFormat, VideoSubFormat},
    runtime::get_runtime_handle,
};

use backbuffer::{BackBuffer, BackBufferOptions};
use thiserror::Error;
use wgpu::{
    Backends, Buffer, BufferUsages, Color, CommandEncoderDescriptor, CompositeAlphaMode, Device,
    DeviceDescriptor, IndexFormat, Instance, InstanceDescriptor, LoadOp, MemoryHints, Operations,
    PowerPreference, PresentMode, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    RequestAdapterOptions, StoreOp, Surface, SurfaceConfiguration, TextureFormat, TextureUsages,
    TextureViewDescriptor,
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
    BackBufferError(#[from] BackBufferError),
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
    config: SurfaceConfiguration,
    surface: Surface<'a>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    back_buffer: BackBuffer,
    viewport: Viewport,
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
        let viewport = Viewport::new(source.size, size);

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

        let back_buffer = BackBuffer::new(BackBufferOptions {
            device: device.clone(),
            queue: queue.clone(),
            size: source.size,
            format: source.format,
            sub_format: source.sub_format,
            #[cfg(target_os = "windows")]
            direct3d,
        })?;

        Ok(Self {
            viewport,
            back_buffer,
            vertex_buffer,
            index_buffer,
            surface,
            device,
            queue,
            config,
        })
    }

    pub fn resize(&mut self, size: Size) {
        self.viewport.resize(size);

        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    // Submit the texture to the renderer, it should be noted that the renderer will
    // not render this texture immediately, the processing flow will enter the
    // render queue and wait for the queue to automatically schedule the rendering
    // to the surface.
    pub fn submit(&mut self, texture: Texture) -> Result<(), GraphicsError> {
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        let (pipeline, bind_group) = self.back_buffer.get_view(&mut encoder, texture)?;
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

            render_pass.set_viewport(
                self.viewport.x,
                self.viewport.y,
                self.viewport.width,
                self.viewport.height,
                0.0,
                1.0,
            );

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

#[derive(Debug)]
struct Viewport {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    texture: Size,
}

impl Viewport {
    fn new(texture: Size, surface: Size) -> Self {
        let (texture_width, texture_height, surface_width, surface_height) = (
            texture.width as f32,
            texture.height as f32,
            surface.width as f32,
            surface.height as f32,
        );

        let texture_ratio = texture_width / texture_height;
        let surface_ratio = surface_width / surface_height;

        let (width, height, x, y) = if texture_ratio > surface_ratio {
            let width = surface_width;
            let height = surface_width / texture_ratio;
            let y = (surface_height - height) / 2.0;
            (width, height, 0.0, y)
        } else {
            let height = surface_height;
            let width = surface_height * texture_ratio;
            let x = (surface_width - width) / 2.0;
            (width, height, x, 0.0)
        };

        Self {
            texture,
            x,
            y,
            width,
            height,
        }
    }

    fn resize(&mut self, surface: Size) {
        *self = Self::new(self.texture, surface);
    }
}
