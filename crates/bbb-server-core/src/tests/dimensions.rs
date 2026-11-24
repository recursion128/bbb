//! Several levels at once: the stems the server declares, the directories they
//! save into, and the teleports that move a session between them.

use super::*;
use bbb_server_types::save::Java26LevelKey;

/// `MinecraftServer.createLevels` (`MinecraftServer.java:461-473`) builds one
/// `ServerLevel` per stem of the baked `LEVEL_STEM` registry, and each one is
/// sized by the `dimension_type` its own stem names -- not by the dimension it
/// is filed under and not by a constant.
///
#[test]
fn every_declared_stem_becomes_a_level_sized_by_its_own_dimension_type() {
    let world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    assert_eq!(
        world.served_builtin_dimensions(),
        vec![
            Java26BuiltinDimension::Overworld,
            Java26BuiltinDimension::Nether,
            Java26BuiltinDimension::End,
        ]
    );
    // `data/minecraft/dimension_type/overworld.json` is `min_y = -64`,
    // `height = 384`; `the_nether.json` and `the_end.json` are both `min_y = 0`,
    // `height = 256`.
    assert_eq!(
        world.level_column_shape(Java26BuiltinDimension::Overworld),
        (-64, 24)
    );
    assert_eq!(
        world.level_column_shape(Java26BuiltinDimension::Nether),
        (0, 16)
    );
    assert_eq!(
        world.level_column_shape(Java26BuiltinDimension::End),
        (0, 16)
    );
}

/// Each level generates from its own stem, so the nether's columns are the
/// created save's nether flat generator rather than a copy of the overworld's:
/// the same fixture floor y, over a floor 64 blocks higher up.
#[test]
fn each_level_generates_the_column_its_own_stem_declares() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let fixture = FlatFixture::new().unwrap();
    let stone = fixture.stone_state_id();
    let air = fixture.air_state_id();
    for dimension in world.served_builtin_dimensions() {
        let (min_y, sections) = world.level_column_shape(dimension);
        let top = min_y + sections as i32 * 16;
        assert_eq!(
            world.block_state(
                dimension,
                BlockPos {
                    x: 3,
                    y: min_y,
                    z: 5
                }
            ),
            Some(stone),
            "{dimension:?} floor"
        );
        assert_eq!(
            world.block_state(
                dimension,
                BlockPos {
                    x: 3,
                    y: fixture.floor_y(),
                    z: 5
                }
            ),
            Some(stone),
            "{dimension:?} top of the fixture column"
        );
        assert_eq!(
            world.block_state(
                dimension,
                BlockPos {
                    x: 3,
                    y: fixture.floor_y() + 1,
                    z: 5
                }
            ),
            Some(air),
            "{dimension:?} above the fixture column"
        );
        // The build height is the dimension type's, so a position one block
        // above the top of the column is outside the world.
        assert_eq!(
            world.block_state(dimension, BlockPos { x: 3, y: top, z: 5 }),
            None,
            "{dimension:?} above the build height"
        );
    }
}

/// `ClientboundLoginPacket` carries `this.server.levelKeys()`
/// (`PlayerList.java:170`), which is every level the server runs.
#[test]
fn the_login_lists_every_level_the_server_runs() {
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let tick = world.tick(receiver.drain());
    let login = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Login(login),
                ..
            } => Some(login),
            _ => None,
        })
        .expect("the join opens with a login");
    assert_eq!(
        login.levels,
        vec![
            "minecraft:overworld".to_string(),
            "minecraft:the_nether".to_string(),
            "minecraft:the_end".to_string(),
        ]
    );
    // A session starts in the level its dimension names, and the spawn info
    // says which that is.
    assert_eq!(login.common_spawn_info.dimension, "minecraft:overworld");
    // `createCommonSpawnInfo` writes the level's dimension *type* as an index
    // into the registry the configuration phase published
    // (`ServerPlayer.java:2151-2164`, `CommonPlayerSpawnInfo.java:43`), and the
    // built-in pack registers `minecraft:overworld` first.
    assert_eq!(
        login.common_spawn_info.dimension_type_id,
        world
            .synchronized_registries()
            .dimension_type_index("minecraft:overworld")
            .unwrap()
    );
    assert_eq!(login.common_spawn_info.dimension_type_id, 0);
}

/// `ServerLevel` reads and writes `getDimensionPath(dimension)`
/// (`LevelStorageSource.java:508-510`), so a block a level changed is saved
/// into that level's own directory and no other's.
#[test]
fn a_block_written_in_one_level_is_saved_into_that_levels_dimension_directory() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let air = FlatFixture::new().unwrap().air_state_id();
    assert!(world.set_block(
        Java26BuiltinDimension::Nether,
        BlockPos { x: 1, y: 40, z: 2 },
        air,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let request = world
        .save_request(0, 1, bbb_server_persistence::RegionCompression::Zlib)
        .unwrap();
    // Nothing has touched the overworld, so only the nether column is written.
    let written: Vec<Java26LevelKey> = request
        .chunk_writes
        .iter()
        .map(|chunk| chunk.dimension.clone())
        .collect();
    assert_eq!(
        written,
        vec![Java26LevelKey::from(Java26BuiltinDimension::Nether)]
    );
    assert_eq!(
        (
            request.chunk_writes[0].chunk_x,
            request.chunk_writes[0].chunk_z
        ),
        (0, 0)
    );
}

/// `PrepareSpawnTask.start` (`PrepareSpawnTask.java:52-58`) sends a session to
/// the level its playerdata `Dimension` names, and
/// `ServerPlayer.addAdditionalSaveData` (`ServerPlayer.java:414`) writes back
/// the level it is in.
#[test]
fn a_saved_player_dimension_places_the_session_in_that_level_and_survives_the_save() {
    let uuid = Uuid::from_u128(42);
    let mut candidate = restore_candidate(0, None, Vec::new());
    let mut player = restore_fixture_player(uuid, [8.5, 65.0, 8.5], [0.0, 0.0], true);
    player = with_dimension(player, "minecraft:the_nether");
    candidate.players.insert(
        uuid,
        (bbb_server_persistence::PlayerDataSource::Primary, player),
    );
    let mut world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(crate::chunk::noise_generator::overworld_startup(true)),
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();

    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(3),
            profile: GameProfile {
                uuid,
                name: "alice".to_string(),
                properties: Vec::new(),
            },
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    assert_eq!(
        world.player_dimension(ConnectionId(3)),
        Java26BuiltinDimension::Nether
    );
    let login = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Login(login),
                ..
            } => Some(login),
            _ => None,
        })
        .expect("the join opens with a login");
    assert_eq!(login.common_spawn_info.dimension, "minecraft:the_nether");

    let request = world
        .save_request(0, 1, bbb_server_persistence::RegionCompression::Zlib)
        .unwrap();
    assert_eq!(request.player_patches.len(), 1);
    assert_eq!(
        request.player_patches[0].created.dimension,
        "minecraft:the_nether"
    );
}

