//! Save and restore: the candidate a level document parses into, the round trip
//! a save request makes, and the unattended save's own interval.

use super::*;

mod ender_pearls;
mod entity_living_lists;
mod restart_fields;
mod statistics;

fn restore_single_state_section(y: i8, name: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        nbt("Y", NbtSummaryValue::Byte(y)),
        nbt(
            "block_states",
            NbtSummaryValue::Compound(vec![nbt(
                "palette",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![nbt(
                    "Name",
                    NbtSummaryValue::string(name),
                )])]),
            )]),
        ),
        nbt(
            "biomes",
            NbtSummaryValue::Compound(vec![nbt(
                "palette",
                NbtSummaryValue::List(vec![NbtSummaryValue::string(
                    "minecraft:plains".to_string(),
                )]),
            )]),
        ),
    ])
}

/// Section Y=-4 with a two-entry palette: stone everywhere except dirt at
/// local (1, 0, 2), which is world (1, -64, 2). A two-entry block palette
/// packs 4 bits per entry (16 per long, 256 longs); entry index 33 lands in
/// long 2 at shift 4.
fn restore_dirt_section() -> NbtSummaryValue {
    let mut data = vec![0_i64; 256];
    data[2] = 1 << 4;
    NbtSummaryValue::Compound(vec![
        nbt("Y", NbtSummaryValue::Byte(-4)),
        nbt(
            "block_states",
            NbtSummaryValue::Compound(vec![
                nbt(
                    "palette",
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::Compound(vec![nbt(
                            "Name",
                            NbtSummaryValue::string("minecraft:stone"),
                        )]),
                        NbtSummaryValue::Compound(vec![nbt(
                            "Name",
                            NbtSummaryValue::string("minecraft:dirt"),
                        )]),
                    ]),
                ),
                nbt("data", NbtSummaryValue::LongArray(data)),
            ]),
        ),
        nbt(
            "biomes",
            NbtSummaryValue::Compound(vec![nbt(
                "palette",
                NbtSummaryValue::List(vec![NbtSummaryValue::string(
                    "minecraft:plains".to_string(),
                )]),
            )]),
        ),
    ])
}

fn restore_mixed_biome_section(second_biome: &str) -> NbtSummaryValue {
    let mut section = restore_single_state_section(-4, "minecraft:stone");
    let NbtSummaryValue::Compound(fields) = &mut section else {
        unreachable!()
    };
    let biome = fields
        .iter_mut()
        .find(|entry| entry.name == "biomes")
        .unwrap();
    biome.value = NbtSummaryValue::Compound(vec![
        nbt(
            "palette",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::string("minecraft:plains"),
                NbtSummaryValue::string(second_biome),
            ]),
        ),
        // Biome storage is one bit for two values; entry 1 is local (1,0,0).
        nbt("data", NbtSummaryValue::LongArray(vec![2])),
    ]);
    section
}

fn restore_fixture_chunk(sections: Vec<NbtSummaryValue>) -> Java26ChunkData {
    Java26ChunkData::parse(restore_fixture_chunk_raw(sections), restore_chunk_context()).unwrap()
}

#[test]
fn restore_preserves_a_playing_jukebox_and_its_saved_column() {
    let NbtSummaryValue::Compound(record) = bbb_protocol::parse_snbt_compound(
        r#"{id:"minecraft:jukebox",x:1,y:-64,z:2,RecordItem:{id:"minecraft:music_disc_13",count:1,components:{"minecraft:jukebox_playable":"minecraft:cat"}},ticks_since_song_started:42L}"#,
    ).unwrap() else { unreachable!() };
    let mut section = restore_dirt_section();
    let NbtSummaryValue::Compound(fields) = &mut section else {
        unreachable!()
    };
    let NbtSummaryValue::Compound(states) = &mut fields
        .iter_mut()
        .find(|field| field.name == "block_states")
        .unwrap()
        .value
    else {
        unreachable!()
    };
    let NbtSummaryValue::List(palette) = &mut states
        .iter_mut()
        .find(|field| field.name == "palette")
        .unwrap()
        .value
    else {
        unreachable!()
    };
    palette[1] = bbb_protocol::parse_snbt_compound(
        r#"{Name:"minecraft:jukebox",Properties:{has_record:"true"}}"#,
    )
    .unwrap();
    let mut raw = restore_fixture_chunk_raw(vec![section]);
    raw.entries_mut()
        .iter_mut()
        .find(|field| field.name == "block_entities")
        .unwrap()
        .value = NbtSummaryValue::List(vec![NbtSummaryValue::Compound(record)]);
    let saved = Java26ChunkData::parse(&raw, restore_chunk_context()).unwrap();
    let candidate = restore_candidate(4200, Some(saved.clone()), Vec::new());
    let mut world = spawn_pinned_restore(creative_fixture_config(), &candidate);
    let at = BlockPos { x: 1, y: -64, z: 2 };
    let jukebox = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .unwrap()
        .jukebox()
        .unwrap();
    assert_eq!(
        jukebox.item().item_resource_id(),
        Some("minecraft:music_disc_13")
    );
    assert_eq!(jukebox.song_player().song_id(), Some("minecraft:cat"));
    assert_eq!(jukebox.song_player().ticks_since_song_started(), 42);
    assert_eq!(
        live_block(&mut world, BlockPos { x: 0, ..at }),
        Some(
            bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
                "minecraft:stone",
                &BTreeMap::new()
            )
            .unwrap()
        )
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 2, y: -64, z: 2 },
        block_state_id("minecraft:diamond_block"),
        SET_BLOCK_UPDATE_ALL,
    ));
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let write = request
        .chunk_writes
        .iter()
        .find(|write| (write.chunk_x, write.chunk_z) == (0, 0))
        .unwrap();
    let written = Java26ChunkData::parse(&write.document, restore_chunk_context()).unwrap();
    let record = written
        .block_entities()
        .iter()
        .find(|record| field(record, "id") == &NbtSummaryValue::String("minecraft:jukebox".into()))
        .unwrap();
    assert_eq!(
        field(record, "ticks_since_song_started"),
        &NbtSummaryValue::Long(42)
    );
    let NbtSummaryValue::Compound(item) = field(record, "RecordItem") else {
        unreachable!()
    };
    let NbtSummaryValue::Compound(components) = field(item, "components") else {
        unreachable!()
    };
    assert_eq!(
        field(components, "minecraft:jukebox_playable"),
        &NbtSummaryValue::String("minecraft:cat".into())
    );
}

#[test]
fn restore_projects_game_time_and_a_matching_chunk_as_an_empty_overlay() {
    let candidate = restore_candidate(
        4_200,
        Some(restore_fixture_chunk(vec![
            restore_single_state_section(-4, "minecraft:stone"),
            restore_single_state_section(4, "minecraft:air"),
        ])),
        Vec::new(),
    );
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    // `Data.Time` restores into the world's own game time; the server tick
    // counter is `MinecraftServer.tickCount`, which starts at zero on every
    // boot regardless of what the save says.
    assert_eq!(world.game_time(), 4_200);
    assert_eq!(world.publication().tick, 0);
    assert!(world.publication().changed_blocks.is_empty());

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(request.level_state.game_time, 4_200);
    assert_eq!(
        request.level_state.spawn,
        Java26LevelSpawn::new("minecraft:overworld".to_string(), [0, 64, 0], 0.0, 0.0)
    );
    // Only the generation ring the forced ticket loaded at startup is new.
    assert!(!request
        .chunk_writes
        .iter()
        .any(|write| (write.chunk_x, write.chunk_z) == (0, 0)));
    assert!(request.player_patches.is_empty());
}

#[test]
fn ticket_save_reload_preserves_partial_portal_time_and_does_not_revive_expiry() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-portal-ticket-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-portal-ticket-restart");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    world
        .place_portal_ticket(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 1, y: 64, z: 1 },
        )
        .unwrap();
    for _ in 0..25 {
        world.tick(Vec::new());
    }

    let request = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(
        request.chunk_tickets[&level_key(Java26BuiltinDimension::Overworld)],
        vec![bbb_server_persistence::Java26SavedChunkTicket {
            chunk_x: 0,
            chunk_z: 0,
            level: 30,
            ticks_left: Some(275),
        }]
    );
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    assert_eq!(
        reloaded.chunk_tickets[&level_key(Java26BuiltinDimension::Overworld)],
        request.chunk_tickets[&level_key(Java26BuiltinDimension::Overworld)]
    );
    let mut restarted = ServerWorld::restore(creative_fixture_config(), &reloaded).unwrap();
    for _ in 0..275 {
        restarted.tick(Vec::new());
    }
    assert_eq!(
        restarted.restored_chunk_ticket_for_test(Java26BuiltinDimension::Overworld, (0, 0)),
        Some((30, Some(0)))
    );
    restarted.tick(Vec::new());
    assert_eq!(
        restarted.restored_chunk_ticket_for_test(Java26BuiltinDimension::Overworld, (0, 0)),
        None
    );

    let expired = restarted
        .save_request(1_786_000_000_002, 102, RegionCompression::Zlib)
        .unwrap();
    assert!(expired.chunk_tickets[&level_key(Java26BuiltinDimension::Overworld)].is_empty());
    storage.save_snapshot(&reloaded, &expired).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    assert!(reloaded
        .chunk_tickets
        .get(&level_key(Java26BuiltinDimension::Overworld))
        .is_none_or(Vec::is_empty));
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn forceload_ticket_save_projection_reopens_as_forced() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-forceload-ticket-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-forceload-ticket-restart");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(operator_config(&[("op", 1)]), &candidate).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    chat_command(&sender, &receiver, &mut world, 1, "forceload add 32 48");

    let request = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    assert!(
        request.chunk_tickets[&level_key(Java26BuiltinDimension::Overworld)].contains(
            &bbb_server_persistence::Java26SavedChunkTicket {
                chunk_x: 2,
                chunk_z: 3,
                level: 31,
                ticks_left: None,
            }
        )
    );
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let restarted = ServerWorld::restore(creative_fixture_config(), &reloaded).unwrap();
    assert_eq!(
        restarted.restored_chunk_ticket_for_test(Java26BuiltinDimension::Overworld, (2, 3)),
        Some((31, None))
    );
    assert!(restarted
        .entity_ticking_chunks(Java26BuiltinDimension::Overworld)
        .contains(&(2, 3)));
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn restore_projects_saved_block_states_into_the_overlay() {
    let candidate = restore_candidate(
        4_200,
        Some(restore_fixture_chunk(vec![restore_dirt_section()])),
        Vec::new(),
    );
    let world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    let dirt_id = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:dirt",
        &BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(
        world.publication().changed_blocks,
        vec![BlockSnapshot {
            pos: BlockPos { x: 1, y: -64, z: 2 },
            block_state_id: dirt_id,
        }]
    );
}

