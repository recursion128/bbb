use super::*;
use crate::world_creation::default_overworld_level_source;
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_server_types::chunk_data::ChunkColumn;

fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
    vanilla_block_state_id_by_name_and_properties(
        name,
        &properties
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
    .unwrap()
}
fn bell() -> i32 {
    state(
        "minecraft:bell",
        &[
            ("attachment", "floor"),
            ("facing", "north"),
            ("powered", "false"),
        ],
    )
}
fn tags() -> RegistryTagSnapshot {
    RegistryTagSnapshot::new([(
        "minecraft:point_of_interest_type".to_string(),
        "minecraft:village".to_string(),
        vec![
            "minecraft:meeting".to_string(),
            "minecraft:home".to_string(),
        ],
    )])
}
fn saved(valid: bool, kind: &str, tickets: i32) -> Java26PoiChunkData {
    let empty = Java26PoiChunkData::empty(0, 0, -4, 24).unwrap();
    let raw = empty
        .raw_with_section_edits(
            &[Java26PoiSectionEdit {
                y: 0,
                valid,
                records: vec![Java26PoiRecordEdit {
                    pos: [1, 4, 1],
                    type_id: kind.to_string(),
                    free_tickets: tickets,
                }],
            }],
            -4,
            24,
        )
        .unwrap();
    Java26PoiChunkData::parse(raw, 0, 0, -4, 24).unwrap()
}

#[test]
fn poi_state_mapping_includes_bed_heads_and_all_bootstrap_blocks() {
    let head = state(
        "minecraft:red_bed",
        &[("facing", "north"), ("occupied", "false"), ("part", "head")],
    );
    let foot = state(
        "minecraft:red_bed",
        &[("facing", "north"), ("occupied", "false"), ("part", "foot")],
    );
    assert_eq!(
        poi_type_for_state(head).unwrap().resource_id,
        "minecraft:home"
    );
    assert_eq!(poi_type_for_state(foot), None);
    assert_eq!(poi_type_for_state(bell()).unwrap().max_tickets, 32);
    let mut found = BTreeSet::new();
    for state in 0..VANILLA_BLOCK_STATE_COUNT {
        if let Some(kind) = poi_type_for_state(state) {
            found.insert(kind.resource_id);
        }
    }
    assert_eq!(found.len(), VANILLA_POI_TYPES.len());
    assert_eq!(poi_type_for_state(-1), None);
}

#[test]
fn poi_queries_load_saved_records_without_terrain_and_village_uses_occupied_loaded_tags() {
    let mut store = ChunkStore::new(default_overworld_level_source().unwrap(), 0);
    store.restore_poi([saved(true, "minecraft:meeting", 31)]);
    let mut saved_records = Vec::new();
    store
        .poi()
        .visit_records_in_range([1, 4, 1], 0, PoiOccupancy::IsOccupied, |record| {
            saved_records.push((record.pos, record.type_id, record.free_tickets))
        });
    assert_eq!(saved_records, [([1, 4, 1], "minecraft:meeting", 31)]);
    assert_eq!(store.poi().sections_to_village([0, 0, 0], &tags()), 7);
    assert_eq!(
        store
            .poi_mut()
            .records_in_range([1, 4, 1], 0, PoiOccupancy::IsOccupied, |_| true)
            .len(),
        1
    );
    assert_eq!(store.column_count(), 0);
    assert_eq!(store.poi().sections_to_village([6, 6, 6], &tags()), 6);
    assert_eq!(store.poi().sections_to_village([7, 0, 0], &tags()), 7);
    assert_eq!(store.poi_mut().release_ticket([1, 4, 1]), Some(true));
    let mut occupied = 0;
    store
        .poi()
        .visit_records_in_range([1, 4, 1], 0, PoiOccupancy::IsOccupied, |_| occupied += 1);
    assert_eq!(occupied, 0);
    let mut released_records = Vec::new();
    store
        .poi()
        .visit_records_in_range([1, 4, 1], 0, PoiOccupancy::Any, |record| {
            released_records.push((record.pos, record.type_id, record.free_tickets));
        });
    assert_eq!(released_records, [([1, 4, 1], "minecraft:meeting", 32)]);
    assert_eq!(store.poi().sections_to_village([0, 0, 0], &tags()), 7);
}

#[test]
fn poi_invalid_section_refresh_keeps_original_record_tickets_and_type_by_position() {
    let mut poi = PoiManager::from_saved([saved(false, "minecraft:home", -1)]);
    let mut column = ChunkColumn::new(-64, 24, 0, 0);
    column.set_block_state(1, 4, 1, bell());
    column.set_block_state(2, 4, 1, bell());
    poi.check_column((0, 0), &column, &[(0, true)]);
    assert_eq!(poi.record([1, 4, 1]).unwrap().type_id, "minecraft:home");
    assert_eq!(poi.record([1, 4, 1]).unwrap().free_tickets, -1);
    assert_eq!(poi.record([2, 4, 1]).unwrap().free_tickets, 32);
    assert!(!poi.acquire_ticket([1, 4, 1]));
    assert_eq!(poi.release_ticket([1, 4, 1]), Some(true));
    let first = poi.save_edits(&Java26LevelKey::parse("minecraft:overworld").unwrap());
    assert_eq!(first.len(), 1);
    assert!(poi.acquire_ticket([2, 4, 1]));
    assert!(!poi.acknowledge_save((0, 0), first[0].epoch));
    let second = poi.save_edits(&Java26LevelKey::parse("minecraft:overworld").unwrap());
    assert!(poi.acknowledge_save((0, 0), second[0].epoch));
    assert!(poi
        .save_edits(&Java26LevelKey::parse("minecraft:overworld").unwrap())
        .is_empty());
}

