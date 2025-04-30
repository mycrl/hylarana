mod texture;

use std::{borrow::Cow, sync::Arc};

use self::texture::{bgra::Bgra, i420::I420, nv12::Nv12, rgba::Rgba};
use crate::{Vertex, transform::TransformError};

#[cfg(target_os = "windows")]
use crate::transform::direct3d::Transformer;

#[cfg(target_os = "macos")]
use crate::transform::metal::Transformer;

use common::{
    Size,
    frame::{VideoFormat, VideoSubFormat},
};

use smallvec::SmallVec;
use thiserror::Error;

#[cfg(target_os = "macos")]
use common::macos::CVPixelBufferRef;

#[cfg(target_os = "windows")]
use common::win32::{Direct3DDevice, windows::Win32::Graphics::Direct3D11::ID3D11Texture2D};

use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    ColorTargetState, ColorWrites, CommandEncoder, Device, Extent3d, FilterMode, FragmentState,
    ImageCopyTexture, ImageDataLayout, IndexFormat, MultisampleState, Origin3d,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, Queue,
    RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, Texture as WGPUTexture, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureView, TextureViewDescriptor, TextureViewDimension, VertexState,
};

#[derive(Debug, Error)]
pub enum BackBufferError {
    #[error(transparent)]
    TransformError(#[from] TransformError),
}

#[derive(Debug)]
pub enum Texture2DRaw {
    #[cfg(target_os = "windows")]
    ID3D11Texture2D(ID3D11Texture2D, u32),
    #[cfg(target_os = "macos")]
    CVPixelBufferRef(CVPixelBufferRef),
}

#[derive(Debug)]
pub struct Texture2DBuffer<'a> {
    pub buffers: &'a [&'a [u8]],
    pub linesize: &'a [u32],
}

#[derive(Debug)]
pub enum Texture2DResource<'a> {
    Texture(Texture2DRaw),
    Buffer(Texture2DBuffer<'a>),
}

#[derive(Debug)]
pub enum Texture<'a> {
    Bgra(Texture2DResource<'a>),
    Rgba(Texture2DResource<'a>),
    Nv12(Texture2DResource<'a>),
    I420(Texture2DBuffer<'a>),
}

trait Texture2DSample {
    const VIEWS_COUNT: usize;

    fn fragment_shader() -> ShaderModuleDescriptor<'static>;
    fn create_texture_descriptor(
        size: Size,
        sub_format: VideoSubFormat,
    ) -> impl IntoIterator<Item = (Size, TextureFormat)>;

    fn views_descriptors<'a>(
        &'a self,
        texture: Option<&'a WGPUTexture>,
    ) -> impl IntoIterator<Item = (&'a WGPUTexture, TextureFormat, TextureAspect)>;

    fn copy_buffer_descriptors<'a>(
        &self,
        buffers: &'a [&'a [u8]],
    ) -> impl IntoIterator<Item = (&'a [u8], &WGPUTexture, TextureAspect, Size)>;

    fn create(
        device: &Device,
        size: Size,
        sub_format: VideoSubFormat,
    ) -> impl Iterator<Item = WGPUTexture> {
        Self::create_texture_descriptor(size, sub_format)
            .into_iter()
            .map(|(size, format)| {
                device.create_texture(&TextureDescriptor {
                    label: None,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    // The textures created here are all needed to allow external writing of data,
                    // and all need the COPY_DST flag.
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                    view_formats: &[],
                    size: Extent3d {
                        depth_or_array_layers: 1,
                        width: size.width,
                        height: size.height,
                    },
                    format,
                })
            })
    }

    /// Creates a new BindGroupLayout.
    ///
    /// A BindGroupLayout is a handle to the GPU-side layout of a binding group.
    /// It can be used to create a BindGroupOptions object, which in turn can
    /// be used to create a BindGroup object with Device::create_bind_group. A
    /// series of BindGroupLayouts can also be used to create a
    /// PipelineLayoutOptions, which can be used to create a PipelineLayout.
    fn bind_group_layout(&self, device: &Device) -> BindGroupLayout {
        let mut entries: SmallVec<[BindGroupLayoutEntry; 5]> = SmallVec::with_capacity(5);
        for i in 0..Self::VIEWS_COUNT {
            entries.push(BindGroupLayoutEntry {
                count: None,
                binding: i as u32,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
            });
        }

        entries.push(BindGroupLayoutEntry {
            binding: entries.len() as u32,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        });

        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &entries,
        })
    }

    /// Creates a new BindGroup.
    ///
    /// A BindGroup represents the set of resources bound to the bindings
    /// described by a BindGroupLayout. It can be created with
    /// Device::create_bind_group. A BindGroup can be bound to a particular
    /// RenderPass with RenderPass::set_bind_group, or to a ComputePass with
    /// ComputePass::set_bind_group.
    fn bind_group(
        &self,
        device: &Device,
        sampler: &Sampler,
        layout: &BindGroupLayout,
        texture: Option<&WGPUTexture>,
    ) -> BindGroup {
        let mut views: SmallVec<[TextureView; 5]> = SmallVec::with_capacity(5);
        for (texture, format, aspect) in self.views_descriptors(texture) {
            views.push(texture.create_view(&TextureViewDescriptor {
                dimension: Some(TextureViewDimension::D2),
                format: Some(format),
                aspect,
                ..Default::default()
            }));
        }

        let mut entries: SmallVec<[BindGroupEntry; 5]> = SmallVec::with_capacity(5);
        for (i, view) in views.iter().enumerate() {
            entries.push(BindGroupEntry {
                binding: i as u32,
                resource: BindingResource::TextureView(view),
            });
        }

        entries.push(BindGroupEntry {
            binding: entries.len() as u32,
            resource: BindingResource::Sampler(sampler),
        });

        device.create_bind_group(&BindGroupDescriptor {
            label: None,
            entries: &entries,
            layout,
        })
    }

    /// Schedule a write of some data into a texture.
    fn update(&self, queue: &Queue, resource: &Texture2DBuffer) {
        for (buffer, texture, aspect, size) in self.copy_buffer_descriptors(resource.buffers) {
            queue.write_texture(
                ImageCopyTexture {
                    aspect,
                    texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                },
                buffer,
                ImageDataLayout {
                    offset: 0,
                    // Bytes per "row" in an image.
                    //
                    // A row is one row of pixels or of compressed blocks in the x direction.
                    bytes_per_row: Some(size.width),
                    rows_per_image: Some(size.height),
                },
                texture.size(),
            );
        }
    }
}

