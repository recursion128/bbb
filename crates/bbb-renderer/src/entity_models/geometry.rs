use glam::{EulerRot, Mat4, Vec3};

use super::{EntityModelTextureRef, EntityModelUvRect};

const MODEL_UNIT_SCALE: f32 = 1.0 / 16.0;

/// Per-vertex lightmap input `[block, sky]` (each `0.0..=1.0`) written by the
/// cube emitters and overwritten per entity by the scene light fill. Defaults to
/// fully lit so any vertex the fill misses renders bright rather than black.
pub(super) const ENTITY_VERTEX_FULL_BRIGHT_LIGHT: [f32; 2] = [1.0, 1.0];

/// Per-vertex overlay coords `[u, v]` (vanilla `OverlayTexture.NO_OVERLAY` =
/// `pack(0, 10)`): no white flash, no red row. The scene overlay fill overwrites
/// it per entity when the entity is hurt.
pub(super) const ENTITY_VERTEX_NO_OVERLAY: [f32; 2] = [0.0, 10.0];

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct EntityModelVertex {
    pub(super) position: [f32; 3],
    pub(super) color: [f32; 4],
    pub(super) light: [f32; 2],
    pub(super) overlay: [f32; 2],
    pub(super) normal: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct EntityModelTexturedVertex {
    pub(super) position: [f32; 3],
    pub(super) uv: [f32; 2],
    pub(super) tint: [f32; 4],
    pub(super) light: [f32; 2],
    pub(super) overlay: [f32; 2],
    pub(super) normal: [f32; 3],
}

pub(super) struct EntityModelMesh {
    pub(super) vertices: Vec<EntityModelVertex>,
    pub(super) indices: Vec<u32>,
    pub(super) opaque_faces: usize,
}

impl EntityModelMesh {
    pub(super) fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            opaque_faces: 0,
        }
    }
}

pub(super) struct EntityModelTexturedMesh {
    pub(super) vertices: Vec<EntityModelTexturedVertex>,
    pub(super) indices: Vec<u32>,
    pub(super) cutout_faces: usize,
}

impl EntityModelTexturedMesh {
    pub(super) fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            cutout_faces: 0,
        }
    }
}

/// A vertex of the end portal/gateway cube mesh. Vanilla `RenderTypes.endPortal()` /
/// `endGateway()` are position-only custom geometry, but this backend samples the two vanilla
/// textures from the shared entity atlas, so each vertex carries the atlas sub-rects for
/// `end_sky.png` and `end_portal.png`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct EntityModelPortalVertex {
    pub(super) position: [f32; 3],
    pub(super) sky_rect_min: [f32; 2],
    pub(super) sky_rect_size: [f32; 2],
    pub(super) portal_rect_min: [f32; 2],
    pub(super) portal_rect_size: [f32; 2],
}

pub(super) struct EntityModelPortalMesh {
    pub(super) vertices: Vec<EntityModelPortalVertex>,
    pub(super) indices: Vec<u32>,
}

impl EntityModelPortalMesh {
    pub(super) fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
}

/// A vertex of the scrolling-overlay mesh (vanilla `breezeWind` / `energySwirl` render types): a
/// texture-matrix `OffsetTextureTransform` over a `GL_REPEAT` texture. Because our textures live in a
/// shared atlas (no per-texture `REPEAT`), the scroll is reproduced in the shader: the atlas UV is
/// inverted back to a local `0..1` UV, the per-instance offset is added, and the shader `fract`s it
/// and maps it back into `[uv_rect_min, uv_rect_min + uv_rect_size]` — the per-fragment `fract`
/// recreating the `REPEAT` seam.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct EntityModelScrollVertex {
    pub(super) position: [f32; 3],
    /// Local UV within the texture (`0..1`) with the per-instance scroll offset already added.
    pub(super) local_uv: [f32; 2],
    /// The texture's atlas sub-rect origin / size, so the shader can wrap the scrolled local UV back
    /// into the atlas without bleeding into neighbouring textures.
    pub(super) uv_rect_min: [f32; 2],
    pub(super) uv_rect_size: [f32; 2],
    pub(super) tint: [f32; 4],
    pub(super) light: [f32; 2],
    pub(super) overlay: [f32; 2],
}

