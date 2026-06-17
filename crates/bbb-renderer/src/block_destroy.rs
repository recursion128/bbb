use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;

use crate::{
    gpu::DEPTH_FORMAT,
    terrain::{TerrainTint, TerrainUvRect, TerrainVertex},
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BlockDestroyOverlay {
    pub pos: [i32; 3],
    pub uv: TerrainUvRect,
}

const BLOCK_DESTROY_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x2, 4 => Float32x3, 5 => Float32, 6 => Float32, 7 => Sint32];
const BLOCK_DESTROY_FACE_OFFSET: f32 = 0.003;

const BLOCK_DESTROY_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var terrain_atlas: texture_2d<f32>;

@group(0) @binding(2)
var terrain_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) light: vec2<f32>,
    @location(4) tint: vec3<f32>,
    @location(5) shade: f32,
    @location(6) ambient_occlusion: f32,
    @location(7) block_state_id: i32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(terrain_atlas, terrain_sampler, input.uv);
    if texel.a <= 0.01 {
        discard;
    }
    return vec4<f32>(texel.rgb, texel.a * 0.85);
}
"#;

pub(super) struct BlockDestroyOverlayGpu {
    pub(super) overlay: BlockDestroyOverlay,
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) index_buffer: wgpu::Buffer,
    pub(super) index_count: u32,
}

pub(super) fn create_block_destroy_overlay_gpu(
    device: &wgpu::Device,
    overlay: BlockDestroyOverlay,
) -> BlockDestroyOverlayGpu {
    let mesh = block_destroy_overlay_mesh(overlay);
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-block-destroy-overlay-vertices"),
        contents: bytemuck::cast_slice(&mesh.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bbb-block-destroy-overlay-indices"),
        contents: bytemuck::cast_slice(&mesh.indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    BlockDestroyOverlayGpu {
        overlay,
        vertex_buffer,
        index_buffer,
        index_count: mesh.indices.len() as u32,
    }
}

pub(super) fn create_block_destroy_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    terrain_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-block-destroy-overlay-shader"),
        source: wgpu::ShaderSource::Wgsl(BLOCK_DESTROY_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-block-destroy-overlay-pipeline-layout"),
        bind_group_layouts: &[terrain_bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bbb-block-destroy-overlay-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[block_destroy_vertex_layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

struct BlockDestroyOverlayMesh {
    vertices: Vec<TerrainVertex>,
    indices: Vec<u32>,
}

fn block_destroy_overlay_mesh(overlay: BlockDestroyOverlay) -> BlockDestroyOverlayMesh {
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for face in DESTROY_OVERLAY_FACES {
        emit_block_destroy_face(&mut vertices, &mut indices, overlay, face);
    }
    BlockDestroyOverlayMesh { vertices, indices }
}

fn emit_block_destroy_face(
    vertices: &mut Vec<TerrainVertex>,
    indices: &mut Vec<u32>,
    overlay: BlockDestroyOverlay,
    face: DestroyOverlayFace,
) {
    let base = vertices.len() as u32;
    let min = [
        overlay.pos[0] as f32,
        overlay.pos[1] as f32,
        overlay.pos[2] as f32,
    ];
    let max = [min[0] + 1.0, min[1] + 1.0, min[2] + 1.0];
    for (corner, uv) in
        face.corners
            .into_iter()
            .zip([[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]])
    {
        let mut position = [
            if corner[0] == 0 { min[0] } else { max[0] },
            if corner[1] == 0 { min[1] } else { max[1] },
            if corner[2] == 0 { min[2] } else { max[2] },
        ];
        position[0] += face.normal[0] * BLOCK_DESTROY_FACE_OFFSET;
        position[1] += face.normal[1] * BLOCK_DESTROY_FACE_OFFSET;
        position[2] += face.normal[2] * BLOCK_DESTROY_FACE_OFFSET;
        vertices.push(TerrainVertex {
            position,
            normal: face.normal,
            uv: overlay.uv.map(uv),
            light: [1.0, 1.0],
            tint: TerrainTint::WHITE.as_shader_tint(),
            shade: 1.0,
            ambient_occlusion: 1.0,
            block_state_id: -1,
        });
    }
    indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[derive(Debug, Clone, Copy)]
struct DestroyOverlayFace {
    normal: [f32; 3],
    corners: [[u8; 3]; 4],
}

const DESTROY_OVERLAY_FACES: [DestroyOverlayFace; 6] = [
    DestroyOverlayFace {
        normal: [0.0, -1.0, 0.0],
        corners: [[0, 0, 1], [1, 0, 1], [1, 0, 0], [0, 0, 0]],
    },
    DestroyOverlayFace {
        normal: [0.0, 1.0, 0.0],
        corners: [[0, 1, 0], [1, 1, 0], [1, 1, 1], [0, 1, 1]],
    },
    DestroyOverlayFace {
        normal: [0.0, 0.0, -1.0],
        corners: [[1, 0, 0], [0, 0, 0], [0, 1, 0], [1, 1, 0]],
    },
    DestroyOverlayFace {
        normal: [0.0, 0.0, 1.0],
        corners: [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]],
    },
    DestroyOverlayFace {
        normal: [-1.0, 0.0, 0.0],
        corners: [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]],
    },
    DestroyOverlayFace {
        normal: [1.0, 0.0, 0.0],
        corners: [[1, 0, 1], [1, 0, 0], [1, 1, 0], [1, 1, 1]],
    },
];

fn block_destroy_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<TerrainVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &BLOCK_DESTROY_VERTEX_ATTRIBUTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_destroy_overlay_mesh_emits_offset_cube_faces() {
        let mesh = block_destroy_overlay_mesh(BlockDestroyOverlay {
            pos: [2, 3, -4],
            uv: TerrainUvRect {
                min: [0.25, 0.5],
                max: [0.5, 0.75],
            },
        });

        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
        assert_eq!(mesh.vertices[0].position, [2.0, 2.997, -3.0]);
        assert_eq!(mesh.vertices[0].normal, [0.0, -1.0, 0.0]);
        assert_eq!(mesh.vertices[0].uv, [0.25, 0.75]);
        assert_eq!(mesh.vertices[1].uv, [0.5, 0.75]);
        assert_eq!(mesh.vertices[2].uv, [0.5, 0.5]);
        assert_eq!(mesh.vertices[3].uv, [0.25, 0.5]);
        assert_eq!(mesh.indices[0..6], [0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn block_destroy_overlay_mesh_emits_all_six_faces() {
        let mesh = block_destroy_overlay_mesh(BlockDestroyOverlay {
            pos: [0, 0, 0],
            uv: TerrainUvRect::UNIT,
        });

        let normals: Vec<_> = mesh
            .vertices
            .chunks_exact(4)
            .map(|face| face[0].normal)
            .collect();
        assert_eq!(
            normals,
            vec![
                [0.0, -1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, -1.0],
                [0.0, 0.0, 1.0],
                [-1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            ]
        );
    }
}
