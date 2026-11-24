use super::*;
use bbb_server_persistence::{
    generation_state_chunk_document, Java26CreatedChunk, Java26EntityChunkData,
};
use bbb_server_types::chunk_data::{
    ChunkColumn, ChunkFeaturesState, ChunkGenerationState, FeatureHeightmaps,
};
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;

fn original_mobs_config() -> ServerWorldConfig {
    let mut config = ServerWorldConfig {
        datapacks: Some(crate::chunk::noise_generator::overworld_startup(true)),
        ..ServerWorldConfig::default()
    };
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let biomes = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/biome")
        .unwrap();
    for biome in &mut biomes.elements {
        let mut json: serde_json::Value = serde_json::from_slice(&biome.source_json).unwrap();
        json["creature_spawn_probability"] = serde_json::json!(0.75);
        biome.source_json = serde_json::to_vec(&json).unwrap().into();
        biome.source =
            Some(bbb_registry_codec::decode_direct_registry_json(&biome.source_json).unwrap());
        let mut json: serde_json::Value = serde_json::from_slice(&biome.source_json).unwrap();
        json["spawners"]["creature"] = serde_json::json!([
            {"type": "minecraft:pig", "weight": 1, "minCount": 2, "maxCount": 2}
        ]);
        biome.source_json = serde_json::to_vec(&json).unwrap().into();
        biome.source =
            Some(bbb_registry_codec::decode_direct_registry_json(&biome.source_json).unwrap());
        biome.source_json = serde_json::to_vec(&biome.json()).unwrap().into();
        biome.codec_value = None;
    }
    config
}

