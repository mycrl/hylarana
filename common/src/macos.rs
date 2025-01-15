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
        kCVPixelBufferLock_ReadOnly, kCVPixelFormatType_32BGRA, kCVPixelFormatType_32RGBA,
        kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
        kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange, kCVPixelFormatType_420YpCbCr8Planar,
        CVPixelBufferGetBaseAddressOfPlane, CVPixelBufferGetBytesPerRowOfPlane,
        CVPixelBufferGetHeight, CVPixelBufferGetPixelFormatType, CVPixelBufferGetWidth,
        CVPixelBufferLockBaseAddress, CVPixelBufferUnlockBaseAddress,
    },
    r#return::kCVReturnSuccess,
};

pub use metal::{
    foreign_types::ForeignTypeRef, Device, MTLPixelFormat, MTLTexture, MTLTextureType, TextureRef,
};

use crate::{frame::VideoFormat, Size};

pub struct PixelBufferRef {
    size: Size,
    data: [*const u8; 2],
    linesize: [usize; 2],
    buffer: CVPixelBufferRef,
}

impl PixelBufferRef {
    pub fn size(&self) -> Size {
        self.size
    }

    pub fn data(&self) -> &[*const u8; 2] {
        &self.data
    }

    pub fn linesize(&self) -> &[usize; 2] {
        &self.linesize
    }
}

impl From<CVPixelBufferRef> for PixelBufferRef {
    fn from(buffer: CVPixelBufferRef) -> Self {
        unsafe {
            CVPixelBufferLockBaseAddress(buffer, kCVPixelBufferLock_ReadOnly);
        }

        let mut this = Self {
            size: get_pixel_buffer_size(buffer),
            buffer,
            data: [null(); 2],
            linesize: [0; 2],
        };

        for i in 0..2 {
            this.data[i] = unsafe { CVPixelBufferGetBaseAddressOfPlane(buffer, i) as *const _ };
            this.linesize[i] = unsafe { CVPixelBufferGetBytesPerRowOfPlane(buffer, i) };
        }

        this
    }
}

impl Drop for PixelBufferRef {
    fn drop(&mut self) {
        unsafe {
            CVPixelBufferUnlockBaseAddress(self.buffer, kCVPixelBufferLock_ReadOnly);
        }
    }
}

#[allow(non_upper_case_globals)]
pub fn get_pixel_buffer_format(buffer: CVPixelBufferRef) -> VideoFormat {
    match unsafe { CVPixelBufferGetPixelFormatType(buffer) } {
        kCVPixelFormatType_32RGBA => VideoFormat::RGBA,
        kCVPixelFormatType_32BGRA => VideoFormat::BGRA,
        kCVPixelFormatType_420YpCbCr8Planar => VideoFormat::I420,
        kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange
        | kCVPixelFormatType_420YpCbCr8BiPlanarFullRange => VideoFormat::NV12,
        format => unimplemented!("unsupports format = {:?}", format),
    }
}

pub fn get_pixel_buffer_size(buffer: CVPixelBufferRef) -> Size {
    Size {
        width: unsafe { CVPixelBufferGetWidth(buffer) } as u32,
        height: unsafe { CVPixelBufferGetHeight(buffer) } as u32,
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
                _ => unimplemented!("unsupports format = {:?}", format),
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
