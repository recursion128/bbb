//! Cats navigate to resting blocks and publish poses independently of sit orders.

use super::*;
use bbb_protocol::entity_synched_data::{
    VANILLA_CAT_IS_LYING_DATA_ID, VANILLA_CAT_RELAX_STATE_ONE_DATA_ID,
    VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID, VANILLA_TAMABLE_ANIMAL_FLAG_SITTING,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID;
use bbb_protocol::packets::{
    BlockHitResult, ContainerCloseRequest, Direction, InteractionHand, UseItemOn,
};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn lying_update(tick: &TickResult, cat: i32, lying: bool) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
                if data.id == cat && data.values.iter().any(|value|
                    value.data_id == VANILLA_CAT_IS_LYING_DATA_ID
                        && value.value == EntityDataValueKind::Boolean(lying))
        )
    })
}

fn sitting_update(tick: &TickResult, cat: i32, sitting: bool) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
                if data.id == cat && data.values.iter().any(|value|
                    value.data_id == VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID
                        && matches!(value.value, EntityDataValueKind::Byte(flags)
                            if (flags & VANILLA_TAMABLE_ANIMAL_FLAG_SITTING != 0) == sitting))
        )
    })
}

fn click_chest(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    pos: BlockPos,
    sequence: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos,
                direction: Direction::North,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.0,
                inside: false,
                world_border_hit: false,
            },
            sequence,
        }),
    )
}

fn opened_container(tick: &TickResult) -> Option<i32> {
    tick.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::OpenScreen(screen),
            ..
        } => Some(screen.container_id),
        _ => None,
    })
}

fn owned_cat(world: &mut ServerWorld, z: f64, sitting: bool, sound_variant: &str) -> i32 {
    let (cat, _) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_CAT_ID,
        Vec3d {
            x: 11.5,
            y: 68.0,
            z,
        },
        crate::entity::EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_CAT_ID).unwrap(),
        ),
    );
    let NbtSummaryValue::Compound(mut entries) =
        world.stored_entity_command_data_for_test(DIM, cat).unwrap()
    else {
        panic!("cat record");
    };
    entries.retain(|entry| {
        entry.name != "Owner" && entry.name != "Sitting" && entry.name != "sound_variant"
    });
    entries.extend([
        NbtSummaryEntry {
            name: "sound_variant".into(),
            value: NbtSummaryValue::String(sound_variant.into()),
        },
        NbtSummaryEntry {
            name: "Owner".into(),
            value: NbtSummaryValue::IntArray(vec![0, 0, 0, 1]),
        },
        NbtSummaryEntry {
            name: "Sitting".into(),
            value: NbtSummaryValue::Byte(i8::from(sitting)),
        },
    ]);
    world.load_stored_entity_record_for_test(DIM, cat, &entries);
    cat
}

