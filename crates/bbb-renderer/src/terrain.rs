use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainMaterialClass {
    Empty,
    Opaque,
    Cutout,
    Fluid,
    Translucent,
}

impl TerrainMaterialClass {
    fn is_meshed_opaque(self) -> bool {
        matches!(self, Self::Opaque)
    }

    fn is_meshed_cutout(self) -> bool {
        matches!(self, Self::Cutout)
    }

    fn is_meshed_translucent(self) -> bool {
        matches!(self, Self::Fluid | Self::Translucent)
    }

    fn is_meshed_terrain(self) -> bool {
        self.is_meshed_opaque() || self.is_meshed_cutout() || self.is_meshed_translucent()
    }

    fn occludes_opaque(self) -> bool {
        matches!(self, Self::Opaque)
    }

    fn occludes_terrain(self) -> bool {
        matches!(self, Self::Opaque)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainRenderShape {
    Cube,
    Cross,
    Box {
        from: [u8; 3],
        to: [u8; 3],
        face_present: [bool; 6],
        face_uvs: [[u8; 4]; 6],
        face_cull: [bool; 6],
    },
    Boxes(Vec<TerrainBox>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainBox {
    pub from: [u8; 3],
    pub to: [u8; 3],
    pub face_present: [bool; 6],
    pub face_uvs: [[u8; 4]; 6],
    pub face_cull: [bool; 6],
    pub texture_indices: [u32; 6],
    pub tint: [TerrainTint; 6],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainCell {
    pub block_state_id: i32,
    pub material: TerrainMaterialClass,
    pub texture_indices: [u32; 6],
    pub render_shape: TerrainRenderShape,
    pub light: TerrainLight,
    pub tint: [TerrainTint; 6],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainLight {
    pub sky: u8,
    pub block: u8,
}

impl TerrainLight {
    pub const FULL_BRIGHT: Self = Self { sky: 15, block: 0 };

    fn as_shader_light(self) -> [f32; 2] {
        [
            self.block.min(15) as f32 / 15.0,
            self.sky.min(15) as f32 / 15.0,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainTint {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl TerrainTint {
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };

    pub fn from_rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    fn as_shader_tint(self) -> [f32; 3] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
        ]
    }
}

impl TerrainCell {
    pub const EMPTY: Self = Self {
        block_state_id: 0,
        material: TerrainMaterialClass::Empty,
        texture_indices: [0; 6],
        render_shape: TerrainRenderShape::Cube,
        light: TerrainLight::FULL_BRIGHT,
        tint: [TerrainTint::WHITE; 6],
    };

    pub fn with_texture(
        block_state_id: i32,
        material: TerrainMaterialClass,
        texture_index: u32,
    ) -> Self {
        Self {
            block_state_id,
            material,
            texture_indices: [texture_index; 6],
            render_shape: TerrainRenderShape::Cube,
            light: TerrainLight::FULL_BRIGHT,
            tint: [TerrainTint::WHITE; 6],
        }
    }

    pub fn with_shape(
        block_state_id: i32,
        material: TerrainMaterialClass,
        texture_index: u32,
        render_shape: TerrainRenderShape,
    ) -> Self {
        Self {
            block_state_id,
            material,
            texture_indices: [texture_index; 6],
            render_shape,
            light: TerrainLight::FULL_BRIGHT,
            tint: [TerrainTint::WHITE; 6],
        }
    }

    pub fn with_light(mut self, light: TerrainLight) -> Self {
        self.light = light;
        self
    }

    pub fn with_tint(mut self, tint: [TerrainTint; 6]) -> Self {
        self.tint = tint;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainChunkSnapshot {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub min_y: i32,
    pub height: usize,
    pub cells: Vec<TerrainCell>,
}

impl TerrainChunkSnapshot {
    pub fn new(
        chunk_x: i32,
        chunk_z: i32,
        min_y: i32,
        height: usize,
        cells: Vec<TerrainCell>,
    ) -> Self {
        assert_eq!(cells.len(), 16 * height * 16);
        Self {
            chunk_x,
            chunk_z,
            min_y,
            height,
            cells,
        }
    }

    pub fn cell(&self, x: i32, y: i32, z: i32) -> Option<&TerrainCell> {
        if !(0..16).contains(&x) || !(0..self.height as i32).contains(&y) || !(0..16).contains(&z) {
            return None;
        }
        self.cells
            .get(cell_index(x as usize, y as usize, z as usize, self.height))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainFace {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl TerrainFace {
    fn index(self) -> usize {
        match self {
            Self::Down => 0,
            Self::Up => 1,
            Self::North => 2,
            Self::South => 3,
            Self::West => 4,
            Self::East => 5,
        }
    }
}

#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Serialize, Deserialize, bytemuck::Pod, bytemuck::Zeroable,
)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub light: [f32; 2],
    pub tint: [f32; 3],
    pub block_state_id: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TerrainMesh {
    pub vertices: Vec<TerrainVertex>,
    pub indices: Vec<u32>,
    pub source_sections: usize,
    pub opaque_faces: usize,
    pub cutout_faces: usize,
    pub translucent_faces: usize,
    pub culled_faces: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TerrainMeshLayers {
    pub opaque: Vec<TerrainMesh>,
    pub cutout: Vec<TerrainMesh>,
    pub translucent: Vec<TerrainMesh>,
    pub source_sections: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TerrainUvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl TerrainUvRect {
    pub const UNIT: Self = Self {
        min: [0.0, 0.0],
        max: [1.0, 1.0],
    };

    fn map(self, uv: [f32; 2]) -> [f32; 2] {
        [
            self.min[0] + (self.max[0] - self.min[0]) * uv[0],
            self.min[1] + (self.max[1] - self.min[1]) * uv[1],
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainTextureAtlas {
    pub rects: Vec<TerrainUvRect>,
    pub fallback_index: u32,
}

impl TerrainTextureAtlas {
    pub fn unit() -> Self {
        Self {
            rects: vec![TerrainUvRect::UNIT],
            fallback_index: 0,
        }
    }

    fn rect(&self, texture_index: u32) -> TerrainUvRect {
        self.rects
            .get(texture_index as usize)
            .copied()
            .or_else(|| self.rects.get(self.fallback_index as usize).copied())
            .unwrap_or(TerrainUvRect::UNIT)
    }
}

pub fn build_opaque_chunk_mesh(snapshot: &TerrainChunkSnapshot) -> TerrainMesh {
    build_opaque_terrain_meshes(std::slice::from_ref(snapshot))
        .into_iter()
        .next()
        .unwrap_or_default()
}

pub fn build_opaque_terrain_meshes(snapshots: &[TerrainChunkSnapshot]) -> Vec<TerrainMesh> {
    let atlas = TerrainTextureAtlas::unit();
    build_opaque_terrain_meshes_with_atlas(snapshots, &atlas)
}

pub fn build_opaque_terrain_meshes_with_atlas(
    snapshots: &[TerrainChunkSnapshot],
    atlas: &TerrainTextureAtlas,
) -> Vec<TerrainMesh> {
    let lookup = TerrainChunkLookup::new(snapshots);
    snapshots
        .iter()
        .map(|snapshot| {
            build_chunk_mesh_with_lookup(snapshot, &lookup, atlas, TerrainMeshMode::OpaqueOnly)
        })
        .collect()
}

pub fn build_terrain_meshes_with_atlas(
    snapshots: &[TerrainChunkSnapshot],
    atlas: &TerrainTextureAtlas,
) -> Vec<TerrainMesh> {
    let lookup = TerrainChunkLookup::new(snapshots);
    snapshots
        .iter()
        .map(|snapshot| {
            build_chunk_mesh_with_lookup(snapshot, &lookup, atlas, TerrainMeshMode::OpaqueCutout)
        })
        .collect()
}

pub fn build_terrain_mesh_layers_with_atlas(
    snapshots: &[TerrainChunkSnapshot],
    atlas: &TerrainTextureAtlas,
) -> TerrainMeshLayers {
    let lookup = TerrainChunkLookup::new(snapshots);
    let source_sections = snapshots
        .iter()
        .map(|snapshot| snapshot.height.div_ceil(16))
        .sum();
    let opaque = snapshots
        .iter()
        .map(|snapshot| {
            build_chunk_mesh_with_lookup(snapshot, &lookup, atlas, TerrainMeshMode::OpaqueOnly)
        })
        .collect();
    let cutout = snapshots
        .iter()
        .map(|snapshot| {
            build_chunk_mesh_with_lookup(snapshot, &lookup, atlas, TerrainMeshMode::CutoutOnly)
        })
        .collect();
    let translucent = snapshots
        .iter()
        .map(|snapshot| {
            build_chunk_mesh_with_lookup(snapshot, &lookup, atlas, TerrainMeshMode::TranslucentOnly)
        })
        .collect();

    TerrainMeshLayers {
        opaque,
        cutout,
        translucent,
        source_sections,
    }
}

#[derive(Debug, Clone, Copy)]
enum TerrainMeshMode {
    OpaqueOnly,
    CutoutOnly,
    TranslucentOnly,
    OpaqueCutout,
}

impl TerrainMeshMode {
    fn is_meshed(self, material: TerrainMaterialClass) -> bool {
        match self {
            Self::OpaqueOnly => material.is_meshed_opaque(),
            Self::CutoutOnly => material.is_meshed_cutout(),
            Self::TranslucentOnly => material.is_meshed_translucent(),
            Self::OpaqueCutout => material.is_meshed_terrain(),
        }
    }

    fn is_occluded_by(self, material: TerrainMaterialClass) -> bool {
        match self {
            Self::OpaqueOnly => material.occludes_opaque(),
            Self::CutoutOnly => material.occludes_terrain(),
            Self::TranslucentOnly => material.occludes_terrain(),
            Self::OpaqueCutout => material.occludes_terrain(),
        }
    }

    fn culls_face_between(
        self,
        current: TerrainMaterialClass,
        neighbor: TerrainMaterialClass,
    ) -> bool {
        self.is_occluded_by(neighbor)
            || (matches!(current, TerrainMaterialClass::Fluid)
                && matches!(neighbor, TerrainMaterialClass::Fluid))
    }
}

fn build_chunk_mesh_with_lookup(
    snapshot: &TerrainChunkSnapshot,
    lookup: &TerrainChunkLookup<'_>,
    atlas: &TerrainTextureAtlas,
    mode: TerrainMeshMode,
) -> TerrainMesh {
    let mut mesh = TerrainMesh {
        source_sections: snapshot.height.div_ceil(16),
        ..TerrainMesh::default()
    };
    for y in 0..snapshot.height as i32 {
        for z in 0..16 {
            for x in 0..16 {
                let cell = snapshot
                    .cell(x, y, z)
                    .expect("in-bounds terrain cell exists");
                if !mode.is_meshed(cell.material) {
                    continue;
                }

                let world_x = snapshot.chunk_x * 16 + x;
                let world_y = snapshot.min_y + y;
                let world_z = snapshot.chunk_z * 16 + z;
                match &cell.render_shape {
                    TerrainRenderShape::Cross => {
                        emit_cross(
                            &mut mesh,
                            world_x,
                            world_y,
                            world_z,
                            cell.block_state_id,
                            cell.material,
                            cell.light,
                            cell.tint,
                            cell.texture_indices,
                            atlas,
                        );
                        continue;
                    }
                    TerrainRenderShape::Box {
                        from,
                        to,
                        face_present,
                        face_uvs,
                        face_cull,
                    } => {
                        emit_box(
                            &mut mesh,
                            world_x,
                            world_y,
                            world_z,
                            cell.block_state_id,
                            cell.material,
                            cell.light,
                            cell.tint,
                            cell.texture_indices,
                            atlas,
                            *from,
                            *to,
                            *face_present,
                            *face_uvs,
                            *face_cull,
                            lookup,
                            mode,
                        );
                        continue;
                    }
                    TerrainRenderShape::Boxes(model_boxes) => {
                        for model_box in model_boxes {
                            emit_box(
                                &mut mesh,
                                world_x,
                                world_y,
                                world_z,
                                cell.block_state_id,
                                cell.material,
                                cell.light,
                                model_box.tint,
                                model_box.texture_indices,
                                atlas,
                                model_box.from,
                                model_box.to,
                                model_box.face_present,
                                model_box.face_uvs,
                                model_box.face_cull,
                                lookup,
                                mode,
                            );
                        }
                        continue;
                    }
                    TerrainRenderShape::Cube => {}
                }

                for face in FACES {
                    let neighbor =
                        lookup.cell(world_x + face.dx, world_y + face.dy, world_z + face.dz);
                    if neighbor
                        .map(|neighbor| mode.culls_face_between(cell.material, neighbor.material))
                        .unwrap_or(false)
                    {
                        mesh.culled_faces += 1;
                        continue;
                    }
                    emit_face(
                        &mut mesh,
                        world_x,
                        world_y,
                        world_z,
                        cell.block_state_id,
                        cell.material,
                        cell.light,
                        cell.tint[face.face.index()],
                        atlas.rect(cell.texture_indices[face.face.index()]),
                        face,
                    );
                }
            }
        }
    }
    mesh
}

struct TerrainChunkLookup<'a> {
    chunks: HashMap<(i32, i32), &'a TerrainChunkSnapshot>,
}

impl<'a> TerrainChunkLookup<'a> {
    fn new(snapshots: &'a [TerrainChunkSnapshot]) -> Self {
        Self {
            chunks: snapshots
                .iter()
                .map(|snapshot| ((snapshot.chunk_x, snapshot.chunk_z), snapshot))
                .collect(),
        }
    }

    fn cell(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<&TerrainCell> {
        let chunk_x = world_x.div_euclid(16);
        let chunk_z = world_z.div_euclid(16);
        let snapshot = self.chunks.get(&(chunk_x, chunk_z))?;
        snapshot.cell(
            world_x.rem_euclid(16),
            world_y - snapshot.min_y,
            world_z.rem_euclid(16),
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct FaceDef {
    face: TerrainFace,
    normal: [f32; 3],
    dx: i32,
    dy: i32,
    dz: i32,
    corners: [[f32; 3]; 4],
}

const FACES: [FaceDef; 6] = [
    FaceDef {
        face: TerrainFace::Down,
        normal: [0.0, -1.0, 0.0],
        dx: 0,
        dy: -1,
        dz: 0,
        corners: [
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
    },
    FaceDef {
        face: TerrainFace::Up,
        normal: [0.0, 1.0, 0.0],
        dx: 0,
        dy: 1,
        dz: 0,
        corners: [
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ],
    },
    FaceDef {
        face: TerrainFace::North,
        normal: [0.0, 0.0, -1.0],
        dx: 0,
        dy: 0,
        dz: -1,
        corners: [
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
    },
    FaceDef {
        face: TerrainFace::South,
        normal: [0.0, 0.0, 1.0],
        dx: 0,
        dy: 0,
        dz: 1,
        corners: [
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 0.0, 1.0],
        ],
    },
    FaceDef {
        face: TerrainFace::West,
        normal: [-1.0, 0.0, 0.0],
        dx: -1,
        dy: 0,
        dz: 0,
        corners: [
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
    },
    FaceDef {
        face: TerrainFace::East,
        normal: [1.0, 0.0, 0.0],
        dx: 1,
        dy: 0,
        dz: 0,
        corners: [
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
        ],
    },
];

fn emit_face(
    mesh: &mut TerrainMesh,
    x: i32,
    y: i32,
    z: i32,
    block_state_id: i32,
    material: TerrainMaterialClass,
    light: TerrainLight,
    tint: TerrainTint,
    uv_rect: TerrainUvRect,
    face: FaceDef,
) {
    let base = mesh.vertices.len() as u32;
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for (corner, uv) in face.corners.into_iter().zip(uvs) {
        mesh.vertices.push(TerrainVertex {
            position: [
                x as f32 + corner[0],
                y as f32 + corner[1],
                z as f32 + corner[2],
            ],
            normal: face.normal,
            uv: uv_rect.map(uv),
            light: light.as_shader_light(),
            tint: tint.as_shader_tint(),
            block_state_id,
        });
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    match material {
        TerrainMaterialClass::Opaque => mesh.opaque_faces += 1,
        TerrainMaterialClass::Cutout => mesh.cutout_faces += 1,
        TerrainMaterialClass::Fluid | TerrainMaterialClass::Translucent => {
            mesh.translucent_faces += 1
        }
        _ => {}
    }
}

fn emit_cross(
    mesh: &mut TerrainMesh,
    x: i32,
    y: i32,
    z: i32,
    block_state_id: i32,
    material: TerrainMaterialClass,
    light: TerrainLight,
    tint: [TerrainTint; 6],
    texture_indices: [u32; 6],
    atlas: &TerrainTextureAtlas,
) {
    const CROSS_FACES: [(TerrainFace, [f32; 3], [[f32; 3]; 4]); 4] = [
        (
            TerrainFace::North,
            [-0.70710677, 0.0, 0.70710677],
            [
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
                [1.0, 0.0, 1.0],
            ],
        ),
        (
            TerrainFace::South,
            [0.70710677, 0.0, -0.70710677],
            [
                [1.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
        ),
        (
            TerrainFace::West,
            [-0.70710677, 0.0, -0.70710677],
            [
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
        ),
        (
            TerrainFace::East,
            [0.70710677, 0.0, 0.70710677],
            [
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
                [1.0, 0.0, 0.0],
            ],
        ),
    ];

    for (face, normal, corners) in CROSS_FACES {
        emit_custom_quad(
            mesh,
            x,
            y,
            z,
            block_state_id,
            material,
            light,
            tint[face.index()],
            atlas.rect(texture_indices[face.index()]),
            normal,
            corners,
        );
    }
}

fn emit_box(
    mesh: &mut TerrainMesh,
    x: i32,
    y: i32,
    z: i32,
    block_state_id: i32,
    material: TerrainMaterialClass,
    light: TerrainLight,
    tint: [TerrainTint; 6],
    texture_indices: [u32; 6],
    atlas: &TerrainTextureAtlas,
    from: [u8; 3],
    to: [u8; 3],
    face_present: [bool; 6],
    face_uvs: [[u8; 4]; 6],
    face_cull: [bool; 6],
    lookup: &TerrainChunkLookup<'_>,
    mode: TerrainMeshMode,
) {
    let min = [
        from[0] as f32 / 16.0,
        from[1] as f32 / 16.0,
        from[2] as f32 / 16.0,
    ];
    let mut max = [
        to[0] as f32 / 16.0,
        to[1] as f32 / 16.0,
        to[2] as f32 / 16.0,
    ];
    if matches!(material, TerrainMaterialClass::Fluid)
        && lookup
            .cell(x, y + 1, z)
            .is_some_and(|neighbor| matches!(neighbor.material, TerrainMaterialClass::Fluid))
    {
        max[1] = 1.0;
    }

    for face in FACES {
        let face_index = face.face.index();
        if !face_present[face_index] {
            continue;
        }
        if face_cull[face_index] {
            let neighbor = lookup.cell(x + face.dx, y + face.dy, z + face.dz);
            if neighbor
                .map(|neighbor| mode.culls_face_between(material, neighbor.material))
                .unwrap_or(false)
            {
                mesh.culled_faces += 1;
                continue;
            }
        }

        emit_custom_quad_with_uvs(
            mesh,
            x,
            y,
            z,
            block_state_id,
            material,
            light,
            tint[face_index],
            atlas.rect(texture_indices[face_index]),
            face.normal,
            box_face_corners(face.face, min, max),
            face_uvs_from_crop(face_uvs[face_index]),
        );
    }
}

fn box_face_corners(face: TerrainFace, min: [f32; 3], max: [f32; 3]) -> [[f32; 3]; 4] {
    match face {
        TerrainFace::Down => [
            [min[0], min[1], max[2]],
            [max[0], min[1], max[2]],
            [max[0], min[1], min[2]],
            [min[0], min[1], min[2]],
        ],
        TerrainFace::Up => [
            [min[0], max[1], min[2]],
            [max[0], max[1], min[2]],
            [max[0], max[1], max[2]],
            [min[0], max[1], max[2]],
        ],
        TerrainFace::North => [
            [max[0], min[1], min[2]],
            [max[0], max[1], min[2]],
            [min[0], max[1], min[2]],
            [min[0], min[1], min[2]],
        ],
        TerrainFace::South => [
            [min[0], min[1], max[2]],
            [min[0], max[1], max[2]],
            [max[0], max[1], max[2]],
            [max[0], min[1], max[2]],
        ],
        TerrainFace::West => [
            [min[0], min[1], min[2]],
            [min[0], max[1], min[2]],
            [min[0], max[1], max[2]],
            [min[0], min[1], max[2]],
        ],
        TerrainFace::East => [
            [max[0], min[1], max[2]],
            [max[0], max[1], max[2]],
            [max[0], max[1], min[2]],
            [max[0], min[1], min[2]],
        ],
    }
}

fn face_uvs_from_crop(uv: [u8; 4]) -> [[f32; 2]; 4] {
    let min_u = uv[0] as f32 / 16.0;
    let min_v = uv[1] as f32 / 16.0;
    let max_u = uv[2] as f32 / 16.0;
    let max_v = uv[3] as f32 / 16.0;
    [
        [min_u, min_v],
        [max_u, min_v],
        [max_u, max_v],
        [min_u, max_v],
    ]
}

fn emit_custom_quad(
    mesh: &mut TerrainMesh,
    x: i32,
    y: i32,
    z: i32,
    block_state_id: i32,
    material: TerrainMaterialClass,
    light: TerrainLight,
    tint: TerrainTint,
    uv_rect: TerrainUvRect,
    normal: [f32; 3],
    corners: [[f32; 3]; 4],
) {
    emit_custom_quad_with_uvs(
        mesh,
        x,
        y,
        z,
        block_state_id,
        material,
        light,
        tint,
        uv_rect,
        normal,
        corners,
        [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
    );
}

fn emit_custom_quad_with_uvs(
    mesh: &mut TerrainMesh,
    x: i32,
    y: i32,
    z: i32,
    block_state_id: i32,
    material: TerrainMaterialClass,
    light: TerrainLight,
    tint: TerrainTint,
    uv_rect: TerrainUvRect,
    normal: [f32; 3],
    corners: [[f32; 3]; 4],
    uvs: [[f32; 2]; 4],
) {
    let base = mesh.vertices.len() as u32;
    for (corner, uv) in corners.into_iter().zip(uvs) {
        mesh.vertices.push(TerrainVertex {
            position: [
                x as f32 + corner[0],
                y as f32 + corner[1],
                z as f32 + corner[2],
            ],
            normal,
            uv: uv_rect.map(uv),
            light: light.as_shader_light(),
            tint: tint.as_shader_tint(),
            block_state_id,
        });
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    match material {
        TerrainMaterialClass::Opaque => mesh.opaque_faces += 1,
        TerrainMaterialClass::Cutout => mesh.cutout_faces += 1,
        TerrainMaterialClass::Fluid | TerrainMaterialClass::Translucent => {
            mesh.translucent_faces += 1
        }
        _ => {}
    }
}

fn cell_index(x: usize, y: usize, z: usize, height: usize) -> usize {
    debug_assert!(x < 16);
    debug_assert!(y < height);
    debug_assert!(z < 16);
    ((y * 16) + z) * 16 + x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meshes_single_opaque_block_as_six_faces() {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
        cells[cell_index(1, 0, 2, 1)] =
            TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
        let snapshot = TerrainChunkSnapshot::new(2, -3, 0, 1, cells);

        let mesh = build_opaque_chunk_mesh(&snapshot);
        assert_eq!(mesh.source_sections, 1);
        assert_eq!(mesh.opaque_faces, 6);
        assert_eq!(mesh.culled_faces, 0);
        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.block_state_id == 1));
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.position[0] >= 33.0 && vertex.position[2] <= -45.0));
    }

    #[test]
    fn culls_internal_faces_between_opaque_blocks() {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
        cells[cell_index(1, 0, 2, 1)] =
            TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
        cells[cell_index(2, 0, 2, 1)] =
            TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
        let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

        let mesh = build_opaque_chunk_mesh(&snapshot);
        assert_eq!(mesh.source_sections, 1);
        assert_eq!(mesh.opaque_faces, 10);
        assert_eq!(mesh.culled_faces, 2);
        assert_eq!(mesh.vertices.len(), 40);
        assert_eq!(mesh.indices.len(), 60);
    }

    #[test]
    fn culls_internal_faces_between_fluid_blocks() {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
        cells[cell_index(1, 0, 2, 1)] =
            TerrainCell::with_texture(86, TerrainMaterialClass::Fluid, 0);
        cells[cell_index(2, 0, 2, 1)] =
            TerrainCell::with_texture(86, TerrainMaterialClass::Fluid, 0);
        let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

        let layers =
            build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());
        assert_eq!(layers.translucent.len(), 1);
        assert_eq!(layers.translucent[0].translucent_faces, 10);
        assert_eq!(layers.translucent[0].culled_faces, 2);
        assert_eq!(layers.translucent[0].vertices.len(), 40);
        assert_eq!(layers.translucent[0].indices.len(), 60);
    }

    #[test]
    fn fluid_box_extends_to_above_fluid_without_internal_top_face() {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
        cells[cell_index(1, 0, 2, 2)] =
            TerrainCell::with_shape(86, TerrainMaterialClass::Fluid, 0, fluid_box_shape(14));
        cells[cell_index(1, 1, 2, 2)] =
            TerrainCell::with_shape(87, TerrainMaterialClass::Fluid, 0, fluid_box_shape(14));
        let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 2, cells);

        let layers =
            build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());
        let mesh = &layers.translucent[0];

        assert_eq!(mesh.translucent_faces, 10);
        assert_eq!(mesh.culled_faces, 2);
        assert!(mesh
            .vertices
            .iter()
            .filter(|vertex| vertex.block_state_id == 86)
            .any(|vertex| vertex.position[1] == 1.0));
        assert!(mesh
            .vertices
            .iter()
            .filter(|vertex| vertex.block_state_id == 87)
            .any(|vertex| vertex.position[1] == 1.875));
    }

    #[test]
    fn culls_faces_between_adjacent_chunk_snapshots() {
        let left = single_block_snapshot(0, 0, 15, 0, 2);
        let right = single_block_snapshot(1, 0, 0, 0, 2);

        let meshes = build_opaque_terrain_meshes(&[left, right]);
        assert_eq!(meshes.len(), 2);
        assert_eq!(
            meshes.iter().map(|mesh| mesh.opaque_faces).sum::<usize>(),
            10
        );
        assert_eq!(
            meshes.iter().map(|mesh| mesh.culled_faces).sum::<usize>(),
            2
        );
        assert_eq!(
            meshes.iter().map(|mesh| mesh.vertices.len()).sum::<usize>(),
            40
        );
        assert_eq!(
            meshes.iter().map(|mesh| mesh.indices.len()).sum::<usize>(),
            60
        );
    }

    #[test]
    fn culls_fluid_faces_between_adjacent_chunk_snapshots() {
        let left = single_fluid_snapshot(0, 0, 15, 0, 2);
        let right = single_fluid_snapshot(1, 0, 0, 0, 2);

        let layers =
            build_terrain_mesh_layers_with_atlas(&[left, right], &TerrainTextureAtlas::unit());
        assert_eq!(layers.translucent.len(), 2);
        assert_eq!(
            layers
                .translucent
                .iter()
                .map(|mesh| mesh.translucent_faces)
                .sum::<usize>(),
            10
        );
        assert_eq!(
            layers
                .translucent
                .iter()
                .map(|mesh| mesh.culled_faces)
                .sum::<usize>(),
            2
        );
    }

    #[test]
    fn maps_face_uvs_into_texture_atlas_rect() {
        let mut snapshot = single_block_snapshot(0, 0, 1, 0, 2);
        snapshot.cells[cell_index(1, 0, 2, 1)].texture_indices = [1; 6];
        let atlas = TerrainTextureAtlas {
            rects: vec![
                TerrainUvRect::UNIT,
                TerrainUvRect {
                    min: [0.25, 0.5],
                    max: [0.5, 0.75],
                },
            ],
            fallback_index: 0,
        };

        let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &atlas)
            .into_iter()
            .next()
            .unwrap();

        assert_eq!(mesh.vertices[0].uv, [0.25, 0.5]);
        assert_eq!(mesh.vertices[1].uv, [0.5, 0.5]);
        assert_eq!(mesh.vertices[2].uv, [0.5, 0.75]);
        assert_eq!(mesh.vertices[3].uv, [0.25, 0.75]);
    }

    #[test]
    fn maps_different_faces_to_different_texture_rects() {
        let mut snapshot = single_block_snapshot(0, 0, 1, 0, 2);
        snapshot.cells[cell_index(1, 0, 2, 1)].texture_indices = [1, 2, 0, 0, 0, 0];
        let atlas = TerrainTextureAtlas {
            rects: vec![
                TerrainUvRect::UNIT,
                TerrainUvRect {
                    min: [0.0, 0.0],
                    max: [0.25, 0.25],
                },
                TerrainUvRect {
                    min: [0.5, 0.5],
                    max: [0.75, 0.75],
                },
            ],
            fallback_index: 0,
        };

        let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &atlas)
            .into_iter()
            .next()
            .unwrap();

        assert_eq!(mesh.vertices[0].uv, [0.0, 0.0]);
        assert_eq!(mesh.vertices[4].uv, [0.5, 0.5]);
    }

    #[test]
    fn combined_terrain_mesh_includes_cutout_faces() {
        let mut snapshot = single_block_snapshot(0, 0, 1, 0, 2);
        snapshot.cells[cell_index(1, 0, 2, 1)].material = TerrainMaterialClass::Cutout;

        let opaque = build_opaque_chunk_mesh(&snapshot);
        assert_eq!(opaque.opaque_faces, 0);
        assert_eq!(opaque.cutout_faces, 0);

        let combined = build_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(combined.opaque_faces, 0);
        assert_eq!(combined.cutout_faces, 6);
        assert_eq!(combined.vertices.len(), 24);
        assert_eq!(combined.indices.len(), 36);
    }

    #[test]
    fn cross_cutout_mesh_emits_vanilla_cross_quads() {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
        cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_shape(
            2,
            TerrainMaterialClass::Cutout,
            0,
            TerrainRenderShape::Cross,
        );
        let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

        let mesh = build_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
            .into_iter()
            .next()
            .unwrap();

        assert_eq!(mesh.opaque_faces, 0);
        assert_eq!(mesh.cutout_faces, 4);
        assert_eq!(mesh.culled_faces, 0);
        assert_eq!(mesh.vertices.len(), 16);
        assert_eq!(mesh.indices.len(), 24);
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.position == [1.0, 0.0, 2.0]));
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.position == [2.0, 1.0, 3.0]));
    }

    #[test]
    fn cross_cutout_faces_are_not_neighbor_culled() {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
        cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_shape(
            2,
            TerrainMaterialClass::Cutout,
            0,
            TerrainRenderShape::Cross,
        );
        cells[cell_index(2, 0, 2, 1)] =
            TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
        let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

        let layers =
            build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());

        assert_eq!(layers.opaque[0].opaque_faces, 6);
        assert_eq!(layers.cutout[0].cutout_faces, 4);
        assert_eq!(layers.cutout[0].culled_faces, 0);
    }

    #[test]
    fn box_model_mesh_uses_bounds_and_face_uv_crop() {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
        cells[cell_index(1, 0, 2, 1)] =
            TerrainCell::with_shape(3, TerrainMaterialClass::Opaque, 1, slab_box_shape());
        let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);
        let atlas = TerrainTextureAtlas {
            rects: vec![
                TerrainUvRect::UNIT,
                TerrainUvRect {
                    min: [0.25, 0.25],
                    max: [0.75, 0.75],
                },
            ],
            fallback_index: 0,
        };

        let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &atlas)
            .into_iter()
            .next()
            .unwrap();

        assert_eq!(mesh.opaque_faces, 6);
        assert_eq!(mesh.vertices.len(), 24);
        assert!(mesh.vertices.iter().any(|vertex| vertex.position[1] == 0.5));
        assert!(!mesh.vertices.iter().any(|vertex| vertex.position[1] == 1.0));
        assert!(mesh.vertices.iter().any(|vertex| vertex.uv == [0.25, 0.5]));
    }

    #[test]
    fn box_model_culls_only_faces_marked_by_cullface() {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
        cells[cell_index(1, 0, 2, 1)] =
            TerrainCell::with_shape(3, TerrainMaterialClass::Opaque, 0, slab_box_shape());
        cells[cell_index(1, 0, 1, 1)] =
            TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
        cells[cell_index(1, 0, 3, 1)] =
            TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
        let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

        let mesh =
            build_opaque_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
                .into_iter()
                .next()
                .unwrap();

        assert_eq!(mesh.culled_faces, 4);
        assert_eq!(mesh.opaque_faces, 14);
    }

    #[test]
    fn multi_box_model_skips_absent_faces() {
        let mut upper = TerrainBox {
            from: [8, 8, 0],
            to: [16, 16, 16],
            face_present: [true; 6],
            face_uvs: [[0, 0, 16, 16]; 6],
            face_cull: [false; 6],
            texture_indices: [0; 6],
            tint: [TerrainTint::WHITE; 6],
        };
        upper.face_present[TerrainFace::Down.index()] = false;
        let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
        cells[cell_index(1, 0, 2, 1)] = TerrainCell {
            block_state_id: 4,
            material: TerrainMaterialClass::Opaque,
            texture_indices: [0; 6],
            light: TerrainLight::FULL_BRIGHT,
            tint: [TerrainTint::WHITE; 6],
            render_shape: TerrainRenderShape::Boxes(vec![
                TerrainBox {
                    from: [0, 0, 0],
                    to: [16, 8, 16],
                    face_present: [true; 6],
                    face_uvs: [[0, 0, 16, 16]; 6],
                    face_cull: [false; 6],
                    texture_indices: [0; 6],
                    tint: [TerrainTint::WHITE; 6],
                },
                upper,
            ]),
        };
        let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

        let mesh =
            build_opaque_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
                .into_iter()
                .next()
                .unwrap();

        assert_eq!(mesh.opaque_faces, 11);
        assert_eq!(mesh.vertices.len(), 44);
        assert_eq!(mesh.indices.len(), 66);
    }

    #[test]
    fn boxes_use_per_box_texture_and_tint() {
        let grass_tint = TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59);
        let foliage_tint = TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18);
        let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
        cells[cell_index(1, 0, 2, 1)] = TerrainCell {
            block_state_id: 4,
            material: TerrainMaterialClass::Opaque,
            texture_indices: [0; 6],
            light: TerrainLight::FULL_BRIGHT,
            tint: [TerrainTint::WHITE; 6],
            render_shape: TerrainRenderShape::Boxes(vec![
                TerrainBox {
                    from: [0, 0, 0],
                    to: [16, 8, 16],
                    face_present: [true; 6],
                    face_uvs: [[0, 0, 16, 16]; 6],
                    face_cull: [false; 6],
                    texture_indices: [1; 6],
                    tint: [grass_tint; 6],
                },
                TerrainBox {
                    from: [0, 8, 0],
                    to: [16, 16, 16],
                    face_present: [true; 6],
                    face_uvs: [[0, 0, 16, 16]; 6],
                    face_cull: [false; 6],
                    texture_indices: [2; 6],
                    tint: [foliage_tint; 6],
                },
            ]),
        };
        let atlas = TerrainTextureAtlas {
            rects: vec![
                TerrainUvRect::UNIT,
                TerrainUvRect {
                    min: [0.25, 0.25],
                    max: [0.5, 0.5],
                },
                TerrainUvRect {
                    min: [0.5, 0.5],
                    max: [0.75, 0.75],
                },
            ],
            fallback_index: 0,
        };
        let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

        let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &atlas)
            .into_iter()
            .next()
            .unwrap();

        assert_eq!(mesh.vertices[0].uv, [0.25, 0.25]);
        assert_eq!(mesh.vertices[0].tint, grass_tint.as_shader_tint());
        assert_eq!(mesh.vertices[24].uv, [0.5, 0.5]);
        assert_eq!(mesh.vertices[24].tint, foliage_tint.as_shader_tint());
    }

    #[test]
    fn mesh_vertices_carry_terrain_light() {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
        cells[cell_index(1, 0, 2, 1)] =
            TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0)
                .with_light(TerrainLight { sky: 9, block: 6 });
        let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

        let mesh = build_opaque_chunk_mesh(&snapshot);

        assert_eq!(mesh.vertices[0].light, [6.0 / 15.0, 9.0 / 15.0]);
    }

    #[test]
    fn mesh_vertices_carry_face_tint() {
        let mut face_tints = [TerrainTint::WHITE; 6];
        face_tints[TerrainFace::Down.index()] = TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59);
        face_tints[TerrainFace::Up.index()] = TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18);
        let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
        cells[cell_index(1, 0, 2, 1)] =
            TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0).with_tint(face_tints);
        let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

        let mesh = build_opaque_chunk_mesh(&snapshot);

        assert_eq!(
            mesh.vertices[0].tint,
            TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59).as_shader_tint()
        );
        assert_eq!(
            mesh.vertices[4].tint,
            TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18).as_shader_tint()
        );
    }

    #[test]
    fn layer_builder_splits_opaque_and_cutout_meshes() {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
        cells[cell_index(1, 0, 2, 1)] =
            TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
        cells[cell_index(3, 0, 2, 1)] =
            TerrainCell::with_texture(2, TerrainMaterialClass::Cutout, 0);
        cells[cell_index(5, 0, 2, 1)] =
            TerrainCell::with_texture(3, TerrainMaterialClass::Translucent, 0);
        cells[cell_index(7, 0, 2, 1)] =
            TerrainCell::with_texture(4, TerrainMaterialClass::Fluid, 0);
        let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

        let layers =
            build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());
        assert_eq!(layers.source_sections, 1);
        assert_eq!(layers.opaque.len(), 1);
        assert_eq!(layers.cutout.len(), 1);
        assert_eq!(layers.translucent.len(), 1);
        assert_eq!(layers.opaque[0].opaque_faces, 6);
        assert_eq!(layers.opaque[0].cutout_faces, 0);
        assert_eq!(layers.opaque[0].translucent_faces, 0);
        assert_eq!(layers.cutout[0].opaque_faces, 0);
        assert_eq!(layers.cutout[0].cutout_faces, 6);
        assert_eq!(layers.cutout[0].translucent_faces, 0);
        assert_eq!(layers.translucent[0].opaque_faces, 0);
        assert_eq!(layers.translucent[0].cutout_faces, 0);
        assert_eq!(layers.translucent[0].translucent_faces, 12);
    }

    fn single_block_snapshot(
        chunk_x: i32,
        chunk_z: i32,
        x: usize,
        y: usize,
        z: usize,
    ) -> TerrainChunkSnapshot {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
        cells[cell_index(x, y, z, 1)] =
            TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
        TerrainChunkSnapshot::new(chunk_x, chunk_z, 0, 1, cells)
    }

    fn single_fluid_snapshot(
        chunk_x: i32,
        chunk_z: i32,
        x: usize,
        y: usize,
        z: usize,
    ) -> TerrainChunkSnapshot {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
        cells[cell_index(x, y, z, 1)] =
            TerrainCell::with_texture(86, TerrainMaterialClass::Fluid, 0);
        TerrainChunkSnapshot::new(chunk_x, chunk_z, 0, 1, cells)
    }

    fn slab_box_shape() -> TerrainRenderShape {
        let mut face_uvs = [[0, 0, 16, 16]; 6];
        face_uvs[TerrainFace::North.index()] = [0, 8, 16, 16];
        face_uvs[TerrainFace::South.index()] = [0, 8, 16, 16];
        face_uvs[TerrainFace::West.index()] = [0, 8, 16, 16];
        face_uvs[TerrainFace::East.index()] = [0, 8, 16, 16];
        let mut face_cull = [true; 6];
        face_cull[TerrainFace::Up.index()] = false;
        TerrainRenderShape::Box {
            from: [0, 0, 0],
            to: [16, 8, 16],
            face_present: [true; 6],
            face_uvs,
            face_cull,
        }
    }

    fn fluid_box_shape(height: u8) -> TerrainRenderShape {
        let mut face_uvs = [[0, 0, 16, 16]; 6];
        let side_v0 = 16 - height;
        face_uvs[TerrainFace::North.index()] = [0, side_v0, 16, 16];
        face_uvs[TerrainFace::South.index()] = [0, side_v0, 16, 16];
        face_uvs[TerrainFace::West.index()] = [0, side_v0, 16, 16];
        face_uvs[TerrainFace::East.index()] = [0, side_v0, 16, 16];
        TerrainRenderShape::Box {
            from: [0, 0, 0],
            to: [16, height, 16],
            face_present: [true; 6],
            face_uvs,
            face_cull: [true; 6],
        }
    }
}
