use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;

use crate::{
    gpu::DEPTH_FORMAT,
    pipeline_builder::{depth_stencil_state, RenderPipelineBuilder},
    terrain::{
        mesh::geometry::{box_face_corners, CROSS_FACES, FACES},
        TerrainFace, TerrainRenderShape, TerrainTint, TerrainUvRect, TerrainVertex,
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

const BLOCK_DESTROY_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x2, 4 => Float32x3, 5 => Float32, 6 => Float32, 7 => Sint32];
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
    @location(1) spherical_distance: f32,
    @location(2) cylindrical_distance: f32,
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
    out.uv = input.uv;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(terrain_atlas, terrain_sampler, input.uv);
    if color.a < 0.1 {
        discard;
    }
    return apply_fog(color, input.spherical_distance, input.cylindrical_distance);
}
"#;

pub(super) struct BlockDestroyOverlaysGpu {
    pub(super) overlays: Vec<BlockDestroyOverlay>,
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) index_buffer: wgpu::Buffer,
    pub(super) index_count: u32,
}

pub(super) fn create_block_destroy_overlays_gpu(
    device: &wgpu::Device,
    overlays: Vec<BlockDestroyOverlay>,
) -> BlockDestroyOverlaysGpu {
    let mesh = block_destroy_overlays_mesh(&overlays);
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
) -> wgpu::RenderPipeline {
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
        .build()
}

struct BlockDestroyOverlayMesh {
    vertices: Vec<TerrainVertex>,
    indices: Vec<u32>,
}

fn block_destroy_overlays_mesh(overlays: &[BlockDestroyOverlay]) -> BlockDestroyOverlayMesh {
    let mut vertices = Vec::with_capacity(overlays.len() * 24);
    let mut indices = Vec::with_capacity(overlays.len() * 36);
    for overlay in overlays {
        emit_block_destroy_overlay(&mut vertices, &mut indices, overlay);
    }
    BlockDestroyOverlayMesh { vertices, indices }
}

#[cfg(test)]
fn block_destroy_overlay_mesh(overlay: BlockDestroyOverlay) -> BlockDestroyOverlayMesh {
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    emit_block_destroy_overlay(&mut vertices, &mut indices, &overlay);
    BlockDestroyOverlayMesh { vertices, indices }
}