#[test]
fn a_tame_cat_lies_on_beds_and_perches_only_on_closed_chests() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    config.keep_alive_interval_ticks = 10_000;
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let classic = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:cat_sound_variant")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:classic")
        .unwrap();
    let mut sounds: serde_json::Value = serde_json::from_slice(&classic.source_json).unwrap();
    sounds["adult_sounds"]["purr_sound"] = serde_json::json!("minecraft:block.note_block.harp");
    classic.source_json = serde_json::to_vec(&sounds).unwrap().into();
    classic.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(sounds)).unwrap())
            .unwrap(),
    );
    classic.codec_value = None;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "owner", 1);
    for (id, command) in ["gamerule spawn_mobs false", "time set midnight"]
        .iter()
        .enumerate()
    {
        let tick = console_command(&sender, &receiver, &mut world, id as u64 + 10, command);
        assert!(console_command_answer(&tick).0.is_some(), "{command}");
    }
    let stone = crate::tests::block_state_id("minecraft:stone");
    let air = crate::tests::block_state_id("minecraft:air");
    for x in 3..=13 {
        for y in 64..=71 {
            for z in 6..=11 {
                let support = y == 64
                    || ((10..=13).contains(&x) && y == 67)
                    || (x == 9 && y == 66)
                    || (x == 8 && y == 65);
                world.set_block(
                    DIM,
                    BlockPos { x, y, z },
                    if support { stone } else { air },
                    2,
                );
            }
        }
    }
    for (x, part) in [(4, "foot"), (5, "head")] {
        let properties = [("facing", "east"), ("occupied", "false"), ("part", part)]
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect();
        let state = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:red_bed",
            &properties,
        )
        .unwrap();
        assert!(world.set_block(DIM, BlockPos { x, y: 65, z: 8 }, state, 2));
        assert_eq!(
            world.block_state(DIM, BlockPos { x, y: 65, z: 8 }),
            Some(state)
        );
    }
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 12.5,
            y: 68.0,
            z: 8.5,
        },
    );
    let cat = owned_cat(&mut world, 8.5, false, "minecraft:classic");
    let seated = owned_cat(&mut world, 10.5, true, "minecraft:classic");
    let seated_start = world
        .stored_entity_for_test(DIM, seated)
        .unwrap()
        .position();
    let mut lying = false;
    for _ in 0..400 {
        let tick = world.tick(receiver.drain());
        assert!(
            !lying_update(&tick, seated, true),
            "the seated cat must not lie on the bed"
        );
        if lying_update(&tick, cat, true) {
            let entity = world.stored_entity_for_test(DIM, cat).unwrap();
            let now = entity.position();
            assert!(
                (now.x - 5.0).abs() < 1.5 && (now.z - 8.5).abs() < 1.0,
                "cat at {now:?}"
            );
            assert!((65.0..66.0).contains(&now.y), "cat at {now:?}");
            lying = true;
            break;
        }
    }
    assert!(
        lying,
        "cat at {:?}",
        world.stored_entity_for_test(DIM, cat).unwrap().position()
    );
    assert_eq!(
        world
            .stored_entity_for_test(DIM, seated)
            .unwrap()
            .position(),
        seated_start
    );
    let mut purred = false;
    for _ in 0..10 {
        let tick = world.tick(receiver.drain());
        purred |= tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
                if matches!(&sound.sound, bbb_protocol::packets::SoundEventHolder::Reference { registry_id }
                    if vanilla_sound_event_resource_id(*registry_id) == Some("minecraft:block.note_block.harp"))));
    }
    assert!(
        purred,
        "the published classic override supplies the lying cat's purr"
    );
    // The floor remains below the removed bed.
    world.set_block(DIM, BlockPos { x: 4, y: 65, z: 8 }, air, 2);
    world.set_block(DIM, BlockPos { x: 5, y: 65, z: 8 }, air, 2);
    let mut cleared = false;
    for _ in 0..40 {
        if lying_update(&world.tick(receiver.drain()), cat, false) {
            cleared = true;
            break;
        }
    }
    assert!(cleared, "the removed bed clears lying");

    // Keep ordinary wandering within range of the chest while its menu is open.
    for x in 3..=10 {
        for z in 6..=11 {
            if x == 3 || x == 10 || z == 6 || z == 11 {
                for y in 65..=70 {
                    world.set_block(DIM, BlockPos { x, y, z }, stone, 2);
                }
            }
        }
    }
    let chest_pos = BlockPos { x: 7, y: 65, z: 8 };
    let chest = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:chest",
        &BTreeMap::from([
            ("facing".into(), "north".into()),
            ("type".into(), "single".into()),
            ("waterlogged".into(), "false".into()),
        ]),
    )
    .unwrap();
    world.set_block(DIM, chest_pos, chest, 2);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 6.5,
            y: 65.0,
            z: 7.5,
        },
    );
    let opened = click_chest(&sender, &receiver, &mut world, chest_pos, 1);
    let container_id = opened_container(&opened).expect("the player opened the real chest menu");
    for _ in 0..500 {
        let tick = world.tick(receiver.drain());
        assert!(
            !sitting_update(&tick, cat, true),
            "an open chest is no perch"
        );
    }
    let cat_uuid = world.stored_entity_for_test(DIM, cat).unwrap().uuid();
    let moved = console_command(
        &sender,
        &receiver,
        &mut world,
        30,
        &format!("teleport {cat_uuid} 4.5 66 8.5"),
    );
    assert!(console_command_answer(&moved).0.is_some());
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest { container_id }),
    );
    let mut perched = false;
    for _ in 0..800 {
        let tick = world.tick(receiver.drain());
        if sitting_update(&tick, cat, true) {
            let now = world.stored_entity_for_test(DIM, cat).unwrap().position();
            assert!((now.x - 7.5).hypot(now.z - 8.5) < 1.0, "cat at {now:?}");
            assert!((65.5..66.5).contains(&now.y), "cat at {now:?}");
            perched = true;
            break;
        }
    }
    assert!(
        perched,
        "the cat walked onto the closed chest: position={:?}, path={:?}",
        world.stored_entity_for_test(DIM, cat).unwrap().position(),
        world.ground_path_for_test(DIM, cat)
    );
    let NbtSummaryValue::Compound(record) =
        world.stored_entity_command_data_for_test(DIM, cat).unwrap()
    else {
        panic!("cat record");
    };
    assert!(
        record
            .iter()
            .any(|entry| entry.name == "Sitting" && entry.value == NbtSummaryValue::Byte(0)),
        "a spontaneous pose must not become a saved sit order"
    );
    assert_eq!(
        world
            .stored_entity_for_test(DIM, seated)
            .unwrap()
            .position(),
        seated_start
    );
    let blocked = click_chest(&sender, &receiver, &mut world, chest_pos, 2);
    assert!(
        opened_container(&blocked).is_none(),
        "the actual sitting pose blocks opening"
    );
    world.set_block(DIM, chest_pos, stone, 2);
    let mut cleared = false;
    for _ in 0..40 {
        if sitting_update(&world.tick(receiver.drain()), cat, false) {
            cleared = true;
            break;
        }
    }
    assert!(cleared, "the removed chest clears the cat's sitting pose");
    world.set_block(DIM, chest_pos, chest, 2);
    let moved = console_command(
        &sender,
        &receiver,
        &mut world,
        31,
        &format!("teleport {cat_uuid} 4.5 66 8.5"),
    );
    assert!(console_command_answer(&moved).0.is_some());
    let opened = click_chest(&sender, &receiver, &mut world, chest_pos, 3);
    assert!(
        opened_container(&opened).is_some(),
        "restoring the chest and moving the cat away lets it open"
    );
}

