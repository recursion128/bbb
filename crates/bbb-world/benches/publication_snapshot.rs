//! Reproducible cost baselines for immutable simulation world generations.
//!
//! The fixture dimensions mirror the default 12-chunk view distance
//! (`25 * 25 == 625` columns). Each representative column carries all 24
//! Overworld sections, packed block-state payloads, biome palettes, heightmaps,
//! and sky/block light arrays. Mutation cases retain the previous `Arc` while
//! mutating and publishing the next generation so COW work cannot disappear
//! from the measured end-to-end path.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use bbb_protocol::packets::{
    AddEntity, AttributeSnapshot, BlockPos, BlockUpdate, DataComponentPatchSummary,
    EntityDataValue, EntityDataValueKind, EntityEvent, EntityMove, EquipmentSlot,
    EquipmentSlotUpdate, ItemStackSummary, MobEffectFlags, RegistryData, RegistryDataEntry,
    RemoveEntities, SetEntityData, SetEquipment, SetPassengers, UpdateAttributes, UpdateMobEffect,
    Vec3d,
};
use bbb_protocol::{
    attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
    entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    entity_types::{
        VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    },
    mob_effect_types::VANILLA_MOB_EFFECT_SPEED_ID,
};
use bbb_world::{
    ChunkColumn, ChunkPos, ChunkSection, ChunkState, HeightmapData, LightData, PaletteDomain,
    PaletteKind, PalettedContainerData, WorldStore,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

const CHUNK_COUNTS: [usize; 3] = [81, 289, 625];
const ENTITY_COUNTS: [usize; 3] = [100, 500, 2_000];
const ENTITY_COUNT: usize = 2_000;
const SPARSE_ENTITY_COUNT: usize = 8;
const CHURN_ENTITY_COUNT: usize = 16;
const SECTION_COUNT: usize = 24;
const LIGHT_SECTION_COUNT: usize = SECTION_COUNT + 2;

fn block_states() -> PalettedContainerData {
    PalettedContainerData {
        domain: PaletteDomain::BlockStates,
        bits_per_entry: 15,
        palette_kind: PaletteKind::Global,
        palette_global_ids: Vec::new(),
        packed_data: vec![0; 1_024],
        entry_count: 4_096,
    }
}

fn biomes() -> PalettedContainerData {
    PalettedContainerData {
        domain: PaletteDomain::Biomes,
        bits_per_entry: 0,
        palette_kind: PaletteKind::SingleValue,
        palette_global_ids: vec![0],
        packed_data: Vec::new(),
        entry_count: 64,
    }
}

fn representative_chunk(pos: ChunkPos) -> ChunkColumn {
    ChunkColumn {
        pos,
        state: ChunkState::Decoded,
        heightmaps: vec![HeightmapData {
            kind_id: 1,
            data: vec![0; 37],
        }],
        sections: (0..SECTION_COUNT)
            .map(|_| ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: block_states(),
                biomes: biomes(),
            })
            .collect(),
        block_entities: Vec::new(),
        light: LightData {
            sky_y_mask: vec![(1_i64 << LIGHT_SECTION_COUNT) - 1],
            block_y_mask: vec![(1_i64 << LIGHT_SECTION_COUNT) - 1],
            empty_sky_y_mask: Vec::new(),
            empty_block_y_mask: Vec::new(),
            sky_updates: vec![vec![0xff; 2_048]; LIGHT_SECTION_COUNT],
            block_updates: vec![vec![0; 2_048]; LIGHT_SECTION_COUNT],
        },
    }
}

fn build_chunk_world(count: usize) -> WorldStore {
    let side = count.isqrt();
    assert_eq!(
        side * side,
        count,
        "representative chunk count must be square"
    );
    let offset = side / 2;
    let mut world = WorldStore::new();
    for z in 0..side {
        for x in 0..side {
            world.insert_decoded_chunk(representative_chunk(ChunkPos {
                x: x as i32 - offset as i32,
                z: z as i32 - offset as i32,
            }));
        }
    }
    assert_eq!(world.chunk_count(), count);
    world
}

