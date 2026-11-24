use super::*;
use crate::world_storage::entity_publication_fixtures::{
    area_effect_cloud_entity_entries, loaded_entity_chunk, test_created_save, test_flat_column,
};
use crate::world_storage::{
    Java26EntityChunkWrite, Java26PoiChunkSaveEdits, Java26SaveRequest, SaveHolderResolver,
};
use crate::{Java26CreatedChunk, Java26LevelSaveState, Java26PoiRecordEdit, Java26PoiSectionEdit};
use bbb_protocol::NbtSummaryValue;
use bbb_server_types::save::{Java26BuiltinDimension, WorldSaveLimits, WorldSavePaths};
use std::{collections::BTreeMap, fs};
use uuid::Uuid;

fn edited_created_chunk(
    dimension: Java26LevelKey,
    chunk_x: i32,
    chunk_z: i32,
    inhabited_time: i64,
    block_name: &str,
) -> Java26CreatedChunk {
    let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
    let mut column = test_flat_column(false, &fixture);
    column.set_inhabited_time(inhabited_time);
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(block_name).unwrap();
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
    column.set_block_state(1, -64, 2, state).unwrap();
    Java26CreatedChunk {
        dimension,
        chunk_x,
        chunk_z,
        document: crate::chunk_writer::full_chunk_document(
            &column,
            chunk_x,
            chunk_z,
            &|_| Some("minecraft:plains".to_string()),
            -60,
        )
        .unwrap(),
    }
}

