use crate::{VideoDecoderSettings, VideoEncoderSettings, set_option, set_str_option};

use std::{ffi::c_int, ptr::null_mut};

use common::{
    codec::{VideoDecoderType, VideoEncoderType},
    frame::{VideoFormat, VideoFrame, VideoSubFormat},
    strings::PSTR,
};

use ffmpeg::*;
use thiserror::Error;

#[cfg(any(target_os = "windows", target_os = "macos"))]
use common::Size;

#[cfg(target_os = "windows")]
use common::win32::{Direct3DDevice, windows::core::Interface};

#[cfg(target_os = "macos")]
use common::macos::get_pixel_buffer_format;

#[derive(Error, Debug)]
pub enum VideoDecoderError {
    #[error(transparent)]
    CodecError(#[from] CodecError),
    #[error(transparent)]
    CreateVideoContextError(#[from] CreateVideoContextError),
    #[error(transparent)]
    CreateVideoFrameError(#[from] CreateVideoFrameError),
    #[error("failed to open av codec")]
    OpenAVCodecError,
    #[error("failed to init av parser context")]
    InitAVCodecParserContextError,
    #[error("failed to alloc av packet")]
    AllocAVPacketError,
    #[error("parser parse packet failed")]
    ParsePacketError,
    #[error("send av packet to codec failed")]
    SendPacketToAVCodecError,
    #[error("failed to alloc av frame")]
    AllocAVFrameError,
}

pub struct VideoDecoder {
    context: *mut AVCodecContext,
    parser: *mut AVCodecParserContext,
    packet: *mut AVPacket,
    av_frame: *mut AVFrame,
    frame: VideoFrame,
}

unsafe impl Sync for VideoDecoder {}
unsafe impl Send for VideoDecoder {}

impl VideoDecoder {
    pub fn new(options: VideoDecoderSettings) -> Result<Self, VideoDecoderError> {
        if !CodecType::from(options.codec).is_supported() {
            return Err(VideoDecoderError::CodecError(CodecError::NotSupportCodec));
        }

        let mut this = Self {
            context: null_mut(),
            parser: null_mut(),
            packet: null_mut(),
            av_frame: null_mut(),
            frame: VideoFrame::default(),
        };

        #[cfg(target_os = "windows")]
        let codec = create_video_context(
            &mut this.context,
            CodecType::from(options.codec),
            None,
            options.direct3d,
        )?;

        #[cfg(target_os = "linux")]
        let codec = create_video_context(&mut this.context, CodecType::from(options.codec))?;

        #[cfg(target_os = "macos")]
        let codec = create_video_context(&mut this.context, CodecType::from(options.codec), None)?;

        let context_mut = unsafe { &mut *this.context };
        context_mut.delay = 0;
        context_mut.max_samples = 1;
        context_mut.has_b_frames = 0;
        context_mut.skip_alpha = true as i32;
        context_mut.flags |= AV_CODEC_FLAG_LOW_DELAY as i32;
        context_mut.flags2 |= AV_CODEC_FLAG2_FAST as i32;
        context_mut.hwaccel_flags |= AV_HWACCEL_FLAG_IGNORE_LEVEL as i32;

        #[cfg(target_os = "windows")]
        {
            context_mut.hwaccel_flags |= AV_HWACCEL_FLAG_UNSAFE_OUTPUT as i32;
        }

        if options.codec == VideoDecoderType::Qsv {
            set_option(context_mut, "async_depth", 1);
        }

        if unsafe { avcodec_open2(this.context, codec, null_mut()) } != 0 {
            return Err(VideoDecoderError::OpenAVCodecError);
        }

        if unsafe { avcodec_is_open(this.context) } == 0 {
            return Err(VideoDecoderError::OpenAVCodecError);
        }

        this.parser = unsafe { av_parser_init({ &*codec }.id as i32) };
        if this.parser.is_null() {
            return Err(VideoDecoderError::InitAVCodecParserContextError);
        }

        this.packet = unsafe { av_packet_alloc() };
        if this.packet.is_null() {
            return Err(VideoDecoderError::AllocAVPacketError);
        }

        Ok(this)
    }

    pub fn decode(&mut self, mut buf: &[u8], pts: u64) -> Result<(), VideoDecoderError> {
        if buf.is_empty() {
            return Ok(());
        }

        let mut size = buf.len();
        while size > 0 {
            let packet = unsafe { &mut *self.packet };
            let len = unsafe {
                av_parser_parse2(
                    self.parser,
                    self.context,
                    &mut packet.data,
                    &mut packet.size,
                    buf.as_ptr(),
                    buf.len() as c_int,
                    pts as i64,
                    pts as i64,
                    0,
                )
            };

            // When parsing the code stream, an abnormal return code appears and processing
            // should not be continued.
            if len < 0 {
                return Err(VideoDecoderError::ParsePacketError);
            }

            let len = len as usize;
            buf = &buf[len..];
            size -= len;

            // One or more cells have been parsed.
            if packet.size > 0 {
                if unsafe { avcodec_send_packet(self.context, self.packet) } != 0 {
                    return Err(VideoDecoderError::SendPacketToAVCodecError);
                }
            }
        }

        Ok(())
    }

    pub fn read<'a>(&'a mut self) -> Option<&'a VideoFrame> {
        // When decoding, each video frame uses a newly created one.
        if !self.av_frame.is_null() {
            unsafe {
                av_frame_free(&mut self.av_frame);
            }
        }

        self.av_frame = unsafe { av_frame_alloc() };
        if self.av_frame.is_null() {
            return None;
        }

        if unsafe { avcodec_receive_frame(self.context, self.av_frame) } != 0 {
            return None;
        }

        let frame = unsafe { &*self.av_frame };
        self.frame.width = frame.width as u32;
        self.frame.height = frame.height as u32;

        let format = unsafe { std::mem::transmute::<_, AVPixelFormat>(frame.format) };
        match format {
            // mfxFrameSurface1.Data.MemId contains a pointer to the mfxHDLPair structure
            // when importing the following frames as QSV frames:
            //
            // VAAPI: mfxHDLPair.first contains a VASurfaceID pointer. mfxHDLPair.second is
            // always MFX_INFINITE.
            //
            // DXVA2: mfxHDLPair.first contains IDirect3DSurface9 pointer. mfxHDLPair.second
            // is always MFX_INFINITE.
            //
            // D3D11: mfxHDLPair.first contains a ID3D11Texture2D pointer. mfxHDLPair.second
            // contains the texture array index of the frame if the ID3D11Texture2D is an
            // array texture, or always MFX_INFINITE if it is a normal texture.
            #[cfg(target_os = "windows")]
            AVPixelFormat::AV_PIX_FMT_QSV => {
                let surface = unsafe { &*(frame.data[3] as *const mfxFrameSurface1) };
                let hdl = unsafe { &*(surface.Data.MemId as *const mfxHDLPair) };

                self.frame.data[0] = hdl.first;
                self.frame.data[1] = hdl.second;

                self.frame.sub_format = VideoSubFormat::D3D11;
                self.frame.format = VideoFormat::NV12;
            }
            // The d3d11va video frame texture has no stride.
            #[cfg(target_os = "windows")]
            AVPixelFormat::AV_PIX_FMT_D3D11 => {
                for i in 0..2 {
                    self.frame.data[i] = frame.data[i] as *const _;
                }

                self.frame.sub_format = VideoSubFormat::D3D11;
                self.frame.format = VideoFormat::NV12;
            }
            AVPixelFormat::AV_PIX_FMT_YUV420P => {
                for i in 0..3 {
                    self.frame.data[i] = frame.data[i] as *const _;
                    self.frame.linesize[i] = frame.linesize[i] as u32;
                }

                self.frame.sub_format = VideoSubFormat::SW;
                self.frame.format = VideoFormat::I420;
            }
            #[cfg(target_os = "macos")]
            AVPixelFormat::AV_PIX_FMT_VIDEOTOOLBOX => {
                self.frame.data[0] = frame.data[3] as _;

                self.frame.sub_format = VideoSubFormat::CvPixelBufferRef;
                self.frame.format = get_pixel_buffer_format(frame.data[3] as _);
            }
            _ => unimplemented!("unsupported video frame format = {:?}", format),
        };

        Some(&self.frame)
    }
}

impl Drop for VideoDecoder {
    fn drop(&mut self) {
        if !self.packet.is_null() {
            unsafe {
                av_packet_free(&mut self.packet);
            }
        }

        if !self.parser.is_null() {
            unsafe {
                av_parser_close(self.parser);
            }
        }

        if !self.context.is_null() {
            let ctx_mut = unsafe { &mut *self.context };
            if !ctx_mut.hw_device_ctx.is_null() {
                unsafe {
                    av_buffer_unref(&mut ctx_mut.hw_device_ctx);
                }
            }

            if !ctx_mut.hw_frames_ctx.is_null() {
                unsafe {
                    av_buffer_unref(&mut ctx_mut.hw_frames_ctx);
                }
            }

            unsafe {
                avcodec_free_context(&mut self.context);
            }
        }

        if !self.av_frame.is_null() {
            unsafe {
                av_frame_free(&mut self.av_frame);
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum VideoEncoderError {
    #[error(transparent)]
    CodecError(#[from] CodecError),
    #[error(transparent)]
    CreateVideoContextError(#[from] CreateVideoContextError),
    #[error(transparent)]
    CreateVideoFrameError(#[from] CreateVideoFrameError),
    #[error("failed to open av codec")]
    OpenAVCodecError,
    #[error("failed to alloc av packet")]
    AllocAVPacketError,
    #[error("send frame to codec failed")]
    EncodeFrameError,
}

pub struct VideoEncoder {
    context: *mut AVCodecContext,
    packet: *mut AVPacket,
    frame: *mut AVFrame,
    initialized: bool,
}

unsafe impl Sync for VideoEncoder {}
unsafe impl Send for VideoEncoder {}

impl VideoEncoder {
    pub fn new(options: VideoEncoderSettings) -> Result<Self, VideoEncoderError> {
        if !CodecType::from(options.codec).is_supported() {
            return Err(VideoEncoderError::CodecError(CodecError::NotSupportCodec));
        }

        let mut this = Self {
            context: null_mut(),
            packet: null_mut(),
            frame: null_mut(),
            initialized: false,
        };

        #[cfg(target_os = "windows")]
        let codec = create_video_context(
            &mut this.context,
            CodecType::from(options.codec),
            Some(Size {
                width: options.width,
                height: options.height,
            }),
            options.direct3d,
        )?;

        #[cfg(target_os = "linux")]
        let codec = create_video_context(&mut this.context, CodecType::from(options.codec))?;

        #[cfg(target_os = "macos")]
        let codec = create_video_context(
            &mut this.context,
            CodecType::from(options.codec),
            Some(Size {
                width: options.width,
                height: options.height,
            }),
        )?;

        let context_mut = unsafe { &mut *this.context };
        context_mut.delay = 0;
        context_mut.max_samples = 1;
        context_mut.has_b_frames = 0;
        context_mut.max_b_frames = 0;
        context_mut.color_primaries = AVColorPrimaries::AVCOL_PRI_BT709;
        context_mut.color_trc = AVColorTransferCharacteristic::AVCOL_TRC_BT709;
        context_mut.colorspace = AVColorSpace::AVCOL_SPC_BT709;
        context_mut.flags2 |= AV_CODEC_FLAG2_FAST as i32;
        context_mut.flags |= AV_CODEC_FLAG_LOW_DELAY as i32 | AV_CODEC_FLAG_GLOBAL_HEADER as i32;
        context_mut.profile = FF_PROFILE_HEVC_MAIN as i32;

        // The QSV encoder can only use qsv frames. Although the internal structure is a
        // platform-specific hardware texture, you cannot directly tell qsv a specific
        // format.
        if options.codec == VideoEncoderType::Qsv {
            context_mut.pix_fmt = AVPixelFormat::AV_PIX_FMT_QSV;
        } else {
            context_mut.thread_count = 4;
            context_mut.thread_type = FF_THREAD_SLICE as i32;
            context_mut.pix_fmt = if options.codec == VideoEncoderType::VideoToolBox {
                AVPixelFormat::AV_PIX_FMT_VIDEOTOOLBOX
            } else {
                AVPixelFormat::AV_PIX_FMT_NV12
            };
        }

        // The bitrate of qsv is always too high, so if it is qsv, using half of the
        // current base bitrate is enough.
        let mut bit_rate = options.bit_rate as i64;
        if options.codec == VideoEncoderType::Qsv {
            bit_rate = bit_rate / 2;
        }

        context_mut.bit_rate = bit_rate;
        context_mut.rc_max_rate = bit_rate;
        context_mut.rc_buffer_size = bit_rate as i32;
        context_mut.bit_rate_tolerance = (bit_rate / 10) as i32;
        context_mut.rc_initial_buffer_occupancy = (bit_rate * 3 / 4) as i32;
        context_mut.framerate = unsafe { av_make_q(options.frame_rate as i32, 1) };
        context_mut.time_base = unsafe { av_make_q(1, options.frame_rate as i32) };
        context_mut.pkt_timebase = unsafe { av_make_q(1, options.frame_rate as i32) };
        context_mut.gop_size = options.frame_rate as i32 / 2;
        context_mut.height = options.height as i32;
        context_mut.width = options.width as i32;

        match options.codec {
            VideoEncoderType::X265 => {
                set_str_option(context_mut, "preset", "superfast");
                set_str_option(context_mut, "tune", "zerolatency");
                set_option(
                    context_mut,
                    "sc_threshold",
                    options.key_frame_interval as i64,
                );
            }
            VideoEncoderType::Qsv => {
                set_option(context_mut, "async_depth", 1);
                set_option(context_mut, "low_power", 1);
                set_option(context_mut, "vcm", 1);
            }
            VideoEncoderType::VideoToolBox => {
                set_option(context_mut, "realtime", 1);
                set_option(context_mut, "coder", 1); // vlc
            }
        };

        if unsafe { avcodec_open2(this.context, codec, null_mut()) } != 0 {
            return Err(VideoEncoderError::OpenAVCodecError);
        }

        if unsafe { avcodec_is_open(this.context) } == 0 {
            return Err(VideoEncoderError::OpenAVCodecError);
        }

        this.packet = unsafe { av_packet_alloc() };
        if this.packet.is_null() {
            return Err(VideoEncoderError::AllocAVPacketError);
        }

        // When encoding a video, frames can be reused. Here, a frame is created and
        // then reused by replacing the data inside the frame.
        create_video_frame(&mut this.frame, this.context)?;

        Ok(this)
    }

    pub fn update(&mut self, frame: &VideoFrame) -> bool {
        let av_frame = unsafe { &mut *self.frame };
        match frame.sub_format {
            // mfxFrameSurface1.Data.MemId contains a pointer to the mfxHDLPair structure
            // when importing the following frames as QSV frames:
            //
            // VAAPI: mfxHDLPair.first contains a VASurfaceID pointer. mfxHDLPair.second is
            // always MFX_INFINITE.
            //
            // DXVA2: mfxHDLPair.first contains IDirect3DSurface9 pointer. mfxHDLPair.second
            // is always MFX_INFINITE.
            //
            // D3D11: mfxHDLPair.first contains a ID3D11Texture2D pointer. mfxHDLPair.second
            // contains the texture array index of the frame if the ID3D11Texture2D is an
            // array texture, or always MFX_INFINITE if it is a normal texture.
            #[cfg(target_os = "windows")]
            VideoSubFormat::D3D11 => {
                if av_frame.format == AVPixelFormat::AV_PIX_FMT_QSV as i32 {
                    let surface = unsafe { &mut *(av_frame.data[3] as *mut mfxFrameSurface1) };
                    let hdl = unsafe { &mut *(surface.Data.MemId as *mut mfxHDLPair) };

                    hdl.first = frame.data[0] as *mut _;
                    hdl.second = frame.data[1] as *mut _;
                }
            }
            #[cfg(target_os = "macos")]
            VideoSubFormat::CvPixelBufferRef => {
                av_frame.data[3] = frame.data[0] as _;
            }
            VideoSubFormat::SW => {
                // Anyway, the hardware encoder has no way to check whether the current frame is
                // writable.
                if unsafe { av_frame_make_writable(self.frame) } != 0 {
                    return false;
                }

                // Directly replacing the pointer may cause some problems with pointer access.
                // Copying data to the frame is the safest way.
                unsafe {
                    av_image_copy(
                        av_frame.data.as_mut_ptr(),
                        av_frame.linesize.as_mut_ptr(),
                        frame.data.as_ptr() as _,
                        [
                            frame.linesize[0] as i32,
                            frame.linesize[1] as i32,
                            frame.linesize[2] as i32,
                        ]
                        .as_ptr(),
                        { &*self.context }.pix_fmt,
                        av_frame.width,
                        av_frame.height,
                    );
                }
            }
            #[allow(unreachable_patterns)]
            _ => unimplemented!("unsupported video frame format"),
        }

        true
    }

    pub fn encode(&mut self) -> Result<(), VideoEncoderError> {
        let av_frame = unsafe { &mut *self.frame };
        av_frame.pts = unsafe {
            let context_ref = &*self.context;
            av_rescale_q(
                context_ref.frame_num,
                context_ref.pkt_timebase,
                context_ref.time_base,
            )
        };

        if unsafe { avcodec_send_frame(self.context, self.frame) } != 0 {
            return Err(VideoEncoderError::EncodeFrameError);
        }

        Ok(())
    }

    pub fn read<'a>(&'a mut self) -> Option<(&'a [u8], i32, u64)> {
        let packet_ref = unsafe { &*self.packet };
        let context_ref = unsafe { &*self.context };

        // In the global header mode, ffmpeg will not automatically add sps and pps to
        // the bitstream. You need to manually extract the sps and pps data and add it
        // to the header as configuration information.
        if !self.initialized {
            self.initialized = true;

            return Some((
                unsafe {
                    std::slice::from_raw_parts(
                        context_ref.extradata,
                        context_ref.extradata_size as usize,
                    )
                },
                2,
                packet_ref.pts as u64,
            ));
        }

        if unsafe { avcodec_receive_packet(self.context, self.packet) } != 0 {
            return None;
        }

        Some((
            unsafe { std::slice::from_raw_parts(packet_ref.data, packet_ref.size as usize) },
            packet_ref.flags,
            packet_ref.pts as u64,
        ))
    }

    pub fn frame_count(&mut self) -> u64 {
        unsafe { &*self.context }.frame_num as u64
    }

    pub fn bit_rate(&mut self) -> u64 {
        unsafe { &*self.context }.bit_rate as u64
    }

    pub fn set_bit_rate(&mut self, bit_rate: u64) {
        let context_mut = unsafe { &mut *self.context };

        context_mut.bit_rate = bit_rate as i64;
        context_mut.rc_max_rate = bit_rate as i64;
        context_mut.rc_buffer_size = bit_rate as i32;
        context_mut.rc_initial_buffer_occupancy = (bit_rate * 3 / 4) as i32;
    }
}

impl Drop for VideoEncoder {
    fn drop(&mut self) {
        if !self.packet.is_null() {
            unsafe {
                av_packet_free(&mut self.packet);
            }
        }

        if !self.context.is_null() {
            let ctx_mut = unsafe { &mut *self.context };
            if !ctx_mut.hw_device_ctx.is_null() {
                unsafe {
                    av_buffer_unref(&mut ctx_mut.hw_device_ctx);
                }
            }

            if !ctx_mut.hw_frames_ctx.is_null() {
                unsafe {
                    av_buffer_unref(&mut ctx_mut.hw_frames_ctx);
                }
            }

            unsafe {
                avcodec_free_context(&mut self.context);
            }
        }

        if !self.frame.is_null() {
            unsafe {
                av_frame_free(&mut self.frame);
            }
        }
    }
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
    SetMultithreadProtectedError(#[from] common::win32::windows::core::Error),
    #[error("failed to init av hardware device context")]
    InitAVHardwareDeviceContextError,
    #[error("failed to init qsv device context")]
    InitQsvDeviceContextError,
    #[error("failed to alloc av hardware frame context")]
    AllocAVHardwareFrameContextError,
    #[error("failed to init av hardware frame context")]
    InitAVHardwareFrameContextError,
}

#[derive(Debug, Error, Clone, Copy)]
pub enum CodecError {
    #[error("unsupported codecs")]
    NotSupportCodec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecType {
    Encoder(VideoEncoderType),
    Decoder(VideoDecoderType),
}

impl From<VideoEncoderType> for CodecType {
    fn from(value: VideoEncoderType) -> Self {
        Self::Encoder(value)
    }
}

impl From<VideoDecoderType> for CodecType {
    fn from(value: VideoDecoderType) -> Self {
        Self::Decoder(value)
    }
}

impl CodecType {
    pub fn is_supported(&self) -> bool {
        match self {
            CodecType::Encoder(kind) => {
                if cfg!(target_os = "windows") {
                    *kind != VideoEncoderType::VideoToolBox
                } else if cfg!(target_os = "linux") {
                    *kind == VideoEncoderType::X265
                } else {
                    *kind == VideoEncoderType::X265 || *kind == VideoEncoderType::VideoToolBox
                }
            }
            CodecType::Decoder(kind) => {
                if cfg!(target_os = "windows") {
                    *kind != VideoDecoderType::VideoToolBox
                } else if cfg!(target_os = "linux") {
                    *kind == VideoDecoderType::HEVC
                } else {
                    *kind == VideoDecoderType::HEVC || *kind == VideoDecoderType::VideoToolBox
                }
            }
        }
    }

    pub const fn is_encoder(&self) -> bool {
        if let Self::Encoder(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_qsv(self) -> bool {
        match self {
            CodecType::Encoder(kind) => kind == VideoEncoderType::Qsv,
            CodecType::Decoder(kind) => kind == VideoDecoderType::Qsv,
        }
    }

    pub fn is_hardware(&self) -> bool {
        match self {
            Self::Decoder(codec) => *codec != VideoDecoderType::HEVC,
            Self::Encoder(codec) => *codec != VideoEncoderType::X265,
        }
    }

    pub unsafe fn find_av_codec(&self) -> *const AVCodec {
        match self {
            Self::Encoder(kind) => unsafe {
                avcodec_find_encoder_by_name(PSTR::from(kind.to_string()).as_ptr())
            },
            Self::Decoder(kind) => {
                if *kind == VideoDecoderType::D3D11 || *kind == VideoDecoderType::VideoToolBox {
                    unsafe { avcodec_find_decoder(AVCodecID::AV_CODEC_ID_HEVC) }
                } else {
                    unsafe { avcodec_find_decoder_by_name(PSTR::from(kind.to_string()).as_ptr()) }
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn create_video_context(
    context: &mut *mut AVCodecContext,
    kind: CodecType,
    size: Option<Size>,
    direct3d: Option<Direct3DDevice>,
) -> Result<*const AVCodec, CreateVideoContextError> {
    // It is not possible to directly find the d3d11va decoder, so special
    // processing is required here. For d3d11va, the hardware context is initialized
    // below.
    let codec = unsafe { kind.find_av_codec() };
    if codec.is_null() {
        return Err(CreateVideoContextError::NotFoundAVCodec);
    }

    *context = unsafe { avcodec_alloc_context3(codec) };
    if context.is_null() {
        return Err(CreateVideoContextError::AllocAVContextError);
    }

    // The hardware codec is used, and the hardware context is initialized here for
    // the hardware codec.
    if kind.is_hardware() {
        let hw_device_ctx =
            unsafe { av_hwdevice_ctx_alloc(AVHWDeviceType::AV_HWDEVICE_TYPE_D3D11VA) };
        if hw_device_ctx.is_null() {
            return Err(CreateVideoContextError::AllocAVHardwareDeviceContextError);
        }

        // Use externally created d3d devices and do not let ffmpeg create d3d devices
        // itself.
        let direct3d = if let Some(direct3d) = direct3d {
            direct3d
        } else {
            return Err(CreateVideoContextError::MissingDirect3DDevice);
        };

        // Special handling is required for qsv, which requires multithreading to be
        // enabled for the d3d device.
        if kind.is_qsv() {
            if let Err(e) = direct3d.set_multithread_protected(true) {
                return Err(CreateVideoContextError::SetMultithreadProtectedError(e));
            }
        }

        let d3d11_hwctx = unsafe {
            let hwctx = (&mut *hw_device_ctx).data as *mut AVHWDeviceContext;
            &mut *((&mut *hwctx).hwctx as *mut AVD3D11VADeviceContext)
        };

        d3d11_hwctx.device = direct3d.device.as_raw() as *mut _;
        d3d11_hwctx.device_context = direct3d.context.as_raw() as *mut _;

        if unsafe { av_hwdevice_ctx_init(hw_device_ctx) } != 0 {
            return Err(CreateVideoContextError::InitAVHardwareDeviceContextError);
        }

        // Creating a qsv device is a little different, the qsv hardware context needs
        // to be derived from the platform's native hardware context.
        let context_mut = unsafe { &mut **context };
        if kind.is_qsv() {
            let mut qsv_device_ctx = std::ptr::null_mut();
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
            if kind.is_encoder() {
                let hw_frames_ctx = unsafe { av_hwframe_ctx_alloc(context_mut.hw_device_ctx) };
                if hw_frames_ctx.is_null() {
                    return Err(CreateVideoContextError::AllocAVHardwareFrameContextError);
                }

                let size = size.expect("encoder needs init hardware frame for size");
                unsafe {
                    let frames_ctx = &mut *((&mut *hw_frames_ctx).data as *mut AVHWFramesContext);
                    frames_ctx.sw_format = AVPixelFormat::AV_PIX_FMT_NV12;
                    frames_ctx.format = AVPixelFormat::AV_PIX_FMT_QSV;
                    frames_ctx.width = size.width as i32;
                    frames_ctx.height = size.height as i32;
                    frames_ctx.initial_pool_size = 5;
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

#[cfg(target_os = "linux")]
pub fn create_video_context(
    context: &mut *mut AVCodecContext,
    kind: CodecType,
) -> Result<*const AVCodec, CreateVideoContextError> {
    let codec = unsafe { kind.find_av_codec() };
    if codec.is_null() {
        return Err(CreateVideoContextError::NotFoundAVCodec);
    }

    *context = unsafe { avcodec_alloc_context3(codec) };
    if context.is_null() {
        return Err(CreateVideoContextError::AllocAVContextError);
    }

    Ok(codec)
}

#[cfg(target_os = "macos")]
pub fn create_video_context(
    context: &mut *mut AVCodecContext,
    kind: CodecType,
    size: Option<Size>,
) -> Result<*const AVCodec, CreateVideoContextError> {
    let codec = unsafe { kind.find_av_codec() };
    if codec.is_null() {
        return Err(CreateVideoContextError::NotFoundAVCodec);
    }

    *context = unsafe { avcodec_alloc_context3(codec) };
    if context.is_null() {
        return Err(CreateVideoContextError::AllocAVContextError);
    }

    if kind.is_hardware() {
        let mut hw_device_ctx = std::ptr::null_mut();
        if unsafe {
            av_hwdevice_ctx_create(
                &mut hw_device_ctx,
                AVHWDeviceType::AV_HWDEVICE_TYPE_VIDEOTOOLBOX,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            )
        } != 0
        {
            return Err(CreateVideoContextError::InitAVHardwareDeviceContextError);
        }

        let context_mut = unsafe { &mut **context };
        context_mut.hw_device_ctx = unsafe { av_buffer_ref(hw_device_ctx) };

        if kind.is_encoder() {
            let hw_frames_ctx = unsafe { av_hwframe_ctx_alloc(context_mut.hw_device_ctx) };
            if hw_frames_ctx.is_null() {
                return Err(CreateVideoContextError::AllocAVHardwareFrameContextError);
            }

            let size = size.expect("encoder needs init hardware frame for size");
            unsafe {
                let frames_ctx = &mut *((&mut *hw_frames_ctx).data as *mut AVHWFramesContext);
                frames_ctx.sw_format = AVPixelFormat::AV_PIX_FMT_NV12;
                frames_ctx.format = AVPixelFormat::AV_PIX_FMT_VIDEOTOOLBOX;
                frames_ctx.width = size.width as i32;
                frames_ctx.height = size.height as i32;
                frames_ctx.initial_pool_size = 5;
            }

            if unsafe { av_hwframe_ctx_init(hw_frames_ctx) } != 0 {
                return Err(CreateVideoContextError::InitAVHardwareFrameContextError);
            }

            unsafe {
                context_mut.hw_frames_ctx = av_buffer_ref(hw_frames_ctx);
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
    if !context_ref.hw_device_ctx.is_null() {
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