/// A document naming a level this server does not run is the `Optional.map`
/// that answers empty at `PrepareSpawnTask.java:55`, which falls through to the
/// respawn dimension rather than refusing the login.
#[test]
fn a_saved_player_dimension_the_server_does_not_run_falls_back_to_the_respawn_level() {
    let uuid = Uuid::from_u128(43);
    let mut candidate = restore_candidate(0, None, Vec::new());
    let player = with_dimension(
        restore_fixture_player(uuid, [8.5, 65.0, 8.5], [0.0, 0.0], true),
        "minecraft:the_end",
    );
    candidate.players.insert(
        uuid,
        (bbb_server_persistence::PlayerDataSource::Primary, player),
    );
    // The built-in pack has no `minecraft:dimension` rows to merge back in,
    // so a save that declares no End stem has no End level.
    candidate
        .world_gen_settings
        .dimensions
        .remove(&Java26LevelKey::from(Java26BuiltinDimension::End));
    let mut world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(crate::chunk::noise_generator::overworld_startup(true)),
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();
    assert_eq!(
        world.served_builtin_dimensions(),
        vec![
            Java26BuiltinDimension::Overworld,
            Java26BuiltinDimension::Nether,
        ]
    );

    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(4),
            profile: GameProfile {
                uuid,
                name: "alice".to_string(),
                properties: Vec::new(),
            },
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert_eq!(
        world.player_dimension(ConnectionId(4)),
        Java26BuiltinDimension::Overworld
    );
}

fn startup_with_dimension(
    id: &str,
    value: serde_json::Value,
) -> Arc<bbb_server_datapack::startup::DatapackStartup> {
    let mut startup = crate::chunk::noise_generator::test_support::fresh_overworld_startup(true);
    let registry = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:dimension")
        .unwrap();
    registry.elements.retain(|element| element.id != id);
    registry
        .elements
        .push(bbb_server_datapack::dynamic_registry::RawRegistryElement {
            id: id.to_string(),
            source_pack: "file/test_dimensions".to_string(),
            source_json: serde_json::to_vec(&value).unwrap().into(),
            source: Some(
                bbb_registry_codec::decode_direct_registry_json(
                    &serde_json::to_vec(&(value)).unwrap(),
                )
                .unwrap(),
            ),
            codec_value: None,
        });
    Arc::new(startup)
}

fn vanilla_end_stem_json() -> serde_json::Value {
    let snapshot =
        bbb_server_datapack::vanilla_snapshot::VanillaDatapackSnapshot::embedded().unwrap();
    let preset: serde_json::Value = serde_json::from_slice(
        snapshot
            .read_path("worldgen/world_preset/normal.json")
            .unwrap(),
    )
    .unwrap();
    preset["dimensions"]["minecraft:the_end"].clone()
}

#[test]
fn an_explicit_datapack_end_stem_is_unioned_with_saved_dimensions() {
    let uuid = Uuid::from_u128(44);
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate
        .world_gen_settings
        .dimensions
        .remove(&Java26LevelKey::from(Java26BuiltinDimension::End));
    candidate.players.insert(
        uuid,
        (
            bbb_server_persistence::PlayerDataSource::Primary,
            with_dimension(
                restore_fixture_player(uuid, [8.5, 65.0, 8.5], [0.0, 0.0], true),
                "minecraft:the_end",
            ),
        ),
    );
    let startup = startup_with_dimension("minecraft:the_end", vanilla_end_stem_json());
    let mut world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(startup),
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();
    assert!(world
        .served_builtin_dimensions()
        .contains(&Java26BuiltinDimension::End));

    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(5),
            profile: GameProfile {
                uuid,
                name: "alice".to_string(),
                properties: Vec::new(),
            },
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert_eq!(
        world.player_dimension(ConnectionId(5)),
        Java26BuiltinDimension::End
    );
}

#[test]
fn an_explicit_datapack_stem_wins_over_the_saved_stem_at_the_same_key() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate
        .world_gen_settings
        .dimensions
        .get_mut(&Java26LevelKey::from(Java26BuiltinDimension::End))
        .map(|stem| {
            stem.generator = "minecraft:debug".to_string();
            stem.raw_generator = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "type".into(),
                value: NbtSummaryValue::string("minecraft:debug"),
            }]);
        })
        .unwrap();
    let saved_only = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    assert!(saved_only
        .served_builtin_dimensions()
        .contains(&Java26BuiltinDimension::End));
    assert_eq!(saved_only.level_sea_level(Java26BuiltinDimension::End), 63);
    let startup = startup_with_dimension("minecraft:the_end", vanilla_end_stem_json());
    let world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(startup),
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();
    assert!(world
        .served_builtin_dimensions()
        .contains(&Java26BuiltinDimension::End));
    assert_eq!(world.level_sea_level(Java26BuiltinDimension::End), 0);
    assert!(!world
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == "minecraft:the_end"));
}

#[test]
fn a_custom_datapack_noise_level_is_served() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    let custom = Java26LevelKey::parse("example:moon").expect("custom level key");
    candidate.raids.insert(custom.clone(), Default::default());
    candidate.world_borders.insert(
        custom.clone(),
        bbb_server_persistence::Java26WorldBorderSettings::default(),
    );
    let startup = startup_with_dimension("example:moon", vanilla_end_stem_json());
    let world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(startup),
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();
    assert!(world.level_id(&custom).is_some());
    assert!(world.unserved_levels().is_empty());
}