#[test]
fn restore_loads_saved_blocks_into_the_authoritative_chunk_column() {
    let candidate = restore_candidate(
        4_200,
        Some(restore_fixture_chunk(vec![restore_dirt_section()])),
        Vec::new(),
    );
    let mut world = spawn_pinned_restore(creative_fixture_config(), &candidate);
    let dirt_id = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:dirt",
        &BTreeMap::new(),
    )
    .unwrap();
    let stone_id = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:stone",
        &BTreeMap::new(),
    )
    .unwrap();

    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(9),
            profile: profile("erin", 9),
        },
    )
    .unwrap();
    let joined = world.tick(receiver.drain());
    let sent_column = |chunk_x: i32, chunk_z: i32| {
        let chunk = joined
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::LevelChunkWithLight(chunk),
                    ..
                } if (chunk.x, chunk.z) == (chunk_x, chunk_z) => Some(chunk),
                _ => None,
            })
            .expect("the join view covers this column");
        // The overworld's own floor, out of the dimension type its stem names
        // rather than out of a constant.
        let min_y = bbb_server_datapack::dimension_type::builtin_dimension_type_shape(
            "minecraft:overworld",
        )
        .unwrap()
        .min_y;
        bbb_server_types::chunk_data::ChunkColumn::decode_section_data(
            min_y,
            &chunk.chunk_data.section_data,
        )
        .unwrap()
    };
    // The join chunk is encoded from the restored column, so the saved dirt
    // block is already visible in the section data itself.
    let column = sent_column(0, 0);
    assert_eq!(column.get_block_state(1, -64, 2), Some(dirt_id));
    assert_eq!(column.get_block_state(0, -64, 2), Some(stone_id));
    assert_eq!(column.get_block_state(1, -63, 2), Some(stone_id));

    // The neighbouring column has no saved document, so it is exactly what the
    // declared generator produces: the save wins only where it says something.
    let neighbour = sent_column(1, 0);
    let plains_biome_id = crate::registry_sync::SynchronizedRegistries::load(None)
        .unwrap()
        .registry_element_index("minecraft:worldgen/biome", "minecraft:plains")
        .expect("the synchronized built-in registry publishes plains");
    assert_eq!(neighbour.get_block_state(1, -64, 2), Some(stone_id));
    assert_eq!(
        neighbour.encode_section_data(),
        default_overworld_level_source()
            .unwrap()
            .generate_chunk(1, 0, plains_biome_id)
            .into_column()
            .encode_section_data()
    );
}

#[test]
fn restore_preserves_a_saved_section_biome_palette_on_the_wire() {
    let candidate = restore_candidate(
        4_200,
        Some(restore_fixture_chunk(vec![restore_mixed_biome_section(
            "minecraft:desert",
        )])),
        Vec::new(),
    );
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(91),
            profile: profile("biome_restore", 91),
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    let packet = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::LevelChunkWithLight(packet),
                ..
            } if (packet.x, packet.z) == (0, 0) => Some(packet),
            _ => None,
        })
        .unwrap();
    let column = bbb_server_types::chunk_data::ChunkColumn::decode_section_data(
        -64,
        &packet.chunk_data.section_data,
    )
    .unwrap();
    let registries = crate::registry_sync::SynchronizedRegistries::load(None).unwrap();
    let plains = registries
        .registry_element_index("minecraft:worldgen/biome", "minecraft:plains")
        .unwrap();
    let desert = registries
        .registry_element_index("minecraft:worldgen/biome", "minecraft:desert")
        .unwrap();
    assert_eq!(column.get_noise_biome(0, -16, 0), Some(plains));
    assert_eq!(column.get_noise_biome(1, -16, 0), Some(desert));
}

#[test]
fn join_applies_a_restored_motion_inside_world_bounds() {
    let uuid = Uuid::from_u128(7);
    let custom_name = bbb_protocol::component_styled_literal_nbt(
        "Remembered",
        &bbb_protocol::ComponentStyle {
            italic: Some(false),
            ..Default::default()
        },
    )
    .unwrap();
    let mut raw = restore_fixture_player(uuid, [3.5, 70.0, 4.5], [90.0, -30.0], true).into_raw();
    raw.entries_mut()
        .push(nbt("CustomName", custom_name.clone()));
    let custom_data = vec![nbt("remembered", NbtSummaryValue::Short(42))];
    raw.entries_mut()
        .push(nbt("data", NbtSummaryValue::Compound(custom_data.clone())));
    let saved_player = Java26PlayerData::parse(raw, uuid).unwrap();
    let candidate = restore_candidate(4_200, None, vec![(uuid, saved_player)]);
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(7),
            profile: profile("alice", 7),
        },
    )
    .unwrap();
    let joined = world.tick(receiver.drain());

    let player = &joined.snapshot.players[0];
    assert_eq!(
        player.position,
        Vec3d {
            x: 3.5,
            y: 70.0,
            z: 4.5
        }
    );
    assert_eq!(
        player.rotation,
        PlayerRotation {
            y_rot: 90.0,
            x_rot: -30.0
        }
    );
    // The join tick's travel moves the new body by its zero delta, which
    // leaves the saved `OnGround` alone (`Entity.java:757`).
    assert!(player.on_ground);
    assert!(matches!(
        &joined.events[12],
        WorldEvent::Send {
            packet: ClientboundPlayPacket::PlayerPosition(update),
            ..
        } if update.position == player.position
            && update.y_rot == 90.0
            && update.x_rot == -30.0
    ));
    assert_eq!(
        world
            .save_request(1, 2, RegionCompression::Zlib)
            .unwrap()
            .player_patches[0]
            .custom_name,
        Some(custom_name.clone())
    );
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(7),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(8),
            profile: profile("alice", 7),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    let patch = world
        .save_request(2, 3, RegionCompression::Zlib)
        .unwrap()
        .player_patches
        .pop()
        .unwrap();
    assert_eq!(patch.custom_name, Some(custom_name.clone()));
    assert_eq!(patch.custom_data, custom_data);
    let resolver = crate::world::registry_runtime::item_stack_resolver_for_test();
    let updated = candidate.players[&uuid]
        .1
        .raw_with_player_save_patch(&patch, resolver)
        .unwrap();
    let updated = Java26PlayerData::parse(updated, uuid).unwrap();
    assert_eq!(updated.custom_name(), Some(&custom_name));
    assert_eq!(updated.custom_data(), custom_data);
}

#[test]
fn join_falls_back_to_the_default_spawn_when_the_restored_position_is_outside_bounds() {
    let uuid = Uuid::from_u128(7);
    let candidate = restore_candidate(
        4_200,
        None,
        vec![(
            uuid,
            // Above the overworld build height, which `Level.isInWorldBounds`
            // refuses; the world is no longer one chunk wide, so a distant but
            // legal x is not out of bounds any more.
            restore_fixture_player(uuid, [100.5, 400.0, 0.5], [90.0, -30.0], true),
        )],
    );
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(7),
            profile: profile("alice", 7),
        },
    )
    .unwrap();
    let joined = world.tick(receiver.drain());

    let player = &joined.snapshot.players[0];
    assert_eq!(
        player.position,
        Vec3d {
            x: -9.5,
            y: 64.0,
            z: -9.5
        }
    );
    assert_eq!(
        player.rotation,
        PlayerRotation {
            y_rot: 0.0,
            x_rot: 0.0
        }
    );
    assert!(!player.on_ground);
}