fn entity_packet(id: i32, index: usize, entity_type_id: i32) -> AddEntity {
    AddEntity {
        id,
        uuid: uuid::Uuid::from_u128(0x1000_0000_0000_0000_0000_0000_0000_0000 + id as u128),
        entity_type_id,
        position: Vec3d {
            x: (index % 64) as f64,
            y: 64.0,
            z: (index / 64) as f64,
        },
        delta_movement: Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}

fn build_entity_world_with(
    count: usize,
    mut entity_type_for_index: impl FnMut(usize) -> i32,
) -> WorldStore {
    let mut world = WorldStore::new();
    for index in 0..count {
        let id = index as i32 + 1;
        world.apply_add_entity(entity_packet(id, index, entity_type_for_index(index)));
    }
    assert_eq!(world.entity_count(), count);
    world
}

fn build_entity_world(count: usize) -> WorldStore {
    build_entity_world_with(count, |_| VANILLA_ENTITY_TYPE_ZOMBIE_ID)
}

fn build_heterogeneous_entity_world(count: usize) -> WorldStore {
    build_entity_world_with(count, |index| {
        VANILLA_ENTITY_TYPES[index % VANILLA_ENTITY_TYPES.len()].id
    })
}

fn entity_move(id: i32, delta: i16) -> EntityMove {
    EntityMove {
        id,
        has_position: true,
        delta_x: delta,
        delta_y: 0,
        delta_z: -delta,
        y_rot: Some(f32::from(delta)),
        x_rot: Some(f32::from(-delta)),
        on_ground: true,
    }
}

fn build_passenger_interpolation_world(count: usize) -> WorldStore {
    assert_eq!(count % 2, 0);
    let mut world = build_entity_world_with(count, |index| {
        if index % 2 == 0 {
            VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID
        } else {
            VANILLA_ENTITY_TYPE_ZOMBIE_ID
        }
    });
    for pair in 0..count / 2 {
        let vehicle_id = (pair * 2 + 1) as i32;
        let passenger_id = vehicle_id + 1;
        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id,
            passenger_ids: vec![passenger_id],
        }));
    }
    for id in 1..=count as i32 {
        assert!(world.apply_entity_move(entity_move(id, 128)));
    }
    world
}

fn build_session_static_world() -> WorldStore {
    let mut world = WorldStore::new();
    for registry_index in 0..32 {
        world.record_registry_data(RegistryData {
            registry: format!("benchmark:registry_{registry_index}"),
            entries: (0..256)
                .map(|entry_index| RegistryDataEntry {
                    id: format!("benchmark:entry_{registry_index}_{entry_index}"),
                    raw_data: None,
                })
                .collect(),
            raw_payload_len: 0,
        });
    }
    world.set_default_item_max_stack_sizes((0..2_000).map(|item_id| (item_id, 64)).collect());
    world.set_default_item_max_damage((0..2_000).map(|item_id| (item_id, item_id + 1)).collect());
    world.set_default_item_crafting_remainders(
        (0..2_000)
            .map(|item_id| (item_id, (item_id + 1) % 2_000))
            .collect::<BTreeMap<_, _>>(),
    );
    world
}

fn benchmark_configuration() -> Criterion {
    Criterion::default()
        .without_plots()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
}

fn bench_unchanged_publication(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_publication/unchanged");
    let empty = WorldStore::new();
    group.bench_function("empty", |b| {
        b.iter(|| black_box(Arc::new(empty.publication_snapshot())));
    });
    for count in CHUNK_COUNTS {
        let world = build_chunk_world(count);
        group.bench_with_input(BenchmarkId::new("chunks", count), &count, |b, _| {
            b.iter(|| black_box(Arc::new(world.publication_snapshot())));
        });
    }
    for count in ENTITY_COUNTS {
        let entities = build_entity_world(count);
        group.bench_with_input(BenchmarkId::new("entities", count), &count, |b, _| {
            b.iter(|| black_box(Arc::new(entities.publication_snapshot())));
        });
    }
    let session_static = build_session_static_world();
    group.bench_function("session_static/8192_registry_entries_2000_items", |b| {
        b.iter(|| black_box(Arc::new(session_static.publication_snapshot())));
    });
    group.finish();
}

fn bench_entity_tick_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/tick_only");
    let mut empty = WorldStore::new();
    group.bench_function("empty", |b| {
        b.iter(|| empty.advance_entity_client_animations(black_box(1)));
    });
    for count in ENTITY_COUNTS {
        let mut world = build_entity_world(count);
        group.bench_with_input(BenchmarkId::new("zombies", count), &count, |b, _| {
            b.iter(|| world.advance_entity_client_animations(black_box(1)));
        });
    }
    let mut heterogeneous = build_heterogeneous_entity_world(ENTITY_COUNT);
    group.bench_function("heterogeneous/2000", |b| {
        b.iter(|| heterogeneous.advance_entity_client_animations(black_box(1)));
    });
    group.finish();
}

