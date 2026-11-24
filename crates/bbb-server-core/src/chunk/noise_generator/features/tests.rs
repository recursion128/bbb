mod fossil_template;
mod structure_item_publication;

use super::super::tests::overworld_program;
use super::{
    FeaturePlacementSource, FeatureRegionData, FeatureWorldGenRegion, NoiseFinalDensityProgram,
};
use crate::block_entity::{saved_record, saved_record_with_resolver};
use crate::chunk::ChunkStore;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_identity,
};
use bbb_protocol::block_types::{
    VANILLA_BLOCK_BEE_NEST_ID, VANILLA_BLOCK_CHEST_ID, VANILLA_BLOCK_SPAWNER_ID,
    VANILLA_BLOCK_SUSPICIOUS_SAND_ID,
};
use bbb_protocol::packets::BlockPos;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::{Java26ChunkContext, Java26ChunkData, Java26ChunkStatus};
use bbb_server_types::save::SaveResult;
use bbb_server_worldgen::noise::buried_treasure::BURIED_TREASURE_LOOT_TABLE;
use bbb_server_worldgen::noise::ocean_monument::OCEAN_MONUMENT_ID;
use bbb_server_worldgen::noise::stronghold::STRONGHOLD_ID;
use bbb_server_worldgen::noise::woodland_mansion::WOODLAND_MANSION_ID;
use bbb_server_worldgen::WorldGenLevel;
use std::sync::Arc;

pub(crate) fn with_feature_region(
    program: &Arc<NoiseFinalDensityProgram>,
    target: [i32; 2],
    place: impl for<'a> FnOnce(
        &mut FeatureWorldGenRegion<'a>,
        &FeaturePlacementSource<'a>,
    ) -> SaveResult<()>,
) -> ChunkStore {
    let mut store =
        ChunkStore::new_noise_carvers_with_structure_generation(program.clone(), false).unwrap();
    let chunks = crate::chunk::carvers_region_coordinates(target)
        .unwrap()
        .into_iter()
        .map(|[x, z]| (x, z))
        .collect();
    for &(x, z) in &chunks {
        store.try_load_column_unlit(x, z).unwrap();
    }
    let source = program.feature_placement_source(target).unwrap();
    let mut data = FeatureRegionData::new(
        &mut store.chunks,
        &mut store.block_entities,
        &mut store.poi,
        &mut store.pending_light_changes,
        &chunks,
    );
    let mut region = data.region(&source, target).unwrap();
    place(&mut region, &source).unwrap();
    let _ = region;
    drop(data);
    store
}

fn target_blocks(region: &FeatureWorldGenRegion<'_>) -> Vec<i32> {
    let [cx, cz] = region.target_chunk();
    (region.get_min_y()..=region.get_max_y())
        .flat_map(|y| {
            (0..16).flat_map(move |z| {
                (0..16).map(move |x| {
                    region.get_block(BlockPos {
                        x: cx * 16 + x,
                        y,
                        z: cz * 16 + z,
                    })
                })
            })
        })
        .collect()
}

fn block_entity_records(
    store: &ChunkStore,
    target: [i32; 2],
    program: &NoiseFinalDensityProgram,
) -> Vec<Vec<NbtSummaryEntry>> {
    store
        .block_entities()
        .chunk((target[0], target[1]))
        .map(|entity| match program.item_publication() {
            Some(publication) => saved_record_with_resolver(entity, publication.resolver()),
            None => saved_record(entity),
        })
        .collect::<bbb_server_types::Result<_>>()
        .unwrap()
}

pub(crate) fn parsed_carvers(
    store: &ChunkStore,
    target: [i32; 2],
    program: &NoiseFinalDensityProgram,
) -> Java26ChunkData {
    let key = (target[0], target[1]);
    let mut document = bbb_server_persistence::generation_state_chunk_document(
        &store.loaded(key).column,
        target[0],
        target[1],
        &|id| program.biome_names.get(id as usize).cloned(),
        store.state(key),
        None,
        store.proto_entity_records(key.0, key.1),
    )
    .unwrap();
    for field in document.entries_mut() {
        if field.name == "block_entities" {
            field.value = bbb_protocol::NbtSummaryValue::List(
                block_entity_records(store, target, program)
                    .into_iter()
                    .map(bbb_protocol::NbtSummaryValue::Compound)
                    .collect(),
            );
        }
    }
    Java26ChunkData::parse(
        document,
        Java26ChunkContext {
            expected_x: target[0],
            expected_z: target[1],
            min_section_y: store.loaded(key).column.min_y().div_euclid(16),
            section_count: store.loaded(key).column.section_count(),
        },
    )
    .unwrap()
}

