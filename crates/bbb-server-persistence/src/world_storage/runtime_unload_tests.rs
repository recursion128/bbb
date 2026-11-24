use super::entity_publication_fixtures::{
    area_effect_cloud_entity_entries, loaded_entity_chunk, test_flat_column,
};
use super::world_save_tests::{save_fixture_world, save_patch, save_patch_request};
use super::*;
use crate::{Java26PoiRecordEdit, Java26PoiSectionEdit};
use std::fs;

#[test]
fn runtime_unload_save_writes_selected_regions_without_whole_files() {
    let root = std::env::temp_dir().join(format!("bbb-runtime-unload-save-{}", std::process::id()));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let initial = storage.load_candidate().unwrap();
    let player_uuid = Uuid::from_u128(0x756e_6c6f_6164_5f73_6176_655f_706c_6179);
    storage
        .save_snapshot(&initial, &save_patch_request(vec![save_patch(player_uuid)]))
        .unwrap();
    let baseline = storage.load_candidate().unwrap();
    let level_before = fs::read(paths.level_dat()).unwrap();
    let player_path = paths
        .player_data_dir()
        .join(format!("{}.dat", player_uuid.hyphenated()));
    let player_before = fs::read(&player_path).unwrap();
    let clocks_before = fs::read(paths.world_clocks_dat()).unwrap();
    let dimension = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    let mut request = save_patch_request(vec![save_patch(player_uuid)]);
    request.level_state.game_time += 1;
    request.player_patches[0].position[0] += 100.0;
    request.world_clocks = Java26WorldClocks::new(Vec::new());
    let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
    let mut edited_column = test_flat_column(false, &fixture);
    edited_column.set_inhabited_time(42);
    let gold_block = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:gold_block")
            .unwrap()
            .id,
    )
    .unwrap();
    edited_column
        .set_block_state(1, -64, 2, gold_block)
        .unwrap();
    request.chunk_writes.push(Java26CreatedChunk {
        dimension: dimension.clone(),
        chunk_x: 0,
        chunk_z: 0,
        document: crate::chunk_writer::full_chunk_document(
            &edited_column,
            0,
            0,
            &|_| Some("minecraft:plains".to_string()),
            -60,
        )
        .unwrap(),
    });
    let entity_uuid = Uuid::from_u128(0x756e_6c6f_6164_5f73_6176_655f_656e_7469);
    request.entity_writes.push(Java26EntityChunkWrite {
        dimension: dimension.clone(),
        chunk_x: 0,
        chunk_z: 0,
        document: loaded_entity_chunk(area_effect_cloud_entity_entries(entity_uuid))
            .data
            .into_raw(),
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
    storage.save_runtime_unloads(&baseline, &request).unwrap();
    let column = storage
        .read_runtime_column(&baseline, &dimension, 0, 0)
        .unwrap();
    assert_eq!(
        column.chunk.unwrap().data.sections()[0]
            .block_states
            .as_ref()
            .unwrap()
            .value_at(1, 0, 2)
            .unwrap()
            .name,
        "minecraft:gold_block"
    );
    assert_eq!(
        column
            .entities
            .unwrap()
            .data
            .entities()
            .next()
            .unwrap()
            .uuid(),
        entity_uuid
    );
    assert_eq!(
        column.poi.unwrap().data.records().next().unwrap().pos(),
        [1, -64, 2]
    );
    assert_eq!(fs::read(paths.level_dat()).unwrap(), level_before);
    assert_eq!(fs::read(player_path).unwrap(), player_before);
    assert_eq!(fs::read(paths.world_clocks_dat()).unwrap(), clocks_before);
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}
