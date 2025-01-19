use super::Texture2DSample;

pub mod bgra {
    use super::Texture2DSample;

    use std::borrow::Cow;

    use common::{frame::VideoSubFormat, Size};
    use wgpu::{
        Device, ShaderModuleDescriptor, ShaderSource, Texture, TextureAspect, TextureFormat,
    };

    const FRAGMENT_SHADER: &str = r#"
    @group(0) @binding(0) var texture_: texture_2d<f32>;
    @group(0) @binding(1) var sampler_: sampler;

    @fragment fn main(@location(0) coords: vec2<f32>) -> @location(0) vec4<f32> {
        return textureSample(texture_, sampler_, coords);
    }"#;

    pub struct Bgra(Option<Texture>);

    impl Bgra {
        pub(crate) fn new(device: &Device, size: Size, sub_format: VideoSubFormat) -> Self {
            Self(Self::create(device, size, sub_format).next())
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
            sub_format: VideoSubFormat,
        ) -> impl IntoIterator<Item = (Size, TextureFormat)> {
            if sub_format == VideoSubFormat::SW {
                vec![(size, TextureFormat::Bgra8Unorm)]
            } else {
                Vec::new()
            }
        }

        fn views_descriptors<'a>(
            &'a self,
            texture: Option<&'a Texture>,
        ) -> impl IntoIterator<Item = (&'a Texture, TextureFormat, TextureAspect)> {
            [(
                texture.unwrap_or_else(|| self.0.as_ref().unwrap()),
                TextureFormat::Bgra8Unorm,
                TextureAspect::All,
            )]
        }

        fn copy_buffer_descriptors<'a>(
            &self,
            buffers: &'a [&'a [u8]],
        ) -> impl IntoIterator<Item = (&'a [u8], &Texture, TextureAspect, Size)> {
            let texture = self.0.as_ref().unwrap();
            let size = texture.size();
            [(
                buffers[0],
                texture,
                TextureAspect::All,
                Size {
                    width: size.width * 4,
                    height: size.height,
                },
            )]
        }
    }
}

pub mod i420 {
    use super::Texture2DSample;

    use std::borrow::Cow;

    use common::{frame::VideoSubFormat, Size};
    use wgpu::{
        Device, ShaderModuleDescriptor, ShaderSource, Texture, TextureAspect, TextureFormat,
    };

    const FRAGMENT_SHADER: &str = r#"
    @group(0) @binding(0) var y_texture: texture_2d<f32>;
    @group(0) @binding(1) var u_texture: texture_2d<f32>;
    @group(0) @binding(2) var v_texture: texture_2d<f32>;
    @group(0) @binding(3) var sampler_: sampler;

    @fragment fn main(@location(0) coords: vec2<f32>) -> @location(0) vec4<f32> {
        let y = textureSample(y_texture, sampler_, coords).r;
        let u = textureSample(u_texture, sampler_, coords).r - 0.5;
        let v = textureSample(v_texture, sampler_, coords).r - 0.5;

        let r = y + 1.5748 * v;
        let g = y - 0.187324 * u - 0.468124 * v;
        let b = y + 1.8556 * u;

        return vec4<f32>(r, g, b, 1.0);
    }"#;

    /// YCbCr, Y′CbCr, or Y Pb/Cb Pr/Cr, also written as YCBCR or Y′CBCR, is a
    /// family of color spaces used as a part of the color image pipeline in
    /// video and digital photography systems. Y′ is the luma component and
    /// CB and CR are the blue-difference and red-difference chroma
    /// components. Y′ (with prime) is distinguished from Y, which is
    /// luminance, meaning that light intensity is nonlinearly encoded based
    /// on gamma corrected RGB primaries.
    ///
    /// Y′CbCr color spaces are defined by a mathematical coordinate
    /// transformation from an associated RGB primaries and white point. If
    /// the underlying RGB color space is absolute, the Y′CbCr color space
    /// is an absolute color space as well; conversely, if the RGB space is
    /// ill-defined, so is Y′CbCr. The transformation is defined in
    /// equations 32, 33 in ITU-T H.273. Nevertheless that rule does not
    /// apply to P3-D65 primaries used by Netflix with BT.2020-NCL matrix,
    /// so that means matrix was not derived from primaries, but now Netflix
    /// allows BT.2020 primaries (since 2021).[1] The same happens with
    /// JPEG: it has BT.601 matrix derived from System M primaries, yet the
    /// primaries of most images are BT.709.
    pub struct I420(Texture, Texture, Texture);

    impl I420 {
        pub(crate) fn new(device: &Device, size: Size, sub_format: VideoSubFormat) -> Self {
            let mut textures = Self::create(device, size, sub_format);
            Self(
                textures.next().unwrap(),
                textures.next().unwrap(),
                textures.next().unwrap(),
            )
        }
    }

    impl Texture2DSample for I420 {
        fn fragment_shader() -> ShaderModuleDescriptor<'static> {
            ShaderModuleDescriptor {
                label: None,
                source: ShaderSource::Wgsl(Cow::Borrowed(FRAGMENT_SHADER)),
            }
        }

