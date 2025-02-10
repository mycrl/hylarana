use std::sync::Arc;

use common::{
    frame::VideoFormat,
    macos::{CVPixelBufferRef, MTLTextureType, MetalTextureCache, PixelBuffer},
    Size,
};

use wgpu::{
    hal::{api::Metal, Api, CopyExtent},
    CommandEncoder, Device, Extent3d, Origin3d, TexelCopyTextureInfo, Texture, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

use super::TransformError;

pub struct Transformer {
    cache: MetalTextureCache,
    device: Arc<Device>,
    texture: Texture,
}

unsafe impl Send for Transformer {}
unsafe impl Sync for Transformer {}

impl Transformer {
    pub fn new(
        device: Arc<Device>,
        size: Size,
        format: VideoFormat,
    ) -> Result<Self, TransformError> {
        // Get the wgpu underlying metal device.
        let mut raw_device = None;
        unsafe {
            device.as_hal::<Metal, _, _>(|device| {
                if let Some(device) = device {
                    raw_device = Some(device.raw_device().lock().clone());
                }
            });
        }

        // Creates a metal texture buffer for converting pixel buffers to metal
        // textures.
        let raw_device = raw_device.ok_or_else(|| TransformError::NotFoundMetalBackend)?;
        let cache = MetalTextureCache::new(raw_device)?;

        let texture = device.create_texture(&TextureDescriptor {
            label: None,
            format: match format {
                VideoFormat::BGRA => TextureFormat::Bgra8Unorm,
                VideoFormat::RGBA => TextureFormat::Rgba8Unorm,
                _ => unimplemented!("unsupports format = {:?}", format),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
            size: Extent3d {
                depth_or_array_layers: 1,
                width: size.width,
                height: size.height,
            },
        });

        Ok(Self {
            device,
            cache,
            texture,
        })
    }

    pub fn transform(
        &mut self,
        encoder: &mut CommandEncoder,
        buffer: CVPixelBufferRef,
    ) -> Result<&Texture, TransformError> {
        let size = self.texture.size();

        encoder.copy_texture_to_texture(
            TexelCopyTextureInfo {
                texture: &unsafe {
                    self.device.create_texture_from_hal::<Metal>(
                        <Metal as Api>::Device::texture_from_raw(
                            self.cache.map(PixelBuffer::from(buffer))?.get_texture()?,
                            self.texture.format(),
                            MTLTextureType::D2,
                            1,
                            1,
                            CopyExtent {
                                width: size.width,
                                height: size.height,
                                depth: 1,
                            },
                        ),
                        &TextureDescriptor {
                            label: None,
                            size,
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: TextureDimension::D2,
                            format: self.texture.format(),
                            usage: TextureUsages::COPY_SRC,
                            view_formats: &[],
                        },
                    )
                },
                mip_level: 0,
                origin: Origin3d::default(),
                aspect: TextureAspect::All,
            },
            TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: Origin3d::default(),
                aspect: TextureAspect::All,
            },
            self.texture.size(),
        );

        self.cache.flush();
        Ok(&self.texture)
    }
}