#[test]
fn save_request_round_trips_the_restored_overlay_and_player_motions() {
    let uuid = Uuid::from_u128(7);
    let candidate = restore_candidate(
        4_200,
        Some(restore_fixture_chunk(vec![restore_dirt_section()])),
        vec![(
            uuid,
            restore_fixture_player(uuid, [3.5, 70.0, 4.5], [90.0, -30.0], true),
        )],
    );
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();

    // Before the first tick the projection is exactly the restored candidate.
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(request.level_state.game_time, 4_200);
    assert_eq!(request.level_state.last_played, 1_786_000_000_999);
    assert_eq!(request.region_timestamp, 200);
    assert_eq!(request.region_compression, RegionCompression::Zlib);
    assert!(request.player_patches.is_empty());
    assert!(!request
        .chunk_writes
        .iter()
        .any(|write| (write.chunk_x, write.chunk_z) == (0, 0)));
    assert_eq!(
        world
            .chunk_store_for_test(Java26BuiltinDimension::Overworld)
            .get_block_state(1, -64, 2),
        Some(block_state_id("minecraft:dirt"))
    );

    // After the join tick the connected player projects into a motion patch.
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(7),
            profile: profile("alice", 7),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(request.level_state.game_time, 4_201);
    // `Player.readAdditionalSaveData`'s `fromSlots` (`Player.java:641`) took
    // the fixture document's one `EnderItems` entry into the live container.
    let mut expected_ender_items = vec![ItemStackSummary::empty(); 27];
    expected_ender_items[26] = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_DIRT_ID),
        count: 1,
        component_patch: Default::default(),
    };
    let mut expected_inventory = vec![ItemStackSummary::empty(); 43];
    expected_inventory[35] = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 32,
        component_patch: Default::default(),
    };
    assert_eq!(
        request.player_patches[0]
            .created
            .ender_items
            .runtime_slots()
            .iter()
            .map(|stack| stack.summary().clone())
            .collect::<Vec<_>>(),
        expected_ender_items,
    );
    let expected_ender_items = request.player_patches[0].created.ender_items.clone();
    assert_eq!(
        request.player_patches[0]
            .created
            .inventory
            .runtime_slots()
            .iter()
            .map(|stack| stack.summary().clone())
            .collect::<Vec<_>>(),
        expected_inventory,
    );
    let expected_inventory = request.player_patches[0].created.inventory.clone();
    assert_eq!(
        request.player_patches,
        vec![Java26PlayerSavePatch {
            // One tick of `ServerPlayer.doTick`'s counter arm
            // (`ServerPlayer.java:649-661`): the two unconditional time stats,
            // plus the two whose flags a standing, living, never-sleeping
            // session raises. `sneak_time` needs the shift flag and is absent.
            stats: {
                let mut stats = bbb_server_types::stats::ServerStatsCounter::new();
                for name in [
                    bbb_server_types::stats::CUSTOM_STAT_PLAY_TIME,
                    bbb_server_types::stats::CUSTOM_STAT_TOTAL_WORLD_TIME,
                    bbb_server_types::stats::CUSTOM_STAT_TIME_SINCE_DEATH,
                    bbb_server_types::stats::CUSTOM_STAT_TIME_SINCE_REST,
                ] {
                    stats.increment(bbb_server_types::stats::custom_stat(name), 1);
                }
                stats
            },
            uuid,
            position: [3.5, 70.0, 4.5],
            motion: [0.0; 3],
            rotation: [90.0, -30.0],
            on_ground: true,
            custom_name: None,
            custom_data: Vec::new(),
            tags: Vec::new(),
            root_vehicle: None,
            ender_pearls: Vec::new(),
            shoulder_entities: Default::default(),
            respawn: None,
            last_death_location: None,
            previous_game_mode: None,
            entered_nether_pos: None,
            raid_omen_position: None,
            // One `WardenSpawnTracker.tick` since the join.
            warden_spawn_tracker: Some(bbb_server_persistence::Java26WardenSpawnTracker::new(1, 0, 0)),
            // The state a document authored from scratch would need, which the
            // session carries whether or not the save already holds one.
            created: Java26CreatedPlayer {
                portal_cooldown: 0,
                sleep_timer: 0,
                sleeping_pos: None,
                last_hurt_by: Default::default(),
                silent: false,
                glowing: false,
                spawn_extra_particles_on_fall: true,
                seen_credits: false,
                has_visual_fire: false,
                fall_distance: 0.0,
                impulse_reset_grace_time: 0,
                impulse_impact_position: None,
                remaining_fire_ticks: -20,
                air_supply: 300,
                ticks_frozen: 0,
                dimension: "minecraft:overworld".to_string(),
                game_mode: GameType::Creative,
                entity_invulnerable: false,
                // `updatePlayerAttributes` (`ServerPlayer.java:610-627`)
                // materializes both ranges; their creative modifiers are
                // transient and stay out of the save.
                attributes: vec![
                    crate::entity::PackedAttribute {
                        attribute_id:
                            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
                        base_value: 4.5,
                        modifiers: Vec::new(),
                    }
                    .to_saved_nbt(),
                    crate::entity::PackedAttribute {
                        attribute_id:
                            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID,
                        base_value: 3.0,
                        modifiers: Vec::new(),
                    }
                    .to_saved_nbt(),
                    crate::entity::PackedAttribute {
                        attribute_id:
                            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                        base_value: f64::from(0.1_f32),
                        modifiers: Vec::new(),
                    }
                    .to_saved_nbt(),
                    crate::entity::PackedAttribute {
                        attribute_id:
                            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID,
                        base_value: 60_000_000.0,
                        modifiers: Vec::new(),
                    }
                    .to_saved_nbt(),
                ],
                active_effects: Vec::new(),
                locator_bar_icon: bbb_protocol::packets::WaypointIcon {
                    style: "minecraft:default".to_string(),
                    color_rgb: None,
                },
                damage: bbb_server_datapack::damage::PlayerDamageState::created(),
                food: bbb_server_types::food::FoodData::created(),
                // The restore read the document's own `XpP`/`XpLevel`/`XpTotal`,
                // `XpSeed` and `Score` (`Player.java:629-637`), which the fixture
                // carries, so the save writes them back rather than the field
                // initialisers.
                experience: bbb_server_types::experience::PlayerExperience::from_saved(0.5, 1, 7)
                    .with_enchantment_seed(4)
                    .with_score(9),
                abilities: PlayerAbilities {
                    invulnerable: true,
                    flying: false,
                    can_fly: true,
                    instabuild: true,
                    flying_speed: 0.05,
                    walking_speed: 0.1,
                },
                inventory: expected_inventory,
                ender_items: expected_ender_items,
                fall_flying: false,
                selected_item_slot: 1,
                recipe_book_settings: [false; 8],
                recipe_book_recipes: vec!["minecraft:stone".to_string()],
                recipe_book_to_be_displayed: vec!["minecraft:stone".to_string()],
            },
        }]
    );
    assert!(!request
        .chunk_writes
        .iter()
        .any(|write| (write.chunk_x, write.chunk_z) == (0, 0)));
}

#[test]
fn saved_chunk_patches_retire_without_losing_changes_made_during_a_save() {
    let root = std::env::temp_dir().join(format!("bbb-chunk-dirty-ack-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-chunk-dirty-ack");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let mut world = ServerWorld::restore(operator_config(&[("op", 1)]), &candidate).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let dimension = Java26BuiltinDimension::Overworld;
    let ground_pos = BlockPos { x: 0, y: 63, z: 0 };
    let ground = world.block_state(dimension, ground_pos);
    let marker = BlockPos { x: 1, y: 100, z: 1 };
    let late_marker = BlockPos { x: 2, y: 100, z: 1 };

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 1 100 1 minecraft:diamond_block",
    );
    let frozen = world.save_request(1, 101, RegionCompression::Zlib).unwrap();
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 1 100 1 minecraft:gold_block",
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 2 100 1 minecraft:emerald_block",
    );
    storage.save_snapshot(&candidate, &frozen).unwrap();
    world.acknowledge_detached_save(&frozen);

    let latest = world.save_request(2, 102, RegionCompression::Zlib).unwrap();
    storage.save_snapshot(&candidate, &latest).unwrap();
    world.acknowledge_save_success(&latest);
    let region = paths.region_dir().join("r.0.0.mca");
    let saved_bytes = std::fs::read(&region).unwrap();
    let quiet = world.save_request(3, 103, RegionCompression::Zlib).unwrap();
    storage.save_snapshot(&candidate, &quiet).unwrap();
    world.acknowledge_save_success(&quiet);
    assert_eq!(std::fs::read(&region).unwrap(), saved_bytes);

    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let mut restarted = ServerWorld::restore(operator_config(&[("op", 1)]), &candidate).unwrap();
    assert_eq!(
        restarted.block_state(dimension, marker),
        Some(block_state_id("minecraft:gold_block"))
    );
    assert_eq!(
        restarted.block_state(dimension, late_marker),
        Some(block_state_id("minecraft:emerald_block"))
    );
    assert_eq!(restarted.block_state(dimension, ground_pos), ground);

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 1 100 1 minecraft:air",
    );
    let reverted = world.save_request(4, 104, RegionCompression::Zlib).unwrap();
    storage.save_snapshot(&candidate, &reverted).unwrap();
    world.acknowledge_save_success(&reverted);
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let mut restarted = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    assert_eq!(
        restarted.block_state(dimension, marker),
        Some(block_state_id("minecraft:air"))
    );
    assert_eq!(
        restarted.block_state(dimension, late_marker),
        Some(block_state_id("minecraft:emerald_block"))
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn restored_inventory_reaches_join_without_fixture_items() {
    let uuid = Uuid::from_u128(0x1a2b);
    let player = restore_fixture_player(uuid, [3.5, 70.0, 4.5], [90.0, -30.0], true);
    let mut raw = player.into_raw();
    raw.entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "Inventory")
        .unwrap()
        .value = NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
        nbt("Slot", NbtSummaryValue::Byte(35)),
        nbt("id", NbtSummaryValue::string("minecraft:diamond_sword")),
        nbt("count", NbtSummaryValue::Int(1)),
        nbt(
            "components",
            NbtSummaryValue::Compound(vec![nbt(
                "minecraft:enchantments",
                NbtSummaryValue::Compound(vec![nbt(
                    "minecraft:sharpness",
                    NbtSummaryValue::Int(5),
                )]),
            )]),
        ),
    ])]);
    let player = Java26PlayerData::parse(raw, uuid).unwrap();
    let candidate = restore_candidate(4_200, None, vec![(uuid, player)]);
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(7),
            profile: profile("alice", 0x1a2b),
        },
    )
    .unwrap();
    let joined = world.tick(receiver.drain());
    let slots = &joined.snapshot.players[0].inventory.slots;
    assert_eq!(slots[0], ItemStackSummary::empty());
    assert_eq!(
        slots[35].item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_SWORD_ID)
    );
    assert_eq!(slots[35].component_patch.enchantments.len(), 1);
    assert_eq!(slots[35].component_patch.enchantments[0].level, 5);
}