fn original_mobs_proto_chunks() -> Vec<Java26CreatedChunk> {
    let mut column = ChunkColumn::new(-64, 24, block_state_id("minecraft:air"), 0);
    for x in 0..16 {
        for z in 0..16 {
            column
                .set_block_state(
                    x,
                    63,
                    z,
                    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
                        "minecraft:grass_block",
                        &BTreeMap::from([("snowy".into(), "false".into())]),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
    }
    let state = ChunkGenerationState::Features(ChunkFeaturesState {
        postprocessing_positions: Vec::new(),
        carving_mask: Vec::new(),
        final_heightmaps: FeatureHeightmaps::prime(&column),
    });
    // A FEATURES halo supplies the ticket's light dependencies without unrelated terrain work.
    (-4..=4)
        .flat_map(|x| (-4..=4).map(move |z| (x, z)))
        .map(|(x, z)| Java26CreatedChunk {
            dimension: Java26BuiltinDimension::Overworld.into(),
            chunk_x: x,
            chunk_z: z,
            document: generation_state_chunk_document(
                &column,
                x,
                z,
                &|_| Some("minecraft:plains".into()),
                &state,
                Some(
                    &NoiseStructureChunkData::from_complete_reference_window(x, z, [], |_, _| {
                        Vec::new()
                    })
                    .to_nbt(),
                ),
                &[],
            )
            .unwrap(),
        })
        .collect()
}

fn original_mobs_records(world: &mut ServerWorld) -> BTreeMap<Uuid, Vec<NbtSummaryEntry>> {
    let request = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    let mut result = BTreeMap::new();
    for write in request.entity_writes {
        if write.dimension != Java26LevelKey::from(Java26BuiltinDimension::Overworld) {
            continue;
        }
        let document =
            Java26EntityChunkData::parse(write.document, write.chunk_x, write.chunk_z).unwrap();
        for entity in document.entities() {
            assert_eq!(entity.id(), "minecraft:pig");
            assert!(result
                .insert(entity.uuid(), entity.raw().to_vec())
                .is_none());
        }
    }
    result
}

fn original_mobs_age_and_variant(
    records: &BTreeMap<Uuid, Vec<NbtSummaryEntry>>,
) -> BTreeMap<Uuid, (NbtSummaryValue, NbtSummaryValue)> {
    records
        .iter()
        .map(|(uuid, fields)| {
            let field = |name| {
                fields
                    .iter()
                    .find(|entry| entry.name == name)
                    .unwrap_or_else(|| panic!("generated pig lacks {name}"))
                    .value
                    .clone()
            };
            (*uuid, (field("Age"), field("variant")))
        })
        .collect()
}

#[test]
fn original_mobs_ticket_full_publishes_once_and_disk_restore_preserves_generated_pigs() {
    let config = original_mobs_config();
    let root =
        std::env::temp_dir().join(format!("bbb-original-mobs-restart-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let candidate = registry_biome_source_candidate(
        fixed_biome_source(NbtSummaryValue::String("minecraft:plains".into())),
        false,
    );
    storage
        .create_world(
            &Java26CreatedSave {
                level: crate::world_creation::created_level_document(&Java26WorldCreation {
                    level_name: "original mobs restart".into(),
                    game_type: ServerWorld::fixture_game_type_id(),
                    difficulty: Difficulty::Easy.serialized_name().into(),
                    spawn_position: [0, 64, 0],
                    last_played: 1_786_000_000_000,
                })
                .unwrap(),
                world_gen_settings: candidate.world_gen_settings.document.clone(),
                chunks: original_mobs_proto_chunks(),
            },
            RegionCompression::Zlib,
            100,
        )
        .unwrap();
    let baseline = crate::world_creation::load_candidate_from_bootstrap(
        &storage,
        storage.load_bootstrap().unwrap(),
        &config.datapacks.as_ref().unwrap().registries,
    )
    .unwrap();
    let mut disabled = baseline.clone();
    disabled.game_rules.insert(
        "minecraft:spawn_mobs".into(),
        bbb_protocol::game_rules::GameRuleValue::Boolean(false),
    );
    disabled.chunk_tickets.insert(
        Java26BuiltinDimension::Overworld.into(),
        vec![bbb_server_persistence::Java26SavedChunkTicket {
            chunk_x: 0,
            chunk_z: 0,
            level: 31,
            ticks_left: None,
        }],
    );
    let disabled_world = ServerWorld::restore(config.clone(), &disabled).unwrap();
    assert!(matches!(
        disabled_world
            .chunk_store_for_test(Java26BuiltinDimension::Overworld)
            .generation_state(0, 0),
        Some(ChunkGenerationState::NoiseFull(_))
    ));
    assert!(disabled_world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .is_empty());
    assert!(disabled_world
        .chunk_store_for_test(Java26BuiltinDimension::Overworld)
        .published_worldgen_entity_records(0, 0)
        .is_empty());
    drop(disabled_world);
    let mut world = crate::tests::restore_stored(config.clone(), &baseline);
    assert!(world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .is_empty());
    assert!(matches!(
        world
            .chunk_store_for_test(Java26BuiltinDimension::Overworld)
            .generation_state(0, 0),
        Some(ChunkGenerationState::Features(_))
    ));
    world
        .place_portal_ticket(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 0, y: 64, z: 0 },
        )
        .unwrap();
    let chunks = world.chunk_store_for_test(Java26BuiltinDimension::Overworld);
    assert!(matches!(
        chunks.generation_state(0, 0),
        Some(ChunkGenerationState::NoiseFull(_))
    ));
    assert!(chunks.published_worldgen_entity_records(0, 0).is_empty());
    let before = original_mobs_records(&mut world);
    assert!(
        !before.is_empty(),
        "FEATURES to FULL must actually generate creatures"
    );
    let expected = original_mobs_age_and_variant(&before);
    world
        .place_portal_ticket(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 0, y: 64, z: 0 },
        )
        .unwrap();
    assert_eq!(original_mobs_records(&mut world), before);
    let request = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let saved = crate::world_creation::load_candidate_from_bootstrap(
        &storage,
        storage.load_bootstrap().unwrap(),
        &config.datapacks.as_ref().unwrap().registries,
    )
    .unwrap();
    let mut restored = ServerWorld::restore(config, &saved).unwrap();
    assert_eq!(
        original_mobs_age_and_variant(&original_mobs_records(&mut restored)),
        expected
    );
    restored
        .place_portal_ticket(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 0, y: 64, z: 0 },
        )
        .unwrap();
    assert_eq!(
        original_mobs_age_and_variant(&original_mobs_records(&mut restored)),
        expected
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