pub(crate) fn structure_cache(
    target: [i32; 2],
    source: bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData,
) -> crate::chunk::structure_status::NoiseStructureStatusCache {
    use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;
    let source_chunk = source.chunk();
    let empty = NoiseStructureChunkData::from_supplied_starts(target[0], target[1], [], []);
    let target_data = if source_chunk == target {
        &source
    } else {
        &empty
    };
    let (references, sources) = NoiseStructureChunkData::references_from_complete_source_window(
        target_data,
        |x, z| -> Result<_, String> {
            Ok(if [x, z] == source_chunk {
                source.clone()
            } else {
                NoiseStructureChunkData::from_supplied_starts(x, z, [], [])
            })
        },
    )
    .unwrap();
    let mut cache = crate::chunk::structure_status::NoiseStructureStatusCache::new(true);
    for data in sources.into_values() {
        cache.restore_chunk_component(data).unwrap();
    }
    cache.restore_chunk_component(references).unwrap();
    cache
}

/// The first seed with an Overworld woodland mansion near the origin that
/// `accept` admits, with the chunk it starts in and its starts.
pub(super) fn find_mansion(
    accept: impl Fn(
        &bbb_server_worldgen::noise::woodland_mansion::WoodlandMansionStructureStart,
    ) -> bool,
) -> (
    i64,
    [i32; 2],
    Vec<bbb_server_worldgen::noise::woodland_mansion::WoodlandMansionStructureStart>,
) {
    (0..64)
        .find_map(|seed| {
            let (chunk, starts) = overworld_program(true, seed)
                .first_overworld_woodland_mansion_start_in_window(128)
                .unwrap()?;
            accept(&starts[0]).then_some((seed, chunk, starts))
        })
        .expect("a seed with a matching woodland mansion")
}

#[test]
fn production_woodland_mansion_invocation_places_in_canonical_columns() {
    let (seed, target, starts) = find_mansion(|_| true);
    let program = overworld_program(true, seed);
    assert_eq!(starts.len(), 1);
    assert_eq!(starts[0].structure_id(), WOODLAND_MANSION_ID);
    let invocation = program
        .woodland_mansion_structure_decoration_invocation(target)
        .unwrap();
    let source_chunk = starts[0].start_chunk();
    let source = bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData::from_supplied_starts(
        source_chunk[0], source_chunk[1], [], [],
    ).with_woodland_mansion_starts(starts);
    let mut cache = structure_cache(target, source);
    with_feature_region(&program, target, |region, _source| {
        let before = target_blocks(region);
        program.structure_generation_context().unwrap()
        .place_referenced_structure::<bbb_server_worldgen::noise::woodland_mansion::WoodlandMansionStructureStart>(
            &mut cache, &invocation, region,
        )?;
        assert_ne!(target_blocks(region), before);
        Ok(())
    });
}

#[test]
fn owned_jigsaw_features_structure_is_plan_derived_including_decoration_step() {
    let program = overworld_program(true, 0);
    assert_eq!(
        program.owned_jigsaw_features_structure(Some("minecraft:village_plains")),
        Some("minecraft:village_plains")
    );
    assert_eq!(
        program.owned_jigsaw_features_structure(Some("minecraft:trial_chambers")),
        Some("minecraft:trial_chambers")
    );
    assert_eq!(
        program.owned_jigsaw_features_structure(Some("minecraft:ancient_city")),
        Some("minecraft:ancient_city")
    );
    assert_eq!(
        program.owned_jigsaw_features_structure(Some("minecraft:mineshaft")),
        None
    );
    assert_eq!(
        program
            .jigsaw_structure_decoration_invocation([0, 0], "minecraft:ancient_city")
            .unwrap()
            .step,
        7
    );
    program
        .generate_jigsaw_structure_starts_component(-660, -471, &[])
        .unwrap();
}

