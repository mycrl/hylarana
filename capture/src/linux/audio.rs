use crate::{AudioCaptureSourceDescription, CaptureHandler, Source, SourceType};

use std::{
    process::Command,
    ptr::null_mut,
    sync::{atomic::AtomicBool, Arc},
    thread,
};

use hylarana_common::{atomic::EasyAtomic, c_str, frame::AudioFrame};
use mirror_ffmpeg_sys::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioCaptureError {
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
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
    #[error("failed to init swr")]
    SwrInitFailed,
}

pub struct Capture {
    fmt_ctx: *mut AVFormatContext,
    codec_ctx: *mut AVCodecContext,
    packet: *mut AVPacket,
    frame: *mut AVFrame,
    swr_ctx: *mut SwrContext,
    resampled: [[i16; 6400]; 1],
    audio_frame: AudioFrame,
}

unsafe impl Send for Capture {}
unsafe impl Sync for Capture {}

impl Capture {
    pub fn new(options: &AudioCaptureSourceDescription) -> Result<Self, AudioCaptureError> {
        let mut this = Self {
            packet: unsafe { av_packet_alloc() },
            frame: unsafe { av_frame_alloc() },
            fmt_ctx: null_mut(),
            codec_ctx: null_mut(),
            swr_ctx: null_mut(),
            resampled: [[0; 6400]],
            audio_frame: AudioFrame::default(),
        };

        this.audio_frame.sample_rate = options.sample_rate;

        let format = unsafe { av_find_input_format(c_str!("pulse")) };
        if format.is_null() {
            return Err(AudioCaptureError::NotFoundInputFormat);
        }

        if unsafe {
            avformat_open_input(
                &mut this.fmt_ctx,
                c_str!(options.source.id.as_str()),
                format,
                null_mut(),
            )
        } != 0
        {
            return Err(AudioCaptureError::NotOpenInputFormat);
        }

        if unsafe { avformat_find_stream_info(this.fmt_ctx, null_mut()) } != 0 {
            return Err(AudioCaptureError::NotFoundInputStream);
        }

        let ctx_ref = unsafe { &*this.fmt_ctx };
        if ctx_ref.nb_streams == 0 {
            return Err(AudioCaptureError::NotFoundInputStream);
        }

        // Desktop capture generally has only one stream.
        let streams = unsafe { std::slice::from_raw_parts(ctx_ref.streams, 1) };
        let stream = unsafe { &*(streams[0]) };
        let codecpar = unsafe { &*stream.codecpar };

        let codec = unsafe { avcodec_find_decoder(codecpar.codec_id) };
        if codec.is_null() {
            return Err(AudioCaptureError::NotFoundDecoder);
        }

        this.codec_ctx = unsafe { avcodec_alloc_context3(codec) };
        if this.codec_ctx.is_null() {
            return Err(AudioCaptureError::CreateDecoderError);
        }

        if unsafe { avcodec_parameters_to_context(this.codec_ctx, stream.codecpar) } != 0 {
            return Err(AudioCaptureError::SetParametersError);
        }

        if unsafe { avcodec_open2(this.codec_ctx, codec, null_mut()) } != 0 {
            return Err(AudioCaptureError::NotOpenDecoder);
        }

        let ch_layout = AVChannelLayout {
            order: AVChannelOrder::AV_CHANNEL_ORDER_NATIVE,
            nb_channels: 1,
            u: AVChannelLayout__bindgen_ty_1 {
                mask: AV_CH_LAYOUT_MONO,
            },
            opaque: null_mut(),
        };

        let codec_ref = unsafe { &*this.codec_ctx };
        if unsafe {
            swr_alloc_set_opts2(
                &mut this.swr_ctx,
                &ch_layout,
                AVSampleFormat::AV_SAMPLE_FMT_S16,
                options.sample_rate as i32,
                &codec_ref.ch_layout,
                codec_ref.sample_fmt,
                codec_ref.sample_rate,
                0,
                null_mut(),
            )
        } != 0
        {
            return Err(AudioCaptureError::SwrInitFailed);
        }

        if unsafe { swr_init(this.swr_ctx) } != 0 {
            return Err(AudioCaptureError::SwrInitFailed);
        }

        Ok(this)
    }

    fn read(&mut self) -> Option<&AudioFrame> {
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

        if unsafe { avcodec_receive_frame(self.codec_ctx, self.frame) } != 0 {
            return None;
        }

        let frame_ref = unsafe { &*self.frame };
        if unsafe {
            swr_convert(
                self.swr_ctx,
                [self.resampled[0].as_mut_ptr() as _].as_ptr(),
                frame_ref.nb_samples,
                frame_ref.data.as_ptr() as _,
                frame_ref.nb_samples,
            )
        } < 0
        {
            return None;
        }

        self.audio_frame.frames = frame_ref.nb_samples as u32;
        self.audio_frame.data = self.resampled[0].as_ptr();

        Some(&self.audio_frame)
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

        if !self.frame.is_null() {
            unsafe {
                av_frame_free(&mut self.frame);
            }
        }

        if !self.swr_ctx.is_null() {
            unsafe {
                swr_free(&mut self.swr_ctx);
            }
        }
    }
}

#[derive(Default)]
pub struct AudioCapture(Arc<AtomicBool>);

unsafe impl Send for AudioCapture {}
unsafe impl Sync for AudioCapture {}

impl CaptureHandler for AudioCapture {
    type Frame = AudioFrame;
    type Error = AudioCaptureError;
    type CaptureDescriptor = AudioCaptureSourceDescription;

    fn get_sources() -> Result<Vec<Source>, Self::Error> {
        let mut sources = Vec::with_capacity(10);

        for line in String::from_utf8(
            Command::new("pactl")
                .arg("list")
                .arg("sources")
                .arg("short")
                .output()?
                .stdout,
        )?
        .lines()
        {
            if let Some((index, desc)) = line.split_once("\t") {
                if let Some((id, _)) = desc.split_once("\t") {
                    sources.push(Source {
                        name: id.to_string(),
                        id: id.to_string(),
                        index: index.parse()?,
                        kind: SourceType::Audio,
                        is_default: false,
                    });
                }
            }
        }

        Ok(sources)
    }

    fn start<S: crate::FrameArrived<Frame = Self::Frame> + 'static>(
        &self,
        options: Self::CaptureDescriptor,
        mut arrived: S,
    ) -> Result<(), Self::Error> {
        let mut capture = Capture::new(&options)?;

        let status = Arc::downgrade(&self.0);
        self.0.update(true);

        thread::Builder::new()
            .name("LinuxAudioCaptureThread".to_string())
            .spawn(move || {
                while let Some(frame) = capture.read() {
                    if let Some(status) = status.upgrade() {
                        if !status.get() {
                            break;
                        }
                    } else {
                        break;
                    }

                    if !arrived.sink(frame) {
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
