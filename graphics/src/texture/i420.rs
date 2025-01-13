use super::Texture2DSample;

    use std::borrow::Cow;

    use hylarana_common::Size;
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
        pub(crate) fn new(device: &Device, size: Size) -> Self {
            let mut textures = Self::create(device, size);
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
