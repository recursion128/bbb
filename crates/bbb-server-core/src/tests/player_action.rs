//! `ServerGamePacketListenerImpl.handlePlayerAction`'s actions that name no
//! block, sent the way the client sends them: `BlockPos.ZERO`, `Direction.DOWN`
//! and sequence zero.

use super::*;
use bbb_protocol::item_types::{
    VANILLA_ITEM_DIRT_ID, VANILLA_ITEM_SHIELD_ID, VANILLA_ITEM_STONE_ID,
};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_DROPPED_ID;
use bbb_server_types::stats::{custom_stat, StatKey};

/// Menu slot of the first hotbar slot and of the offhand in `InventoryMenu`.
const HOTBAR_MENU_SLOT: i16 = 36;
const OFFHAND_MENU_SLOT: i16 = 45;
/// `Inventory` index of the offhand.
const OFFHAND_INVENTORY_SLOT: usize = 40;

fn stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

fn hand_action(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    action: PlayerActionKind,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::PlayerAction(PlayerAction {
            action,
            pos: BlockPos { x: 0, y: 0, z: 0 },
            direction: Direction::Down,
            sequence: 0,
        }),
    )
}

fn world_with(game_type: GameType) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type,
        force_game_mode: true,
        ..datapack_config()
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    (sender, receiver, world)
}

fn assert_open(tick: &TickResult) {
    assert!(
        !tick
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::Close { .. })),
        "{tick:?}"
    );
    assert!(tick
        .snapshot
        .players
        .iter()
        .any(|player| player.connection_id == ConnectionId(1)));
}

fn alice_slots(tick: &TickResult) -> &[ItemStackSummary] {
    &tick
        .snapshot
        .players
        .iter()
        .find(|player| player.connection_id == ConnectionId(1))
        .expect("alice is still playing")
        .inventory
        .slots
}

fn slot_updates(tick: &TickResult) -> Vec<bbb_protocol::packets::ContainerSetSlot> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::ContainerSetSlot(update),
            } => Some(update.clone()),
            _ => None,
        })
        .collect()
}

/// The live item entities of the overworld as `(item id, count, thrower)`.
fn ground_items(world: &ServerWorld) -> Vec<(String, i32, Option<Uuid>)> {
    let dimension = Java26BuiltinDimension::Overworld;
    world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(dimension, id)?;
            let crate::entity::EntityData::Item(item) = entity.data() else {
                return None;
            };
            let stack = item.item();
            Some((
                stack.item_resource_id()?.to_string(),
                stack.count(),
                item.thrower(),
            ))
        })
        .collect()
}

fn dropped_stat(world: &ServerWorld, item_id: i32) -> i32 {
    world.player_stat_for_test(
        ConnectionId(1),
        StatKey::new(VANILLA_STAT_TYPE_ITEM_DROPPED_ID, item_id),
    )
}

fn drop_stat(world: &ServerWorld) -> i32 {
    world.player_stat_for_test(ConnectionId(1), custom_stat("minecraft:drop"))
}

/// `ServerPlayer.drop(false)` (`ServerPlayer.java:2064-2075`): one item leaves
/// the held stack as a thrown item entity, both drop stats are awarded, and
/// the remote slot is forced to what the client already predicted, so no slot
/// update comes back.
#[test]
fn player_action_drop_item_throws_one_item_out_of_the_held_stack() {
    let (sender, receiver, mut world) = world_with(GameType::Survival);
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        HOTBAR_MENU_SLOT,
        stack(VANILLA_ITEM_STONE_ID, 5),
    );

    let dropped = hand_action(&sender, &receiver, &mut world, PlayerActionKind::DropItem);

    assert_open(&dropped);
    assert_eq!(alice_slots(&dropped)[0], stack(VANILLA_ITEM_STONE_ID, 4));
    assert_eq!(slot_updates(&dropped), Vec::new());
    let uuid = dropped.snapshot.players[0].uuid;
    assert_eq!(
        ground_items(&world),
        vec![("minecraft:stone".to_string(), 1, Some(uuid))]
    );
    assert_eq!(dropped_stat(&world, VANILLA_ITEM_STONE_ID), 1);
    assert_eq!(drop_stat(&world), 1);
}

/// `ServerPlayer.drop(true)`: `Inventory.removeFromSelected(true)`
/// (`Inventory.java:520-523`) takes the whole stack, and `ITEM_DROPPED` counts
/// every item of it (`ServerPlayer.java:2008`) while `DROP` counts one.
#[test]
fn player_action_drop_all_items_throws_the_whole_held_stack() {
    let (sender, receiver, mut world) = world_with(GameType::Survival);
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        HOTBAR_MENU_SLOT,
        stack(VANILLA_ITEM_STONE_ID, 5),
    );

    let dropped = hand_action(
        &sender,
        &receiver,
        &mut world,
        PlayerActionKind::DropAllItems,
    );

    assert_open(&dropped);
    assert_eq!(alice_slots(&dropped)[0], ItemStackSummary::empty());
    assert_eq!(slot_updates(&dropped), Vec::new());
    let uuid = dropped.snapshot.players[0].uuid;
    assert_eq!(
        ground_items(&world),
        vec![("minecraft:stone".to_string(), 5, Some(uuid))]
    );
    assert_eq!(dropped_stat(&world, VANILLA_ITEM_STONE_ID), 5);
    assert_eq!(drop_stat(&world), 1);
}

