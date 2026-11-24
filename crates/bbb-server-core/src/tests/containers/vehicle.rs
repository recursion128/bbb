use super::*;
use crate::entity::{item_slots, ChestBoatData, MinecartContainerData};
use crate::tests::entities::console_command_answer;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
    VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
};
use bbb_protocol::menu_types::{VANILLA_MENU_TYPE_GENERIC_9X3_ID, VANILLA_MENU_TYPE_HOPPER_ID};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn interaction(id: i32, secondary: bool) -> PlayServerbound {
    PlayServerbound::Interact(InteractEntity {
        entity_id: id,
        hand: InteractionHand::MainHand,
        location: Vec3d::default(),
        using_secondary_action: secondary,
    })
}

fn click(
    world: &mut ServerWorld,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    slot: i16,
    input: ContainerInput,
) -> TickResult {
    let inventory = world.player_inventory_for_test(ConnectionId(1));
    let packet = ContainerClick {
        container_id: inventory.container_id(),
        state_id: inventory.state_id(),
        slot_num: slot,
        button_num: 0,
        input,
        changed_slots: BTreeMap::new(),
        carried_item: HashedStack::empty(),
    };
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::ContainerClick(packet),
    )
}

fn slots(world: &ServerWorld, id: i32) -> Vec<bbb_server_types::item_stack::RuntimeItemStack> {
    let entity = world.stored_entity_for_test(DIM, id).unwrap();
    (0..item_slots::container_size(entity).unwrap())
        .map(|slot| item_slots::read(entity, slot as i32).unwrap().unwrap())
        .collect()
}

fn closed(tick: &TickResult) -> bool {
    tick.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerClose(_),
                ..
            }
        )
    })
}