pub(super) struct EntityModelScrollMesh {
    pub(super) vertices: Vec<EntityModelScrollVertex>,
    pub(super) indices: Vec<u32>,
}

impl EntityModelScrollMesh {
    pub(super) fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
}

/// A vertex of the GPU dissolve mesh (vanilla `RenderTypes.entityCutoutDissolve` — the dying ender
/// dragon body). It carries the ordinary textured vertex attributes plus a second `mask_uv` set that
/// samples the dissolve mask (`dragon_exploding.png`) at the *same* normalized model UV as the base
/// texture, matching vanilla `entity.fsh`: `texture(DissolveMaskSampler, texCoord0)`. Because both the
/// base texture and the mask live in the shared entity atlas, `mask_uv` is baked to the mask's atlas
/// sub-rect at mesh-build time (see [`append_dissolve_textured_mesh`]).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct EntityModelDissolveVertex {
    pub(super) position: [f32; 3],
    pub(super) uv: [f32; 2],
    pub(super) mask_uv: [f32; 2],
    pub(super) tint: [f32; 4],
    pub(super) light: [f32; 2],
    pub(super) overlay: [f32; 2],
    pub(super) normal: [f32; 3],
}

pub(super) struct EntityModelDissolveMesh {
    pub(super) vertices: Vec<EntityModelDissolveVertex>,
    pub(super) indices: Vec<u32>,
    pub(super) cutout_faces: usize,
}

impl EntityModelDissolveMesh {
    pub(super) fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            cutout_faces: 0,
        }
    }
}

/// Appends a normal textured render (`textured`, carrying atlas-absolute base UVs) to the dissolve
/// mesh, deriving each vertex's `mask_uv` from its base `uv`: the base atlas UV is inverted to a
/// normalized `0..1` model UV within `base_rect`, then re-projected into the dissolve mask's atlas
/// sub-rect `mask_rect`. This reproduces vanilla `entity.fsh` sampling `DissolveMaskSampler` at the
/// same `texCoord0` the base texture uses (`dragon.png` and `dragon_exploding.png` share the model's
/// UV layout). Indices are re-based onto `dissolve`'s current vertex count.
pub(super) fn append_dissolve_textured_mesh(
    dissolve: &mut EntityModelDissolveMesh,
    textured: &EntityModelTexturedMesh,
    base_rect: EntityModelUvRect,
    mask_rect: EntityModelUvRect,
) {
    let base = u32::try_from(dissolve.vertices.len()).expect("dissolve vertex count fits in u32");
    let base_size = [
        base_rect.max[0] - base_rect.min[0],
        base_rect.max[1] - base_rect.min[1],
    ];
    let mask_size = [
        mask_rect.max[0] - mask_rect.min[0],
        mask_rect.max[1] - mask_rect.min[1],
    ];
    for vertex in &textured.vertices {
        let normalized_u = if base_size[0] != 0.0 {
            (vertex.uv[0] - base_rect.min[0]) / base_size[0]
        } else {
            0.0
        };
        let normalized_v = if base_size[1] != 0.0 {
            (vertex.uv[1] - base_rect.min[1]) / base_size[1]
        } else {
            0.0
        };
        dissolve.vertices.push(EntityModelDissolveVertex {
            position: vertex.position,
            uv: vertex.uv,
            mask_uv: [
                mask_rect.min[0] + normalized_u * mask_size[0],
                mask_rect.min[1] + normalized_v * mask_size[1],
            ],
            tint: vertex.tint,
            light: vertex.light,
            overlay: vertex.overlay,
            normal: vertex.normal,
        });
    }
    dissolve
        .indices
        .extend(textured.indices.iter().map(|index| index + base));
    dissolve.cutout_faces += textured.cutout_faces;
}