/// `LivingEntity.drop` (`LivingEntity.java:758-761`) returns before an entity
/// for an empty stack, and `ServerPlayer.drop` awards nothing without one.
#[test]
fn player_action_drop_with_an_empty_hand_drops_nothing() {
    let (sender, receiver, mut world) = world_with(GameType::Survival);

    for action in [PlayerActionKind::DropItem, PlayerActionKind::DropAllItems] {
        let dropped = hand_action(&sender, &receiver, &mut world, action);
        assert_open(&dropped);
        assert_eq!(slot_updates(&dropped), Vec::new());
    }
    assert_eq!(ground_items(&world), Vec::new());
    assert_eq!(drop_stat(&world), 0);
}

/// The spectator refusals (`ServerGamePacketListenerImpl.java:1266`, `:1282`,
/// `:1291`, `:1297`): a spectator's hand actions change nothing.
#[test]
fn player_action_spectator_drops_and_swaps_nothing() {
    let (sender, receiver, mut world) = world_with(GameType::Spectator);
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        HOTBAR_MENU_SLOT,
        stack(VANILLA_ITEM_STONE_ID, 5),
    );
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        OFFHAND_MENU_SLOT,
        stack(VANILLA_ITEM_DIRT_ID, 3),
    );

    for action in [
        PlayerActionKind::DropItem,
        PlayerActionKind::DropAllItems,
        PlayerActionKind::SwapItemWithOffhand,
        PlayerActionKind::Stab,
        PlayerActionKind::ReleaseUseItem,
    ] {
        let tick = hand_action(&sender, &receiver, &mut world, action);
        assert_open(&tick);
        assert_eq!(slot_updates(&tick), Vec::new(), "{action:?}");
        let slots = alice_slots(&tick);
        assert_eq!(slots[0], stack(VANILLA_ITEM_STONE_ID, 5), "{action:?}");
        assert_eq!(
            slots[OFFHAND_INVENTORY_SLOT],
            stack(VANILLA_ITEM_DIRT_ID, 3),
            "{action:?}"
        );
    }
    assert_eq!(ground_items(&world), Vec::new());
}

/// Creative has no arm of its own in `handlePlayerAction`: the held stack
/// shrinks as in survival, and the creative drop throttle
/// (`ServerGamePacketListenerImpl.java:2016-2022`) belongs to
/// `SetCreativeModeSlot` alone.
#[test]
fn player_action_creative_drop_shrinks_the_stack_and_ignores_the_creative_throttle() {
    let (sender, receiver, mut world) = world_with(GameType::Creative);
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        HOTBAR_MENU_SLOT,
        stack(VANILLA_ITEM_STONE_ID, 5),
    );
    world.set_drop_spam_count_for_test(ConnectionId(1), 1480);

    let dropped = hand_action(&sender, &receiver, &mut world, PlayerActionKind::DropItem);

    assert_open(&dropped);
    assert_eq!(alice_slots(&dropped)[0], stack(VANILLA_ITEM_STONE_ID, 4));
    assert_eq!(ground_items(&world).len(), 1);
    assert_eq!(dropped_stat(&world, VANILLA_ITEM_STONE_ID), 1);
}

/// `AbstractContainerMenu.findSlot` (`AbstractContainerMenu.java:808-817`)
/// answers the open chest menu's own index for the held slot, so the remote
/// slot forced there is the one the chest menu broadcasts.
#[test]
fn player_action_drop_with_a_chest_open_sends_no_slot_update() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let chest = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:chest",
        &BTreeMap::from([
            ("facing".to_string(), "north".to_string()),
            ("type".to_string(), "single".to_string()),
            ("waterlogged".to_string(), "false".to_string()),
        ]),
    )
    .expect("minecraft:chest is a vanilla 26.1 block");
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        chest,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert!(opened.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::OpenScreen(_),
            ..
        }
    )));

    let dropped = hand_action(&sender, &receiver, &mut world, PlayerActionKind::DropItem);

    assert_open(&dropped);
    assert_eq!(alice_slots(&dropped)[0].count, 63);
    assert_eq!(slot_updates(&dropped), Vec::new());
    assert_eq!(ground_items(&world).len(), 1);
}

