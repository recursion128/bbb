use super::*;

fn map_data(color: u8) -> bbb_server_persistence::Java26MapSavedData {
    bbb_server_persistence::Java26MapSavedData {
        dimension: "minecraft:overworld".to_string(),
        center_x: 0,
        center_z: 0,
        scale: 0,
        colors: vec![color; 128 * 128],
        tracking_position: true,
        unlimited_tracking: false,
        locked: false,
        banners: Vec::new(),
        frames: Vec::new(),
    }
}

fn filled_map(map_id: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
            ],
            map_id: Some(map_id),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    }
}

fn persistent_filled_map(map_id: i32) -> bbb_server_types::item_stack::RuntimeItemStack {
    bbb_server_types::item_stack::RuntimeItemStack::from_summary(filled_map(map_id))
}

fn map_block(name: &str) -> i32 {
    crate::block_behavior::default_block_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
}

fn map_packets(tick: &TickResult) -> Vec<(ConnectionId, bbb_protocol::packets::MapItemData)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::MapItemData(packet),
            } => Some((*connection_id, packet.clone())),
            _ => None,
        })
        .collect()
}

fn collect_map_colors(tick: &TickResult, map_id: i32, colors: &mut BTreeMap<(i32, i32), u8>) {
    for (_, packet) in map_packets(tick)
        .into_iter()
        .filter(|(_, packet)| packet.map_id == map_id)
    {
        if let Some(patch) = packet.color_patch {
            for y in 0..i32::from(patch.height) {
                for x in 0..i32::from(patch.width) {
                    colors.insert(
                        (i32::from(patch.start_x) + x, i32::from(patch.start_y) + y),
                        patch.colors[(x + y * i32::from(patch.width)) as usize],
                    );
                }
            }
        }
    }
}

fn map_banner_markers(
    world: &mut ServerWorld,
    first: &TickResult,
    map_id: i32,
) -> Vec<bbb_protocol::packets::MapDecoration> {
    let mut packets = map_packets(first);
    for _ in 0..6 {
        packets.extend(map_packets(&world.tick(Vec::new())));
    }
    packets
        .into_iter()
        .filter(|(_, packet)| packet.map_id == map_id)
        .filter_map(|(_, packet)| packet.decorations)
        .next_back()
        .expect("changed map decorations are sent")
        .into_iter()
        .filter(|marker| (10..=25).contains(&marker.type_id))
        .collect()
}