#[test]
fn production_jigsaw_invocation_places_trial_chambers_in_canonical_columns() {
    let program = overworld_program(true, 0);
    let target = program.jigsaw_start_chunk("minecraft:trial_chambers", "minecraft:trial_chambers");
    let starts = program
        .generate_jigsaw_structure_starts_component(target[0], target[1], &[])
        .unwrap()
        .starts()
        .to_vec();
    assert_eq!(starts.len(), 1);
    assert_eq!(starts[0].structure_id(), "minecraft:trial_chambers");
    let invocation = program
        .jigsaw_structure_decoration_invocation(target, "minecraft:trial_chambers")
        .unwrap();
    assert_eq!(invocation.step, 3);
    let source_chunk = starts[0].start_chunk();
    let source = bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData::from_supplied_starts(
        source_chunk[0], source_chunk[1], [], [],
    ).with_jigsaw_starts(starts);
    let mut cache = structure_cache(target, source);
    with_feature_region(&program, target, |region, _source| {
        let before = target_blocks(region);
        program.structure_generation_context().unwrap()
        .place_referenced_structure::<bbb_server_worldgen::noise::structure_starts::JigsawStructureStart>(
            &mut cache, &invocation, region,
        )?;
        assert_ne!(target_blocks(region), before);
        Ok(())
    });
}

#[test]
fn production_stronghold_invocation_places_in_canonical_columns() {
    let program = overworld_program(true, 0);
    let (target, starts) = program
        .first_overworld_stronghold_start_in_window(256)
        .unwrap()
        .expect("seed 0 has an Overworld stronghold start in the search window");
    assert_eq!(starts.len(), 1);
    assert_eq!(starts[0].structure_id(), STRONGHOLD_ID);
    let invocation = program
        .stronghold_structure_decoration_invocation(target)
        .unwrap();
    let source_chunk = starts[0].start_chunk();
    let source = bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData::from_supplied_starts(
        source_chunk[0], source_chunk[1], [], [],
    ).with_stronghold_starts(starts);
    let mut cache = structure_cache(target, source);
    with_feature_region(&program, target, |region, _source| {
        let before = target_blocks(region);
        program.structure_generation_context().unwrap()
        .place_referenced_structure::<bbb_server_worldgen::noise::stronghold::StrongholdStructureStart>(
            &mut cache, &invocation, region,
        )?;
        assert_ne!(target_blocks(region), before);
        Ok(())
    });
}

#[test]
fn production_ocean_monument_invocation_places_in_canonical_columns() {
    let program = overworld_program(true, 0);
    let (target, starts) = program
        .first_overworld_ocean_monument_start_in_window(256)
        .unwrap()
        .expect("seed 0 has an Overworld ocean-monument start in the search window");
    assert_eq!(starts.len(), 1);
    assert_eq!(starts[0].structure_id(), OCEAN_MONUMENT_ID);
    let invocation = program
        .leftover_ocean_monument_structure_decoration_invocation(target)
        .unwrap();
    let source_chunk = starts[0].start_chunk();
    let source = bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData::from_supplied_starts(
        source_chunk[0], source_chunk[1], [], [],
    ).with_ocean_monument_starts(starts);
    let mut cache = structure_cache(target, source);
    with_feature_region(&program, target, |region, _source| {
        let before = target_blocks(region);
        program.structure_generation_context().unwrap()
        .place_referenced_structure::<bbb_server_worldgen::noise::ocean_monument::OceanMonumentStructureStart>(
            &mut cache, &invocation, region,
        )?;
        assert_ne!(target_blocks(region), before);
        Ok(())
    });
}

#[test]
fn production_buried_treasure_places_and_persists_its_chest_owner() {
    let program = overworld_program(true, 0);
    let target = program.buried_treasure_chunk();
    let mut start = program
        .generate_buried_treasure_start_component(target[0], target[1], false)
        .unwrap()
        .unwrap();
    let mut chest = None;
    let store = with_feature_region(&program, target, |region, _source| {
        chest = start
            .piece_mut()
            .place(region, 17)
            .map(|placement| placement.chest_position());
        Ok(())
    });
    let chest = chest.expect("supported treasure substrate");
    assert_eq!(
        vanilla_block_state_identity(
            store
                .loaded((target[0], target[1]))
                .column
                .get_block_state(chest.x.rem_euclid(16), chest.y, chest.z.rem_euclid(16))
                .unwrap()
        )
        .unwrap()
        .name,
        "minecraft:chest"
    );
    let parsed = parsed_carvers(&store, target, &program);
    assert_eq!(parsed.status(), Java26ChunkStatus::Carvers);
    assert_eq!(parsed.block_entities().len(), 1);
    let record = &parsed.block_entities()[0];
    assert!(record.iter().any(|entry| entry.name == "LootTable"
        && entry.value == NbtSummaryValue::string(BURIED_TREASURE_LOOT_TABLE)));
    assert!(record
        .iter()
        .any(|entry| entry.name == "LootTableSeed" && entry.value == NbtSummaryValue::Long(17)));
}

