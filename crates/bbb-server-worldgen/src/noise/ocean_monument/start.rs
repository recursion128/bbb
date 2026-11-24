//! Ocean-monument `StructureStart` materialization and surround/biome gates.

use crate::random::large_feature_random;
use std::collections::BTreeSet;

use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::BlockPos;

use super::{
    OceanMonumentGenerationStub, OceanMonumentPiece, OceanMonumentStructureStart, BUILDING_DEPTH,
    BUILDING_HEIGHT, BUILDING_MIN_Y, BUILDING_WEST_NORTH_OFFSET, BUILDING_WIDTH, HORIZONTAL,
    SURROUND_OFFSET, SURROUND_RADIUS,
};
use crate::noise::scattered::make_bounding_box;
use crate::noise::structures::{NoiseStructurePlan, StructureHeightmap, StructureStartSetOwner};

impl NoiseStructurePlan {
    /// Runs the ocean-monument random-spread half of `createStructures` and
    /// biome-gates the completed generation stub (`ChunkGenerator.java:476-589`,
    /// `OceanMonumentStructure.java:30-52`, `Structure.java:121-140`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_missing_random_spread_ocean_monument_starts(
        &self,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[OceanMonumentStructureStart],
        sea_level: i32,
        surrounding: &BTreeSet<String>,
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
    ) -> Vec<OceanMonumentStructureStart> {
        let mut existing_ids: BTreeSet<String> = existing_starts
            .iter()
            .map(|start| start.structure_id().to_string())
            .collect();

        let mut generated = Vec::new();
        for set in self.sets() {
            if self.structure_start_owner(set) != StructureStartSetOwner::OceanMonument
                || set
                    .structures
                    .iter()
                    .any(|selection| existing_ids.contains(&selection.structure))
            {
                continue;
            }
            let Some(attempts) =
                self.random_spread_attempt_order(&set.id, level_seed, chunk_x, chunk_z)
            else {
                continue;
            };
            for structure_id in attempts {
                let Some(stub) = self.ocean_monument_generation_stub(
                    &structure_id,
                    level_seed,
                    chunk_x,
                    chunk_z,
                    sea_level,
                    surrounding,
                    &mut first_occupied_height,
                    &mut biome_at,
                    true,
                ) else {
                    continue;
                };
                let terrain_adjustment = self.structures().get(&structure_id).map_or(
                    crate::noise::structures::StructureTerrainAdjustment::None,
                    |def| def.terrain_adjustment,
                );
                let start = OceanMonumentStructureStart::from_generation_stub(stub, 0)
                    .with_terrain_adjustment(terrain_adjustment);
                existing_ids.insert(structure_id);
                generated.push(start);
                break;
            }
        }
        generated
    }

    /// `PlaceCommand.placeStructure` (`PlaceCommand.java:277-294`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_forced_ocean_monument_start(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        sea_level: i32,
        surrounding: &BTreeSet<String>,
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
    ) -> Option<OceanMonumentStructureStart> {
        let terrain_adjustment = self.structures().get(structure_id).map_or(
            crate::noise::structures::StructureTerrainAdjustment::None,
            |def| def.terrain_adjustment,
        );
        self.ocean_monument_generation_stub(
            structure_id,
            level_seed,
            chunk_x,
            chunk_z,
            sea_level,
            surrounding,
            &mut first_occupied_height,
            &mut biome_at,
            false,
        )
        .map(|stub| {
            OceanMonumentStructureStart::from_generation_stub(stub, 0)
                .with_terrain_adjustment(terrain_adjustment)
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn ocean_monument_generation_stub(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        sea_level: i32,
        surrounding: &BTreeSet<String>,
        first_occupied_height: &mut impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        biome_at: &mut impl FnMut(BlockPos) -> Option<String>,
        require_allowed_biome: bool,
    ) -> Option<OceanMonumentGenerationStub> {
        let definition = self.structures().get(structure_id)?;
        let min_x = chunk_x * 16;
        let min_z = chunk_z * 16;
        // `OceanMonumentStructure.findGenerationPoint` (`OceanMonumentStructure.java:31-39`).
        let sampled = biomes_within(
            min_x + SURROUND_OFFSET,
            sea_level,
            min_z + SURROUND_OFFSET,
            SURROUND_RADIUS,
            &mut *biome_at,
        )?;
        if sampled
            .iter()
            .any(|biome| !surrounding.contains(biome.as_str()))
        {
            return None;
        }

        let anchor_x = min_x + 8;
        let anchor_z = min_z + 8;
        let generation_anchor = BlockPos {
            x: anchor_x,
            y: first_occupied_height(anchor_x, anchor_z, StructureHeightmap::OceanFloorWg)?,
            z: anchor_z,
        };
        let biome = normalize_resource_location(biome_at(generation_anchor)?).ok()?;
        if require_allowed_biome && !definition.biomes.contains(&biome) {
            return None;
        }

        let mut random = large_feature_random(level_seed, chunk_x, chunk_z);
        let orientation = HORIZONTAL[random.next_int(HORIZONTAL.len() as i32) as usize];
        let bounding_box = make_bounding_box(
            min_x - BUILDING_WEST_NORTH_OFFSET,
            BUILDING_MIN_Y,
            min_z - BUILDING_WEST_NORTH_OFFSET,
            orientation,
            BUILDING_WIDTH,
            BUILDING_HEIGHT,
            BUILDING_DEPTH,
        );
        Some(OceanMonumentGenerationStub::new(
            [chunk_x, chunk_z],
            generation_anchor,
            structure_id,
            OceanMonumentPiece::new(bounding_box, orientation),
        ))
    }
}

/// `BiomeSource.getBiomesWithin` (`BiomeSource.java:44-68`).
pub fn biomes_within(
    x: i32,
    y: i32,
    z: i32,
    radius: i32,
    mut biome_at: impl FnMut(BlockPos) -> Option<String>,
) -> Option<BTreeSet<String>> {
    let [x0, y0, z0] = [x, y, z].map(|coord| super::quart_from_block(coord - radius));
    let [x1, y1, z1] = [x, y, z].map(|coord| super::quart_from_block(coord + radius));
    let mut biomes = BTreeSet::new();
    for quart_z in z0..=z1 {
        for quart_x in x0..=x1 {
            for quart_y in y0..=y1 {
                let position = BlockPos {
                    x: quart_x * 4,
                    y: quart_y * 4,
                    z: quart_z * 4,
                };
                biomes.insert(normalize_resource_location(biome_at(position)?).ok()?);
            }
        }
    }
    Some(biomes)
}

#[cfg(test)]
mod tests {
    use crate::noise::biome::NoiseBiomeSource;
    use crate::noise::ocean_monument::{
        required_surrounding_biomes, HORIZONTAL, OCEAN_MONUMENT_ID,
    };
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    use super::*;

    fn startup() -> DatapackStartup {
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap()
    }

    fn overworld_plan(startup: &DatapackStartup) -> NoiseStructurePlan {
        let source = NoiseBiomeSource::from_registry(
            &startup.registries,
            "minecraft:overworld",
            "minecraft:overworld",
            0,
        )
        .unwrap();
        NoiseStructurePlan::from_startup(startup, source.overworld_biome_order().unwrap()).unwrap()
    }

    fn forced_ocean() -> impl FnMut(BlockPos) -> Option<String> {
        |_| Some("minecraft:deep_ocean".to_string())
    }

    #[test]
    fn forced_deep_ocean_emits_one_omb_envelope() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = plan.first_candidate(crate::noise::ocean_monument::OCEAN_MONUMENT_SET_ID, 0);
        let surrounding = required_surrounding_biomes(&startup).unwrap();
        let starts = plan.generate_missing_random_spread_ocean_monument_starts(
            0,
            chunk[0],
            chunk[1],
            &[],
            63,
            &surrounding,
            |_, _, heightmap| {
                assert_eq!(heightmap, StructureHeightmap::OceanFloorWg);
                Some(40)
            },
            forced_ocean(),
        );
        assert_eq!(starts.len(), 1);
        assert_eq!(starts[0].structure_id(), OCEAN_MONUMENT_ID);
        assert_eq!(starts[0].start_chunk(), chunk);
        assert_eq!(
            starts[0].piece().bounding_box(),
            [
                chunk[0] * 16 - 29,
                39,
                chunk[1] * 16 - 29,
                chunk[0] * 16 + 28,
                61,
                chunk[1] * 16 + 28
            ]
        );
        assert!(HORIZONTAL.contains(&starts[0].piece().orientation()));
    }

    #[test]
    fn land_biome_in_surround_box_misses() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = plan.first_candidate(crate::noise::ocean_monument::OCEAN_MONUMENT_SET_ID, 0);
        let surrounding = required_surrounding_biomes(&startup).unwrap();
        let starts = plan.generate_missing_random_spread_ocean_monument_starts(
            0,
            chunk[0],
            chunk[1],
            &[],
            63,
            &surrounding,
            |_, _, _| Some(40),
            |_| Some("minecraft:plains".to_string()),
        );
        assert!(starts.is_empty());
    }

    #[test]
    fn surround_hit_then_non_monument_biome_gate_misses() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = plan.first_candidate(crate::noise::ocean_monument::OCEAN_MONUMENT_SET_ID, 0);
        let surrounding = required_surrounding_biomes(&startup).unwrap();
        let starts = plan.generate_missing_random_spread_ocean_monument_starts(
            0,
            chunk[0],
            chunk[1],
            &[],
            63,
            &surrounding,
            |_, _, _| Some(40),
            |position| {
                if position.x == chunk[0] * 16 + 8 && position.z == chunk[1] * 16 + 8 {
                    Some("minecraft:ocean".to_string())
                } else {
                    Some("minecraft:deep_ocean".to_string())
                }
            },
        );
        assert!(starts.is_empty());
    }
}
