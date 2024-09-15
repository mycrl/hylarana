use std::{ffi::c_void, ptr::null_mut};

use crate::video::{VideoDecoderType, VideoEncoderType};

use ffmpeg_sys_next::*;
use utils::strings::Strings;

#[cfg(target_os = "windows")]
use utils::win32::{Direct3DDevice, Interface};

use thiserror::Error;

#[derive(Clone, Copy)]
pub enum CodecType {
    Encoder(VideoEncoderType),
    Decoder(VideoDecoderType),
}

impl CodecType {
    fn is_qsv(self) -> bool {
        match self {
            CodecType::Encoder(kind) => kind == VideoEncoderType::Qsv,
            CodecType::Decoder(kind) => kind == VideoDecoderType::Qsv,
        }
    }

    fn is_d3d(self) -> bool {
        match self {
            CodecType::Decoder(kind) => kind == VideoDecoderType::D3D11,
            _ => false,
        }
    }

    fn is_encoder(&self) -> bool {
        if let Self::Encoder(_) = self {
            true
        } else {
            false
        }
    }
}

#[derive(Default)]
pub struct HardwareFrameSize {
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[cfg(target_os = "windows")]
struct AVD3D11VADeviceContext {
    device: *const c_void,
    device_context: *const c_void,
    video_device: *const c_void,
    video_context: *const c_void,
    lock: extern "C" fn(lock_ctx: *const c_void),
    unlock: extern "C" fn(lock_ctx: *const c_void),
    lock_ctx: *const c_void,
}

#[derive(Error, Debug)]
pub enum CreateVideoContextError {
    #[error("not found av codec")]
    NotFoundAVCodec,
    #[error("failed to alloc av context")]
    AllocAVContextError,
    #[error("failed to alloc av hardware device context")]
    AllocAVHardwareDeviceContextError,
    #[error("missing direct3d device")]
    MissingDirect3DDevice,
    #[cfg(target_os = "windows")]
    #[error(transparent)]
    SetMultithreadProtectedError(#[from] windows_core::Error),
    #[error("failed to init av hardware device context")]
    InitAVHardwareDeviceContextError,
    #[error("failed to init qsv device context")]
    InitQsvDeviceContextError,
    #[error("failed to alloc av hardware frame context")]
    AllocAVHardwareFrameContextError,
    #[error("failed to init av hardware frame context")]
    InitAVHardwareFrameContextError,
}

pub struct CreateVideoContextDescriptor<'a> {
    pub context: &'a mut *mut AVCodecContext,
    pub kind: CodecType,
    #[cfg(target_os = "windows")]
    pub frame_size: Option<HardwareFrameSize>,
    #[cfg(target_os = "windows")]
    pub direct3d: Option<Direct3DDevice>,
}

pub fn create_video_context(
    options: CreateVideoContextDescriptor,
) -> Result<*const AVCodec, CreateVideoContextError> {
    // It is not possible to directly find the d3d11va decoder, so special
    // processing is required here. For d3d11va, the hardware context is initialized
    // below.
    let codec = match options.kind {
        CodecType::Encoder(kind) => {
            let codec: &str = kind.into();
            unsafe { avcodec_find_encoder_by_name(Strings::from(codec).as_ptr()) }
        }
        CodecType::Decoder(kind) => {
            if kind == VideoDecoderType::D3D11 {
                unsafe { avcodec_find_decoder(AVCodecID::AV_CODEC_ID_H264) }
            } else {
                let codec: &str = kind.into();
                unsafe { avcodec_find_decoder_by_name(Strings::from(codec).as_ptr()) }
            }
        }
    };

    if codec.is_null() {
        return Err(CreateVideoContextError::NotFoundAVCodec);
    }

    *options.context = unsafe { avcodec_alloc_context3(codec) };
    if options.context.is_null() {
        return Err(CreateVideoContextError::AllocAVContextError);
    }

    // The hardware codec is used, and the hardware context is initialized here for
    // the hardware codec.
    #[cfg(target_os = "windows")]
    if options.kind.is_d3d() || options.kind.is_qsv() {
        let hw_device_ctx =
            unsafe { av_hwdevice_ctx_alloc(AVHWDeviceType::AV_HWDEVICE_TYPE_D3D11VA) };
        if hw_device_ctx.is_null() {
            return Err(CreateVideoContextError::AllocAVHardwareDeviceContextError);
        }

        let direct3d = if let Some(direct3d) = options.direct3d {
            direct3d
        } else {
            return Err(CreateVideoContextError::MissingDirect3DDevice);
        };

        // Special handling is required for qsv, which requires multithreading to be
        // enabled for the d3d device.
        if options.kind.is_qsv() {
            if let Err(e) = direct3d.set_multithread_protected(true) {
                return Err(CreateVideoContextError::SetMultithreadProtectedError(e));
            }
        }

        // Use externally created d3d devices and do not let ffmpeg create d3d devices
        // itself.
        let d3d11_hwctx = unsafe {
            let hwctx = (&mut *hw_device_ctx).data as *mut AVHWDeviceContext;
            &mut *((&mut *hwctx).hwctx as *mut AVD3D11VADeviceContext)
        };

        d3d11_hwctx.device = direct3d.device.as_raw();
        d3d11_hwctx.device_context = direct3d.context.as_raw();

        if unsafe { av_hwdevice_ctx_init(hw_device_ctx) } != 0 {
            return Err(CreateVideoContextError::InitAVHardwareDeviceContextError);
        }

        // Creating a qsv device is a little different, the qsv hardware context needs
        // to be derived from the platform's native hardware context.
        let context_mut = unsafe { &mut **options.context };
        if options.kind.is_qsv() {
            let mut qsv_device_ctx = null_mut();
            if unsafe {
                av_hwdevice_ctx_create_derived(
                    &mut qsv_device_ctx,
                    AVHWDeviceType::AV_HWDEVICE_TYPE_QSV,
                    hw_device_ctx,
                    0,
                )
            } != 0
            {
                return Err(CreateVideoContextError::InitQsvDeviceContextError);
            }

            unsafe {
                context_mut.hw_device_ctx = av_buffer_ref(qsv_device_ctx);
            }

            // Similarly, the qsv hardware frame also needs to be created and initialized
            // independently.
            if options.kind.is_encoder() {
                let hw_frames_ctx = unsafe { av_hwframe_ctx_alloc(context_mut.hw_device_ctx) };
                if hw_frames_ctx.is_null() {
                    return Err(CreateVideoContextError::AllocAVHardwareFrameContextError);
                }

                if let Some(size) = options.frame_size {
                    unsafe {
                        let frames_ctx =
                            &mut *((&mut *hw_frames_ctx).data as *mut AVHWFramesContext);
                        frames_ctx.sw_format = AVPixelFormat::AV_PIX_FMT_NV12;
                        frames_ctx.format = AVPixelFormat::AV_PIX_FMT_QSV;
                        frames_ctx.width = size.width as i32;
                        frames_ctx.height = size.height as i32;
                        frames_ctx.initial_pool_size = 20;
                    }
                }

                if unsafe { av_hwframe_ctx_init(hw_frames_ctx) } != 0 {
                    return Err(CreateVideoContextError::InitAVHardwareFrameContextError);
                }

                unsafe {
                    context_mut.hw_frames_ctx = av_buffer_ref(hw_frames_ctx);
                }
            }
        } else {
            unsafe {
                context_mut.hw_device_ctx = av_buffer_ref(hw_device_ctx);
            }
        }
    }

    Ok(codec)
}

#[derive(Error, Debug)]
pub enum CreateVideoFrameError {
    #[error("failed to alloc av frame")]
    AllocAVFrameError,
    #[error("failed to alloc hardware av frame buffer")]
    AllocHardwareAVFrameBufferError,
    #[error("failed to alloc av frame buffer")]
    AllocAVFrameBufferError,
}

pub fn create_video_frame(
    frame: &mut *mut AVFrame,
    context: *const AVCodecContext,
    kind: CodecType,
) -> Result<(), CreateVideoFrameError> {
    *frame = unsafe { av_frame_alloc() };
    if frame.is_null() {
        return Err(CreateVideoFrameError::AllocAVFrameError);
    }

    let context_ref = unsafe { &*context };
    let frame_mut = unsafe { &mut **frame };

    frame_mut.width = context_ref.width;
    frame_mut.height = context_ref.height;
    frame_mut.format = context_ref.pix_fmt as i32;

    // qsv needs to indicate the use of hardware textures, otherwise qsv will return
    // software textures.
    if kind.is_qsv() {
        if unsafe { av_hwframe_get_buffer(context_ref.hw_frames_ctx, *frame, 0) } != 0 {
            return Err(CreateVideoFrameError::AllocHardwareAVFrameBufferError);
        }
    } else {
        if unsafe { av_frame_get_buffer(*frame, 0) } != 0 {
            return Err(CreateVideoFrameError::AllocAVFrameBufferError);
        }
    }

    Ok(())
}

pub fn set_option(context: &mut AVCodecContext, key: &str, value: i64) {
    unsafe {
        av_opt_set_int(context.priv_data, Strings::from(key).as_ptr(), value, 0);
    }
}

pub fn set_str_option(context: &mut AVCodecContext, key: &str, value: &str) {
    unsafe {
        av_opt_set(
            context.priv_data,
            Strings::from(key).as_ptr(),
            Strings::from(value).as_ptr(),
            0,
        );
    }
}