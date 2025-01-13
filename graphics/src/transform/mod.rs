#[cfg(target_os = "windows")]
mod direct3d;

#[cfg(target_os = "macos")]
mod metal;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransformError {
    #[cfg(target_os = "windows")]
    #[error(transparent)]
    DxError(#[from] hylarana_common::win32::windows::core::Error),
    #[error("not found wgpu dx12 device")]
    NotFoundDxBackend,
    #[error("dx11 shared handle is invalid")]
    InvalidDxSharedHandle,
    #[error("not found wgpu metal device")]
    NotFoundMetalBackend,
    #[error("failed to create metal texture cache")]
    CreateMetalTextureCacheError,
    #[error("failed to create metal texture")]
    CreateMetalTextureError,
}