/// The chunk nearest the origin (by square rings) whose feature region,
/// built by `place`, satisfies `accept`.
fn first_feature_target(
    program: &Arc<NoiseFinalDensityProgram>,
    place: impl for<'a> Fn(
        &mut FeatureWorldGenRegion<'a>,
        &FeaturePlacementSource<'a>,
    ) -> SaveResult<()>,
    accept: impl Fn(&ChunkStore, [i32; 2]) -> bool,
) -> ([i32; 2], ChunkStore) {
    (0..=12)
        .flat_map(|ring: i32| {
            (-ring..=ring).flat_map(move |x| {
                (-ring..=ring)
                    .filter(move |z| x.abs().max(z.abs()) == ring)
                    .map(move |z| [x, z])
            })
        })
        .find_map(|target| {
            let store =
                with_feature_region(program, target, |region, source| place(region, source));
            accept(&store, target).then_some((target, store))
        })
        .expect("a chunk near the origin whose features match")
}

fn column_has(
    column: &bbb_server_types::chunk_data::ChunkColumn,
    accept: impl Fn(&str) -> bool,
) -> bool {
    (column.min_y()..column.min_y() + column.section_count() as i32 * 16).any(|y| {
        (0..16).any(|z| {
            (0..16).any(|x| {
                vanilla_block_state_identity(column.get_block_state(x, y, z).unwrap())
                    .is_some_and(|state| accept(state.name.as_str()))
            })
        })
    })
}

#[test]
fn built_in_local_modifications_place_amethyst_in_canonical_columns() {
    let program = overworld_program(true, 0);
    first_feature_target(
        &program,
        |region, source| region.place_feature_step(source, 2),
        |store, _| {
            store
                .chunks
                .values()
                .any(|chunk| column_has(&chunk.column, |name| name == "minecraft:amethyst_block"))
        },
    );
}

#[test]
fn production_top_layer_modification_freezes_a_cold_overworld_chunk() {
    let program = overworld_program(true, 0);
    let len = i32::try_from(bbb_server_types::chunk_data::VANILLA_26_1_BIOME_REGISTRY_LEN).unwrap();
    let snowy = (0..len)
        .filter(|id| {
            program.biome_name(*id).is_some_and(|name| {
                matches!(
                    name.as_str(),
                    "minecraft:snowy_plains" | "minecraft:snowy_taiga" | "minecraft:frozen_river"
                )
            })
        })
        .collect();
    let (found, _) = program
        .find_closest_biome_3d(BlockPos { x: 0, y: 64, z: 0 }, &snowy, 6_400, 32, 64)
        .expect("a snowy biome near the origin");
    let target = [found.x >> 4, found.z >> 4];
    let store = with_feature_region(&program, target, |region, source| {
        region.place_feature_step(source, 10)
    });
    let column = &store.loaded((target[0], target[1])).column;
    assert!(column_has(column, |name| matches!(
        name,
        "minecraft:ice" | "minecraft:snow"
    )));
}

#[test]
fn end_gateway_feature_exit_survives_generated_record_reload() {
    use crate::block_entity::BlockEntity;
    use bbb_registry_codec::legacy_random::LegacyRandomSource;
    let program = overworld_program(true, 0);
    let resources = program.feature_resources.as_ref().unwrap();
    let retained =
        resources.registries.worldgen.configured_features["minecraft:end_gateway_return"].clone();
    let origin = BlockPos { x: 8, y: 100, z: 8 };
    let exit = BlockPos {
        x: 100,
        y: 50,
        z: 0,
    };
    let store = with_feature_region(&program, [0, 0], |region, _source| {
        assert!(bbb_server_worldgen::place_configured_feature(
            &retained,
            region,
            origin,
            &mut LegacyRandomSource::with_seed(0)
        ));
        Ok(())
    });
    let parsed = parsed_carvers(&store, [0, 0], &program);
    let record = parsed
        .block_entities()
        .iter()
        .find(|entries| {
            entries.iter().any(|entry| {
                entry.name == "id"
                    && entry.value == NbtSummaryValue::string("minecraft:end_gateway")
            })
        })
        .unwrap();
    let state = store
        .loaded((0, 0))
        .column
        .get_block_state(origin.x, origin.y, origin.z)
        .unwrap();
    let loaded = BlockEntity::load_static(origin, state, record)
        .unwrap()
        .unwrap();
    let gateway = loaded.end_gateway().unwrap();
    assert_eq!(gateway.exit_portal(), Some(exit));
    assert!(gateway.exact_teleport());
}

