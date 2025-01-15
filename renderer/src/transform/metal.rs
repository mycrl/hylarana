use std::sync::Arc;

use common::{
    frame::VideoFormat,
    macos::{
        create_cv_metal_texture, create_metal_texture_cache, get_pixel_buffer_format,
        get_pixel_buffer_size, get_texture_from_cv_texture, texture_cache_release,
        texture_ref_release, CVMetalTextureCacheRef, CVPixelBufferRef, ForeignTypeRef, MTLTexture,
        MTLTextureType, TextureRef,
    },
};

use wgpu::{
    hal::{api::Metal, Api, CopyExtent},
    Device, Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

use super::TransformError;

pub struct Transformer {
    cache: CVMetalTextureCacheRef,
    device: Arc<Device>,
    texture: Option<Texture>,
    texture_ref: Option<*mut MTLTexture>,
}

unsafe impl Send for Transformer {}
unsafe impl Sync for Transformer {}

impl Transformer {
    pub fn new(device: Arc<Device>) -> Result<Self, TransformError> {
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
        let cache = create_metal_texture_cache(raw_device)
            .ok_or_else(|| TransformError::CreateCVTextureCacheError)?;

        Ok(Self {
            device,
            cache,
            texture: None,
            texture_ref: None,
        })
    }

    pub fn transform(&mut self, texture: CVPixelBufferRef) -> Result<&Texture, TransformError> {
        let size = get_pixel_buffer_size(texture);
        let video_format = get_pixel_buffer_format(texture);

        // Converts a pixel buffer to a metal texture.
        let texture = get_texture_from_cv_texture(
            create_cv_metal_texture(texture, video_format, size, self.cache)
                .ok_or_else(|| TransformError::CreateCVMetalTextureError)?,
        )
        .ok_or_else(|| TransformError::CreateCVMetalTextureError)?;

        // Every texture needs to be cleaned up or it will cause a memory leak.
        if let Some(texture_ref) = self.texture_ref.replace(texture) {
            texture_ref_release(texture_ref);
        }

        // Only BGRA and RGBA textures are supported.
        let format = match video_format {
            VideoFormat::BGRA => TextureFormat::Bgra8Unorm,
            VideoFormat::RGBA => TextureFormat::Rgba8Unorm,
            _ => unimplemented!("unsupports format = {:?}", video_format),
        };

        // Converted from metal texture to wgpu usable texture via hal layer.
        self.texture.replace(unsafe {
            self.device.create_texture_from_hal::<Metal>(
                <Metal as Api>::Device::texture_from_raw(
                    TextureRef::from_ptr(texture).to_owned(),
                    format,
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
                    size: Extent3d {
                        width: size.width,
                        height: size.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    usage: TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                    format,
                },
            )
        });

        Ok(self.texture.as_ref().unwrap())
    }
}

impl Drop for Transformer {
    fn drop(&mut self) {
        texture_cache_release(self.cache);
    }
}