        fn create_texture_descriptor(
            size: Size,
            _: VideoSubFormat,
        ) -> impl IntoIterator<Item = (Size, TextureFormat)> {
            [
                (size, TextureFormat::R8Unorm),
                (
                    Size {
                        width: size.width / 2,
                        height: size.height / 2,
                    },
                    TextureFormat::R8Unorm,
                ),
                (
                    Size {
                        width: size.width / 2,
                        height: size.height / 2,
                    },
                    TextureFormat::R8Unorm,
                ),
            ]
        }

        fn views_descriptors<'a>(
            &'a self,
            _: Option<&'a Texture>,
        ) -> impl IntoIterator<Item = (&'a Texture, TextureFormat, TextureAspect)> {
            [
                (&self.0, TextureFormat::R8Unorm, TextureAspect::All),
                (&self.1, TextureFormat::R8Unorm, TextureAspect::All),
                (&self.2, TextureFormat::R8Unorm, TextureAspect::All),
            ]
        }

        fn copy_buffer_descriptors<'a>(
            &self,
            buffers: &'a [&'a [u8]],
        ) -> impl IntoIterator<Item = (&'a [u8], &Texture, TextureAspect, Size)> {
            let size = {
                let size = self.0.size();
                Size {
                    width: size.width,
                    height: size.height,
                }
            };

            [
                (buffers[0], &self.0, TextureAspect::All, size),
                (
                    buffers[1],
                    &self.1,
                    TextureAspect::All,
                    Size {
                        width: size.width / 2,
                        height: size.height / 2,
                    },
                ),
                (
                    buffers[2],
                    &self.2,
                    TextureAspect::All,
                    Size {
                        width: size.width / 2,
                        height: size.height / 2,
                    },
                ),
            ]
        }
    }
}

pub mod nv12 {
    use super::Texture2DSample;

    use std::borrow::Cow;

    use common::{frame::VideoSubFormat, Size};
    use wgpu::{
        Device, ShaderModuleDescriptor, ShaderSource, Texture, TextureAspect, TextureFormat,
    };

    const FRAGMENT_SHADER: &str = r#"
    @group(0) @binding(0) var y_texture: texture_2d<f32>;
    @group(0) @binding(1) var uv_texture: texture_2d<f32>;
    @group(0) @binding(2) var sampler_: sampler;

    @fragment fn main(@location(0) coords: vec2<f32>) -> @location(0) vec4<f32> {
        let y = textureSample(y_texture, sampler_, coords).r;
        let u = textureSample(uv_texture, sampler_, coords).r - 0.5;
        let v = textureSample(uv_texture, sampler_, coords).g - 0.5;

        let r = y + 1.5748 * v;
        let g = y - 0.187324 * u - 0.468124 * v;
        let b = y + 1.8556 * u;

        return vec4<f32>(r, g, b, 1.0);
    }"#;

    /// YCbCr, Y′CbCr, or Y Pb/Cb Pr/Cr, also written as YCBCR or Y′CBCR, is a
    /// family of color spaces used as a part of the color image pipeline in
    /// video and digital photography systems. Y′ is the luma component and
    /// CB and CR are the blue-difference and red-difference chroma
    /// components. Y′ (with prime) is distinguished from Y, which is
    /// luminance, meaning that light intensity is nonlinearly encoded based
    /// on gamma corrected RGB primaries.
    ///
    /// Y′CbCr color spaces are defined by a mathematical coordinate
    /// transformation from an associated RGB primaries and white point. If
    /// the underlying RGB color space is absolute, the Y′CbCr color space
    /// is an absolute color space as well; conversely, if the RGB space is
    /// ill-defined, so is Y′CbCr. The transformation is defined in
    /// equations 32, 33 in ITU-T H.273. Nevertheless that rule does not
    /// apply to P3-D65 primaries used by Netflix with BT.2020-NCL matrix,
    /// so that means matrix was not derived from primaries, but now Netflix
    /// allows BT.2020 primaries (since 2021).[1] The same happens with
    /// JPEG: it has BT.601 matrix derived from System M primaries, yet the
    /// primaries of most images are BT.709.
    ///
    /// NV12 is possibly the most commonly-used 8-bit 4:2:0 format. It is the
    /// default for Android camera preview.[19] The entire image in Y is written
    /// out, followed by interleaved lines that go U0, V0, U1, V1, etc.
    pub struct Nv12(Option<(Texture, Texture)>);

    impl Nv12 {
        pub(crate) fn new(device: &Device, size: Size, sub_format: VideoSubFormat) -> Self {
            let mut textures = Self::create(device, size, sub_format);
            Self(if sub_format == VideoSubFormat::D3D11 {
                None
            } else {
                Some((textures.next().unwrap(), textures.next().unwrap()))
            })
        }
    }

    impl Texture2DSample for Nv12 {
        fn fragment_shader() -> ShaderModuleDescriptor<'static> {
            ShaderModuleDescriptor {
                label: None,
                source: ShaderSource::Wgsl(Cow::Borrowed(FRAGMENT_SHADER)),
            }
        }