fn bench_entity_tick_and_publication(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/tick_and_publish");
    let mut empty = WorldStore::new();
    let mut empty_held = Arc::new(empty.publication_snapshot());
    group.bench_function("empty", |b| {
        b.iter(|| {
            empty.advance_entity_client_animations(black_box(1));
            empty_held = black_box(Arc::new(empty.publication_snapshot()));
        });
    });
    black_box(empty_held);
    for count in ENTITY_COUNTS {
        let mut world = build_entity_world(count);
        let mut held = Arc::new(world.publication_snapshot());
        group.bench_with_input(BenchmarkId::new("zombies", count), &count, |b, _| {
            b.iter(|| {
                world.advance_entity_client_animations(black_box(1));
                held = black_box(Arc::new(world.publication_snapshot()));
            });
        });
        black_box(held);
    }
    let mut heterogeneous = build_heterogeneous_entity_world(ENTITY_COUNT);
    let mut held = Arc::new(heterogeneous.publication_snapshot());
    group.bench_function("heterogeneous/2000", |b| {
        b.iter(|| {
            heterogeneous.advance_entity_client_animations(black_box(1));
            held = black_box(Arc::new(heterogeneous.publication_snapshot()));
        });
    });
    black_box(held);
    group.finish();
}

fn bench_entity_retained_snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_publication/retained_snapshot");
    let world = build_entity_world(ENTITY_COUNT);
    let mut held = Arc::new(world.publication_snapshot());
    group.bench_function("next_generation/2000_entities", |b| {
        b.iter(|| {
            held = black_box(Arc::new(world.publication_snapshot()));
        });
    });
    black_box(held);
    group.finish();
}

fn bench_entity_sparse_mutation(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_mutation/sparse");
    let mut world = build_entity_world(ENTITY_COUNT);
    let mut held = Arc::new(world.publication_snapshot());
    let mut delta = 1_i16;
    group.bench_function("transform_metadata_8_of_2000_then_publish", |b| {
        b.iter(|| {
            delta = -delta;
            for id in 1..=SPARSE_ENTITY_COUNT as i32 {
                assert!(world.apply_entity_move(entity_move(id, delta)));
                assert!(world.apply_set_entity_data(SetEntityData {
                    id,
                    values: vec![EntityDataValue {
                        data_id: 0,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                        value: EntityDataValueKind::Byte(delta as i8),
                    }],
                }));
            }
            held = black_box(Arc::new(world.publication_snapshot()));
        });
    });
    black_box(held);
    group.finish();
}

fn bench_entity_mutation_families(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_mutation/families");
    let mut world = build_entity_world(ENTITY_COUNT);
    let mut held = Arc::new(world.publication_snapshot());
    let mut value = 1_i8;
    group.bench_function(
        "metadata_equipment_effect_attribute_event_then_publish",
        |b| {
            b.iter(|| {
                value = -value;
                assert!(world.apply_set_entity_data(SetEntityData {
                    id: 1,
                    values: vec![EntityDataValue {
                        data_id: 0,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                        value: EntityDataValueKind::Byte(value),
                    }],
                }));
                assert!(world.apply_set_equipment(SetEquipment {
                    entity_id: 2,
                    slots: vec![EquipmentSlotUpdate {
                        slot: EquipmentSlot::MainHand,
                        item: if value > 0 {
                            ItemStackSummary {
                                item_id: Some(1),
                                count: 1,
                                component_patch: DataComponentPatchSummary::default(),
                            }
                        } else {
                            ItemStackSummary::empty()
                        },
                    }],
                }));
                assert!(world.apply_update_mob_effect(UpdateMobEffect {
                    entity_id: 3,
                    effect_id: VANILLA_MOB_EFFECT_SPEED_ID,
                    amplifier: i32::from(value > 0),
                    duration_ticks: 200,
                    flags: MobEffectFlags::default(),
                }));
                assert!(world.apply_update_attributes(UpdateAttributes {
                    entity_id: 4,
                    attributes: vec![AttributeSnapshot {
                        attribute_id: VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                        base: 20.0 + f64::from(value),
                        modifiers: Vec::new(),
                    }],
                }));
                assert!(world.apply_entity_event(EntityEvent {
                    entity_id: 5,
                    event_id: 2,
                }));
                held = black_box(Arc::new(world.publication_snapshot()));
            });
        },
    );
    black_box(held);
    group.finish();
}

