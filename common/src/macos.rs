use std::fmt::Display;

use core_foundation::base::TCFType;
use core_video::{
    metal_texture::CVMetalTexture,
    metal_texture_cache::{CVMetalTextureCache, CVMetalTextureCacheRef},
    pixel_buffer::{
        kCVPixelBufferLock_ReadOnly, kCVPixelFormatType_32BGRA, kCVPixelFormatType_32RGBA,
        kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
        kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange, kCVPixelFormatType_420YpCbCr8Planar,
        CVPixelBufferGetBaseAddressOfPlane, CVPixelBufferGetBytesPerRowOfPlane,
        CVPixelBufferGetHeight, CVPixelBufferGetPixelFormatType, CVPixelBufferGetWidth,
        CVPixelBufferLockBaseAddress, CVPixelBufferUnlockBaseAddress,
    },
};

pub use core_video::{pixel_buffer::CVPixelBufferRef, r#return::CVReturn as ErrorCode};

pub use metal::{
    foreign_types::ForeignTypeRef, Device, MTLPixelFormat, MTLTexture, MTLTextureType, TextureRef,
};

use crate::{frame::VideoFormat, Size};

#[derive(Debug)]
pub struct Error(ErrorCode);

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error code={}", self.0)
    }
}

impl From<ErrorCode> for Error {
    fn from(value: ErrorCode) -> Self {
        Self(value)
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

pub struct PixelMomeryBuffer<'a> {
    pub size: Size,
    pub format: VideoFormat,
    pub data: [&'a [u8]; 3],
    pub linesize: [usize; 3],
    buffer: CVPixelBufferRef,
}

impl<'a> PixelMomeryBuffer<'a> {
    pub fn as_ref(&self) -> CVPixelBufferRef {
        self.buffer
    }
}

impl<'a> From<(CVPixelBufferRef, VideoFormat, Size)> for PixelMomeryBuffer<'a> {
    fn from((buffer, format, size): (CVPixelBufferRef, VideoFormat, Size)) -> Self {
        unsafe {
            CVPixelBufferLockBaseAddress(buffer, kCVPixelBufferLock_ReadOnly);
        }

        let mut this = Self {
            size,
            format,
            buffer,
            data: [&[]; 3],
            linesize: [0; 3],
        };

        for i in 0..3 {
            this.linesize[i] = unsafe { CVPixelBufferGetBytesPerRowOfPlane(buffer, i) };
            this.data[i] = unsafe {
                std::slice::from_raw_parts(
                    CVPixelBufferGetBaseAddressOfPlane(buffer, i) as *const _,
                    this.linesize[i]
                        * if format == VideoFormat::I420 {
                            size.height / 2
                        } else {
                            size.height
                        } as usize,
                )
            };
        }

        this
    }
}

impl<'a> From<CVPixelBufferRef> for PixelMomeryBuffer<'a> {
    fn from(buffer: CVPixelBufferRef) -> Self {
        Self::from((
            buffer,
            get_pixel_buffer_format(buffer),
            get_pixel_buffer_size(buffer),
        ))
    }
}

impl<'a> Drop for PixelMomeryBuffer<'a> {
    fn drop(&mut self) {
        unsafe {
            CVPixelBufferUnlockBaseAddress(self.buffer, kCVPixelBufferLock_ReadOnly);
        }
    }
}

#[derive(Clone, Copy)]
pub struct PixelBuffer {
    buffer: CVPixelBufferRef,
    pub format: VideoFormat,
    pub size: Size,
}

impl PixelBuffer {
    pub fn as_ref(&self) -> CVPixelBufferRef {
        self.buffer
    }
}

impl From<CVPixelBufferRef> for PixelBuffer {
    fn from(buffer: CVPixelBufferRef) -> Self {
        Self::from((
            buffer,
            get_pixel_buffer_format(buffer),
            get_pixel_buffer_size(buffer),
        ))
    }
}

impl From<(CVPixelBufferRef, VideoFormat, Size)> for PixelBuffer {
    fn from((buffer, format, size): (CVPixelBufferRef, VideoFormat, Size)) -> Self {
        Self {
            buffer,
            format,
            size,
        }
    }
}

pub struct MetalTextureCache(CVMetalTextureCache);

impl MetalTextureCache {
    pub fn new(device: Device) -> Result<Self, Error> {
        Ok(Self(CVMetalTextureCache::new(None, device, None)?))
    }

    pub fn update(&self, buffer: PixelBuffer) -> Result<CVMetalTexture, Error> {
        Ok(self.0.create_texture_from_image(
            buffer.as_ref(),
            None,
            match buffer.format {
                VideoFormat::BGRA => MTLPixelFormat::BGRA8Unorm,
                VideoFormat::RGBA => MTLPixelFormat::RGBA8Unorm,
                _ => unimplemented!("unsupports format = {:?}", buffer.format),
            },
            buffer.size.width as usize,
            buffer.size.height as usize,
            0,
        )?)
    }

    pub fn flush(&self) {
        self.0.flush(0);
    }

    pub fn as_ref(&self) -> CVMetalTextureCacheRef {
        self.0.as_concrete_TypeRef()
    }
}
