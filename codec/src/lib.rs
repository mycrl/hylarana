mod audio;
mod video;

pub use self::{
    audio::{
        AudioDecoder, AudioDecoderError, AudioEncoder, AudioEncoderError,
        create_opus_identification_header,
    },
    video::{
        CodecError, CodecType, VideoDecoder, VideoDecoderError, VideoEncoder, VideoEncoderError,
    },
};

use common::{
    codec::{VideoDecoderType, VideoEncoderType},
    strings::PSTR,
};

use ffmpeg::*;

#[cfg(target_os = "windows")]
use common::win32::Direct3DDevice;

#[derive(Debug, Clone)]
pub struct VideoEncoderSettings {
    /// Name of the codec implementation.
    ///
    /// The name is globally unique among encoders and among decoders (but an
    /// encoder and a decoder can share the same name). This is the primary way
    /// to find a codec from the user perspective.
    pub codec: VideoEncoderType,
    pub frame_rate: u8,
    /// picture width / height
    pub width: u32,
    /// picture width / height
    pub height: u32,
    /// the average bitrate
    pub bit_rate: u64,
    /// the number of pictures in a group of pictures, or 0 for intra_only
    pub key_frame_interval: u32,
    #[cfg(target_os = "windows")]
    pub direct3d: Option<Direct3DDevice>,
}

#[derive(Debug, Clone)]
pub struct VideoDecoderSettings {
    /// Name of the codec implementation.
    ///
    /// The name is globally unique among encoders and among decoders (but
    /// an encoder and a decoder can share the same name). This is
    /// the primary way to find a codec from the user perspective.
    pub codec: VideoDecoderType,
    #[cfg(target_os = "windows")]
    pub direct3d: Option<Direct3DDevice>,
}

#[derive(Debug, Clone, Copy)]
pub struct AudioEncoderSettings {
    pub bit_rate: u64,
    pub sample_rate: u64,
}

mod logger {
    use std::ffi::{c_char, c_int, c_void};

    use common::strings::PSTR;
    use ffmpeg::*;
    use log::Level;

    #[repr(C)]
    #[derive(Debug)]
    #[allow(dead_code)]
    enum LoggerLevel {
        Panic = 0,
        Fatal = 8,
        Error = 16,
        Warn = 24,
        Info = 32,
        Verbose = 40,
        Debug = 48,
        Trace = 56,
    }

    impl Into<Level> for LoggerLevel {
        fn into(self) -> Level {
            match self {
                Self::Panic | Self::Fatal | Self::Error => Level::Error,
                Self::Info | Self::Verbose => Level::Info,
                Self::Warn => Level::Warn,
                Self::Debug => Level::Debug,
                Self::Trace => Level::Trace,
            }
        }
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[allow(non_camel_case_types)]
    type va_list = *mut __va_list_tag;

    #[cfg(all(target_os = "linux", not(target_arch = "x86_64")))]
    #[allow(non_camel_case_types)]
    type va_list = [u64; 4];

    unsafe extern "C" {
        // Write formatted data from variable argument list to sized buffer
        // Composes a string with the same text that would be printed if format was used
        // on printf, but using the elements in the variable argument list identified by
        // arg instead of additional function arguments and storing the resulting
        // content as a C string in the buffer pointed by s (taking n as the maximum
        // buffer capacity to fill).
        //
        // If the resulting string would be longer than n-1 characters, the remaining
        // characters are discarded and not stored, but counted for the value returned
        // by the function.
        //
        // Internally, the function retrieves arguments from the list identified by arg
        // as if va_arg was used on it, and thus the state of arg is likely to be
        // altered by the call.
        //
        // In any case, arg should have been initialized by va_start at some point
        // before the call, and it is expected to be released by va_end at some point
        // after the call.
        #[allow(improper_ctypes)]
        fn vsnprintf(s: *mut c_char, n: usize, format: *const c_char, args: va_list) -> c_int;
    }

    #[allow(improper_ctypes_definitions)]
    pub unsafe extern "C" fn logger_proc(
        _: *mut c_void,
        level: c_int,
        message: *const c_char,
        args: va_list,
    ) {
        let mut chars: [c_char; 1024] = [0; 1024];
        unsafe { vsnprintf(chars.as_mut_ptr(), 2048, message, args) };

        let level: LoggerLevel = unsafe { std::mem::transmute(level) };
        if let Ok(message) = PSTR::from(chars.as_ptr()).to_string() {
            log::log!(
                target: "ffmpeg",
                level.into(),
                "{}",
                message.as_str().strip_suffix("\n").unwrap_or(&message)
            );
        }
    }
}

pub fn startup() {
    unsafe {
        av_log_set_callback(Some(logger::logger_proc));
    }
}

pub fn shutdown() {
    unsafe {
        av_log_set_callback(None);
    }
}

pub(crate) fn set_option(context: &mut AVCodecContext, key: &str, value: i64) {
    unsafe {
        av_opt_set_int(context.priv_data, PSTR::from(key).as_ptr(), value, 0);
    }
}

pub(crate) fn set_str_option(context: &mut AVCodecContext, key: &str, value: &str) {
    unsafe {
        av_opt_set(
            context.priv_data,
            PSTR::from(key).as_ptr(),
            PSTR::from(value).as_ptr(),
            0,
        );
    }
}
