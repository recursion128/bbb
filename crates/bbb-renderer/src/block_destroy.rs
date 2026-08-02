use anyhow::Result;
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;

use crate::{
    gpu::DEPTH_FORMAT,
    pipeline_builder::{depth_stencil_state, RenderPipelineBuilder},
    sheeted_decal::sheeted_decal_uv,
    terrain::{
        mesh::geometry::{box_face_corners, CROSS_FACES, FACES},
        TerrainRenderShape, TerrainUvRect,
    },
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockDestroyOverlay {
    pub pos: [i32; 3],
    pub uv: TerrainUvRect,
    /// The block's terrain render shape at this position, projected from the same
    /// `TerrainTextureState::block_render_data` source the chunk mesher uses. The crumbling decal is
    /// emitted over this shape's faces so stairs/slabs/fences/cross blocks crack over their real
    /// geometry, mirroring vanilla `BlockFeatureRenderer.renderBreakingBlockModelSubmits`, which
    /// feeds the block model's own quads through the crumbling buffer.
    pub shape: TerrainRenderShape,
}

const BLOCK_DESTROY_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x2, 4 => Float32x2];
const BLOCK_DESTROY_FACE_OFFSET: f32 = 0.003;
const BLOCK_DESTROY_CULL_MODE: Option<wgpu::Face> = Some(wgpu::Face::Back);
const BLOCK_DESTROY_DEPTH_WRITE_ENABLED: bool = false;
const BLOCK_DESTROY_DEPTH_COMPARE: wgpu::CompareFunction = wgpu::CompareFunction::LessEqual;
const BLOCK_DESTROY_CRUMBLING_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Dst,
        dst_factor: wgpu::BlendFactor::Src,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::Zero,
        operation: wgpu::BlendOperation::Add,
    },
};
const BLOCK_DESTROY_DEPTH_BIAS: wgpu::DepthBiasState = wgpu::DepthBiasState {
    constant: -10,
    slope_scale: -1.0,
    clamp: 0.0,
};

const BLOCK_DESTROY_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    lightmap_factors: vec4<f32>,
    lightmap_effects: vec4<f32>,
    block_light_tint: vec4<f32>,
    sky_light_color: vec4<f32>,
    ambient_color: vec4<f32>,
    night_vision_color: vec4<f32>,
    camera_position: vec4<f32>,
    fog_color: vec4<f32>,
    fog_distances: vec4<f32>,
    fog_visibility_ends: vec4<f32>,
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
    @location(2) decal_uv: vec2<f32>,
    @location(3) uv_rect_min: vec2<f32>,
    @location(4) uv_rect_size: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) decal_uv: vec2<f32>,
    @location(1) uv_rect_min: vec2<f32>,
    @location(2) uv_rect_size: vec2<f32>,
    @location(3) spherical_distance: f32,
    @location(4) cylindrical_distance: f32,
};

fn linear_fog_value(vertex_distance: f32, fog_start: f32, fog_end: f32) -> f32 {
    if (vertex_distance <= fog_start) {
        return 0.0;
    }
    if (vertex_distance >= fog_end) {
        return 1.0;
    }
    return (vertex_distance - fog_start) / (fog_end - fog_start);
}

fn apply_fog(color: vec4<f32>, spherical_distance: f32, cylindrical_distance: f32) -> vec4<f32> {
    let fog_value = max(
        linear_fog_value(spherical_distance, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(cylindrical_distance, camera.fog_distances.z, camera.fog_distances.w),
    );
    return vec4<f32>(mix(color.rgb, camera.fog_color.rgb, fog_value * camera.fog_color.a), color.a);
}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.decal_uv = input.decal_uv;
    out.uv_rect_min = input.uv_rect_min;
    out.uv_rect_size = input.uv_rect_size;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let atlas_uv = input.uv_rect_min + fract(input.decal_uv) * input.uv_rect_size;
    let color = textureSample(terrain_atlas, terrain_sampler, atlas_uv);
    if color.a < 0.1 {
        discard;
    }
    return apply_fog(color, input.spherical_distance, input.cylindrical_distance);
}
"#;