#[test]
fn map_banner_clicks_toggle_named_markers_and_scanning_removes_missing_banners() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 0 64 1 red_banner",
    );
    let pos = BlockPos { x: 0, y: 64, z: 1 };
    let name = NbtSummaryValue::String("Harbor".into());
    let mut record = save_native_fixture_block_entity(
        world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, pos)
            .unwrap(),
    );
    record.push(nbt("CustomName", name.clone()));
    let banner = load_native_fixture_block_entity(pos, map_block("minecraft:red_banner"), &record);
    *world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, pos)
        .unwrap() = banner;
    world.install_map_for_test(27, &map_data(0), false).unwrap();
    world.set_inventory_slot_for_test(ConnectionId(1), 0, filled_map(27));
    let click = |world: &mut ServerWorld, sequence| {
        play(
            &sender,
            &receiver,
            world,
            1,
            PlayServerbound::UseItemOn(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: BlockHitResult {
                    pos,
                    direction: Direction::Up,
                    cursor_x: 0.5,
                    cursor_y: 0.5,
                    cursor_z: 0.5,
                    inside: false,
                    world_border_hit: false,
                },
                sequence,
            }),
        )
    };
    let added = click(&mut world, 1);
    let markers = map_banner_markers(&mut world, &added, 27);
    assert_eq!(
        markers,
        vec![bbb_protocol::packets::MapDecoration {
            type_id: 24,
            x: 1,
            y: 3,
            rot: 8,
            name: Some(
                bbb_protocol::RichTextComponent::try_from_component_nbt(name.clone()).unwrap()
            ),
        }]
    );
    let saved = world
        .save_request(1_786_000_000_050, 150, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(
        saved.map_writes[&27].banners,
        vec![bbb_server_persistence::Java26MapBanner {
            pos: [0, 64, 1],
            color: "red".into(),
            name: Some(name),
        }]
    );
    world.acknowledge_save_success(&saved);
    let removed = click(&mut world, 2);
    assert!(map_banner_markers(&mut world, &removed, 27).is_empty());
    let added = click(&mut world, 3);
    assert_eq!(map_banner_markers(&mut world, &added, 27), markers);
    assert_eq!(
        world.publication().players[0].inventory.slots[0],
        filled_map(27)
    );

    let broken = chat_command(&sender, &receiver, &mut world, 1, "setblock 0 64 1 air");
    let mut cleared = map_packets(&broken).into_iter().any(|(_, packet)| {
        packet.map_id == 27
            && packet
                .decorations
                .is_some_and(|markers| markers.iter().all(|m| m.type_id != 24))
    });
    for _ in 0..320 {
        if cleared {
            break;
        }
        cleared = map_packets(&world.tick(Vec::new()))
            .into_iter()
            .any(|(_, packet)| {
                packet.map_id == 27
                    && packet
                        .decorations
                        .is_some_and(|markers| markers.iter().all(|m| m.type_id != 24))
            });
    }
    assert!(cleared, "a terrain scan clears the removed banner's marker");
    let saved = world
        .save_request(1_786_000_000_051, 151, RegionCompression::Zlib)
        .unwrap();
    assert!(saved.map_writes[&27].banners.is_empty());
}

#[test]
fn held_maps_draw_surface_materials_slopes_and_water_through_incremental_packets() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    for x in 0..4 {
        for z in -1..=0 {
            world.set_raw_block_state_for_test(
                Java26BuiltinDimension::Overworld,
                BlockPos { x, y: 64, z },
                map_block("minecraft:grass_block"),
            );
        }
    }
    world.set_raw_block_state_for_test(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 0, y: 67, z: 0 },
        map_block("minecraft:stone"),
    );
    world.set_raw_block_state_for_test(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 1, y: 66, z: 0 },
        map_block("minecraft:glass"),
    );
    world.set_raw_block_state_for_test(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 2, y: 65, z: 0 },
        map_block("minecraft:water"),
    );
    for y in 54..=65 {
        world.set_raw_block_state_for_test(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 3, y, z: 0 },
            map_block("minecraft:water"),
        );
    }
    for map_id in [20, 21, 22] {
        world
            .install_map_for_test(map_id, &map_data(0), false)
            .unwrap();
    }
    world.set_inventory_slot_for_test(ConnectionId(1), 0, filled_map(20));
    world.set_inventory_slot_for_test(ConnectionId(1), 40, filled_map(21));
    world.set_inventory_slot_for_test(ConnectionId(1), 10, filled_map(22));
    let mut main = BTreeMap::new();
    let mut offhand = BTreeMap::new();
    for _ in 0..48 {
        let tick = world.tick(Vec::new());
        collect_map_colors(&tick, 20, &mut main);
        collect_map_colors(&tick, 21, &mut offhand);
        if main.get(&(67, 64)) == Some(&48) && offhand.get(&(67, 64)) == Some(&48) {
            break;
        }
    }
    for colors in [&main, &offhand] {
        assert_eq!(colors.get(&(64, 64)), Some(&46), "raised stone is bright");
        assert_eq!(
            colors.get(&(65, 64)),
            Some(&5),
            "NONE glass reveals flat grass"
        );
        assert_eq!(colors.get(&(66, 64)), Some(&50), "shallow water is bright");
        assert_eq!(colors.get(&(67, 64)), Some(&48), "deep water is dark");
    }
    let saved = world
        .save_request(1_786_000_000_030, 130, RegionCompression::Zlib)
        .unwrap();
    assert!(
        !saved.map_writes.contains_key(&22),
        "inventory storage does not explore"
    );
    world.acknowledge_save_success(&saved);
    world.set_raw_block_state_for_test(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 0, y: 67, z: 0 },
        map_block("minecraft:dirt"),
    );
    let mut incremental = false;
    for _ in 0..160 {
        let tick = world.tick(Vec::new());
        incremental |= map_packets(&tick).iter().any(|(_, packet)| {
            packet.map_id == 20
                && packet
                    .color_patch
                    .as_ref()
                    .is_some_and(|patch| patch.width < 128 || patch.height < 128)
        });
        collect_map_colors(&tick, 20, &mut main);
        if main.get(&(64, 64)) == Some(&42) {
            break;
        }
    }
    assert_eq!(
        main.get(&(64, 64)),
        Some(&42),
        "held maps revisit changed terrain"
    );
    assert!(
        incremental,
        "existing holders receive bounded color patches"
    );

    let mut locked = map_data(19);
    locked.locked = true;
    let mut other_level = map_data(19);
    other_level.dimension = "minecraft:the_nether".into();
    world.install_map_for_test(23, &locked, false).unwrap();
    world.install_map_for_test(24, &other_level, false).unwrap();
    world.set_inventory_slot_for_test(ConnectionId(1), 0, filled_map(23));
    world.set_inventory_slot_for_test(ConnectionId(1), 40, filled_map(24));
    world.tick(Vec::new());
    let saved = world
        .save_request(1_786_000_000_031, 131, RegionCompression::Zlib)
        .unwrap();
    assert!(!saved.map_writes.contains_key(&23));
    assert!(!saved.map_writes.contains_key(&24));
}

