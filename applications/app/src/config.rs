use std::{env::current_exe, fs};

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
        if cfg!(target_os = "macos") {
            let path = dirs::home_dir()
                .expect("The current user's home directory could not be found, probably because the user ID is missing.")
                .join("Library/Application Support/Hylarana")
                .to_str()
                .unwrap()
                .to_string();

            if !fs::exists(&path).unwrap_or(false) {
                fs::create_dir(&path).unwrap();
            }

            path
        } else if cfg!(target_os = "windows") {
            join_with_current_dir("./").unwrap()
        } else {
            unimplemented!()
        }
    }

    pub fn default_uri() -> String {
        "webview://index.html".to_string()
    }

    pub fn default_cheme_path() -> String {
        if cfg!(target_os = "macos") {
            join_with_current_dir("../Resources/webview").unwrap()
        } else {
            join_with_current_dir("webview").unwrap()
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
        format!(
            "{}-{}",
            dirs::home_dir()
                .unwrap()
                .to_str()
                .unwrap()
                .replace("\\", "/")
                .split("/")
                .last()
                .unwrap()
                .to_string(),
            if cfg!(target_os = "windows") {
                "windows"
            } else if cfg!(target_os = "linux") {
                "linux"
            } else {
                "macos"
            }
        )
    }
}

fn join_with_current_dir(chlid: &str) -> Option<String> {
    let mut path = current_exe().ok()?;

    path.pop();
    Some(
        path.join(chlid)
            .canonicalize()
            .ok()?
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
