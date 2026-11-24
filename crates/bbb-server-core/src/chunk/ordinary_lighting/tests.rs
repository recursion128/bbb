use super::*;
use crate::world_creation::default_overworld_level_source;
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use std::collections::BTreeMap;
use std::time::Instant;

fn store() -> ChunkStore {
    ChunkStore::new(default_overworld_level_source().unwrap(), 0)
}

fn state(name: &str) -> i32 {
    vanilla_block_state_id_by_name_and_properties(name, &BTreeMap::new()).unwrap()
}

#[test]
fn ordinary_height_query_reads_current_blocks_before_relight() {
    use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;
    use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;

    let mut store = store();
    store.try_ensure_served_full_columns(&[(0, 0)]).unwrap();
    let kind = VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;
    let original = store
        .final_heightmap_first_available(0, 0, kind, 8, 8)
        .unwrap();
    store
        .set_block_state(8, 100, 8, state("minecraft:stone"))
        .unwrap();
    assert_eq!(
        store.final_heightmap_first_available(0, 0, kind, 8, 8),
        Some(101)
    );
    store
        .set_block_state(8, 100, 8, state("minecraft:air"))
        .unwrap();
    assert_eq!(
        store.final_heightmap_first_available(0, 0, kind, 8, 8),
        Some(original)
    );

    let start = Instant::now();
    for _ in 0..32 {
        assert_eq!(
            FeatureHeightmaps::prime(std::hint::black_box(store.column(0, 0).unwrap()))
                .first_available(kind, 8, 8),
            Some(original),
        );
    }
    let full_scan = start.elapsed();
    let start = Instant::now();
    for _ in 0..32 {
        assert_eq!(
            std::hint::black_box(&store).final_heightmap_first_available(0, 0, kind, 8, 8),
            Some(original)
        );
    }
    eprintln!(
        "32 height queries: full maps={full_scan:?}, point scan={:?}",
        start.elapsed()
    );
}

#[test]
fn ordinary_live_border_light_and_save_dirty_survive_multiple_flushes() {
    let mut store = store();
    store
        .try_ensure_served_full_columns(&[(0, 0), (1, 0)])
        .unwrap();
    assert!(store.take_live_light_updates().unwrap().is_empty());
    assert_eq!(store.loaded_chunks().collect::<Vec<_>>(), [(0, 0), (1, 0)]);
    assert_eq!(store.sky_light_level(-1, 100, 0), 0);
    assert_eq!(store.sky_light_level(8, 100, 8), 15);
    assert_eq!(store.block_light_level(16, 100, 8), 0);
    store
        .set_block_state(15, 100, 8, state("minecraft:glowstone"))
        .unwrap();
    assert!(store.full_chunk_packet_data(0, 0).is_err());
    store.flush_live_relights().unwrap();
    assert_eq!(store.block_light_level(16, 100, 8), 14);
    store
        .set_block_state(15, 100, 8, state("minecraft:air"))
        .unwrap();
    store.flush_live_relights().unwrap();
    assert_eq!(store.block_light_level(16, 100, 8), 0);
    let updates = store.take_live_light_updates().unwrap();
    assert_eq!(
        updates
            .iter()
            .map(|u| (u.chunk_x, u.chunk_z))
            .collect::<Vec<_>>(),
        [(0, 0), (1, 0)]
    );
    assert!(store.take_live_light_updates().unwrap().is_empty());
    assert_eq!(
        store.ordinary_derived_dirty_chunks().collect::<Vec<_>>(),
        [(0, 0), (1, 0)]
    );
    assert_eq!(store.loaded_chunks().count(), 2);
}

#[test]
fn ordinary_restore_rebuilds_once_quietly_from_all_saved_neighbors() {
    let mut original = store();
    original
        .try_ensure_served_full_columns(&[(0, 0), (1, 0)])
        .unwrap();
    original
        .set_block_state(15, 100, 8, state("minecraft:glowstone"))
        .unwrap();
    original.flush_live_relights().unwrap();
    let mut restored = store();
    restored.begin_ordinary_light_restore();
    for &key in &[(1, 0), (0, 0)] {
        restored.insert_restored_column(
            key.0,
            key.1,
            original.column(key.0, key.1).unwrap().clone(),
            original.generation_state(key.0, key.1).unwrap().clone(),
        );
    }
    assert!(restored.full_chunk_packet_data(0, 0).is_err());
    assert!(restored.try_ensure_served_full_columns(&[(0, 0)]).is_err());
    restored.finish_ordinary_light_restore().unwrap();
    assert_eq!(restored.block_light_level(16, 100, 8), 14);
    for (x, z) in restored.loaded_chunks() {
        assert_eq!(
            restored.full_chunk_packet_data(x, z).unwrap(),
            original.full_chunk_packet_data(x, z).unwrap()
        );
    }
    assert!(restored.take_live_light_updates().unwrap().is_empty());
    assert!(restored.ordinary_derived_dirty_chunks().next().is_none());
    assert!(restored.pending_initial_persistence().next().is_none());
    assert_eq!(restored.loaded_chunks().count(), 2);
}