#[test]
fn ceiling_dimension_maps_show_noise_instead_of_the_surface() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "execute in minecraft:the_nether run tp alice 0 70 0",
    );
    let mut data = map_data(0);
    data.dimension = "minecraft:the_nether".into();
    world.install_map_for_test(25, &data, false).unwrap();
    world.set_inventory_slot_for_test(ConnectionId(1), 0, filled_map(25));
    let mut colors = BTreeMap::new();
    for _ in 0..16 {
        collect_map_colors(&world.tick(Vec::new()), 25, &mut colors);
        if colors.get(&(64, 64)) == Some(&41) {
            break;
        }
    }
    assert_eq!(
        colors.get(&(64, 64)),
        Some(&41),
        "ceiling noise at the origin is normal dirt"
    );
}

#[test]
fn held_map_loads_terrain_beyond_view_without_extending_simulation() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
    );
    let mut data = map_data(0);
    data.scale = 4;
    world.install_map_for_test(26, &data, false).unwrap();
    world.set_inventory_slot_for_test(ConnectionId(1), 40, filled_map(26));
    let mut colors = BTreeMap::new();
    for _ in 0..48 {
        collect_map_colors(&world.tick(Vec::new()), 26, &mut colors);
        if colors.get(&(64, 70)) == Some(&45) {
            break;
        }
    }
    assert_eq!(
        colors.get(&(64, 70)),
        Some(&45),
        "scale-four map draws the flat stone column at z=96"
    );
    assert!(!world
        .entity_ticking_chunks(Java26BuiltinDimension::Overworld)
        .contains(&(0, 6)));
    assert!(!world
        .block_ticking_chunks(Java26BuiltinDimension::Overworld)
        .contains(&(0, 6)));
}