pub(super) struct BlockDestroyOverlaysGpu {
    pub(super) overlays: Vec<BlockDestroyOverlay>,
    pub(super) vertex_buffer: Option<wgpu::Buffer>,
    pub(super) index_buffer: Option<wgpu::Buffer>,
    pub(super) index_count: u32,
}

pub(super) fn create_block_destroy_overlays_gpu(
    device: &wgpu::Device,
    overlays: Vec<BlockDestroyOverlay>,
) -> BlockDestroyOverlaysGpu {
    let mesh = block_destroy_overlays_mesh(&overlays);
    let vertex_buffer = (!mesh.vertices.is_empty()).then(|| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bbb-block-destroy-overlay-vertices"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        })
    });
    let index_buffer = (!mesh.indices.is_empty()).then(|| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bbb-block-destroy-overlay-indices"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        })
    });
    BlockDestroyOverlaysGpu {
        overlays,
        vertex_buffer,
        index_buffer,
        index_count: mesh.indices.len() as u32,
    }
}

pub(super) fn create_block_destroy_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    terrain_bind_group_layout: &wgpu::BindGroupLayout,
) -> Result<wgpu::RenderPipeline> {
    create_block_destroy_pipeline_with_shaders(device, format, terrain_bind_group_layout, None)
}

pub(crate) fn create_block_destroy_pipeline_with_shaders(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    terrain_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedShaderPair>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        let (vertex, fragment) = crate::core_shaders::compile_selected_shader_pair(
            device,
            "bbb-selected-block-destroy",
            shaders,
        )?;
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bbb-selected-block-destroy-layout"),
            bind_group_layouts: &[terrain_bind_group_layout],
            push_constant_ranges: &[],
        });
        return Ok(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("bbb-selected-block-destroy-pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &vertex,
                    entry_point: "main",
                    buffers: &[block_destroy_vertex_layout()],
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: BLOCK_DESTROY_CULL_MODE,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    bias: BLOCK_DESTROY_DEPTH_BIAS,
                    ..depth_stencil_state(
                        DEPTH_FORMAT,
                        BLOCK_DESTROY_DEPTH_WRITE_ENABLED,
                        BLOCK_DESTROY_DEPTH_COMPARE,
                    )
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fragment,
                    entry_point: "main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(BLOCK_DESTROY_CRUMBLING_BLEND),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
            }),
        );
    }
    Ok(
        RenderPipelineBuilder::new(device, "bbb-block-destroy-overlay-pipeline")
            .shader("bbb-block-destroy-overlay-shader", BLOCK_DESTROY_SHADER)
            .layout(
                "bbb-block-destroy-overlay-pipeline-layout",
                &[terrain_bind_group_layout],
            )
            .vertex_buffers(&[block_destroy_vertex_layout()])
            .color_target(format, Some(BLOCK_DESTROY_CRUMBLING_BLEND))
            .cull_mode(BLOCK_DESTROY_CULL_MODE)
            .depth_stencil(wgpu::DepthStencilState {
                bias: BLOCK_DESTROY_DEPTH_BIAS,
                ..depth_stencil_state(
                    DEPTH_FORMAT,
                    BLOCK_DESTROY_DEPTH_WRITE_ENABLED,
                    BLOCK_DESTROY_DEPTH_COMPARE,
                )
            })
            .build(),
    )
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct BlockDestroyVertex {
    pub(super) position: [f32; 3],
    pub(super) normal: [f32; 3],
    /// Raw `SheetedDecalTextureGenerator` coordinates. They intentionally remain
    /// unwrapped until the fragment shader so repeated model spans interpolate
    /// across every destroy-texture tile instead of collapsing integer edges.
    pub(super) decal_uv: [f32; 2],
    pub(super) uv_rect_min: [f32; 2],
    pub(super) uv_rect_size: [f32; 2],
}