#[test]
fn a_custom_flat_level_is_served_and_publishes_its_world_border() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    let custom = Java26LevelKey::parse("example:moon").expect("custom level key");
    let overworld = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    let stem = candidate.world_gen_settings.dimensions[&overworld].clone();
    candidate
        .world_gen_settings
        .dimensions
        .insert(custom.clone(), stem);
    candidate.raids.insert(custom.clone(), Default::default());
    candidate.world_borders.insert(
        custom.clone(),
        bbb_server_persistence::Java26WorldBorderSettings::default(),
    );
    let uuid = Uuid::from_u128(44);
    candidate.players.insert(
        uuid,
        (
            bbb_server_persistence::PlayerDataSource::Primary,
            with_dimension(
                restore_fixture_player(uuid, [8.5, 65.0, 8.5], [0.0, 0.0], true),
                custom.as_str(),
            ),
        ),
    );

    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    assert!(!world
        .unserved_levels()
        .iter()
        .any(|(dimension, _)| *dimension == custom.as_str()));

    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(44),
            profile: profile("moonwalker", 44),
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    assert_eq!(
        world.player_level_id(ConnectionId(44)),
        world.require_level_id(&custom)
    );
    let login = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(44),
                packet: ClientboundPlayPacket::Login(login),
            } => Some(login),
            _ => None,
        })
        .expect("the custom-level session is published");
    assert_eq!(login.common_spawn_info.dimension, custom.as_str());
    assert!(login.levels.iter().any(|level| level == custom.as_str()));

    let teleport_id = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .unwrap();
    for packet in [
        PlayServerbound::AcceptTeleportation { id: teleport_id },
        PlayServerbound::PlayerLoaded,
    ] {
        enqueue(
            &sender,
            WorldCommand::Play {
                connection_id: ConnectionId(44),
                packet,
            },
        )
        .unwrap();
    }
    world.tick(receiver.drain());
    for x in -2..=2 {
        for z in -2..=2 {
            world.ensure_entity_transfer_full_chunk_for_test_by_key(
                &world.require_level_id(&custom),
                (x, z),
            );
        }
    }

    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(44),
            packet: PlayServerbound::UseItemOn(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: BlockHitResult {
                    pos: BlockPos { x: 8, y: 64, z: 8 },
                    direction: Direction::Up,
                    cursor_x: 0.5,
                    cursor_y: 0.5,
                    cursor_z: 0.5,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 1,
            }),
        },
    )
    .unwrap();
    let clicked = world.tick(receiver.drain());
    let acknowledges = |tick: &TickResult, sequence| {
        assert_eq!(
            tick.events
                .iter()
                .filter(|event| matches!(event,
                    WorldEvent::Send {
                        connection_id: ConnectionId(44),
                        packet: ClientboundPlayPacket::BlockChangedAck(ack),
                    } if ack.sequence == sequence))
                .count(),
            1,
            "{tick:?}"
        );
    };
    acknowledges(&clicked, 1);

    let custom_id = world.require_level_id(&custom);
    let air = FlatFixture::new().unwrap().air_state_id();
    let lever_pos = BlockPos { x: 9, y: 64, z: 8 };
    let lever = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:lever",
        &BTreeMap::from([
            ("face".into(), "floor".into()),
            ("facing".into(), "north".into()),
            ("powered".into(), "false".into()),
        ]),
    )
    .unwrap();
    assert!(world.set_block(custom_id, lever_pos, lever, SET_BLOCK_UPDATE_ALL));
    let toggled = right_click(&sender, &receiver, &mut world, 44, lever_pos, 2);
    acknowledges(&toggled, 2);
    let powered = world.block_state_by_key(&custom_id, lever_pos).unwrap();
    assert_eq!(
        crate::block_behavior::string_property(powered, "powered").as_deref(),
        Some("true")
    );
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, lever_pos),
        Some(air)
    );

    play(
        &sender,
        &receiver,
        &mut world,
        44,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 37,
            item: ItemStackSummary {
                item_id: Some(item_id("minecraft:stone")),
                count: 1,
                component_patch: Default::default(),
            },
        }),
    );
    let support = BlockPos { x: 10, y: 63, z: 8 };
    let placed = right_click(&sender, &receiver, &mut world, 44, support, 3);
    acknowledges(&placed, 3);
    let placed_pos = BlockPos { y: 64, ..support };
    assert_eq!(
        world.block_state_by_key(&custom_id, placed_pos),
        Some(block_state_id("minecraft:stone"))
    );
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, placed_pos),
        Some(air)
    );

    let log_pos = BlockPos {
        x: 10,
        y: 64,
        z: 10,
    };
    assert!(world.set_block(
        custom_id,
        log_pos,
        crate::block_behavior::default_block_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_OAK_LOG_ID
        ),
        SET_BLOCK_UPDATE_ALL,
    ));
    play(
        &sender,
        &receiver,
        &mut world,
        44,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 37,
            item: ItemStackSummary {
                item_id: Some(item_id("minecraft:iron_axe")),
                count: 1,
                component_patch: Default::default(),
            },
        }),
    );
    let stripped = right_click(&sender, &receiver, &mut world, 44, log_pos, 4);
    acknowledges(&stripped, 4);
    assert_eq!(
        world.block_state_by_key(&custom_id, log_pos),
        Some(crate::block_behavior::default_block_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_STRIPPED_OAK_LOG_ID
        ))
    );
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, log_pos),
        Some(air)
    );

    let chest_pos = BlockPos { x: 10, y: 64, z: 9 };
    assert!(world.set_block(
        custom_id,
        chest_pos,
        crate::block_behavior::default_block_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_CHEST_ID
        ),
        SET_BLOCK_UPDATE_ALL
    ));
    let opened = right_click(&sender, &receiver, &mut world, 44, chest_pos, 5);
    acknowledges(&opened, 5);
    let container_id = opened
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(44),
                packet: ClientboundPlayPacket::OpenScreen(screen),
            } => {
                assert_eq!(
                    screen.menu_type_id,
                    bbb_protocol::menu_types::VANILLA_MENU_TYPE_GENERIC_9X3_ID
                );
                Some(screen.container_id)
            }
            _ => None,
        })
        .expect("the custom-level chest opens");
    assert!(opened.events.iter().any(|event| matches!(event,
        WorldEvent::Send {
            connection_id: ConnectionId(44),
            packet: ClientboundPlayPacket::ContainerSetContent(content),
        } if content.container_id == container_id && content.items.len() == 63)));
    assert!(block_events_sent(&opened)
        .iter()
        .any(|event| event.pos == chest_pos && event.b1 == 1));
    let closed = play(
        &sender,
        &receiver,
        &mut world,
        44,
        PlayServerbound::ContainerClose(ContainerCloseRequest { container_id }),
    );
    assert!(block_events_sent(&closed)
        .iter()
        .any(|event| event.pos == chest_pos && event.b1 == 0));
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, chest_pos),
        Some(air)
    );

    let anchor_pos = BlockPos { x: 8, y: 71, z: 8 };
    let anchor = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:respawn_anchor",
        &BTreeMap::from([("charges".into(), "1".into())]),
    )
    .unwrap();
    let stone = block_state_id("minecraft:stone");
    assert!(world.set_block(custom_id, anchor_pos, anchor, SET_BLOCK_UPDATE_ALL));
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(10, "custom-level-survival", AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: "gamemode survival moonwalker".into(),
            },
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    let exploded = play(
        &sender,
        &receiver,
        &mut world,
        44,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: anchor_pos,
                direction: Direction::Down,
                cursor_x: 0.5,
                cursor_y: 0.0,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: 6,
        }),
    );
    acknowledges(&exploded, 6);
    assert!(set_healths(&exploded)
        .iter()
        .any(|(connection_id, health)| *connection_id == ConnectionId(44)
            && *health > 0.0
            && *health < 20.0));
    assert!(exploded.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(44),
            packet: ClientboundPlayPacket::Explode(_),
        }
    )));
    assert_eq!(world.block_state_by_key(&custom_id, anchor_pos), Some(air));
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, anchor_pos),
        Some(air)
    );
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(11, "custom-level-creative", AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: "gamemode creative moonwalker".into(),
            },
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    world.set_block(
        custom_id,
        BlockPos { x: 8, y: 63, z: 8 },
        stone,
        SET_BLOCK_UPDATE_ALL,
    );
    world.set_block(
        custom_id,
        BlockPos { x: 8, y: 64, z: 8 },
        air,
        SET_BLOCK_UPDATE_ALL,
    );

    let mined_pos = BlockPos { x: 11, y: 64, z: 8 };
    assert!(world.set_block(
        custom_id,
        mined_pos,
        block_state_id("minecraft:dirt"),
        SET_BLOCK_UPDATE_ALL
    ));
    let picked = play(
        &sender,
        &receiver,
        &mut world,
        44,
        PlayServerbound::PickItemFromBlock(PickItemFromBlock {
            pos: mined_pos,
            include_data: false,
        }),
    );
    let inventory = &picked.snapshot.players[0].inventory;
    assert_eq!(
        inventory.slots[inventory.selected_slot as usize].item_id,
        Some(item_id("minecraft:dirt"))
    );
    let mined = play(
        &sender,
        &receiver,
        &mut world,
        44,
        PlayServerbound::PlayerAction(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: mined_pos,
            direction: Direction::Up,
            sequence: 100,
        }),
    );
    acknowledges(&mined, 100);
    assert_eq!(world.block_state_by_key(&custom_id, mined_pos), Some(air));
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, mined_pos),
        Some(air)
    );

    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, "custom-level-setblock", AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: "execute in example:moon run setblock 9 65 9 minecraft:stone".into(),
            },
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    let command_pos = BlockPos { x: 9, y: 65, z: 9 };
    let stone = block_state_id("minecraft:stone");
    let air = FlatFixture::new().unwrap().air_state_id();
    assert_eq!(
        world.block_state_by_key(&world.require_level_id(&custom), command_pos),
        Some(stone)
    );
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, command_pos),
        Some(air)
    );

    let falling = BlockPos { x: 8, y: 70, z: 8 };
    let landing = BlockPos { x: 8, y: 64, z: 8 };
    let sand = block_state_id("minecraft:sand");
    assert!(world.set_block(
        world.require_level_id(&custom),
        falling,
        sand,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    for (dimension, chunk) in world.pending_runtime_entity_chunks() {
        world
            .complete_runtime_entity_chunk(&dimension, chunk, None)
            .unwrap();
    }
    let game_time = world.game_time();
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), game_time + 40);
    assert_eq!(
        world.block_state_by_key(&world.require_level_id(&custom), falling),
        Some(air)
    );
    assert_eq!(
        world.block_state_by_key(&world.require_level_id(&custom), landing),
        Some(sand)
    );
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, landing),
        Some(air)
    );

    let marker_position = Vec3d {
        x: 10.5,
        y: 65.0,
        z: 10.5,
    };
    let (old_marker, _) = world.spawn_stored_entity_for_test_by_key(
        &world.require_level_id(&overworld),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MARKER_ID,
        marker_position,
        crate::entity::EntityData::Marker(crate::entity::MarkerData::new()),
    );
    world.ensure_entity_transfer_full_chunk_for_test_by_key(
        &world.require_level_id(&custom),
        (0, 0),
    );
    let moved = world
        .transfer_stored_entity_tree_cross_level(
            overworld.clone(),
            custom.clone(),
            old_marker,
            crate::entity::StoredEntityTransition {
                position: marker_position,
                delta_movement: Vec3d::default(),
                y_rot: 0.0,
                x_rot: 0.0,
                relatives_mask: 0,
            },
        )
        .unwrap();
    assert!(world
        .stored_entity_for_test_by_key(&world.require_level_id(&overworld), old_marker)
        .is_none());
    assert!(world
        .stored_entity_for_test_by_key(&world.require_level_id(&custom), moved.new_root_id)
        .is_some());

    let ticks = world
        .stored_entity_for_test_by_key(&world.require_level_id(&custom), moved.new_root_id)
        .unwrap()
        .tick_count();
    world.tick(receiver.drain());
    assert_eq!(
        world
            .stored_entity_for_test_by_key(&world.require_level_id(&custom), moved.new_root_id)
            .unwrap()
            .tick_count(),
        ticks + 1,
    );

    for (slot, resource, count) in [
        (36, "minecraft:map", 1),
        (37, "minecraft:spyglass", 1),
        (38, "minecraft:snowball", 2),
    ] {
        play(
            &sender,
            &receiver,
            &mut world,
            44,
            PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                slot_num: slot,
                item: ItemStackSummary {
                    item_id: Some(item_id(resource)),
                    count,
                    component_patch: Default::default(),
                },
            }),
        );
    }
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(12, "custom-air-use-survival", AdminScope::Command),
            request: AdminRequest::ExecuteCommand {
                command: "gamemode survival moonwalker".into(),
            },
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    let mut thrown_id = None;
    let mut thrown_uuid = None;
    for (slot, sequence) in [(0, 101), (1, 102), (2, 103)] {
        play(
            &sender,
            &receiver,
            &mut world,
            44,
            PlayServerbound::SetCarriedItem { slot },
        );
        let used = play(
            &sender,
            &receiver,
            &mut world,
            44,
            PlayServerbound::UseItem(UseItem {
                hand: InteractionHand::MainHand,
                sequence,
                y_rot: 0.0,
                x_rot: -80.0,
            }),
        );
        acknowledges(&used, sequence);
        match slot {
            0 => {
                let held = world.player_inventory_slot_for_test(ConnectionId(44), 0);
                assert_eq!(held.item_id, Some(item_id("minecraft:filled_map")));
                assert_eq!(held.count, 1);
                assert_eq!(held.component_patch.map_id, Some(0));
            }
            1 => {
                let (using, hand, remaining, held) =
                    world.player_use_state_for_test(ConnectionId(44));
                assert!(using);
                assert_eq!(hand, InteractionHand::MainHand);
                assert!(remaining > 0);
                assert_eq!(held.item_id, Some(item_id("minecraft:spyglass")));
            }
            2 => {
                assert!(!world.player_use_state_for_test(ConnectionId(44)).0);
                assert_eq!(
                    world
                        .player_inventory_slot_for_test(ConnectionId(44), 2)
                        .count,
                    1
                );
                let add = used
                    .events
                    .iter()
                    .find_map(|event| match event {
                        WorldEvent::Send {
                            connection_id: ConnectionId(44),
                            packet: ClientboundPlayPacket::AddEntity(add),
                        } if add.entity_type_id
                            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNOWBALL_ID =>
                        {
                            Some(add)
                        }
                        _ => None,
                    })
                    .expect("the air-use projectile is published in the custom level");
                thrown_id = Some(add.id);
                let entity = world
                    .stored_entity_for_test_by_key(&custom_id, add.id)
                    .expect("the custom level owns the projectile");
                thrown_uuid = Some(entity.uuid());
                assert!(world
                    .stored_entity_for_test_by_key(&world.require_level_id(&overworld), add.id)
                    .is_none());
                assert!(sounds_sent(&used).iter().any(|sound| sound.sound
                    == SoundEventHolder::Reference {
                        registry_id: vanilla_sound_event_registry_id(
                            "minecraft:entity.snowball.throw"
                        )
                        .unwrap(),
                    }));
            }
            _ => unreachable!(),
        }
    }
    assert!(world
        .stored_entity_for_test_by_key(&custom_id, thrown_id.unwrap())
        .is_some());

    for (dimension, chunk) in world.pending_runtime_entity_chunks() {
        world
            .complete_runtime_entity_chunk(&dimension, chunk, None)
            .unwrap();
    }
    let request = world
        .save_request(0, 1, bbb_server_persistence::RegionCompression::Zlib)
        .unwrap();
    assert!(request.world_borders.contains_key(&custom));
    assert_eq!(request.map_writes[&0].dimension, custom.as_str());
    let thrown_uuid = thrown_uuid.unwrap();
    let saved_projectile_levels: Vec<_> = request
        .entity_writes
        .iter()
        .filter_map(|write| {
            let chunk = bbb_server_persistence::Java26EntityChunkData::parse(
                write.document.clone(),
                write.chunk_x,
                write.chunk_z,
            )
            .unwrap();
            let contains_projectile = chunk.entities().any(|entity| entity.uuid() == thrown_uuid);
            contains_projectile.then_some(&write.dimension)
        })
        .collect();
    assert_eq!(saved_projectile_levels, vec![&custom]);
    assert_eq!(request.player_patches[0].created.dimension, custom.as_str());
    assert!(request
        .chunk_writes
        .iter()
        .any(|write| write.dimension == custom));
    assert!(!request.chunk_writes.iter().any(|write| {
        write.dimension == Java26LevelKey::from(Java26BuiltinDimension::Overworld)
            && (write.chunk_x, write.chunk_z) == (0, 0)
    }));

    use crate::entity::{EntityData, GeneralLivingData};
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_BLAZE_ID,
        VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
        VANILLA_ENTITY_TYPE_SLIME_ID,
    };
    let run = |world: &mut ServerWorld, operation, command: String| {
        enqueue(
            &sender,
            WorldCommand::Admin {
                envelope: admin_envelope(
                    operation,
                    &format!("custom-entity-interaction-{operation}"),
                    AdminScope::Command,
                ),
                request: AdminRequest::ExecuteCommand { command },
            },
        )
        .unwrap();
        let tick = world.tick(receiver.drain());
        let outcome = admin_outcome(&tick);
        assert!(
            matches!(&outcome.result,
                Ok(AdminResult::Command(report)) if report.result == Some(1)
            ),
            "{outcome:?}"
        );
        tick
    };
    run(&mut world, 100, "gamemode creative moonwalker".into());
    let shot = thrown_id.unwrap();
    let projectile = world
        .stored_entity_for_test_by_key(&custom_id, shot)
        .unwrap();
    let position = projectile.position();
    let motion = projectile.delta_movement();
    let shot_position = Vec3d {
        x: position.x + motion.x * 0.5,
        y: position.y + motion.y * 0.5,
        z: position.z + motion.z * 0.5,
    };
    let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_BLAZE_ID).unwrap();
    data.set_no_ai(true);
    let (blaze, blaze_uuid) = world.spawn_stored_entity_for_test_by_key(
        &custom_id,
        VANILLA_ENTITY_TYPE_BLAZE_ID,
        shot_position,
        EntityData::GeneralLiving(data),
    );
    for _ in 0..20 {
        world.tick(receiver.drain());
        if world
            .stored_entity_for_test_by_key(&custom_id, shot)
            .is_none()
        {
            break;
        }
    }
    assert!(world
        .stored_entity_for_test_by_key(&custom_id, shot)
        .is_none());
    assert_eq!(
        world
            .stored_entity_for_test_by_key(&custom_id, blaze)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        17.0
    );
    run(
        &mut world,
        101,
        format!("execute in example:moon run effect give {blaze_uuid} minecraft:oozing infinite"),
    );
    assert!(world
        .stored_entity_for_test_by_key(&custom_id, blaze)
        .unwrap()
        .living()
        .unwrap()
        .active_effect("minecraft:oozing")
        .is_some());
    let mut killed = run(
        &mut world,
        102,
        format!("execute in example:moon run kill {blaze_uuid}"),
    );
    let mut slimes = std::collections::BTreeSet::new();
    for _ in 0..60 {
        for event in &killed.events {
            if let WorldEvent::Send {
                connection_id: ConnectionId(44),
                packet: ClientboundPlayPacket::AddEntity(add),
            } = event
            {
                if add.entity_type_id == VANILLA_ENTITY_TYPE_SLIME_ID {
                    slimes.insert(add.id);
                }
            }
        }
        if slimes.len() >= 2 {
            break;
        }
        killed = world.tick(receiver.drain());
    }
    assert_eq!(slimes.len(), 2);
    let overworld_id = world.require_level_id(&overworld);
    for id in slimes {
        assert!(world
            .stored_entity_for_test_by_key(&custom_id, id)
            .is_some());
        assert!(world
            .stored_entity_for_test_by_key(&overworld_id, id)
            .is_none());
    }

    let player_position = world.publication().players[0].position;
    let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_IRON_GOLEM_ID).unwrap();
    data.set_no_ai(true);
    let (golem, golem_uuid) = world.spawn_stored_entity_for_test_by_key(
        &custom_id,
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
        Vec3d {
            x: player_position.x + 1.0,
            ..player_position
        },
        EntityData::GeneralLiving(data),
    );
    run(
        &mut world,
        103,
        format!("execute in example:moon run damage {golem_uuid} 40 minecraft:generic"),
    );
    assert_eq!(
        world
            .stored_entity_for_test_by_key(&custom_id, golem)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        60.0
    );
    let interact = |world: &mut ServerWorld, entity_id, location| {
        play(
            &sender,
            &receiver,
            world,
            44,
            PlayServerbound::Interact(InteractEntity {
                entity_id,
                hand: InteractionHand::MainHand,
                location,
                using_secondary_action: false,
            }),
        )
    };
    world.set_inventory_slot_for_test(
        ConnectionId(44),
        3,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:iron_ingot")),
            count: 1,
            component_patch: Default::default(),
        },
    );
    play(
        &sender,
        &receiver,
        &mut world,
        44,
        PlayServerbound::SetCarriedItem { slot: 3 },
    );
    let repaired = interact(
        &mut world,
        golem,
        Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    );
    assert_eq!(
        world
            .stored_entity_for_test_by_key(&custom_id, golem)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        85.0
    );
    assert!(sounds_sent(&repaired).iter().any(|sound| sound.sound
        == SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id("minecraft:entity.iron_golem.repair")
                .unwrap(),
        }));

    let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap();
    data.set_no_ai(true);
    let (cow, _) = world.spawn_stored_entity_for_test_by_key(
        &custom_id,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            z: player_position.z + 1.0,
            ..player_position
        },
        EntityData::GeneralLiving(data),
    );
    world.set_inventory_slot_for_test(
        ConnectionId(44),
        3,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:lead")),
            count: 1,
            component_patch: Default::default(),
        },
    );
    let leashed = interact(
        &mut world,
        cow,
        Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    );
    assert!(leashed.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityLink(link), .. }
            if link.source_id == cow && link.dest_id == world.player_entity_id_for_test(ConnectionId(44))
    )));

    let (stand, _) = world.spawn_stored_entity_for_test_by_key(
        &custom_id,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: player_position.x - 1.0,
            ..player_position
        },
        EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    );
    world.set_inventory_slot_for_test(
        ConnectionId(44),
        3,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:iron_helmet")),
            count: 1,
            component_patch: Default::default(),
        },
    );
    world.tick(receiver.drain());
    interact(
        &mut world,
        stand,
        Vec3d {
            x: 0.0,
            y: 1.8,
            z: 0.0,
        },
    );
    assert_eq!(
        world
            .stored_entity_for_test_by_key(&custom_id, stand)
            .unwrap()
            .living()
            .unwrap()
            .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::Head)
            .unwrap()
            .item_id(),
        Some(item_id("minecraft:iron_helmet"))
    );
    assert_eq!(
        world
            .player_inventory_slot_for_test(ConnectionId(44), 3)
            .count,
        1
    );
    for id in [golem, cow, stand] {
        assert!(world
            .stored_entity_for_test_by_key(&custom_id, id)
            .is_some());
        assert!(world
            .stored_entity_for_test_by_key(&overworld_id, id)
            .is_none());
    }
}

