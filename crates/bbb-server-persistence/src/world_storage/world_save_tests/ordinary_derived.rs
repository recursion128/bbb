use super::*;

#[test]
fn world_save_overwrites_existing_full_chunk_with_updated_heightmaps_and_light() {
    let root =
        std::env::temp_dir().join(format!("bbb-ordinary-derived-save-{}", std::process::id()));
    save_fixture_world(&root);
    let mut column = bbb_server_types::chunk_data::ChunkColumn::new(-64, 24, 0, 0);
    let biome = |_: i32| Some("minecraft:plains".to_owned());
    let mut original = crate::chunk_writer::full_chunk_document(&column, 0, 0, &biome, 0).unwrap();
    original
        .entries_mut()
        .retain(|entry| entry.name != "isLightOn");
    original
        .entries_mut()
        .push(nbt_entry("isLightOn", NbtSummaryValue::Byte(1)));
    overwrite_fixture_chunk(&root, &original);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap();
    column.set_block_state(1, 80, 2, stone).unwrap();
    let mut request = save_patch_request(Vec::new());
    request.chunk_writes = vec![Java26CreatedChunk {
        dimension: Java26BuiltinDimension::Overworld.into(),
        chunk_x: 0,
        chunk_z: 0,
        document: crate::chunk_writer::full_chunk_document(&column, 0, 0, &biome, 80).unwrap(),
    }];
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let saved = storage.load_candidate().unwrap();
    let data = &stored_chunk(
        &saved,
        &level_key(Java26BuiltinDimension::Overworld),
        (0, 0),
    )
    .data;
    assert!(!data.is_light_on());
    let expected = crate::Java26ChunkData::parse(
        crate::chunk_writer::full_chunk_document(&column, 0, 0, &biome, 80).unwrap(),
        crate::Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .unwrap();
    assert_eq!(data.heightmaps(), expected.heightmaps());
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn world_save_preserves_legacy_blending_upgrade_and_retrogen_on_whole_chunk_writes() {
    let root = std::env::temp_dir().join(format!(
        "bbb-ordinary-derived-proto-save-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let mut column = bbb_server_types::chunk_data::ChunkColumn::new(-64, 24, 0, 0);
    let biome = |_: i32| Some("minecraft:plains".to_owned());
    let mut original = crate::chunk_writer::full_chunk_document(&column, 0, 0, &biome, 0).unwrap();
    original
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "Status")
        .unwrap()
        .value = NbtSummaryValue::string("minecraft:carvers");
    original
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "Heightmaps")
        .unwrap()
        .value = NbtSummaryValue::Compound(Vec::new());
    original
        .entries_mut()
        .push(nbt_entry("entities", NbtSummaryValue::List(Vec::new())));
    original.entries_mut().push(nbt_entry(
        "blending_data",
        NbtSummaryValue::Compound(vec![
            nbt_entry("min_section", NbtSummaryValue::Int(-4)),
            nbt_entry("max_section", NbtSummaryValue::Int(20)),
        ]),
    ));
    original.entries_mut().push(nbt_entry(
        "UpgradeData",
        NbtSummaryValue::Compound(vec![nbt_entry("Sides", NbtSummaryValue::Byte(3))]),
    ));
    original.entries_mut().push(nbt_entry(
        "below_zero_retrogen",
        NbtSummaryValue::Compound(vec![
            nbt_entry("target_status", NbtSummaryValue::string("minecraft:spawn")),
            nbt_entry(
                "missing_bedrock",
                NbtSummaryValue::LongArray(vec![1, 0, 0, 0]),
            ),
        ]),
    ));
    overwrite_fixture_chunk(&root, &original);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let baseline = &stored_chunk(
        &candidate,
        &level_key(Java26BuiltinDimension::Overworld),
        (0, 0),
    )
    .data;
    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap();
    column.set_inhabited_time(7);
    column.set_block_state(1, 80, 2, stone).unwrap();
    let mut document = crate::chunk_writer::generation_state_chunk_document(
        &column,
        0,
        0,
        &biome,
        &bbb_server_types::chunk_data::ChunkGenerationState::Carvers {
            postprocessing_positions: Vec::new(),
            carving_mask: Vec::new(),
        },
        None,
        &[],
    )
    .unwrap();
    crate::write_legacy_chunk_fields(
        &mut document,
        crate::Java26ChunkStatus::Carvers,
        crate::Java26LegacyChunkFields {
            blending_data: baseline.blending_data(),
            upgrade_data: baseline.upgrade_data(),
            below_zero_retrogen: baseline.below_zero_retrogen(),
        },
    );
    let mut request = save_patch_request(Vec::new());
    request.chunk_writes = vec![Java26CreatedChunk {
        dimension: Java26BuiltinDimension::Overworld.into(),
        chunk_x: 0,
        chunk_z: 0,
        document,
    }];
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let saved = storage.load_candidate().unwrap();
    let data = &stored_chunk(
        &saved,
        &level_key(Java26BuiltinDimension::Overworld),
        (0, 0),
    )
    .data;
    assert_eq!(data.status(), crate::Java26ChunkStatus::Carvers);
    assert_eq!(data.heightmaps(), baseline.heightmaps());
    assert_eq!(data.is_light_on(), baseline.is_light_on());
    assert_eq!(data.inhabited_time(), 7);
    assert_eq!(data.blending_data(), baseline.blending_data());
    assert_eq!(data.upgrade_data(), baseline.upgrade_data());
    assert_eq!(data.below_zero_retrogen(), baseline.below_zero_retrogen());
    assert_eq!(
        data.sections()
            .iter()
            .find(|section| section.y == 5)
            .and_then(|section| section.block_states.as_ref())
            .and_then(|states| states.value_at(1, 0, 2))
            .map(|state| state.name.clone()),
        Some("minecraft:stone".to_owned())
    );
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}
