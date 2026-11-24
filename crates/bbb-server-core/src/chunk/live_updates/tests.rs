use std::collections::BTreeMap;

use super::*;
use crate::chunk::noise_generator::overworld_program;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
};
use bbb_protocol::block_types::{VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_STONE_ID};
use bbb_server_persistence::{
    generation_state_chunk_document_with_light, Java26ChunkContext, Java26ChunkData,
};

const CENTER: (i32, i32) = (0, 0);
const EAST: (i32, i32) = (1, 0);
const FAR: (i32, i32) = (4, 0);

fn store() -> ChunkStore {
    ChunkStore::new_overworld_carvers_with_structure_generation(overworld_program(true, 0), false)
        .unwrap()
}

fn full_store() -> ChunkStore {
    let mut store = store();
    store
        .try_ensure_served_full_columns(&[CENTER, EAST, FAR])
        .unwrap();
    store
}

fn air() -> i32 {
    vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).unwrap()
}

fn stone() -> i32 {
    vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap()
}

fn glowstone() -> i32 {
    vanilla_block_state_id_by_name_and_properties("minecraft:glowstone", &BTreeMap::new()).unwrap()
}

fn full_document(store: &ChunkStore, center: (i32, i32)) -> bbb_server_types::save::NamedCompound {
    let program = overworld_program(true, 0);
    generation_state_chunk_document_with_light(
        store.column(center.0, center.1).unwrap(),
        center.0,
        center.1,
        &|id| program.biome_name(id),
        store.generation_state(center.0, center.1).unwrap(),
        store
            .structure_component(center.0, center.1)
            .map(|s| s.to_nbt())
            .as_ref(),
        store.proto_entity_records(center.0, center.1),
        &store.saved_light_sections(center),
    )
    .unwrap()
}

