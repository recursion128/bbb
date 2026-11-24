use glam::{EulerRot, Mat4, Vec3};

use super::model_layer::ModelCube;
use super::EntityModelUvRect;

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
    /// Local UV and integer atlas entry bounds for a pack standalone resource. A zero size
    /// disables this payload and preserves the ordinary atlas sampler path.
    pub(super) standalone_local_uv: [f32; 2],
    pub(super) standalone_pixel_origin: [f32; 2],
    /// `[width, height, blur, clamp]`, encoded as floats for a portable vertex ABI.
    pub(super) standalone_size_flags: [f32; 4],
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
    pub(super) parts: Vec<super::retained::ModelPartDraw>,
    pub(super) deferred_vertices: usize,
    pub(super) retain_parts: bool,
    pub(super) part_origin: Option<[f32; 3]>,
    pub(super) submission_part_start: usize,
}

impl EntityModelTexturedMesh {
    pub(super) fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            cutout_faces: 0,
            parts: Vec::new(),
            deferred_vertices: 0,
            retain_parts: false,
            part_origin: None,
            submission_part_start: 0,
        }
    }

    pub(super) fn vertex_count(&self) -> usize {
        self.vertices.len() + self.deferred_vertices
    }

    pub(super) fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.cutout_faces = 0;
        self.parts.clear();
        self.deferred_vertices = 0;
        self.retain_parts = false;
        self.part_origin = None;
        self.submission_part_start = 0;
    }
}

impl Default for EntityModelTexturedMesh {
    fn default() -> Self {
        Self::new()
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

/// A vertex for tiled custom entity geometry and texture-matrix overlays. Because these textures
/// live in a shared atlas (no per-texture `REPEAT`), the shader wraps local UVs back into the atlas
/// sub-rect. The normal is retained for exact `entityCutout` / `endCrystalBeam` cardinal lighting;
/// Breeze, energy swirl, and beacon-beam shaders intentionally ignore it.
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
    pub(super) normal: [f32; 3],
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
/// per-instance `offset` is added, and `rect` is carried so atlas-backed shaders can
/// `fract`-wrap the scrolled local UV back into the atlas sub-rect (reproducing the
/// vanilla texture-matrix scroll over a `GL_REPEAT` texture). Dedicated glint
/// shaders use the same local UV but deliberately ignore the retained rect.
/// Indices are re-based onto `scroll`'s current vertex count.
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
            normal: vertex.normal,
        });
    }
    scroll
        .indices
        .extend(textured.indices.iter().map(|index| index + base));
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PartPose {
    pub(super) offset: [f32; 3],
    pub(super) rotation: [f32; 3],
}

// Per-cube face visibility bits, mirroring the vanilla `Set<Direction> visibleFaces` that
// `CubeListBuilder.addBox(..., visibleFaces)` threads into `ModelPart.Cube` (a hidden face's
// polygon is simply not built). Bit order follows `Direction.get3DDataValue()`.
pub(super) const MODEL_CUBE_FACE_DOWN: u8 = 1 << 0;
pub(super) const MODEL_CUBE_FACE_UP: u8 = 1 << 1;
pub(super) const MODEL_CUBE_FACE_NORTH: u8 = 1 << 2;
pub(super) const MODEL_CUBE_FACE_SOUTH: u8 = 1 << 3;
pub(super) const MODEL_CUBE_FACE_WEST: u8 = 1 << 4;
pub(super) const MODEL_CUBE_FACE_EAST: u8 = 1 << 5;

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

pub(super) fn part_pose_transform(pose: PartPose) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(pose.offset) * MODEL_UNIT_SCALE)
        * Mat4::from_euler(
            EulerRot::ZYX,
            pose.rotation[2],
            pose.rotation[1],
            pose.rotation[0],
        )
}

/// Vanilla `ModelPart.Cube` corners without texture coordinates, for untextured passes.
pub(super) fn emit_untextured_model_cube(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    cube: &ModelCube,
) {
    let min = Vec3::from_array(cube.min()) * MODEL_UNIT_SCALE;
    let max = Vec3::from_array(cube.max()) * MODEL_UNIT_SCALE;
    emit_model_cube_from_min_max(mesh, transform, min, max, [1.0; 4], cube.visible_faces);
}