#[test]
fn a_cat_relaxes_beside_its_sleeping_owner_and_brings_a_morning_gift() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    config.keep_alive_interval_ticks = 10_000;
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let sounds = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:cat_sound_variant")
        .unwrap();
    let mut quiet = sounds
        .elements
        .iter()
        .find(|entry| entry.id == "minecraft:classic")
        .unwrap()
        .clone();
    let mut value: serde_json::Value = serde_json::from_slice(&quiet.source_json).unwrap();
    value["adult_sounds"]["purr_sound"] =
        serde_json::json!({"sound_id":"test:quiet_purr", "range":32.0});
    quiet.id = "test:quiet".into();
    quiet.source_json = serde_json::to_vec(&value).unwrap().into();
    quiet.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(value)).unwrap())
            .unwrap(),
    );
    quiet.codec_value = None;
    sounds.elements.push(quiet);
    let dimension = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:dimension_type")
        .unwrap()
        .elements
        .iter_mut()
        .find(|entry| entry.id == "minecraft:overworld")
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&dimension.source_json).unwrap();
    value["attributes"]["minecraft:gameplay/bed_rule"] =
        serde_json::json!({"can_sleep":"always","can_set_spawn":"always"});
    value["attributes"]["minecraft:gameplay/cat_waking_up_gift_chance"] = serde_json::json!(1.0);
    dimension.source_json = serde_json::to_vec(&value).unwrap().into();
    dimension.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(value)).unwrap())
            .unwrap(),
    );
    let gift = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .unwrap()
        .elements
        .iter_mut()
        .find(|entry| entry.id == "minecraft:gameplay/cat_morning_gift")
        .unwrap();
    gift.value = bbb_server_datapack::loot::LootElementValue::Table(
        bbb_server_datapack::loot::parse_loot_table(&serde_json::json!({
            "type":"minecraft:gift",
            "pools":[{"rolls":1,
                "conditions":[{"condition":"minecraft:entity_properties","entity":"this","predicate":{"type":"minecraft:cat"}}],
                "entries":[{"type":"minecraft:item","name":"minecraft:rabbit_hide","functions":[
                    {"function":"minecraft:set_count","count":2},
                    {"function":"minecraft:set_components","components":{"minecraft:custom_data":{"morning_gift":"cat"}}}
                ]}]}]
        }), None).unwrap());
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "owner", 1);
    for (id, command) in [
        "gamerule spawn_mobs false",
        "gamerule players_sleeping_percentage 101",
        "gamerule mob_drops false",
        "time set midnight",
    ]
    .iter()
    .enumerate()
    {
        let tick = console_command(&sender, &receiver, &mut world, id as u64 + 40, command);
        assert!(console_command_answer(&tick).0.is_some(), "{command}");
    }
    let stone = crate::tests::block_state_id("minecraft:stone");
    let air = crate::tests::block_state_id("minecraft:air");
    for x in 2..=13 {
        for z in 5..=11 {
            for y in 64..=70 {
                world.set_block(
                    DIM,
                    BlockPos { x, y, z },
                    if y == 64 { stone } else { air },
                    2,
                );
            }
        }
    }
    for (x, part) in [(4, "foot"), (5, "head")] {
        let properties = [("facing", "east"), ("occupied", "false"), ("part", part)]
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect();
        let bed = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:red_bed",
            &properties,
        )
        .unwrap();
        world.set_block(DIM, BlockPos { x, y: 65, z: 8 }, bed, 2);
    }
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 5.5,
            y: 65.0,
            z: 7.5,
        },
    );
    let cat = owned_cat(&mut world, 8.5, false, "test:quiet");
    let uuid = world.stored_entity_for_test(DIM, cat).unwrap().uuid();
    let moved = console_command(
        &sender,
        &receiver,
        &mut world,
        44,
        &format!("teleport {uuid} 10.5 65 8.5"),
    );
    assert!(console_command_answer(&moved).0.is_some());
    let sleeping = click_chest(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 5, y: 65, z: 8 },
        1,
    );
    let relax_update = |tick: &TickResult, relaxing: bool| {
        tick.events.iter().any(|event| {
            matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
            if data.id == cat && data.values.iter().any(|value|
                value.data_id == VANILLA_CAT_RELAX_STATE_ONE_DATA_ID
                    && value.value == EntityDataValueKind::Boolean(relaxing)))
        })
    };
    let mut relaxed = relax_update(&sleeping, true);
    let mut lying = lying_update(&sleeping, cat, true);
    let mut purred = false;
    let mut relaxed_cleared = false;
    for _ in 0..400 {
        let tick = world.tick(receiver.drain());
        relaxed |= relax_update(&tick, true);
        relaxed_cleared |= relaxed && relax_update(&tick, false);
        lying |= lying_update(&tick, cat, true);
        purred |= tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
                if matches!(&sound.sound, bbb_protocol::packets::SoundEventHolder::Direct { location, fixed_range }
                    if location == "test:quiet_purr" && *fixed_range == Some(32.0))
                    && sound.source == bbb_protocol::packets::SoundSource::Neutral
                    && (0.2..=1.0).contains(&sound.volume) && sound.pitch == 1.0));
        if relaxed && lying && purred {
            break;
        }
    }
    assert!(
        relaxed && lying && purred,
        "relax={relaxed}, lying={lying}, purr={purred}, cat={:?}",
        world.stored_entity_for_test(DIM, cat).unwrap().position()
    );
    let position = world.stored_entity_for_test(DIM, cat).unwrap().position();
    assert!(
        (position.x - 5.5).hypot(position.z - 8.5) < 2.0,
        "cat reached the sleeping owner: {position:?}"
    );
    for _ in 0..100 {
        world.tick(receiver.drain());
    }
    let player_id = world.player_entity_id_for_test(ConnectionId(1));
    let woke = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerCommand(bbb_protocol::packets::PlayerCommand {
            entity_id: player_id,
            action: bbb_protocol::player_command_actions::PlayerCommandAction::StopSleeping,
            data: 0,
        }),
    );
    let custom_data = Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "morning_gift".into(),
        value: NbtSummaryValue::String("cat".into()),
    }]));
    let gift_sync = |tick: &TickResult| {
        tick.events.iter().any(|event| {
            let WorldEvent::Send {
                packet: ClientboundPlayPacket::SetEntityData(data),
                ..
            } = event
            else {
                return false;
            };
            data.values.iter().any(|value| {
                value.data_id == bbb_protocol::entity_synched_data::VANILLA_ITEM_ENTITY_ITEM_DATA_ID
                    && matches!(&value.value, EntityDataValueKind::ItemStack(stack)
                    if stack.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_RABBIT_HIDE_ID)
                        && stack.count == 2 && stack.component_patch.custom_data == custom_data)
            })
        })
    };
    let gift_stack = |world: &ServerWorld| {
        let items = world.stored_items_for_test(DIM);
        if items.len() != 1 {
            return None;
        }
        let EntityData::Item(item) = world.stored_entity_for_test(DIM, items[0].0)?.data() else {
            return None;
        };
        Some((item.item().clone(), item.pickup_delay()))
    };
    let mut native_gift = gift_stack(&world);
    let mut gift_synched = gift_sync(&woke);
    let mut cleared_lying = lying_update(&woke, cat, false);
    let mut cleared_relax = relaxed_cleared || relax_update(&woke, false);
    for _ in 0..40 {
        let tick = world.tick(receiver.drain());
        gift_synched |= gift_sync(&tick);
        native_gift = native_gift.or_else(|| gift_stack(&world));
        cleared_lying |= lying_update(&tick, cat, false);
        cleared_relax |= relax_update(&tick, false);
        if cleared_lying && cleared_relax && gift_synched && native_gift.is_some() {
            break;
        }
    }
    assert!(
        cleared_lying && cleared_relax,
        "waking clears cat resting poses"
    );
    assert!(gift_synched, "the actual gift stack reached the player");
    let (gift, pickup_delay) = native_gift.expect("one actual gift item");
    assert_eq!(gift.item_resource_id(), Some("minecraft:rabbit_hide"));
    assert_eq!(gift.count(), 2);
    assert_eq!(pickup_delay, 0);
    assert_eq!(gift.summary().component_patch.custom_data, custom_data);
}

