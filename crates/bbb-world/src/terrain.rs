use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{BlockPos, ChunkPos, PaletteKind};

mod generated_26_1;

use generated_26_1::{
    VANILLA_AIR_BLOCKS, VANILLA_BARS_BLOCKS, VANILLA_COPPER_GRATE_BLOCKS,
    VANILLA_FIXED_SOURCE_WATER_BLOCKS, VANILLA_INVISIBLE_RENDER_SHAPE_BLOCKS,
    VANILLA_IRON_BARS_CLASS_BLOCKS, VANILLA_NO_COLLISION_BANNER_BLOCKS,
    VANILLA_NO_COLLISION_BUTTON_BLOCKS, VANILLA_NO_COLLISION_CROP_BLOCKS,
    VANILLA_NO_COLLISION_CROP_SUPPLEMENT_BLOCKS, VANILLA_NO_COLLISION_FIRE_BLOCKS,
    VANILLA_NO_COLLISION_PRESSURE_PLATE_BLOCKS, VANILLA_NO_COLLISION_RAIL_BLOCKS,
    VANILLA_NO_COLLISION_SIGN_BLOCKS, VANILLA_NO_COLLISION_SMALL_FLOWER_BLOCKS,
    VANILLA_NO_COLLISION_THIN_OVERLAY_BLOCKS, VANILLA_NO_COLLISION_TORCH_BLOCKS,
    VANILLA_NO_COLLISION_WIRE_BLOCKS, VANILLA_NO_TERRAIN_PARTICLE_BLOCKS,
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
    #[serde(default)]
    pub skip_rendering: TerrainSkipRendering,
    pub fluid: Option<TerrainFluidState>,
    pub light: TerrainLight,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainSkipRendering {
    pub same_block_key: u64,
    pub same_block_culls_all_faces: bool,
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
    if is_half_transparent_skip_rendering_block_name(name) {
        return TerrainSkipRendering {
            same_block_culls_all_faces: true,
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

pub(crate) fn block_name_has_empty_collision_shape(name: &str) -> bool {
    name.contains("sapling")
        || matches!(
            name,
            "minecraft:short_grass"
                | "minecraft:tall_grass"
                | "minecraft:grass"
                | "minecraft:fern"
                | "minecraft:large_fern"
                | "minecraft:cobweb"
                | "minecraft:sweet_berry_bush"
                | "minecraft:brown_mushroom"
                | "minecraft:red_mushroom"
                | "minecraft:crimson_roots"
                | "minecraft:warped_roots"
                | "minecraft:nether_sprouts"
                | "minecraft:weeping_vines"
                | "minecraft:weeping_vines_plant"
                | "minecraft:twisting_vines"
                | "minecraft:twisting_vines_plant"
                | "minecraft:cave_vines"
                | "minecraft:cave_vines_plant"
                | "minecraft:vine"
                | "minecraft:pale_hanging_moss"
                | "minecraft:kelp"
                | "minecraft:kelp_plant"
                | "minecraft:seagrass"
                | "minecraft:tall_seagrass"
                | "minecraft:lilac"
                | "minecraft:rose_bush"
                | "minecraft:peony"
                | "minecraft:pitcher_plant"
                | "minecraft:dead_bush"
                | "minecraft:bush"
                | "minecraft:firefly_bush"
                | "minecraft:short_dry_grass"
                | "minecraft:tall_dry_grass"
                | "minecraft:mangrove_propagule"
                | "minecraft:crimson_fungus"
                | "minecraft:warped_fungus"
                | "minecraft:small_dripleaf"
                | "minecraft:spore_blossom"
                | "minecraft:lever"
        )
        || VANILLA_NO_COLLISION_TORCH_BLOCKS.contains(&name)
        || VANILLA_NO_COLLISION_CROP_SUPPLEMENT_BLOCKS.contains(&name)
        || VANILLA_NO_COLLISION_WIRE_BLOCKS.contains(&name)
        || VANILLA_NO_COLLISION_THIN_OVERLAY_BLOCKS.contains(&name)
        || VANILLA_NO_COLLISION_RAIL_BLOCKS.contains(&name)
        || VANILLA_NO_COLLISION_SIGN_BLOCKS.contains(&name)
        || VANILLA_NO_COLLISION_BUTTON_BLOCKS.contains(&name)
        || VANILLA_NO_COLLISION_PRESSURE_PLATE_BLOCKS.contains(&name)
        || VANILLA_NO_COLLISION_BANNER_BLOCKS.contains(&name)
        || VANILLA_NO_COLLISION_SMALL_FLOWER_BLOCKS.contains(&name)
        || VANILLA_NO_COLLISION_CROP_BLOCKS.contains(&name)
        || VANILLA_NO_COLLISION_FIRE_BLOCKS.contains(&name)
}

fn is_bars_tag_block_name(name: &str) -> bool {
    VANILLA_BARS_BLOCKS.contains(&name)
}

fn is_iron_bars_block_name(name: &str) -> bool {
    is_bars_tag_block_name(name) || VANILLA_IRON_BARS_CLASS_BLOCKS.contains(&name)
}

fn is_half_transparent_skip_rendering_block_name(name: &str) -> bool {
    matches!(
        name,
        "minecraft:glass"
            | "minecraft:tinted_glass"
            | "minecraft:ice"
            | "minecraft:frosted_ice"
            | "minecraft:blue_ice"
            | "minecraft:slime_block"
            | "minecraft:honey_block"
    ) || name
        .strip_prefix("minecraft:")
        .is_some_and(|stem| stem.ends_with("_stained_glass"))
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