/// `MapItem.update` (`MapItem.java:88-95`) redraws every sixteenth column
/// each tick whatever the scale, so a scale-four map fills its whole reveal
/// disc within a couple of column sweeps.
#[test]
fn a_zoomed_held_map_fills_its_reveal_disc_in_column_sweeps() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
    );
    let mut data = map_data(0);
    data.scale = 4;
    world.install_map_for_test(28, &data, false).unwrap();
    world.set_inventory_slot_for_test(ConnectionId(1), 40, filled_map(28));
    // Radius 8 around pixel (64, 64); the inner disc has no dithered pixels.
    let disc: Vec<(i32, i32)> = (56..72)
        .flat_map(|x: i32| (56..72).map(move |y: i32| (x, y)))
        .filter(|&(x, y)| (x - 64).pow(2) + (y - 64).pow(2) <= 36)
        .collect();
    let mut colors = BTreeMap::new();
    for _ in 0..64 {
        collect_map_colors(&world.tick(Vec::new()), 28, &mut colors);
        if disc
            .iter()
            .all(|pixel| colors.get(pixel).is_some_and(|&color| color != 0))
        {
            break;
        }
    }
    let drawn = disc
        .iter()
        .filter(|pixel| colors.get(pixel).is_some_and(|&color| color != 0))
        .count();
    assert_eq!(drawn, disc.len(), "the whole disc is drawn");
}

#[test]
fn storage_and_offhand_maps_each_create_a_holder_and_send_one_full_update() {
    for inventory_slot in [0, 40] {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
        let mut data = map_data(23);
        data.locked = true;
        world.install_map_for_test(7, &data, false).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        world.set_inventory_slot_for_test(ConnectionId(1), inventory_slot, filled_map(7));

        let first = world.tick(Vec::new());
        let packets = map_packets(&first);
        assert_eq!(packets.len(), 1, "inventory slot {inventory_slot}");
        assert_eq!(packets[0].0, ConnectionId(1));
        assert_eq!(packets[0].1.map_id, 7);
        let patch = packets[0]
            .1
            .color_patch
            .as_ref()
            .expect("a new holder receives all colors");
        assert_eq!((patch.width, patch.height), (128, 128));
        assert_eq!(patch.colors, vec![23; 128 * 128]);
        assert!(map_packets(&world.tick(Vec::new())).is_empty());
    }
}

#[test]
fn disconnect_and_respawn_each_reset_connection_scoped_map_holders() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    world.install_map_for_test(8, &map_data(17), false).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, filled_map(8));
    assert_eq!(map_packets(&world.tick(Vec::new())).len(), 1);
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    // The departed session comes back holding the map its own save carried, so
    // the join is already the tick that answers the reset holder set.
    let (rejoined, _) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "alice", 1);
    assert_eq!(map_packets(&rejoined).len(), 1);

    let mut config = operator_config(&[("bob", 2)]);
    config.game_type = GameType::Survival;
    let mut respawn_world = ServerWorld::new(config).unwrap();
    respawn_world
        .install_map_for_test(8, &map_data(17), false)
        .unwrap();
    join_loaded(&sender, &receiver, &mut respawn_world, 2, "bob", 2);
    respawn_world.set_inventory_slot_for_test(ConnectionId(2), 0, filled_map(8));
    assert_eq!(map_packets(&respawn_world.tick(Vec::new())).len(), 1);
    chat_command(&sender, &receiver, &mut respawn_world, 2, "damage bob 40");
    play(
        &sender,
        &receiver,
        &mut respawn_world,
        2,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    respawn_world.set_inventory_slot_for_test(ConnectionId(2), 0, filled_map(8));
    assert_eq!(map_packets(&respawn_world.tick(Vec::new())).len(), 1);
}

