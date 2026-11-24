use super::*;
use crate::world::WorldCommand;
use bbb_facts::container_inputs::ContainerInput;
use bbb_protocol::packets::{
    ContainerClick, HashedStack, ItemEnchantmentSummary, ItemStackSummary, PlayServerbound,
};
use std::collections::BTreeMap;

const PLAYER: ConnectionId = ConnectionId(1);
// InventoryMenu.java: the head armor slot follows the 2x2 grid and its result.
const HEAD_SLOT: i16 = 5;

fn click_head_slot(
    world: &mut ServerWorld,
    sender: &crate::mailbox::WorldCommandSender,
    receiver: &crate::mailbox::WorldCommandReceiver,
) {
    let state_id = world.levels[PLAYER].inventory.state_id();
    sender
        .try_send(WorldCommand::Play {
            connection_id: PLAYER,
            packet: PlayServerbound::ContainerClick(ContainerClick {
                container_id: 0,
                state_id,
                slot_num: HEAD_SLOT,
                button_num: 0,
                input: ContainerInput::Pickup,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::empty(),
            }),
        })
        .unwrap();
    world.tick(receiver.drain());
}

/// `ArmorSlot.mayPickup` (ArmorSlot.java:55-60): a survival player cannot take
/// off a helmet cursed with binding; a creative one can.
#[test]
fn a_binding_cursed_helmet_stays_on_in_survival_and_comes_off_in_creative() {
    let (sender, receiver) = crate::tests::mailbox();
    let mut world = ServerWorld::new(crate::world::ServerWorldConfig {
        game_type: bbb_protocol::game_types::GameType::Survival,
        ..crate::tests::datapack_config()
    })
    .unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let binding = world
        .datapack_tables
        .enchantment_slots
        .iter()
        .position(|enchantment| enchantment.resource_id == "minecraft:binding_curse")
        .unwrap();
    let mut helmet = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_IRON_HELMET_ID),
        count: 1,
        component_patch: Default::default(),
    };
    let patch = &mut helmet.component_patch;
    patch.enchantments = vec![ItemEnchantmentSummary {
        holder_id: i32::try_from(binding).unwrap(),
        level: 1,
    }];
    patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
    world.stock_menu_slot_for_test(PLAYER, HEAD_SLOT, helmet.clone());

    click_head_slot(&mut world, &sender, &receiver);
    let inventory = &world.levels[PLAYER].inventory;
    assert!(inventory.carried().is_empty());
    assert_eq!(inventory.slot_item(HEAD_SLOT).summary(), &helmet);

    world.levels[PLAYER].abilities.instabuild = true;
    click_head_slot(&mut world, &sender, &receiver);
    let inventory = &world.levels[PLAYER].inventory;
    assert_eq!(inventory.carried().summary(), &helmet);
    assert!(inventory.slot_item(HEAD_SLOT).is_empty());
}

/// `ArmorSlot.setByPlayer` (ArmorSlot.java:34-37) runs `onEquipItem`, so a
/// helmet clicked into the head slot plays its equip sound to everyone nearby
/// and raises `GameEvent.EQUIP`.
#[test]
fn clicking_a_helmet_into_the_head_slot_plays_the_equip_sound() {
    let (sender, receiver) = crate::tests::mailbox();
    let mut world = ServerWorld::new(crate::world::ServerWorldConfig {
        game_type: bbb_protocol::game_types::GameType::Survival,
        ..crate::tests::datapack_config()
    })
    .unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let helmet = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_IRON_HELMET_ID),
        count: 1,
        component_patch: Default::default(),
    };
    // The first hotbar slot of `InventoryMenu`.
    world.stock_menu_slot_for_test(PLAYER, 36, helmet.clone());
    let click = |world: &mut ServerWorld, slot_num| {
        let state_id = world.levels[PLAYER].inventory.state_id();
        sender
            .try_send(WorldCommand::Play {
                connection_id: PLAYER,
                packet: PlayServerbound::ContainerClick(ContainerClick {
                    container_id: 0,
                    state_id,
                    slot_num,
                    button_num: 0,
                    input: ContainerInput::Pickup,
                    changed_slots: BTreeMap::new(),
                    carried_item: HashedStack::empty(),
                }),
            })
            .unwrap();
        world.tick(receiver.drain())
    };
    click(&mut world, 36);
    let events_before = world
        .game_events(bbb_server_types::save::Java26BuiltinDimension::Overworld)
        .1;
    let tick = click(&mut world, HEAD_SLOT);
    assert_eq!(
        world.levels[PLAYER]
            .inventory
            .slot_item(HEAD_SLOT)
            .summary(),
        &helmet
    );
    let equip = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:item.armor.equip_iron",
    )
    .unwrap();
    let mut heard = tick
        .events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .filter_map(|(connection_id, packet)| match packet {
            crate::connection::ClientboundPlayPacket::Sound(sound)
                if sound.sound
                    == bbb_protocol::packets::SoundEventHolder::Reference {
                        registry_id: equip,
                    } =>
            {
                Some(connection_id)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    heard.sort();
    assert_eq!(heard, vec![PLAYER, ConnectionId(2)]);
    let (recorded, count) =
        world.game_events(bbb_server_types::save::Java26BuiltinDimension::Overworld);
    assert!(count > events_before);
    assert!(recorded
        .iter()
        .any(|event| event.game_event == crate::level_event::VanillaGameEvent::EQUIP));
}

fn admin_command(
    world: &mut ServerWorld,
    sender: &crate::mailbox::WorldCommandSender,
    receiver: &crate::mailbox::WorldCommandReceiver,
    command: &str,
) {
    use crate::admin::{AdminEnvelope, AdminRequest, AdminScope};
    sender
        .try_send(WorldCommand::Admin {
            envelope: AdminEnvelope {
                operation_id: 2,
                request_id: uuid::Uuid::new_v4().to_string(),
                principal: "fixture".into(),
                scope: AdminScope::Command,
                deadline_tick: u64::MAX,
                now: 0,
            },
            request: AdminRequest::ExecuteCommand {
                command: command.to_string(),
            },
        })
        .unwrap();
    let result = world.tick(receiver.drain());
    assert!(result
        .events
        .iter()
        .any(|event| matches!(event, WorldEvent::Admin(outcome) if outcome.result.is_ok())));
}

/// RandomizableContainerBlockEntity.java:79 refuses a spectator an unlooted
/// chest, and ServerPlayer.java:1330-1333 says so on the action bar.
#[test]
fn a_spectator_is_told_it_cannot_open_an_unlooted_chest() {
    use bbb_protocol::packets::{BlockHitResult, Direction, InteractionHand, UseItemOn};
    let (sender, receiver) = crate::tests::mailbox();
    let mut world = ServerWorld::new(crate::world::ServerWorldConfig {
        game_type: bbb_protocol::game_types::GameType::Survival,
        ..crate::tests::datapack_config()
    })
    .unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(
        PLAYER,
        Vec3d {
            x: 0.5,
            y: 101.0,
            z: 0.5,
        },
    );
    let chest = BlockPos { x: 2, y: 101, z: 0 };
    admin_command(
        &mut world,
        &sender,
        &receiver,
        "setblock 2 101 0 minecraft:chest{LootTable:\"minecraft:chests/simple_dungeon\"}",
    );
    world.levels[PLAYER].game_type = GameType::Spectator;
    sender
        .try_send(WorldCommand::Play {
            connection_id: PLAYER,
            packet: PlayServerbound::UseItemOn(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: BlockHitResult {
                    pos: chest,
                    direction: Direction::Up,
                    cursor_x: 0.5,
                    cursor_y: 0.5,
                    cursor_z: 0.5,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 1,
            }),
        })
        .unwrap();
    let tick = world.tick(receiver.drain());
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SystemChat(chat),
            ..
        } if chat.overlay && format!("{:?}", chat.content).contains("container.spectatorCantOpen")
    )));
    assert!(!tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::OpenScreen(_),
            ..
        }
    )));
}

