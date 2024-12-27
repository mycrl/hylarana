use crate::{CaptureHandler, FrameArrived, Source, SourceType, VideoCaptureSourceDescription};

use std::{
    ptr::null_mut,
    sync::{atomic::AtomicBool, Arc},
    thread::{self, sleep},
    time::Duration,
};

use hylarana_common::{
    atomic::EasyAtomic,
    frame::{VideoFormat, VideoFrame, VideoSubFormat},
    strings::PSTR,
};

use mirror_ffmpeg_sys::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScreenCaptureError {
    #[error(transparent)]
    CreateThreadError(#[from] std::io::Error),
    #[error("not create hardware device context")]
    CreateHWDeviceContextError,
    #[error("not create hardware frame context")]
    CreateHWFrameContextError,
    #[error("not found input format")]
    NotFoundInputFormat,
    #[error("not open input format")]
    NotOpenInputFormat,
    #[error("not open input stream")]
    NotFoundInputStream,
    #[error("not found decoder")]
    NotFoundDecoder,
    #[error("failed to create decoder")]
    CreateDecoderError,
    #[error("failed to set parameters to decoder")]
    SetParametersError,
    #[error("not open decoder")]
    NotOpenDecoder,
    #[error("failed to create sw scale context")]
    CreateSWScaleContextError,
}

#[derive(Default)]
pub struct ScreenCapture(Arc<AtomicBool>);

impl CaptureHandler for ScreenCapture {
    type Frame = VideoFrame;
    type Error = ScreenCaptureError;
    type CaptureOptions = VideoCaptureSourceDescription;

    // x11 Capture does not currently support multiple screens.
    fn get_sources() -> Result<Vec<Source>, Self::Error> {
        Ok(vec![Source {
            index: 0,
            is_default: true,
            kind: SourceType::Screen,
            id: "/dev/dri/card0".to_string(),
            name: "default display".to_string(),
        }])
    }

    fn start<S: FrameArrived<Frame = Self::Frame> + 'static>(
        &self,
        options: Self::CaptureOptions,
        mut arrived: S,
    ) -> Result<(), Self::Error> {
        let mut capture = Capture::new(&options)?;

        let status = Arc::downgrade(&self.0);
        self.0.update(true);

        thread::Builder::new()
            .name("LinuxScreenCaptureThread".to_string())
            .spawn(move || {
                let mut frame = VideoFrame::default();
                frame.width = options.size.width;
                frame.height = options.size.height;
                frame.sub_format = VideoSubFormat::SW;
                frame.format = VideoFormat::NV12;

                while let Some(avframe) = capture.read() {
                    if let Some(status) = status.upgrade() {
                        if !status.get() {
                            break;
                        }
                    } else {
                        break;
                    }

                    let format = unsafe { std::mem::transmute::<_, AVPixelFormat>(avframe.format) };
                    match format {
                        AVPixelFormat::AV_PIX_FMT_NV12 => {
                            for i in 0..2 {
                                frame.data[i] = avframe.data[i] as _;
                                frame.linesize[i] = avframe.linesize[i] as usize;
                            }

                            if !arrived.sink(&frame) {
                                break;
                            }
                        }
                        _ => unimplemented!("not supports capture pix fmt = {:?}", format),
                    }

                    sleep(Duration::from_millis(1000 / options.fps as u64));
                }
            })?;

        Ok(())
    }

    fn stop(&self) -> Result<(), Self::Error> {
        self.0.update(false);
        Ok(())
    }
}

struct Capture {
    fmt_ctx: *mut AVFormatContext,
    codec_ctx: *mut AVCodecContext,
    packet: *mut AVPacket,
    frames: [*mut AVFrame; 3],
    hw_device_ctx: *mut AVBufferRef,
    filter_graph: *mut AVFilterGraph,
    buffersrc_ctx: *mut AVFilterContext,
    buffersink_ctx: *mut AVFilterContext,
}

unsafe impl Send for Capture {}
unsafe impl Sync for Capture {}

