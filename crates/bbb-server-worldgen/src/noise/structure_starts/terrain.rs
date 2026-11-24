use std::collections::btree_map::Entry;

use super::*;

use crate::noise::beardifier::TerrainAdjustingStart;
use crate::noise::end_city::END_CITY_TYPE_ID;
use crate::noise::igloo::IGLOO_TYPE_ID;
use crate::noise::nether_fortress::FORTRESS_TYPE_ID;
use crate::noise::nether_fossil::NETHER_FOSSIL_TYPE_ID;
use crate::noise::ocean_monument::OCEAN_MONUMENT_TYPE_ID;
use crate::noise::ocean_ruin::OCEAN_RUIN_TYPE_ID;
use crate::noise::ruined_portal::RUINED_PORTAL_TYPE_ID;
use crate::noise::scattered::{DESERT_PYRAMID_TYPE_ID, JUNGLE_TEMPLE_TYPE_ID, SWAMP_HUT_TYPE_ID};
use crate::noise::shipwreck::SHIPWRECK_TYPE_ID;
use crate::noise::stronghold::STRONGHOLD_TYPE_ID;
use crate::noise::woodland_mansion::WOODLAND_MANSION_TYPE_ID;

const BURIED_TREASURE_TYPE_ID: &str = "minecraft:buried_treasure";
const JIGSAW_TYPE_ID: &str = "minecraft:jigsaw";
const MINESHAFT_TYPE_ID: &str = "minecraft:mineshaft";

impl NoiseStructureChunkData {
    /// Loads the terrain-adjusting starts selected by the target's References
    /// in registry order (`StructureManager.java:50-80`, `Beardifier.java:38-87`).
    pub fn resolve_referenced_terrain_starts<E>(
        &self,
        plan: &NoiseStructurePlan,
        mut source_at: impl FnMut(i32, i32) -> Result<NoiseStructureChunkData, E>,
    ) -> Result<Vec<TerrainAdjustingStart>, E> {
        let mut sources = BTreeMap::new();
        let mut resolved = Vec::new();
        for structure_id in plan.structure_order() {
            let (Some(positions), Some(definition)) = (
                self.references.get(structure_id),
                plan.structures().get(structure_id),
            ) else {
                continue;
            };
            if definition.terrain_adjustment == StructureTerrainAdjustment::None {
                continue;
            }
            for &packed in positions {
                let source = unpack_chunk(packed);
                let source_data = if source == self.chunk {
                    self
                } else {
                    match sources.entry(source) {
                        Entry::Occupied(entry) => &*entry.into_mut(),
                        Entry::Vacant(entry) => &*entry.insert(source_at(source[0], source[1])?),
                    }
                };
                resolved.extend(source_data.terrain_start(
                    structure_id,
                    &definition.type_id,
                    definition.terrain_adjustment,
                ));
            }
        }
        Ok(resolved)
    }

    pub fn resolve_referenced_terrain_starts_borrowed<'a, E>(
        &self,
        plan: &NoiseStructurePlan,
        mut source_at: impl FnMut(i32, i32) -> Result<&'a NoiseStructureChunkData, E>,
    ) -> Result<Vec<TerrainAdjustingStart>, E> {
        let mut resolved = Vec::new();
        for structure_id in plan.structure_order() {
            let (Some(positions), Some(definition)) = (
                self.references.get(structure_id),
                plan.structures().get(structure_id),
            ) else {
                continue;
            };
            if definition.terrain_adjustment == StructureTerrainAdjustment::None {
                continue;
            }
            for &packed in positions {
                let source = unpack_chunk(packed);
                let source_data = if source == self.chunk {
                    self
                } else {
                    source_at(source[0], source[1])?
                };
                resolved.extend(source_data.terrain_start(
                    structure_id,
                    &definition.type_id,
                    definition.terrain_adjustment,
                ));
            }
        }
        Ok(resolved)
    }

    fn terrain_start(
        &self,
        structure_id: &str,
        type_id: &str,
        terrain_adjustment: StructureTerrainAdjustment,
    ) -> Option<TerrainAdjustingStart> {
        if type_id == JIGSAW_TYPE_ID {
            if let Some(start) = self
                .starts
                .iter()
                .find(|start| start.structure_id() == structure_id)
            {
                return Some(TerrainAdjustingStart::Jigsaw {
                    start: start.clone(),
                });
            }
            let restored = self.restored_bounds.get(structure_id)?;
            return (!restored.pieces.is_empty()).then_some(TerrainAdjustingStart::Rigid {
                terrain_adjustment,
                pieces: restored.pieces.clone(),
            });
        }
        if !matches!(
            type_id,
            BURIED_TREASURE_TYPE_ID
                | MINESHAFT_TYPE_ID
                | SHIPWRECK_TYPE_ID
                | OCEAN_RUIN_TYPE_ID
                | IGLOO_TYPE_ID
                | DESERT_PYRAMID_TYPE_ID
                | JUNGLE_TEMPLE_TYPE_ID
                | SWAMP_HUT_TYPE_ID
                | RUINED_PORTAL_TYPE_ID
                | OCEAN_MONUMENT_TYPE_ID
                | WOODLAND_MANSION_TYPE_ID
                | STRONGHOLD_TYPE_ID
                | FORTRESS_TYPE_ID
                | NETHER_FOSSIL_TYPE_ID
                | END_CITY_TYPE_ID
        ) {
            return None;
        }
        let pieces = self.raw_start_geometry(structure_id)?;
        (!pieces.is_empty()).then_some(TerrainAdjustingStart::Rigid {
            terrain_adjustment,
            pieces,
        })
    }
}