/// Hotbar-key swaps reach `ArmorSlot.setByPlayer` (`AbstractContainerMenu.java:472-511`),
/// and a click that empties an armor slot hands `setByPlayer` the removed
/// stack (`Slot.java:110`), so both raise the equip game events.
#[test]
fn hotbar_swaps_and_clicks_on_the_head_slot_raise_equip_and_unequip() {
    use crate::level_event::VanillaGameEvent;
    let dimension = bbb_server_types::save::Java26BuiltinDimension::Overworld;
    let (sender, receiver) = crate::tests::mailbox();
    let mut world = ServerWorld::new(crate::world::ServerWorldConfig {
        game_type: bbb_protocol::game_types::GameType::Survival,
        ..crate::tests::datapack_config()
    })
    .unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let helmet = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_IRON_HELMET_ID),
        count: 1,
        component_patch: Default::default(),
    };
    world.stock_menu_slot_for_test(PLAYER, 36, helmet.clone());
    let equip = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:item.armor.equip_iron",
    )
    .unwrap();
    let click = |world: &mut ServerWorld, button_num, input| {
        let before = world.game_events(dimension).1;
        let state_id = world.levels[PLAYER].inventory.state_id();
        sender
            .try_send(WorldCommand::Play {
                connection_id: PLAYER,
                packet: PlayServerbound::ContainerClick(ContainerClick {
                    container_id: 0,
                    state_id,
                    slot_num: HEAD_SLOT,
                    button_num,
                    input,
                    changed_slots: BTreeMap::new(),
                    carried_item: HashedStack::empty(),
                }),
            })
            .unwrap();
        let tick = world.tick(receiver.drain());
        let heard = tick.events.iter().any(|event| {
            matches!(event, WorldEvent::Send {
                packet: crate::connection::ClientboundPlayPacket::Sound(sound), ..
            } if sound.sound
                == bbb_protocol::packets::SoundEventHolder::Reference { registry_id: equip })
        });
        let (recorded, count) = world.game_events(dimension);
        let fresh = usize::try_from(count - before).unwrap();
        let raised = recorded[recorded.len() - fresh.min(recorded.len())..]
            .iter()
            .map(|event| event.game_event)
            .filter(|event| [VanillaGameEvent::EQUIP, VanillaGameEvent::UNEQUIP].contains(event))
            .collect::<Vec<_>>();
        (heard, raised)
    };
    assert_eq!(
        click(&mut world, 0, ContainerInput::Swap),
        (true, vec![VanillaGameEvent::EQUIP])
    );
    assert_eq!(
        world.levels[PLAYER]
            .inventory
            .slot_item(HEAD_SLOT)
            .summary(),
        &helmet
    );
    assert_eq!(
        click(&mut world, 0, ContainerInput::Swap),
        (false, vec![VanillaGameEvent::UNEQUIP])
    );
    click(&mut world, 0, ContainerInput::Swap);
    assert_eq!(
        click(&mut world, 0, ContainerInput::Pickup),
        (false, vec![VanillaGameEvent::UNEQUIP])
    );
    assert!(world.levels[PLAYER]
        .inventory
        .slot_item(HEAD_SLOT)
        .is_empty());
}
