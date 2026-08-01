use serde::{Deserialize, Serialize};

mod face_occlusion_26_1;
pub(crate) mod mesh;

use face_occlusion_26_1::VANILLA_FACE_OCCLUSION_MASKS;

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
                for cross in crosses {
                    cross.face_transparency = [TerrainTransparency::OPAQUE; 6];
                }
            }
            Self::Box {
                face_transparency, ..
            } => *face_transparency = [TerrainTransparency::OPAQUE; 6],
            Self::Boxes(boxes) => {
                for model_box in boxes {
                    model_box.face_transparency = [TerrainTransparency::OPAQUE; 6];
                }
            }
            Self::Quads(quads) => {
                for quad in quads {
                    quad.transparency = TerrainTransparency::OPAQUE;
                }
            }
        }
    }
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
    /// Exact loaded-empty-section input. Invisible non-air blocks remain false.
    #[serde(default)]
    pub is_air: bool,
    /// Exact Vanilla 26.1 `BlockState.isSolidRender()` input for `VisGraph`.
    #[serde(default)]
    pub solid_render: bool,
    /// Exact Vanilla `BlockState.canOcclude()`. A full-cube render model is not
    /// necessarily an occluder (`TransparentBlock` and `LeavesBlock` use
    /// `noOcclusion`).
    #[serde(default)]
    pub can_occlude: bool,
    /// Exact Vanilla `getFaceOcclusionShape` palette indices in
    /// Down/Up/North/South/West/East order.
    #[serde(default)]
    pub face_occlusion_shape_indices: [u8; 6],
    /// Exact `BlockState.isViewBlocking()` input for Vanilla's smooth block
    /// model lighting corner selection.
    #[serde(default)]
    pub view_blocking: bool,
    /// Exact `BlockState.getLightDampening()` input paired with
    /// `view_blocking` by Vanilla's `BlockModelLighter`.
    #[serde(default)]
    pub light_dampening: u8,
    /// Exact `BlockState.getShadeBrightness()` encoded on 0..=255.
    #[serde(default = "full_shade_brightness")]
    pub shade_brightness: u8,
    pub fluid: Option<TerrainFluid>,
    pub fluid_texture_indices: [u32; 6],
    pub fluid_overlay_texture_index: Option<u32>,
    pub fluid_overlay_neighbor: bool,
    pub fluid_tint: [TerrainTint; 6],
    pub texture_indices: [u32; 6],
    pub render_shape: TerrainRenderShape,
    pub ambient_occlusion: bool,
    pub light: TerrainLight,
    pub tint: [TerrainTint; 6],
    pub face_transparency: [TerrainTransparency; 6],
    #[serde(default)]
    pub skip_rendering: TerrainSkipRendering,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainSkipRendering {
    pub same_block_key: u64,
    pub same_block_culls_all_faces: bool,
    #[serde(default)]
    pub same_block_culls_vertical_faces: bool,
    #[serde(default)]
    pub culls_with_leaves: bool,
    pub iron_bars_block: bool,
    pub bars_tag: bool,
    pub north: bool,
    pub south: bool,
    pub west: bool,
    pub east: bool,
    #[serde(default)]
    pub connection_property_mask: u8,
}

