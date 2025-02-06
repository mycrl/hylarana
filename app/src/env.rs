use std::{
    env::{current_dir, current_exe},
    fs::{create_dir, exists, read_to_string, write},
    time::SystemTime,
};

use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static ROOT: Lazy<String> = Lazy::new(|| {
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

pub static CACHE_PATH: Lazy<String> = Lazy::new(|| format!("{}/caches", ROOT.as_str()));

pub static SETTINGS_PATH: Lazy<String> = Lazy::new(|| format!("{}/caches/settings", ROOT.as_str()));

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
    pub fn new() -> Result<Self> {
        // The cache path may not exist, create the cache directory here, this is mainly
        // provided for webview use.
        if !is_exsit(CACHE_PATH.as_str()) {
            create_dir(CACHE_PATH.as_str())?;
        }

        if !is_exsit(SETTINGS_PATH.as_str()) {
            update_settings(&Settings::default())?;
        }

        Ok(Self {
            settings: get_settings()?,
            cache_path: CACHE_PATH.to_string(),
            scheme_path: format!("{}/resources", ROOT.as_str()),
        })
    }

    pub fn update_name(&mut self, name: String) -> Result<()> {
        self.settings.name = name;
        update_settings(&self.settings)?;

        Ok(())
    }
}

fn update_settings(settings: &Settings) -> Result<()> {
    write(SETTINGS_PATH.as_str(), serde_json::to_vec(settings)?)?;

    Ok(())
}

fn get_settings() -> Result<Settings> {
    Ok(serde_json::from_slice(
        read_to_string(SETTINGS_PATH.as_str())?.as_bytes(),
    )?)
}

fn is_exsit(path: &str) -> bool {
    exists(path).unwrap_or_else(|_| false)
}