/// The save request has to carry a player the save has never held, because the
/// storage authors that player's first document out of it. A restored world with
/// no playerdata at all is exactly that case.
#[test]
fn save_request_carries_a_first_time_players_whole_state() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(
        creative_fixture_config(),
        &restore_candidate(0, None, vec![]),
    )
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 9, "newcomer", 9);
    play(
        &sender,
        &receiver,
        &mut world,
        9,
        PlayServerbound::RecipeBookChangeSettings(RecipeBookChangeSettingsCommand {
            book_type: RecipeBookType::Furnace,
            open: true,
            filtering: true,
        }),
    );

    let request = world
        .save_request(1_786_000_002_000, 202, RegionCompression::Zlib)
        .unwrap();
    let patch = &request.player_patches[0];
    assert_eq!(patch.uuid, profile("newcomer", 9).uuid);
    // The world's own game type, not a second setting that could disagree.
    assert_eq!(
        patch.created.game_mode.id(),
        ServerWorld::fixture_game_type_id()
    );
    assert_eq!(patch.created.dimension, "minecraft:overworld");
    assert!(patch.created.abilities.can_fly && patch.created.abilities.instabuild);
    // The furnace pair is book type ordinal 1, so it lands in flags 2 and 3.
    assert_eq!(
        patch.created.recipe_book_settings,
        [false, false, true, true, false, false, false, false]
    );

    // And the document the storage would author out of it is one the reader
    // takes back.
    let document = bbb_server_persistence::created_player_document(
        patch,
        request.item_stack_resolver.as_dyn(),
    )
    .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(document, patch.uuid).unwrap();
    assert_eq!(parsed.position(), patch.position);
    assert_eq!(parsed.rotation(), patch.rotation);
    assert_eq!(parsed.game_mode(), Some(patch.created.game_mode));
    assert_eq!(
        parsed.recipe_book().settings(),
        patch.created.recipe_book_settings
    );
    assert!(parsed.recipe_book().recipes().is_empty());
    assert!(parsed.recipe_book().to_be_displayed().is_empty());
}

/// `PlayerEnderChestContainer.storeAsSlots`
/// (`PlayerEnderChestContainer.java:40-47`) writes the non-empty slots into
/// `EnderItems` (`Player.java:659`), and `fromSlots` (`:28-38`, read at
/// `Player.java:641`) takes them back.
#[test]
fn the_ender_chest_round_trips_through_playerdata() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(
        creative_fixture_config(),
        &restore_candidate(0, None, vec![]),
    )
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 9, "newcomer", 9);
    let stored = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID),
        count: 5,
        component_patch: Default::default(),
    };
    let first_slot = ItemStackSummary {
        count: 2,
        ..stored.clone()
    };
    world.set_ender_chest_slot_for_test(ConnectionId(9), 0, first_slot.clone());
    world.set_ender_chest_slot_for_test(ConnectionId(9), 26, stored.clone());

    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(9),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    join_loaded(&sender, &receiver, &mut world, 10, "newcomer", 9);
    assert_eq!(
        world.ender_chest_slot_for_test(ConnectionId(10), 26),
        stored
    );
    assert_eq!(
        world.ender_chest_slot_for_test(ConnectionId(10), 0),
        first_slot
    );

    let request = world
        .save_request(1_786_000_002_000, 202, RegionCompression::Zlib)
        .unwrap();
    let patch = &request.player_patches[0];
    assert_eq!(patch.created.ender_items.runtime_slots().len(), 27);
    let mut expected = vec![ItemStackSummary::empty(); 27];
    expected[0] = first_slot.clone();
    expected[26] = stored.clone();
    assert_eq!(
        patch
            .created
            .ender_items
            .runtime_slots()
            .iter()
            .map(|stack| stack.summary().clone())
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        patch.created.ender_items.runtime_slots()[26].summary(),
        &stored
    );

    let document = bbb_server_persistence::created_player_document(
        patch,
        request.item_stack_resolver.as_dyn(),
    )
    .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(document, patch.uuid).unwrap();
    let Some(NbtSummaryValue::List(items)) = parsed.ender_items_input() else {
        panic!("saved ender list");
    };
    assert_eq!(items.len(), 2);
    assert_eq!(
        bbb_protocol::nbt_summary::value_field(&items[0], "Slot"),
        Some(&NbtSummaryValue::Byte(0))
    );
    assert_eq!(
        bbb_protocol::nbt_summary::value_field(&items[1], "Slot"),
        Some(&NbtSummaryValue::Byte(26))
    );

    // And a world restored from that document hands the stack back to the
    // returning session.
    let restored = ServerWorld::restore(
        creative_fixture_config(),
        &restore_candidate(0, None, vec![(patch.uuid, parsed)]),
    );
    let mut restored = restored.unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut restored, 9, "newcomer", 9);
    assert_eq!(
        restored.ender_chest_slot_for_test(ConnectionId(9), 26),
        stored
    );
    assert_eq!(
        restored.ender_chest_slot_for_test(ConnectionId(9), 0),
        first_slot
    );
}

#[test]
fn awarded_recipe_survives_playerdata_save_and_restore() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(
        creative_fixture_config(),
        &restore_candidate(0, None, vec![]),
    )
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 9, "newcomer", 9);
    play(
        &sender,
        &receiver,
        &mut world,
        9,
        PlayServerbound::RecipeBookChangeSettings(RecipeBookChangeSettingsCommand {
            book_type: RecipeBookType::Furnace,
            open: true,
            filtering: true,
        }),
    );
    world.award_player_recipe_for_test(ConnectionId(9), "minecraft:stone");
    assert!(world.player_knows_recipe_for_test(ConnectionId(9), "minecraft:stone"));

    let request = world
        .save_request(1_786_000_002_000, 202, RegionCompression::Zlib)
        .unwrap();
    let patch = &request.player_patches[0];
    assert_eq!(patch.created.recipe_book_recipes, ["minecraft:stone"]);
    assert_eq!(
        patch.created.recipe_book_to_be_displayed,
        ["minecraft:stone"]
    );
    assert_eq!(
        patch.created.recipe_book_settings,
        [false, false, true, true, false, false, false, false]
    );

    let document = bbb_server_persistence::created_player_document(
        patch,
        request.item_stack_resolver.as_dyn(),
    )
    .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(document, patch.uuid).unwrap();
    assert_eq!(parsed.recipe_book().recipes(), &["minecraft:stone"]);
    assert_eq!(parsed.recipe_book().to_be_displayed(), &["minecraft:stone"]);
    assert_eq!(
        parsed.recipe_book().settings(),
        patch.created.recipe_book_settings
    );

    let uuid = patch.uuid;
    let candidate = restore_candidate(0, None, vec![(uuid, parsed)]);
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    join_loaded(&sender, &receiver, &mut world, 9, "newcomer", 9);
    assert!(world.player_knows_recipe_for_test(ConnectionId(9), "minecraft:stone"));
    assert_eq!(
        world.player_recipe_book_settings(ConnectionId(9)),
        [
            RecipeBookTypeSettings::default(),
            RecipeBookTypeSettings {
                open: true,
                filtering: true,
            },
            RecipeBookTypeSettings::default(),
            RecipeBookTypeSettings::default(),
        ]
    );
}

