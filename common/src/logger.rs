use std::fs::{create_dir, metadata};

use fern::{DateBased, Dispatch};
use log::LevelFilter;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoggerInitError {
    #[error(transparent)]
    LogError(#[from] log::SetLoggerError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

pub fn init_logger(level: LevelFilter, path: Option<&str>) -> Result<(), LoggerInitError> {
    let mut logger = Dispatch::new()
        .level(level)
        .level_for("wgpu", LevelFilter::Warn)
        .level_for("wgpu_core", LevelFilter::Warn)
        .level_for("wgpu_hal", LevelFilter::Warn)
        .level_for("wgpu_hal::auxil::dxgi::exception", LevelFilter::Error)
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}] - ({}) - {}",
                record.level(),
                record.file_static().unwrap_or("*"),
                message
            ))
        })
        .chain(std::io::stdout());

    if let Some(path) = path {
        if metadata(path).is_err() {
            create_dir(path)?;
        }

        logger = logger.chain(DateBased::new(path, "%Y-%m-%d-hylarana.log"))
    }

    logger.apply()?;
    Ok(())
}

pub fn enable_panic_logger() {
    std::panic::set_hook(Box::new(|info| {
        log::error!(
            "pnaic: location={:?}, message={:?}",
            info.location(),
            info.payload()
                .downcast_ref::<&str>()
                .map(|it| Some(it.to_string()))
                .unwrap_or_else(|| info.payload().downcast_ref::<String>().cloned())
        );
    }));
}

pub mod android {
    #[cfg(target_os = "android")]
    use std::ffi::{c_char, c_int};

    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AndroidLogLevel {
        Verbose = 2,
        Debug,
        Info,
        Warn,
        Error,
    }

    impl AndroidLogLevel {
        pub fn from_level(level: log::Level) -> Self {
            match level {
                log::Level::Trace => Self::Verbose,
                log::Level::Debug => Self::Debug,
                log::Level::Info => Self::Info,
                log::Level::Warn => Self::Warn,
                log::Level::Error => Self::Error,
            }
        }
    }

    unsafe extern "C" {
        #[cfg(target_os = "android")]
        #[link_name = "__android_log_write"]
        fn android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
    }

    #[cfg(target_os = "android")]
    pub struct AndroidLogger {
        package: String,
    }

    #[cfg(target_os = "android")]
    impl log::Log for AndroidLogger {
        fn flush(&self) {}
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }

        fn log(&self, record: &log::Record) {
            unsafe {
                android_log_write(
                    AndroidLogLevel::from_level(record.level()) as c_int,
                    format!("{}\0", self.package).as_ptr() as *const _,
                    format!(
                        "({}) - {}\0",
                        record.file_static().unwrap_or("*"),
                        record.args()
                    )
                    .as_ptr() as *const _,
                );
            }
        }
    }

    #[allow(unused_variables)]
    pub fn init_logger(package: &str, level: log::LevelFilter) {
        #[cfg(target_os = "android")]
        {
            log::set_max_level(level);
            log::set_boxed_logger(Box::new(AndroidLogger {
                package: package.to_string(),
            }))
            .unwrap();
        }
    }
}