/// A stem naming a generator this server does not implement opens the save and
/// declines that one level.
///
/// This is the shape of every official world: `WorldPresets.FLAT` leaves
/// `minecraft:noise` on the nether and the end stems, so a server that refused
/// the map whole would refuse every save it did not write itself. The level is
/// simply not among the ones it serves -- it does not tick, the login does not
/// list it, and nothing can be sent to it.
#[test]
fn a_stem_this_server_has_no_generator_for_is_declared_and_left_unserved() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    for name in ["minecraft:the_nether", "minecraft:the_end"] {
        let level_key = Java26LevelKey::parse(name).expect("built-in level key");
        let stem = candidate
            .world_gen_settings
            .dimensions
            .get_mut(&level_key)
            .expect("the created settings declare every built-in stem");
        stem.generator = "minecraft:noise".to_string();
    }
    let world = ServerWorld::restore(ServerWorldConfig::default(), &candidate)
        .expect("a save whose other stems are readable opens");
    assert_eq!(
        world.served_builtin_dimensions(),
        vec![Java26BuiltinDimension::Overworld],
        "only the stem with a generator became a level"
    );
    assert_eq!(
        world.unserved_levels(),
        vec![
            ("minecraft:the_nether", "minecraft:noise"),
            ("minecraft:the_end", "minecraft:noise"),
        ],
        "and the other two are declared as unserved rather than forgotten"
    );
}

