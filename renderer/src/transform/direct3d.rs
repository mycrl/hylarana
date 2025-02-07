use super::TransformError;

use common::{
    frame::VideoFormat,
    win32::{
        windows::Win32::Graphics::{
            Direct3D11::{
                ID3D11Texture2D, D3D11_RESOURCE_MISC_SHARED, D3D11_TEXTURE2D_DESC,
                D3D11_USAGE_DEFAULT,
            },
            Direct3D12::ID3D12Resource,
            Dxgi::Common::{
                DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_NV12, DXGI_FORMAT_R8G8B8A8_UNORM,
            },
        },
        Direct3DDevice, EasyTexture,
    },
    Size,
};

use wgpu::{
    hal::api::Dx12, Device, Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages,
};

pub struct Transformer {
    direct3d: Direct3DDevice,
    raw_texture: ID3D11Texture2D,
    texture: Texture,
}

unsafe impl Sync for Transformer {}
unsafe impl Send for Transformer {}

impl Transformer {
    pub fn new(
        direct3d: Direct3DDevice,
        device: &Device,
        size: Size,
        format: VideoFormat,
    ) -> Result<Self, TransformError> {
        // Gets the incoming texture properties, the new texture contains only an array
        // of textures and is a shareable texture resource.
        let mut d3d11_desc = D3D11_TEXTURE2D_DESC::default();
        d3d11_desc.Width = size.width;
        d3d11_desc.Height = size.height;
        d3d11_desc.MipLevels = 1;
        d3d11_desc.ArraySize = 1;
        d3d11_desc.SampleDesc.Count = 1;
        d3d11_desc.SampleDesc.Quality = 0;
        d3d11_desc.BindFlags = 0;
        d3d11_desc.CPUAccessFlags = 0;
        d3d11_desc.Usage = D3D11_USAGE_DEFAULT;
        d3d11_desc.MiscFlags = D3D11_RESOURCE_MISC_SHARED.0 as u32;
        d3d11_desc.Format = match format {
            VideoFormat::NV12 => DXGI_FORMAT_NV12,
            VideoFormat::BGRA => DXGI_FORMAT_B8G8R8A8_UNORM,
            VideoFormat::RGBA => DXGI_FORMAT_R8G8B8A8_UNORM,
            _ => unimplemented!("not supports format={:?}", format),
        };

        // Creates a new texture, which serves as the current texture to be used, and to
        // which external input textures are updated.
        let mut raw_texture = None;
        unsafe {
            direct3d
                .device
                .CreateTexture2D(&d3d11_desc, None, Some(&mut raw_texture))?;
        }

        let raw_texture = raw_texture.unwrap();

        // Get the texture's shared resources. dx11 textures need to be shared resources
        // if they are to be used by dx12 devices.
        let texture = {
            let desc = TextureDescriptor {
                label: None,
                mip_level_count: d3d11_desc.MipLevels,
                sample_count: d3d11_desc.SampleDesc.Count,
                dimension: TextureDimension::D2,
                usage: TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
                size: Extent3d {
                    depth_or_array_layers: d3d11_desc.ArraySize,
                    width: d3d11_desc.Width,
                    height: d3d11_desc.Height,
                },
                format: match d3d11_desc.Format {
                    DXGI_FORMAT_NV12 => TextureFormat::NV12,
                    DXGI_FORMAT_R8G8B8A8_UNORM => TextureFormat::Rgba8Unorm,
                    DXGI_FORMAT_B8G8R8A8_UNORM => TextureFormat::Bgra8Unorm,
                    _ => unimplemented!("not supports texture format"),
                },
            };

            // Converts dx12 resources to textures that wgpu can use.
            unsafe {
                device.create_texture_from_hal::<Dx12>(
                    <Dx12 as wgpu::hal::Api>::Device::texture_from_raw(
                        {
                            device.as_hal::<Dx12, _, _>(|hdevice| {
                                let mut resource = None::<ID3D12Resource>;

                                hdevice
                                    .ok_or_else(|| TransformError::NotFoundDxBackend)?
                                    .raw_device()
                                    .OpenSharedHandle(
                                        {
                                            let handle = raw_texture.get_shared()?;
                                            if handle.is_invalid() {
                                                return Err(TransformError::InvalidDxSharedHandle);
                                            }

                                            handle
                                        },
                                        &mut resource,
                                    )
                                    .map(|_| resource.unwrap())
                                    .map_err(|e| TransformError::WindowsError(e))
                            })?
                        },
                        desc.format,
                        desc.dimension,
                        desc.size,
                        desc.mip_level_count,
                        desc.sample_count,
                    ),
                    &desc,
                )
            }
        };

        Ok(Self {
            raw_texture,
            texture,
            direct3d,
        })
    }

    pub fn transform(
        &mut self,
        texture: &ID3D11Texture2D,
        index: u32,
    ) -> Result<&Texture, TransformError> {
        // Copies the input texture to the internal texture.
        unsafe {
            self.direct3d.context.CopySubresourceRegion(
                &self.raw_texture,
                0,
                0,
                0,
                0,
                texture,
                index,
                None,
            );
        }

        Ok(&self.texture)
    }
}