#[test]
fn framed_map_tracker_updates_every_level_player_and_removal_reaches_the_save() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    world.install_map_for_test(9, &map_data(4), false).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let attached = BlockPos { x: 2, y: 64, z: 0 };
    let (entity_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        Vec3d {
            x: 2.5,
            y: 64.5,
            z: 0.5,
        },
        crate::entity::EntityData::ItemFrame(
            crate::entity::ItemFrameData::new(attached, Direction::North).with_saved_fields(
                Some(persistent_filled_map(9)),
                0,
                1.0,
                false,
                false,
            ),
        ),
    );

    let tracked = world.tick(Vec::new());
    assert_eq!(
        map_packets(&tracked)
            .into_iter()
            .map(|(connection_id, _)| connection_id)
            .collect::<Vec<_>>(),
        vec![ConnectionId(1), ConnectionId(2)]
    );
    let initial = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(initial.map_writes[&9].frames[0].entity_id, entity_id);
    world.acknowledge_save_success(&initial);

    assert!(world.remove_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        entity_id,
        crate::entity::RemovalReason::Discarded,
    ));
    let removed = world
        .save_request(1_786_000_000_002, 102, RegionCompression::Zlib)
        .unwrap();
    assert!(removed.map_writes[&9].frames.is_empty());
}

#[test]
fn framed_map_tracker_skips_a_player_whose_avatar_left_the_level() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    world.install_map_for_test(10, &map_data(6), false).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.mark_avatar_removed_for_test(ConnectionId(2));
    let attached = BlockPos { x: 3, y: 64, z: 0 };
    world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        Vec3d {
            x: 3.5,
            y: 64.5,
            z: 0.5,
        },
        crate::entity::EntityData::ItemFrame(
            crate::entity::ItemFrameData::new(attached, Direction::North).with_saved_fields(
                Some(persistent_filled_map(10)),
                0,
                1.0,
                false,
                false,
            ),
        ),
    );

    assert_eq!(
        map_packets(&world.tick(Vec::new()))
            .into_iter()
            .map(|(connection_id, _)| connection_id)
            .collect::<Vec<_>>(),
        vec![ConnectionId(1)]
    );
}

#[test]
fn save_acknowledgement_only_retires_the_exact_map_snapshot() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    world.install_map_for_test(11, &map_data(1), true).unwrap();
    let first = world
        .save_request(1_786_000_000_010, 110, RegionCompression::Zlib)
        .unwrap();
    world.update_map_color_for_test(11, 3, 4, 8).unwrap();
    world.acknowledge_save_success(&first);

    let second = world
        .save_request(1_786_000_000_011, 111, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(second.map_writes[&11].colors[3 + 4 * 128], 8);
    world.acknowledge_save_success(&second);
    assert!(world
        .save_request(1_786_000_000_012, 112, RegionCompression::Zlib)
        .unwrap()
        .map_writes
        .is_empty());
}