fn bench_entity_spawn_remove_churn(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_mutation/spawn_remove_churn");
    let mut world = build_entity_world(ENTITY_COUNT);
    let mut held = Arc::new(world.publication_snapshot());
    let churn_ids =
        ((ENTITY_COUNT - CHURN_ENTITY_COUNT + 1) as i32..=ENTITY_COUNT as i32).collect::<Vec<_>>();
    group.bench_function("16_of_2000_then_publish", |b| {
        b.iter(|| {
            assert_eq!(
                world.apply_remove_entities(RemoveEntities {
                    entity_ids: churn_ids.clone(),
                }),
                CHURN_ENTITY_COUNT
            );
            for &id in &churn_ids {
                world.apply_add_entity(entity_packet(
                    id,
                    usize::try_from(id - 1).unwrap(),
                    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                ));
            }
            held = black_box(Arc::new(world.publication_snapshot()));
        });
    });
    black_box(held);
    group.finish();
}

fn bench_entity_interpolation_and_passengers(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/interpolation_passengers");
    let mut world = build_passenger_interpolation_world(ENTITY_COUNT);
    let mut held = Arc::new(world.publication_snapshot());
    group.bench_function("1000_vehicles_1000_passengers_then_publish", |b| {
        b.iter(|| {
            world.advance_entity_client_animations(black_box(1));
            held = black_box(Arc::new(world.publication_snapshot()));
        });
    });
    black_box(held);
    group.finish();
}

fn bench_entity_effect_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_tick/effect_generation");
    let mut world =
        build_entity_world_with(ENTITY_COUNT, |_| VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID);
    group.bench_function("2000_firework_trails", |b| {
        b.iter(|| {
            world.advance_entity_client_animations(black_box(1));
            black_box(world.take_firework_rocket_trail_particle_states());
        });
    });
    group.finish();
}

fn bench_entity_projection(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_projection");
    let world = build_heterogeneous_entity_world(ENTITY_COUNT);
    group.bench_function("full_entity_state/2000", |b| {
        b.iter(|| {
            for id in 1..=ENTITY_COUNT as i32 {
                black_box(world.probe_entity(black_box(id)));
            }
        });
    });
    group.bench_function("narrow_transforms/2000", |b| {
        b.iter(|| black_box(world.entity_transforms()));
    });
    group.bench_function("protocol_lookup_narrow_transform/2000", |b| {
        b.iter(|| {
            for id in 1..=ENTITY_COUNT as i32 {
                black_box(world.probe_entity_transform(black_box(id)));
            }
        });
    });
    group.bench_function("renderer_model_sources/2000", |b| {
        b.iter(|| black_box(world.entity_model_sources_at_partial_tick(black_box(0.5))));
    });
    group.finish();
}

fn bench_entity_protocol_lookup_mutation(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_mutation/protocol_lookup");
    let mut world = build_entity_world(ENTITY_COUNT);
    let mut delta = 1_i16;
    group.bench_function("1000_transform_updates_of_2000", |b| {
        b.iter(|| {
            delta = -delta;
            for id in 1..=1_000 {
                assert!(world.apply_entity_move(entity_move(black_box(id), delta)));
            }
        });
    });
    group.finish();
}

fn bench_mutation_then_publication(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_publication/mutation_then_publish");

    let mut block_world = build_chunk_world(625);
    let mut held = Arc::new(block_world.publication_snapshot());
    let mut block_state_id = 1;
    group.bench_function("single_block/625_chunks", |b| {
        b.iter(|| {
            block_state_id = if block_state_id == 0 { 1 } else { 0 };
            assert!(block_world.apply_block_update(BlockUpdate {
                pos: BlockPos { x: 0, y: 0, z: 0 },
                block_state_id,
            }));
            let next = Arc::new(block_world.publication_snapshot());
            held = black_box(next);
        });
    });
    black_box(held);

    group.finish();
}

fn bench_retained_snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("world_publication/retained_snapshot");
    let world = build_chunk_world(625);
    let mut held = Arc::new(world.publication_snapshot());
    group.bench_function("next_generation/625_chunks", |b| {
        b.iter(|| {
            let next = Arc::new(world.publication_snapshot());
            held = black_box(next);
        });
    });
    black_box(held);
    group.finish();
}

criterion_group! {
    name = benches;
    config = benchmark_configuration();
    targets =
        bench_unchanged_publication,
        bench_entity_tick_only,
        bench_entity_tick_and_publication,
        bench_entity_retained_snapshot,
        bench_entity_sparse_mutation,
        bench_entity_mutation_families,
        bench_entity_spawn_remove_churn,
        bench_entity_interpolation_and_passengers,
        bench_entity_effect_generation,
        bench_entity_projection,
        bench_entity_protocol_lookup_mutation,
        bench_mutation_then_publication,
        bench_retained_snapshot
}
criterion_main!(benches);
