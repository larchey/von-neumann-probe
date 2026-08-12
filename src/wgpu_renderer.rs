use bevy_math::Vec2;
use std::sync::Arc;

pub struct WgpuRenderer {
    surface: Option<wgpu::Surface<'static>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    config: Option<wgpu::SurfaceConfiguration>,
    size: (u32, u32),
    pipeline: Option<wgpu::RenderPipeline>,
    vertex_buffer: Option<wgpu::Buffer>,
    camera_buffer: Option<wgpu::Buffer>,
    bind_group: Option<wgpu::BindGroup>,
}

impl Default for WgpuRenderer {
    fn default() -> Self {
        Self {
            surface: None,
            device: None,
            queue: None,
            config: None,
            size: (1280, 720),
            pipeline: None,
            vertex_buffer: None,
            camera_buffer: None,
            bind_group: None,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

unsafe impl bytemuck::Pod for CameraUniform {}
unsafe impl bytemuck::Zeroable for CameraUniform {}

impl WgpuRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn init(&mut self) {
        println!("[RENDERER] Initializing wgpu backend...");
    }

    pub fn render(&mut self, entities: &[(Vec2, [f32; 3], f32)]) {
    }

    pub fn resize(&mut self, new_size: (u32, u32)) {
        self.size = new_size;
    }
}

const VERTEX_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

struct CameraUniform {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

pub struct RenderBatch {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

impl RenderBatch {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn add_circle(&mut self, center: Vec2, radius: f32, color: [f32; 3], segments: u16) {
        let base_index = self.vertices.len() as u16;

        self.vertices.push(Vertex {
            position: [center.x, center.y],
            color,
        });

        for i in 0..=segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            self.vertices.push(Vertex {
                position: [
                    center.x + angle.cos() * radius,
                    center.y + angle.sin() * radius,
                ],
                color,
            });

            if i > 0 {
                self.indices.push(base_index);
                self.indices.push(base_index + i);
                self.indices.push(base_index + i + 1);
            }
        }
    }

    pub fn add_line(&mut self, start: Vec2, end: Vec2, color: [f32; 3], thickness: f32) {
        let base_index = self.vertices.len() as u16;

        let dir = (end - start).normalize();
        let perp = Vec2::new(-dir.y, dir.x) * thickness / 2.0;

        self.vertices.push(Vertex {
            position: [(start + perp).x, (start + perp).y],
            color,
        });
        self.vertices.push(Vertex {
            position: [(start - perp).x, (start - perp).y],
            color,
        });
        self.vertices.push(Vertex {
            position: [(end + perp).x, (end + perp).y],
            color,
        });
        self.vertices.push(Vertex {
            position: [(end - perp).x, (end - perp).y],
            color,
        });

        self.indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index + 1,
            base_index + 3,
            base_index + 2,
        ]);
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}