pub(super) struct BlockDestroyOverlayMesh {
    pub(super) vertices: Vec<BlockDestroyVertex>,
    pub(super) indices: Vec<u32>,
}

impl BlockDestroyOverlayMesh {
    pub(super) fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
}

fn block_destroy_overlays_mesh(overlays: &[BlockDestroyOverlay]) -> BlockDestroyOverlayMesh {
    let mut mesh = BlockDestroyOverlayMesh::new();
    mesh.vertices.reserve(overlays.len() * 24);
    mesh.indices.reserve(overlays.len() * 36);
    for overlay in overlays {
        emit_block_destroy_overlay(&mut mesh.vertices, &mut mesh.indices, overlay);
    }
    mesh
}

#[cfg(test)]
fn block_destroy_overlay_mesh(overlay: BlockDestroyOverlay) -> BlockDestroyOverlayMesh {
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    emit_block_destroy_overlay(&mut vertices, &mut indices, &overlay);
    BlockDestroyOverlayMesh { vertices, indices }
}

fn emit_block_destroy_overlay(
    vertices: &mut Vec<BlockDestroyVertex>,
    indices: &mut Vec<u32>,
    overlay: &BlockDestroyOverlay,
) {
    let pos = overlay.pos;
    let uv = overlay.uv;
    match &overlay.shape {
        TerrainRenderShape::Cube => {
            emit_block_destroy_box(
                vertices,
                indices,
                pos,
                uv,
                [0, 0, 0],
                [16, 16, 16],
                [true; 6],
            );
        }
        TerrainRenderShape::Quads(quads) => {
            for quad in quads {
                let base = vertices.len() as u32;
                for corner in quad.corners {
                    let local = [corner[0] / 16.0, corner[1] / 16.0, corner[2] / 16.0];
                    vertices.push(destroy_vertex(pos, local, quad.normal, uv));
                }
                indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
        TerrainRenderShape::Box {
            from,
            to,
            face_present,
            ..
        } => {
            emit_block_destroy_box(vertices, indices, pos, uv, *from, *to, *face_present);
        }
        TerrainRenderShape::Boxes(boxes) => {
            for shape_box in boxes {
                emit_block_destroy_box(
                    vertices,
                    indices,
                    pos,
                    uv,
                    shape_box.from,
                    shape_box.to,
                    shape_box.face_present,
                );
            }
        }
        TerrainRenderShape::Cross { .. } | TerrainRenderShape::Crosses(_) => {
            emit_block_destroy_cross(vertices, indices, pos, uv);
        }
    }
}

/// Emits the crumbling faces of one axis-aligned box (`from`/`to` in vanilla `0..=16` model units),
/// reusing the chunk mesher's `box_face_corners` + `[0, 1, 2, 0, 2, 3]` winding so the decal is
/// visible from exactly the sides the block's own faces are (see the fluid back-face note in
/// `terrain::mesh::emitter`).
fn emit_block_destroy_box(
    vertices: &mut Vec<BlockDestroyVertex>,
    indices: &mut Vec<u32>,
    pos: [i32; 3],
    uv_rect: TerrainUvRect,
    from: [u8; 3],
    to: [u8; 3],
    face_present: [bool; 6],
) {
    let min = [
        from[0] as f32 / 16.0,
        from[1] as f32 / 16.0,
        from[2] as f32 / 16.0,
    ];
    let max = [
        to[0] as f32 / 16.0,
        to[1] as f32 / 16.0,
        to[2] as f32 / 16.0,
    ];
    for face in FACES {
        if !face_present[face.face.index()] {
            continue;
        }
        let base = vertices.len() as u32;
        for corner in box_face_corners(face.face, min, max) {
            vertices.push(destroy_vertex(pos, corner, face.normal, uv_rect));
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

/// Emits the crumbling faces of a cross (foliage) block: the mesher's two `CROSS_FACES` planes, one
/// one-sided quad per direction so the decal shows on both sides of each plane. The planes span the
/// full `[0, 1]` cell, so the decal covers the whole crumbling sprite.
fn emit_block_destroy_cross(
    vertices: &mut Vec<BlockDestroyVertex>,
    indices: &mut Vec<u32>,
    pos: [i32; 3],
    uv_rect: TerrainUvRect,
) {
    for (_face, normal, corners) in CROSS_FACES {
        let base = vertices.len() as u32;
        for corner in corners {
            vertices.push(destroy_vertex(pos, corner, normal, uv_rect));
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

/// One crumbling vertex: the box-local `corner` offset to world space and nudged out along the face
/// normal by `BLOCK_DESTROY_FACE_OFFSET` (kept from the prior z-fight mechanism, together with the
/// pipeline depth bias), with the decal UV projected from the block-local corner.
fn destroy_vertex(
    pos: [i32; 3],
    corner: [f32; 3],
    normal: [f32; 3],
    uv_rect: TerrainUvRect,
) -> BlockDestroyVertex {
    BlockDestroyVertex {
        position: [
            pos[0] as f32 + corner[0] + normal[0] * BLOCK_DESTROY_FACE_OFFSET,
            pos[1] as f32 + corner[1] + normal[1] * BLOCK_DESTROY_FACE_OFFSET,
            pos[2] as f32 + corner[2] + normal[2] * BLOCK_DESTROY_FACE_OFFSET,
        ],
        normal,
        decal_uv: sheeted_decal_uv(corner, normal, 1.0),
        uv_rect_min: uv_rect.min,
        uv_rect_size: [
            uv_rect.max[0] - uv_rect.min[0],
            uv_rect.max[1] - uv_rect.min[1],
        ],
    }
}

/// Vanilla `SheetedDecalTextureGenerator.setNormal` projects the crumbling UV from each vertex's
/// block-local position onto the plane perpendicular to the face's nearest `Direction` (the
/// crumbling buffer runs at `textureScale = 1.0`). The raw coordinate is retained for
/// fragment-time repeat inside the atlas rectangle; folding at vertices would collapse model spans
/// whose endpoints differ by a whole tile.
#[cfg(test)]
fn destroy_decal_uv(face: crate::terrain::TerrainFace, local: [f32; 3]) -> [f32; 2] {
    sheeted_decal_uv(local, terrain_face_normal(face), 1.0)
}

#[cfg(test)]
fn terrain_face_normal(face: crate::terrain::TerrainFace) -> [f32; 3] {
    use crate::terrain::TerrainFace;
    match face {
        TerrainFace::Down => [0.0, -1.0, 0.0],
        TerrainFace::Up => [0.0, 1.0, 0.0],
        TerrainFace::North => [0.0, 0.0, -1.0],
        TerrainFace::South => [0.0, 0.0, 1.0],
        TerrainFace::West => [-1.0, 0.0, 0.0],
        TerrainFace::East => [1.0, 0.0, 0.0],
    }
}

fn block_destroy_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<BlockDestroyVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &BLOCK_DESTROY_VERTEX_ATTRIBUTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::{TerrainFace, TerrainQuad, TerrainTint, TerrainTransparency};

    #[test]
    fn block_destroy_crumbling_state_matches_vanilla_pipeline() {
        // Vanilla 26.1 `RenderPipelines.CRUMBLING` uses
        // `BlendFunction(DST_COLOR, SRC_COLOR, ONE, ZERO)` and
        // default cull plus `DepthStencilState(LESS_EQUAL, false, -1.0F, -10.0F)`.
        assert_eq!(BLOCK_DESTROY_CULL_MODE, Some(wgpu::Face::Back));
        assert!(!BLOCK_DESTROY_DEPTH_WRITE_ENABLED);
        assert_eq!(
            BLOCK_DESTROY_DEPTH_COMPARE,
            wgpu::CompareFunction::LessEqual
        );
        assert_eq!(
            BLOCK_DESTROY_CRUMBLING_BLEND.color.src_factor,
            wgpu::BlendFactor::Dst
        );
        assert_eq!(
            BLOCK_DESTROY_CRUMBLING_BLEND.color.dst_factor,
            wgpu::BlendFactor::Src
        );
        assert_eq!(
            BLOCK_DESTROY_CRUMBLING_BLEND.alpha.src_factor,
            wgpu::BlendFactor::One
        );
        assert_eq!(
            BLOCK_DESTROY_CRUMBLING_BLEND.alpha.dst_factor,
            wgpu::BlendFactor::Zero
        );
        assert_eq!(BLOCK_DESTROY_DEPTH_BIAS.slope_scale, -1.0);
        assert_eq!(BLOCK_DESTROY_DEPTH_BIAS.constant, -10);
        assert_eq!(BLOCK_DESTROY_DEPTH_BIAS.clamp, 0.0);
    }

    #[test]
    fn block_destroy_shader_uses_vanilla_crumbling_alpha_cutout() {
        // Vanilla `core/rendertype_crumbling.fsh` discards fragments below 0.1
        // and leaves the sampled color alpha intact for the crumbling blend.
        assert!(BLOCK_DESTROY_SHADER.contains("if color.a < 0.1"));
        assert!(BLOCK_DESTROY_SHADER.contains("fract(input.decal_uv)"));
        assert!(BLOCK_DESTROY_SHADER.contains("return apply_fog(color"));
        assert!(!BLOCK_DESTROY_SHADER.contains("0.85"));
        assert!(!BLOCK_DESTROY_SHADER.contains("<= 0.01"));
    }

    #[test]
    fn block_destroy_overlay_mesh_emits_offset_cube_faces() {
        let mesh = block_destroy_overlay_mesh(cube_overlay(
            [2, 3, -4],
            TerrainUvRect {
                min: [0.25, 0.5],
                max: [0.5, 0.75],
            },
        ));

        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
        assert_eq!(mesh.vertices[0].position, [2.0, 2.997, -3.0]);
        assert_eq!(mesh.vertices[0].normal, [0.0, -1.0, 0.0]);
        // Keep Vanilla's raw repeating decal coordinates until the fragment shader; the atlas
        // rectangle rides each vertex separately.
        assert_close(mesh.vertices[0].decal_uv, [0.0, -1.0]);
        assert_close(mesh.vertices[1].decal_uv, [1.0, -1.0]);
        assert_close(mesh.vertices[2].decal_uv, [1.0, 0.0]);
        assert_close(mesh.vertices[3].decal_uv, [0.0, 0.0]);
        assert_eq!(mesh.vertices[0].uv_rect_min, [0.25, 0.5]);
        assert_eq!(mesh.vertices[0].uv_rect_size, [0.25, 0.25]);
        // Reuses the chunk mesher's `[0,1,2,0,2,3]` winding (not the old reversed table).
        assert_eq!(mesh.indices[0..6], [0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn block_destroy_overlay_mesh_emits_all_six_faces() {
        let mesh = block_destroy_overlay_mesh(cube_overlay([0, 0, 0], TerrainUvRect::UNIT));

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

    #[test]
    fn block_destroy_overlay_mesh_matches_terrain_block_face_winding() {
        // Vanilla renders the crumbling decal over the block model's own quads, so the crack must be
        // visible from exactly the sides the block faces are. bbb's chunk mesher builds faces with
        // `box_face_corners` + `[0,1,2,0,2,3]`, whose right-hand-rule normal points *into* the block
        // (opposite the outward face normal); the fluid back-face note in `terrain::mesh::emitter`
        // confirms this is the outside-visible winding under the terrain pipeline's back-face cull.
        let mesh = block_destroy_overlay_mesh(cube_overlay([0, 0, 0], TerrainUvRect::UNIT));

        for face_index in 0..FACES.len() {
            let triangle = face_index * 6;
            let a = mesh.vertices[mesh.indices[triangle] as usize].position;
            let b = mesh.vertices[mesh.indices[triangle + 1] as usize].position;
            let c = mesh.vertices[mesh.indices[triangle + 2] as usize].position;
            let normal = triangle_normal(a, b, c);
            assert!(
                dot3(normal, FACES[face_index].normal) < 0.0,
                "face {face_index} winding must match the terrain block face (inward RHR normal)"
            );
        }
    }

    #[test]
    fn block_destroy_overlays_mesh_batches_multiple_positions() {
        let overlays = [
            cube_overlay([0, 0, 0], TerrainUvRect::UNIT),
            cube_overlay([2, 0, 0], TerrainUvRect::UNIT),
        ];

        let mesh = block_destroy_overlays_mesh(&overlays);

        assert_eq!(mesh.vertices.len(), 48);
        assert_eq!(mesh.indices.len(), 72);
        assert_eq!(mesh.vertices[0].position, [0.0, -0.003, 1.0]);
        assert_eq!(mesh.vertices[24].position, [2.0, -0.003, 1.0]);
        assert_eq!(mesh.indices[36..42], [24, 25, 26, 24, 26, 27]);
    }

    #[test]
    fn block_destroy_overlay_mesh_follows_bottom_slab_half_height() {
        // A bottom slab is a box y in `0..8/16`. The crack tracks that geometry: its side faces span
        // world y `0..0.5` and, via the decal projection `[px, 1-py]`, sample only the lower half of
        // the crumbling sprite (`v in 0.5..1`) instead of stretching the full texture over a cube.
        let mesh = block_destroy_overlay_mesh(BlockDestroyOverlay {
            pos: [0, 0, 0],
            uv: TerrainUvRect::UNIT,
            shape: TerrainRenderShape::Box {
                from: [0, 0, 0],
                to: [16, 8, 16],
                face_present: [true; 6],
                face_uvs: [[0, 0, 16, 16]; 6],
                face_uv_rotations: [0; 6],
                face_shade: [true; 6],
                face_light_emission: [0; 6],
                face_cull: [None; 6],
                face_transparency: [TerrainTransparency::OPAQUE; 6],
            },
        });

        assert_eq!(mesh.vertices.len(), 24);
        // Up face (FACES index 1) sits at the slab top y=0.5 (nudged +0.003 outward).
        assert_eq!(mesh.vertices[4].position[1], 0.503);
        // South face (FACES index 3) is verts 12..16: world y in `{0, 0.5}`, raw decal v in
        // `{0, -0.5}`. Fragment-time wrapping samples the corresponding full/lower-half positions.
        assert_eq!(mesh.vertices[12].position[1], 0.0);
        assert_close(mesh.vertices[12].decal_uv, [0.0, 0.0]);
        assert_eq!(mesh.vertices[13].position[1], 0.5);
        assert_close(mesh.vertices[13].decal_uv, [0.0, -0.5]);
        assert!(mesh
            .vertices
            .chunks_exact(4)
            .nth(3)
            .unwrap()
            .iter()
            .all(|vertex| vertex.decal_uv[1] >= -0.500_01 && vertex.decal_uv[1] <= 0.000_01));
    }

    #[test]
    fn block_destroy_overlay_mesh_follows_multi_box_stairs() {
        // Stairs are two boxes; the crack emits both, so the face (and vertex) count is the sum.
        let mesh = block_destroy_overlay_mesh(BlockDestroyOverlay {
            pos: [0, 0, 0],
            uv: TerrainUvRect::UNIT,
            shape: TerrainRenderShape::Boxes(vec![
                test_box([0, 0, 0], [16, 8, 16]),
                test_box([0, 8, 0], [8, 16, 16]),
            ]),
        });

        // Two all-faces boxes => 12 faces => 48 vertices / 72 indices.
        assert_eq!(mesh.vertices.len(), 48);
        assert_eq!(mesh.indices.len(), 72);
        // The upper step's Up face reaches the full-block top y=1.0 (+0.003 outward).
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| (vertex.position[1] - 1.003).abs() < 1e-4));
    }

    #[test]
    fn block_destroy_overlay_mesh_follows_cross_planes() {
        // A cross (foliage) block cracks over its two diagonal planes: four one-sided CROSS_FACES
        // quads (16 vertices), each spanning the full sprite.
        let mesh = block_destroy_overlay_mesh(BlockDestroyOverlay {
            pos: [0, 0, 0],
            uv: TerrainUvRect::UNIT,
            shape: TerrainRenderShape::Cross {
                shade: true,
                light_emission: 0,
            },
        });

        assert_eq!(mesh.vertices.len(), 16);
        assert_eq!(mesh.indices.len(), 24);
        let normals: Vec<_> = mesh
            .vertices
            .chunks_exact(4)
            .map(|face| face[0].normal)
            .collect();
        assert_eq!(
            normals,
            CROSS_FACES
                .iter()
                .map(|(_, normal, _)| *normal)
                .collect::<Vec<_>>()
        );
        // The raw coordinates span one complete repeating tile on each plane.
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| close(vertex.decal_uv[0], 0.0) && close(vertex.decal_uv[1], 0.0)));
        assert!(mesh.vertices.iter().any(|vertex| {
            close(vertex.decal_uv[0].abs(), 1.0) && close(vertex.decal_uv[1].abs(), 1.0)
        }));
    }

    #[test]
    fn block_destroy_decal_uv_projects_block_local_position() {
        // Hand-checked SheetedDecalTextureGenerator projections (`textureScale = 1`).
        assert_close(
            destroy_decal_uv(TerrainFace::Down, [0.2, 0.0, 0.7]),
            [0.2, -0.7],
        );
        assert_close(
            destroy_decal_uv(TerrainFace::Up, [0.2, 1.0, 0.7]),
            [0.2, 0.7],
        );
        assert_close(
            destroy_decal_uv(TerrainFace::North, [0.25, 0.75, 0.0]),
            [-0.25, -0.75],
        );
        assert_close(
            destroy_decal_uv(TerrainFace::East, [1.0, 0.5, 0.3]),
            [0.3, -0.5],
        );

        // And the same raw projection reaches a real mesh vertex carrying a non-unit sprite rect.
        // West face first corner of box `4..12 / 0..16 / 2..14` is block-local `[0.25, 0, 0.125]`
        // => decal `[-0.125, 0]`; the shader wraps it within `[0.1,0.2]..[0.5,0.9]`.
        let mesh = block_destroy_overlay_mesh(BlockDestroyOverlay {
            pos: [0, 0, 0],
            uv: TerrainUvRect {
                min: [0.1, 0.2],
                max: [0.5, 0.9],
            },
            shape: TerrainRenderShape::Box {
                from: [4, 0, 2],
                to: [12, 16, 14],
                face_present: [true; 6],
                face_uvs: [[0, 0, 16, 16]; 6],
                face_uv_rotations: [0; 6],
                face_shade: [true; 6],
                face_light_emission: [0; 6],
                face_cull: [None; 6],
                face_transparency: [TerrainTransparency::OPAQUE; 6],
            },
        });
        // West is FACES index 4 => verts 16..20.
        assert_eq!(mesh.vertices[16].position, [0.247, 0.0, 0.125]);
        assert_close(mesh.vertices[16].decal_uv, [-0.125, 0.0]);
        assert_eq!(mesh.vertices[16].uv_rect_min, [0.1, 0.2]);
        assert_close(mesh.vertices[16].uv_rect_size, [0.4, 0.7]);
    }

    #[test]
    fn block_destroy_overlay_mesh_emits_empty_quad_model_as_empty() {
        let mesh = block_destroy_overlay_mesh(BlockDestroyOverlay {
            pos: [0, 0, 0],
            uv: TerrainUvRect::UNIT,
            shape: TerrainRenderShape::Quads(Vec::new()),
        });

        assert!(mesh.vertices.is_empty());
        assert!(mesh.indices.is_empty());
    }

    #[test]
    fn block_destroy_overlay_mesh_preserves_arbitrary_baked_quad_geometry() {
        let corners = [
            [2.0, 1.0, 3.0],
            [14.0, 4.0, 2.0],
            [13.0, 15.0, 12.0],
            [1.0, 11.0, 14.0],
        ];
        let normal = [0.2, 0.9, 0.3];
        let mesh = block_destroy_overlay_mesh(BlockDestroyOverlay {
            pos: [4, -2, 7],
            uv: TerrainUvRect {
                min: [0.25, 0.5],
                max: [0.375, 0.75],
            },
            shape: TerrainRenderShape::Quads(vec![TerrainQuad {
                corners,
                normal,
                uvs: [[0.0; 2]; 4],
                cull: None,
                texture_index: 17,
                tint: TerrainTint::WHITE,
                transparency: TerrainTransparency::OPAQUE,
                shade: true,
                light_emission: 0,
            }]),
        });

        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices, [0, 1, 2, 0, 2, 3]);
        assert_close3(
            mesh.vertices[0].position,
            [
                4.0 + 2.0 / 16.0 + normal[0] * BLOCK_DESTROY_FACE_OFFSET,
                -2.0 + 1.0 / 16.0 + normal[1] * BLOCK_DESTROY_FACE_OFFSET,
                7.0 + 3.0 / 16.0 + normal[2] * BLOCK_DESTROY_FACE_OFFSET,
            ],
        );
        assert_eq!(mesh.vertices[0].normal, normal);
        // Up is the nearest direction; its exact sheeted projection is `[x, z]`.
        assert_close(mesh.vertices[0].decal_uv, [2.0 / 16.0, 3.0 / 16.0]);
        assert_close(mesh.vertices[3].decal_uv, [1.0 / 16.0, 14.0 / 16.0]);
        assert_eq!(mesh.vertices[0].uv_rect_min, [0.25, 0.5]);
        assert_eq!(mesh.vertices[0].uv_rect_size, [0.125, 0.25]);
    }

    fn cube_overlay(pos: [i32; 3], uv: TerrainUvRect) -> BlockDestroyOverlay {
        BlockDestroyOverlay {
            pos,
            uv,
            shape: TerrainRenderShape::Cube,
        }
    }

    fn test_box(from: [u8; 3], to: [u8; 3]) -> crate::terrain::TerrainBox {
        crate::terrain::TerrainBox {
            from,
            to,
            face_present: [true; 6],
            face_uvs: [[0, 0, 16, 16]; 6],
            face_uv_rotations: [0; 6],
            face_shade: [true; 6],
            face_light_emission: [0; 6],
            face_cull: [None; 6],
            texture_indices: [0; 6],
            tint: [TerrainTint::WHITE; 6],
            face_transparency: [TerrainTransparency::OPAQUE; 6],
        }
    }

    fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ]
    }

    fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    fn assert_close(actual: [f32; 2], expected: [f32; 2]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(close(actual, expected), "{actual} != {expected}");
        }
    }

    fn assert_close3(actual: [f32; 3], expected: [f32; 3]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(close(actual, expected), "{actual} != {expected}");
        }
    }

    fn close(actual: f32, expected: f32) -> bool {
        (actual - expected).abs() < 1.0e-5
    }
}