fn emit_block_destroy_overlay(
    vertices: &mut Vec<TerrainVertex>,
    indices: &mut Vec<u32>,
    overlay: &BlockDestroyOverlay,
) {
    let pos = overlay.pos;
    let uv = overlay.uv;
    match &overlay.shape {
        // A `Quads` shape has no single crumbling-friendly box, so it degrades to the unit cube
        // crack (recorded in the Terrain Block Presentation Parity ledger entry).
        TerrainRenderShape::Cube | TerrainRenderShape::Quads(_) => {
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
    vertices: &mut Vec<TerrainVertex>,
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
            vertices.push(destroy_vertex(pos, corner, face.normal, uv_rect, face.face));
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

/// Emits the crumbling faces of a cross (foliage) block: the mesher's two `CROSS_FACES` planes, one
/// one-sided quad per direction so the decal shows on both sides of each plane. The planes span the
/// full `[0, 1]` cell, so the decal covers the whole crumbling sprite.
fn emit_block_destroy_cross(
    vertices: &mut Vec<TerrainVertex>,
    indices: &mut Vec<u32>,
    pos: [i32; 3],
    uv_rect: TerrainUvRect,
) {
    const CROSS_UVS: [[f32; 2]; 4] = [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
    for (_face, normal, corners) in CROSS_FACES {
        let base = vertices.len() as u32;
        for (corner, uv) in corners.into_iter().zip(CROSS_UVS) {
            let position = [
                pos[0] as f32 + corner[0] + normal[0] * BLOCK_DESTROY_FACE_OFFSET,
                pos[1] as f32 + corner[1] + normal[1] * BLOCK_DESTROY_FACE_OFFSET,
                pos[2] as f32 + corner[2] + normal[2] * BLOCK_DESTROY_FACE_OFFSET,
            ];
            vertices.push(TerrainVertex {
                position,
                normal,
                uv: uv_rect.map(uv),
                light: [1.0, 1.0],
                tint: TerrainTint::WHITE.as_shader_tint(),
                shade: 1.0,
                ambient_occlusion: 1.0,
                block_state_id: -1,
            });
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
    face: TerrainFace,
) -> TerrainVertex {
    TerrainVertex {
        position: [
            pos[0] as f32 + corner[0] + normal[0] * BLOCK_DESTROY_FACE_OFFSET,
            pos[1] as f32 + corner[1] + normal[1] * BLOCK_DESTROY_FACE_OFFSET,
            pos[2] as f32 + corner[2] + normal[2] * BLOCK_DESTROY_FACE_OFFSET,
        ],
        normal,
        uv: uv_rect.map(destroy_decal_uv(face, corner)),
        light: [1.0, 1.0],
        tint: TerrainTint::WHITE.as_shader_tint(),
        shade: 1.0,
        ambient_occlusion: 1.0,
        block_state_id: -1,
    }
}

/// Vanilla `SheetedDecalTextureGenerator.setNormal` projects the crumbling UV from each vertex's
/// block-local position onto the plane perpendicular to the face's nearest `Direction` (the
/// crumbling buffer runs at `textureScale = 1.0`). For an axis-aligned face at local position
/// `p ∈ [0, 1]^3` that reduces to the two perpendicular components; vanilla's crumbling texture
/// tiles, so we fold the negated components (`-p`) back into `[0, 1]` as `1 - p` to index the atlas
/// sprite once per block. Partial boxes therefore sample only the covered slice of the sprite (e.g.
/// a bottom slab's sides show the lower half), matching the projected decal.
fn destroy_decal_uv(face: TerrainFace, local: [f32; 3]) -> [f32; 2] {
    let [px, py, pz] = local;
    match face {
        TerrainFace::Down => [px, 1.0 - pz],
        TerrainFace::Up => [px, pz],
        TerrainFace::North => [1.0 - px, 1.0 - py],
        TerrainFace::South => [px, 1.0 - py],
        TerrainFace::West => [1.0 - pz, 1.0 - py],
        TerrainFace::East => [pz, 1.0 - py],
    }
}

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
    use crate::terrain::TerrainTransparency;

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
        // Down face decal UV = SheetedDecalTextureGenerator projection of the block-local corners
        // `[0,0,1] [1,0,1] [1,0,0] [0,0,0]` → `[px, 1-pz]`, mapped into the destroy-stage sprite rect.
        assert_eq!(mesh.vertices[0].uv, [0.25, 0.5]);
        assert_eq!(mesh.vertices[1].uv, [0.5, 0.5]);
        assert_eq!(mesh.vertices[2].uv, [0.5, 0.75]);
        assert_eq!(mesh.vertices[3].uv, [0.25, 0.75]);
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
        // South face (FACES index 3) is verts 12..16: world y in `{0, 0.5}`, decal v in `{1.0, 0.5}`.
        assert_eq!(mesh.vertices[12].position[1], 0.0);
        assert_eq!(mesh.vertices[12].uv, [0.0, 1.0]);
        assert_eq!(mesh.vertices[13].position[1], 0.5);
        assert_eq!(mesh.vertices[13].uv, [0.0, 0.5]);
        assert!(mesh
            .vertices
            .chunks_exact(4)
            .nth(3)
            .unwrap()
            .iter()
            .all(|vertex| vertex.uv[1] >= 0.5));
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
        // The decal covers the whole crumbling sprite on each plane.
        assert!(mesh.vertices.iter().any(|vertex| vertex.uv == [0.0, 0.0]));
        assert!(mesh.vertices.iter().any(|vertex| vertex.uv == [1.0, 1.0]));
    }

    #[test]
    fn block_destroy_decal_uv_projects_block_local_position() {
        // Hand-checked SheetedDecalTextureGenerator projections (`textureScale = 1`).
        assert_eq!(
            destroy_decal_uv(TerrainFace::Down, [0.2, 0.0, 0.7]),
            [0.2, 0.3]
        );
        assert_eq!(
            destroy_decal_uv(TerrainFace::Up, [0.2, 1.0, 0.7]),
            [0.2, 0.7]
        );
        assert_eq!(
            destroy_decal_uv(TerrainFace::North, [0.25, 0.75, 0.0]),
            [0.75, 0.25]
        );
        assert_eq!(
            destroy_decal_uv(TerrainFace::East, [1.0, 0.5, 0.3]),
            [0.3, 0.5]
        );

        // And the same projection reaches a real mesh vertex, mapped into a non-unit sprite rect.
        // West face first corner of box `4..12 / 0..16 / 2..14` is block-local `[0.25, 0, 0.125]`
        // => decal `[1-0.125, 1-0] = [0.875, 1.0]` => rect `[0.1,0.2]..[0.5,0.9]` => `[0.45, 0.9]`.
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
        assert_eq!(mesh.vertices[16].uv, [0.45, 0.9]);
    }

    #[test]
    fn block_destroy_overlay_mesh_degrades_quads_to_cube() {
        // `Quads` has no crumbling-friendly box, so it degrades to the unit cube crack.
        let quads = block_destroy_overlay_mesh(BlockDestroyOverlay {
            pos: [0, 0, 0],
            uv: TerrainUvRect::UNIT,
            shape: TerrainRenderShape::Quads(Vec::new()),
        });
        let cube = block_destroy_overlay_mesh(cube_overlay([0, 0, 0], TerrainUvRect::UNIT));

        assert_eq!(quads.vertices.len(), 24);
        assert_eq!(quads.indices.len(), 36);
        assert_eq!(quads.vertices, cube.vertices);
        assert_eq!(quads.indices, cube.indices);
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
}