#[test]
fn vehicle_menus_click_shift_share_and_save_complete_stacks_in_all_three_layouts() {
    let root = std::env::temp_dir().join(format!("bbb-vehicle-menus-{}", Uuid::new_v4()));
    let storage = created_fixture_save(&root, "bbb-vehicle-menus");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let config = operator_config(&[("alice", 1)]);
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.tick_rate_mut().set_frozen(true);
    let position = world.publication().players[0].position;
    let mut expected = Vec::new();
    for (kind, data, size, menu_type) in [
        (
            VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
            EntityData::ChestMinecart(MinecartContainerData::new_chest()),
            27,
            VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        ),
        (
            VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
            EntityData::HopperMinecart(MinecartContainerData::new_hopper()),
            5,
            VANILLA_MENU_TYPE_HOPPER_ID,
        ),
        (
            VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
            EntityData::ChestBoat(ChestBoatData::new()),
            27,
            VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        ),
    ] {
        chat_command(&sender, &receiver, &mut world, 1,
            "item replace entity alice weapon.mainhand with diamond_sword[minecraft:enchantments={\"minecraft:sharpness\":3},minecraft:custom_data={cargo:42}]");
        let stack = world
            .player_inventory_for_test(ConnectionId(1))
            .selected_stack()
            .clone();
        assert!(!stack.is_empty());
        assert!(!stack.summary().component_patch.enchantments.is_empty());
        let (id, uuid) = world.spawn_stored_entity_for_test(DIM, kind, position, data);
        let opened = play(&sender, &receiver, &mut world, 1, interaction(id, true));
        let screen = opened_screen(&opened).expect("vehicle opens its screen");
        assert_eq!(screen.menu_type_id, menu_type);
        assert_eq!(
            container_contents(&opened)[0].items.len(),
            size as usize + 36
        );
        assert!(world
            .player_inventory_for_test(ConnectionId(1))
            .open_container_anchor()
            .is_none());
        assert!(opened_screen(&play(
            &sender,
            &receiver,
            &mut world,
            2,
            interaction(id, true)
        ))
        .is_some());
        click(
            &mut world,
            &sender,
            &receiver,
            size + 27,
            ContainerInput::Pickup,
        );
        click(&mut world, &sender, &receiver, 0, ContainerInput::Pickup);
        assert_eq!(slots(&world, id)[0], stack);
        assert_eq!(
            world
                .player_inventory_for_test(ConnectionId(2))
                .container_items()[0],
            stack
        );
        click(&mut world, &sender, &receiver, 0, ContainerInput::QuickMove);
        assert!(slots(&world, id).iter().all(|item| item.is_empty()));
        click(
            &mut world,
            &sender,
            &receiver,
            size + 35,
            ContainerInput::QuickMove,
        );
        assert_eq!(slots(&world, id)[0], stack);
        for connection in [1, 2] {
            let container_id = world
                .player_inventory_for_test(ConnectionId(connection))
                .container_id();
            play(
                &sender,
                &receiver,
                &mut world,
                connection,
                PlayServerbound::ContainerClose(ContainerCloseRequest { container_id }),
            );
        }
        expected.push((uuid, stack));
    }
    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let reloaded = spawn_pinned_restore(config, &candidate);
    for (uuid, stack) in expected {
        let id = reloaded
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .find(|&id| reloaded.stored_entity_for_test(DIM, id).unwrap().uuid() == uuid)
            .unwrap();
        assert_eq!(slots(&reloaded, id)[0], stack);
    }
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn chest_boat_rides_opens_with_secondary_and_mounted_inventory_and_closes_on_invalid_vehicle() {
    let config = operator_config(&[("alice", 1)]);
    let mut world = spawn_pinned_world(config.clone());
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick_rate_mut().set_frozen(true);
    let position = world.publication().players[0].position;
    let uuid = Uuid::from_u128(4422);
    let summoned = console_command(
        &sender, &receiver, &mut world, 4422,
        &format!("summon minecraft:oak_chest_boat {} {} {} {{UUID:[I;0,0,0,4422],Items:[{{Slot:0b,id:\"minecraft:diamond\",count:3}}]}}", position.x, position.y, position.z),
    );
    assert_eq!(console_command_answer(&summoned).0, Some(1));
    let boat = world.live_entity_id_for_uuid_for_test(DIM, uuid).unwrap();
    let cargo = slots(&world, boat)[0].clone();
    assert_eq!(cargo.summary().count, 3);
    assert_eq!(
        cargo.summary().item_id,
        Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:diamond")
                .unwrap()
                .id
        )
    );
    world.set_player_boarding_cooldown_for_test(ConnectionId(1), 20);
    let failed = play(&sender, &receiver, &mut world, 1, interaction(boat, false));
    assert!(
        opened_screen(&failed).is_none(),
        "failed riding does not open an available seat"
    );
    world.set_player_boarding_cooldown_for_test(ConnectionId(1), 0);
    let ridden = play(&sender, &receiver, &mut world, 1, interaction(boat, false));
    assert!(opened_screen(&ridden).is_none());
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(boat)
    );
    let teleport_id = ridden
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .expect("boarding positions the player");
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: teleport_id },
    );
    let rider = world.player_entity_id_for_test(ConnectionId(1));
    let opened = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: rider,
            action: PlayerCommandAction::OpenInventory,
            data: 0,
        }),
    );
    assert_eq!(
        opened_screen(&opened).unwrap().menu_type_id,
        VANILLA_MENU_TYPE_GENERIC_9X3_ID
    );
    assert_eq!(slots(&world, boat)[0], cargo);
    world.detach_external_player_for_test(ConnectionId(1));
    let request = world
        .save_request(1_786_000_001_442, 203, RegionCompression::Zlib)
        .unwrap();
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    crate::tests::entities::restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(config, &candidate);
    restored.ensure_entity_transfer_full_chunk_for_test(
        DIM,
        (
            position.x.floor() as i32 >> 4,
            position.z.floor() as i32 >> 4,
        ),
    );
    let restored_boat = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    assert_eq!(slots(&restored, restored_boat)[0], cargo);
    let far = Vec3d {
        x: position.x + 30.0,
        ..position
    };
    world.set_player_position_for_test(ConnectionId(1), far);
    assert!(closed(&world.tick(receiver.drain())));
    world.set_player_position_for_test(ConnectionId(1), position);
    assert!(opened_screen(&play(
        &sender,
        &receiver,
        &mut world,
        1,
        interaction(boat, true)
    ))
    .is_some());
    let removed = chat_command(&sender, &receiver, &mut world, 1, &format!("kill {uuid}"));
    assert!(closed(&removed));
}