/// The whole chain the missing creation path broke, end to end and through the
/// filesystem: a save this server created, a player it has never held who joins
/// and moves, a save taken while that player is *online*, and a restart that
/// still has the player where they were and the block they placed.
///
/// Before the fix the save in the middle failed outright, so nothing here
/// survived — not the player, not the block, not the clock.
#[test]
fn a_first_time_player_and_their_block_survive_a_save_and_a_restart() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-first-join-restart-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let paths = WorldSavePaths::new(&root).unwrap();
    let fixture = FlatFixture::new().unwrap();

    // The runtime's own startup path: an empty directory becomes a save.
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let column = default_overworld_level_source()
        .unwrap()
        .generate_chunk(0, 0, fixture.biome_id())
        .into_column();
    let created = created_save(
        &Java26WorldCreation {
            level_name: "bbb-first-join".to_string(),
            game_type: ServerWorld::fixture_game_type_id(),
            difficulty: Difficulty::Easy.serialized_name().to_string(),
            spawn_position: ServerWorld::fixture_spawn_block(),
            last_played: 1_786_000_000_000,
        },
        &fixture,
        &[(0, 0, &column)],
    )
    .unwrap();
    storage
        .create_world(&created, RegionCompression::Zlib, 100)
        .unwrap();
    let candidate = storage.load_candidate().unwrap();
    assert!(
        candidate.players.is_empty() && candidate.stored_players.is_empty(),
        "a created save holds nobody's playerdata"
    );

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "newcomer", 1);
    let moved = Vec3d {
        x: 5.5,
        y: 65.0,
        z: 6.5,
    };
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        moved,
        Some(PlayerRotation {
            y_rot: 90.0,
            x_rot: -30.0,
        }),
    );
    let placed_tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: BlockPos { x: 5, y: 63, z: 6 },
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: 1,
        }),
    );
    let placed = *placed_tick
        .snapshot
        .changed_blocks
        .first()
        .expect("the fixture stack places a block above the floor");
    let saved_item = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_DIRT_ID),
        count: 7,
        component_patch: Default::default(),
    };
    world.set_inventory_slot_for_test(ConnectionId(1), 5, saved_item.clone());
    world.set_player_fall_flying_for_test(ConnectionId(1), true);
    let moved =
        super::player_fall_flying::wear_an_elytra_in_the_air(&sender, &receiver, &mut world, 1);
    let locator_bar_icon = bbb_protocol::packets::WaypointIcon {
        style: "minecraft:target_x".to_string(),
        color_rgb: Some(0x12_34_56),
    };
    world.set_player_waypoint_icon_for_test(ConnectionId(1), locator_bar_icon.clone());

    // The save that used to fail: the player is still connected.
    let request = world
        .save_request(1_786_000_009_999, 300, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(request.player_patches.len(), 1);
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    // Restart on the same directory.
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let uuid = profile("newcomer", 1).uuid;
    assert!(reloaded.players.is_empty());
    let (_, saved) = reloaded
        .load_player(uuid)
        .unwrap()
        .expect("the newcomer's first save created their playerdata");
    assert_eq!(saved.position(), [moved.x, moved.y, moved.z]);
    assert_eq!(saved.rotation(), [90.0, -30.0]);
    assert_eq!(saved.dimension(), "minecraft:overworld");
    assert!(saved.fall_flying());
    let Some(NbtSummaryValue::List(items)) = saved.inventory_input() else {
        panic!("saved inventory list");
    };
    assert_eq!(
        bbb_protocol::nbt_summary::value_field(&items[0], "Slot"),
        Some(&NbtSummaryValue::Byte(0))
    );
    assert_eq!(
        bbb_protocol::nbt_summary::value_field(&items[1], "Slot"),
        Some(&NbtSummaryValue::Byte(5))
    );
    assert_eq!(saved.locator_bar_icon(), &locator_bar_icon);

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(creative_fixture_config(), &reloaded).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "newcomer", 1);
    let restarted = world.tick(Vec::new());
    // The block came back with the world, not with the player.
    assert!(restarted.snapshot.changed_blocks.contains(&placed));
    let rejoined = restarted
        .snapshot
        .players
        .iter()
        .find(|player| player.uuid == uuid)
        .expect("the newcomer rejoined")
        .clone();
    assert_eq!(rejoined.position, moved);
    assert_eq!(rejoined.inventory.slots[5], saved_item);
    assert_eq!(
        world.player_waypoint_icon_for_test(ConnectionId(1)),
        &locator_bar_icon
    );
    assert_eq!(
        rejoined.rotation,
        PlayerRotation {
            y_rot: 90.0,
            x_rot: -30.0
        }
    );
    let restarted_save = world
        .save_request(1_786_000_010_000, 301, RegionCompression::Zlib)
        .unwrap();
    assert!(restarted_save.player_patches[0].created.fall_flying);
    assert_eq!(
        restarted_save.player_patches[0].created.locator_bar_icon,
        locator_bar_icon
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// A block placed in a column the save has never held.
///
/// The created save publishes the spawn column and leaves the rest to the
/// generator it declares, so the first edit two chunks out has no document to
/// patch. It is written entire instead — the same shape `create_world` writes —
/// and the reader takes it back.
#[test]
fn a_block_placed_beyond_the_saved_columns_is_written_entire_and_reloads() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-generated-chunk-write-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let paths = WorldSavePaths::new(&root).unwrap();
    let fixture = FlatFixture::new().unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let column = default_overworld_level_source()
        .unwrap()
        .generate_chunk(0, 0, fixture.biome_id())
        .into_column();
    let created = created_save(
        &Java26WorldCreation {
            level_name: "bbb-generated-chunk".to_string(),
            game_type: ServerWorld::fixture_game_type_id(),
            difficulty: Difficulty::Easy.serialized_name().to_string(),
            spawn_position: ServerWorld::fixture_spawn_block(),
            last_played: 1_786_000_000_000,
        },
        &fixture,
        &[(0, 0, &column)],
    )
    .unwrap();
    storage
        .create_world(&created, RegionCompression::Zlib, 100)
        .unwrap();
    let candidate = crate::tests::read_stored_candidate(storage.load_candidate().unwrap());
    assert_eq!(
        candidate.dimensions[&level_key(Java26BuiltinDimension::Overworld)]
            .chunks
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![(0, 0)],
        "a created save publishes the spawn column and nothing else"
    );

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "walker", 1);
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 33.5,
            y: 65.0,
            z: 0.5,
        },
        None,
    );
    let placed_tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: BlockPos { x: 33, y: 63, z: 0 },
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: 1,
        }),
    );
    let placed = BlockPos { x: 33, y: 64, z: 0 };
    assert!(placed_tick
        .snapshot
        .changed_blocks
        .iter()
        .any(|block| block.pos == placed));

    let request = world
        .save_request(1_786_000_009_999, 300, RegionCompression::Zlib)
        .unwrap();
    assert!(
        !request
            .chunk_writes
            .iter()
            .any(|write| (write.chunk_x, write.chunk_z) == (0, 0)),
        "the spawn column was never touched, so it is not rewritten"
    );
    // Every column generated around the player is unsaved, the edited one included.
    assert!(request
        .chunk_writes
        .iter()
        .any(|write| (write.chunk_x, write.chunk_z) == (2, 0)));
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = crate::tests::read_stored_candidate(storage.load_candidate().unwrap());
    let written = reloaded.dimensions[&level_key(Java26BuiltinDimension::Overworld)]
        .chunks
        .get(&(2, 0))
        .expect("the generated column reached the region file");
    assert_eq!(written.data.last_update(), world.game_time());
    assert_eq!(written.data.status().as_str(), "minecraft:full");
    let section = written
        .data
        .sections()
        .iter()
        .find(|section| section.y == 4)
        .expect("the section holding y = 64");
    assert_eq!(
        section
            .block_states
            .as_ref()
            .unwrap()
            .value_at(1, 0, 0)
            .unwrap()
            .name,
        "minecraft:stone"
    );

    // And the restarted world reads it back as its own edit again.
    let restarted = ServerWorld::restore(creative_fixture_config(), &reloaded).unwrap();
    assert!(restarted
        .publication()
        .changed_blocks
        .iter()
        .any(|block| block.pos == placed));
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// One autosave request out of `ticks`, or none, with the tick it arrived on.
fn autosave_tick(world: &mut ServerWorld, ticks: i32) -> Option<u64> {
    let (_sender, receiver) = mailbox();
    let mut arrived = None;
    for _ in 0..ticks {
        let tick = world.tick(receiver.drain());
        for event in &tick.events {
            if let WorldEvent::SaveRequested {
                trigger: SaveTrigger::Autosave,
                flush,
            } = event
            {
                // `autoSave` calls `saveEverything(true, false, false)`
                // (MinecraftServer.java:1018), so the unattended save never
                // asks for a flush.
                assert!(!flush, "the unattended save is the unflushed one");
                assert!(arrived.is_none(), "one interval asks for one save");
                arrived = Some(tick.tick);
            }
        }
    }
    arrived
}

#[test]
fn the_unattended_save_arrives_one_vanilla_interval_after_startup() {
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    // `ticksUntilAutosave` starts at `AUTOSAVE_INTERVAL`
    // (MinecraftServer.java:243), so the first save is a full 6000 ticks in
    // and not one tick earlier.
    assert_eq!(world.ticks_until_autosave(), 6000);
    assert_eq!(autosave_tick(&mut world, 5999), None);
    assert_eq!(world.ticks_until_autosave(), 1);
    assert_eq!(autosave_tick(&mut world, 1), Some(6000));
}

#[test]
fn the_unattended_interval_restarts_at_the_computed_value() {
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    assert_eq!(autosave_tick(&mut world, 6000), Some(6000));
    // `autoSave` resets from `computeNextAutosaveInterval`
    // (MinecraftServer.java:1013), which is the 20 tps rate over the intended
    // 300 seconds, above the 100-tick floor.
    assert_eq!(world.ticks_until_autosave(), 6000);
    assert_eq!(autosave_tick(&mut world, 6000), Some(12_000));
}

#[test]
fn slowing_the_tick_rate_pulls_the_unattended_save_forward() {
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    world.tick(Vec::new());
    assert_eq!(world.ticks_until_autosave(), 5999);
    // `computeNextAutosaveInterval` reads the live rate
    // (MinecraftServer.java:1034), so one tick per second over the intended 300
    // seconds is 300 ticks, and `onTickRateChanged` (`:1041-1046`) shortens the
    // countdown to it because it is the smaller of the two.
    world.set_tick_rate(1.0, &mut Vec::new());
    assert_eq!(world.ticks_until_autosave(), 300);
    // A rate change the other way never pushes the save back out: the
    // comparison at `:1043` is one-sided.
    world.set_tick_rate(20.0, &mut Vec::new());
    assert_eq!(world.ticks_until_autosave(), 300);
    // Every session hears the new rate, because `setTickRate` broadcasts
    // (ServerTickRateManager.java:115-119).
    assert_eq!(world.tick_rate().tickrate(), 20.0);
}

#[test]
fn an_operator_save_does_not_postpone_the_unattended_one() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, "save", AdminScope::Manage),
            request: AdminRequest::Save { flush: true },
        },
    )
    .unwrap();
    let tick = world.tick(receiver.drain());
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::SaveRequested {
            trigger: SaveTrigger::Management,
            ..
        }
    )));
    // `saveEverything` never touches `ticksUntilAutosave`; only `autoSave`
    // resets it (MinecraftServer.java:1013), so the operator's save leaves the
    // unattended schedule exactly where the tick count put it.
    assert_eq!(world.ticks_until_autosave(), 5999);
}

#[test]
fn a_pending_tick_survives_the_save_request() {
    let sand = ([1_usize, 2_usize, 2_usize], "minecraft:sand", &[][..]);
    let candidate = restore_candidate(
        10,
        Some(restore_chunk_with_ticks(
            vec![restore_section_with_cells(4, "minecraft:air", &[sand])],
            // Far enough out that no tick runs before the save is taken.
            vec![restore_saved_tick("minecraft:sand", [1, 66, 2], 400, -1)],
            Vec::new(),
        )),
        Vec::new(),
    );
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    let (_sender, receiver) = mailbox();
    world.tick(receiver.drain());
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let write = request
        .chunk_writes
        .iter()
        .find(|write| (write.chunk_x, write.chunk_z) == (0, 0))
        .expect("the scenario chunk is saved");
    let parsed = Java26ChunkData::parse(&write.document, restore_chunk_context()).unwrap();
    // `LevelChunkTicks.pack` re-derives the delay against the game time the
    // save is taken at, so one elapsed tick shortens it by one.
    assert_eq!(
        parsed.block_ticks(),
        &[bbb_server_types::chunk_ticks::Java26SavedTick {
            id: "minecraft:sand".to_string(),
            x: 1,
            y: 66,
            z: 2,
            delay: 399,
            priority: -1,
        }]
    );
    assert!(parsed.fluid_ticks().is_empty());
}

#[test]
fn the_saves_game_rules_reach_the_rule_table() {
    let mut candidate = restore_candidate(10, None, Vec::new());
    candidate.game_rules.insert(
        "minecraft:random_tick_speed".to_string(),
        bbb_protocol::game_rules::GameRuleValue::Integer(0),
    );
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, "rules", AdminScope::Read),
            request: AdminRequest::GameRules,
        },
    )
    .unwrap();
    let result = world.tick(receiver.drain());
    let Some(WorldEvent::Admin(outcome)) = result
        .events
        .into_iter()
        .find(|event| matches!(event, WorldEvent::Admin(_)))
    else {
        panic!("expected the administration answer")
    };
    let Ok(AdminResult::GameRules(reports)) = outcome.result else {
        panic!("expected the game-rule readout")
    };
    let report = reports
        .iter()
        .find(|report| report.key == "minecraft:random_tick_speed")
        .expect("the rule is enabled");
    assert_eq!(
        report.value,
        bbb_protocol::game_rules::GameRuleValue::Integer(0)
    );
}

