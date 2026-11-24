use std::sync::Arc;

pub use bbb_render_types::TerrainLight;
pub use block_table::{LodSurfaceSample, TerrainBlockRenderTable};

use crate::render_core::gpu::TerrainGpuVertex;

pub(crate) mod block_table;
mod face_occlusion_26_1;
#[doc(hidden)]
pub mod fixture;
pub(crate) mod mesh;
mod snapshot;

use face_occlusion_26_1::vanilla_face_occlusion_masks;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainMaterialClass {
    Empty,
    Opaque,
    Cutout,
    Fluid,
    Translucent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TerrainRenderShape {
    Cube,
    Cross {
        shade: bool,
        light_emission: u8,
    },
    Crosses(Arc<[TerrainCross]>),
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
    Boxes(Arc<[TerrainBox]>),
    Quads(Arc<[TerrainQuad]>),
}

impl TerrainRenderShape {
    /// Routes every quad in this model through the solid terrain layer.
    ///
    /// Vanilla's fast-leaves path changes the output layer selected by
    /// `ModelBlockRenderer.forceOpaque`; keeping this operation on the shared
    /// render-shape value covers root faces and all nested model shapes from
    /// one owner.
    pub fn force_opaque(&mut self) {
        match self {
            Self::Cube | Self::Cross { .. } => {}
            Self::Crosses(crosses) => {
                for cross in Arc::make_mut(crosses) {
                    cross.face_transparency = [TerrainTransparency::OPAQUE; 6];
                }
            }
            Self::Box {
                face_transparency, ..
            } => *face_transparency = [TerrainTransparency::OPAQUE; 6],
            Self::Boxes(boxes) => {
                for model_box in Arc::make_mut(boxes) {
                    model_box.face_transparency = [TerrainTransparency::OPAQUE; 6];
                }
            }
            Self::Quads(quads) => {
                for quad in Arc::make_mut(quads) {
                    quad.transparency = TerrainTransparency::OPAQUE;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainCross {
    pub texture_indices: [u32; 6],
    pub tint: [TerrainTint; 6],
    pub face_transparency: [TerrainTransparency; 6],
    pub shade: bool,
    pub light_emission: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainFluidKind {
    Water,
    Lava,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TerrainSkipRendering {
    pub same_block_key: u64,
    pub same_block_culls_all_faces: bool,
    pub same_block_culls_vertical_faces: bool,
    pub culls_with_leaves: bool,
    pub iron_bars_block: bool,
    pub bars_tag: bool,
    pub north: bool,
    pub south: bool,
    pub west: bool,
    pub east: bool,
    pub connection_property_mask: u8,
}

impl TerrainSkipRendering {
    pub const NONE: Self = Self {
        same_block_key: 0,
        same_block_culls_all_faces: false,
        same_block_culls_vertical_faces: false,
        culls_with_leaves: false,
        iron_bars_block: false,
        bars_tag: false,
        north: false,
        south: false,
        west: false,
        east: false,
        connection_property_mask: 0,
    };

    fn is_same_block(self, neighbor: Self) -> bool {
        self.same_block_key != 0 && self.same_block_key == neighbor.same_block_key
    }

    fn is_connected(self, face: TerrainFace) -> bool {
        match face {
            TerrainFace::North => self.north,
            TerrainFace::South => self.south,
            TerrainFace::West => self.west,
            TerrainFace::East => self.east,
            TerrainFace::Down | TerrainFace::Up => false,
        }
    }

    fn has_connection_property(self, face: TerrainFace) -> bool {
        let bit = match face {
            TerrainFace::North => 0,
            TerrainFace::South => 1,
            TerrainFace::West => 2,
            TerrainFace::East => 3,
            TerrainFace::Down | TerrainFace::Up => return false,
        };
        self.connection_property_mask & (1 << bit) != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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

/// Per-dimension block/fluid face shading, mirroring vanilla
/// `net.minecraft.world.level.CardinalLighting` selected by
/// `DimensionType.cardinalLightType`. Only `DEFAULT` and `NETHER` exist in
/// 26.1; every built-in dimension except the Nether (overworld / end / caves)
/// uses `DEFAULT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

pub use snapshot::TerrainChunkSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn index(self) -> usize {
        match self {
            Self::Down => 0,
            Self::Up => 1,
            Self::North => 2,
            Self::South => 3,
            Self::West => 4,
            Self::East => 5,
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::North => Self::South,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
        }
    }

    pub(crate) fn is_horizontal(self) -> bool {
        matches!(self, Self::North | Self::South | Self::West | Self::East)
    }
}

fn face_occlusion_mask(index: u8) -> Option<[u64; 4]> {
    vanilla_face_occlusion_masks()
        .get(usize::from(index))
        .copied()
}

/// Exact `Block.shouldRenderFace` face-shape decision after
/// `BlockState.skipRendering` has returned false.
fn face_occlusion_culls(own_index: u8, occluder_index: u8) -> bool {
    let Some(occluder) = face_occlusion_mask(occluder_index) else {
        return false;
    };
    if occluder.iter().all(|word| *word == u64::MAX) {
        return true;
    }
    if occluder.iter().all(|word| *word == 0) {
        return false;
    }
    let Some(own) = face_occlusion_mask(own_index) else {
        return false;
    };
    if own.iter().all(|word| *word == 0) {
        return false;
    }
    own.into_iter()
        .zip(occluder)
        .all(|(own, occluder)| own & !occluder == 0)
}

fn fluid_face_required_mask(direction: TerrainFace, height: f32) -> [u64; 4] {
    if matches!(direction, TerrainFace::Down | TerrainFace::Up) {
        return [u64::MAX; 4];
    }
    let height_cells = ((height * 16.0 - 1.0e-6).ceil() as i32).clamp(0, 16) as usize;
    let mut required = [0; 4];
    for second_axis in 0..16 {
        for first_axis in 0..16 {
            let vertical_axis = match direction {
                TerrainFace::North | TerrainFace::South => second_axis,
                TerrainFace::West | TerrainFace::East => first_axis,
                TerrainFace::Down | TerrainFace::Up => unreachable!(),
            };
            if vertical_axis < height_cells {
                let bit = second_axis * 16 + first_axis;
                required[bit / 64] |= 1 << (bit % 64);
            }
        }
    }
    required
}

fn face_occlusion_covers_fluid_face(index: u8, direction: TerrainFace, height: f32) -> bool {
    // `Shapes.blockOccludes(Shapes.box(0, 0, 0, 1, height, 1), occluder,
    // UP)` has no boundary face until the fluid box reaches y=1. DOWN always
    // touches y=0, while horizontal faces cover only their current height.
    if matches!(direction, TerrainFace::Up) && height < 1.0 {
        return false;
    }
    let Some(occluder) = face_occlusion_mask(index) else {
        return false;
    };
    let required = fluid_face_required_mask(direction, height);
    required
        .into_iter()
        .zip(occluder)
        .all(|(required, occluder)| required & !occluder == 0)
}

/// Vanilla `FluidRenderer.isFaceOccludedBySelf` followed by
/// `isFaceOccludedByNeighbor`. Same-fluid suppression remains a distinct
/// fluid-state decision at the caller.
fn fluid_face_occlusion_culls(
    current_face_indices: [u8; 6],
    direction: TerrainFace,
    neighbor: &block_table::TerrainBlockState,
    height: f32,
) -> bool {
    if face_occlusion_covers_fluid_face(
        current_face_indices[direction.index()],
        direction.opposite(),
        1.0,
    ) {
        return true;
    }
    let neighbor_height = if matches!(direction, TerrainFace::Down) {
        0.888_888_9
    } else {
        height
    };
    face_occlusion_covers_fluid_face(
        neighbor.face_occlusion_shape_indices[direction.opposite().index()],
        direction,
        neighbor_height,
    )
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TerrainMesh {
    pub vertices: Vec<TerrainGpuVertex>,
    /// Opaque and cutout meshes in shared quad order omit the explicit index payload.
    pub shared_quad_indices: bool,
    pub indices: Vec<u32>,
    pub source_sections: usize,
    pub opaque_faces: usize,
    pub cutout_faces: usize,
    pub translucent_faces: usize,
    pub culled_faces: usize,
}

impl TerrainMesh {
    pub fn index_count(&self) -> usize {
        if self.shared_quad_indices {
            self.vertices.len() / 4 * 6
        } else {
            self.indices.len()
        }
    }
}

pub(crate) fn sequential_quad_indices(quad: u32) -> [u32; 6] {
    let base = quad * 4;
    [base, base + 1, base + 2, base, base + 2, base + 3]
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TerrainMeshLayers {
    pub opaque: Vec<TerrainMesh>,
    pub cutout: Vec<TerrainMesh>,
    pub translucent: Vec<TerrainMesh>,
    pub source_sections: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TerrainSectionMesh {
    pub section_y: i32,
    pub mesh: TerrainMesh,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TerrainSectionMeshLayers {
    pub opaque: Vec<TerrainSectionMesh>,
    pub cutout: Vec<TerrainSectionMesh>,
    pub translucent: Vec<TerrainSectionMesh>,
    /// One entry for every compiled section, including empty render layers.
    pub visibility: Vec<TerrainSectionVisibility>,
    pub source_sections: usize,
}

/// Vanilla `CompiledSectionMesh` visibility and renderability projection.
///
/// Bits use `TerrainFace::ALL` order in a row-major six-by-six matrix:
/// `from + to * 6`, matching `VisibilitySet`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TerrainSectionVisibility {
    pub section_y: i32,
    visible_faces: u64,
    pub has_renderable_layers: bool,
    pub loaded_empty: bool,
}

impl TerrainSectionVisibility {
    const FACE_COUNT: usize = 6;
    const ALL_VISIBLE_BITS: u64 = (1_u64 << (Self::FACE_COUNT * Self::FACE_COUNT)) - 1;

    pub fn all_visible(section_y: i32, has_renderable_layers: bool, loaded_empty: bool) -> Self {
        Self {
            section_y,
            visible_faces: Self::ALL_VISIBLE_BITS,
            has_renderable_layers,
            loaded_empty,
        }
    }

    pub fn none_visible(section_y: i32, has_renderable_layers: bool, loaded_empty: bool) -> Self {
        Self {
            section_y,
            visible_faces: 0,
            has_renderable_layers,
            loaded_empty,
        }
    }

    pub fn faces_can_see_each_other(self, from: TerrainFace, to: TerrainFace) -> bool {
        self.visible_faces & Self::face_bit(from, to) != 0
    }

    fn set_visible(&mut self, from: TerrainFace, to: TerrainFace) {
        self.visible_faces |= Self::face_bit(from, to) | Self::face_bit(to, from);
    }

    fn face_bit(from: TerrainFace, to: TerrainFace) -> u64 {
        1_u64 << (from.index() + to.index() * Self::FACE_COUNT)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainUvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl TerrainUvRect {
    pub const UNIT: Self = Self {
        min: [0.0, 0.0],
        max: [1.0, 1.0],
    };

    pub fn map(self, uv: [f32; 2]) -> [f32; 2] {
        [
            self.min[0] + (self.max[0] - self.min[0]) * uv[0],
            self.min[1] + (self.max[1] - self.min[1]) * uv[1],
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
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

    pub(crate) fn rect(&self, texture_index: u32) -> TerrainUvRect {
        self.rects
            .get(texture_index as usize)
            .copied()
            .or_else(|| self.rects.get(self.fallback_index as usize).copied())
            .unwrap_or(TerrainUvRect::UNIT)
    }
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

/// Bakes a `MovingBlockRenderState` model into standalone quads while retaining
/// the dimension's cardinal-lighting table. Geometry/model selection and biome
/// tint are resolved by native before this boundary; this function owns the
/// final per-face `CardinalLighting.getShade` multiplier.
pub fn bake_moving_block_quads(
    shape: &TerrainRenderShape,
    texture_indices: [u32; 6],
    tint: [TerrainTint; 6],
    face_transparency: [TerrainTransparency; 6],
    cardinal_lighting: TerrainCardinalLighting,
    atlas: &TerrainTextureAtlas,
) -> Vec<crate::ItemModelQuad> {
    mesh::bake_moving_block_quads(
        shape,
        texture_indices,
        tint,
        face_transparency,
        cardinal_lighting,
        atlas,
    )
}

/// Meshes the named sections of `target` in one pass per section, sampling
/// faces, light and biome tint across its 3x3 chunk `neighborhood`.
/// Out-of-height section ids are ignored.
pub fn build_terrain_section_meshes<'a>(
    target: &'a TerrainChunkSnapshot,
    neighborhood: impl IntoIterator<Item = &'a TerrainChunkSnapshot>,
    table: &'a TerrainBlockRenderTable,
    camera_position: [f32; 3],
    section_ys: &[i32],
) -> TerrainSectionMeshLayers {
    build_terrain_section_meshes_with_index_order(
        target,
        neighborhood,
        table,
        camera_position,
        section_ys,
        false,
    )
}

pub fn build_terrain_section_meshes_with_index_order<'a>(
    target: &'a TerrainChunkSnapshot,
    neighborhood: impl IntoIterator<Item = &'a TerrainChunkSnapshot>,
    table: &'a TerrainBlockRenderTable,
    camera_position: [f32; 3],
    section_ys: &[i32],
    shared_quad_indices: bool,
) -> TerrainSectionMeshLayers {
    let mut lookup = mesh::TerrainChunkLookup::new(target, neighborhood, table);
    let mut layers = TerrainSectionMeshLayers {
        opaque: Vec::with_capacity(section_ys.len()),
        cutout: Vec::with_capacity(section_ys.len()),
        translucent: Vec::with_capacity(section_ys.len()),
        visibility: Vec::with_capacity(section_ys.len()),
        source_sections: 0,
    };
    for section_y in section_ys.iter().copied() {
        lookup.prepare_section(section_y);
        let Some(mut meshes) =
            mesh::build_section_mesh_layers(target, &lookup, section_y, shared_quad_indices)
        else {
            continue;
        };
        let section_origin = [
            target.chunk_x.saturating_mul(16),
            section_y.saturating_mul(16),
            target.chunk_z.saturating_mul(16),
        ];
        let local_camera = std::array::from_fn(|axis| {
            (f64::from(camera_position[axis]) - f64::from(section_origin[axis])) as f32
        });
        mesh::sort_translucent_quads_by_distance(&mut meshes.translucent, local_camera);
        let has_renderable_layers = [&meshes.opaque, &meshes.cutout, &meshes.translucent]
            .into_iter()
            .any(|mesh| !mesh.vertices.is_empty() && mesh.index_count() != 0);
        layers.visibility.push(resolve_section_visibility(
            target,
            &lookup,
            table,
            section_y,
            has_renderable_layers,
        ));
        layers.opaque.push(TerrainSectionMesh {
            section_y,
            mesh: meshes.opaque,
        });
        layers.cutout.push(TerrainSectionMesh {
            section_y,
            mesh: meshes.cutout,
        });
        layers.translucent.push(TerrainSectionMesh {
            section_y,
            mesh: meshes.translucent,
        });
    }
    layers.source_sections = layers.opaque.len();
    layers
}

/// Vanilla `VisGraph.resolve`, including its `< 256` opaque fast path.
fn resolve_section_visibility(
    target: &TerrainChunkSnapshot,
    lookup: &mesh::TerrainChunkLookup<'_>,
    table: &TerrainBlockRenderTable,
    section_y: i32,
    has_renderable_layers: bool,
) -> TerrainSectionVisibility {
    if let Some(id) = target.uniform_block_state_id(section_y) {
        let state = table.state(id);
        return if state.solid_render {
            TerrainSectionVisibility::none_visible(section_y, has_renderable_layers, state.is_air)
        } else {
            TerrainSectionVisibility::all_visible(section_y, has_renderable_layers, state.is_air)
        };
    }
    const SECTION_EDGE: usize = 16;
    const CELL_COUNT: usize = SECTION_EDGE * SECTION_EDGE * SECTION_EDGE;
    let section_min_y = section_y.saturating_mul(16);
    let mut blocked = [false; CELL_COUNT];
    let mut opaque_count = 0;
    let mut loaded_empty = true;
    for local_y in 0..SECTION_EDGE {
        let world_y = section_min_y.saturating_add(local_y as i32);
        if !target.contains_world_y(world_y) {
            continue;
        }
        for local_z in 0..SECTION_EDGE {
            for local_x in 0..SECTION_EDGE {
                let index = section_cell_index(local_x, local_y, local_z);
                let state = lookup
                    .center_block_state_id(index)
                    .map(|id| table.state(id));
                let solid_render = state.is_some_and(|state| state.solid_render);
                loaded_empty &= state.is_none_or(|state| state.is_air);
                blocked[index] = solid_render;
                opaque_count += usize::from(solid_render);
            }
        }
    }

    if opaque_count < 256 {
        return TerrainSectionVisibility::all_visible(
            section_y,
            has_renderable_layers,
            loaded_empty,
        );
    }
    let mut visibility = TerrainSectionVisibility {
        section_y,
        visible_faces: 0,
        has_renderable_layers,
        loaded_empty,
    };
    if opaque_count == CELL_COUNT {
        return visibility;
    }

    for x in 0..SECTION_EDGE {
        for y in 0..SECTION_EDGE {
            for z in 0..SECTION_EDGE {
                if x != 0
                    && x != SECTION_EDGE - 1
                    && y != 0
                    && y != SECTION_EDGE - 1
                    && z != 0
                    && z != SECTION_EDGE - 1
                {
                    continue;
                }
                let start = section_cell_index(x, y, z);
                if blocked[start] {
                    continue;
                }
                let connected = flood_section_component(&mut blocked, start);
                for from in connected.iter().copied() {
                    for to in connected.iter().copied() {
                        visibility.set_visible(from, to);
                    }
                }
            }
        }
    }
    visibility
}

fn flood_section_component(blocked: &mut [bool; 4096], start: usize) -> Vec<TerrainFace> {
    use std::collections::VecDeque;

    let mut connected = [false; 6];
    let mut queue = VecDeque::from([start]);
    blocked[start] = true;
    while let Some(index) = queue.pop_front() {
        let x = index & 15;
        let z = (index >> 4) & 15;
        let y = (index >> 8) & 15;
        if x == 0 {
            connected[TerrainFace::West.index()] = true;
        } else if x == 15 {
            connected[TerrainFace::East.index()] = true;
        }
        if y == 0 {
            connected[TerrainFace::Down.index()] = true;
        } else if y == 15 {
            connected[TerrainFace::Up.index()] = true;
        }
        if z == 0 {
            connected[TerrainFace::North.index()] = true;
        } else if z == 15 {
            connected[TerrainFace::South.index()] = true;
        }
        for face in TerrainFace::ALL {
            let neighbor = match face {
                TerrainFace::Down if y > 0 => Some(index - 256),
                TerrainFace::Up if y < 15 => Some(index + 256),
                TerrainFace::North if z > 0 => Some(index - 16),
                TerrainFace::South if z < 15 => Some(index + 16),
                TerrainFace::West if x > 0 => Some(index - 1),
                TerrainFace::East if x < 15 => Some(index + 1),
                _ => None,
            };
            if let Some(neighbor) = neighbor {
                if !blocked[neighbor] {
                    blocked[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }
    }
    TerrainFace::ALL
        .into_iter()
        .filter(|face| connected[face.index()])
        .collect()
}

fn section_cell_index(x: usize, y: usize, z: usize) -> usize {
    x | (z << 4) | (y << 8)
}

#[cfg(test)]
mod section_mesh_tests;