fn restored_center(generated: &ChunkStore) -> ChunkStore {
    let parsed = Java26ChunkData::parse(
        full_document(generated, CENTER),
        Java26ChunkContext {
            expected_x: CENTER.0,
            expected_z: CENTER.1,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .unwrap();
    let mut restored = store();
    restored.insert_column(
        CENTER.0,
        CENTER.1,
        generated.column(CENTER.0, CENTER.1).unwrap().clone(),
    );
    restored.restore_saved_generation_metadata(&parsed).unwrap();
    restored
}

fn intent(store: &ChunkStore, chunk: (i32, i32)) -> Option<super::super::WholeRewriteIntent> {
    store
        .pending_whole_rewrites()
        .find(|intent| intent.chunk == chunk)
}

#[test]
fn persisted_and_restored_durable_writes_enter_whole_rewrite_intent() {
    let generated = full_store();
    let mut persisted = full_store();
    persisted.mark_persisted(CENTER.0, CENTER.1);
    let mut restored = restored_center(&generated);

    for store in [&mut persisted, &mut restored] {
        assert!(store.is_storage_backed_noise_full(CENTER.0, CENTER.1));
        assert_eq!(
            store.set_live_block_state(0, 319, 0, stone()).unwrap(),
            Some(air())
        );
        assert!(intent(store, CENTER).is_some());
        assert!(!store.is_storage_backed_noise_full(CENTER.0, CENTER.1));
    }
}

#[test]
fn relight_marks_only_durable_columns_with_changed_payloads() {
    let mut store = full_store();
    for chunk in [CENTER, EAST, FAR] {
        store.mark_persisted(chunk.0, chunk.1);
    }
    assert!(store.take_live_light_updates().unwrap().is_empty());
    let mut restored = restored_center(&store);
    assert!(restored.take_live_light_updates().unwrap().is_empty());
    // Keep the light section allocated when the emitter is removed.
    store.set_live_block_state(14, 200, 8, stone()).unwrap();
    store.flush_live_relights().unwrap();
    store.take_live_light_updates().unwrap();
    let east_before = store.saved_light_sections(EAST);
    let far_before = store.saved_light_sections(FAR);

    assert_eq!(
        store.set_live_block_state(15, 200, 8, glowstone()).unwrap(),
        Some(air())
    );
    store.flush_live_relights().unwrap();

    assert_ne!(store.saved_light_sections(EAST), east_before);
    assert_eq!(store.saved_light_sections(FAR), far_before);
    assert!(intent(&store, CENTER).is_some());
    assert!(intent(&store, EAST).is_some());
    assert!(intent(&store, FAR).is_none());
    assert!(store.is_storage_backed_noise_full(FAR.0, FAR.1));
    let first_dirty = store.live_light_updates.clone();
    assert!(!first_dirty[&CENTER].block.is_empty());
    assert!(!first_dirty[&EAST].block.is_empty());
    assert!(!first_dirty.contains_key(&FAR));

    store.set_live_block_state(15, 200, 8, air()).unwrap();
    store.flush_live_relights().unwrap();
    for (chunk, first) in first_dirty {
        assert!(first
            .block
            .is_subset(&store.live_light_updates[&chunk].block));
        assert!(first.sky.is_subset(&store.live_light_updates[&chunk].sky));
    }
    let latest = store.column_packet_light_data(CENTER).unwrap();
    let dirty = store.live_light_updates[&CENTER].clone();
    let updates = store.take_live_light_updates().unwrap();
    let update = updates
        .iter()
        .find(|update| (update.chunk_x, update.chunk_z) == CENTER)
        .unwrap();
    let actual = packet_sections(
        &update.light_data.block_y_mask,
        &update.light_data.empty_block_y_mask,
        &update.light_data.block_updates,
    );
    let latest = packet_sections(
        &latest.block_y_mask,
        &latest.empty_block_y_mask,
        &latest.block_updates,
    );
    assert!(!actual.is_empty());
    for (section, data) in actual {
        assert!(dirty.block.contains(&section));
        assert_eq!(Some(&data), latest.get(&section));
        assert!(data.iter().all(|&byte| byte == 0));
    }
    assert!(store.take_live_light_updates().unwrap().is_empty());

    store.set_live_block_state(8, 319, 8, stone()).unwrap();
    store.flush_live_relights().unwrap();
    assert!(!store.live_light_updates[&CENTER].sky.is_empty());
    store.take_live_light_updates().unwrap();

    let glass =
        vanilla_block_state_id_by_name_and_properties("minecraft:glass", &BTreeMap::new()).unwrap();
    store.set_live_block_state(8, 319, 8, glass).unwrap();
    let sky_only = store.take_live_light_updates().unwrap();
    assert!(sky_only
        .iter()
        .any(|update| !update.light_data.sky_updates.is_empty()));
    assert!(sky_only
        .iter()
        .all(|update| update.light_data.block_y_mask.is_empty()
            && update.light_data.empty_block_y_mask.is_empty()));

    store.has_skylight = false;
    store.set_live_block_state(15, 200, 8, glowstone()).unwrap();
    let updates = store.take_live_light_updates().unwrap();
    assert!(updates
        .iter()
        .any(|update| !update.light_data.block_updates.is_empty()));
    for update in updates {
        assert!(update.light_data.sky_y_mask.is_empty());
        assert!(update.light_data.empty_sky_y_mask.is_empty());
        assert!(update.light_data.sky_updates.is_empty());
    }
}

#[test]
fn live_relight_supports_registered_custom_mod_blocks_and_emission() {
    use std::sync::Arc;

    use bbb_protocol::content_registry::{ContentRegistryBuilder, ModBlockRegistration};

    let mut builder = ContentRegistryBuilder::new();
    builder.register_block(ModBlockRegistration {
        resource_id: "modded:glow_crystal".to_string(),
        state_count: 1,
        light_emission: 14,
        destroy_speed: 1.0,
        full_cube_collision: true,
        dynamic_light_rgb: None,
    });
    builder.register_block(ModBlockRegistration {
        resource_id: "modded:dark_pillar".to_string(),
        state_count: 2,
        light_emission: 0,
        destroy_speed: 2.0,
        full_cube_collision: true,
        dynamic_light_rgb: None,
    });
    builder.register_block(ModBlockRegistration {
        resource_id: "modded:clear_crystal".to_string(),
        state_count: 1,
        light_emission: 0,
        destroy_speed: 0.3,
        full_cube_collision: false,
        dynamic_light_rgb: None,
    });
    let registry = Arc::new(builder.freeze().unwrap());
    let glow_crystal = registry
        .block_by_resource_id("modded:glow_crystal")
        .unwrap()
        .first_state_id;
    let dark_pillar = registry
        .block_by_resource_id("modded:dark_pillar")
        .unwrap()
        .first_state_id
        + 1;
    let clear_crystal = registry
        .block_by_resource_id("modded:clear_crystal")
        .unwrap()
        .first_state_id;

    let mut store = full_store();
    store.install_content_registry(Arc::clone(&registry));
    store.take_live_light_updates().unwrap();

    // Placing a custom light-emitting cube propagates block light across the chunk seam.
    assert_eq!(
        store
            .set_live_block_state(15, 200, 8, glow_crystal)
            .unwrap(),
        Some(air())
    );
    store.flush_live_relights().unwrap();
    assert_eq!(store.block_light_level(15, 200, 8), 14);
    assert_eq!(store.block_light_level(16, 200, 8), 13);
    assert_eq!(store.block_light_level(14, 200, 8), 13);
    assert_eq!(store.sky_light_level(15, 200, 8), 0);
    let updates = store.take_live_light_updates().unwrap();
    assert!(updates
        .iter()
        .any(|u| (u.chunk_x, u.chunk_z) == CENTER && !u.light_data.block_updates.is_empty()));
    assert!(updates
        .iter()
        .any(|u| (u.chunk_x, u.chunk_z) == EAST && !u.light_data.block_updates.is_empty()));

    // A full-cube non-emitting custom block occludes both block light and sky light.
    store.set_live_block_state(14, 200, 8, dark_pillar).unwrap();
    store
        .set_live_block_state(8, 319, 8, clear_crystal)
        .unwrap();
    store.flush_live_relights().unwrap();
    assert_eq!(store.block_light_level(14, 200, 8), 0);
    assert_eq!(store.block_light_level(13, 200, 8), 10);
    assert_eq!(store.sky_light_level(14, 200, 8), 0);
    assert_eq!(store.sky_light_level(8, 319, 8), 15);
    assert_eq!(store.sky_light_level(8, 318, 8), 15);

    // Removing the custom emitter clears the propagated block light back to zero.
    store.set_live_block_state(15, 200, 8, air()).unwrap();
    store.flush_live_relights().unwrap();
    assert_eq!(store.block_light_level(15, 200, 8), 0);
    assert_eq!(store.block_light_level(16, 200, 8), 0);

    // Unregistered state IDs still fail closed during relighting.
    store
        .set_live_block_state(6, 200, 6, registry.total_block_state_count())
        .unwrap();
    assert!(store.flush_live_relights().is_err());
}

#[test]
fn live_updates_filter_layers_independently_and_keep_section_order() {
    let mut data = LightUpdateData {
        sky_y_mask: vec![0b1010, 1],
        block_y_mask: vec![0b101],
        empty_sky_y_mask: vec![0b100],
        empty_block_y_mask: vec![0b1000],
        sky_updates: vec![vec![1; 2048], vec![3; 2048], vec![64; 2048]],
        block_updates: vec![vec![0; 2048], vec![2; 2048]],
    };
    filter_light_data(
        &mut data,
        &DirtyLightSections {
            sky: rustc_hash::FxHashSet::from_iter([2, 3, 64]),
            block: rustc_hash::FxHashSet::from_iter([0, 3, 8]),
        },
    );
    assert_eq!(data.sky_y_mask, vec![0b1000, 1]);
    assert_eq!(data.sky_updates, vec![vec![3; 2048], vec![64; 2048]]);
    assert_eq!(data.empty_sky_y_mask, vec![0b100]);
    assert_eq!(data.block_y_mask, vec![1]);
    assert_eq!(data.block_updates, vec![vec![0; 2048]]);
    assert_eq!(data.empty_block_y_mask, vec![0b1000]);
    // An absent latest layer remains absent even if its section was dirtied.
    assert!(!packet_bit(&data.block_y_mask, 8));
    assert!(!packet_bit(&data.empty_block_y_mask, 8));
}

fn packet_sections<'a>(
    mask: &[i64],
    empty_mask: &[i64],
    updates: &'a [Vec<u8>],
) -> BTreeMap<usize, &'a [u8]> {
    let mut arrays = updates.iter();
    let mut sections = BTreeMap::new();
    for index in 0..mask.len().max(empty_mask.len()) * 64 {
        if packet_bit(mask, index) {
            sections.insert(index, arrays.next().unwrap().as_slice());
        } else if packet_bit(empty_mask, index) {
            sections.insert(index, &[][..]);
        }
    }
    sections
}