#[test]
fn the_builtin_overworld_noise_stem_uses_the_startup_generation_and_is_served() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate.world_gen_settings.dimensions.insert(
        Java26LevelKey::from(Java26BuiltinDimension::Overworld),
        crate::chunk::noise_generator::overworld_noise_stem(),
    );
    for name in ["minecraft:the_nether", "minecraft:the_end"] {
        let level_key = Java26LevelKey::parse(name).expect("built-in level key");
        candidate
            .world_gen_settings
            .dimensions
            .get_mut(&level_key)
            .expect("the fixture declares every built-in stem")
            .generator = "minecraft:noise".to_string();
    }
    let startup = crate::chunk::noise_generator::overworld_startup(true);
    let world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(Arc::clone(&startup)),
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();

    assert_eq!(
        world.served_builtin_dimensions(),
        vec![
            Java26BuiltinDimension::Overworld,
            Java26BuiltinDimension::Nether,
            Java26BuiltinDimension::End,
        ]
    );
    assert!(world.unserved_levels().is_empty());
    assert_eq!(world.level_sea_level(Java26BuiltinDimension::Overworld), 63);
}

#[test]
fn an_owned_alternate_overworld_noise_profile_is_served() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    let mut stem = crate::chunk::noise_generator::overworld_noise_stem();
    let NbtSummaryValue::Compound(generator) = &mut stem.raw_generator else {
        unreachable!()
    };
    generator
        .iter_mut()
        .find(|entry| entry.name == "settings")
        .expect("the noise fixture has settings")
        .value = NbtSummaryValue::string("minecraft:large_biomes");
    candidate.world_gen_settings.dimensions.insert(
        Java26LevelKey::from(Java26BuiltinDimension::Overworld),
        stem,
    );
    let startup = crate::chunk::noise_generator::overworld_startup(true);
    let world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(startup),
            ..ServerWorldConfig::default()
        },
        &candidate,
    )
    .unwrap();
    assert_eq!(world.level_sea_level(Java26BuiltinDimension::Overworld), 63);
    assert!(world.unserved_levels().is_empty());
}

