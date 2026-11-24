use super::*;

use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::VANILLA_BLOCK_BARRIER_ID;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_server_datapack::dynamic_registry::RawRegistryElement;
use bbb_server_persistence::{Java26LevelData, Java26LevelStem};

fn debug_stem(dimension_type: &str) -> Java26LevelStem {
    Java26LevelStem {
        dimension_type: dimension_type.to_string(),
        generator: "minecraft:debug".to_string(),
        raw_generator: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "type".into(),
            value: NbtSummaryValue::string("minecraft:debug"),
        }]),
    }
}

fn set_initialized(candidate: &mut Java26WorldCandidate, initialized: bool) {
    let mut raw = candidate.level.raw().clone();
    let NbtSummaryValue::Compound(data) = &mut raw.entries_mut()[0].value else {
        unreachable!("fixture Data is a compound")
    };
    data.iter_mut()
        .find(|entry| entry.name == "initialized")
        .expect("fixture has initialized")
        .value = NbtSummaryValue::Byte(i8::from(initialized));
    candidate.level = Java26LevelData::parse(raw).unwrap();
}

pub(super) fn set_hardcore(candidate: &mut Java26WorldCandidate, hardcore: bool) {
    let mut raw = candidate.level.raw().clone();
    let NbtSummaryValue::Compound(data) = &mut raw.entries_mut()[0].value else {
        unreachable!("fixture Data is a compound")
    };
    let NbtSummaryValue::Compound(settings) = &mut data
        .iter_mut()
        .find(|entry| entry.name == "difficulty_settings")
        .expect("fixture has difficulty settings")
        .value
    else {
        unreachable!("difficulty settings is a compound")
    };
    settings
        .iter_mut()
        .find(|entry| entry.name == "hardcore")
        .expect("fixture has hardcore")
        .value = NbtSummaryValue::Byte(i8::from(hardcore));
    candidate.level = Java26LevelData::parse(raw).unwrap();
}

fn set_overworld_debug(candidate: &mut Java26WorldCandidate) {
    candidate.world_gen_settings.dimensions.insert(
        Java26BuiltinDimension::Overworld.into(),
        debug_stem("minecraft:overworld"),
    );
}

fn serve_builtin_flat_levels(candidate: &mut Java26WorldCandidate) {
    let overworld = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    let template = candidate.world_gen_settings.dimensions[&overworld].clone();
    for (dimension, dimension_type) in [
        (Java26BuiltinDimension::Nether, "minecraft:the_nether"),
        (Java26BuiltinDimension::End, "minecraft:the_end"),
    ] {
        candidate.world_gen_settings.dimensions.insert(
            dimension.into(),
            Java26LevelStem {
                dimension_type: dimension_type.to_string(),
                ..template.clone()
            },
        );
    }
}

fn startup_with_custom_debug(
    level_id: &str,
    dimension_type_id: &str,
    min_y: i32,
    height: i32,
) -> Arc<bbb_server_datapack::startup::DatapackStartup> {
    let mut startup = crate::chunk::noise_generator::test_support::fresh_overworld_startup(true);
    let dimension_types = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:dimension_type")
        .unwrap();
    let mut dimension_type = serde_json::from_slice::<serde_json::Value>(
        &(dimension_types
            .elements
            .iter()
            .find(|element| element.id == "minecraft:overworld")
            .unwrap())
        .source_json,
    )
    .unwrap();
    dimension_type["min_y"] = serde_json::json!(min_y);
    dimension_type["height"] = serde_json::json!(height);
    dimension_type["logical_height"] = serde_json::json!(height);
    dimension_types.elements.push(RawRegistryElement {
        id: dimension_type_id.to_string(),
        source_pack: "file/debug_dimension".to_string(),
        source_json: serde_json::to_vec(&dimension_type).unwrap().into(),
        source: Some(
            bbb_registry_codec::decode_direct_registry_json(
                &serde_json::to_vec(&(dimension_type)).unwrap(),
            )
            .unwrap(),
        ),
        codec_value: None,
    });

    let dimensions = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:dimension")
        .unwrap();
    let stem = serde_json::json!({
        "type": dimension_type_id,
        "generator": { "type": "minecraft:debug" }
    });
    dimensions.elements.push(RawRegistryElement {
        id: level_id.to_string(),
        source_pack: "file/debug_dimension".to_string(),
        source_json: serde_json::to_vec(&stem).unwrap().into(),
        source: Some(
            bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(stem)).unwrap())
                .unwrap(),
        ),
        codec_value: None,
    });
    Arc::new(startup)
}

