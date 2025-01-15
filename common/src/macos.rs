use std::ptr::{null, null_mut};

pub use core_video::{
    metal_texture::CVMetalTextureRef, metal_texture_cache::CVMetalTextureCacheRef,
    pixel_buffer::CVPixelBufferRef,
};

use core_foundation_sys::base::{kCFAllocatorDefault, CFRelease};
use core_video::{
    metal_texture::CVMetalTextureGetTexture,
    metal_texture_cache::{
        CVMetalTextureCacheCreate, CVMetalTextureCacheCreateTextureFromImage,
        CVMetalTextureCacheFlush,
    },
    pixel_buffer::{
        kCVPixelFormatType_32BGRA, kCVPixelFormatType_32RGBA,
        kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
        kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange, kCVPixelFormatType_420YpCbCr8Planar,
        CVPixelBufferGetHeight, CVPixelBufferGetPixelFormatType, CVPixelBufferGetWidth,
    },
    r#return::kCVReturnSuccess,
};

pub use metal::{
    foreign_types::ForeignTypeRef, Device, MTLPixelFormat, MTLTexture, MTLTextureType, TextureRef,
};

use crate::{frame::VideoFormat, Size};

pub trait EasyTexture {
    fn size(&self) -> Size;
    fn format(&self) -> VideoFormat;
}

impl EasyTexture for CVPixelBufferRef {
    fn size(&self) -> Size {
        unsafe {
            Size {
                width: CVPixelBufferGetWidth(self.clone()) as u32,
                height: CVPixelBufferGetHeight(self.clone()) as u32,
            }
        }
    }

    #[allow(non_upper_case_globals)]
    fn format(&self) -> VideoFormat {
        match unsafe { CVPixelBufferGetPixelFormatType(self.clone()) } {
            kCVPixelFormatType_32RGBA => VideoFormat::RGBA,
            kCVPixelFormatType_32BGRA => VideoFormat::BGRA,
            kCVPixelFormatType_420YpCbCr8Planar => VideoFormat::I420,
            kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange
            | kCVPixelFormatType_420YpCbCr8BiPlanarFullRange => VideoFormat::NV12,
            _ => unimplemented!(),
        }
    }
}

pub fn create_metal_texture_cache(device: Device) -> Option<CVMetalTextureCacheRef> {
    let mut texture_cache = null_mut();
    if unsafe {
        CVMetalTextureCacheCreate(
            kCFAllocatorDefault,
            null(),
            device,
            null(),
            &mut texture_cache,
        )
    } != kCVReturnSuccess
    {
        None
    } else {
        Some(texture_cache)
    }
}

pub fn create_cv_metal_texture(
    buffer: CVPixelBufferRef,
    format: VideoFormat,
    size: Size,
    texture_cache: CVMetalTextureCacheRef,
) -> Option<CVMetalTextureRef> {
    let mut texture = null_mut();
    if unsafe {
        CVMetalTextureCacheCreateTextureFromImage(
            kCFAllocatorDefault,
            texture_cache,
            buffer,
            null(),
            match format {
                VideoFormat::BGRA => MTLPixelFormat::BGRA8Unorm,
                VideoFormat::RGBA => MTLPixelFormat::RGBA8Unorm,
                _ => unimplemented!(),
            },
            size.width as usize,
            size.height as usize,
            0,
            &mut texture,
        )
    } != kCVReturnSuccess
    {
        None
    } else {
        Some(texture)
    }
}

pub fn get_texture_from_cv_texture(texture: CVMetalTextureRef) -> Option<*mut MTLTexture> {
    let texture = unsafe { CVMetalTextureGetTexture(texture) };
    if texture.is_null() {
        None
    } else {
        Some(texture)
    }
}

pub fn texture_ref_release(texture: *mut MTLTexture) {
    unsafe {
        CFRelease(texture as _);
    }
}

pub fn texture_cache_release(texture_cache: CVMetalTextureCacheRef) {
    unsafe {
        CVMetalTextureCacheFlush(texture_cache, 0);
    }
}
