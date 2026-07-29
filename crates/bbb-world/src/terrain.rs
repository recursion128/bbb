use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{BlockPos, ChunkPos, PaletteKind};

mod generated_26_1;

#[cfg(test)]
use generated_26_1::VANILLA_BLOCK_COLLISION_SHAPE_OVERRIDE_BLOCKS;
use generated_26_1::{
    VANILLA_AIR_BLOCKS, VANILLA_AMETHYST_CLUSTER_BLOCKS, VANILLA_ANVIL_BLOCKS, VANILLA_BARS_BLOCKS,
    VANILLA_BED_BLOCKS, VANILLA_BLOCK_ANIMATE_TICK_BLOCKS, VANILLA_BLOCK_FRICTION_OVERRIDES,
    VANILLA_BLOCK_SPEED_FACTOR_OVERRIDES, VANILLA_BRUSHABLE_BLOCKS, VANILLA_BUTTON_BLOCKS,
    VANILLA_CAMPFIRE_BLOCKS, VANILLA_CANDLE_BLOCKS, VANILLA_CANDLE_CAKE_BLOCKS,
    VANILLA_CAULDRON_BLOCKS, VANILLA_CHAIN_BLOCKS, VANILLA_CHEST_BLOCKS,
    VANILLA_COPPER_GOLEM_STATUE_BLOCKS, VANILLA_COPPER_GRATE_BLOCKS, VANILLA_CROP_BLOCKS,
    VANILLA_DEAD_BUSH_AMBIENT_BLOCKS, VANILLA_DOOR_BLOCKS, VANILLA_DRY_GRASS_AMBIENT_BLOCKS,
    VANILLA_ENCHANTMENT_POWER_PROVIDER_BLOCKS, VANILLA_ENCHANTMENT_POWER_TRANSMITTER_BLOCKS,
    VANILLA_FALLING_BLOCKS, VANILLA_FALL_DAMAGE_RESETTING_BLOCKS, VANILLA_FENCE_BLOCKS,
    VANILLA_FENCE_GATE_BLOCKS, VANILLA_FIRE_BLOCKS, VANILLA_FIRE_FLAMMABLE_BLOCKS,
    VANILLA_FIXED_SOURCE_WATER_BLOCKS, VANILLA_FLOOR_SKULL_BLOCKS, VANILLA_FLOOR_TORCH_BLOCKS,
    VANILLA_FLOWER_POT_BLOCKS, VANILLA_HALF_TRANSPARENT_SKIP_RENDERING_BLOCKS,
    VANILLA_HANGING_SIGN_BLOCKS, VANILLA_IMPERMEABLE_BLOCKS, VANILLA_INVISIBLE_RENDER_SHAPE_BLOCKS,
    VANILLA_IRON_BARS_CLASS_BLOCKS, VANILLA_LANTERN_BLOCKS, VANILLA_LEAF_ANIMATE_PARTICLE_PROFILES,
    VANILLA_LEAVES_BLOCKS, VANILLA_LIGHTNING_ROD_BLOCKS, VANILLA_LOG_BLOCKS,
    VANILLA_NO_COLLISION_BLOCKS, VANILLA_NO_TERRAIN_PARTICLE_BLOCKS, VANILLA_ORE_BLOCKS,
    VANILLA_PALE_OAK_LOG_BLOCKS, VANILLA_PLANK_BLOCKS, VANILLA_PRESSURE_PLATE_BLOCKS,
    VANILLA_RAIL_BLOCKS, VANILLA_SAME_BLOCK_ALL_FACE_SKIP_RENDERING_BLOCKS,
    VANILLA_SAME_BLOCK_VERTICAL_FACE_SKIP_RENDERING_BLOCKS, VANILLA_SAPLING_BLOCKS,
    VANILLA_SHELF_BLOCKS, VANILLA_SHULKER_BOX_BLOCKS, VANILLA_SLAB_BLOCKS,
    VANILLA_SOUL_FIRE_BASE_BLOCKS, VANILLA_STAIR_BLOCKS, VANILLA_STANDING_SIGN_BLOCKS,
    VANILLA_TERRACOTTA_BLOCKS, VANILLA_TORCH_ANIMATE_PARTICLE_PROFILES, VANILLA_TRAPDOOR_BLOCKS,
    VANILLA_TRIGGERS_AMBIENT_DESERT_DRY_VEGETATION_BLOCK_SOUNDS,
    VANILLA_TRIGGERS_AMBIENT_DESERT_SAND_BLOCK_SOUNDS,
    VANILLA_TRIGGERS_AMBIENT_DRIED_GHAST_BLOCK_SOUNDS, VANILLA_WALL_BLOCKS,
    VANILLA_WALL_SIGN_BLOCKS, VANILLA_WALL_SKULL_BLOCKS, VANILLA_WALL_TORCH_BLOCKS,
    VANILLA_WOOL_CARPET_BLOCKS,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockProbe {
    pub pos: BlockPos,
    pub chunk: ChunkPos,
    pub local_x: u8,
    pub local_y: u8,
    pub local_z: u8,
    pub section_y: i32,
    pub section_index: usize,
    pub block_state_id: i32,
    pub block_name: Option<String>,
    pub block_properties: BTreeMap<String, String>,
    pub material: TerrainMaterialClass,
    pub fluid: Option<TerrainFluidState>,
    pub block_palette_kind: PaletteKind,
    pub block_palette_index: Option<usize>,
    pub biome_id: Option<i32>,
    pub biome_palette_kind: PaletteKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainMaterialClass {
    Empty,
    Invisible,
    Opaque,
    Cutout,
    Fluid,
    Translucent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainFluidKind {
    Water,
    Lava,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafParticleTint {
    None,
    FixedArgb(u32),
    /// Vanilla `ClientLevel.getClientLeafTintColor`, including the fixed
    /// spruce/birch sources and biome-blended foliage sources.
    ClientLeaf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeafAnimateParticleProfile {
    pub block_resource_id: &'static str,
    leaf_particle_chance_bits: u32,
    pub particle_resource_id: &'static str,
    pub tint: LeafParticleTint,
}

impl LeafAnimateParticleProfile {
    pub const fn new(
        block_resource_id: &'static str,
        leaf_particle_chance_bits: u32,
        particle_resource_id: &'static str,
        tint: LeafParticleTint,
    ) -> Self {
        Self {
            block_resource_id,
            leaf_particle_chance_bits,
            particle_resource_id,
            tint,
        }
    }

    pub const fn leaf_particle_chance(self) -> f32 {
        f32::from_bits(self.leaf_particle_chance_bits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TorchAnimateParticleProfile {
    pub block_resource_id: &'static str,
    pub particle_resource_id: &'static str,
    pub wall: bool,
}

impl TorchAnimateParticleProfile {
    pub const fn new(
        block_resource_id: &'static str,
        particle_resource_id: &'static str,
        wall: bool,
    ) -> Self {
        Self {
            block_resource_id,
            particle_resource_id,
            wall,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainFluidState {
    pub kind: TerrainFluidKind,
    pub amount: u8,
    pub falling: bool,
}

impl TerrainFluidState {
    pub fn new(kind: TerrainFluidKind, amount: u8, falling: bool) -> Self {
        Self {
            kind,
            amount: amount.clamp(1, 8),
            falling,
        }
    }

    pub fn own_height(self) -> f64 {
        f64::from(self.amount) / 9.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainBlockCell {
    pub block_state_id: i32,
    pub block_name: Option<String>,
    pub block_properties: BTreeMap<String, String>,
    pub biome_id: Option<i32>,
    pub material: TerrainMaterialClass,
    /// Exact Vanilla 26.1 `BlockState.isSolidRender()` fact consumed by the
    /// client section visibility flood fill.
    #[serde(default)]
    pub solid_render: bool,
    #[serde(default)]
    pub skip_rendering: TerrainSkipRendering,
    pub fluid: Option<TerrainFluidState>,
    pub light: TerrainLight,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainSkipRendering {
    pub same_block_key: u64,
    pub same_block_culls_all_faces: bool,
    #[serde(default)]
    pub same_block_culls_vertical_faces: bool,
    #[serde(default)]
    pub leaves_block: bool,
    pub iron_bars_block: bool,
    pub bars_tag: bool,
    pub north: bool,
    pub south: bool,
    pub west: bool,
    pub east: bool,
}

impl TerrainSkipRendering {
    pub const NONE: Self = Self {
        same_block_key: 0,
        same_block_culls_all_faces: false,
        same_block_culls_vertical_faces: false,
        leaves_block: false,
        iron_bars_block: false,
        bars_tag: false,
        north: false,
        south: false,
        west: false,
        east: false,
    };

    fn with_same_block_key(mut self, block_name: &str) -> Self {
        self.same_block_key = stable_block_name_key(block_name);
        self
    }

    fn with_connections(mut self, properties: &BTreeMap<String, String>) -> Self {
        self.north = block_property_is_true(properties, "north");
        self.south = block_property_is_true(properties, "south");
        self.west = block_property_is_true(properties, "west");
        self.east = block_property_is_true(properties, "east");
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainLight {
    pub sky: u8,
    pub block: u8,
}

impl TerrainLight {
    pub const FULL_BRIGHT: Self = Self { sky: 15, block: 0 };
    pub const DARK: Self = Self { sky: 0, block: 0 };

    pub(crate) fn clamp(self) -> Self {
        Self {
            sky: self.sky.min(15),
            block: self.block.min(15),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainChunkSnapshot {
    pub pos: ChunkPos,
    pub min_y: i32,
    pub height: usize,
    pub cells: Vec<TerrainBlockCell>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainSectionMeshRevision {
    pub section_y: i32,
    pub revision: u64,
}

/// Small publication-shared render invalidation owner for one chunk column.
///
/// Full-height events advance `full_mesh_revision` and clear the per-section
/// overrides. Local block/light changes then copy and update only this small
/// owner while older publication generations retain their exact values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TerrainChunkRenderRevisions {
    snapshot_context_revision: u64,
    full_mesh_revision: u64,
    section_mesh_revisions: Vec<u64>,
}

impl TerrainChunkRenderRevisions {
    pub(crate) fn snapshot_context_revision(&self) -> u64 {
        self.snapshot_context_revision
    }

    pub(crate) fn mark_snapshot_context(&mut self, revision: u64) {
        self.snapshot_context_revision = revision;
    }

    pub(crate) fn mark_all_sections(&mut self, revision: u64) {
        self.full_mesh_revision = revision;
        self.section_mesh_revisions.clear();
    }

    pub(crate) fn mark_section(
        &mut self,
        min_section_y: i32,
        section_count: usize,
        section_y: i32,
        revision: u64,
    ) {
        let Some(section_offset) = section_y.checked_sub(min_section_y) else {
            return;
        };
        let Ok(section_index) = usize::try_from(section_offset) else {
            return;
        };
        if section_index >= section_count {
            return;
        }
        if self.section_mesh_revisions.len() < section_count {
            self.section_mesh_revisions.resize(section_count, 0);
        }
        self.section_mesh_revisions[section_index] = revision;
    }

    pub(crate) fn mesh_revision(&self) -> u64 {
        self.section_mesh_revisions
            .iter()
            .copied()
            .fold(self.full_mesh_revision, u64::max)
    }

    pub(crate) fn section_revision(&self, section_index: usize) -> u64 {
        self.section_mesh_revisions
            .get(section_index)
            .copied()
            .unwrap_or(0)
            .max(self.full_mesh_revision)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainChunkSummary {
    pub pos: Option<ChunkPos>,
    pub height: usize,
    pub total_blocks: usize,
    pub empty_blocks: usize,
    pub invisible_blocks: usize,
    pub opaque_blocks: usize,
    pub cutout_blocks: usize,
    pub fluid_blocks: usize,
    pub fluid_state_blocks: usize,
    pub translucent_blocks: usize,
}

impl TerrainChunkSnapshot {
    pub fn summary(&self) -> TerrainChunkSummary {
        let mut summary = TerrainChunkSummary {
            pos: Some(self.pos),
            height: self.height,
            total_blocks: self.cells.len(),
            ..TerrainChunkSummary::default()
        };
        for cell in &self.cells {
            match cell.material {
                TerrainMaterialClass::Empty => summary.empty_blocks += 1,
                TerrainMaterialClass::Invisible => summary.invisible_blocks += 1,
                TerrainMaterialClass::Opaque => summary.opaque_blocks += 1,
                TerrainMaterialClass::Cutout => summary.cutout_blocks += 1,
                TerrainMaterialClass::Fluid => summary.fluid_blocks += 1,
                TerrainMaterialClass::Translucent => summary.translucent_blocks += 1,
            }
            if cell.fluid.is_some() {
                summary.fluid_state_blocks += 1;
            }
        }
        summary
    }
}

pub(crate) fn classify_terrain_material(block_name: Option<&str>) -> TerrainMaterialClass {
    let Some(name) = block_name else {
        return TerrainMaterialClass::Opaque;
    };
    match name {
        name if block_name_is_air(name) => TerrainMaterialClass::Empty,
        "minecraft:water" | "minecraft:lava" => TerrainMaterialClass::Fluid,
        name if block_name_has_invisible_render_shape(name) => TerrainMaterialClass::Invisible,
        _ => TerrainMaterialClass::Opaque,
    }
}

pub fn classify_terrain_skip_rendering(
    block_name: Option<&str>,
    properties: &BTreeMap<String, String>,
) -> TerrainSkipRendering {
    let Some(name) = block_name else {
        return TerrainSkipRendering::NONE;
    };
    if block_name_is_leaves(name) {
        return TerrainSkipRendering {
            leaves_block: true,
            ..TerrainSkipRendering::NONE
        };
    }
    if is_half_transparent_skip_rendering_block_name(name)
        || VANILLA_SAME_BLOCK_ALL_FACE_SKIP_RENDERING_BLOCKS.contains(&name)
    {
        return TerrainSkipRendering {
            same_block_culls_all_faces: true,
            ..TerrainSkipRendering::NONE.with_same_block_key(name)
        };
    }
    if VANILLA_SAME_BLOCK_VERTICAL_FACE_SKIP_RENDERING_BLOCKS.contains(&name) {
        return TerrainSkipRendering {
            same_block_culls_vertical_faces: true,
            ..TerrainSkipRendering::NONE.with_same_block_key(name)
        };
    }
    if is_iron_bars_block_name(name) {
        return TerrainSkipRendering {
            iron_bars_block: true,
            bars_tag: is_bars_tag_block_name(name),
            ..TerrainSkipRendering::NONE
                .with_same_block_key(name)
                .with_connections(properties)
        };
    }
    TerrainSkipRendering::NONE
}

pub fn block_name_is_air(name: &str) -> bool {
    VANILLA_AIR_BLOCKS.contains(&name)
}

pub fn block_name_has_falling_dust_animate_tick(name: &str) -> bool {
    VANILLA_FALLING_BLOCKS.contains(&name) || VANILLA_BRUSHABLE_BLOCKS.contains(&name)
}

pub fn block_name_is_dry_grass_ambient(name: &str) -> bool {
    VANILLA_DRY_GRASS_AMBIENT_BLOCKS.contains(&name)
}

pub fn block_name_is_dead_bush_ambient(name: &str) -> bool {
    VANILLA_DEAD_BUSH_AMBIENT_BLOCKS.contains(&name)
}

pub fn block_name_is_fire(name: &str) -> bool {
    VANILLA_FIRE_BLOCKS.contains(&name)
}

pub fn block_name_is_terracotta(name: &str) -> bool {
    VANILLA_TERRACOTTA_BLOCKS.contains(&name)
}

pub fn block_name_triggers_ambient_desert_sand_block_sounds(name: &str) -> bool {
    VANILLA_TRIGGERS_AMBIENT_DESERT_SAND_BLOCK_SOUNDS.contains(&name)
}

pub fn block_name_triggers_ambient_desert_dry_vegetation_block_sounds(name: &str) -> bool {
    VANILLA_TRIGGERS_AMBIENT_DESERT_DRY_VEGETATION_BLOCK_SOUNDS.contains(&name)
}

pub(crate) fn block_name_is_impermeable(name: &str) -> bool {
    VANILLA_IMPERMEABLE_BLOCKS.contains(&name)
}

pub(crate) fn block_name_triggers_dried_ghast_ambient_sound(name: &str) -> bool {
    VANILLA_TRIGGERS_AMBIENT_DRIED_GHAST_BLOCK_SOUNDS.contains(&name)
}

pub(crate) fn block_name_is_enchantment_power_provider(name: &str) -> bool {
    VANILLA_ENCHANTMENT_POWER_PROVIDER_BLOCKS.contains(&name)
}

pub(crate) fn block_name_is_enchantment_power_transmitter(name: &str) -> bool {
    VANILLA_ENCHANTMENT_POWER_TRANSMITTER_BLOCKS.contains(&name)
}

pub(crate) fn block_name_is_fire_flammable(name: &str) -> bool {
    VANILLA_FIRE_FLAMMABLE_BLOCKS.contains(&name)
}

pub fn block_name_resets_fall_damage(name: &str) -> bool {
    VANILLA_FALL_DAMAGE_RESETTING_BLOCKS.contains(&name)
}

pub(crate) fn block_name_is_soul_fire_base(name: &str) -> bool {
    VANILLA_SOUL_FIRE_BASE_BLOCKS.contains(&name)
}

/// Vanilla `BlockBehaviour.Properties.friction`: `0.6F` unless the generated
/// `Blocks.java` registration declares an exact override.
pub(crate) fn vanilla_block_friction(block_name: Option<&str>) -> f32 {
    block_name
        .and_then(|name| {
            VANILLA_BLOCK_FRICTION_OVERRIDES
                .iter()
                .find_map(|(candidate, bits)| (*candidate == name).then(|| f32::from_bits(*bits)))
        })
        .unwrap_or(0.6)
}

/// Vanilla `BlockBehaviour.Properties.speedFactor`: `1.0F` unless the
/// generated `Blocks.java` registration declares an exact override.
pub(crate) fn vanilla_block_speed_factor(block_name: Option<&str>) -> f32 {
    block_name
        .and_then(|name| {
            VANILLA_BLOCK_SPEED_FACTOR_OVERRIDES
                .iter()
                .find_map(|(candidate, bits)| (*candidate == name).then(|| f32::from_bits(*bits)))
        })
        .unwrap_or(1.0)
}

/// Vanilla `BlockBehaviour.Properties.jumpFactor`: `1.0F` except for the
/// Honey Block registration's exact `0.5F` override in `Blocks.java`.
pub(crate) fn vanilla_block_jump_factor(block_name: Option<&str>) -> f32 {
    match block_name {
        Some("minecraft:honey_block") => 0.5,
        _ => 1.0,
    }
}

pub fn block_name_should_spawn_terrain_particles(name: &str) -> bool {
    !VANILLA_NO_TERRAIN_PARTICLE_BLOCKS.contains(&name)
}

pub(crate) fn terrain_fluid_state(
    block_name: Option<&str>,
    properties: &BTreeMap<String, String>,
) -> Option<TerrainFluidState> {
    let kind = match block_name? {
        "minecraft:water" => TerrainFluidKind::Water,
        "minecraft:lava" => TerrainFluidKind::Lava,
        name if is_fixed_source_water_block_name(name) => {
            return Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, false));
        }
        name if is_waterlogged(properties) => {
            return Some(TerrainFluidState::new(
                TerrainFluidKind::Water,
                8,
                block_name_is_copper_grate(name),
            ));
        }
        _ => return None,
    };
    let level = properties
        .get("level")
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(0);
    let (amount, falling) = match level {
        0 => (8, false),
        1..=7 => (8 - level, false),
        _ => (8, true),
    };
    Some(TerrainFluidState::new(kind, amount, falling))
}

fn is_waterlogged(properties: &BTreeMap<String, String>) -> bool {
    properties
        .get("waterlogged")
        .is_some_and(|value| value == "true")
}

fn is_fixed_source_water_block_name(name: &str) -> bool {
    VANILLA_FIXED_SOURCE_WATER_BLOCKS.contains(&name)
}

pub(crate) fn block_name_is_copper_grate(name: &str) -> bool {
    VANILLA_COPPER_GRATE_BLOCKS.contains(&name)
}

pub fn block_name_has_invisible_render_shape(name: &str) -> bool {
    VANILLA_INVISIBLE_RENDER_SHAPE_BLOCKS.contains(&name)
}

pub fn block_name_is_half_transparent(name: &str) -> bool {
    VANILLA_HALF_TRANSPARENT_SKIP_RENDERING_BLOCKS.contains(&name)
}

pub fn block_name_is_leaves(name: &str) -> bool {
    VANILLA_LEAVES_BLOCKS.contains(&name)
}

pub fn leaf_animate_particle_profile(name: &str) -> Option<LeafAnimateParticleProfile> {
    VANILLA_LEAF_ANIMATE_PARTICLE_PROFILES
        .iter()
        .copied()
        .find(|profile| profile.block_resource_id == name)
}

pub fn torch_animate_particle_profile(name: &str) -> Option<TorchAnimateParticleProfile> {
    VANILLA_TORCH_ANIMATE_PARTICLE_PROFILES
        .iter()
        .copied()
        .find(|profile| profile.block_resource_id == name)
}

pub fn block_name_has_animate_tick(name: &str) -> bool {
    VANILLA_BLOCK_ANIMATE_TICK_BLOCKS.contains(&name)
}

pub fn vanilla_block_animate_tick_names() -> &'static [&'static str] {
    VANILLA_BLOCK_ANIMATE_TICK_BLOCKS
}

pub fn block_name_is_door(name: &str) -> bool {
    VANILLA_DOOR_BLOCKS.contains(&name)
}

pub fn block_name_is_trapdoor(name: &str) -> bool {
    VANILLA_TRAPDOOR_BLOCKS.contains(&name)
}

pub fn block_name_is_rail(name: &str) -> bool {
    VANILLA_RAIL_BLOCKS.contains(&name)
}

pub fn block_name_is_button(name: &str) -> bool {
    VANILLA_BUTTON_BLOCKS.contains(&name)
}

pub fn block_name_is_pressure_plate(name: &str) -> bool {
    VANILLA_PRESSURE_PLATE_BLOCKS.contains(&name)
}

pub fn block_name_is_standing_sign(name: &str) -> bool {
    VANILLA_STANDING_SIGN_BLOCKS.contains(&name)
}

pub fn block_name_is_wall_sign(name: &str) -> bool {
    VANILLA_WALL_SIGN_BLOCKS.contains(&name)
}

pub fn block_name_is_slab(name: &str) -> bool {
    VANILLA_SLAB_BLOCKS.contains(&name)
}

pub fn block_name_is_stair(name: &str) -> bool {
    VANILLA_STAIR_BLOCKS.contains(&name)
}

pub fn block_name_is_wall(name: &str) -> bool {
    VANILLA_WALL_BLOCKS.contains(&name)
}

pub fn block_name_is_fence(name: &str) -> bool {
    VANILLA_FENCE_BLOCKS.contains(&name)
}

pub fn block_name_is_fence_gate(name: &str) -> bool {
    VANILLA_FENCE_GATE_BLOCKS.contains(&name)
}

pub fn block_name_is_wool_carpet(name: &str) -> bool {
    VANILLA_WOOL_CARPET_BLOCKS.contains(&name)
}

pub fn block_name_is_flower_pot(name: &str) -> bool {
    VANILLA_FLOWER_POT_BLOCKS.contains(&name)
}

pub fn block_name_is_sapling(name: &str) -> bool {
    VANILLA_SAPLING_BLOCKS.contains(&name)
}

pub fn block_name_is_bars_or_pane(name: &str) -> bool {
    VANILLA_BARS_BLOCKS.contains(&name) || VANILLA_IRON_BARS_CLASS_BLOCKS.contains(&name)
}

pub fn block_name_is_bed(name: &str) -> bool {
    VANILLA_BED_BLOCKS.contains(&name)
}

pub fn block_name_is_candle(name: &str) -> bool {
    VANILLA_CANDLE_BLOCKS.contains(&name)
}

pub fn block_name_is_candle_cake(name: &str) -> bool {
    VANILLA_CANDLE_CAKE_BLOCKS.contains(&name)
}

pub fn block_name_is_chain(name: &str) -> bool {
    VANILLA_CHAIN_BLOCKS.contains(&name)
}

pub fn block_name_is_copper_golem_statue(name: &str) -> bool {
    VANILLA_COPPER_GOLEM_STATUE_BLOCKS.contains(&name)
}

pub fn block_name_is_hanging_sign(name: &str) -> bool {
    VANILLA_HANGING_SIGN_BLOCKS.contains(&name)
}

pub fn block_name_is_lantern(name: &str) -> bool {
    VANILLA_LANTERN_BLOCKS.contains(&name)
}

pub fn block_name_is_rod(name: &str) -> bool {
    name == "minecraft:end_rod" || VANILLA_LIGHTNING_ROD_BLOCKS.contains(&name)
}

pub fn block_name_is_lightning_rod(name: &str) -> bool {
    VANILLA_LIGHTNING_ROD_BLOCKS.contains(&name)
}

pub fn block_name_is_shelf(name: &str) -> bool {
    VANILLA_SHELF_BLOCKS.contains(&name)
}

pub fn block_name_is_shulker_box(name: &str) -> bool {
    VANILLA_SHULKER_BOX_BLOCKS.contains(&name)
}

pub fn block_name_is_crop(name: &str) -> bool {
    VANILLA_CROP_BLOCKS.contains(&name)
}

pub fn block_name_is_anvil(name: &str) -> bool {
    VANILLA_ANVIL_BLOCKS.contains(&name)
}

pub fn block_name_is_campfire(name: &str) -> bool {
    VANILLA_CAMPFIRE_BLOCKS.contains(&name)
}

pub fn block_name_is_cauldron(name: &str) -> bool {
    VANILLA_CAULDRON_BLOCKS.contains(&name)
}

pub fn block_name_is_chest(name: &str) -> bool {
    VANILLA_CHEST_BLOCKS.contains(&name)
}

pub fn block_name_is_amethyst_cluster(name: &str) -> bool {
    VANILLA_AMETHYST_CLUSTER_BLOCKS.contains(&name)
}

pub fn block_name_is_floor_skull(name: &str) -> bool {
    VANILLA_FLOOR_SKULL_BLOCKS.contains(&name)
}

pub fn block_name_is_wall_skull(name: &str) -> bool {
    VANILLA_WALL_SKULL_BLOCKS.contains(&name)
}

pub fn block_name_is_floor_torch(name: &str) -> bool {
    VANILLA_FLOOR_TORCH_BLOCKS.contains(&name)
}

pub fn block_name_is_wall_torch(name: &str) -> bool {
    VANILLA_WALL_TORCH_BLOCKS.contains(&name)
}

pub fn block_name_is_log(name: &str) -> bool {
    VANILLA_LOG_BLOCKS.contains(&name)
}

pub fn block_name_is_pale_oak_log(name: &str) -> bool {
    VANILLA_PALE_OAK_LOG_BLOCKS.contains(&name)
}

pub fn block_name_is_planks(name: &str) -> bool {
    VANILLA_PLANK_BLOCKS.contains(&name)
}

pub fn block_name_is_ore(name: &str) -> bool {
    name == "minecraft:nether_quartz_ore" || VANILLA_ORE_BLOCKS.contains(&name)
}

pub(crate) fn block_name_has_empty_collision_shape(name: &str) -> bool {
    // Scaffolding overrides the default no-collision behavior with a
    // CollisionContext-dependent shape and is handled before this fallback.
    name != "minecraft:scaffolding" && VANILLA_NO_COLLISION_BLOCKS.contains(&name)
}

#[cfg(test)]
pub(crate) fn vanilla_block_collision_shape_override_names() -> &'static [&'static str] {
    VANILLA_BLOCK_COLLISION_SHAPE_OVERRIDE_BLOCKS
}

fn is_bars_tag_block_name(name: &str) -> bool {
    VANILLA_BARS_BLOCKS.contains(&name)
}

fn is_iron_bars_block_name(name: &str) -> bool {
    is_bars_tag_block_name(name) || VANILLA_IRON_BARS_CLASS_BLOCKS.contains(&name)
}

fn is_half_transparent_skip_rendering_block_name(name: &str) -> bool {
    block_name_is_half_transparent(name)
}

fn block_property_is_true(properties: &BTreeMap<String, String>, key: &str) -> bool {
    properties.get(key).is_some_and(|value| value == "true")
}

fn stable_block_name_key(name: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let hash = name.as_bytes().iter().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    });
    if hash == 0 {
        1
    } else {
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_terrain_material_does_not_guess_texture_render_layer() {
        assert_eq!(
            classify_terrain_material(Some("minecraft:air")),
            TerrainMaterialClass::Empty
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:water")),
            TerrainMaterialClass::Fluid
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:barrier")),
            TerrainMaterialClass::Invisible
        );
        for name in [
            "minecraft:stone",
            "minecraft:short_grass",
            "minecraft:oak_leaves",
            "minecraft:glass",
            "minecraft:nether_portal",
        ] {
            assert_eq!(
                classify_terrain_material(Some(name)),
                TerrainMaterialClass::Opaque,
                "{name} must defer its render layer to baked texture transparency"
            );
        }
    }
    #[test]
    fn identifies_vanilla_invisible_render_shape_block_names() {
        for &name in VANILLA_INVISIBLE_RENDER_SHAPE_BLOCKS {
            assert!(block_name_has_invisible_render_shape(name), "{name}");
        }

        for name in [
            "minecraft:stone",
            "minecraft:short_grass",
            "minecraft:glass",
            "minecraft:nether_portal",
        ] {
            assert!(!block_name_has_invisible_render_shape(name), "{name}");
        }
    }

    #[test]
    fn block_friction_uses_generated_overrides_and_vanilla_default() {
        assert_eq!(vanilla_block_friction(Some("minecraft:stone")), 0.6);
        assert_eq!(vanilla_block_friction(Some("minecraft:ice")), 0.98);
        assert_eq!(vanilla_block_friction(Some("minecraft:slime_block")), 0.8);
        assert_eq!(vanilla_block_friction(Some("minecraft:blue_ice")), 0.989);
        assert_eq!(vanilla_block_friction(None), 0.6);
    }

    #[test]
    fn block_speed_factor_uses_generated_overrides_and_vanilla_default() {
        assert_eq!(vanilla_block_speed_factor(Some("minecraft:stone")), 1.0);
        assert_eq!(vanilla_block_speed_factor(Some("minecraft:soul_sand")), 0.4);
        assert_eq!(
            vanilla_block_speed_factor(Some("minecraft:honey_block")),
            0.4
        );
        assert_eq!(vanilla_block_speed_factor(None), 1.0);
    }

    #[test]
    fn block_jump_factor_matches_vanilla_honey_override_and_default() {
        assert_eq!(
            vanilla_block_jump_factor(Some("minecraft:honey_block")),
            0.5
        );
        assert_eq!(vanilla_block_jump_factor(Some("minecraft:stone")), 1.0);
        assert_eq!(vanilla_block_jump_factor(None), 1.0);
    }

    #[test]
    fn identifies_vanilla_terrain_particle_spawn_block_names() {
        for name in ["minecraft:barrier", "minecraft:structure_void"] {
            assert!(!block_name_should_spawn_terrain_particles(name), "{name}");
        }

        for name in [
            "minecraft:stone",
            "minecraft:air",
            "minecraft:water",
            "minecraft:moving_piston",
        ] {
            assert!(block_name_should_spawn_terrain_particles(name), "{name}");
        }
    }

    #[test]
    fn identifies_generated_fire_flammability_without_name_guessing() {
        for name in [
            "minecraft:oak_planks",
            "minecraft:tnt",
            "minecraft:short_grass",
        ] {
            assert!(block_name_is_fire_flammable(name), "{name}");
        }
        for name in [
            "minecraft:stone",
            "minecraft:crimson_planks",
            "minecraft:custom_planks",
        ] {
            assert!(!block_name_is_fire_flammable(name), "{name}");
        }
        assert!(block_name_is_soul_fire_base("minecraft:soul_sand"));
        assert!(block_name_is_soul_fire_base("minecraft:soul_soil"));
        assert!(!block_name_is_soul_fire_base("minecraft:sand"));
    }

    #[test]
    fn identifies_generated_fall_damage_resetting_membership() {
        for name in [
            "minecraft:ladder",
            "minecraft:cave_vines_plant",
            "minecraft:sweet_berry_bush",
            "minecraft:cobweb",
        ] {
            assert!(block_name_resets_fall_damage(name), "{name}");
        }
        for name in [
            "minecraft:stone",
            "minecraft:water",
            "minecraft:custom_ladder",
        ] {
            assert!(!block_name_resets_fall_damage(name), "{name}");
        }
    }

    #[test]
    fn identifies_generated_object_shape_memberships_without_suffix_guessing() {
        for (member, lookalike, classifier) in [
            (
                "minecraft:white_bed",
                "minecraft:custom_bed",
                block_name_is_bed as fn(&str) -> bool,
            ),
            (
                "minecraft:white_candle",
                "minecraft:custom_candle",
                block_name_is_candle,
            ),
            (
                "minecraft:white_candle_cake",
                "minecraft:custom_candle_cake",
                block_name_is_candle_cake,
            ),
            (
                "minecraft:copper_chain",
                "minecraft:custom_chain",
                block_name_is_chain,
            ),
            (
                "minecraft:waxed_copper_golem_statue",
                "minecraft:custom_copper_golem_statue",
                block_name_is_copper_golem_statue,
            ),
            (
                "minecraft:copper_lantern",
                "minecraft:custom_copper_lantern",
                block_name_is_lantern,
            ),
            (
                "minecraft:waxed_lightning_rod",
                "minecraft:custom_lightning_rod",
                block_name_is_rod,
            ),
            (
                "minecraft:oak_hanging_sign",
                "minecraft:custom_hanging_sign",
                block_name_is_hanging_sign,
            ),
            (
                "minecraft:white_shulker_box",
                "minecraft:custom_shulker_box",
                block_name_is_shulker_box,
            ),
            (
                "minecraft:oak_shelf",
                "minecraft:custom_shelf",
                block_name_is_shelf,
            ),
        ] {
            assert!(classifier(member), "{member}");
            assert!(!classifier(lookalike), "{lookalike}");
        }
        assert!(block_name_is_rod("minecraft:end_rod"));
        assert!(!block_name_is_bed("example:white_bed"));
        for block_name in [
            "minecraft:wheat",
            "minecraft:melon_stem",
            "minecraft:pumpkin_stem",
            "minecraft:pitcher_crop",
        ] {
            assert!(block_name_is_crop(block_name), "{block_name}");
        }
        assert!(!block_name_is_crop("minecraft:nether_wart"));
        assert!(!block_name_is_crop("minecraft:custom_crop"));
        for block_name in [
            "minecraft:anvil",
            "minecraft:soul_campfire",
            "minecraft:powder_snow_cauldron",
            "minecraft:chest",
            "minecraft:trapped_chest",
            "minecraft:copper_chest",
            "minecraft:waxed_oxidized_copper_chest",
        ] {
            assert!(
                block_name_is_anvil(block_name)
                    || block_name_is_campfire(block_name)
                    || block_name_is_cauldron(block_name)
                    || block_name_is_chest(block_name),
                "{block_name}"
            );
        }
        assert!(!block_name_is_chest("minecraft:ender_chest"));
        assert!(!block_name_is_chest("minecraft:custom_chest"));
        assert!(block_name_is_amethyst_cluster(
            "minecraft:small_amethyst_bud"
        ));
        assert!(!block_name_is_amethyst_cluster(
            "minecraft:custom_amethyst_bud"
        ));
        assert!(block_name_is_floor_skull("minecraft:player_head"));
        assert!(!block_name_is_floor_skull("minecraft:player_wall_head"));
        assert!(block_name_is_wall_skull("minecraft:player_wall_head"));
        assert!(!block_name_is_wall_skull("minecraft:player_head"));
        assert!(block_name_is_floor_torch("minecraft:copper_torch"));
        assert!(!block_name_is_floor_torch("minecraft:copper_wall_torch"));
        assert!(block_name_is_wall_torch("minecraft:copper_wall_torch"));
        assert!(!block_name_is_wall_torch("minecraft:copper_torch"));
        for block_name in [
            "minecraft:oak_log",
            "minecraft:stripped_oak_wood",
            "minecraft:crimson_stem",
            "minecraft:warped_hyphae",
        ] {
            assert!(block_name_is_log(block_name), "{block_name}");
        }
        for block_name in [
            "minecraft:pale_oak_log",
            "minecraft:pale_oak_wood",
            "minecraft:stripped_pale_oak_log",
            "minecraft:stripped_pale_oak_wood",
        ] {
            assert!(block_name_is_pale_oak_log(block_name), "{block_name}");
        }
        assert!(!block_name_is_pale_oak_log("minecraft:oak_log"));
        assert!(block_name_is_planks("minecraft:bamboo_planks"));
        assert!(!block_name_is_log("minecraft:custom_log"));
        assert!(!block_name_is_planks("minecraft:custom_planks"));
        for block_name in [
            "minecraft:coal_ore",
            "minecraft:deepslate_diamond_ore",
            "minecraft:nether_gold_ore",
            "minecraft:nether_quartz_ore",
        ] {
            assert!(block_name_is_ore(block_name), "{block_name}");
        }
        for block_name in [
            "minecraft:custom_ore",
            "minecraft:deepslate_custom_ore",
            "example:diamond_ore",
        ] {
            assert!(!block_name_is_ore(block_name), "{block_name}");
        }
    }

    #[test]
    fn classifies_vanilla_skip_rendering_blocks() {
        let glass = classify_terrain_skip_rendering(Some("minecraft:glass"), &BTreeMap::new());
        assert_ne!(glass.same_block_key, 0);
        assert!(glass.same_block_culls_all_faces);
        assert!(!glass.iron_bars_block);
        assert!(!glass.bars_tag);
        assert_eq!(
            glass.same_block_key,
            classify_terrain_skip_rendering(Some("minecraft:glass"), &BTreeMap::new())
                .same_block_key
        );
        assert_ne!(
            glass.same_block_key,
            classify_terrain_skip_rendering(
                Some("minecraft:white_stained_glass"),
                &BTreeMap::new()
            )
            .same_block_key
        );
        for &name in VANILLA_HALF_TRANSPARENT_SKIP_RENDERING_BLOCKS {
            let skip = classify_terrain_skip_rendering(Some(name), &BTreeMap::new());
            assert!(skip.same_block_culls_all_faces, "{name}");
            assert_ne!(skip.same_block_key, 0, "{name}");
        }
        for &name in VANILLA_LEAVES_BLOCKS {
            assert!(block_name_is_leaves(name), "{name}");
            assert!(!block_name_is_half_transparent(name), "{name}");
            assert!(
                classify_terrain_skip_rendering(Some(name), &BTreeMap::new()).leaves_block,
                "{name}"
            );
        }
        assert!(block_name_is_door("minecraft:oak_door"));
        assert!(block_name_is_door("minecraft:iron_door"));
        assert!(!block_name_is_door("minecraft:custom_door"));
        assert!(block_name_is_trapdoor("minecraft:oak_trapdoor"));
        assert!(block_name_is_trapdoor("minecraft:iron_trapdoor"));
        assert!(!block_name_is_trapdoor("minecraft:custom_trapdoor"));
        assert!(block_name_is_rail("minecraft:powered_rail"));
        assert!(!block_name_is_rail("minecraft:custom_rail"));
        assert!(block_name_is_button("minecraft:stone_button"));
        assert!(!block_name_is_button("minecraft:custom_button"));
        assert!(block_name_is_pressure_plate("minecraft:oak_pressure_plate"));
        assert!(!block_name_is_pressure_plate(
            "minecraft:custom_pressure_plate"
        ));
        assert!(block_name_is_standing_sign("minecraft:oak_sign"));
        assert!(!block_name_is_standing_sign("minecraft:oak_wall_sign"));
        assert!(block_name_is_wall_sign("minecraft:oak_wall_sign"));
        assert!(!block_name_is_wall_sign("minecraft:oak_hanging_sign"));
        for (member, lookalike) in [
            ("minecraft:oak_slab", "minecraft:custom_slab"),
            ("minecraft:oak_stairs", "minecraft:custom_stairs"),
            ("minecraft:cobblestone_wall", "minecraft:custom_wall"),
            ("minecraft:oak_fence", "minecraft:custom_fence"),
            ("minecraft:oak_fence_gate", "minecraft:custom_fence_gate"),
            ("minecraft:white_carpet", "minecraft:custom_carpet"),
            ("minecraft:flower_pot", "minecraft:potted_custom"),
        ] {
            assert!(
                block_name_is_slab(member)
                    || block_name_is_stair(member)
                    || block_name_is_wall(member)
                    || block_name_is_fence(member)
                    || block_name_is_fence_gate(member)
                    || block_name_is_wool_carpet(member)
                    || block_name_is_flower_pot(member),
                "{member}"
            );
            assert!(!block_name_is_slab(lookalike));
            assert!(!block_name_is_stair(lookalike));
            assert!(!block_name_is_wall(lookalike));
            assert!(!block_name_is_fence(lookalike));
            assert!(!block_name_is_fence_gate(lookalike));
            assert!(!block_name_is_wool_carpet(lookalike));
            assert!(!block_name_is_flower_pot(lookalike));
        }
        assert!(block_name_is_sapling("minecraft:oak_sapling"));
        assert!(block_name_is_sapling("minecraft:mangrove_propagule"));
        assert!(!block_name_is_sapling("minecraft:bamboo_sapling"));
        assert!(!block_name_is_sapling("minecraft:custom_sapling"));
        assert!(block_name_has_empty_collision_shape(
            "minecraft:oak_sapling"
        ));
        assert!(block_name_has_empty_collision_shape(
            "minecraft:mangrove_propagule"
        ));
        assert!(block_name_has_empty_collision_shape(
            "minecraft:bamboo_sapling"
        ));
        for block_name in [
            "minecraft:air",
            "minecraft:stone_button",
            "minecraft:leaf_litter",
            "minecraft:pink_petals",
            "minecraft:attached_pumpkin_stem",
            "minecraft:copper_wall_torch",
            "minecraft:tube_coral_wall_fan",
        ] {
            assert!(
                block_name_has_empty_collision_shape(block_name),
                "{block_name}"
            );
        }
        assert!(!block_name_has_empty_collision_shape(
            "minecraft:custom_sapling"
        ));
        assert!(!block_name_has_empty_collision_shape("example:oak_sapling"));
        assert!(!block_name_has_empty_collision_shape(
            "minecraft:scaffolding"
        ));
        assert!(!block_name_has_empty_collision_shape("minecraft:grass"));
        assert!(block_name_is_bars_or_pane("minecraft:copper_bars"));
        assert!(block_name_is_bars_or_pane(
            "minecraft:white_stained_glass_pane"
        ));
        assert!(!block_name_is_bars_or_pane("minecraft:custom_bars"));
        assert!(!block_name_is_bars_or_pane("minecraft:custom_glass_pane"));
        assert!(
            classify_terrain_skip_rendering(Some("minecraft:copper_grate"), &BTreeMap::new())
                .same_block_culls_all_faces
        );
        assert!(
            classify_terrain_skip_rendering(Some("minecraft:powder_snow"), &BTreeMap::new())
                .same_block_culls_all_faces
        );
        let mangrove_roots =
            classify_terrain_skip_rendering(Some("minecraft:mangrove_roots"), &BTreeMap::new());
        assert!(mangrove_roots.same_block_culls_vertical_faces);
        assert!(!mangrove_roots.same_block_culls_all_faces);
        assert_eq!(
            classify_terrain_skip_rendering(
                Some("minecraft:muddy_mangrove_roots"),
                &BTreeMap::new()
            ),
            TerrainSkipRendering::NONE
        );

        let pane = classify_terrain_skip_rendering(
            Some("minecraft:white_stained_glass_pane"),
            &properties([
                ("north", "true"),
                ("south", "false"),
                ("west", "false"),
                ("east", "true"),
            ]),
        );
        assert_ne!(pane.same_block_key, 0);
        assert!(!pane.same_block_culls_all_faces);
        assert!(pane.iron_bars_block);
        assert!(!pane.bars_tag);
        assert!(pane.north);
        assert!(!pane.south);
        assert!(!pane.west);
        assert!(pane.east);

        for &name in VANILLA_IRON_BARS_CLASS_BLOCKS {
            assert!(classify_terrain_skip_rendering(Some(name), &BTreeMap::new()).iron_bars_block);
        }

        let bars = classify_terrain_skip_rendering(
            Some("minecraft:copper_bars"),
            &properties([
                ("north", "false"),
                ("south", "true"),
                ("west", "true"),
                ("east", "false"),
            ]),
        );
        assert!(bars.iron_bars_block);
        assert!(bars.bars_tag);
        assert!(!bars.north);
        assert!(bars.south);
        assert!(bars.west);
        assert!(!bars.east);

        assert_eq!(
            classify_terrain_skip_rendering(Some("minecraft:stone"), &BTreeMap::new()),
            TerrainSkipRendering::NONE
        );
        assert_eq!(
            classify_terrain_skip_rendering(None, &BTreeMap::new()),
            TerrainSkipRendering::NONE
        );
    }

    #[test]
    fn leaf_animate_profiles_preserve_vanilla_particle_families() {
        assert_eq!(
            VANILLA_LEAF_ANIMATE_PARTICLE_PROFILES
                .iter()
                .map(|profile| profile.block_resource_id)
                .collect::<Vec<_>>(),
            VANILLA_LEAVES_BLOCKS
        );
        let oak = leaf_animate_particle_profile("minecraft:oak_leaves").unwrap();
        assert_eq!(oak.leaf_particle_chance(), 0.01);
        assert_eq!(oak.particle_resource_id, "minecraft:tinted_leaves");
        assert_eq!(oak.tint, LeafParticleTint::ClientLeaf);

        let cherry = leaf_animate_particle_profile("minecraft:cherry_leaves").unwrap();
        assert_eq!(cherry.leaf_particle_chance(), 0.1);
        assert_eq!(cherry.particle_resource_id, "minecraft:cherry_leaves");
        assert_eq!(cherry.tint, LeafParticleTint::None);

        let pale_oak = leaf_animate_particle_profile("minecraft:pale_oak_leaves").unwrap();
        assert_eq!(pale_oak.leaf_particle_chance(), 0.02);
        assert_eq!(pale_oak.particle_resource_id, "minecraft:pale_oak_leaves");
        assert_eq!(pale_oak.tint, LeafParticleTint::None);

        for block_name in [
            "minecraft:azalea_leaves",
            "minecraft:flowering_azalea_leaves",
        ] {
            let profile = leaf_animate_particle_profile(block_name).unwrap();
            assert_eq!(profile.leaf_particle_chance(), 0.01);
            assert_eq!(profile.particle_resource_id, "minecraft:tinted_leaves");
            assert_eq!(profile.tint, LeafParticleTint::FixedArgb(0xff70_922d));
        }
        assert_eq!(leaf_animate_particle_profile("example:oak_leaves"), None);
    }

    #[test]
    fn animate_tick_catalog_and_torch_profiles_preserve_vanilla_owners() {
        assert_eq!(VANILLA_BLOCK_ANIMATE_TICK_BLOCKS.len(), 126);
        for profile in VANILLA_TORCH_ANIMATE_PARTICLE_PROFILES {
            assert!(block_name_has_animate_tick(profile.block_resource_id));
        }
        assert_eq!(
            torch_animate_particle_profile("minecraft:torch"),
            Some(TorchAnimateParticleProfile::new(
                "minecraft:torch",
                "minecraft:flame",
                false,
            ))
        );
        assert_eq!(
            torch_animate_particle_profile("minecraft:soul_wall_torch"),
            Some(TorchAnimateParticleProfile::new(
                "minecraft:soul_wall_torch",
                "minecraft:soul_fire_flame",
                true,
            ))
        );
        assert_eq!(
            torch_animate_particle_profile("minecraft:copper_wall_torch"),
            Some(TorchAnimateParticleProfile::new(
                "minecraft:copper_wall_torch",
                "minecraft:copper_fire_flame",
                true,
            ))
        );
        assert_eq!(
            torch_animate_particle_profile("minecraft:redstone_torch"),
            None
        );
        assert!(!block_name_has_animate_tick("minecraft:stone"));
    }

    #[test]
    fn falling_and_desert_memberships_follow_generated_vanilla_owners_and_tags() {
        for block_name in [
            "minecraft:sand",
            "minecraft:red_sand",
            "minecraft:gravel",
            "minecraft:dragon_egg",
            "minecraft:anvil",
            "minecraft:white_concrete_powder",
            "minecraft:suspicious_sand",
            "minecraft:suspicious_gravel",
        ] {
            assert!(block_name_has_falling_dust_animate_tick(block_name));
        }
        assert!(!block_name_has_falling_dust_animate_tick(
            "minecraft:white_concrete"
        ));

        assert!(block_name_is_dry_grass_ambient("minecraft:short_dry_grass"));
        assert!(block_name_is_dry_grass_ambient("minecraft:tall_dry_grass"));
        assert!(block_name_is_dead_bush_ambient("minecraft:dead_bush"));
        assert!(!block_name_is_dead_bush_ambient(
            "minecraft:potted_dead_bush"
        ));

        assert!(block_name_is_fire("minecraft:fire"));
        assert!(block_name_is_fire("minecraft:soul_fire"));
        assert!(!block_name_is_fire("minecraft:campfire"));
        assert!(block_name_is_terracotta("minecraft:terracotta"));
        assert!(block_name_is_terracotta("minecraft:red_terracotta"));
        assert!(!block_name_is_terracotta("minecraft:red_sand"));

        for block_name in ["minecraft:sand", "minecraft:red_sand"] {
            assert!(block_name_triggers_ambient_desert_sand_block_sounds(
                block_name
            ));
            assert!(block_name_triggers_ambient_desert_dry_vegetation_block_sounds(block_name));
        }
        assert!(
            block_name_triggers_ambient_desert_dry_vegetation_block_sounds(
                "minecraft:blue_terracotta"
            )
        );
        assert!(!block_name_triggers_ambient_desert_sand_block_sounds(
            "minecraft:sandstone"
        ));
    }

    #[test]
    fn maps_fluid_state_from_liquids_and_waterlogged_blocks() {
        assert_eq!(
            terrain_fluid_state(Some("minecraft:water"), &properties([("level", "3")])),
            Some(TerrainFluidState::new(TerrainFluidKind::Water, 5, false))
        );
        assert_eq!(
            terrain_fluid_state(Some("minecraft:lava"), &properties([("level", "8")])),
            Some(TerrainFluidState::new(TerrainFluidKind::Lava, 8, true))
        );
        assert_eq!(
            terrain_fluid_state(
                Some("minecraft:oak_slab"),
                &properties([("waterlogged", "true")])
            ),
            Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, false))
        );
        for &name in VANILLA_COPPER_GRATE_BLOCKS {
            assert_eq!(
                terrain_fluid_state(Some(name), &properties([("waterlogged", "true")])),
                Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, true))
            );
        }
        assert_eq!(
            terrain_fluid_state(
                Some("minecraft:oak_slab"),
                &properties([("waterlogged", "false")])
            ),
            None
        );
        for &name in VANILLA_FIXED_SOURCE_WATER_BLOCKS {
            assert_eq!(
                terrain_fluid_state(Some(name), &BTreeMap::new()),
                Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, false))
            );
        }
    }

    #[test]
    fn fluid_state_own_height_uses_vanilla_amount_over_nine() {
        assert_eq!(
            TerrainFluidState::new(TerrainFluidKind::Water, 8, false).own_height(),
            8.0 / 9.0
        );
        assert_eq!(
            TerrainFluidState::new(TerrainFluidKind::Water, 5, false).own_height(),
            5.0 / 9.0
        );
        assert_eq!(
            TerrainFluidState::new(TerrainFluidKind::Lava, 1, false).own_height(),
            1.0 / 9.0
        );
    }

    #[test]
    fn summary_counts_invisible_blocks_separately() {
        let snapshot = TerrainChunkSnapshot {
            pos: ChunkPos { x: 0, z: 0 },
            min_y: 0,
            height: 1,
            cells: vec![
                terrain_cell(TerrainMaterialClass::Invisible, None),
                terrain_cell(TerrainMaterialClass::Empty, None),
                terrain_cell(
                    TerrainMaterialClass::Opaque,
                    Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, false)),
                ),
            ],
        };

        let summary = snapshot.summary();

        assert_eq!(summary.total_blocks, 3);
        assert_eq!(summary.invisible_blocks, 1);
        assert_eq!(summary.empty_blocks, 1);
        assert_eq!(summary.opaque_blocks, 1);
        assert_eq!(summary.fluid_state_blocks, 1);
    }

    fn terrain_cell(
        material: TerrainMaterialClass,
        fluid: Option<TerrainFluidState>,
    ) -> TerrainBlockCell {
        TerrainBlockCell {
            block_state_id: 0,
            block_name: None,
            block_properties: BTreeMap::new(),
            biome_id: None,
            material,
            solid_render: false,
            skip_rendering: TerrainSkipRendering::NONE,
            fluid,
            light: TerrainLight::FULL_BRIGHT,
        }
    }

    fn properties<const N: usize>(entries: [(&str, &str); N]) -> BTreeMap<String, String> {
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }
}
