use std::{
    env,
    env::current_exe,
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    about = env!("CARGO_PKG_DESCRIPTION"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
)]
pub struct AppConfig {
    #[arg(long, env = "HYLARANA_CACHE_PATH", default_value_t = Self::default_cache_path())]
    pub cache_path: String,
    #[arg(long, env = "HYLARANA_URI", default_value_t = Self::default_uri())]
    pub uri: String,
    #[arg(long, env = "HYLARANA_CHEME_PATH", default_value_t = Self::default_cheme_path())]
    pub cheme_path: String,
    #[arg(long, env = "HYLARANA_SUBPROCESS_PATH", default_value_t = Self::default_subprocess_path())]
    pub subprocess_path: String,
    #[arg(long, env = "HYLARANA_USERNAME", default_value_t = Self::default_username())]
    pub username: String,
}

impl AppConfig {
    pub fn default_cache_path() -> String {
        if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
            let path = dirs::data_local_dir()
                .expect("The current user's local data directory could not be found.")
                .join("Hylarana");

            fs::create_dir_all(&path).unwrap();
            normalize_path(path).unwrap()
        } else {
            unimplemented!()
        }
    }

    pub fn default_uri() -> String {
        "webview://localhost/index.html".to_string()
    }

    pub fn default_cheme_path() -> String {
        if cfg!(target_os = "macos") {
            join_with_current_dir("../Resources/webview").unwrap()
        } else {
            first_existing_path(["webview", "../app/webview"]).unwrap()
        }
    }

    pub fn default_subprocess_path() -> String {
        if cfg!(target_os = "macos") {
            join_with_current_dir(
                "../Frameworks/Hylarana Helper.app/Contents/MacOS/Hylarana Helper",
            )
            .unwrap()
        } else if cfg!(target_os = "windows") {
            join_with_current_dir("hylarana-app-helper.exe").unwrap()
        } else {
            unimplemented!()
        }
    }

    pub fn default_username() -> String {
        ["USERNAME", "USER", "LOGNAME"]
            .into_iter()
            .find_map(|key| {
                env::var(key)
                    .ok()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
            })
            .or_else(|| {
                dirs::home_dir()
                    .and_then(|path| path.file_name().map(|name| name.to_owned()))
                    .and_then(|name| name.into_string().ok())
                    .filter(|value| !value.is_empty())
            })
            .unwrap_or_else(|| "unknown".to_string())
    }
}

fn join_with_current_dir(chlid: &str) -> Option<String> {
    let mut path = current_exe().ok()?;

    path.pop();
    normalize_existing_path(path.join(chlid))
}

fn first_existing_path<const N: usize>(children: [&str; N]) -> Option<String> {
    let mut path = current_exe().ok()?;
    path.pop();

    children
        .into_iter()
        .find_map(|child| normalize_existing_path(path.join(child)))
}

fn normalize_existing_path(path: PathBuf) -> Option<String> {
    normalize_path(path.canonicalize().ok()?)
}

fn normalize_path(path: impl AsRef<Path>) -> Option<String> {
    Some(
        path.as_ref()
            .to_str()?
            .to_string()
            .replace("\\\\?\\", "")
            .replace("\\", "/"),
    )
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::parse()
    }
}