        fn create_texture_descriptor(
            size: Size,
            sub_format: VideoSubFormat,
        ) -> impl IntoIterator<Item = (Size, TextureFormat)> {
            if sub_format == VideoSubFormat::D3D11 {
                Vec::new()
            } else {
                vec![
                    (size, TextureFormat::R8Unorm),
                    (
                        Size {
                            width: size.width / 2,
                            height: size.height / 2,
                        },
                        TextureFormat::Rg8Unorm,
                    ),
                ]
            }
        }

        fn views_descriptors<'a>(
            &'a self,
            texture: Option<&'a Texture>,
        ) -> impl IntoIterator<Item = (&'a Texture, TextureFormat, TextureAspect)> {
            // When you create a view directly for a texture, the external texture is a
            // single texture, and you need to create different planes of views on top of
            // the single texture.
            if let Some(texture) = texture {
                [
                    (texture, TextureFormat::R8Unorm, TextureAspect::Plane0),
                    (texture, TextureFormat::Rg8Unorm, TextureAspect::Plane1),
                ]
            } else {
                let textures = self.0.as_ref().unwrap();
                [
                    (&textures.0, TextureFormat::R8Unorm, TextureAspect::All),
                    (&textures.0, TextureFormat::Rg8Unorm, TextureAspect::All),
                ]
            }
        }

        fn copy_buffer_descriptors<'a>(
            &self,
            buffers: &'a [&'a [u8]],
        ) -> impl IntoIterator<Item = (&'a [u8], &Texture, TextureAspect, Size)> {
            let textures = self.0.as_ref().unwrap();
            let size = {
                let size = textures.0.size();
                Size {
                    width: size.width,
                    height: size.height,
                }
            };

            [
                (buffers[0], &textures.0, TextureAspect::All, size),
                (buffers[1], &textures.0, TextureAspect::All, size),
            ]
        }
    }
}

pub mod rgba {
    use super::Texture2DSample;

    use std::borrow::Cow;

    use common::{frame::VideoSubFormat, Size};
    use wgpu::{
        Device, ShaderModuleDescriptor, ShaderSource, Texture, TextureAspect, TextureFormat,
    };

    const FRAGMENT_SHADER: &str = r#"
    @group(0) @binding(0) var texture_: texture_2d<f32>;
    @group(0) @binding(1) var sampler_: sampler;

    @fragment fn main(@location(0) coords: vec2<f32>) -> @location(0) vec4<f32> {
        return textureSample(texture_, sampler_, coords);
    }"#;

    /// RGBA stands for red green blue alpha. While it is sometimes described as
    /// a color space, it is actually a three-channel RGB color model
    /// supplemented with a fourth alpha channel. Alpha indicates how opaque
    /// each pixel is and allows an image to be combined over others using
    /// alpha compositing, with transparent areas and anti-aliasing of the
    /// edges of opaque regions. Each pixel is a 4D vector.
    ///
    /// The term does not define what RGB color space is being used. It also
    /// does not state whether or not the colors are premultiplied by the
    /// alpha value, and if they are it does not state what color space that
    /// premultiplication was done in. This means more information than just
    /// "RGBA" is needed to determine how to handle an image.
    ///
    /// In some contexts the abbreviation "RGBA" means a specific memory layout
    /// (called RGBA8888 below), with other terms such as "BGRA" used for
    /// alternatives. In other contexts "RGBA" means any layout.
    pub struct Rgba(Option<Texture>);

    impl Rgba {
        pub(crate) fn new(device: &Device, size: Size, sub_format: VideoSubFormat) -> Self {
            Self(Self::create(device, size, sub_format).next())
        }
    }

    impl Texture2DSample for Rgba {
        fn fragment_shader() -> ShaderModuleDescriptor<'static> {
            ShaderModuleDescriptor {
                label: None,
                source: ShaderSource::Wgsl(Cow::Borrowed(FRAGMENT_SHADER)),
            }
        }

        fn create_texture_descriptor(
            size: Size,
            sub_format: VideoSubFormat,
        ) -> impl IntoIterator<Item = (Size, TextureFormat)> {
            if sub_format == VideoSubFormat::SW {
                vec![(size, TextureFormat::Rgba8Unorm)]
            } else {
                Vec::new()
            }
        }

        fn views_descriptors<'a>(
            &'a self,
            texture: Option<&'a Texture>,
        ) -> impl IntoIterator<Item = (&'a Texture, TextureFormat, TextureAspect)> {
            [(
                texture.unwrap_or_else(|| self.0.as_ref().unwrap()),
                TextureFormat::Rgba8Unorm,
                TextureAspect::All,
            )]
        }

        fn copy_buffer_descriptors<'a>(
            &self,
            buffers: &'a [&'a [u8]],
        ) -> impl IntoIterator<Item = (&'a [u8], &Texture, TextureAspect, Size)> {
            let texture = self.0.as_ref().unwrap();
            let size = texture.size();
            [(
                buffers[0],
                texture,
                TextureAspect::All,
                Size {
                    width: size.width * 4,
                    height: size.height,
                },
            )]
        }
    }
}
