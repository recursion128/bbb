use bbb_protocol::entity_types::VanillaEntityMobCategory;
use bbb_protocol::packets::BlockPos;
pub use bbb_server_worldgen::noise::structure_starts::spawn_overrides::StructureSpawnOverrideMatch;
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;

use super::NoiseStructureStatusCache;
use crate::chunk::{ChunkSource, ChunkStore};

impl ChunkStore {
    pub fn natural_spawn_structure_target_at(
        &self,
        position: BlockPos,
    ) -> Option<&NoiseStructureChunkData> {
        let ChunkSource::NoiseCarvers(program) = &self.source else {
            return None;
        };
        program.natural_spawn_structure_plan()?;
        self.structure_status.natural_spawn_target(position)
    }

    // ChunkGenerator.java:437-462.
    pub fn natural_spawn_structure_override_at(
        &self,
        category: VanillaEntityMobCategory,
        position: BlockPos,
    ) -> Option<StructureSpawnOverrideMatch<'_>> {
        let ChunkSource::NoiseCarvers(program) = &self.source else {
            return None;
        };
        let plan = program.natural_spawn_structure_plan()?;
        let target = self.structure_status.natural_spawn_target(position)?;
        target.spawn_override_at(plan, category, position, |x, z| {
            self.structure_component(x, z)
        })
    }

    // NaturalSpawner.java:318-339.
    pub fn natural_spawn_fortress_full_box_contains(&self, position: BlockPos) -> bool {
        let ChunkSource::NoiseCarvers(program) = &self.source else {
            return false;
        };
        let Some(plan) = program.natural_spawn_structure_plan() else {
            return false;
        };
        self.structure_status
            .natural_spawn_target(position)
            .is_some_and(|target| {
                target.fortress_full_box_contains(plan, position, |x, z| {
                    self.structure_component(x, z)
                })
            })
    }
}