const fn full_shade_brightness() -> u8 {
    255
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
    pub const NO_BLOCK_STATE_ID: i32 = -1;

    pub const EMPTY: Self = Self {
        block_state_id: Self::NO_BLOCK_STATE_ID,
        material: TerrainMaterialClass::Empty,
        is_air: true,
        solid_render: false,
        can_occlude: false,
        face_occlusion_shape_indices: [0; 6],
        view_blocking: false,
        light_dampening: 0,
        shade_brightness: 255,
        fluid: None,
        fluid_texture_indices: [0; 6],
        fluid_overlay_texture_index: None,
        fluid_overlay_neighbor: false,
        fluid_tint: [TerrainTint::WHITE; 6],
        texture_indices: [0; 6],
        render_shape: TerrainRenderShape::Cube,
        ambient_occlusion: true,
        light: TerrainLight::FULL_BRIGHT,
        tint: [TerrainTint::WHITE; 6],
        face_transparency: [TerrainTransparency::OPAQUE; 6],
        skip_rendering: TerrainSkipRendering::NONE,
    };

    pub fn with_texture(
        block_state_id: i32,
        material: TerrainMaterialClass,
        texture_index: u32,
    ) -> Self {
        Self {
            block_state_id,
            material,
            is_air: matches!(material, TerrainMaterialClass::Empty),
            solid_render: material.occludes_terrain(),
            can_occlude: material.occludes_terrain(),
            face_occlusion_shape_indices: if material.occludes_terrain() {
                [1; 6]
            } else {
                [0; 6]
            },
            view_blocking: false,
            light_dampening: 0,
            shade_brightness: 255,
            fluid: None,
            fluid_texture_indices: [texture_index; 6],
            fluid_overlay_texture_index: None,
            fluid_overlay_neighbor: false,
            fluid_tint: [TerrainTint::WHITE; 6],
            texture_indices: [texture_index; 6],
            render_shape: TerrainRenderShape::Cube,
            ambient_occlusion: true,
            light: TerrainLight::FULL_BRIGHT,
            tint: [TerrainTint::WHITE; 6],
            face_transparency: [TerrainTransparency::OPAQUE; 6],
            skip_rendering: TerrainSkipRendering::NONE,
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
            is_air: matches!(material, TerrainMaterialClass::Empty),
            solid_render: material.occludes_terrain(),
            can_occlude: material.occludes_terrain(),
            face_occlusion_shape_indices: if material.occludes_terrain() {
                [1; 6]
            } else {
                [0; 6]
            },
            view_blocking: false,
            light_dampening: 0,
            shade_brightness: 255,
            fluid: None,
            fluid_texture_indices: [texture_index; 6],
            fluid_overlay_texture_index: None,
            fluid_overlay_neighbor: false,
            fluid_tint: [TerrainTint::WHITE; 6],
            texture_indices: [texture_index; 6],
            render_shape,
            ambient_occlusion: true,
            light: TerrainLight::FULL_BRIGHT,
            tint: [TerrainTint::WHITE; 6],
            face_transparency: [TerrainTransparency::OPAQUE; 6],
            skip_rendering: TerrainSkipRendering::NONE,
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

    pub fn with_fluid_overlay_texture(mut self, texture_index: u32) -> Self {
        self.fluid_overlay_texture_index = Some(texture_index);
        self
    }

    pub fn with_fluid_overlay_neighbor(mut self) -> Self {
        self.fluid_overlay_neighbor = true;
        self
    }

    pub fn with_can_occlude(mut self, can_occlude: bool) -> Self {
        self.can_occlude = can_occlude;
        self
    }

    pub fn with_face_occlusion_shape_indices(mut self, indices: [u8; 6]) -> Self {
        self.face_occlusion_shape_indices = indices;
        self
    }

    pub fn with_ambient_light_facts(
        mut self,
        view_blocking: bool,
        light_dampening: u8,
        shade_brightness: u8,
    ) -> Self {
        self.view_blocking = view_blocking;
        self.light_dampening = light_dampening;
        self.shade_brightness = shade_brightness;
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

    pub fn with_skip_rendering(mut self, skip_rendering: TerrainSkipRendering) -> Self {
        self.skip_rendering = skip_rendering;
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
    VANILLA_FACE_OCCLUSION_MASKS
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
    neighbor: &TerrainCell,
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TerrainSectionMesh {
    pub section_y: i32,
    pub mesh: TerrainMesh,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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

/// Builds only `target` while retaining cross-chunk lookup against its loaded
/// neighborhood. Native's incremental terrain cache uses this to avoid
/// re-meshing unchanged neighbors after a localized world revision.
pub fn build_terrain_chunk_mesh_layers_with_atlas_and_camera<'a>(
    target: &'a TerrainChunkSnapshot,
    neighborhood: impl IntoIterator<Item = &'a TerrainChunkSnapshot>,
    atlas: &TerrainTextureAtlas,
    camera_position: [f32; 3],
) -> TerrainMeshLayers {
    let lookup = mesh::TerrainChunkLookup::with_target(target, neighborhood);
    let opaque = mesh::build_chunk_mesh_with_lookup(
        target,
        &lookup,
        atlas,
        mesh::TerrainMeshMode::OpaqueOnly,
    );
    let cutout = mesh::build_chunk_mesh_with_lookup(
        target,
        &lookup,
        atlas,
        mesh::TerrainMeshMode::CutoutOnly,
    );
    let mut translucent = mesh::build_chunk_mesh_with_lookup(
        target,
        &lookup,
        atlas,
        mesh::TerrainMeshMode::TranslucentOnly,
    );
    mesh::sort_translucent_quads_by_distance(&mut translucent, camera_position);

    TerrainMeshLayers {
        opaque: vec![opaque],
        cutout: vec![cutout],
        translucent: vec![translucent],
        source_sections: target.height.div_ceil(16),
    }
}

/// Builds one independently resident mesh per chunk section and render layer.
/// Neighbor lookup remains chunk-wide, so faces at section and chunk boundaries
/// retain the same occlusion and lighting inputs as the legacy column mesh.
pub fn build_terrain_chunk_section_mesh_layers_with_atlas_and_camera<'a>(
    target: &'a TerrainChunkSnapshot,
    neighborhood: impl IntoIterator<Item = &'a TerrainChunkSnapshot>,
    atlas: &TerrainTextureAtlas,
    camera_position: [f32; 3],
) -> TerrainSectionMeshLayers {
    let section_ys = (0..target.height)
        .step_by(16)
        .map(|start_y| (target.min_y + start_y as i32).div_euclid(16))
        .collect::<Vec<_>>();
    build_terrain_chunk_section_subset_mesh_layers_with_atlas_and_camera(
        target,
        neighborhood,
        atlas,
        camera_position,
        &section_ys,
    )
}

/// Builds only the named sections of one chunk while retaining the exact same
/// cross-chunk lookup as a full chunk build. Invalid/out-of-height section ids
/// are ignored; callers normally pass the typed world revision rows.
pub fn build_terrain_chunk_section_subset_mesh_layers_with_atlas_and_camera<'a>(
    target: &'a TerrainChunkSnapshot,
    neighborhood: impl IntoIterator<Item = &'a TerrainChunkSnapshot>,
    atlas: &TerrainTextureAtlas,
    camera_position: [f32; 3],
    section_ys: &[i32],
) -> TerrainSectionMeshLayers {
    let lookup = mesh::TerrainChunkLookup::with_target(target, neighborhood);
    let opaque = mesh::build_chunk_section_subset_meshes_with_lookup(
        target,
        &lookup,
        atlas,
        mesh::TerrainMeshMode::OpaqueOnly,
        section_ys,
    );
    let cutout = mesh::build_chunk_section_subset_meshes_with_lookup(
        target,
        &lookup,
        atlas,
        mesh::TerrainMeshMode::CutoutOnly,
        section_ys,
    );
    let mut translucent = mesh::build_chunk_section_subset_meshes_with_lookup(
        target,
        &lookup,
        atlas,
        mesh::TerrainMeshMode::TranslucentOnly,
        section_ys,
    );
    for section in &mut translucent {
        mesh::sort_translucent_quads_by_distance(&mut section.mesh, camera_position);
    }
    let visibility = section_ys
        .iter()
        .copied()
        .filter(|section_y| section_is_in_snapshot(target, *section_y))
        .map(|section_y| {
            let has_renderable_layers = [&opaque, &cutout, &translucent].into_iter().any(|layer| {
                layer.iter().any(|section| {
                    section.section_y == section_y
                        && !section.mesh.vertices.is_empty()
                        && !section.mesh.indices.is_empty()
                })
            });
            resolve_section_visibility(target, section_y, has_renderable_layers)
        })
        .collect();

    TerrainSectionMeshLayers {
        source_sections: opaque.len(),
        opaque,
        cutout,
        translucent,
        visibility,
    }
}

fn section_is_in_snapshot(target: &TerrainChunkSnapshot, section_y: i32) -> bool {
    let start_y = section_y.saturating_mul(16).saturating_sub(target.min_y);
    start_y >= 0 && start_y < target.height as i32
}

/// Vanilla `VisGraph.resolve`, including its `< 256` opaque fast path.
fn resolve_section_visibility(
    target: &TerrainChunkSnapshot,
    section_y: i32,
    has_renderable_layers: bool,
) -> TerrainSectionVisibility {
    const SECTION_EDGE: usize = 16;
    const CELL_COUNT: usize = SECTION_EDGE * SECTION_EDGE * SECTION_EDGE;
    let section_min_y = section_y.saturating_mul(16);
    let mut blocked = [false; CELL_COUNT];
    let mut opaque_count = 0;
    let mut loaded_empty = true;
    for local_y in 0..SECTION_EDGE {
        for local_z in 0..SECTION_EDGE {
            for local_x in 0..SECTION_EDGE {
                let snapshot_y = section_min_y
                    .saturating_add(local_y as i32)
                    .saturating_sub(target.min_y);
                let cell = target.cell(local_x as i32, snapshot_y, local_z as i32);
                let solid_render = cell.is_some_and(|cell| cell.solid_render);
                loaded_empty &= cell.is_none_or(|cell| cell.is_air);
                let index = section_cell_index(local_x, local_y, local_z);
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
mod section_mesh_tests {
    use super::*;

    #[test]
    fn section_partition_preserves_column_mesh_geometry_and_section_identity() {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 32 * 16];
        for y in [0, 16] {
            let index = mesh::cell_index(1, y, 1, 32);
            cells[index] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
        }
        let snapshot = TerrainChunkSnapshot::new(3, -2, 0, 32, cells);
        let atlas = TerrainTextureAtlas::unit();
        let column = build_terrain_chunk_mesh_layers_with_atlas_and_camera(
            &snapshot,
            std::iter::empty(),
            &atlas,
            [0.0, 0.0, 0.0],
        );
        let sections = build_terrain_chunk_section_mesh_layers_with_atlas_and_camera(
            &snapshot,
            std::iter::empty(),
            &atlas,
            [0.0, 0.0, 0.0],
        );

        assert_eq!(
            sections
                .opaque
                .iter()
                .map(|section| section.section_y)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(
            sections
                .opaque
                .iter()
                .map(|section| section.mesh.vertices.len())
                .sum::<usize>(),
            column.opaque[0].vertices.len()
        );
        assert_eq!(
            sections
                .opaque
                .iter()
                .map(|section| section.mesh.indices.len())
                .sum::<usize>(),
            column.opaque[0].indices.len()
        );
        assert_eq!(
            sections
                .opaque
                .iter()
                .map(|section| section.mesh.opaque_faces)
                .sum::<usize>(),
            column.opaque[0].opaque_faces
        );
        assert_eq!(sections.source_sections, column.source_sections);
    }

    #[test]
    fn section_subset_matches_the_same_full_build_sections() {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 48 * 16];
        for y in [1, 17, 33] {
            let index = mesh::cell_index(1, y, 1, 48);
            cells[index] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
        }
        let snapshot = TerrainChunkSnapshot::new(3, -2, 0, 48, cells);
        let atlas = TerrainTextureAtlas::unit();
        let full = build_terrain_chunk_section_mesh_layers_with_atlas_and_camera(
            &snapshot,
            std::iter::empty(),
            &atlas,
            [0.0, 0.0, 0.0],
        );
        let subset = build_terrain_chunk_section_subset_mesh_layers_with_atlas_and_camera(
            &snapshot,
            std::iter::empty(),
            &atlas,
            [0.0, 0.0, 0.0],
            &[0, 2],
        );

        assert_eq!(subset.source_sections, 2);
        assert_eq!(
            subset.opaque,
            vec![full.opaque[0].clone(), full.opaque[2].clone()]
        );
        assert_eq!(
            subset.cutout,
            vec![full.cutout[0].clone(), full.cutout[2].clone()]
        );
        assert_eq!(
            subset.translucent,
            vec![full.translucent[0].clone(), full.translucent[2].clone()]
        );
    }

    #[test]
    fn section_visibility_sparse_and_empty_sections_are_all_visible() {
        let empty = TerrainChunkSnapshot::new(0, 0, 0, 16, vec![TerrainCell::EMPTY; 16 * 16 * 16]);
        let empty_layers = build_terrain_chunk_section_mesh_layers_with_atlas_and_camera(
            &empty,
            std::iter::empty(),
            &TerrainTextureAtlas::unit(),
            [0.0; 3],
        );
        let empty_visibility = empty_layers.visibility[0];
        assert!(!empty_visibility.has_renderable_layers);
        for from in TerrainFace::ALL {
            for to in TerrainFace::ALL {
                assert!(empty_visibility.faces_can_see_each_other(from, to));
            }
        }

        let mut sparse_cells = vec![TerrainCell::EMPTY; 16 * 16 * 16];
        for index in 0..255 {
            sparse_cells[index] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
        }
        let sparse = TerrainChunkSnapshot::new(0, 0, 0, 16, sparse_cells);
        let sparse_layers = build_terrain_chunk_section_mesh_layers_with_atlas_and_camera(
            &sparse,
            std::iter::empty(),
            &TerrainTextureAtlas::unit(),
            [0.0; 3],
        );
        assert!(sparse_layers.visibility[0]
            .faces_can_see_each_other(TerrainFace::West, TerrainFace::East));
    }

    #[test]
    fn section_visibility_fully_solid_section_blocks_every_face_pair() {
        let full = TerrainChunkSnapshot::new(
            0,
            0,
            0,
            16,
            vec![TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0); 16 * 16 * 16],
        );
        let layers = build_terrain_chunk_section_mesh_layers_with_atlas_and_camera(
            &full,
            std::iter::empty(),
            &TerrainTextureAtlas::unit(),
            [0.0; 3],
        );
        let visibility = layers.visibility[0];
        assert!(visibility.has_renderable_layers);
        for from in TerrainFace::ALL {
            for to in TerrainFace::ALL {
                assert!(!visibility.faces_can_see_each_other(from, to));
            }
        }
    }

    #[test]
    fn section_visibility_solid_wall_separates_west_from_east() {
        let mut cells = vec![TerrainCell::EMPTY; 16 * 16 * 16];
        for y in 0..16 {
            for z in 0..16 {
                cells[mesh::cell_index(8, y, z, 16)] =
                    TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
            }
        }
        let split = TerrainChunkSnapshot::new(0, 0, 0, 16, cells);
        let layers = build_terrain_chunk_section_mesh_layers_with_atlas_and_camera(
            &split,
            std::iter::empty(),
            &TerrainTextureAtlas::unit(),
            [0.0; 3],
        );
        let visibility = layers.visibility[0];
        assert!(!visibility.faces_can_see_each_other(TerrainFace::West, TerrainFace::East));
        assert!(visibility.faces_can_see_each_other(TerrainFace::West, TerrainFace::North));
        assert!(visibility.faces_can_see_each_other(TerrainFace::East, TerrainFace::South));
    }
}