#[test]
fn map_saved_data_round_trips_through_storage_and_world_restore() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-map-runtime-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-map-runtime");
    let baseline = storage.load_candidate().unwrap();
    let config = ServerWorldConfig {
        game_type: GameType::Survival,
        ..ServerWorldConfig::default()
    };
    let mut world = ServerWorld::restore(config.clone(), &baseline).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 201.75,
            y: 65.0,
            z: -101.25,
        },
    );
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:map")),
            count: 1,
            component_patch: Default::default(),
        },
    );
    let used = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItem(UseItem {
            hand: InteractionHand::MainHand,
            sequence: 1,
            y_rot: 0.0,
            x_rot: 0.0,
        }),
    );
    let held = world.player_inventory_slot_for_test(ConnectionId(1), 0);
    assert_eq!(held.item_id, Some(item_id("minecraft:filled_map")));
    assert_eq!(held.count, 1);
    assert_eq!(held.component_patch.map_id, Some(0));
    assert!(used.events.iter().any(|event| matches!(event, WorldEvent::Send {
        connection_id: ConnectionId(1), packet: ClientboundPlayPacket::Sound(sound),
    } if sound.sound == SoundEventHolder::Reference { registry_id: vanilla_sound_event_registry_id("minecraft:ui.cartography_table.take_result").unwrap() })));
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID,
                item_id("minecraft:map")
            )
        ),
        1
    );
    let mut explored = BTreeMap::new();
    collect_map_colors(&used, 0, &mut explored);
    for _ in 0..8 {
        if explored.get(&(64, 64)).is_some_and(|color| *color != 0) {
            break;
        }
        collect_map_colors(&world.tick(Vec::new()), 0, &mut explored);
    }
    let explored_center = explored[&(64, 64)];
    assert_ne!(explored_center, 0, "the held map explores before saving");
    let mut data = map_data(31);
    data.banners.push(bbb_server_persistence::Java26MapBanner {
        pos: [3, 70, -4],
        color: "red".to_string(),
        name: Some(NbtSummaryValue::string("home")),
    });
    data.frames.push(bbb_server_persistence::Java26MapFrame {
        pos: [8, 65, 2],
        rotation: 180,
        entity_id: 44,
    });
    world.install_map_for_test(-7, &data, true).unwrap();
    let request = world
        .save_request(1_786_000_000_020, 120, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(request.map_last_id, Some(0));
    assert_eq!(
        (
            request.map_writes[&0].center_x,
            request.map_writes[&0].center_z
        ),
        (256, -128)
    );
    assert_eq!(
        request.map_writes[&0].colors[64 + 64 * 128],
        explored_center
    );
    storage.save_snapshot(&baseline, &request).unwrap();
    world.acknowledge_save_success(&request);

    let reopened = storage.load_candidate().unwrap();
    assert!(reopened.maps.is_empty());
    assert_eq!(
        reopened.load_map(0).unwrap().unwrap().colors,
        request.map_writes[&0].colors
    );
    let reopened_neg7 = reopened.load_map(-7).unwrap().unwrap();
    assert_eq!(reopened_neg7.colors, vec![31; 128 * 128]);
    assert_eq!(
        reopened_neg7.banners[0].name,
        Some(NbtSummaryValue::string("home"))
    );
    assert_eq!(reopened_neg7.frames[0].entity_id, 44);
    assert_eq!(reopened.map_last_id, 0);
    let mut restored = ServerWorld::restore(config, &reopened).unwrap();
    assert!(!restored.is_map_loaded_in_memory_for_test(-7));
    assert!(!restored.is_map_loaded_in_memory_for_test(0));
    assert!(matches!(
        restored.map_admission_for_test(-7),
        crate::entity::FramedMapAdmission::Present { .. }
    ));
    assert!(restored.is_map_loaded_in_memory_for_test(-7));
    assert!(!restored.is_map_loaded_in_memory_for_test(0));
    assert!(restored.maps_are_clean_for_test());
    let (joined, handshake) = join_loaded_ticks(&sender, &receiver, &mut restored, 1, "alice", 1);
    let mut restored_colors = BTreeMap::new();
    collect_map_colors(&joined, 0, &mut restored_colors);
    collect_map_colors(&handshake, 0, &mut restored_colors);
    assert_eq!(restored_colors.get(&(64, 64)), Some(&explored_center));
    restored.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:map")),
            count: 2,
            component_patch: Default::default(),
        },
    );
    play(
        &sender,
        &receiver,
        &mut restored,
        1,
        PlayServerbound::UseItem(UseItem {
            hand: InteractionHand::MainHand,
            sequence: 2,
            y_rot: 0.0,
            x_rot: 0.0,
        }),
    );
    let held = restored.player_inventory_slot_for_test(ConnectionId(1), 0);
    assert_eq!(
        (held.item_id, held.count),
        (Some(item_id("minecraft:map")), 1)
    );
    let added = restored.player_inventory_slot_for_test(ConnectionId(1), 1);
    assert_eq!(added.item_id, Some(item_id("minecraft:filled_map")));
    assert_eq!(added.component_patch.map_id, Some(1));
    let request = restored
        .save_request(1_786_000_000_021, 121, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(request.map_last_id, Some(1));
    storage.save_snapshot(&reopened, &request).unwrap();
    restored.acknowledge_save_success(&request);
    assert!(restored.maps_are_clean_for_test());
    assert_eq!(storage.load_candidate().unwrap().map_last_id, 1);
    let _ = std::fs::remove_dir_all(root);
}