#[test]
fn ordinary_batch_and_individual_loads_publish_identical_light_without_halo() {
    let columns = (-1..=1)
        .flat_map(|x| (-1..=1).map(move |z| (x, z)))
        .collect::<Vec<_>>();
    let mut individual = store();
    let start = Instant::now();
    for &(x, z) in &columns {
        individual.try_load_column(x, z).unwrap();
    }
    let individual_time = start.elapsed();
    let mut batch = store();
    let start = Instant::now();
    batch.try_ensure_served_full_columns(&columns).unwrap();
    let batch_time = start.elapsed();
    for &(x, z) in &columns {
        assert_eq!(
            individual.full_chunk_packet_data(x, z).unwrap(),
            batch.full_chunk_packet_data(x, z).unwrap()
        );
    }
    assert_eq!(
        batch.loaded_chunks().collect::<BTreeSet<_>>(),
        columns.into_iter().collect::<BTreeSet<_>>()
    );
    assert_eq!(individual.loaded_chunks().count(), 9);
    eprintln!("ordinary 9-column load: individual={individual_time:?}, batch={batch_time:?}");
}

impl ChunkStore {
    /// Discards all light and pending checks, then lights every loaded FULL
    /// column from its sources.
    fn relight_from_scratch(&mut self) {
        self.pending_light_changes.clear();
        self.light_storage = Default::default();
        let columns = self.unlit_ordinary_columns();
        self.light_new_columns(&columns, &BTreeSet::new()).unwrap();
    }
}

fn assert_same_loaded_light(local: &ChunkStore, full: &ChunkStore) {
    assert_eq!(
        local.loaded_chunks().collect::<BTreeSet<_>>(),
        full.loaded_chunks().collect::<BTreeSet<_>>()
    );
    for (x, z) in local.loaded_chunks() {
        assert_eq!(
            local.full_chunk_packet_data(x, z).unwrap(),
            full.full_chunk_packet_data(x, z).unwrap(),
            "packet {x},{z}"
        );
        assert_eq!(
            local.saved_light_sections((x, z)),
            full.saved_light_sections((x, z)),
            "saved layers {x},{z}"
        );
        for y in [-100, -64, -17, 0, 63, 79, 95, 100, 159, 199, 319, 400] {
            for (dx, dz) in [(0, 0), (8, 8), (15, 15)] {
                let pos = (x * 16 + dx, y, z * 16 + dz);
                assert_eq!(
                    local.sky_light_level(pos.0, pos.1, pos.2),
                    full.sky_light_level(pos.0, pos.1, pos.2),
                    "sky {pos:?}"
                );
                assert_eq!(
                    local.block_light_level(pos.0, pos.1, pos.2),
                    full.block_light_level(pos.0, pos.1, pos.2),
                    "block {pos:?}"
                );
            }
        }
    }
}

#[test]
fn ordinary_local_relight_matches_full_for_border_sources_roofs_and_new_neighbors() {
    for skylight in [false, true] {
        let mut local = ChunkStore::new_with_biome_registry_and_skylight(
            default_overworld_level_source().unwrap(),
            0,
            bbb_server_types::chunk_data::VANILLA_26_1_BIOME_REGISTRY_LEN,
            skylight,
        );
        let mut full = ChunkStore::new_with_biome_registry_and_skylight(
            default_overworld_level_source().unwrap(),
            0,
            bbb_server_types::chunk_data::VANILLA_26_1_BIOME_REGISTRY_LEN,
            skylight,
        );
        for store in [&mut local, &mut full] {
            store
                .try_ensure_served_full_columns(&[(0, 0), (1, 0), (1, 1), (8, 8)])
                .unwrap();
        }
        for name in ["minecraft:glowstone", "minecraft:stone", "minecraft:air"] {
            for pos in [(15, 100, 15), (16, 200, 16), (15, 319, 15)] {
                local
                    .set_block_state(pos.0, pos.1, pos.2, state(name))
                    .unwrap();
                full.set_block_state(pos.0, pos.1, pos.2, state(name))
                    .unwrap();
            }
            let far = local.full_chunk_packet_data(8, 8).err();
            assert!(far.is_some());
            let far_layers = local.saved_light_sections((8, 8));
            full.relight_from_scratch();
            local.flush_live_relights().unwrap();
            assert_eq!(local.saved_light_sections((8, 8)), far_layers);
            assert_same_loaded_light(&local, &full);
        }
        local
            .set_block_state(15, 100, 8, state("minecraft:glowstone"))
            .unwrap();
        full.set_block_state(15, 100, 8, state("minecraft:glowstone"))
            .unwrap();
        local.flush_live_relights().unwrap();
        local.try_load_column(0, 1).unwrap();
        full.try_load_column(0, 1).unwrap();
        full.relight_from_scratch();
        assert_same_loaded_light(&local, &full);
    }
}