#[test]
fn feature_block_entity_adapter_materializes_and_serializes_metadata() {
    let program = overworld_program(true, 0);
    let chest = BlockPos { x: 1, y: 2, z: 3 };
    let brushable = BlockPos { x: 2, y: 2, z: 3 };
    let spawner = BlockPos { x: 3, y: 2, z: 3 };
    let hive = BlockPos { x: 4, y: 2, z: 3 };
    let store = with_feature_region(&program, [0, 0], |region, _source| {
        for (pos, block) in [
            (chest, VANILLA_BLOCK_CHEST_ID),
            (brushable, VANILLA_BLOCK_SUSPICIOUS_SAND_ID),
            (spawner, VANILLA_BLOCK_SPAWNER_ID),
            (hive, VANILLA_BLOCK_BEE_NEST_ID),
        ] {
            assert!(region.set_block(pos, vanilla_block_default_state_id(block).unwrap(), 2));
        }
        region.set_block_entity_loot_table(chest, "minecraft:chests/simple_dungeon", 17);
        region.set_brushable_block_loot_table(brushable, "minecraft:archaeology/desert_well", 23);
        region.set_spawner_entity_id(spawner, "minecraft:zombie");
        region.store_bee_nest_occupant(hive, 31);
        region.store_bee_nest_occupant(hive, 37);
        Ok(())
    });
    let records = block_entity_records(&store, [0, 0], &program);
    assert_eq!(records.len(), 4);
    assert!(records
        .iter()
        .any(|record| record.iter().any(|entry| entry.name == "LootTable")));
    assert!(records
        .iter()
        .any(|record| record.iter().any(|entry| entry.name == "SpawnData")));
    assert!(records
        .iter()
        .any(|record| record.iter().any(|entry| entry.name == "bees"
            && matches!(&entry.value,NbtSummaryValue::List(bees) if bees.len()==2))));
}

/// StructureTemplate.java:298-307: `append_loot` data carries no `id` and
/// loads into the suspicious gravel `setBlock` created (trail ruins).
#[test]
fn template_loot_data_without_an_id_loads_into_the_placed_block_entity() {
    let program = overworld_program(true, 0);
    let gravel = BlockPos { x: 5, y: 2, z: 3 };
    let store = with_feature_region(&program, [0, 0], |region, _source| {
        let state = vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_SUSPICIOUS_GRAVEL_ID,
        )
        .unwrap();
        assert!(region.set_block(gravel, state, 2));
        region.load_structure_block_entity(
            gravel,
            state,
            &[
                NbtSummaryEntry {
                    name: "LootTable".into(),
                    value: NbtSummaryValue::String(
                        "minecraft:archaeology/trail_ruins_common".into(),
                    ),
                },
                NbtSummaryEntry {
                    name: "LootTableSeed".into(),
                    value: NbtSummaryValue::Long(9),
                },
            ],
        );
        Ok(())
    });
    let records = block_entity_records(&store, [0, 0], &program);
    assert!(records.iter().any(|record| record.iter().any(|entry| {
        entry.name == "LootTable"
            && entry.value == NbtSummaryValue::string("minecraft:archaeology/trail_ruins_common")
    })));
}

#[test]
fn placed_features_preserve_ticks_and_block_entities_in_carvers_save() {
    let program = overworld_program(true, 0);
    let (target, store) = first_feature_target(
        &program,
        |region, source| {
            for step in 0..source.step_count() {
                region.place_feature_step(source, step)?;
            }
            Ok(())
        },
        |store, target| {
            let column = &store.loaded((target[0], target[1])).column;
            (!column.block_ticks().pack(0).is_empty() || !column.fluid_ticks().pack(0).is_empty())
                && store
                    .block_entities()
                    .chunk((target[0], target[1]))
                    .next()
                    .is_some()
        },
    );
    let parsed = parsed_carvers(&store, target, &program);
    let column = &store.loaded((target[0], target[1])).column;
    let block_ticks = column
        .block_ticks()
        .pack(0)
        .into_iter()
        .map(|tick| tick.to_saved())
        .collect::<Vec<_>>();
    let fluid_ticks = column
        .fluid_ticks()
        .pack(0)
        .into_iter()
        .map(|tick| tick.to_saved())
        .collect::<Vec<_>>();
    assert!(!block_ticks.is_empty() || !fluid_ticks.is_empty());
    assert!(!parsed.block_entities().is_empty());
    assert_eq!(parsed.block_ticks(), block_ticks);
    assert_eq!(parsed.fluid_ticks(), fluid_ticks);
    assert_eq!(
        parsed.block_entities(),
        block_entity_records(&store, target, &program)
    );
}
