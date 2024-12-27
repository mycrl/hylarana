use crate::{CaptureHandler, FrameArrived, Source, SourceType, VideoCaptureSourceDescription};

use std::{
    ptr::{null, null_mut},
    sync::{atomic::AtomicBool, Arc},
    thread,
};

use hylarana_common::{
    atomic::EasyAtomic,
    frame::{VideoFormat, VideoFrame, VideoSubFormat},
    Size,
};

use mirror_ffmpeg_sys::*;
use thiserror::Error;
use v4l::{
    buffer::Type,
    capability::Flags,
    context::enum_devices,
    io::{mmap::stream::Stream, traits::CaptureStream},
    video::Capture,
    Device, FourCC,
};

#[derive(Error, Debug)]
pub enum CameraCaptureError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("failed to create sw scale context")]
    CreateSWSWScaleContextError,
}

#[derive(Default)]
pub struct CameraCapture(Arc<AtomicBool>);

impl CaptureHandler for CameraCapture {
    type Frame = VideoFrame;
    type Error = CameraCaptureError;
    type CaptureOptions = VideoCaptureSourceDescription;

    fn get_sources() -> Result<Vec<Source>, Self::Error> {
        let mut sources = Vec::with_capacity(5);

        // Multiple handles may exist for the same camera device, filtered out here for
        // `VIDEO_CAPTURE` type devices.
        for item in enum_devices() {
            if let (Some(name), Some(id)) =
                (item.name(), item.path().to_str().map(|s| s.to_string()))
            {
                if let Ok(device) = Device::with_path(&id) {
                    if let Ok(caps) = device.query_caps() {
                        if caps.capabilities.contains(Flags::VIDEO_CAPTURE)
                            || caps.capabilities.contains(Flags::VIDEO_CAPTURE_MPLANE)
                        {
                            sources.push(Source {
                                index: item.index(),
                                kind: SourceType::Camera,
                                is_default: item.index() == 0,
                                name,
                                id,
                            });
                        }
                    }
                }
            }
        }

        Ok(sources)
    }

    fn start<S: FrameArrived<Frame = Self::Frame> + 'static>(
        &self,
        options: Self::CaptureOptions,
        mut arrived: S,
    ) -> Result<(), Self::Error> {
        let status = Arc::downgrade(&self.0);
        self.0.update(true);

        // Fixed to YUYV, there may be compatibility issues here as not all devices may
        // support YUYV.
        let device = Device::with_path(options.source.id)?;
        {
            let mut format = device.format()?;
            format.width = options.size.width;
            format.height = options.size.height;
            format.fourcc = FourCC::new(b"YUYV");
            device.set_format(&format)?;
        }

        let mut swscale = SWScale::new(options.size)?;
        let mut stream = Stream::new(&device, Type::VideoCapture)?;
        thread::Builder::new()
            .name("LinuxCameraCaptureThread".to_string())
            .spawn(move || {
                let mut frame = VideoFrame::default();
                frame.width = options.size.width;
                frame.height = options.size.height;
                frame.sub_format = VideoSubFormat::SW;
                frame.format = VideoFormat::NV12;

                while let Ok((buffer, _)) = stream.next() {
                    if let Some(status) = status.upgrade() {
                        if !status.get() {
                            break;
                        }
                    } else {
                        break;
                    }

                    let scaled = swscale.scale(buffer);
                    for i in 0..2 {
                        frame.data[i] = scaled.data[i] as _;
                        frame.linesize[i] = scaled.linesize[i] as usize;
                    }

                    if !arrived.sink(&frame) {
                        break;
                    }
                }
            })?;

        Ok(())
    }

    fn stop(&self) -> Result<(), Self::Error> {
        self.0.update(false);
        Ok(())
    }
}

struct SWScale {
    sws_ctx: *mut SwsContext,
    frames: [*mut AVFrame; 2],
}

unsafe impl Send for SWScale {}
unsafe impl Sync for SWScale {}

impl SWScale {
    fn new(size: Size) -> Result<Self, CameraCaptureError> {
        let mut this = Self {
            sws_ctx: null_mut(),
            frames: [unsafe { av_frame_alloc() }, unsafe { av_frame_alloc() }],
        };

        unsafe {
            let frame_mut = &mut *this.frames[0];
            frame_mut.format = AVPixelFormat::AV_PIX_FMT_YUYV422 as i32;
            frame_mut.width = size.width as i32;
            frame_mut.height = size.height as i32;
        }

        // The captures are all YUYV, here converted to NV12.
        unsafe {
            let frame_mut = &mut *this.frames[1];
            frame_mut.format = AVPixelFormat::AV_PIX_FMT_NV12 as i32;
            frame_mut.width = size.width as i32;
            frame_mut.height = size.height as i32;

            av_image_alloc(
                frame_mut.data.as_mut_ptr(),
                frame_mut.linesize.as_mut_ptr(),
                frame_mut.width,
                frame_mut.height,
                AVPixelFormat::AV_PIX_FMT_NV12,
                32,
            );
        }

        this.sws_ctx = unsafe {
            sws_getContext(
                size.width as i32,
                size.height as i32,
                AVPixelFormat::AV_PIX_FMT_YUYV422,
                size.width as i32,
                size.height as i32,
                AVPixelFormat::AV_PIX_FMT_NV12,
                SWS_FAST_BILINEAR,
                null_mut(),
                null_mut(),
                null(),
            )
        };

        if this.sws_ctx.is_null() {
            return Err(CameraCaptureError::CreateSWSWScaleContextError);
        }

        Ok(this)
    }

    fn scale(&mut self, buffer: &[u8]) -> &AVFrame {
        unsafe {
            let frame_mut = &mut *self.frames[0];
            frame_mut.linesize[0] = frame_mut.width * 2;
            frame_mut.data[0] = buffer.as_ptr() as *mut _;
        }

        unsafe {
            let frame_mut = &mut *self.frames[0];
            let scaled_frame_mut = &mut *self.frames[1];
            sws_scale(
                self.sws_ctx,
                frame_mut.data.as_ptr() as _,
                frame_mut.linesize.as_ptr(),
                0,
                frame_mut.height,
                scaled_frame_mut.data.as_mut_ptr(),
                scaled_frame_mut.linesize.as_mut_ptr(),
            );
        }

        unsafe { &*self.frames[1] }
    }
}

impl Drop for SWScale {
    fn drop(&mut self) {
        for frame in &mut self.frames {
            if !frame.is_null() {
                unsafe {
                    av_frame_free(frame);
                }
            }
        }

        if !self.sws_ctx.is_null() {
            unsafe {
                sws_freeContext(self.sws_ctx);
            }
        }
    }
}
