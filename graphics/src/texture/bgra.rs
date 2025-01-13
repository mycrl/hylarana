use super::Texture2DSample;

    use std::borrow::Cow;

    use hylarana_common::Size;
    use wgpu::{
        Device, ShaderModuleDescriptor, ShaderSource, Texture, TextureAspect, TextureFormat,
    };

    const FRAGMENT_SHADER: &str = r#"
    @group(0) @binding(0) var texture_: texture_2d<f32>;
    @group(0) @binding(1) var sampler_: sampler;

    @fragment fn main(@location(0) coords: vec2<f32>) -> @location(0) vec4<f32> {
        return textureSample(texture_, sampler_, coords);
    }"#;

    pub struct Bgra(Texture);

    impl Bgra {
        pub(crate) fn new(device: &Device, size: Size) -> Self {
            Self(Self::create(device, size).next().unwrap())
        }
    }

    impl Texture2DSample for Bgra {
        fn fragment_shader() -> ShaderModuleDescriptor<'static> {
            ShaderModuleDescriptor {
                label: None,
                source: ShaderSource::Wgsl(Cow::Borrowed(FRAGMENT_SHADER)),
            }
        }

        fn create_texture_descriptor(
            size: Size,
        ) -> impl IntoIterator<Item = (Size, TextureFormat)> {
            [(size, TextureFormat::Bgra8Unorm)]
        }

        fn views_descriptors<'a>(
            &'a self,
            texture: Option<&'a Texture>,
        ) -> impl IntoIterator<Item = (&'a Texture, TextureFormat, TextureAspect)> {
            [(
                texture.unwrap_or_else(|| &self.0),
                TextureFormat::Bgra8Unorm,
                TextureAspect::All,
            )]
        }

        fn copy_buffer_descriptors<'a>(
            &self,
            buffers: &'a [&'a [u8]],
        ) -> impl IntoIterator<Item = (&'a [u8], &Texture, TextureAspect, Size)> {
            let size = self.0.size();
            [(
                buffers[0],
                &self.0,
                TextureAspect::All,
                Size {
                    width: size.width * 4,
                    height: size.height,
                },
            )]
        }
    }
