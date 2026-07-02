use serde::{Deserialize, Serialize};

mod mesh;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TerrainRenderShape {
    Cube,
    Cross {
        shade: bool,
        light_emission: u8,
    },
    Crosses(Vec<TerrainCross>),
    Box {
        from: [u8; 3],
        to: [u8; 3],
        face_present: [bool; 6],
        face_uvs: [[u8; 4]; 6],
        face_uv_rotations: [u8; 6],
        face_shade: [bool; 6],
        face_light_emission: [u8; 6],
        face_cull: [Option<TerrainFace>; 6],
        face_transparency: [TerrainTransparency; 6],
    },
    Boxes(Vec<TerrainBox>),
    Quads(Vec<TerrainQuad>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainBox {
    pub from: [u8; 3],
    pub to: [u8; 3],
    pub face_present: [bool; 6],
    pub face_uvs: [[u8; 4]; 6],
    pub face_uv_rotations: [u8; 6],
    pub face_shade: [bool; 6],
    pub face_light_emission: [u8; 6],
    pub face_cull: [Option<TerrainFace>; 6],
    pub texture_indices: [u32; 6],
    pub tint: [TerrainTint; 6],
    pub face_transparency: [TerrainTransparency; 6],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainCross {
    pub texture_indices: [u32; 6],
    pub tint: [TerrainTint; 6],
    pub face_transparency: [TerrainTransparency; 6],
    pub shade: bool,
    pub light_emission: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TerrainQuad {
    pub corners: [[f32; 3]; 4],
    pub normal: [f32; 3],
    pub uvs: [[f32; 2]; 4],
    pub cull: Option<TerrainFace>,
    pub texture_index: u32,
    pub tint: TerrainTint,
    pub transparency: TerrainTransparency,
    pub shade: bool,
    pub light_emission: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainFluidKind {
    Water,
    Lava,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainFluid {
    pub kind: TerrainFluidKind,
    pub amount: u8,
    pub falling: bool,
}

impl TerrainFluid {
    pub fn new(kind: TerrainFluidKind, amount: u8, falling: bool) -> Self {
        Self {
            kind,
            amount: amount.clamp(1, 8),
            falling,
        }
    }

    fn own_height(self) -> f32 {
        self.amount as f32 / 9.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainCell {
    pub block_state_id: i32,
    pub material: TerrainMaterialClass,
    pub fluid: Option<TerrainFluid>,
    pub fluid_texture_indices: [u32; 6],
    pub fluid_tint: [TerrainTint; 6],
    pub texture_indices: [u32; 6],
    pub render_shape: TerrainRenderShape,
    pub ambient_occlusion: bool,
    pub light: TerrainLight,
    pub tint: [TerrainTint; 6],
    pub face_transparency: [TerrainTransparency; 6],
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

    pub(crate) fn as_shader_tint(self) -> [f32; 3] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
        ]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainTransparency {
    pub has_transparent: bool,
    pub has_translucent: bool,
}

impl TerrainTransparency {
    pub const OPAQUE: Self = Self {
        has_transparent: false,
        has_translucent: false,
    };

    pub const TRANSLUCENT: Self = Self {
        has_transparent: false,
        has_translucent: true,
    };

    pub fn or(self, other: Self) -> Self {
        Self {
            has_transparent: self.has_transparent || other.has_transparent,
            has_translucent: self.has_translucent || other.has_translucent,
        }
    }
}

impl TerrainCell {
    pub const EMPTY: Self = Self {
        block_state_id: 0,
        material: TerrainMaterialClass::Empty,
        fluid: None,
        fluid_texture_indices: [0; 6],
        fluid_tint: [TerrainTint::WHITE; 6],
        texture_indices: [0; 6],
        render_shape: TerrainRenderShape::Cube,
        ambient_occlusion: true,
        light: TerrainLight::FULL_BRIGHT,
        tint: [TerrainTint::WHITE; 6],
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    };

    pub fn with_texture(
        block_state_id: i32,
        material: TerrainMaterialClass,
        texture_index: u32,
    ) -> Self {
        Self {
            block_state_id,
            material,
            fluid: None,
            fluid_texture_indices: [texture_index; 6],
            fluid_tint: [TerrainTint::WHITE; 6],
            texture_indices: [texture_index; 6],
            render_shape: TerrainRenderShape::Cube,
            ambient_occlusion: true,
            light: TerrainLight::FULL_BRIGHT,
            tint: [TerrainTint::WHITE; 6],
            face_transparency: [TerrainTransparency::OPAQUE; 6],
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
            fluid: None,
            fluid_texture_indices: [texture_index; 6],
            fluid_tint: [TerrainTint::WHITE; 6],
            texture_indices: [texture_index; 6],
            render_shape,
            ambient_occlusion: true,
            light: TerrainLight::FULL_BRIGHT,
            tint: [TerrainTint::WHITE; 6],
            face_transparency: [TerrainTransparency::OPAQUE; 6],
        }
    }

    pub fn with_light(mut self, light: TerrainLight) -> Self {
        self.light = light;
        self
    }

    pub fn with_fluid(mut self, fluid: TerrainFluid) -> Self {
        self.fluid = Some(fluid);
        self.fluid_texture_indices = self.texture_indices;
        self.fluid_tint = self.tint;
        self
    }

    pub fn with_fluid_render_data(
        mut self,
        fluid: TerrainFluid,
        texture_indices: [u32; 6],
        tint: [TerrainTint; 6],
    ) -> Self {
        self.fluid = Some(fluid);
        self.fluid_texture_indices = texture_indices;
        self.fluid_tint = tint;
        self
    }

    pub fn with_ambient_occlusion(mut self, ambient_occlusion: bool) -> Self {
        self.ambient_occlusion = ambient_occlusion;
        self
    }

    pub fn with_tint(mut self, tint: [TerrainTint; 6]) -> Self {
        self.tint = tint;
        self
    }

    pub fn with_face_transparency(mut self, face_transparency: [TerrainTransparency; 6]) -> Self {
        self.face_transparency = face_transparency;
        self
    }
}

/// Per-dimension block/fluid face shading, mirroring vanilla
/// `net.minecraft.world.level.CardinalLighting` selected by
/// `DimensionType.cardinalLightType`. Only `DEFAULT` and `NETHER` exist in
/// 26.1; every built-in dimension except the Nether (overworld / end / caves)
/// uses `DEFAULT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TerrainCardinalLighting {
    /// `CardinalLighting.DEFAULT = (0.5, 1.0, 0.8, 0.8, 0.6, 0.6)`.
    #[default]
    Default,
    /// `CardinalLighting.NETHER = (0.9, 0.9, 0.8, 0.8, 0.6, 0.6)`.
    Nether,
}

impl TerrainCardinalLighting {
    /// `[down, up, north, south, west, east]` multipliers.
    const fn values(self) -> [f32; 6] {
        match self {
            Self::Default => [0.5, 1.0, 0.8, 0.8, 0.6, 0.6],
            Self::Nether => [0.9, 0.9, 0.8, 0.8, 0.6, 0.6],
        }
    }

    fn by_face(self, face: TerrainFace) -> f32 {
        let [down, up, north, south, west, east] = self.values();
        match face {
            TerrainFace::Down => down,
            TerrainFace::Up => up,
            TerrainFace::North => north,
            TerrainFace::South => south,
            TerrainFace::West => west,
            TerrainFace::East => east,
        }
    }

    fn up(self) -> f32 {
        self.values()[1]
    }

    /// Vanilla `BlockModelLighter`: a shaded face uses `byFace(direction)`, an
    /// unshaded face uses `up()`.
    pub fn shade(self, enabled: bool, face: TerrainFace) -> f32 {
        if enabled {
            self.by_face(face)
        } else {
            self.up()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainChunkSnapshot {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub min_y: i32,
    pub height: usize,
    pub cells: Vec<TerrainCell>,
    #[serde(default)]
    pub cardinal_lighting: TerrainCardinalLighting,
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
            cardinal_lighting: TerrainCardinalLighting::Default,
        }
    }

    /// Selects the vanilla `CardinalLighting` for this chunk's dimension
    /// (`DEFAULT` for every dimension except the Nether).
    pub fn with_cardinal_lighting(mut self, cardinal_lighting: TerrainCardinalLighting) -> Self {
        self.cardinal_lighting = cardinal_lighting;
        self
    }

    pub fn cell(&self, x: i32, y: i32, z: i32) -> Option<&TerrainCell> {
        if !(0..16).contains(&x) || !(0..self.height as i32).contains(&y) || !(0..16).contains(&z) {
            return None;
        }
        self.cells.get(mesh::cell_index(
            x as usize,
            y as usize,
            z as usize,
            self.height,
        ))
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
    pub const ALL: [Self; 6] = [
        Self::Down,
        Self::Up,
        Self::North,
        Self::South,
        Self::West,
        Self::East,
    ];

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
    pub shade: f32,
    pub ambient_occlusion: f32,
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

    pub(crate) fn map(self, uv: [f32; 2]) -> [f32; 2] {
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

/// Bakes a block's render shape into standalone item-model quads (the block-item path of the item-model
/// renderer). `texture_indices` / `tint` are the per-face atlas data the native `block_render_data`
/// produces alongside `shape`; the returned [`crate::ItemModelQuad`]s carry vanilla `0..=16` corners and
/// atlas-absolute UVs, ready for [`crate::bake_item_model_mesh`] under a display/world transform. Every
/// present face renders (no chunk culling / lighting); foliage `Cross` shapes are never items and bake
/// empty.
pub fn bake_block_item_quads(
    shape: &TerrainRenderShape,
    texture_indices: [u32; 6],
    tint: [TerrainTint; 6],
    atlas: &TerrainTextureAtlas,
) -> Vec<crate::ItemModelQuad> {
    mesh::bake_block_item_quads(shape, texture_indices, tint, atlas)
}

pub fn build_opaque_terrain_meshes(snapshots: &[TerrainChunkSnapshot]) -> Vec<TerrainMesh> {
    let atlas = TerrainTextureAtlas::unit();
    build_opaque_terrain_meshes_with_atlas(snapshots, &atlas)
}

pub fn build_opaque_terrain_meshes_with_atlas(
    snapshots: &[TerrainChunkSnapshot],
    atlas: &TerrainTextureAtlas,
) -> Vec<TerrainMesh> {
    let lookup = mesh::TerrainChunkLookup::new(snapshots);
    snapshots
        .iter()
        .map(|snapshot| {
            mesh::build_chunk_mesh_with_lookup(
                snapshot,
                &lookup,
                atlas,
                mesh::TerrainMeshMode::OpaqueOnly,
            )
        })
        .collect()
}

pub fn build_terrain_meshes_with_atlas(
    snapshots: &[TerrainChunkSnapshot],
    atlas: &TerrainTextureAtlas,
) -> Vec<TerrainMesh> {
    let lookup = mesh::TerrainChunkLookup::new(snapshots);
    snapshots
        .iter()
        .map(|snapshot| {
            mesh::build_chunk_mesh_with_lookup(
                snapshot,
                &lookup,
                atlas,
                mesh::TerrainMeshMode::OpaqueCutout,
            )
        })
        .collect()
}

pub fn build_terrain_mesh_layers_with_atlas(
    snapshots: &[TerrainChunkSnapshot],
    atlas: &TerrainTextureAtlas,
) -> TerrainMeshLayers {
    build_terrain_mesh_layers_with_atlas_and_camera(snapshots, atlas, [0.0, 0.0, 0.0])
}

pub fn build_terrain_mesh_layers_with_atlas_and_camera(
    snapshots: &[TerrainChunkSnapshot],
    atlas: &TerrainTextureAtlas,
    camera_position: [f32; 3],
) -> TerrainMeshLayers {
    let lookup = mesh::TerrainChunkLookup::new(snapshots);
    let source_sections = snapshots
        .iter()
        .map(|snapshot| snapshot.height.div_ceil(16))
        .sum();
    let opaque = snapshots
        .iter()
        .map(|snapshot| {
            mesh::build_chunk_mesh_with_lookup(
                snapshot,
                &lookup,
                atlas,
                mesh::TerrainMeshMode::OpaqueOnly,
            )
        })
        .collect();
    let cutout = snapshots
        .iter()
        .map(|snapshot| {
            mesh::build_chunk_mesh_with_lookup(
                snapshot,
                &lookup,
                atlas,
                mesh::TerrainMeshMode::CutoutOnly,
            )
        })
        .collect();
    let translucent = snapshots
        .iter()
        .map(|snapshot| {
            let mut mesh = mesh::build_chunk_mesh_with_lookup(
                snapshot,
                &lookup,
                atlas,
                mesh::TerrainMeshMode::TranslucentOnly,
            );
            mesh::sort_translucent_quads_by_distance(&mut mesh, camera_position);
            mesh
        })
        .collect();

    TerrainMeshLayers {
        opaque,
        cutout,
        translucent,
        source_sections,
    }
}
