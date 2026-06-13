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
    pub block_palette_kind: PaletteKind,
    pub block_palette_index: Option<usize>,
    pub biome_id: Option<i32>,
    pub biome_palette_kind: PaletteKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainMaterialClass {
    Empty,
    Opaque,
    Cutout,
    Fluid,
    Translucent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainBlockCell {
    pub block_state_id: i32,
    pub block_name: Option<String>,
    pub block_properties: BTreeMap<String, String>,
    pub biome_id: Option<i32>,
    pub material: TerrainMaterialClass,
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
    pub opaque_blocks: usize,
    pub cutout_blocks: usize,
    pub fluid_blocks: usize,
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
                TerrainMaterialClass::Opaque => summary.opaque_blocks += 1,
                TerrainMaterialClass::Cutout => summary.cutout_blocks += 1,
                TerrainMaterialClass::Fluid => summary.fluid_blocks += 1,
                TerrainMaterialClass::Translucent => summary.translucent_blocks += 1,
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
        name if is_cutout_block_name(name) => TerrainMaterialClass::Cutout,
        name if is_translucent_block_name(name) => TerrainMaterialClass::Translucent,
        _ => TerrainMaterialClass::Opaque,
    }
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
    }
}