fn created_storage(name: &str, short: bool) -> Java26WorldStorage {
    let root =
        std::env::temp_dir().join(format!("bbb-runtime-column-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
    let column = test_flat_column(short, &fixture);
    let mut save = test_created_save(
        &crate::Java26WorldCreation {
            level_name: name.to_string(),
            game_type: 0,
            difficulty: "normal".to_string(),
            spawn_position: [0, 64, 0],
            last_played: 1,
        },
        &fixture,
        &[(0, 0, &column)],
    )
    .unwrap();
    if short {
        let mut entries = save.world_gen_settings.entries_mut();
        for key in ["data", "dimensions", "minecraft:overworld"] {
            let NbtSummaryValue::Compound(value) = &mut entries
                .iter_mut()
                .find(|entry| entry.name == key)
                .unwrap()
                .value
            else {
                unreachable!()
            };
            entries = value;
        }
        entries
            .iter_mut()
            .find(|entry| entry.name == "type")
            .unwrap()
            .value = NbtSummaryValue::String("minecraft:the_nether".into());
    }
    storage
        .create_world(&save, crate::RegionCompression::Zlib, 1)
        .unwrap();
    storage
}

fn save_request(candidate: &Java26WorldCandidate) -> Java26SaveRequest {
    Java26SaveRequest {
        data_packs: None,
        level_state: Java26LevelSaveState {
            game_time: candidate.level.game_time(),
            last_played: candidate.level.last_played(),
            game_type: candidate.level.game_type(),
            initialized: candidate.level.initialized(),
            difficulty: bbb_protocol::difficulties::Difficulty::Normal,
            difficulty_locked: false,
            spawn: candidate.level.spawn().clone(),
        },
        region_timestamp: 200,
        region_compression: crate::RegionCompression::Zlib,
        player_patches: Vec::new(),
        departed_player_stats: BTreeMap::new(),
        player_advancements: BTreeMap::new(),
        poi_edits: Vec::new(),
        chunk_writes: Vec::new(),
        chunk_write_epochs: BTreeMap::new(),
        entity_writes: Vec::new(),
        game_rules: candidate.game_rules.clone(),
        world_clocks: candidate.world_clocks.clone(),
        map_writes: BTreeMap::new(),
        map_last_id: None,
        world_borders: candidate.world_borders.clone(),
        raids: candidate.raids.clone(),
        ender_dragon_fights: BTreeMap::new(),
        chunk_tickets: candidate.chunk_tickets.clone(),
        random_sequences: candidate.random_sequences.clone(),
        scheduled_events: candidate.scheduled_events.clone(),
        stopwatches: candidate.stopwatches.clone(),
        weather: candidate.weather,
        wandering_trader: candidate.wandering_trader,
        scoreboard: candidate.scoreboard.clone(),
        custom_boss_events: candidate.custom_boss_events.clone(),
        command_storage_writes: BTreeMap::new(),
        item_stack_resolver: SaveHolderResolver::unavailable(),
    }
}

#[test]
fn runtime_column_reads_latest_save_with_original_candidate() {
    let storage = created_storage("latest", false);
    let root = storage.paths.root().to_path_buf();
    let candidate = storage.load_candidate().unwrap();
    let dimension = Java26BuiltinDimension::Overworld.into();
    let before = storage
        .read_runtime_column(&candidate, &dimension, 0, 0)
        .unwrap();
    assert_eq!(before.chunk.as_ref().unwrap().timestamp, 1);
    assert!(before.entities.is_none() && before.poi.is_none());
    let mut request = save_request(&candidate);
    request.chunk_writes.push(edited_created_chunk(
        dimension.clone(),
        0,
        0,
        42,
        "minecraft:gold_block",
    ));
    let uuid = Uuid::from_u128(0x7265_6164_5f6c_6174_6573_745f_636f_6c75);
    let entity = loaded_entity_chunk(area_effect_cloud_entity_entries(uuid));
    request.entity_writes.push(Java26EntityChunkWrite {
        dimension: dimension.clone(),
        chunk_x: 0,
        chunk_z: 0,
        document: entity.data.into_raw(),
    });
    request.poi_edits.push(Java26PoiChunkSaveEdits {
        dimension: dimension.clone(),
        chunk_x: 0,
        chunk_z: 0,
        epoch: 1,
        sections: vec![Java26PoiSectionEdit {
            y: -4,
            valid: true,
            records: vec![Java26PoiRecordEdit {
                pos: [1, -64, 2],
                type_id: "minecraft:home".into(),
                free_tickets: 1,
            }],
        }],
    });
    storage.save_snapshot(&candidate, &request).unwrap();
    let latest = storage
        .read_runtime_column(&candidate, &dimension, 0, 0)
        .unwrap();
    let chunk = latest.chunk.unwrap();
    assert_eq!(chunk.timestamp, 200);
    assert_eq!(
        chunk.data.sections()[0]
            .block_states
            .as_ref()
            .unwrap()
            .value_at(1, 0, 2)
            .unwrap()
            .name,
        "minecraft:gold_block"
    );
    assert_ne!(chunk.data, before.chunk.unwrap().data);
    let entities = latest.entities.unwrap();
    assert_eq!(entities.timestamp, 200);
    assert_eq!(entities.data.entities().next().unwrap().uuid(), uuid);
    let poi = latest.poi.unwrap();
    assert_eq!(poi.timestamp, 200);
    assert_eq!(poi.data.records().next().unwrap().pos(), [1, -64, 2]);
    assert!(candidate.dimensions[&dimension]
        .stored_entity_chunks
        .is_empty());
    fs::write(
        storage
            .paths
            .dimension_dir(&dimension)
            .join("region/r.0.0.mca"),
        b"broken",
    )
    .unwrap();
    assert_eq!(
        storage
            .read_runtime_entity_chunk(&candidate, &dimension, 0, 0)
            .unwrap(),
        Some(entities)
    );
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_column_first_written_this_session_takes_later_edits() {
    let storage = created_storage("session-written", false);
    let root = storage.paths.root().to_path_buf();
    let candidate = storage.load_candidate().unwrap();
    let dimension: Java26LevelKey = Java26BuiltinDimension::Overworld.into();
    let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
    let column = test_flat_column(false, &fixture);
    let created = test_created_save(
        &crate::Java26WorldCreation {
            level_name: "session-written".to_string(),
            game_type: 0,
            difficulty: "normal".to_string(),
            spawn_position: [0, 64, 0],
            last_played: 1,
        },
        &fixture,
        &[(1, 0, &column)],
    )
    .unwrap();
    let mut request = save_request(&candidate);
    request.chunk_writes = created.chunks;
    storage.save_snapshot(&candidate, &request).unwrap();

    let mut request = save_request(&candidate);
    request.chunk_writes.push(edited_created_chunk(
        dimension.clone(),
        1,
        0,
        0,
        "minecraft:gold_block",
    ));
    storage.save_snapshot(&candidate, &request).unwrap();
    let chunk = storage
        .read_runtime_column(&candidate, &dimension, 1, 0)
        .unwrap()
        .chunk
        .unwrap();
    assert_eq!(
        chunk.data.sections()[0]
            .block_states
            .as_ref()
            .unwrap()
            .value_at(1, 0, 2)
            .unwrap()
            .name,
        "minecraft:gold_block"
    );
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn runtime_column_reuses_declared_stem_geometry() {
    let storage = created_storage("short", true);
    let root = storage.paths.root().to_path_buf();
    let candidate = storage.load_candidate().unwrap();
    let dimension = Java26BuiltinDimension::Overworld.into();
    let geometry = candidate
        .dimension_geometries
        .get(&dimension)
        .unwrap()
        .sections();
    assert_eq!(geometry.min_section_y, 0);
    assert_eq!(geometry.section_count, 16);
    let column = storage
        .read_runtime_column(&candidate, &dimension, 0, 0)
        .unwrap();
    assert_eq!(
        column.chunk.unwrap(),
        candidate
            .column_reader
            .as_ref()
            .unwrap()
            .read_column(&dimension, 0, 0)
            .unwrap()
            .chunk
            .unwrap()
    );
    let missing = storage
        .read_runtime_column(&candidate, &dimension, -1, -1)
        .unwrap();
    assert!(missing.chunk.is_none() && missing.entities.is_none() && missing.poi.is_none());
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

fn home_poi_edit(dimension: &Java26LevelKey, pos: [i32; 3]) -> Java26PoiChunkSaveEdits {
    Java26PoiChunkSaveEdits {
        dimension: dimension.clone(),
        chunk_x: 0,
        chunk_z: 0,
        epoch: 1,
        sections: vec![Java26PoiSectionEdit {
            y: -4,
            valid: true,
            records: vec![Java26PoiRecordEdit {
                pos,
                type_id: "minecraft:home".into(),
                free_tickets: 1,
            }],
        }],
    }
}

/// A read and a save of the same region files never overlap: the save
/// rewrites headers in place, which a read halfway through would misreport.
#[test]
fn record_reads_and_saves_take_turns_on_the_region_files() {
    use std::time::Duration;
    let storage = created_storage("region-io", false);
    let root = storage.paths.root().to_path_buf();
    let candidate = storage.load_candidate().unwrap();
    let dimension: Java26LevelKey = Java26BuiltinDimension::Overworld.into();
    let reader = storage.runtime_reader(&candidate).unwrap();
    std::thread::scope(|scope| {
        let writing = storage.region_io.write().unwrap();
        let read = scope.spawn(|| reader.read_column(&dimension, 0, 0));
        std::thread::sleep(Duration::from_millis(200));
        assert!(!read.is_finished());
        drop(writing);
        assert!(read.join().unwrap().unwrap().chunk.is_some());

        let reading = storage.region_io.read().unwrap();
        let save = scope.spawn(|| storage.save_snapshot(&candidate, &save_request(&candidate)));
        std::thread::sleep(Duration::from_millis(200));
        assert!(!save.is_finished());
        drop(reading);
        save.join().unwrap().unwrap();
    });
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

/// `SectionStorage.java:189-208`: a POI record that does not read leaves the
/// column without POI, its terrain intact, and the next POI save replaces it.
#[test]
fn an_unreadable_poi_record_keeps_the_terrain_and_is_replaced_on_save() {
    let storage = created_storage("broken-poi", false);
    let root = storage.paths.root().to_path_buf();
    let candidate = storage.load_candidate().unwrap();
    let dimension: Java26LevelKey = Java26BuiltinDimension::Overworld.into();
    let mut request = save_request(&candidate);
    request
        .poi_edits
        .push(home_poi_edit(&dimension, [1, -64, 2]));
    storage.save_snapshot(&candidate, &request).unwrap();
    let poi_file = storage
        .paths
        .dimension_dir(&dimension)
        .join("poi/r.0.0.mca");
    let mut bytes = fs::read(&poi_file).unwrap();
    let sector = u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]) as usize;
    bytes[sector * 4096 + 5..sector * 4096 + 69].fill(0xA5);
    fs::write(&poi_file, bytes).unwrap();

    let column = storage
        .read_runtime_column(&candidate, &dimension, 0, 0)
        .unwrap();
    assert!(column.chunk.is_some());
    assert!(column.poi.is_none());
    assert!(column.poi_error.is_some());

    let mut request = save_request(&candidate);
    request
        .poi_edits
        .push(home_poi_edit(&dimension, [3, -64, 4]));
    storage.save_snapshot(&candidate, &request).unwrap();
    let poi = storage
        .runtime_reader(&candidate)
        .unwrap()
        .read_poi(&dimension, 0, 0)
        .unwrap()
        .unwrap();
    assert_eq!(
        poi.data
            .records()
            .map(|record| record.pos())
            .collect::<Vec<_>>(),
        vec![[3, -64, 4]]
    );
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn corrupt_poi_region_header_fails_column_read_and_save_preflight_before_region_mutation() {
    let storage = created_storage("corrupt-poi-header", false);
    let root = storage.paths.root().to_path_buf();
    let candidate = storage.load_candidate().unwrap();
    let dimension: Java26LevelKey = Java26BuiltinDimension::Overworld.into();
    let dim_dir = storage.paths.dimension_dir(&dimension);
    let poi_dir = dim_dir.join("poi");
    fs::create_dir_all(&poi_dir).unwrap();
    fs::write(poi_dir.join("r.0.0.mca"), b"broken").unwrap();

    assert!(matches!(
        storage.read_runtime_column(&candidate, &dimension, 0, 0),
        Err(SaveError::InvalidRegion { .. })
    ));

    let region_path = dim_dir.join("region/r.0.0.mca");
    let region_before = fs::read(&region_path).unwrap();
    let mut request = save_request(&candidate);
    request.chunk_writes.push(edited_created_chunk(
        dimension.clone(),
        0,
        0,
        99,
        "minecraft:gold_block",
    ));
    request
        .poi_edits
        .push(home_poi_edit(&dimension, [1, -64, 2]));

    assert!(matches!(
        storage.save_snapshot(&candidate, &request),
        Err(SaveError::InvalidRegion { .. })
    ));
    assert_eq!(fs::read(&region_path).unwrap(), region_before);

    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}