#[test]
fn ordinary_local_relight_removes_last_nonempty_topology_without_losing_distant_islands() {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    use bbb_server_datapack::flat_generator::{FlatLevelGeneratorSettings, FlatLevelSource};
    let settings = FlatLevelGeneratorSettings::parse(
        &NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "layers".into(),
            value: NbtSummaryValue::List(Vec::new()),
        }]),
        "test.flat",
    )
    .unwrap();
    let source = FlatLevelSource::new(settings, -64, 384).unwrap();
    let mut local = ChunkStore::new(source.clone(), 0);
    let mut full = ChunkStore::new(source, 0);
    for store in [&mut local, &mut full] {
        store.begin_ordinary_light_restore();
        for chunk in [(0, 0), (1, 0), (9, 9)] {
            store.try_load_column(chunk.0, chunk.1).unwrap();
        }
        store
            .set_block_state(15, -64, 8, state("minecraft:stone"))
            .unwrap();
        store
            .set_block_state(144, 200, 144, state("minecraft:glowstone"))
            .unwrap();
        store.finish_ordinary_light_restore().unwrap();
    }
    assert_eq!(local.light_storage.bottom_section_y(), Some(-5));
    local
        .set_block_state(15, -64, 8, state("minecraft:air"))
        .unwrap();
    full.set_block_state(15, -64, 8, state("minecraft:air"))
        .unwrap();
    full.relight_from_scratch();
    local.flush_live_relights().unwrap();
    assert_eq!(
        local.light_storage.bottom_section_y(),
        full.light_storage.bottom_section_y()
    );
    assert_eq!(local.light_storage.bottom_section_y(), Some(11));
    assert_same_loaded_light(&local, &full);
}

#[test]
fn ordinary_local_relight_feedback_matches_full_for_a_large_loaded_region() {
    let mut local = store();
    let columns = (-3..=3)
        .flat_map(|x| (-3..=3).map(move |z| (x, z)))
        .collect::<Vec<_>>();
    local.try_ensure_served_full_columns(&columns).unwrap();
    local
        .set_block_state(15, 100, 8, state("minecraft:glowstone"))
        .unwrap();
    let mut full = store();
    full.try_ensure_served_full_columns(&columns).unwrap();
    full.set_block_state(15, 100, 8, state("minecraft:glowstone"))
        .unwrap();
    let start = Instant::now();
    full.relight_from_scratch();
    let full_time = start.elapsed();
    let start = Instant::now();
    local.flush_live_relights().unwrap();
    let local_time = start.elapsed();
    assert_same_loaded_light(&local, &full);
    eprintln!("ordinary existing-column edit in 49 loaded columns: full={full_time:?}, local={local_time:?}");
}

#[test]
fn ordinary_incremental_relight_matches_full_for_a_random_edit_sequence() {
    let slab = vanilla_block_state_id_by_name_and_properties(
        "minecraft:oak_slab",
        &BTreeMap::from([
            ("type".to_string(), "bottom".to_string()),
            ("waterlogged".to_string(), "false".to_string()),
        ]),
    )
    .unwrap();
    let palette = [
        state("minecraft:stone"),
        state("minecraft:glowstone"),
        state("minecraft:air"),
        state("minecraft:glass"),
        slab,
    ];
    let mut local = store();
    let mut full = store();
    let columns = (-1..=1)
        .flat_map(|x| (-1..=1).map(move |z| (x, z)))
        .collect::<Vec<_>>();
    local.try_ensure_served_full_columns(&columns).unwrap();
    full.try_ensure_served_full_columns(&columns).unwrap();
    let mut seed = 0x2545_f491_u64;
    let mut next = |bound: u64| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((seed >> 33) % bound) as i32
    };
    for round in 0..6 {
        for _ in 0..60 {
            let (x, y, z) = (next(48) - 16, next(30) - 64, next(48) - 16);
            let block = palette[next(palette.len() as u64) as usize];
            local.set_block_state(x, y, z, block).unwrap();
            full.set_block_state(x, y, z, block).unwrap();
        }
        local.flush_live_relights().unwrap();
        full.relight_from_scratch();
        assert_same_loaded_light(&local, &full);
        for x in -16..32 {
            for z in -16..32 {
                for y in -64..-34 {
                    assert_eq!(
                        (
                            local.block_light_level(x, y, z),
                            local.sky_light_level(x, y, z)
                        ),
                        (
                            full.block_light_level(x, y, z),
                            full.sky_light_level(x, y, z)
                        ),
                        "round {round} at {x},{y},{z}"
                    );
                }
            }
        }
    }
}