enum Texture2DSourceSample {
    Bgra(Bgra),
    Rgba(Rgba),
    Nv12(Nv12),
    I420(I420),
}

pub struct BackBufferOptions {
    #[cfg(target_os = "windows")]
    pub direct3d: Direct3DDevice,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub format: VideoFormat,
    pub sub_format: VideoSubFormat,
    pub size: Size,
}

pub struct BackBuffer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    sampler: Sampler,
    layout: BindGroupLayout,
    pipeline: RenderPipeline,
    sample: Texture2DSourceSample,
    #[cfg(not(target_os = "linux"))]
    transformer: Option<Transformer>,
}

impl BackBuffer {
    pub fn new(
        BackBufferOptions {
            device,
            queue,
            format,
            sub_format,
            size,
            #[cfg(target_os = "windows")]
            direct3d,
        }: BackBufferOptions,
    ) -> Result<Self, BackBufferError> {
        #[cfg(not(target_os = "linux"))]
        let transformer = {
            if sub_format != VideoSubFormat::SW {
                #[cfg(target_os = "windows")]
                {
                    Some(Transformer::new(direct3d, &device, size, format)?)
                }

                #[cfg(target_os = "macos")]
                {
                    Some(Transformer::new(device.clone(), size, format)?)
                }
            } else {
                None
            }
        };

        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mipmap_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let sample = match format {
            VideoFormat::NV12 => Texture2DSourceSample::Nv12(Nv12::new(&device, size, sub_format)),
            VideoFormat::BGRA => Texture2DSourceSample::Bgra(Bgra::new(&device, size, sub_format)),
            VideoFormat::RGBA => Texture2DSourceSample::Rgba(Rgba::new(&device, size, sub_format)),
            VideoFormat::I420 => Texture2DSourceSample::I420(I420::new(&device, size, sub_format)),
        };

        let layout = match &sample {
            Texture2DSourceSample::Bgra(it) => it.bind_group_layout(&device),
            Texture2DSourceSample::Rgba(it) => it.bind_group_layout(&device),
            Texture2DSourceSample::Nv12(it) => it.bind_group_layout(&device),
            Texture2DSourceSample::I420(it) => it.bind_group_layout(&device),
        };

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&layout],
                push_constant_ranges: &[],
            })),
            vertex: VertexState {
                entry_point: Some("main"),
                module: &device.create_shader_module(ShaderModuleDescriptor {
                    label: None,
                    source: ShaderSource::Wgsl(Cow::Borrowed(Vertex::VERTEX_SHADER)),
                }),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[Vertex::desc()],
            },
            fragment: Some(FragmentState {
                entry_point: Some("main"),
                module: &device.create_shader_module(match &sample {
                    Texture2DSourceSample::Rgba(_) => Rgba::fragment_shader(),
                    Texture2DSourceSample::Bgra(_) => Bgra::fragment_shader(),
                    Texture2DSourceSample::Nv12(_) => Nv12::fragment_shader(),
                    Texture2DSourceSample::I420(_) => I420::fragment_shader(),
                }),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                    format: TextureFormat::Bgra8Unorm,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(IndexFormat::Uint16),
                ..Default::default()
            },
            multisample: MultisampleState::default(),
            depth_stencil: None,
            multiview: None,
            cache: None,
        });

        Ok(Self {
            #[cfg(not(target_os = "linux"))]
            transformer,
            device: device,
            queue: queue,
            sample,
            sampler,
            layout,
            pipeline,
        })
    }

    /// If it is a hardware texture, it will directly create view for the
    /// current texture, if it is a software texture, it will write the data to
    /// the internal texture first, and then create the view for the internal
    /// texture, so it is a more time-consuming operation to use the software
    /// texture.
    #[allow(unused_variables)]
    pub fn get_view(
        &mut self,
        encoder: &mut CommandEncoder,
        texture: Texture,
    ) -> Result<(&RenderPipeline, BindGroup), BackBufferError> {
        // Only software textures need to be updated to the sample via update.
        #[allow(unreachable_patterns)]
        match &texture {
            Texture::Bgra(Texture2DResource::Buffer(buffer)) => {
                if let Texture2DSourceSample::Bgra(it) = &self.sample {
                    it.update(&self.queue, buffer);
                }
            }
            Texture::Rgba(Texture2DResource::Buffer(buffer)) => {
                if let Texture2DSourceSample::Rgba(it) = &self.sample {
                    it.update(&self.queue, buffer);
                }
            }
            Texture::Nv12(Texture2DResource::Buffer(buffer)) => {
                if let Texture2DSourceSample::Nv12(it) = &self.sample {
                    it.update(&self.queue, buffer);
                }
            }
            Texture::I420(texture) => {
                if let Texture2DSourceSample::I420(it) = &self.sample {
                    it.update(&self.queue, texture);
                }
            }
            _ => (),
        }

        #[cfg(target_os = "linux")]
        let texture = None;

        #[cfg(not(target_os = "linux"))]
        let texture = match &texture {
            Texture::Rgba(texture) | Texture::Bgra(texture) | Texture::Nv12(texture) => {
                if let Some(transformer) = &mut self.transformer {
                    match texture {
                        #[cfg(not(target_os = "linux"))]
                        Texture2DResource::Texture(texture) => match texture {
                            #[cfg(target_os = "windows")]
                            Texture2DRaw::ID3D11Texture2D(it, index) => {
                                Some(transformer.transform(it, *index)?)
                            }
                            #[cfg(target_os = "macos")]
                            Texture2DRaw::CVPixelBufferRef(it) => {
                                Some(transformer.transform(encoder, *it)?)
                            }
                        },
                        Texture2DResource::Buffer(_) => None,
                        #[allow(unreachable_patterns)]
                        _ => None,
                    }
                } else {
                    None
                }
            }
            Texture::I420(_) => None,
        };

        Ok((
            &self.pipeline,
            match &self.sample {
                Texture2DSourceSample::Bgra(it) => {
                    it.bind_group(&self.device, &self.sampler, &self.layout, texture)
                }
                Texture2DSourceSample::Rgba(it) => {
                    it.bind_group(&self.device, &self.sampler, &self.layout, texture)
                }
                Texture2DSourceSample::Nv12(it) => {
                    it.bind_group(&self.device, &self.sampler, &self.layout, texture)
                }
                Texture2DSourceSample::I420(it) => {
                    it.bind_group(&self.device, &self.sampler, &self.layout, texture)
                }
            },
        ))
    }
}
