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
    #[error("failed to create HWDevice context")]
    FailedToCreateHWDeviceContext,
    #[error("failed to create HWFrame context")]
    FailedToCreateHWFrameContext,
}

#[derive(Default)]
pub struct ScreenCapture(Arc<AtomicBool>);

impl CaptureHandler for ScreenCapture {
    type Frame = VideoFrame;
    type Error = ScreenCaptureError;
    type CaptureOptions = VideoCaptureSourceDescription;

    // kms capture does not currently support multiple screens.
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
    options: VideoCaptureSourceDescription,
    fmt_ctx: *mut AVFormatContext,
    codec_ctx: *mut AVCodecContext,
    packet: *mut AVPacket,
    frames: [*mut AVFrame; 3],
    filter_graph: *mut AVFilterGraph,
    buffersrc_ctx: *mut AVFilterContext,
    buffersink_ctx: *mut AVFilterContext,
}

unsafe impl Send for Capture {}
unsafe impl Sync for Capture {}

impl Capture {
    fn new(options: &VideoCaptureSourceDescription) -> Result<Self, ScreenCaptureError> {
        let mut this = Self {
            options: options.clone(),
            filter_graph: unsafe { avfilter_graph_alloc() },
            packet: unsafe { av_packet_alloc() },
            buffersink_ctx: null_mut(),
            buffersrc_ctx: null_mut(),
            codec_ctx: null_mut(),
            fmt_ctx: null_mut(),
            frames: [
                unsafe { av_frame_alloc() },
                unsafe { av_frame_alloc() },
                unsafe { av_frame_alloc() },
            ],
        };

        // Currently you can only capture the screen in the desktop environment.
        let format = unsafe { av_find_input_format(PSTR::from("kmsgrab").as_ptr()) };
        if format.is_null() {
            return Err(ScreenCaptureError::NotFoundInputFormat);
        }

        // It's just in BGRA format, which is probably all that's available in the x11
        // desktop environment.
        let mut format_options = null_mut();
        for (k, v) in [
            ("format".to_string(), "bgr0".to_string()),
            ("framerete".to_string(), options.fps.to_string()),
        ] {
            unsafe {
                av_dict_set(
                    &mut format_options,
                    PSTR::from(k).as_ptr(),
                    PSTR::from(v).as_ptr(),
                    0,
                );
            }
        }

        if unsafe {
            avformat_open_input(
                &mut this.fmt_ctx,
                PSTR::from(options.source.id.as_str()).as_ptr(),
                format,
                &mut format_options,
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

        let codec = unsafe { avcodec_find_decoder({ &*stream.codecpar }.codec_id) };
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

        // unsafe {
        //     let codec = &mut *this.codec_ctx;

        //     let mut hw_device_ctx = null_mut();
        //     if av_hwdevice_ctx_create(
        //         &mut hw_device_ctx,
        //         AVHWDeviceType::AV_HWDEVICE_TYPE_VAAPI,
        //         null_mut(),
        //         null_mut(),
        //         0,
        //     ) < 0
        //     {
        //         return Err(ScreenCaptureError::FailedToCreateHWDeviceContext);
        //     }

        //     let hw_frames_ctx = av_hwframe_ctx_alloc(hw_device_ctx);
        //     if hw_frames_ctx.is_null() {
        //         return Err(ScreenCaptureError::FailedToCreateHWFrameContext);
        //     }

        //     let frames_ctx = &mut *((&mut *hw_frames_ctx).data as *mut
        // AVHWFramesContext);     frames_ctx.sw_format =
        // AVPixelFormat::AV_PIX_FMT_BGR0;     frames_ctx.format =
        // AVPixelFormat::AV_PIX_FMT_VAAPI;     frames_ctx.width = codec.width
        // as i32;     frames_ctx.height = codec.height as i32;
        //     frames_ctx.initial_pool_size = 5;

        //     if av_hwframe_ctx_init(hw_frames_ctx) != 0 {
        //         return Err(ScreenCaptureError::FailedToCreateHWFrameContext);
        //     }

        //     codec.hw_device_ctx = av_buffer_ref(hw_device_ctx);
        //     codec.hw_frames_ctx = av_buffer_ref(hw_frames_ctx);
        // }

        if unsafe { avcodec_open2(this.codec_ctx, codec, null_mut()) } != 0 {
            return Err(ScreenCaptureError::NotOpenDecoder);
        }

        // unsafe {
        //     let filter_graph = &*this.filter_graph;
        //     for filter in std::slice::from_raw_parts_mut(
        //         filter_graph.filters,
        //         filter_graph.nb_filters as usize,
        //     ) {
        //         { &mut **filter }.hw_device_ctx = { &*this.codec_ctx }.hw_device_ctx;
        //     }
        // }

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

        if self.buffersink_ctx.is_null() {
            self.init_filters()?;
        }

        if unsafe {
            av_buffersrc_add_frame_flags(
                self.buffersrc_ctx,
                self.frames[0],
                AV_BUFFERSRC_FLAG_KEEP_REF as i32,
            )
        } < 0
        {
            return None;
        }

        if unsafe { av_buffersink_get_frame(self.buffersink_ctx, self.frames[1]) } < 0 {
            return None;
        }

        Some(unsafe { &*self.frames[1] })
    }

    fn init_filters(&mut self) -> Option<()> {
        if unsafe {
            avfilter_graph_create_filter(
                &mut self.buffersrc_ctx,
                avfilter_get_by_name(PSTR::from("buffer").as_ptr()),
                PSTR::from("in").as_ptr(),
                PSTR::from({
                    let codec = &*self.codec_ctx;
                    format!(
                        "video_size={}x{}:pix_fmt={}:time_base=1/{}:pixel_aspect={}/{}",
                        codec.width,
                        codec.height,
                        codec.pix_fmt as i32,
                        self.options.fps,
                        codec.sample_aspect_ratio.num,
                        codec.sample_aspect_ratio.den
                    )
                })
                .as_ptr(),
                null_mut(),
                self.filter_graph,
            )
        } < 0
        {
            return None;
        }

        if unsafe {
            avfilter_graph_create_filter(
                &mut self.buffersink_ctx,
                avfilter_get_by_name(PSTR::from("buffersink").as_ptr()),
                PSTR::from("out").as_ptr(),
                null_mut(),
                null_mut(),
                self.filter_graph,
            )
        } < 0
        {
            return None;
        }

        unsafe {
            let par = av_buffersrc_parameters_alloc();
            { &mut *par }.hw_frames_ctx = { &*self.frames[0] }.hw_frames_ctx;

            if av_buffersrc_parameters_set(self.buffersrc_ctx, par) < 0 {
                return None;
            }
        }

        let mut outputs = unsafe { avfilter_inout_alloc() };
        let mut inputs = unsafe { avfilter_inout_alloc() };

        unsafe {
            let outputs = &mut *outputs;
            outputs.name = av_strdup(PSTR::from("in").as_ptr());
            outputs.filter_ctx = self.buffersrc_ctx;
            outputs.pad_idx = 0;
            outputs.next = null_mut();
        }

        unsafe {
            let inputs = &mut *inputs;
            inputs.name = av_strdup(PSTR::from("out").as_ptr());
            inputs.filter_ctx = self.buffersink_ctx;
            inputs.pad_idx = 0;
            inputs.next = null_mut();
        }

        if unsafe {
            avfilter_graph_parse_ptr(
            self.filter_graph,
            PSTR::from(format!(
                "hwmap=derive_device=vaapi,scale_vaapi=w={}:h={}:format=nv12,hwdownload,format=nv12",
                self.options.size.width, self.options.size.height
            ))
            .as_ptr(),
            &mut inputs,
            &mut outputs,
            null_mut(),
        )
        } < 0
        {
            return None;
        }

        if unsafe { avfilter_graph_config(self.filter_graph, null_mut()) } < 0 {
            return None;
        }

        Some(())
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