#[test]
fn an_entity_in_flight_reaches_the_save_request() {
    let sand = ([1_usize, 2_usize, 2_usize], "minecraft:sand", &[][..]);
    let candidate = restore_candidate(
        10,
        Some(restore_chunk_with_ticks(
            vec![restore_section_with_cells(4, "minecraft:air", &[sand])],
            vec![restore_saved_tick("minecraft:sand", [1, 66, 2], 0, 0)],
            Vec::new(),
        )),
        Vec::new(),
    );
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    let (_sender, receiver) = mailbox();
    // One tick to run the scheduled tick that creates the entity, one more so
    // it is still falling rather than landed.
    world.tick(receiver.drain());
    world.tick(receiver.drain());

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let write = request
        .entity_writes
        .iter()
        .find(|write| (write.chunk_x, write.chunk_z) == (0, 0))
        .expect("the entity chunk is written");
    let data = Java26EntityChunkData::parse(write.document.clone(), 0, 0).unwrap();
    let records = data.entities().collect::<Vec<_>>();
    assert_eq!(records.len(), 1, "{records:?}");
    assert_eq!(records[0].id(), "minecraft:falling_block");
}

/// A save this server wrote has to carry the settings a command moved.
///
/// `MinecraftServer.saveAllChunks` does two writes for these three values:
/// `storageSource.saveDataTag` re-encodes `PrimaryLevelData` into `level.dat`
/// (`MinecraftServer.java:599-600`), which is where
/// `PrimaryLevelData.setDifficulty` (`PrimaryLevelData.java:250-252`) and
/// `setDifficultyLocked` (`:260-262`) landed, and `savedDataStorage` flushes
/// the dirty `GameRuleMap` into `data/minecraft/game_rules.dat`
/// (`MinecraftServer.java:601-605`, `SavedDataStorage.java:188-215`), which is
/// where `GameRules.set` marked it (`GameRuleMap.java:56-59`).
///
/// The difficulty is asserted on the reloaded document rather than on the
/// restarted world, and that is the vanilla claim rather than a weaker one: a
/// dedicated server re-applies `server.properties` over the level's difficulty
/// at every startup (`DedicatedServer.forceDifficulty`,
/// `DedicatedServer.java:395-397`), so the value a restart *runs* is the
/// property's while the value the save *holds* is the command's. The lock has
/// no such override — `forceDifficulty` passes `ignoreLock = true` rather than
/// clearing it — so the lock is asserted through the restart itself.
#[test]
fn a_difficulty_a_lock_and_a_game_rule_typed_into_chat_survive_a_save_and_a_restart() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-settings-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-settings");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    assert_eq!(candidate.level.difficulty_settings().difficulty(), "easy");
    assert!(!candidate.level.difficulty_settings().locked());
    assert_eq!(
        candidate.game_rules.get("minecraft:random_tick_speed"),
        None,
        "a created save writes no game_rules.dat, so every rule is at its default"
    );

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(operator_config(&[("op", 1)]), &candidate).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    chat_command(&sender, &receiver, &mut world, 1, "difficulty hard");
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule random_tick_speed 7",
    );
    // The lock has no command of its own; it is the packet handler's, and
    // `MinecraftServer.setDifficultyLocked` (MinecraftServer.java:1306-1309)
    // has no gate in front of it.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::LockDifficulty { locked: true },
    );

    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    assert_eq!(
        reloaded.level.difficulty_settings().difficulty(),
        "hard",
        "the save has to hold what /difficulty set"
    );
    assert!(
        reloaded.level.difficulty_settings().locked(),
        "the lock is level data too"
    );
    assert!(
        !reloaded.level.difficulty_settings().hardcore(),
        "nothing in the tree writes hardcore, so it may not move"
    );
    assert_eq!(
        reloaded.game_rules.get("minecraft:random_tick_speed"),
        Some(&bbb_protocol::game_rules::GameRuleValue::Integer(7)),
        "the rule the command wrote has to come back out of game_rules.dat"
    );
    assert_eq!(
        reloaded.game_rules.get("minecraft:send_command_feedback"),
        Some(&bbb_protocol::game_rules::GameRuleValue::Boolean(true)),
        "the written map is every enabled rule, not only the ones that moved"
    );

    // The restart, read through what a client is actually told: the difficulty
    // comes back from the property the way `forceDifficulty` applies it, the
    // lock comes back from the save, and the rule answers its own query.
    let mut restarted = ServerWorld::restore(operator_config(&[("op", 1)]), &reloaded).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut restarted, 1, "op", 1);
    let announced = play(
        &sender,
        &receiver,
        &mut restarted,
        1,
        PlayServerbound::LockDifficulty { locked: true },
    )
    .events
    .iter()
    .find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ChangeDifficulty(update),
            ..
        } => Some(*update),
        _ => None,
    })
    .expect("the lock write re-announces the difficulty");
    assert_eq!(
        announced.difficulty,
        creative_fixture_config().difficulty,
        "forceDifficulty puts the property back over the saved difficulty"
    );
    assert!(announced.locked, "the lock survived the restart");
    let queried = chat_command(
        &sender,
        &receiver,
        &mut restarted,
        1,
        "gamerule random_tick_speed",
    );
    assert_eq!(
        system_chats(&queried)
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>(),
        vec!["commands.gamerule.query random_tick_speed7"]
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// Rewrites the `Data` compound of a closed save's `level.dat`.
///
/// `difficulty_settings.hardcore` has no writer on a running server:
/// `Main.createNewWorldData` reads `properties.hardcore` once, when the level is
/// created (`Main.java:242-248`), and nothing in the tree moves the flag
/// afterwards. A save that carries anything but this server's own created
/// default therefore has to be produced the way another server would have
/// written it.
fn patch_level_data(root: &std::path::Path, patch: impl FnOnce(&mut Vec<NbtSummaryEntry>)) {
    let store = LevelDatStore::new(
        WorldSavePaths::new(root).unwrap(),
        WorldSaveLimits::default(),
    )
    .unwrap();
    let mut document = store.load().unwrap().document;
    let data = document
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "Data")
        .expect("level.dat carries a Data compound");
    let NbtSummaryValue::Compound(entries) = &mut data.value else {
        panic!("level.dat Data has to be a compound");
    };
    patch(entries);
    store.save(&document).unwrap();
}

/// Replaces the value of one entry that has to be there already.
fn replace_nbt_entry(entries: &mut [NbtSummaryEntry], name: &str, value: NbtSummaryValue) {
    let entry = entries
        .iter_mut()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("the compound carries {name}"));
    entry.value = value;
}