#[test]
fn custom_debug_is_served_at_its_published_height_without_becoming_global() {
    let custom = Java26LevelKey::parse("example:debug").unwrap();
    let custom_type = "example:short_debug";
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate.raids.insert(custom.clone(), Default::default());
    candidate.world_borders.insert(
        custom.clone(),
        bbb_server_persistence::Java26WorldBorderSettings::default(),
    );
    let startup = startup_with_custom_debug(custom.as_str(), custom_type, -32, 128);
    let mut world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(startup),
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();

    assert!(!world.is_debug());
    assert!(world.level_ids().contains(&world.require_level_id(&custom)));
    assert!(!world
        .unserved_levels()
        .iter()
        .any(|(level, _)| *level == custom.as_str()));
    assert_eq!(
        world.debug_level_shape_for_test(&world.require_level_id(&custom)),
        (-32, 8)
    );
    assert_eq!(
        world.debug_level_spawn_height_for_test(&world.require_level_id(&custom)),
        64
    );
    let pos = BlockPos { x: 1, y: 80, z: 1 };
    assert!(world.set_block(
        world.require_level_id(&custom),
        pos,
        FlatFixture::new().unwrap().stone_state_id(),
        0,
    ));
}

#[test]
fn fresh_overworld_debug_initializes_global_view_and_refuses_mutation() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    serve_builtin_flat_levels(&mut candidate);
    set_overworld_debug(&mut candidate);
    set_initialized(&mut candidate, false);
    let mut world = ServerWorld::restore(
        ServerWorldConfig {
            difficulty: Difficulty::Normal,
            game_type: GameType::Survival,
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();

    assert!(world.is_debug());
    let (spawn, rotation, difficulty, locked, advance_weather, game_type, clock) =
        world.debug_initialization_facts_for_test();
    assert_eq!(
        spawn,
        Vec3d {
            x: 0.5,
            y: 80.0,
            z: 0.5
        }
    );
    assert_eq!(
        rotation,
        PlayerRotation {
            y_rot: 0.0,
            x_rot: 0.0
        }
    );
    assert_eq!(difficulty, Difficulty::Normal);
    assert!(locked);
    assert_eq!(advance_weather, Some(GameRuleValue::Boolean(false)));
    assert_eq!(game_type, GameType::Spectator);
    assert_eq!(clock, Some(6_000));
    assert!(
        world
            .common_player_spawn_info(Java26BuiltinDimension::Overworld, GameType::Spectator)
            .is_debug
    );

    let barrier = vanilla_block_default_state_id(VANILLA_BLOCK_BARRIER_ID).unwrap();
    for dimension in [
        Java26BuiltinDimension::Overworld,
        Java26BuiltinDimension::Nether,
        Java26BuiltinDimension::End,
    ] {
        assert_eq!(
            world.block_state(dimension, BlockPos { x: 1, y: 60, z: 1 }),
            Some(barrier)
        );
        assert_eq!(
            world.block_state(dimension, BlockPos { x: 3, y: 70, z: 1 }),
            Some(173)
        );
    }

    let overworld = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    let pos = BlockPos { x: 2, y: 80, z: 2 };
    let generated = world
        .debug_generated_block_state_for_test(
            &world.require_level_id(&overworld),
            pos.x,
            pos.y,
            pos.z,
        )
        .unwrap();
    assert!(!world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        FlatFixture::new().unwrap().stone_state_id(),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        world.debug_generated_block_state_for_test(
            &world.require_level_id(&overworld),
            pos.x,
            pos.y,
            pos.z
        ),
        Some(generated)
    );
    assert_eq!(world.debug_suppression_facts_for_test(), (0, 0, 0, Some(0)));
}

#[test]
fn debug_join_and_acknowledged_batch_send_display_chunks_without_stopping() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    serve_builtin_flat_levels(&mut candidate);
    set_overworld_debug(&mut candidate);
    set_initialized(&mut candidate, false);
    let mut world = spawn_pinned_restore(ServerWorldConfig::default(), &candidate);
    let (sender, receiver) = mailbox();
    let (joined, handshake) =
        join_loaded_ticks(&sender, &receiver, &mut world, 1, "debug_viewer", 1);
    let next_batch = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ChunkBatchReceived {
            desired_chunks_per_tick: 9.0,
        },
    );
    for tick in [&joined, &handshake, &next_batch] {
        assert!(
            !tick
                .events
                .iter()
                .any(|event| matches!(event, WorldEvent::StopRequested | WorldEvent::Close { .. })),
            "{tick:?}"
        );
        assert_eq!(tick.snapshot.players.len(), 1);
    }
    let mut delivered = std::collections::BTreeSet::new();
    for tick in [&joined, &next_batch] {
        let packets = tick
            .events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet,
                } if matches!(
                    packet,
                    ClientboundPlayPacket::ChunkBatchStart
                        | ClientboundPlayPacket::LevelChunkWithLight(_)
                        | ClientboundPlayPacket::ChunkBatchFinished { .. }
                ) =>
                {
                    Some(packet)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(packets.len(), 11);
        assert!(matches!(packets[0], ClientboundPlayPacket::ChunkBatchStart));
        assert!(matches!(
            packets[10],
            ClientboundPlayPacket::ChunkBatchFinished { batch_size: 9 }
        ));
        for packet in &packets[1..10] {
            let ClientboundPlayPacket::LevelChunkWithLight(chunk) = packet else {
                panic!("a complete batch contains nine chunk packets");
            };
            assert!(delivered.insert((chunk.x, chunk.z)));
            let column = world
                .chunk_store_for_test(Java26BuiltinDimension::Overworld)
                .column(chunk.x, chunk.z)
                .expect("sent debug column remains loaded");
            assert_eq!(chunk.chunk_data.section_data, column.encode_section_data());
            if (chunk.x, chunk.z) == (0, 0) {
                assert_eq!(
                    column.get_block_state(1, 60, 1),
                    vanilla_block_default_state_id(VANILLA_BLOCK_BARRIER_ID)
                );
                assert_eq!(column.get_block_state(3, 70, 1), Some(173));
            }
        }
    }
    assert!(delivered.contains(&(0, 0)));
    assert_eq!(delivered.len(), 18);
}