impl NoiseStructureStatusCache {
    fn natural_spawn_target(&self, position: BlockPos) -> Option<&NoiseStructureChunkData> {
        self.chunks.get(&[position.x >> 4, position.z >> 4])
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::world_creation::default_overworld_level_source;
    use bbb_server_datapack::debug_generator::DebugLevelSource;
    use bbb_server_worldgen::noise::nether_fortress::{
        FortressPiece, FortressPieceKind, FortressStructureStart, FORTRESS_ID,
    };
    use bbb_server_worldgen::noise::ocean_monument::{
        OceanMonumentPiece, OceanMonumentStructureStart,
    };
    use bbb_server_worldgen::noise::scattered::HorizontalDirection;
    use bbb_server_worldgen::noise::structure_pieces::JigsawBoundingBox;

    use crate::chunk::noise_generator::test_support::{
        fresh_overworld_program, nether_program, overworld_program,
    };
    use bbb_server_types::chunk_data::VANILLA_26_1_BIOME_REGISTRY_LEN;

    use super::*;

    const QUERY: BlockPos = BlockPos { x: 5, y: 64, z: 5 };

    fn empty_component(chunk: [i32; 2]) -> NoiseStructureChunkData {
        NoiseStructureChunkData::from_supplied_starts(chunk[0], chunk[1], [], [])
    }

    fn references_for(
        target: [i32; 2],
        source: &NoiseStructureChunkData,
    ) -> NoiseStructureChunkData {
        let target_component = if source.chunk() == target {
            source.clone()
        } else {
            empty_component(target)
        };
        NoiseStructureChunkData::references_from_complete_source_window(
            &target_component,
            |source_x, source_z| {
                Ok::<_, &str>(if source.chunk() == [source_x, source_z] {
                    source.clone()
                } else {
                    empty_component([source_x, source_z])
                })
            },
        )
        .unwrap()
        .0
    }

    fn install(
        store: &mut ChunkStore,
        target: NoiseStructureChunkData,
        sources: impl IntoIterator<Item = NoiseStructureChunkData>,
    ) {
        let mut status = NoiseStructureStatusCache::new(true);
        for source in sources {
            status
                .restore_chunk_component_at_status(source, true, false)
                .unwrap();
        }
        status
            .restore_chunk_component_at_status(target, true, true)
            .unwrap();
        store.structure_status = status;
    }

    fn monument_source(chunk: [i32; 2], bounds: [i32; 6]) -> NoiseStructureChunkData {
        let start = OceanMonumentStructureStart::from_parts(
            "minecraft:monument",
            chunk,
            0,
            OceanMonumentPiece::new(bounds, HorizontalDirection::South),
        );
        empty_component(chunk).with_ocean_monument_starts([start])
    }

    #[test]
    fn natural_spawn_keeps_present_empty_override() {
        let program = overworld_program(true, 0);
        let mut store =
            ChunkStore::new_overworld_carvers_with_structure_generation(Arc::clone(&program), true)
                .unwrap();
        let source = monument_source([0, 0], [0, 40, 0, 15, 80, 15]);
        let target = references_for([0, 0], &source);

        install(&mut store, target, []);
        assert_eq!(
            store
                .natural_spawn_structure_target_at(QUERY)
                .unwrap()
                .chunk(),
            [0, 0]
        );
        let monsters = store
            .natural_spawn_structure_override_at(VanillaEntityMobCategory::Monster, QUERY)
            .unwrap();
        assert_eq!(monsters.structure_id, "minecraft:monument");
        assert_eq!(monsters.override_.spawns.entries.len(), 1);
        let empty = store
            .natural_spawn_structure_override_at(VanillaEntityMobCategory::Axolotls, QUERY)
            .unwrap();
        assert_eq!(empty.structure_id, "minecraft:monument");
        assert!(empty.override_.spawns.entries.is_empty());
    }

    #[test]
    fn natural_spawn_flat_and_debug_sources_have_no_structure_matches() {
        let flat = ChunkStore::new(default_overworld_level_source().unwrap(), 0);
        assert!(flat.natural_spawn_structure_target_at(QUERY).is_none());
        assert!(flat
            .natural_spawn_structure_override_at(VanillaEntityMobCategory::Monster, QUERY)
            .is_none());
        assert!(!flat.natural_spawn_fortress_full_box_contains(QUERY));

        let debug = ChunkStore::new_debug_with_biome_registry_and_skylight(
            DebugLevelSource::new(-64, 384).unwrap(),
            0,
            VANILLA_26_1_BIOME_REGISTRY_LEN,
            true,
        );
        assert!(debug.natural_spawn_structure_target_at(QUERY).is_none());
        assert!(debug
            .natural_spawn_structure_override_at(VanillaEntityMobCategory::Monster, QUERY)
            .is_none());
        assert!(!debug.natural_spawn_fortress_full_box_contains(QUERY));
    }

    #[test]
    fn natural_spawn_noise_source_without_structure_plan_has_no_matches() {
        let mut program = fresh_overworld_program(true, 17);
        Arc::get_mut(&mut program)
            .unwrap()
            .disable_natural_spawn_structures_for_test();
        let store =
            ChunkStore::new_overworld_carvers_with_structure_generation(program, true).unwrap();
        assert!(store.natural_spawn_structure_target_at(QUERY).is_none());
        assert!(store
            .natural_spawn_structure_override_at(VanillaEntityMobCategory::Monster, QUERY)
            .is_none());
        assert!(!store.natural_spawn_fortress_full_box_contains(QUERY));
    }

    #[test]
    fn natural_spawn_disabled_generation_queries_restored_structures_and_empty_targets() {
        let mut store = ChunkStore::new_overworld_carvers_with_structure_generation(
            overworld_program(true, 17),
            false,
        )
        .unwrap();
        let source = monument_source([0, 0], [0, 40, 0, 15, 80, 15]);
        let target = references_for([0, 0], &source);
        store
            .structure_status
            .restore_chunk_component_at_status(target, true, true)
            .unwrap();
        assert!(!store.structure_status.generate_structures());
        assert_eq!(
            store
                .natural_spawn_structure_target_at(QUERY)
                .unwrap()
                .chunk(),
            [0, 0]
        );
        assert_eq!(
            store
                .natural_spawn_structure_override_at(VanillaEntityMobCategory::Monster, QUERY)
                .unwrap()
                .structure_id,
            "minecraft:monument"
        );
        let empty_query = BlockPos { x: 32, y: 64, z: 5 };
        store
            .structure_status
            .restore_chunk_component_at_status(empty_component([2, 0]), true, true)
            .unwrap();
        assert!(store
            .natural_spawn_structure_target_at(empty_query)
            .is_some());
        assert!(store
            .natural_spawn_structure_override_at(VanillaEntityMobCategory::Monster, empty_query)
            .is_none());
    }

    #[test]
    fn natural_spawn_fortress_query_uses_the_full_start_box() {
        let program = nether_program(0);
        let plan = program.natural_spawn_structure_plan().unwrap();
        let start = FortressStructureStart::from_parts(
            [0, 0],
            0,
            plan.structures()[FORTRESS_ID].terrain_adjustment,
            vec![FortressPiece {
                kind: FortressPieceKind::Start,
                generation_depth: 0,
                bounding_box: JigsawBoundingBox {
                    min: BlockPos { x: 0, y: 64, z: 0 },
                    max: BlockPos {
                        x: 10,
                        y: 70,
                        z: 10,
                    },
                },
                orientation: HorizontalDirection::South,
                random_seed: 0,
                chest_pending: false,
                spawner_placed: false,
            }],
        );
        let source = empty_component([0, 0]).with_fortress_starts([start]);
        let target = references_for([0, 0], &source);
        let mut store =
            ChunkStore::new_nether_carvers_with_structure_generation(program, true).unwrap();
        install(&mut store, target, []);
        assert!(store.natural_spawn_fortress_full_box_contains(QUERY));
        assert!(!store.natural_spawn_fortress_full_box_contains(BlockPos { x: 11, y: 64, z: 5 }));
    }
}
