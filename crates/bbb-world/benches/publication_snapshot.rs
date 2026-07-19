//! Reproducible cost baselines for immutable simulation world generations.
//!
//! The fixture dimensions mirror the default 12-chunk view distance
//! (`25 * 25 == 625` columns). Each representative column carries all 24
//! Overworld sections, packed block-state payloads, biome palettes, heightmaps,
//! and sky/block light arrays. Mutation cases retain the previous `Arc` while
//! mutating and publishing the next generation so COW work cannot disappear
//! from the measured end-to-end path.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
use bbb_protocol::packets::{
    AddEntity, BlockPos, BlockUpdate, RegistryData, RegistryDataEntry, Vec3d,
};
use bbb_world::{
    ChunkColumn, ChunkPos, ChunkSection, ChunkState, HeightmapData, LightData, PaletteDomain,
    PaletteKind, PalettedContainerData, WorldStore,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

const CHUNK_COUNTS: [usize; 3] = [81, 289, 625];
const ENTITY_COUNT: usize = 2_000;
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

fn build_entity_world(count: usize) -> WorldStore {
    let mut world = WorldStore::new();
    for index in 0..count {
        let id = index as i32 + 1;
        world.apply_add_entity(AddEntity {
            id,
            uuid: uuid::Uuid::from_u128(0x1000_0000_0000_0000_0000_0000_0000_0000 + id as u128),
            entity_type_id: VANILLA_ENTITY_TYPE_ZOMBIE_ID,
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
        });
    }
    assert_eq!(world.entity_count(), count);
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
    let entities = build_entity_world(ENTITY_COUNT);
    group.bench_function("entities/2000", |b| {
        b.iter(|| black_box(Arc::new(entities.publication_snapshot())));
    });
    let session_static = build_session_static_world();
    group.bench_function("session_static/8192_registry_entries_2000_items", |b| {
        b.iter(|| black_box(Arc::new(session_static.publication_snapshot())));
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

    let mut entity_world = build_entity_world(ENTITY_COUNT);
    let mut held = Arc::new(entity_world.publication_snapshot());
    group.bench_function("entity_client_tick/2000_entities", |b| {
        b.iter(|| {
            entity_world.advance_entity_client_animations(black_box(1));
            let next = Arc::new(entity_world.publication_snapshot());
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
    targets = bench_unchanged_publication, bench_mutation_then_publication, bench_retained_snapshot
}
criterion_main!(benches);
