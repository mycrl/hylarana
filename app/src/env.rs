use std::{
    env::{current_dir, current_exe},
    fs::{create_dir, exists, read_to_string, write},
    time::SystemTime,
};

use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Settings {
    pub name: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            name: format!(
                "{}",
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
        }
    }
}

#[derive(Debug)]
pub struct Env {
    pub cache_path: String,
    pub scheme_path: String,
    pub settings: Settings,
}

impl Env {
    pub const ROOT: Lazy<String> = Lazy::new(|| {
        current_exe()
            .ok()
            // The path to the current executable file is obtained, but only the directory is needed
            // here, so the filename is removed.
            .map(|mut it| {
                it.pop();
                it
            })
            // Can't even get the current working directory? Give up and destroy.
            .unwrap_or_else(|| current_dir().unwrap())
            .as_path()
            .to_str()
            .unwrap()
            .to_string()
    });

    pub const CACHE_PATH: Lazy<String> = Lazy::new(|| format!("{}/caches", Self::ROOT.as_str()));

    pub const SETTINGS_PATH: Lazy<String> =
        Lazy::new(|| format!("{}/caches/settings", Self::ROOT.as_str()));

    pub const ENV_WEBVIEW_MAIN_PAGE_URL: &str = "WEBVIEW_MAIN_PAGE_URL";

    pub const ENV_ENABLE_WEBVIEW_DEVTOOLS: &str = "ENABLE_WEBVIEW_DEVTOOLS";

    pub fn new() -> Result<Self> {
        // The cache path may not exist, create the cache directory here, this is mainly
        // provided for webview use.
        if !is_exsit(Self::CACHE_PATH.as_str()) {
            create_dir(Self::CACHE_PATH.as_str())?;
        }

        if !is_exsit(Self::SETTINGS_PATH.as_str()) {
            update_settings(&Settings::default())?;
        }

        Ok(Self {
            settings: get_settings()?,
            cache_path: Self::CACHE_PATH.to_string(),
            scheme_path: format!("{}/resources", Self::ROOT.as_str()),
        })
    }

    pub fn update_name(&mut self, name: String) -> Result<()> {
        self.settings.name = name;
        update_settings(&self.settings)?;

        Ok(())
    }
}

fn update_settings(settings: &Settings) -> Result<()> {
    write(Env::SETTINGS_PATH.as_str(), serde_json::to_vec(settings)?)?;

    Ok(())
}

fn get_settings() -> Result<Settings> {
    Ok(serde_json::from_slice(
        read_to_string(Env::SETTINGS_PATH.as_str())?.as_bytes(),
    )?)
}

fn is_exsit(path: &str) -> bool {
    exists(path).unwrap_or_else(|_| false)
}
