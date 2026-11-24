//! Buried-treasure placement candidate, generation anchor and biome gate.

use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::BlockPos;

use super::{BuriedTreasureGenerationStub, BuriedTreasureStructureStart, BURIED_TREASURE_ID};
use crate::noise::structures::{NoiseStructurePlan, StructureHeightmap, StructureStartSetOwner};
use std::collections::BTreeSet;

impl NoiseStructurePlan {
    /// `ChunkGenerator.createStructures` (`ChunkGenerator.java:476-589`) and
    /// `BuriedTreasureStructure.findGenerationPoint`
    /// (`BuriedTreasureStructure.java:19-31`).
    #[allow(clippy::too_many_arguments)]
    pub fn buried_treasure_generation_stub(
        &self,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        has_existing_valid_start: bool,
        first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        biome_at: impl FnMut(BlockPos) -> Option<String>,
    ) -> Option<BuriedTreasureGenerationStub> {
        if has_existing_valid_start {
            return None;
        }
        self.generate_missing_random_spread_buried_treasure_stubs(
            level_seed,
            chunk_x,
            chunk_z,
            &[],
            first_occupied_height,
            biome_at,
        )
        .into_iter()
        .find(|stub| stub.structure_id() == BURIED_TREASURE_ID)
    }

    /// Preserves structure-set registry order and weighted retry for every
    /// registered buried-treasure owner (`ChunkGenerator.java:464-589`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_missing_random_spread_buried_treasure_stubs(
        &self,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[BuriedTreasureStructureStart],
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
    ) -> Vec<BuriedTreasureGenerationStub> {
        let mut existing_ids: BTreeSet<String> = existing_starts
            .iter()
            .map(|start| start.structure_id().to_string())
            .collect();
        let mut generated = Vec::new();
        for set in self.sets() {
            if self.structure_start_owner(set) != StructureStartSetOwner::BuriedTreasure
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
                let Some(stub) = self.buried_treasure_generation_stub_for(
                    &structure_id,
                    chunk_x,
                    chunk_z,
                    &mut first_occupied_height,
                    &mut biome_at,
                    true,
                ) else {
                    continue;
                };
                existing_ids.insert(structure_id);
                generated.push(stub);
                break;
            }
        }
        generated
    }

    /// `PlaceCommand.placeStructure` (`PlaceCommand.java:277-296`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_forced_buried_treasure_start(
        &self,
        structure_id: &str,
        _level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        mut first_occupied_height: impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
    ) -> Option<BuriedTreasureStructureStart> {
        self.buried_treasure_generation_stub_for(
            structure_id,
            chunk_x,
            chunk_z,
            &mut first_occupied_height,
            &mut biome_at,
            false,
        )
        .map(|stub| BuriedTreasureStructureStart::from_generation_stub(stub, 0))
    }

    fn buried_treasure_generation_stub_for(
        &self,
        structure_id: &str,
        chunk_x: i32,
        chunk_z: i32,
        first_occupied_height: &mut impl FnMut(i32, i32, StructureHeightmap) -> Option<i32>,
        biome_at: &mut impl FnMut(BlockPos) -> Option<String>,
        check_allowed_biome: bool,
    ) -> Option<BuriedTreasureGenerationStub> {
        let definition = self.structures().get(structure_id)?;
        let min_x = chunk_x * 16;
        let min_z = chunk_z * 16;
        let anchor_y =
            first_occupied_height(min_x + 8, min_z + 8, StructureHeightmap::OceanFloorWg)?;
        let generation_anchor = BlockPos {
            x: min_x + 8,
            y: anchor_y,
            z: min_z + 8,
        };
        // Generation-stub biome validation samples its anchor (`Structure.java:121-140`).
        let biome = normalize_resource_location(biome_at(generation_anchor)?).ok()?;
        if check_allowed_biome && !definition.biomes.contains(&biome) {
            return None;
        }
        Some(
            BuriedTreasureGenerationStub::new_registered(
                structure_id,
                [chunk_x, chunk_z],
                generation_anchor,
                BlockPos {
                    x: min_x + 9,
                    y: 90,
                    z: min_z + 9,
                },
            )
            .with_terrain_adjustment(definition.terrain_adjustment),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::noise::biome::NoiseBiomeSource;
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

    #[test]
    fn buried_treasure_start_candidate_uses_exact_anchor_and_piece_position() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = plan.first_candidate(crate::noise::buried_treasure::BURIED_TREASURE_SET_ID, 0);
        let (x, z) = (chunk[0] * 16 + 8, chunk[1] * 16 + 8);
        let mut height_queries = Vec::new();
        let mut biome_queries = Vec::new();
        let stub = plan
            .buried_treasure_generation_stub(
                0,
                chunk[0],
                chunk[1],
                false,
                |x, z, heightmap| {
                    height_queries.push((x, z, heightmap));
                    Some(63)
                },
                |position| {
                    biome_queries.push(position);
                    Some("minecraft:beach".to_string())
                },
            )
            .unwrap();

        assert_eq!(stub.source_chunk(), chunk);
        assert_eq!(stub.generation_anchor(), BlockPos { x, y: 63, z });
        assert_eq!(
            stub.piece().position(),
            BlockPos {
                x: x + 1,
                y: 90,
                z: z + 1,
            }
        );
        assert_eq!(height_queries, [(x, z, StructureHeightmap::OceanFloorWg)]);
        assert_eq!(biome_queries, [stub.generation_anchor()]);
    }

    #[test]
    fn buried_treasure_start_biome_gate_accepts_only_definition_biomes() {
        let startup = startup();
        let plan = overworld_plan(&startup);
        let chunk = plan.first_candidate(crate::noise::buried_treasure::BURIED_TREASURE_SET_ID, 0);
        let allowed = plan.buried_treasure_generation_stub(
            0,
            chunk[0],
            chunk[1],
            false,
            |_, _, _| Some(64),
            |_| Some("minecraft:beach".to_string()),
        );
        let disallowed = plan.buried_treasure_generation_stub(
            0,
            chunk[0],
            chunk[1],
            false,
            |_, _, _| Some(64),
            |_| Some("minecraft:plains".to_string()),
        );

        assert!(allowed.is_some());
        assert_eq!(disallowed, None);
    }
}