/// A hardcore save runs at hard and is saved at hard, whatever the property
/// asked for.
///
/// `MinecraftServer.setDifficulty` stores
/// `this.worldData.isHardcore() ? Difficulty.HARD : difficulty`
/// (MinecraftServer.java:1288-1294), and that method is the single funnel every
/// writer of the level's difficulty reaches: the dedicated server's startup
/// force (`DedicatedServer.forceDifficulty`, DedicatedServer.java:395-397,
/// called from `MinecraftServer.loadLevel`, MinecraftServer.java:394-402), the
/// gamemaster packet, and `/difficulty` (DifficultyCommand.java:32-41, which
/// reports success and lets `setDifficulty` decide what is stored).
///
/// The flag itself comes from the save and only from the save: `properties.hardcore`
/// is read once, when a level is created (`Main.java:242-248`), and nothing in
/// the 26.1 tree writes `LevelSettings.DifficultySettings.hardcore` afterwards --
/// `withDifficulty` and `withDifficultyLock` (LevelSettings.java:26-44) both
/// carry it through unchanged.
#[test]
fn a_hardcore_save_runs_and_is_saved_at_hard_whatever_the_difficulty_property_says() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-hardcore-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-hardcore");
    let paths = storage.paths().clone();
    storage.close().unwrap();
    patch_level_data(&root, |data| {
        let settings = data
            .iter_mut()
            .find(|entry| entry.name == "difficulty_settings")
            .expect("a created level carries difficulty_settings");
        let NbtSummaryValue::Compound(settings) = &mut settings.value else {
            panic!("difficulty_settings has to be a compound");
        };
        replace_nbt_entry(settings, "hardcore", NbtSummaryValue::Byte(1));
    });

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let candidate = storage.load_candidate().unwrap();
    assert!(candidate.level.difficulty_settings().hardcore());
    assert_eq!(
        candidate.level.difficulty_settings().difficulty(),
        "easy",
        "the document still says easy; the coercion is the server's, not the file's"
    );
    let config = operator_config(&[("op", 1)]);
    assert_eq!(
        config.difficulty,
        Difficulty::Easy,
        "the property this server starts from is the one a hardcore world overrides"
    );

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(config, &candidate).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    // The lock write re-announces the difficulty without moving it, which is how
    // a test reads what the restore actually settled on.
    let announced = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::LockDifficulty { locked: false },
    )
    .events
    .iter()
    .find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ChangeDifficulty(update),
            ..
        } => Some(*update),
        _ => None,
    })
    .expect("the lock write announces the difficulty");
    assert_eq!(
        announced.difficulty,
        Difficulty::Hard,
        "forceDifficulty runs through setDifficulty, which coerces a hardcore world to hard"
    );

    let after_command = chat_command(&sender, &receiver, &mut world, 1, "difficulty peaceful");
    let still = after_command
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ChangeDifficulty(update),
                ..
            } => Some(*update),
            _ => None,
        })
        .expect("the command announces the difficulty it stored");
    assert_eq!(
        still.difficulty,
        Difficulty::Hard,
        "`/difficulty` reports success and `setDifficulty` still stores hard"
    );

    let request = world
        .save_request(1_786_000_002_000, 202, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    assert_eq!(
        reloaded.level.difficulty_settings().difficulty(),
        "hard",
        "what landed in worldData is what the next level.dat write records"
    );
    assert!(
        reloaded.level.difficulty_settings().hardcore(),
        "the flag itself has no writer, so a save may not lose it"
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// A game rule the file carries that this server's feature set does not enable
/// is dropped, from the running world and from the next `game_rules.dat` alike.
///
/// The server builds its map as
/// `new GameRules(this.worldData.enabledFeatures(), savedDataStorage.computeIfAbsent(GameRuleMap.TYPE))`
/// (MinecraftServer.java:345), and that constructor `reset`s every enabled rule
/// the map does not carry and `remove`s every rule the features do not enable
/// (`gamerules/GameRules.java:99-110`). The removal reaches the file too,
/// because `SavedDataStorage` re-encodes the map the server now holds
/// (SavedDataStorage.java:198-204) rather than patching the document it read.
/// Nothing is reported: `GameRuleMap.remove` is a bare map write
/// (`GameRuleMap.java:65-68`).
///
/// `minecraft:max_minecart_speed` is the one 26.1 rule with a feature gate,
/// `FeatureFlagSet.of(FeatureFlags.MINECART_IMPROVEMENTS)`
/// (`gamerules/GameRules.java:55-57`), and this server's feature set is the
/// vanilla flag alone -- `FeatureFlags.DEFAULT_FLAGS` (FeatureFlags.java:32-42),
/// which is also the only `enabled_features` a save this server opens may
/// declare. So the rule can reach the file (another server wrote it with the
/// feature on) but never the map.
#[test]
fn a_game_rule_this_servers_features_do_not_enable_is_dropped_from_the_world_and_the_next_save() {
    const RULE: &str = "minecraft:max_minecart_speed";
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-feature-rules-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-feature-rules");
    let mut candidate = storage.load_candidate().unwrap();
    // What a `game_rules.dat` written by a server with `minecart_improvements`
    // enabled leaves behind; the reader takes it verbatim, which
    // `the_game_rule_document_round_trips_and_refuses_what_the_reader_would`
    // pins on the file itself.
    candidate.game_rules.insert(
        RULE.to_string(),
        bbb_protocol::game_rules::GameRuleValue::Integer(200),
    );
    candidate.game_rules.insert(
        "minecraft:random_tick_speed".to_string(),
        bbb_protocol::game_rules::GameRuleValue::Integer(7),
    );

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(operator_config(&[("op", 1)]), &candidate).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    // The removed rule cannot be typed back in either: `GameRuleCommand.register`
    // builds its children from `new GameRules(context.enabledFeatures())`
    // (GameRuleCommand.java:15-26), so a rule behind an experiment has no node
    // at all -- which `a_game_rule_behind_an_experiment_has_no_node_at_all`
    // pins on the tree itself. The command therefore fails to parse rather than
    // reaching any writer, and the tick sends only that refusal.
    let typed_back = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule minecraft:max_minecart_speed 500",
    );
    assert_eq!(
        system_chats(&typed_back),
        vec![
            (ConnectionId(1), "command.unknown.argument".to_string()),
            (
                ConnectionId(1),
                "gamerule minecraft:max_minecart_speed 500command.context.here".to_string()
            ),
        ],
        "an argument with no node is Brigadier's `dispatcherUnknownArgument`, reported with the \
         context line the sender is shown"
    );

    let request = world
        .save_request(1_786_000_003_000, 203, RegionCompression::Zlib)
        .unwrap();
    assert!(
        !request.game_rules.contains_key(RULE),
        "neither the file's value nor the command's reaches the next game_rules.dat: vanilla \
         re-encodes the map it holds, and the rule was removed from it"
    );
    assert_eq!(
        request.game_rules.get("minecraft:random_tick_speed"),
        Some(&bbb_protocol::game_rules::GameRuleValue::Integer(7)),
        "only the feature-gated rule goes; the rest of the file is restored and written back"
    );
    assert!(
        request
            .game_rules
            .contains_key("minecraft:max_snow_accumulation_height"),
        "and every enabled rule the file left out is written at its default"
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// `PlayerList.getPlayerAdvancements` reads `players/advancements/<uuid>.json`
/// at a join (`PlayerList.java:788-799`) and `PlayerList.save` writes it back
/// beside the statistics (`:286-297`). This world holds no file handle at
/// either end, so the document travels through the candidate and the save
/// request; what has to match is the file on disk.
#[test]
fn a_saved_advancement_document_reaches_a_session_and_the_next_save() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-player-advancements-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let paths = WorldSavePaths::new(&root).unwrap();
    let fixture = FlatFixture::new().unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let column = default_overworld_level_source()
        .unwrap()
        .generate_chunk(0, 0, fixture.biome_id())
        .into_column();
    let created = created_save(
        &Java26WorldCreation {
            level_name: "bbb-advancements".to_string(),
            game_type: ServerWorld::fixture_game_type_id(),
            difficulty: Difficulty::Easy.serialized_name().to_string(),
            spawn_position: ServerWorld::fixture_spawn_block(),
            last_played: 1_786_000_000_000,
        },
        &fixture,
        &[(0, 0, &column)],
    )
    .unwrap();
    storage
        .create_world(&created, RegionCompression::Zlib, 100)
        .unwrap();

    // The document an official server writes for a completed
    // `minecraft:story/root`, whose one criterion is `crafting_table`
    // (data/minecraft/advancement/story/root.json).
    let uuid = profile("newcomer", 1).uuid;
    let document = serde_json::json!({
        "minecraft:story/root": {
            "criteria": {"crafting_table": "2026-08-31 02:30:45 +0000"},
            "done": true,
        },
        "DataVersion": bbb_server_persistence::JAVA_26_1_DATA_VERSION,
    });
    std::fs::create_dir_all(
        bbb_server_persistence::player_advancements::player_advancements_dir(&paths),
    )
    .unwrap();
    std::fs::write(
        bbb_server_persistence::player_advancements::player_advancements_file(&paths, uuid),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();

    let candidate = storage.load_candidate().unwrap();
    // A document read off disk carries no requirement grid yet, so what it
    // restores is the obtained criterion (`AdvancementProgress.java:37-43`).
    assert!(candidate.player_advancements.is_empty());
    assert_eq!(
        candidate
            .load_player_advancements(uuid)
            .unwrap()
            .unwrap()
            .progress["minecraft:story/root"]
            .criterion(&bbb_protocol::JavaString::from("crafting_table"))
            .and_then(|criterion| criterion.obtained_epoch_millis()),
        Some(1_788_143_445_000)
    );

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(startup_generation()),
            ..creative_fixture_config()
        },
        &candidate,
    )
    .unwrap();
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "newcomer", 1);
    let online = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    // The session's store answers `done` off the live requirement grid rather
    // than off the file (`AdvancementProgress.java:42`), so a document that
    // survives a join is one the grid still calls done.
    let online_document: serde_json::Value =
        serde_json::from_slice(&online.player_advancements[&uuid].to_file_bytes()).unwrap();
    assert_eq!(
        online_document["minecraft:story/root"],
        document["minecraft:story/root"]
    );
    assert_eq!(online_document["DataVersion"], document["DataVersion"]);
    assert_eq!(
        online_document["minecraft:recipes/building_blocks/stone_bricks"]["done"],
        true
    );
    let document = online_document;

    // `PlayerList.remove` saves before it drops the store (`:302`, `:327`), so
    // the document survives the disconnect too.
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    let departed = world
        .save_request(1_786_000_000_002, 102, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            &departed.player_advancements[&uuid].to_file_bytes()
        )
        .unwrap(),
        document
    );
    storage.save_snapshot(&candidate, &departed).unwrap();
    storage.close().unwrap();

    let written: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            bbb_server_persistence::player_advancements::player_advancements_file(&paths, uuid),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(written, document);
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    assert!(storage
        .load_candidate()
        .unwrap()
        .has_player_advancements(uuid));
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// `MinecraftServer.reloadResources` calls `PlayerList.saveAll`
/// (`MinecraftServer.java:1539`) before `PlayerList.reloadResources`
/// (`:1540`), so vanilla's `players/advancements/<uuid>.json` is current the
/// moment a reload finishes. This world owns no file handle, so its projection
/// of `PlayerList.saveAll` is `ServerWorld::save_request`
/// (`PlayerList.java:619-623`) and only the runtime can execute it: the write
/// lands at the next save rather than inside the reload. What that deferral
/// must not do is lose the progress on the way, which is what this pins --
/// progress that has never been anywhere but memory, carried through a reload
/// and read back off disk.
#[test]
fn a_reload_carries_memory_only_advancement_progress_to_the_next_save() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-reload-advancements-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let paths = WorldSavePaths::new(&root).unwrap();
    let fixture = FlatFixture::new().unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let column = default_overworld_level_source()
        .unwrap()
        .generate_chunk(0, 0, fixture.biome_id())
        .into_column();
    let created = created_save(
        &Java26WorldCreation {
            level_name: "bbb-reload-advancements".to_string(),
            game_type: ServerWorld::fixture_game_type_id(),
            difficulty: Difficulty::Easy.serialized_name().to_string(),
            spawn_position: ServerWorld::fixture_spawn_block(),
            last_played: 1_786_000_000_000,
        },
        &fixture,
        &[(0, 0, &column)],
    )
    .unwrap();
    storage
        .create_world(&created, RegionCompression::Zlib, 100)
        .unwrap();

    let candidate = storage.load_candidate().unwrap();
    let startup = startup_generation();
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(Arc::clone(&startup)),
            ..creative_fixture_config()
        },
        &candidate,
    )
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "newcomer", 1);

    let uuid = profile("newcomer", 1).uuid;
    let document =
        bbb_server_persistence::player_advancements::player_advancements_file(&paths, uuid);
    assert!(!document.exists(), "the save has never held one");
    assert!(world.award_player_advancement_for_test(
        ConnectionId(1),
        "minecraft:story/root",
        "crafting_table",
        1_788_143_445_000,
    ));
    assert!(world.player_advancement_is_done(ConnectionId(1), "minecraft:story/root"));
    // The award reached no file: this world writes only through a save request.
    assert!(!document.exists());

    enqueue(
        &sender,
        WorldCommand::Datapacks {
            startup: ReloadedDatapacks::Built(Arc::clone(&startup)),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert!(world.player_advancement_is_done(ConnectionId(1), "minecraft:story/root"));

    let request = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let written = bbb_server_persistence::Java26PlayerAdvancements::parse_bytes(
        &std::fs::read(&document).expect("the save request wrote the document"),
    )
    .expect("the document this world writes is one it reads");
    assert_eq!(
        written.progress["minecraft:story/root"]
            .criteria()
            .get(&bbb_protocol::JavaString::from("crafting_table"))
            .and_then(bbb_server_persistence::CriterionProgress::obtained_epoch_millis),
        Some(1_788_143_445_000)
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// A world built with no datapack generation has no `ServerAdvancementManager`
/// behind `MinecraftServer.getAdvancements()` (`MinecraftServer.java:1496-1498`),
/// so it must not answer for a document it could only overwrite with an empty
/// one.
#[test]
fn a_world_without_a_datapack_generation_writes_no_advancement_document() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "newcomer", 1);
    let request = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(request.player_patches.len(), 1);
    assert!(request.player_advancements.is_empty());
}

/// `Player.addAdditionalSaveData` writes `Score` and `readAdditionalSaveData`
/// reads it back (`Player.java:637`, `:656`), so the returning client is
/// synced the score its death screen shows.
#[test]
fn the_player_score_survives_playerdata_save_and_restore() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(
        operator_config(&[("op", 9)]),
        &restore_candidate(0, None, vec![]),
    )
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 9, "op", 9);
    chat_command(&sender, &receiver, &mut world, 9, "xp add op 14");

    let request = world
        .save_request(1_786_000_002_000, 202, RegionCompression::Zlib)
        .unwrap();
    let patch = &request.player_patches[0];
    let document = bbb_server_persistence::created_player_document(
        patch,
        request.item_stack_resolver.as_dyn(),
    )
    .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(document, patch.uuid).unwrap();
    assert_eq!(parsed.experience().score(), 14);

    let mut restored = ServerWorld::restore(
        creative_fixture_config(),
        &restore_candidate(0, None, vec![(patch.uuid, parsed)]),
    )
    .unwrap();
    let (sender, receiver) = mailbox();
    let (joined, handshake) = join_loaded_ticks(&sender, &receiver, &mut restored, 9, "op", 9);
    let next = restored.tick(receiver.drain());
    let synced: Vec<_> = [joined, handshake, next]
        .iter()
        .flat_map(|tick| &tick.events)
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(9),
                packet: ClientboundPlayPacket::SetEntityData(data),
            } => data.values.iter().find_map(|value| {
                (value.data_id == bbb_protocol::entity_synched_data::VANILLA_PLAYER_SCORE_DATA_ID)
                    .then(|| value.value.clone())
            }),
            _ => None,
        })
        .collect();
    assert_eq!(
        synced,
        vec![bbb_protocol::packets::EntityDataValueKind::Int(14)]
    );
}