#[test]
fn initialized_debug_restore_keeps_saved_initialization_state() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    set_overworld_debug(&mut candidate);
    candidate.game_rules.insert(
        "minecraft:advance_weather".to_string(),
        GameRuleValue::Boolean(true),
    );
    let world = ServerWorld::restore(
        ServerWorldConfig {
            difficulty: Difficulty::Normal,
            game_type: GameType::Survival,
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();
    let (spawn, _, difficulty, locked, advance_weather, game_type, clock) =
        world.debug_initialization_facts_for_test();
    assert_eq!(
        spawn,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5
        }
    );
    assert_eq!(difficulty, Difficulty::Normal);
    assert!(!locked);
    assert_eq!(advance_weather, Some(GameRuleValue::Boolean(true)));
    assert_eq!(game_type, GameType::Survival);
    assert_eq!(clock, Some(0));
}

#[test]
fn fresh_hardcore_debug_forces_hard_after_initial_peaceful_setup() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    set_overworld_debug(&mut candidate);
    set_initialized(&mut candidate, false);
    set_hardcore(&mut candidate, true);
    let world = ServerWorld::restore(
        ServerWorldConfig {
            difficulty: Difficulty::Peaceful,
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();
    assert_eq!(
        world.debug_initialization_facts_for_test().2,
        Difficulty::Hard
    );
}

#[test]
fn global_debug_tick_keeps_scheduled_random_inhabited_and_broadcast_state_untouched() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    set_overworld_debug(&mut candidate);
    set_initialized(&mut candidate, false);
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    world.prime_debug_suppression_for_test();
    let before = world.debug_suppression_facts_for_test();

    world.tick(Vec::new());
    world.tick(Vec::new());

    assert_eq!(world.debug_suppression_facts_for_test(), before);
    assert_eq!(before, (1, 1, 1, Some(0)));
}