/// Appends a normal textured render (`textured`, carrying atlas-absolute UVs) to the scrolling-overlay
/// mesh, converting each vertex: the atlas UV is inverted back to a local `0..1` UV within `rect`, the
/// per-instance `offset` is added, and `rect` is carried so the shader `fract`-wraps the scrolled local
/// UV back into the atlas sub-rect (reproducing the vanilla texture-matrix scroll over a `GL_REPEAT`
/// texture). Indices are re-based onto `scroll`'s current vertex count.
pub(super) fn append_scrolled_textured_mesh(
    scroll: &mut EntityModelScrollMesh,
    textured: &EntityModelTexturedMesh,
    rect: EntityModelUvRect,
    offset: [f32; 2],
) {
    let base = u32::try_from(scroll.vertices.len()).expect("scroll vertex count fits in u32");
    let size = [rect.max[0] - rect.min[0], rect.max[1] - rect.min[1]];
    for vertex in &textured.vertices {
        let local_u = if size[0] != 0.0 {
            (vertex.uv[0] - rect.min[0]) / size[0]
        } else {
            0.0
        };
        let local_v = if size[1] != 0.0 {
            (vertex.uv[1] - rect.min[1]) / size[1]
        } else {
            0.0
        };
        scroll.vertices.push(EntityModelScrollVertex {
            position: vertex.position,
            local_uv: [local_u + offset[0], local_v + offset[1]],
            uv_rect_min: rect.min,
            uv_rect_size: size,
            tint: vertex.tint,
            light: vertex.light,
            overlay: vertex.overlay,
        });
    }
    scroll
        .indices
        .extend(textured.indices.iter().map(|index| index + base));
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ModelPartDesc {
    pub(super) pose: PartPose,
    pub(super) cubes: &'static [ModelCubeDesc],
    pub(super) children: &'static [ModelPartDesc],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ModelCubeDesc {
    pub(super) min: [f32; 3],
    pub(super) size: [f32; 3],
    pub(super) color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TexturedModelPartDesc {
    pub(super) pose: PartPose,
    pub(super) cubes: &'static [TexturedModelCubeDesc],
    pub(super) children: &'static [TexturedModelPartDesc],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TexturedModelCubeDesc {
    pub(super) min: [f32; 3],
    pub(super) size: [f32; 3],
    pub(super) uv_size: [f32; 3],
    pub(super) tex: [f32; 2],
    pub(super) mirror: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PartPose {
    pub(super) offset: [f32; 3],
    pub(super) rotation: [f32; 3],
}

pub(super) const PART_POSE_ZERO: PartPose = PartPose {
    offset: [0.0, 0.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

// Per-cube face visibility bits, mirroring the vanilla `Set<Direction> visibleFaces` that
// `CubeListBuilder.addBox(..., visibleFaces)` threads into `ModelPart.Cube` (a hidden face's
// polygon is simply not built). Bit order follows `Direction.get3DDataValue()`.
pub(super) const MODEL_CUBE_FACE_DOWN: u8 = 1 << 0;
pub(super) const MODEL_CUBE_FACE_UP: u8 = 1 << 1;
pub(super) const MODEL_CUBE_FACE_NORTH: u8 = 1 << 2;
pub(super) const MODEL_CUBE_FACE_SOUTH: u8 = 1 << 3;
pub(super) const MODEL_CUBE_FACE_WEST: u8 = 1 << 4;
pub(super) const MODEL_CUBE_FACE_EAST: u8 = 1 << 5;
/// Vanilla `CubeListBuilder.addBox` default: all six faces visible (`Cube.ALL_VISIBLE`).
pub(super) const MODEL_CUBE_FACES_ALL: u8 = MODEL_CUBE_FACE_DOWN
    | MODEL_CUBE_FACE_UP
    | MODEL_CUBE_FACE_NORTH
    | MODEL_CUBE_FACE_SOUTH
    | MODEL_CUBE_FACE_WEST
    | MODEL_CUBE_FACE_EAST;

/// Overwrites the lightmap input on every colored vertex appended since
/// `start`, applying one entity's sampled `[block, sky]` light to all of its
/// emitted geometry. Mirrors vanilla sampling a single light-probe position per
/// entity and baking that into the model's vertex light coords.
pub(super) fn fill_entity_model_light(mesh: &mut EntityModelMesh, start: usize, light: [f32; 2]) {
    for vertex in &mut mesh.vertices[start..] {
        vertex.light = light;
    }
}

/// Textured/eyes/translucent counterpart of [`fill_entity_model_light`].
pub(super) fn fill_entity_textured_light(
    mesh: &mut EntityModelTexturedMesh,
    start: usize,
    light: [f32; 2],
) {
    for vertex in &mut mesh.vertices[start..] {
        vertex.light = light;
    }
}

/// Overwrites the overlay coords on every colored vertex appended since `start`,
/// applying one entity's `OverlayTexture` coords (`[u, v]`) to all of its
/// emitted geometry. Mirrors vanilla using a single per-entity overlay value.
pub(super) fn fill_entity_model_overlay(
    mesh: &mut EntityModelMesh,
    start: usize,
    overlay: [f32; 2],
) {
    for vertex in &mut mesh.vertices[start..] {
        vertex.overlay = overlay;
    }
}

/// Multiplies the alpha on every colored vertex appended since `start`.
pub(super) fn multiply_entity_model_alpha(mesh: &mut EntityModelMesh, start: usize, alpha: f32) {
    for vertex in &mut mesh.vertices[start..] {
        vertex.color[3] *= alpha;
    }
}

/// Overwrites the color on every colored vertex appended since `start`.
pub(super) fn fill_entity_model_color(mesh: &mut EntityModelMesh, start: usize, color: [f32; 4]) {
    for vertex in &mut mesh.vertices[start..] {
        vertex.color = color;
    }
}

pub(super) fn argb_to_tint(argb: u32) -> [f32; 4] {
    [
        ((argb >> 16) & 0xFF) as f32 / 255.0,
        ((argb >> 8) & 0xFF) as f32 / 255.0,
        (argb & 0xFF) as f32 / 255.0,
        ((argb >> 24) & 0xFF) as f32 / 255.0,
    ]
}

/// Textured/eyes/translucent counterpart of [`fill_entity_model_overlay`].
pub(super) fn fill_entity_textured_overlay(
    mesh: &mut EntityModelTexturedMesh,
    start: usize,
    overlay: [f32; 2],
) {
    for vertex in &mut mesh.vertices[start..] {
        vertex.overlay = overlay;
    }
}

pub(super) fn emit_model_parts(
    mesh: &mut EntityModelMesh,
    parts: &[ModelPartDesc],
    parent_transform: Mat4,
) {
    for part in parts {
        emit_model_part(mesh, part, parent_transform);
    }
}

pub(super) fn emit_model_parts_with_color(
    mesh: &mut EntityModelMesh,
    parts: &[ModelPartDesc],
    parent_transform: Mat4,
    color: [f32; 4],
) {
    for part in parts {
        emit_model_part_with_color(mesh, part, parent_transform, color);
    }
}

pub(super) fn emit_model_part(
    mesh: &mut EntityModelMesh,
    part: &ModelPartDesc,
    parent_transform: Mat4,
) {
    let transform = parent_transform * part_pose_transform(part.pose);
    for cube in part.cubes {
        emit_model_cube(mesh, transform, *cube);
    }
    emit_model_parts(mesh, part.children, transform);
}

pub(super) fn emit_model_part_with_color(
    mesh: &mut EntityModelMesh,
    part: &ModelPartDesc,
    parent_transform: Mat4,
    color: [f32; 4],
) {
    let transform = parent_transform * part_pose_transform(part.pose);
    for cube in part.cubes {
        emit_model_cube_with_color(mesh, transform, *cube, color);
    }
    emit_model_parts_with_color(mesh, part.children, transform, color);
}

/// Textured counterpart of [`emit_model_parts`]: walks a [`TexturedModelPartDesc`] tree, emitting each
/// cube via [`emit_textured_model_cube`] (honouring its per-cube `mirror`/`uv_size`/`tex`) under the
/// accumulated pose transform, against one shared `texture`/`uv_rect`/`tint`.
pub(super) fn emit_textured_model_parts(
    mesh: &mut EntityModelTexturedMesh,
    parts: &[TexturedModelPartDesc],
    parent_transform: Mat4,
    texture: EntityModelTextureRef,
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
) {
    for part in parts {
        emit_textured_model_part(mesh, part, parent_transform, texture, uv_rect, tint);
    }
}

pub(super) fn emit_textured_model_part(
    mesh: &mut EntityModelTexturedMesh,
    part: &TexturedModelPartDesc,
    parent_transform: Mat4,
    texture: EntityModelTextureRef,
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
) {
    let transform = parent_transform * part_pose_transform(part.pose);
    for cube in part.cubes {
        emit_textured_model_cube(mesh, transform, *cube, texture, uv_rect, tint);
    }
    emit_textured_model_parts(mesh, part.children, transform, texture, uv_rect, tint);
}

pub(super) fn emit_model_cube_with_color(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    cube: ModelCubeDesc,
    color: [f32; 4],
) {
    emit_model_cube(
        mesh,
        transform,
        ModelCubeDesc {
            min: cube.min,
            size: cube.size,
            color,
        },
    );
}

pub(super) fn part_pose_transform(pose: PartPose) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(pose.offset) * MODEL_UNIT_SCALE)
        * Mat4::from_euler(
            EulerRot::ZYX,
            pose.rotation[2],
            pose.rotation[1],
            pose.rotation[0],
        )
}

pub(super) fn emit_model_cube(mesh: &mut EntityModelMesh, transform: Mat4, cube: ModelCubeDesc) {
    emit_model_cube_with_faces(mesh, transform, cube, MODEL_CUBE_FACES_ALL);
}

/// [`emit_model_cube`] with a vanilla `visibleFaces` mask: only the faces whose
/// `MODEL_CUBE_FACE_*` bit is set are emitted.
pub(super) fn emit_model_cube_with_faces(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    cube: ModelCubeDesc,
    visible_faces: u8,
) {
    let min = Vec3::from_array(cube.min) * MODEL_UNIT_SCALE;
    let max = min + Vec3::from_array(cube.size) * MODEL_UNIT_SCALE;
    emit_model_cube_from_min_max(mesh, transform, min, max, cube.color, visible_faces);
}

pub(super) fn emit_textured_model_cube(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    cube: TexturedModelCubeDesc,
    texture: EntityModelTextureRef,
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
) {
    emit_textured_model_cube_with_faces(
        mesh,
        transform,
        cube,
        texture,
        uv_rect,
        tint,
        MODEL_CUBE_FACES_ALL,
    );
}

/// [`emit_textured_model_cube`] with a vanilla `visibleFaces` mask: only the faces whose
/// `MODEL_CUBE_FACE_*` bit is set are emitted (`ModelPart.Cube` builds no polygon for a
/// hidden face).
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_textured_model_cube_with_faces(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    cube: TexturedModelCubeDesc,
    texture: EntityModelTextureRef,
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
    visible_faces: u8,
) {
    let mut min = Vec3::from_array(cube.min) * MODEL_UNIT_SCALE;
    let mut max = min + Vec3::from_array(cube.size) * MODEL_UNIT_SCALE;
    if cube.mirror {
        std::mem::swap(&mut min.x, &mut max.x);
    }
    emit_textured_model_cube_from_min_max(
        mesh,
        transform,
        min,
        max,
        cube,
        texture,
        uv_rect,
        tint,
        visible_faces,
    );
}

pub(super) fn emit_model_cube_world_units(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    min: [f32; 3],
    size: [f32; 3],
    color: [f32; 4],
) {
    let min = Vec3::from_array(min);
    let max = min + Vec3::from_array(size);
    emit_model_cube_from_min_max(mesh, transform, min, max, color, MODEL_CUBE_FACES_ALL);
}

fn emit_model_cube_from_min_max(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    min: Vec3,
    max: Vec3,
    color: [f32; 4],
    visible_faces: u8,
) {
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ];
    let faces = [
        ([4, 0, 1, 5], [0.0, -1.0, 0.0], MODEL_CUBE_FACE_DOWN),
        ([2, 3, 7, 6], [0.0, 1.0, 0.0], MODEL_CUBE_FACE_UP),
        ([0, 3, 2, 1], [0.0, 0.0, -1.0], MODEL_CUBE_FACE_NORTH),
        ([5, 6, 7, 4], [0.0, 0.0, 1.0], MODEL_CUBE_FACE_SOUTH),
        ([0, 4, 7, 3], [-1.0, 0.0, 0.0], MODEL_CUBE_FACE_WEST),
        ([1, 2, 6, 5], [1.0, 0.0, 0.0], MODEL_CUBE_FACE_EAST),
    ];

    for (face, normal, face_bit) in faces {
        if visible_faces & face_bit == 0 {
            continue;
        }
        emit_model_face(
            mesh,
            face.map(|index| transform.transform_point3(corners[index])),
            color,
            transform_model_normal(transform, normal),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_textured_model_cube_from_min_max(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    min: Vec3,
    max: Vec3,
    cube: TexturedModelCubeDesc,
    texture: EntityModelTextureRef,
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
    visible_faces: u8,
) {
    let t0 = Vec3::new(min.x, min.y, min.z);
    let t1 = Vec3::new(max.x, min.y, min.z);
    let t2 = Vec3::new(max.x, max.y, min.z);
    let t3 = Vec3::new(min.x, max.y, min.z);
    let l0 = Vec3::new(min.x, min.y, max.z);
    let l1 = Vec3::new(max.x, min.y, max.z);
    let l2 = Vec3::new(max.x, max.y, max.z);
    let l3 = Vec3::new(min.x, max.y, max.z);

    let width = cube.uv_size[0];
    let height = cube.uv_size[1];
    let depth = cube.uv_size[2];
    let x_tex = cube.tex[0];
    let y_tex = cube.tex[1];
    let u0 = x_tex;
    let u1 = x_tex + depth;
    let u2 = x_tex + depth + width;
    let u22 = x_tex + depth + width + width;
    let u3 = x_tex + depth + width + depth;
    let u4 = x_tex + depth + width + depth + width;
    let v0 = y_tex;
    let v1 = y_tex + depth;
    let v2 = y_tex + depth + height;

    if visible_faces & MODEL_CUBE_FACE_DOWN != 0 {
        emit_textured_model_polygon(
            mesh,
            [l1, l0, t0, t1].map(|corner| transform.transform_point3(corner)),
            [u1, v0, u2, v1],
            transform_entity_normal(transform, cube.mirror, [-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
            texture,
            uv_rect,
            tint,
            cube.mirror,
        );
    }
    if visible_faces & MODEL_CUBE_FACE_UP != 0 {
        emit_textured_model_polygon(
            mesh,
            [t2, t3, l3, l2].map(|corner| transform.transform_point3(corner)),
            [u2, v1, u22, v0],
            transform_entity_normal(transform, cube.mirror, [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            texture,
            uv_rect,
            tint,
            cube.mirror,
        );
    }
    if visible_faces & MODEL_CUBE_FACE_WEST != 0 {
        emit_textured_model_polygon(
            mesh,
            [t0, l0, l3, t3].map(|corner| transform.transform_point3(corner)),
            [u0, v1, u1, v2],
            transform_entity_normal(transform, cube.mirror, [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]),
            texture,
            uv_rect,
            tint,
            cube.mirror,
        );
    }
    if visible_faces & MODEL_CUBE_FACE_NORTH != 0 {
        emit_textured_model_polygon(
            mesh,
            [t1, t0, t3, t2].map(|corner| transform.transform_point3(corner)),
            [u1, v1, u2, v2],
            transform_entity_normal(transform, cube.mirror, [-1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
            texture,
            uv_rect,
            tint,
            cube.mirror,
        );
    }
    if visible_faces & MODEL_CUBE_FACE_EAST != 0 {
        emit_textured_model_polygon(
            mesh,
            [l1, t1, t2, l2].map(|corner| transform.transform_point3(corner)),
            [u2, v1, u3, v2],
            transform_entity_normal(transform, cube.mirror, [-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
            texture,
            uv_rect,
            tint,
            cube.mirror,
        );
    }
    if visible_faces & MODEL_CUBE_FACE_SOUTH != 0 {
        emit_textured_model_polygon(
            mesh,
            [l0, l1, l2, l3].map(|corner| transform.transform_point3(corner)),
            [u3, v1, u4, v2],
            transform_entity_normal(transform, cube.mirror, [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
            texture,
            uv_rect,
            tint,
            cube.mirror,
        );
    }
}

fn emit_textured_model_polygon(
    mesh: &mut EntityModelTexturedMesh,
    corners: [Vec3; 4],
    texture_uv: [f32; 4],
    normal: [f32; 3],
    texture: EntityModelTextureRef,
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
    mirror: bool,
) {
    let [u0, v0, u1, v1] = texture_uv;
    let source_uv = [[u1, v0], [u0, v0], [u0, v1], [u1, v1]];
    let mut vertices = [
        (corners[0], source_uv[0]),
        (corners[1], source_uv[1]),
        (corners[2], source_uv[2]),
        (corners[3], source_uv[3]),
    ];
    if mirror {
        vertices.reverse();
    }
    let base = mesh.vertices.len() as u32;
    mesh.vertices
        .extend(vertices.map(|(position, uv)| EntityModelTexturedVertex {
            position: position.to_array(),
            uv: atlas_uv(uv, texture, uv_rect),
            tint,
            light: ENTITY_VERTEX_FULL_BRIGHT_LIGHT,
            overlay: ENTITY_VERTEX_NO_OVERLAY,
            normal,
        }));
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.cutout_faces += 1;
}

fn transform_entity_normal(
    transform: Mat4,
    mirror: bool,
    mirror_axis: [f32; 3],
    normal: [f32; 3],
) -> [f32; 3] {
    let mut normal = Vec3::from_array(normal);
    if mirror && normal.dot(Vec3::from_array(mirror_axis)).abs() > 0.0 {
        normal = -normal;
    }
    transform_model_normal(transform, normal.to_array())
}

fn transform_model_normal(transform: Mat4, normal: [f32; 3]) -> [f32; 3] {
    transform
        .inverse()
        .transpose()
        .transform_vector3(Vec3::from_array(normal))
        .normalize_or_zero()
        .to_array()
}

fn atlas_uv(
    texture_uv: [f32; 2],
    texture: EntityModelTextureRef,
    rect: EntityModelUvRect,
) -> [f32; 2] {
    let source = [
        texture_uv[0] / texture.size[0] as f32,
        texture_uv[1] / texture.size[1] as f32,
    ];
    [
        rect.min[0] + source[0] * (rect.max[0] - rect.min[0]),
        rect.min[1] + source[1] * (rect.max[1] - rect.min[1]),
    ]
}

fn emit_model_face(
    mesh: &mut EntityModelMesh,
    corners: [Vec3; 4],
    color: [f32; 4],
    normal: [f32; 3],
) {
    let base = mesh.vertices.len() as u32;
    mesh.vertices
        .extend(corners.map(|position| EntityModelVertex {
            position: position.to_array(),
            color,
            light: ENTITY_VERTEX_FULL_BRIGHT_LIGHT,
            overlay: ENTITY_VERTEX_NO_OVERLAY,
            normal,
        }));
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.opaque_faces += 1;
}

#[cfg(test)]
/// Historical colored-mesh test helper. The colored fallback now preserves CPU
/// tint and defers face lighting to the shader normal, so `shade` is ignored.
pub(super) fn shade_color(color: [f32; 4], _shade: f32) -> [f32; 4] {
    [
        color[0].clamp(0.0, 1.0),
        color[1].clamp(0.0, 1.0),
        color[2].clamp(0.0, 1.0),
        color[3].clamp(0.0, 1.0),
    ]
}