impl Capture {
    fn new(options: &VideoCaptureSourceDescription) -> Result<Self, ScreenCaptureError> {
        let mut this = Self {
            packet: unsafe { av_packet_alloc() },
            codec_ctx: null_mut(),
            fmt_ctx: null_mut(),
            hw_device_ctx: null_mut(),
            filter_graph: unsafe { avfilter_graph_alloc() },
            buffersrc_ctx: null_mut(),
            buffersink_ctx: null_mut(),
            frames: [
                unsafe { av_frame_alloc() },
                unsafe { av_frame_alloc() },
                unsafe { av_frame_alloc() },
            ],
        };

        // if unsafe {
        //     av_hwdevice_ctx_create(
        //         &mut this.hw_device_ctx,
        //         AVHWDeviceType::AV_HWDEVICE_TYPE_VAAPI,
        //         null_mut(),
        //         null_mut(),
        //         0,
        //     )
        // } < 0
        // {
        //     return Err(ScreenCaptureError::NotFoundInputFormat);
        // }

        // Currently you can only capture the screen in the x11 desktop environment.
        let format = unsafe { av_find_input_format(PSTR::from("kmsgrab").as_ptr()) };
        if format.is_null() {
            return Err(ScreenCaptureError::NotFoundInputFormat);
        }

        if unsafe {
            avformat_open_input(
                &mut this.fmt_ctx,
                PSTR::from(options.source.id.as_str()).as_ptr(),
                format,
                null_mut(),
            )
        } != 0
        {
            return Err(ScreenCaptureError::NotOpenInputFormat);
        }

        if unsafe { avformat_find_stream_info(this.fmt_ctx, null_mut()) } != 0 {
            return Err(ScreenCaptureError::NotFoundInputStream);
        }

        let ctx_ref = unsafe { &*this.fmt_ctx };
        if ctx_ref.nb_streams == 0 {
            return Err(ScreenCaptureError::NotFoundInputStream);
        }

        // Desktop capture generally has only one stream.
        let streams = unsafe { std::slice::from_raw_parts(ctx_ref.streams, 1) };
        let stream = unsafe { &*(streams[0]) };
        let codecpar = unsafe { &*stream.codecpar };

        let codec = unsafe { avcodec_find_decoder(codecpar.codec_id) };
        if codec.is_null() {
            return Err(ScreenCaptureError::NotFoundDecoder);
        }

        this.codec_ctx = unsafe { avcodec_alloc_context3(codec) };
        if this.codec_ctx.is_null() {
            return Err(ScreenCaptureError::CreateDecoderError);
        }

        if unsafe { avcodec_parameters_to_context(this.codec_ctx, stream.codecpar) } != 0 {
            return Err(ScreenCaptureError::SetParametersError);
        }

        // let decoder_ctx = unsafe { &mut *this.codec_ctx };
        // decoder_ctx.hw_device_ctx = unsafe { av_buffer_ref(this.hw_device_ctx) };

        if unsafe { avcodec_open2(this.codec_ctx, codec, null_mut()) } != 0 {
            return Err(ScreenCaptureError::NotOpenDecoder);
        }

        if unsafe {
            avfilter_graph_create_filter(
                &mut this.buffersrc_ctx,
                avfilter_get_by_name(PSTR::from("buffer").as_ptr()),
                PSTR::from("in").as_ptr(),
                PSTR::from("video_size=2560x1440:pix_fmt=bgr0:time_base=1/60").as_ptr(),
                null_mut(),
                this.filter_graph,
            )
        } < 0
        {
            return Err(ScreenCaptureError::CreateSWScaleContextError);
        }
        
        if unsafe {
            avfilter_graph_create_filter(
                &mut this.buffersink_ctx,
                avfilter_get_by_name(PSTR::from("buffersink").as_ptr()),
                PSTR::from("out").as_ptr(),
                null_mut(),
                null_mut(),
                this.filter_graph,
            )
        } < 0
        {
            return Err(ScreenCaptureError::CreateSWScaleContextError);
        }

        let mut outputs = unsafe { avfilter_inout_alloc() };
        unsafe {
            let outputs = &mut *outputs;
            outputs.name = av_strdup(PSTR::from("in").as_ptr());
            outputs.filter_ctx = this.buffersrc_ctx;
            outputs.next = null_mut();
            outputs.pad_idx = 0;
        }

        let mut inputs = unsafe { avfilter_inout_alloc() };
        unsafe {
            let inputs = &mut *inputs;
            inputs.name = av_strdup(PSTR::from("out").as_ptr());
            inputs.filter_ctx = this.buffersink_ctx;
            inputs.next = null_mut();
            inputs.pad_idx = 0;
        }

        if unsafe {
            avfilter_graph_parse_ptr(
                this.filter_graph,
                PSTR::from("hwmap=derive_device=vaapi,hwdownload,format=bgr0").as_ptr(),
                &mut inputs,
                &mut outputs,
                null_mut()
            )
        } < 0
        {
            return Err(ScreenCaptureError::CreateSWScaleContextError);
        }

        if unsafe { avfilter_graph_config(this.filter_graph, null_mut()) } < 0 {
            return Err(ScreenCaptureError::CreateSWScaleContextError);
        }

        Ok(this)
    }

    fn read(&mut self) -> Option<&AVFrame> {
        if !self.packet.is_null() {
            unsafe {
                av_packet_unref(self.packet);
            }
        }

        if unsafe { av_read_frame(self.fmt_ctx, self.packet) } != 0 {
            return None;
        }

        if unsafe { avcodec_send_packet(self.codec_ctx, self.packet) } != 0 {
            return None;
        }

        if unsafe { avcodec_receive_frame(self.codec_ctx, self.frames[0]) } != 0 {
            return None;
        }

        println!("============== 444");
        if unsafe { av_buffersrc_add_frame_flags(self.buffersrc_ctx, self.frames[0], 4) } < 0 {
            return None;
        }

        println!("============== 555");
        if unsafe { av_buffersink_get_frame(self.buffersink_ctx, self.frames[1]) } < 0 {
            return None;
        }

        println!("============== 666");
        if unsafe { av_hwframe_transfer_data(self.frames[2], self.frames[1], 0) } < 0 {
            return None;
        }

        println!("============== 777");
        Some(unsafe { &*self.frames[2] })
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        if !self.fmt_ctx.is_null() {
            unsafe {
                avformat_close_input(&mut self.fmt_ctx);
            }
        }

        if !self.codec_ctx.is_null() {
            unsafe {
                avcodec_free_context(&mut self.codec_ctx);
            }
        }

        if !self.packet.is_null() {
            unsafe {
                av_packet_free(&mut self.packet);
            }
        }

        for frame in &mut self.frames {
            if !frame.is_null() {
                unsafe {
                    av_frame_free(frame);
                }
            }
        }
    }
}