#[test]
fn a_zero_enchanting_seed_is_rerolled_on_load() {
    let uuid = Uuid::from_u128(0x5eed);
    let player = restore_fixture_player(uuid, [3.5, 70.0, 4.5], [90.0, -30.0], true);
    let mut raw = player.into_raw();
    raw.entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "XpSeed")
        .unwrap()
        .value = NbtSummaryValue::Int(0);
    let player = Java26PlayerData::parse(raw, uuid).unwrap();
    let candidate = restore_candidate(4_200, None, vec![(uuid, player)]);
    let mut world = ServerWorld::restore(creative_fixture_config(), &candidate).unwrap();
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(7),
            profile: profile("alice", 0x5eed),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    assert_ne!(
        request.player_patches[0]
            .created
            .experience
            .enchantment_seed(),
        0
    );
}

#[test]
fn restore_defers_offline_player_files_and_maps_until_accessed_and_fails_closed_on_corrupt_join() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-lazy-offline-records-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let paths = WorldSavePaths::new(&root).unwrap();
    let fixture = FlatFixture::new().unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let column = default_overworld_level_source()
        .unwrap()
        .generate_chunk(0, 0, fixture.biome_id())
        .into_column();
    let created = created_save(
        &Java26WorldCreation {
            level_name: "lazy-offline-records".to_string(),
            game_type: ServerWorld::fixture_game_type_id(),
            difficulty: Difficulty::Easy.serialized_name().to_string(),
            spawn_position: ServerWorld::fixture_spawn_block(),
            last_played: 1_786_000_000_000,
        },
        &fixture,
        &[(0, 0, &column)],
    )
    .unwrap();
    storage
        .create_world(&created, RegionCompression::Zlib, 100)
        .unwrap();
    let baseline = storage.load_candidate().unwrap();

    let alice_uuid = profile("alice", 1).uuid;
    let corrupt_uuid = profile("corrupt", 2).uuid;

    // Seed one valid player (playerdata, stats, advancements) and one valid map.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(startup_generation()),
            ..creative_fixture_config()
        },
        &baseline,
    )
    .unwrap();
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    let moved = Vec3d {
        x: 6.5,
        y: 65.0,
        z: 4.5,
    };
    place_player(&sender, &receiver, &mut world, 1, moved, None);
    let valid_map = bbb_server_persistence::Java26MapSavedData {
        dimension: "minecraft:overworld".to_string(),
        center_x: 128,
        center_z: -64,
        scale: 1,
        tracking_position: true,
        unlimited_tracking: false,
        locked: false,
        colors: vec![12; 128 * 128],
        banners: Vec::new(),
        frames: Vec::new(),
    };
    world.install_map_for_test(3, &valid_map, true).unwrap();
    let request = world
        .save_request(1_786_000_000_050, 150, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    // Seed a second offline player with corrupt playerdata/stats/advancements
    // and a second corrupt map file. None of these should block restore or
    // alice's join and save.
    let corrupt_player_path = paths
        .player_data_dir()
        .join(format!("{}.dat", corrupt_uuid.hyphenated()));
    let corrupt_stats_path = paths.player_stats_file(corrupt_uuid);
    let corrupt_adv_path =
        bbb_server_persistence::player_advancements::player_advancements_file(&paths, corrupt_uuid);
    let corrupt_map_path = paths.map_data_dat(9);
    std::fs::create_dir_all(corrupt_player_path.parent().unwrap()).unwrap();
    std::fs::create_dir_all(corrupt_stats_path.parent().unwrap()).unwrap();
    std::fs::create_dir_all(corrupt_adv_path.parent().unwrap()).unwrap();
    std::fs::create_dir_all(corrupt_map_path.parent().unwrap()).unwrap();
    let bad_bytes = b"not-valid-nbt-or-json";
    std::fs::write(&corrupt_player_path, bad_bytes).unwrap();
    std::fs::write(&corrupt_stats_path, bad_bytes).unwrap();
    std::fs::write(&corrupt_adv_path, bad_bytes).unwrap();
    std::fs::write(&corrupt_map_path, bad_bytes).unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let reopened = storage.load_candidate().unwrap();
    assert!(reopened.players.is_empty());
    assert!(reopened.player_stats.is_empty());
    assert!(reopened.player_advancements.is_empty());
    assert!(reopened.maps.is_empty());
    assert!(reopened.has_player(alice_uuid));
    assert!(reopened.has_player(corrupt_uuid));
    assert!(reopened.has_map(3));
    assert!(reopened.has_map(9));

    let (sender, receiver) = mailbox();
    let mut restored = ServerWorld::restore(
        ServerWorldConfig {
            datapacks: Some(startup_generation()),
            ..creative_fixture_config()
        },
        &reopened,
    )
    .unwrap();
    assert!(!restored.is_map_loaded_in_memory_for_test(3));
    assert!(!restored.is_map_loaded_in_memory_for_test(9));

    // Joining alice loads only alice's files and restores her saved position.
    join_loaded(&sender, &receiver, &mut restored, 1, "alice", 1);
    let tick = restored.tick(Vec::new());
    let alice = tick
        .snapshot
        .players
        .iter()
        .find(|player| player.uuid == alice_uuid)
        .expect("alice joined lazily");
    assert_eq!(alice.position, moved);

    // Accessing map 3 loads only map 3, leaving corrupt map 9 untouched.
    assert!(matches!(
        restored.map_admission_for_test(3),
        crate::entity::FramedMapAdmission::Present { .. }
    ));
    assert!(restored.is_map_loaded_in_memory_for_test(3));
    assert!(!restored.is_map_loaded_in_memory_for_test(9));

    // Joining the corrupt player fails closed with WorldEvent::Close and never
    // fabricates a replacement player patch on save.
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(2),
            profile: profile("corrupt", 2),
        },
    )
    .unwrap();
    let failed_join = restored.tick(receiver.drain());
    assert!(failed_join.events.iter().any(|event| matches!(
        event,
        WorldEvent::Close {
            connection_id: ConnectionId(2),
            ..
        }
    )));
    assert!(!failed_join
        .snapshot
        .players
        .iter()
        .any(|player| player.uuid == corrupt_uuid));

    // Saving preserves all untouched corrupt files verbatim on disk.
    let save_after = restored
        .save_request(1_786_000_000_100, 200, RegionCompression::Zlib)
        .unwrap();
    assert!(!save_after
        .player_patches
        .iter()
        .any(|patch| patch.uuid == corrupt_uuid));
    assert!(!save_after.departed_player_stats.contains_key(&corrupt_uuid));
    assert!(!save_after.player_advancements.contains_key(&corrupt_uuid));
    assert!(!save_after.map_writes.contains_key(&9));
    storage.save_snapshot(&reopened, &save_after).unwrap();
    storage.close().unwrap();

    assert_eq!(std::fs::read(&corrupt_player_path).unwrap(), bad_bytes);
    assert_eq!(std::fs::read(&corrupt_stats_path).unwrap(), bad_bytes);
    assert_eq!(std::fs::read(&corrupt_adv_path).unwrap(), bad_bytes);
    assert_eq!(std::fs::read(&corrupt_map_path).unwrap(), bad_bytes);
    std::fs::remove_dir_all(&root).unwrap();
}