/// `ServerPlayer.teleport` (`ServerPlayer.java:1102-1127`) for a transition
/// that changes level: the `KEEP_ALL_DATA` respawn goes first, the old level's
/// watchers lose the body, the new level's gain it, and every later read on
/// that session reaches the new level.
#[test]
fn a_cross_level_teleport_opens_with_a_keep_all_data_respawn_and_moves_the_session() {
    let (sender, receiver) = mailbox();
    for (connection, name, uuid) in [(1_u64, "alice", 1_u128), (2, "bob", 2)] {
        enqueue(
            &sender,
            WorldCommand::Join {
                client_information: Default::default(),
                context: JoinContext::default(),
                connection_id: ConnectionId(connection),
                profile: profile(name, uuid),
            },
        )
        .unwrap();
    }
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    world.tick(receiver.drain());
    for connection_id in [ConnectionId(1), ConnectionId(2)] {
        world.reset_pending_chunks_for_test(connection_id, &[]);
        world.reconcile_player_tracking_for_test(connection_id);
    }

    let mut events = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Nether.into(),
            position: Vec3d {
                x: 100.5,
                y: 70.0,
                z: -20.5,
            },
            y_rot: 90.0,
            x_rot: 10.0,
            relatives_mask: 0,
        },
        &mut events,
    ));
    assert_eq!(
        world.player_dimension(ConnectionId(1)),
        Java26BuiltinDimension::Nether
    );
    assert_eq!(
        world.player_dimension(ConnectionId(2)),
        Java26BuiltinDimension::Overworld
    );

    // `:1104`, ahead of everything else the transition sends.
    let respawn = events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::Respawn(_),
                }
            )
        })
        .expect("the cross-level arm opens with a respawn");
    assert_eq!(respawn, 0);
    let WorldEvent::Send {
        packet: ClientboundPlayPacket::Respawn(respawn),
        ..
    } = &events[0]
    else {
        unreachable!("checked above")
    };
    assert_eq!(respawn.data_to_keep, 3);
    assert_eq!(respawn.common_spawn_info.dimension, "minecraft:the_nether");
    // The arriving level is described by its own dimension type, not by the one
    // the session left: `minecraft:the_nether` is the fourth element the
    // built-in pack registers, behind `overworld`, `overworld_caves` and
    // `the_end` (`ResourceManagerRegistryLoadTask.java:60-62`).
    assert_eq!(respawn.common_spawn_info.dimension_type_id, 3);
    assert_eq!(
        respawn.common_spawn_info.dimension_type_id,
        world
            .synchronized_registries()
            .dimension_type_index("minecraft:the_nether")
            .unwrap()
    );

    // `:1108` reaching `ChunkMap.removeEntity` (`ChunkMap.java:1165-1178`):
    // the session left behind loses the body.
    assert!(events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::RemoveEntities(_),
        }
    )));
    // `:1121` reaching `ChunkMap.addEntity`: nobody is in the nether to be
    // paired with, so no `AddEntity` goes anywhere.
    assert!(!events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::AddEntity(_),
            ..
        }
    )));
    // The tracking view was rebuilt in the new level, so the columns queued for
    // this session are the nether's.
    assert!(events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::SetChunkCacheCenter(center),
        } if (center.chunk_x, center.chunk_z) == (6, -2)
    )));
    // `ServerGamePacketListenerImpl.teleport` (`:1246-1255`) moved the
    // server-side position before the packet went out.
    assert!(events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::PlayerPosition(update),
        } if update.position.x == 100.5 && update.y_rot == 90.0
    )));
}

