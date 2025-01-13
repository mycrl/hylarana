use bytemuck::{Pod, Zeroable};
use wgpu::{BufferAddress, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl Vertex {
    pub const INDICES: &'static [u16] = &[0, 1, 2, 2, 1, 3];

    pub const VERTICES: &'static [Vertex] = &[
        Vertex::new([-1.0, -1.0], [0.0, 0.0]),
        Vertex::new([1.0, -1.0], [1.0, 0.0]),
        Vertex::new([-1.0, 1.0], [0.0, 1.0]),
        Vertex::new([1.0, 1.0], [1.0, 1.0]),
    ];

    pub const VERTEX_SHADER: &str = r#"
    struct VertexOutput {
        @builtin(position) position: vec4<f32>,
        @location(0) coords: vec2<f32>,
    };

    @vertex fn main(@location(0) position: vec2<f32>, @location(1) coords: vec2<f32>) -> VertexOutput {
        var output: VertexOutput;
        output.position = vec4<f32>(position, 0.0, 1.0);
        output.coords = vec2<f32>(coords.x, 1.0 - coords.y);
        return output;
    }"#;

    pub const fn new(position: [f32; 2], tex_coords: [f32; 2]) -> Self {
        Self {
            position,
            tex_coords,
        }
    }

    pub fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    shader_location: 1,
                    format: VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 2]>() as BufferAddress,
                },
            ],
        }
    }
}