#[test]
fn vehicle_pending_loot_rejects_spectators_and_uses_the_opening_player_and_luck() {
    const TABLE: &str = "minecraft:chests/igloo_chest";
    let [x, y, z] = ServerWorld::fixture_spawn_block();
    let mut config = datapack_config_with_loot_table(
        TABLE,
        &serde_json::json!({
            "type": "minecraft:chest", "pools": [{ "rolls": 1, "bonus_rolls": 1,
                "conditions": [{"condition": "minecraft:weather_check", "raining": true, "thundering": false},
                    {"condition": "minecraft:entity_properties", "entity": "this",
                    "predicate": {"type": "minecraft:player", "movement": {
                        "x": {"min": 4.0, "max": 6.0}
                    }}},
                    {"condition": "minecraft:location_check", "offsetX": 1,
                        "predicate": {"block": {"nbt": format!(
                            "{{id:'minecraft:chest',x:{},y:{y},z:{z},Items:[{{Slot:0b,id:'minecraft:diamond',count:3}}]}}", x + 1
                        )}}}],
                "entries": [{"type": "minecraft:item", "name": "minecraft:diamond"}]},
                {"rolls": 1, "conditions": [{
                    "condition": "minecraft:weather_check", "raining": false, "thundering": false
                }], "entries": [{"type": "minecraft:item", "name": "minecraft:stone"}]}]
        }),
    );
    config.game_type = GameType::Spectator;
    let mut world = spawn_pinned_world(config);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let rain = console_command(&sender, &receiver, &mut world, 1, "weather rain 100000t");
    assert_eq!(console_command_answer(&rain).0, Some(100_000));
    for _ in 0..100 {
        world.tick(receiver.drain());
    }
    assert!(world.level_weather(DIM).is_raining());
    assert!(!world.level_weather(DIM).is_thundering());
    world.tick_rate_mut().set_frozen(true);
    for command in [
        format!("setblock {} {y} {z} minecraft:chest", x + 1),
        format!(
            "item replace block {} {y} {z} container.0 with minecraft:diamond 3",
            x + 1
        ),
    ] {
        let setup = console_command(&sender, &receiver, &mut world, 1, &command);
        assert_eq!(console_command_answer(&setup).0, Some(1));
    }
    let position = world.publication().players[0].position;
    let mut data = MinecartContainerData::new_chest();
    data.set_loot_table(TABLE.into(), 31).unwrap();
    let (id, _) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        position,
        EntityData::ChestMinecart(data),
    );
    assert!(opened_screen(&play(
        &sender,
        &receiver,
        &mut world,
        1,
        interaction(id, true)
    ))
    .is_none());
    assert!(item_slots::container_has_loot(
        world.stored_entity_for_test(DIM, id).unwrap()
    ));
    let abilities = world.player_abilities_for_test(ConnectionId(1));
    world.set_player_mode_and_abilities_for_test(ConnectionId(1), GameType::Survival, abilities);
    world.set_player_attribute_base_for_test(
        ConnectionId(1),
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_LUCK_ID,
        2.0,
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: position.x + 0.25,
                ..position
            }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    let opened = play(&sender, &receiver, &mut world, 1, interaction(id, true));
    assert!(opened_screen(&opened).is_some());
    assert!(!item_slots::container_has_loot(
        world.stored_entity_for_test(DIM, id).unwrap()
    ));
    assert_eq!(
        slots(&world, id)
            .iter()
            .map(|item| item.count())
            .sum::<i32>(),
        3
    );
    assert!(slots(&world, id)
        .iter()
        .filter(|item| !item.is_empty())
        .all(|item| { item.item_resource_id() == Some("minecraft:diamond") }));
}