/// `ExecuteCommand.java:255-260` through `TeleportCommand.teleportToPos`
/// (`TeleportCommand.java:174-215`): `/execute in minecraft:the_nether run tp`
/// retargets the source onto the nether and the live cross-level arm opens
/// with `KEEP_ALL_DATA` before anything else (`ServerPlayer.java:1102-1104`).
#[test]
fn execute_in_the_nether_teleport_moves_the_session_with_a_keep_all_data_respawn_first() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    assert_eq!(
        world.player_dimension(ConnectionId(1)),
        Java26BuiltinDimension::Overworld
    );

    let tick = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "execute in minecraft:the_nether run tp @s 100 70 -20",
    );
    assert_eq!(
        world.player_dimension(ConnectionId(1)),
        Java26BuiltinDimension::Nether
    );

    let respawn = tick
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::Respawn(_),
                }
            )
        })
        .expect("the cross-level command opens with a respawn");
    assert_eq!(respawn, 0);
    let WorldEvent::Send {
        packet: ClientboundPlayPacket::Respawn(respawn),
        ..
    } = &tick.events[0]
    else {
        unreachable!("checked above")
    };
    assert_eq!(respawn.data_to_keep, 3);
    assert_eq!(respawn.common_spawn_info.dimension, "minecraft:the_nether");
    assert_eq!(
        respawn.common_spawn_info.dimension_type_id,
        world
            .synchronized_registries()
            .dimension_type_index("minecraft:the_nether")
            .unwrap()
    );

    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::PlayerPosition(update),
        } if update.position.x == 100.5 && update.position.z == -19.5
    )));
    let feedback = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SystemChat(chat),
            } => Some(chat.content.component()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(feedback.len(), 1, "{feedback:?}");
    let NbtSummaryValue::Compound(entries) = feedback[0] else {
        panic!("command feedback is a component");
    };
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.name == "translate")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::string(
            "commands.teleport.success.location.single"
        ))
    );
}

