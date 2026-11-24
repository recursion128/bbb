use super::*;

#[test]
fn ender_chest_menu_transfers_personal_originals_and_rechecks_openers() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let dimension = Java26BuiltinDimension::Overworld;
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_ENDER_CHEST_ID,
    )
    .unwrap();
    assert!(world.set_block(dimension, at, state, SET_BLOCK_UPDATE_ALL));
    world.set_ender_chest_slot_for_test(
        ConnectionId(1),
        7,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 5,
            component_patch: Default::default(),
        },
    );
    let respawned = world
        .player_inventory_for_test(ConnectionId(1))
        .respawned(false);
    assert_eq!(respawned.ender_chest_slots()[7].count(), 5);
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let screen = opened_screen(&opened).expect("the personal container opens");
    assert_eq!(
        screen.menu_type_id,
        bbb_protocol::menu_types::VANILLA_MENU_TYPE_GENERIC_9X3_ID
    );
    let contents = container_contents(&opened);
    assert_eq!(contents[0].items.len(), 63);
    assert_eq!(contents[0].items[7].count, 5);
    assert_eq!(sounds_sent(&opened).len(), 1);
    let raised = world
        .game_events(dimension)
        .0
        .into_iter()
        .find(|event| {
            event.game_event == crate::level_event::VanillaGameEvent::CONTAINER_OPEN
                && event.pos == at
        })
        .expect("opening raises its game event");
    assert_eq!(
        raised.source_entity_id,
        Some(world.player_entity_id_for_test(ConnectionId(1)))
    );
    assert_eq!(
        world
            .level_block_entity_for_test(dimension, at)
            .unwrap()
            .ender_chest()
            .unwrap()
            .open_count(),
        1
    );
    for _ in 0..6 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        world
            .level_block_entity_for_test(dimension, at)
            .unwrap()
            .ender_chest()
            .unwrap()
            .open_count(),
        1
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: screen.container_id,
            state_id: contents[0].state_id,
            slot_num: 7,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    assert_eq!(world.ender_chest_slot_for_test(ConnectionId(1), 7).count, 0);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: screen.container_id,
            state_id: contents[0].state_id,
            slot_num: 8,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    assert_eq!(world.ender_chest_slot_for_test(ConnectionId(1), 8).count, 5);
    assert_eq!(
        world
            .player_inventory_for_test(ConnectionId(1))
            .ender_chest_slots()[8]
            .count(),
        5
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity alice enderchest.8 with dirt 3",
    );
    let menu = &world.player_inventory_for_test(ConnectionId(1));
    assert_eq!(menu.slot_item(8).count(), 3);
    assert_eq!(
        menu.slot_item(8).item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_DIRT_ID)
    );
    assert_eq!(menu.container_items()[8], menu.ender_chest_slots()[8]);
    let closed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest {
            container_id: screen.container_id,
        }),
    );
    assert_eq!(sounds_sent(&closed).len(), 1);
    let raised = world
        .game_events(dimension)
        .0
        .into_iter()
        .find(|event| {
            event.game_event == crate::level_event::VanillaGameEvent::CONTAINER_CLOSE
                && event.pos == at
        })
        .expect("closing raises its game event");
    assert_eq!(
        raised.source_entity_id,
        Some(world.player_entity_id_for_test(ConnectionId(1)))
    );
    assert_eq!(
        world
            .level_block_entity_for_test(dimension, at)
            .unwrap()
            .ender_chest()
            .unwrap()
            .open_count(),
        0
    );
    let reopened = right_click(&sender, &receiver, &mut world, 1, at, 2);
    let reopened_screen = opened_screen(&reopened).expect("personal menu reopens");
    assert_eq!(
        world
            .player_inventory_for_test(ConnectionId(1))
            .container_id(),
        reopened_screen.container_id
    );
    assert!(world
        .player_inventory_for_test(ConnectionId(1))
        .is_ender_chest_menu());
    assert_eq!(
        world
            .get_block_entity(dimension, at)
            .unwrap()
            .ender_chest()
            .unwrap()
            .open_count(),
        1
    );
    // Cross-dimension teleport runs `oldLevel.removePlayerImmediately`
    // (`ServerPlayer.java:1108`) -> `Player.remove` (`Player.java:1244-1250`),
    // closing the open ender chest menu against the Overworld chest immediately.
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "execute in minecraft:the_nether run tp alice 0 64 0",
    );
    assert_eq!(
        world.player_dimension(ConnectionId(1)),
        Java26BuiltinDimension::Nether
    );
    assert_eq!(
        world
            .player_inventory_for_test(ConnectionId(1))
            .container_id(),
        0
    );
    assert!(!world
        .player_inventory_for_test(ConnectionId(1))
        .is_ender_chest_menu());
    assert!(world
        .player_inventory_for_test(ConnectionId(1))
        .active_ender_chest()
        .is_none());
    assert_eq!(
        world
            .get_block_entity(dimension, at)
            .unwrap()
            .ender_chest()
            .unwrap()
            .open_count(),
        0
    );
    // A late client close packet for the already-closed menu must remain a no-op.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest {
            container_id: reopened_screen.container_id,
        }),
    );
    assert!(world
        .player_inventory_for_test(ConnectionId(1))
        .active_ender_chest()
        .is_none());
    assert_eq!(
        world
            .get_block_entity(dimension, at)
            .unwrap()
            .ender_chest()
            .unwrap()
            .open_count(),
        0
    );
}