/// `ServerGamePacketListenerImpl.java:1283-1285`: the client does not predict
/// the swap, so the next `broadcastChanges` reports both hand slots.
#[test]
fn player_action_swap_exchanges_the_hands_and_reports_both_slots() {
    let (sender, receiver, mut world) = world_with(GameType::Survival);
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        HOTBAR_MENU_SLOT,
        stack(VANILLA_ITEM_STONE_ID, 5),
    );
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        OFFHAND_MENU_SLOT,
        stack(VANILLA_ITEM_DIRT_ID, 3),
    );

    let swapped = hand_action(
        &sender,
        &receiver,
        &mut world,
        PlayerActionKind::SwapItemWithOffhand,
    );

    assert_open(&swapped);
    let slots = alice_slots(&swapped);
    assert_eq!(slots[0], stack(VANILLA_ITEM_DIRT_ID, 3));
    assert_eq!(
        slots[OFFHAND_INVENTORY_SLOT],
        stack(VANILLA_ITEM_STONE_ID, 5)
    );
    let updates = slot_updates(&swapped);
    assert_eq!(
        updates
            .iter()
            .map(|update| (update.container_id, update.slot, update.item.clone()))
            .collect::<Vec<_>>(),
        vec![
            (0, HOTBAR_MENU_SLOT, stack(VANILLA_ITEM_DIRT_ID, 3)),
            (0, OFFHAND_MENU_SLOT, stack(VANILLA_ITEM_STONE_ID, 5)),
        ]
    );
    assert_eq!(sounds_sent(&swapped), Vec::new());
    assert_eq!(ground_items(&world), Vec::new());
}

/// `LivingEntity.onEquipItem` (`LivingEntity.java:666-690`): a shield's
/// `Equippable` names the offhand, so swapping it there plays its equip sound
/// to every player in range; swapping it back into the main hand does not.
#[test]
fn player_action_swapping_a_shield_into_the_offhand_plays_its_equip_sound() {
    let (sender, receiver, mut world) = world_with(GameType::Survival);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        HOTBAR_MENU_SLOT,
        stack(VANILLA_ITEM_SHIELD_ID, 1),
    );

    let equipped = hand_action(
        &sender,
        &receiver,
        &mut world,
        PlayerActionKind::SwapItemWithOffhand,
    );

    let equip_sound = SoundEventHolder::Reference {
        registry_id: vanilla_sound_event_registry_id("minecraft:item.armor.equip_generic")
            .expect("a registered sound"),
    };
    let heard = equipped
        .events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .filter_map(|(connection_id, packet)| match packet {
            ClientboundPlayPacket::Sound(sound) => {
                Some((connection_id, sound.sound.clone(), sound.source))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(heard.len(), 2, "{heard:?}");
    for connection in [ConnectionId(1), ConnectionId(2)] {
        assert!(
            heard.contains(&(connection, equip_sound.clone(), SoundSource::Players)),
            "{heard:?}"
        );
    }
    assert_open(&equipped);
    assert_eq!(
        alice_slots(&equipped)[OFFHAND_INVENTORY_SLOT],
        stack(VANILLA_ITEM_SHIELD_ID, 1)
    );

    let unequipped = hand_action(
        &sender,
        &receiver,
        &mut world,
        PlayerActionKind::SwapItemWithOffhand,
    );
    assert_eq!(sounds_sent(&unequipped), Vec::new());
}

/// Neither action closes the connection or touches the inventory:
/// `releaseUsingItem` with nothing in use is `stopUsingItem` alone
/// (`LivingEntity.java:3537-3548`), and a stab without a kinetic owner acts on
/// nothing.
#[test]
fn player_action_release_use_item_and_stab_keep_the_connection() {
    let (sender, receiver, mut world) = world_with(GameType::Survival);
    world.stock_menu_slot_for_test(
        ConnectionId(1),
        HOTBAR_MENU_SLOT,
        stack(VANILLA_ITEM_STONE_ID, 5),
    );

    for action in [PlayerActionKind::ReleaseUseItem, PlayerActionKind::Stab] {
        let tick = hand_action(&sender, &receiver, &mut world, action);
        assert_open(&tick);
        assert_eq!(slot_updates(&tick), Vec::new(), "{action:?}");
        assert_eq!(
            alice_slots(&tick)[0],
            stack(VANILLA_ITEM_STONE_ID, 5),
            "{action:?}"
        );
    }
    assert_eq!(ground_items(&world), Vec::new());
}

/// An inline `equip_sound` naming no registered sound still plays, as a
/// direct sound event (`LivingEntity.onEquipItem`).
#[test]
fn a_custom_equip_sound_plays_as_a_direct_event() {
    let (sender, receiver, mut world) = world_with(GameType::Survival);
    let equippable = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "slot".into(),
            value: NbtSummaryValue::String("offhand".into()),
        },
        NbtSummaryEntry {
            name: "equip_sound".into(),
            value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "sound_id".into(),
                value: NbtSummaryValue::String("test:clank".into()),
            }]),
        },
    ]);
    let shield = bbb_server_types::item_stack::RuntimeItemStack::from_summary(stack(
        VANILLA_ITEM_SHIELD_ID,
        1,
    ))
    .with_component_nbt(
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
        Some(&equippable),
        crate::world::registry_runtime::item_stack_resolver_for_test(),
    )
    .unwrap();
    world.stock_menu_slot_for_test(ConnectionId(1), HOTBAR_MENU_SLOT, shield.summary().clone());

    let equipped = hand_action(
        &sender,
        &receiver,
        &mut world,
        PlayerActionKind::SwapItemWithOffhand,
    );

    assert!(equipped.events.iter().any(|event| matches!(event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Sound(sound),
            ..
        } if matches!(&sound.sound, SoundEventHolder::Direct { location, .. } if location == "test:clank"))));
}
