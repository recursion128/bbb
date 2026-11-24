//! What a menu hands back or tells others when it is removed, and the checks
//! in front of opening one.

use super::*;
use bbb_protocol::item_types::{
    VANILLA_ITEM_DIRT_ID, VANILLA_ITEM_GOLD_INGOT_ID, VANILLA_ITEM_STONE_ID,
};

fn stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

fn click(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    slot_num: i16,
    input: ContainerInput,
) -> TickResult {
    let inventory = world.player_inventory_for_test(ConnectionId(1));
    let (container_id, state_id) = (inventory.container_id(), inventory.state_id());
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id,
            state_id,
            slot_num,
            button_num: 0,
            input,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    )
}

fn disconnect(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
) -> TickResult {
    enqueue(
        sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(connection_id),
        },
    )
    .unwrap();
    world.tick(receiver.drain())
}

/// `Player.remove` (`Player.java:1244-1250`) on a disconnect: the inventory
/// menu's cursor and 2x2 grid are dropped (`AbstractContainerMenu.java:598-606`).
#[test]
fn a_disconnect_drops_the_cursor_and_the_crafting_grid() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.stock_menu_slot_for_test(ConnectionId(1), 36, stack(VANILLA_ITEM_STONE_ID, 5));
    world.stock_menu_slot_for_test(ConnectionId(1), 37, stack(VANILLA_ITEM_DIRT_ID, 3));
    click(&sender, &receiver, &mut world, 36, ContainerInput::Pickup);
    click(&sender, &receiver, &mut world, 1, ContainerInput::Pickup);
    click(&sender, &receiver, &mut world, 37, ContainerInput::Pickup);

    disconnect(&sender, &receiver, &mut world, 1);
    let mut dropped = dropped_ground_items(&world, Java26BuiltinDimension::Overworld)
        .into_iter()
        .map(|(id, count, _)| (id, count))
        .collect::<Vec<_>>();
    dropped.sort();
    assert_eq!(
        dropped,
        vec![
            ("minecraft:dirt".to_string(), 3),
            ("minecraft:stone".to_string(), 5),
        ]
    );
}

/// `ShulkerBoxMenu.removed` -> `stopOpen` (`ShulkerBoxBlockEntity.java:167-194`)
/// runs when the viewer disconnects: the lid closes with its sound.
#[test]
fn a_disconnecting_viewer_closes_the_shulker_box() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        shulker_box_state("up"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert!(opened_screen(&opened).is_some());

    let left = disconnect(&sender, &receiver, &mut world, 1);
    let close = SoundEventHolder::Reference {
        registry_id: vanilla_sound_event_registry_id("minecraft:block.shulker_box.close")
            .expect("a registered sound"),
    };
    assert!(left.events.iter().any(|event| matches!(
        event.packet_for(ConnectionId(2)),
        Some(ClientboundPlayPacket::Sound(sound)) if sound.sound == close
    )));
    let shulker = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|block_entity| block_entity.shulker_box())
        .expect("the shulker box is still standing");
    assert_eq!(shulker.open_count(), 0);
}

/// `LecternMenu` (`LecternMenu.java:22-36`) carries the page as a data slot,
/// so a page turn reaches every reader through `broadcastChanges`.
#[test]
fn a_second_lectern_reader_sees_the_turned_page() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        lectern_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.stock_menu_slot_for_test(ConnectionId(1), 36, two_page_writable_book());
    right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert_eq!(lectern_has_book(&mut world, at), Some("true"));
    right_click(&sender, &receiver, &mut world, 1, at, 2);
    let opened = right_click(&sender, &receiver, &mut world, 2, at, 1);
    let bob_container = opened_screen(&opened)
        .expect("bob opens the lectern")
        .container_id;

    let turned = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerButtonClick(ContainerButtonClick {
            container_id: 1,
            button_id: 2,
        }),
    );
    let next = world.tick(receiver.drain());
    let bob_saw_page = |tick: &TickResult| {
        tick.events.iter().any(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(2),
                    packet: ClientboundPlayPacket::ContainerSetData(data),
                } if data.container_id == bob_container && data.id == 0 && data.value == 1
            )
        })
    };
    assert!(bob_saw_page(&turned) || bob_saw_page(&next));
}

fn oak_slab(kind: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:oak_slab",
        &BTreeMap::from([
            ("type".to_string(), kind.to_string()),
            ("waterlogged".to_string(), "false".to_string()),
        ]),
    )
    .expect("minecraft:oak_slab is a vanilla 26.1 block")
}

/// `ShulkerBoxBlock.canOpen` (`ShulkerBoxBlock.java:88-95`) tests only the
/// lower half of the cell above for collision, entities included.
#[test]
fn a_shulker_lid_opens_under_a_top_slab_but_not_into_a_boat() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let above = BlockPos { x: 0, y: 65, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        shulker_box_state("up"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        above,
        oak_slab("top"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert!(opened_screen(&opened).is_some());

    let other = BlockPos { x: 2, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        other,
        shulker_box_state("up"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        Vec3d {
            x: 2.5,
            y: 65.0,
            z: 1.5,
        },
        crate::entity::EntityData::Boat(crate::entity::BoatData::new()),
    );
    let missed = right_click(&sender, &receiver, &mut world, 1, other, 2);
    assert!(opened_screen(&missed).is_none());
}

/// A double-click that collects furnace results into the cursor runs
/// `FurnaceResultSlot.onTake` through `safeTake`
/// (`AbstractContainerMenu.java:535-557`), so `ITEM_CRAFTED` counts them.
#[test]
fn collecting_furnace_results_with_a_double_click_counts_them_as_crafted() {
    use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_CRAFTED_ID;
    use bbb_server_types::stats::StatKey;

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        furnace_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.furnace_mut())
        .expect("the furnace block entity is live")
        .set_items(
            crate::inventory::test_item_resolver(),
            &[
                crate::tests::native_stack(ItemStackSummary::empty()),
                crate::tests::native_stack(ItemStackSummary::empty()),
                crate::tests::native_stack(stack(VANILLA_ITEM_GOLD_INGOT_ID, 3)),
            ],
            200,
        )
        .expect("the fixture furnace items are accepted");
    world.stock_menu_slot_for_test(ConnectionId(1), 36, stack(VANILLA_ITEM_GOLD_INGOT_ID, 1));
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert!(opened_screen(&opened).is_some());
    // The furnace menu's first hotbar slot.
    click(&sender, &receiver, &mut world, 30, ContainerInput::Pickup);
    click(
        &sender,
        &receiver,
        &mut world,
        30,
        ContainerInput::PickupAll,
    );

    assert_eq!(
        world
            .player_inventory_for_test(ConnectionId(1))
            .carried()
            .count(),
        4
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
                VANILLA_ITEM_GOLD_INGOT_ID
            )
        ),
        3
    );
}