/// A transition naming the level the session is already in is the short arm at
/// `ServerPlayer.java:1095-1100`: the position update and nothing else.
#[test]
fn a_same_level_teleport_sends_only_the_position_update() {
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    world.tick(receiver.drain());

    let mut events = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Overworld.into(),
            position: Vec3d {
                x: 4.5,
                y: 70.0,
                z: 4.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut events,
    ));
    assert_eq!(events.len(), 1);
    assert!(matches!(
        &events[0],
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::PlayerPosition(_),
        }
    ));
}

/// A level the loaded settings do not declare has no `ServerLevel` here, and
/// `MinecraftServer.getLevel` would answer null for it, so the transition is
/// refused rather than filed against a level that does not exist.
#[test]
fn a_teleport_to_a_level_this_server_does_not_run_is_refused() {
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate
        .world_gen_settings
        .dimensions
        .remove(&Java26LevelKey::from(Java26BuiltinDimension::End));
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());

    let mut events = Vec::new();
    assert!(!world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::End.into(),
            position: Vec3d {
                x: 0.5,
                y: 70.0,
                z: 0.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut events,
    ));
    assert!(events.is_empty());
    assert_eq!(
        world.player_dimension(ConnectionId(1)),
        Java26BuiltinDimension::Overworld
    );
}

/// The playerdata document with its `Dimension` replaced, which is the one
/// field these dimension tests vary.
fn with_dimension(player: Java26PlayerData, dimension: &str) -> Java26PlayerData {
    let uuid = player.uuid();
    let mut raw = player.into_raw();
    for entry in raw.entries_mut().iter_mut() {
        if entry.name == "Dimension" {
            entry.value = NbtSummaryValue::string(dimension);
        }
    }
    Java26PlayerData::parse(raw, uuid).unwrap()
}

/// `ServerPlayer.teleport` forgets the experience it last sent
/// (`ServerPlayer.java:1130`), so the respawned client gets its bar back.
#[test]
fn a_dimension_change_resends_the_experience_bar() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    world.tick(Vec::new());

    let moved = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "execute in minecraft:the_nether run tp @s 100 70 -20",
    );
    let next = world.tick(Vec::new());
    let resent = [&moved, &next].iter().any(|tick| {
        tick.events.iter().any(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::SetExperience(_),
                }
            )
        })
    });
    assert!(resent);
}

/// `BaseSpawner.serverTick` (`BaseSpawner.java:82-186`) runs in every level,
/// a datapack level included.
#[test]
fn a_spawner_in_a_custom_level_spawns_its_mob() {
    use crate::tests::entities::{console_command, console_command_answer};

    let mut candidate = restore_candidate(0, None, Vec::new());
    let custom = Java26LevelKey::parse("example:moon").expect("custom level key");
    let overworld = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    let stem = candidate.world_gen_settings.dimensions[&overworld].clone();
    candidate
        .world_gen_settings
        .dimensions
        .insert(custom.clone(), stem);
    candidate.raids.insert(custom.clone(), Default::default());
    candidate.world_borders.insert(
        custom.clone(),
        bbb_server_persistence::Java26WorldBorderSettings::default(),
    );
    let uuid = Uuid::from_u128(44);
    candidate.players.insert(
        uuid,
        (
            bbb_server_persistence::PlayerDataSource::Primary,
            with_dimension(
                restore_fixture_player(uuid, [8.5, 65.0, 8.5], [0.0, 0.0], true),
                custom.as_str(),
            ),
        ),
    );
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(44),
            profile: profile("moonwalker", 44),
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    let teleport_id = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .unwrap();
    for packet in [
        PlayServerbound::AcceptTeleportation { id: teleport_id },
        PlayServerbound::PlayerLoaded,
    ] {
        enqueue(
            &sender,
            WorldCommand::Play {
                connection_id: ConnectionId(44),
                packet,
            },
        )
        .unwrap();
    }
    world.tick(receiver.drain());

    let placed = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "execute in example:moon run setblock 8 65 11 spawner{Delay:0,SpawnRange:1,SpawnData:\
         {entity:{id:\"zombie\"},custom_spawn_rules:{block_light_limit:[0,15],\
         sky_light_limit:[0,15]}}}",
    );
    assert_eq!(console_command_answer(&placed).0, Some(1));
    for _ in 0..40 {
        world.tick(receiver.drain());
    }
    let counted = console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "execute in example:moon if entity @e[type=minecraft:zombie]",
    );
    assert!(console_command_answer(&counted)
        .0
        .is_some_and(|count| count >= 1));
}