#[test]
fn a_tempted_untamed_cat_begs_every_hundred_ticks() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.keep_alive_interval_ticks = 10_000;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "player", 1);
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        10,
        "gamerule spawn_mobs false",
    );
    assert!(console_command_answer(&tick).0.is_some());
    let stone = crate::tests::block_state_id("minecraft:stone");
    let air = crate::tests::block_state_id("minecraft:air");
    for x in 3..=14 {
        for y in 64..=67 {
            for z in 5..=12 {
                let state = if y == 64 { stone } else { air };
                world.set_block(DIM, BlockPos { x, y, z }, state, 2);
            }
        }
    }
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 12.5,
            y: 65.0,
            z: 8.5,
        },
    );
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        36,
        bbb_protocol::packets::ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_COD_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    let (cat, _) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_CAT_ID,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
        crate::entity::EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_CAT_ID).unwrap(),
        ),
    );
    let NbtSummaryValue::Compound(mut entries) =
        world.stored_entity_command_data_for_test(DIM, cat).unwrap()
    else {
        panic!("cat record");
    };
    entries.retain(|entry| entry.name != "sound_variant");
    entries.push(NbtSummaryEntry {
        name: "sound_variant".into(),
        value: NbtSummaryValue::String("minecraft:classic".into()),
    });
    world.load_stored_entity_record_for_test(DIM, cat, &entries);
    let mut begs = 0;
    for _ in 0..300 {
        let tick = world.tick(receiver.drain());
        begs += tick
            .events
            .iter()
            .filter(|event| matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
                    if matches!(&sound.sound, bbb_protocol::packets::SoundEventHolder::Reference { registry_id }
                        if vanilla_sound_event_resource_id(*registry_id) == Some("minecraft:entity.cat.beg_for_food"))))
            .count();
    }
    assert!((2..=3).contains(&begs), "begged {begs} times");
}
