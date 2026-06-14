use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{BlockPos, ChunkPos, PaletteKind};

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainBlockCell {
    pub block_state_id: i32,
    pub block_name: Option<String>,
    pub block_properties: BTreeMap<String, String>,
    pub biome_id: Option<i32>,
    pub material: TerrainMaterialClass,
    pub fluid: Option<TerrainFluidState>,
    pub light: TerrainLight,
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
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air" => {
            TerrainMaterialClass::Empty
        }
        "minecraft:water" | "minecraft:lava" => TerrainMaterialClass::Fluid,
        name if is_invisible_render_block_name(name) => TerrainMaterialClass::Invisible,
        name if is_cutout_block_name(name) => TerrainMaterialClass::Cutout,
        name if is_translucent_block_name(name) => TerrainMaterialClass::Translucent,
        _ => TerrainMaterialClass::Opaque,
    }
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
                is_falling_source_waterlogged_block_name(name),
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
    matches!(
        name,
        "minecraft:bubble_column"
            | "minecraft:seagrass"
            | "minecraft:tall_seagrass"
            | "minecraft:kelp"
            | "minecraft:kelp_plant"
    )
}

fn is_falling_source_waterlogged_block_name(name: &str) -> bool {
    matches!(
        name,
        "minecraft:copper_grate"
            | "minecraft:exposed_copper_grate"
            | "minecraft:weathered_copper_grate"
            | "minecraft:oxidized_copper_grate"
            | "minecraft:waxed_copper_grate"
            | "minecraft:waxed_exposed_copper_grate"
            | "minecraft:waxed_weathered_copper_grate"
            | "minecraft:waxed_oxidized_copper_grate"
    )
}

fn is_invisible_render_block_name(name: &str) -> bool {
    matches!(
        name,
        "minecraft:barrier"
            | "minecraft:bubble_column"
            | "minecraft:structure_void"
            | "minecraft:end_gateway"
            | "minecraft:end_portal"
            | "minecraft:light"
            | "minecraft:moving_piston"
    )
}

fn is_cutout_block_name(name: &str) -> bool {
    name.contains("sapling")
        || name.contains("leaves")
        || name == "minecraft:short_grass"
        || name == "minecraft:tall_grass"
        || name == "minecraft:grass"
        || name.contains("fern")
        || name.contains("flower")
        || name.contains("mushroom")
        || name.contains("roots")
        || name.contains("vine")
        || name.contains("kelp")
        || name.contains("seagrass")
}

fn is_translucent_block_name(name: &str) -> bool {
    name.contains("glass")
        || name.contains("ice")
        || name.contains("slime")
        || name.contains("honey")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_basic_terrain_materials() {
        assert_eq!(
            classify_terrain_material(Some("minecraft:air")),
            TerrainMaterialClass::Empty
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:grass_block")),
            TerrainMaterialClass::Opaque
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:short_grass")),
            TerrainMaterialClass::Cutout
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:water")),
            TerrainMaterialClass::Fluid
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:barrier")),
            TerrainMaterialClass::Invisible
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:bubble_column")),
            TerrainMaterialClass::Invisible
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:structure_void")),
            TerrainMaterialClass::Invisible
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:end_gateway")),
            TerrainMaterialClass::Invisible
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:end_portal")),
            TerrainMaterialClass::Invisible
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:light")),
            TerrainMaterialClass::Invisible
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:moving_piston")),
            TerrainMaterialClass::Invisible
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
        for name in [
            "minecraft:copper_grate",
            "minecraft:exposed_copper_grate",
            "minecraft:weathered_copper_grate",
            "minecraft:oxidized_copper_grate",
            "minecraft:waxed_copper_grate",
            "minecraft:waxed_exposed_copper_grate",
            "minecraft:waxed_weathered_copper_grate",
            "minecraft:waxed_oxidized_copper_grate",
        ] {
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
        for name in [
            "minecraft:bubble_column",
            "minecraft:seagrass",
            "minecraft:tall_seagrass",
            "minecraft:kelp",
            "minecraft:kelp_plant",
        ] {
            assert_eq!(
                terrain_fluid_state(Some(name), &BTreeMap::new()),
                Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, false))
            );
        }
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
