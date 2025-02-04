use std::{
    env::{current_dir, current_exe},
    fs::{create_dir, exists},
};

use once_cell::sync::Lazy;

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

pub struct Env {
    pub cache_path: String,
    pub scheme_path: String,
}

impl Default for Env {
    fn default() -> Self {
        let cache_path = format!("{}/caches", ROOT.as_str());

        // The cache path may not exist, create the cache directory here, this is mainly
        // provided for webview use.
        if !exists(&cache_path).unwrap_or_else(|_| false) {
            create_dir(&cache_path).unwrap();
        }

        Self {
            scheme_path: format!("{}/resources", ROOT.as_str()),
            cache_path,
        }
    }
}