#[test]
fn poi_find_type_uses_chunk_z_x_then_section_y_not_nearest() {
    let mut poi = PoiManager::default();
    for pos in [[0, 0, 16], [16, 16, 0], [16, 0, 0], [0, 0, 0]] {
        poi.block_changed(pos, 0, bell());
    }
    let ordered = poi.records_in_range([16, 0, 16], 48, PoiOccupancy::Any, |_| true);
    assert_eq!(
        ordered.iter().map(|r| r.pos).collect::<Vec<_>>(),
        [[0, 0, 0], [16, 0, 0], [16, 16, 0], [0, 0, 16]]
    );
    assert_eq!(
        poi.find_type_in_range("minecraft:meeting", [16, 0, 16], 48, PoiOccupancy::Any)
            .unwrap()
            .pos,
        [0, 0, 0]
    );
    assert_eq!(
        poi.find_closest_with_type([16, 0, 16], 48, PoiOccupancy::Any, |record| {
            record.type_id == "minecraft:meeting"
        })
        .unwrap()
        .pos,
        [16, 0, 0]
    );
}

#[test]
fn poi_find_type_skips_removed_records_and_tracks_occupancy_in_negative_section() {
    let mut poi = PoiManager::default();
    for y in 0..8 {
        for x in 0..8 {
            poi.block_changed([x - 16, y - 16, -1], 0, bell());
        }
    }
    let removed = [[-13, -13, -1], [-16, -16, -1]];
    for pos in removed {
        poi.block_changed(pos, bell(), 0);
    }
    let found = poi
        .find_type_in_range("minecraft:meeting", [-8, -8, -1], 32, PoiOccupancy::Any)
        .unwrap()
        .pos;
    assert!(!removed.contains(&found));
    assert!(poi.record(found).is_some());
    assert!(poi
        .find_type_in_range(
            "minecraft:meeting",
            [-8, -8, -1],
            32,
            PoiOccupancy::IsOccupied
        )
        .is_none());
    assert!(poi.acquire_ticket(found));
    assert_eq!(
        poi.find_type_in_range(
            "minecraft:meeting",
            [-8, -8, -1],
            32,
            PoiOccupancy::IsOccupied
        )
        .unwrap()
        .pos,
        found
    );
}

#[test]
fn poi_find_type_loads_stored_records_without_dirtying_or_acquiring() {
    let raw = Java26PoiChunkData::empty(0, 0, -4, 24)
        .unwrap()
        .raw_with_section_edits(
            &[Java26PoiSectionEdit {
                y: 0,
                valid: true,
                records: [[1, 1, 0], [0, 0, 0], [2, 0, 0]]
                    .into_iter()
                    .map(|pos| Java26PoiRecordEdit {
                        pos,
                        type_id: "minecraft:meeting".into(),
                        free_tickets: 32,
                    })
                    .collect(),
            }],
            -4,
            24,
        )
        .unwrap();
    let mut poi = PoiManager::from_saved([Java26PoiChunkData::parse(raw, 0, 0, -4, 24).unwrap()]);
    assert!(poi.loaded.is_empty());
    let found = poi
        .find_type_in_range("minecraft:meeting", [1, 0, 0], 8, PoiOccupancy::HasSpace)
        .unwrap()
        .pos;
    assert!([[1, 1, 0], [0, 0, 0], [2, 0, 0]].contains(&found));
    assert_eq!(poi.loaded, BTreeSet::from([(0, 0)]));
    assert!(poi.dirty.is_empty());
    assert_eq!(poi.record([0, 0, 0]).unwrap().free_tickets, 32);
    assert!(poi
        .find_type_in_range("minecraft:meeting", [0, 0, 0], -1, PoiOccupancy::Any)
        .is_none());
}

#[test]
fn poi_save_reload_round_trips_records() {
    let mut poi = PoiManager::default();
    for pos in [[1, 1, 0], [0, 0, 0]] {
        poi.block_changed(pos, 0, bell());
    }
    let edits = poi.save_edits(&Java26LevelKey::parse("minecraft:overworld").unwrap());
    let mut saved: Vec<_> = edits[0].sections[0].records.iter().map(|r| r.pos).collect();
    saved.sort_unstable();
    assert_eq!(saved, [[0, 0, 0], [1, 1, 0]]);
    let raw = Java26PoiChunkData::empty(0, 0, -4, 24)
        .unwrap()
        .raw_with_section_edits(&edits[0].sections, -4, 24)
        .unwrap();
    let mut reloaded =
        PoiManager::from_saved([Java26PoiChunkData::parse(raw, 0, 0, -4, 24).unwrap()]);
    for pos in [[1, 1, 0], [0, 0, 0]] {
        assert_eq!(reloaded.record(pos).unwrap().type_id, "minecraft:meeting");
    }
}