/// Vanilla `ModelPart.Cube` polygons: `texture_size` is the layer's `MaterialDefinition` size,
/// scaled per cube by `texScale`; `normal_matrix` is the pose's inverse transpose.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_textured_model_cube(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    normal_matrix: Mat4,
    cube: &ModelCube,
    texture_size: [f32; 2],
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
) {
    let mut min = Vec3::from_array(cube.min()) * MODEL_UNIT_SCALE;
    let mut max = Vec3::from_array(cube.max()) * MODEL_UNIT_SCALE;
    if cube.mirror {
        std::mem::swap(&mut min.x, &mut max.x);
    }
    let texture_size = [
        texture_size[0] * cube.tex_scale[0],
        texture_size[1] * cube.tex_scale[1],
    ];
    emit_textured_model_cube_from_min_max(
        mesh,
        transform,
        normal_matrix,
        min,
        max,
        cube,
        texture_size,
        uv_rect,
        tint,
    );
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
    normal_matrix: Mat4,
    min: Vec3,
    max: Vec3,
    cube: &ModelCube,
    texture_size: [f32; 2],
    uv_rect: EntityModelUvRect,
    tint: [f32; 4],
) {
    let visible_faces = cube.visible_faces;
    let t0 = Vec3::new(min.x, min.y, min.z);
    let t1 = Vec3::new(max.x, min.y, min.z);
    let t2 = Vec3::new(max.x, max.y, min.z);
    let t3 = Vec3::new(min.x, max.y, min.z);
    let l0 = Vec3::new(min.x, min.y, max.z);
    let l1 = Vec3::new(max.x, min.y, max.z);
    let l2 = Vec3::new(max.x, max.y, max.z);
    let l3 = Vec3::new(min.x, max.y, max.z);

    let [width, height, depth] = cube.dimensions;
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
            transform_entity_normal(
                normal_matrix,
                cube.mirror,
                [-1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
            ),
            texture_size,
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
            transform_entity_normal(
                normal_matrix,
                cube.mirror,
                [-1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ),
            texture_size,
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
            transform_entity_normal(
                normal_matrix,
                cube.mirror,
                [1.0, 0.0, 0.0],
                [-1.0, 0.0, 0.0],
            ),
            texture_size,
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
            transform_entity_normal(
                normal_matrix,
                cube.mirror,
                [-1.0, 0.0, 0.0],
                [0.0, 0.0, -1.0],
            ),
            texture_size,
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
            transform_entity_normal(
                normal_matrix,
                cube.mirror,
                [-1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            ),
            texture_size,
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
            transform_entity_normal(
                normal_matrix,
                cube.mirror,
                [-1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
            ),
            texture_size,
            uv_rect,
            tint,
            cube.mirror,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_textured_model_polygon(
    mesh: &mut EntityModelTexturedMesh,
    corners: [Vec3; 4],
    texture_uv: [f32; 4],
    normal: [f32; 3],
    texture_size: [f32; 2],
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
    let base = mesh.vertex_count() as u32;
    mesh.vertices
        .extend(vertices.map(|(position, uv)| EntityModelTexturedVertex {
            position: position.to_array(),
            uv: atlas_uv(uv, texture_size, uv_rect),
            tint,
            light: ENTITY_VERTEX_FULL_BRIGHT_LIGHT,
            overlay: ENTITY_VERTEX_NO_OVERLAY,
            normal,
            standalone_local_uv: [0.0; 2],
            standalone_pixel_origin: [0.0; 2],
            standalone_size_flags: [0.0; 4],
        }));
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    mesh.cutout_faces += 1;
}

fn transform_entity_normal(
    normal_matrix: Mat4,
    mirror: bool,
    mirror_axis: [f32; 3],
    normal: [f32; 3],
) -> [f32; 3] {
    let mut normal = Vec3::from_array(normal);
    if mirror && normal.dot(Vec3::from_array(mirror_axis)).abs() > 0.0 {
        normal = -normal;
    }
    normal_matrix
        .transform_vector3(normal)
        .normalize_or_zero()
        .to_array()
}

fn transform_model_normal(transform: Mat4, normal: [f32; 3]) -> [f32; 3] {
    transform
        .inverse()
        .transpose()
        .transform_vector3(Vec3::from_array(normal))
        .normalize_or_zero()
        .to_array()
}

fn atlas_uv(texture_uv: [f32; 2], texture_size: [f32; 2], rect: EntityModelUvRect) -> [f32; 2] {
    let source = [
        texture_uv[0] / texture_size[0],
        texture_uv[1] / texture_size[1],
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
