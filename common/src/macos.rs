use std::{
    fmt::Display,
    ptr::{NonNull, null_mut},
    sync::{Arc, Mutex},
};

use block2::RcBlock;
use objc2_av_foundation::{
    AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio, AVMediaTypeVideo,
};

use objc2_core_foundation::kCFAllocatorDefault;
use objc2_core_media::{
    CMAudioFormatDescription, CMAudioFormatDescriptionGetStreamBasicDescription,
};

use objc2_core_video::{
    CVMetalTexture, CVMetalTextureCache, CVMetalTextureCacheCreate,
    CVMetalTextureCacheCreateTextureFromImage, CVMetalTextureCacheFlush, CVMetalTextureGetTexture,
    CVPixelBuffer, CVPixelBufferGetBaseAddressOfPlane, CVPixelBufferGetBytesPerRowOfPlane,
    CVPixelBufferGetHeight, CVPixelBufferGetPixelFormatType, CVPixelBufferGetWidth,
    CVPixelBufferLockBaseAddress, CVPixelBufferLockFlags, CVPixelBufferUnlockBaseAddress,
    kCVPixelFormatType_32BGRA, kCVPixelFormatType_32RGBA,
    kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
    kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange, kCVPixelFormatType_420YpCbCr8Planar,
    kCVReturnSuccess,
};

use objc2::{
    rc::Retained,
    runtime::{Bool, ProtocolObject},
};

use objc2_metal::{MTLDevice as Objc2MTLDevice, MTLPixelFormat as Objc2MTLPixelFormat};

use crate::{Size, frame::VideoFormat};

pub use metal::{
    Device, MTLPixelFormat, MTLTexture, MTLTextureType, Texture, TextureRef,
    foreign_types::ForeignType,
};

pub use objc2_core_audio_types::AudioStreamBasicDescription;

pub type CVPixelBufferRef = *mut CVPixelBuffer;

#[derive(Debug)]
pub struct Error(i32);

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error code={}", self.0)
    }
}

impl From<i32> for Error {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

#[allow(non_upper_case_globals)]
pub fn get_pixel_buffer_format(buffer: CVPixelBufferRef) -> VideoFormat {
    match unsafe { CVPixelBufferGetPixelFormatType(&*buffer) } {
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
        width: unsafe { CVPixelBufferGetWidth(&*buffer) } as u32,
        height: unsafe { CVPixelBufferGetHeight(&*buffer) } as u32,
    }
}

pub fn get_format_description_info<'a>(
    descr: *const CMAudioFormatDescription,
) -> Option<&'a AudioStreamBasicDescription> {
    let ptr = unsafe { CMAudioFormatDescriptionGetStreamBasicDescription(&*descr) };

    if ptr.is_null() {
        None
    } else {
        Some(unsafe { &*ptr })
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
            CVPixelBufferLockBaseAddress(&*buffer, CVPixelBufferLockFlags::ReadOnly);
        }

        let mut this = Self {
            size,
            format,
            buffer,
            data: [&[]; 3],
            linesize: [0; 3],
        };

        for i in 0..3 {
            this.linesize[i] = unsafe { CVPixelBufferGetBytesPerRowOfPlane(&*buffer, i) };

            if this.linesize[i] > 0 {
                this.data[i] = unsafe {
                    std::slice::from_raw_parts(
                        CVPixelBufferGetBaseAddressOfPlane(&*buffer, i) as *const _,
                        this.linesize[i]
                            * if format == VideoFormat::I420 {
                                size.height / 2
                            } else {
                                size.height
                            } as usize,
                    )
                };
            }
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
            CVPixelBufferUnlockBaseAddress(&*self.buffer, CVPixelBufferLockFlags::ReadOnly);
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
    pub fn as_ref(&self) -> &CVPixelBuffer {
        unsafe { &*self.buffer }
    }

    pub fn as_raw(&self) -> CVPixelBufferRef {
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

pub struct MetalTextureCache(Retained<CVMetalTextureCache>);

impl MetalTextureCache {
    pub fn new(device: Device) -> Result<Self, Error> {
        let device: Retained<ProtocolObject<dyn Objc2MTLDevice>> =
            unsafe { Retained::from_raw(device.into_ptr().cast()).unwrap() };

        let mut cache = null_mut();
        let code = unsafe {
            CVMetalTextureCacheCreate(
                kCFAllocatorDefault,
                None,
                device.as_ref(),
                None,
                NonNull::new(&mut cache).unwrap(),
            )
        };

        if code != kCVReturnSuccess || cache.is_null() {
            return Err(Error(code));
        }

        Ok(Self(unsafe { Retained::from_raw(cache).unwrap() }))
    }

    pub fn map(&self, buffer: PixelBuffer) -> Result<MetalTexture, Error> {
        let Size { width, height } = buffer.size;

        let mut texture = null_mut();
        let code = unsafe {
            CVMetalTextureCacheCreateTextureFromImage(
                kCFAllocatorDefault,
                &self.0,
                buffer.as_ref(),
                None,
                match buffer.format {
                    VideoFormat::BGRA => Objc2MTLPixelFormat::BGRA8Unorm,
                    VideoFormat::RGBA => Objc2MTLPixelFormat::RGBA8Unorm,
                    _ => unimplemented!("unsupports format = {:?}", buffer.format),
                },
                width as usize,
                height as usize,
                0,
                NonNull::new(&mut texture).unwrap(),
            )
        };

        if code != kCVReturnSuccess || texture.is_null() {
            return Err(Error(code));
        }

        Ok(MetalTexture(unsafe {
            Retained::from_raw(texture).unwrap()
        }))
    }

    pub fn flush(&self) {
        unsafe {
            CVMetalTextureCacheFlush(&self.0, 0);
        }
    }
}

pub struct MetalTexture(Retained<CVMetalTexture>);

impl MetalTexture {
    pub fn get_texture(&mut self) -> Result<Texture, Error> {
        if let Some(texture) = unsafe { CVMetalTextureGetTexture(&self.0) } {
            Ok(unsafe { Texture::from_ptr(Retained::into_raw(texture).cast()).to_owned() })
        } else {
            Err(Error(-1))
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AVMediaType {
    Video,
    Audio,
}

pub fn request_av_capture_permissions(ty: AVMediaType) {
    let ty = unsafe {
        match ty {
            AVMediaType::Video => AVMediaTypeVideo.unwrap(),
            AVMediaType::Audio => AVMediaTypeAudio.unwrap(),
        }
    };

    if unsafe { AVCaptureDevice::authorizationStatusForMediaType(&ty) }
        != AVAuthorizationStatus::Authorized
    {
        let callback = Arc::new(Mutex::<Option<RcBlock<dyn Fn(Bool)>>>::new(None));

        unsafe {
            let callback_ = callback.clone();
            let fun = RcBlock::new(move |ret: Bool| {
                if ret.is_false() {
                    log::error!("failed to request permissions, type={:?}", ty);
                }

                let _ = callback_.lock().unwrap().take();
            });

            AVCaptureDevice::requestAccessForMediaType_completionHandler(
                &ty,
                &*RcBlock::as_ptr(&fun),
            );

            callback.lock().unwrap().replace(fun);
        }
    }
}
