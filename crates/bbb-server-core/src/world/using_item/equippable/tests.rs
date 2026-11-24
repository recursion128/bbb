use super::*;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::level_event::VanillaGameEvent;
use crate::mailbox::{
    world_command_mailbox, WorldCommandReceiver, WorldCommandSender, WorldMailboxLimits,
};
use crate::world::{ServerWorldConfig, TickResult, WorldCommand};
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::game_types::GameType;
use bbb_protocol::item_component_value_types::ItemRaritySummary;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::{
    InteractionHand, ItemEnchantmentSummary, ItemStackSummary, MovePlayer, PlayServerbound, UseItem,
};

const PLAYER: ConnectionId = ConnectionId(1);

fn stack(resource: &str, count: i32) -> RuntimeItemStack {
    ItemStackSummary {
        item_id: Some(vanilla_item_type_for_resource_id(resource).unwrap().id),
        count,
        component_patch: Default::default(),
    }
    .into()
}

fn fixture(game_type: GameType) -> (ServerWorld, WorldCommandSender, WorldCommandReceiver) {
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type,
        ..crate::tests::datapack_config()
    })
    .unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.reset_pending_chunks_for_test(PLAYER, &[]);
    world.reset_pending_chunks_for_test(ConnectionId(2), &[]);
    for connection_id in [PLAYER, ConnectionId(2)] {
        sender
            .try_send(WorldCommand::Play {
                connection_id,
                packet: PlayServerbound::MovePlayer(MovePlayer {
                    position: None,
                    rotation: None,
                    on_ground: true,
                    horizontal_collision: false,
                }),
            })
            .unwrap();
        world.tick(receiver.drain());
    }
    (world, sender, receiver)
}

fn use_item(
    world: &mut ServerWorld,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    hand: InteractionHand,
) -> TickResult {
    sender
        .try_send(WorldCommand::Play {
            connection_id: PLAYER,
            packet: PlayServerbound::UseItem(UseItem {
                hand,
                sequence: 1,
                y_rot: 0.0,
                x_rot: 0.0,
            }),
        })
        .unwrap();
    world.tick(receiver.drain())
}

fn custom_equippable(
    held: RuntimeItemStack,
    edit: impl FnOnce(&mut bbb_protocol::packets::data_components::EquippableSummary),
) -> RuntimeItemStack {
    let mut component = equippable(&held).unwrap().unwrap().clone();
    edit(&mut component);
    let mut summary = held.summary().clone();
    summary
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID);
    summary.component_patch.equippable = Some(component);
    summary.into()
}

#[test]
fn air_use_equips_and_swaps_both_hands_with_components_and_peer_updates() {
    for game_type in [GameType::Survival, GameType::Creative] {
        for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
            let (mut world, sender, receiver) = fixture(game_type);
            let held = custom_equippable(stack("minecraft:diamond_helmet", 1), |component| {
                component.asset_id = Some("bbb:helmet".into())
            });
            world.levels[PLAYER]
                .inventory
                .set_item_in_hand(hand, held.clone());
            let tick = use_item(&mut world, &sender, &receiver, hand);
            let player = world.levels.player_at(PLAYER);
            assert_eq!(player.inventory.slot(39), Some(&held));
            let expected_hand = if game_type == GameType::Creative {
                held.clone()
            } else {
                RuntimeItemStack::empty()
            };
            assert_eq!(runtime_item_in_hand(player, hand), &expected_hand);
            assert_eq!(
                player.cold.stats.value(StatKey::new(
                    VANILLA_STAT_TYPE_ITEM_USED_ID,
                    held.item_id().unwrap()
                )),
                1
            );
            assert!(tick.events.iter().any(|event| matches!(
                event.packet_for(ConnectionId(2)),
                Some(ClientboundPlayPacket::SetEquipment(update))
                    if update.slots.iter().any(|slot| slot.slot == EquipmentSlot::Head
                        && slot.item == *held.summary())
            )));
            assert!(tick
                .events
                .iter()
                .any(|event| matches!(event.sent_packet(), Some(ClientboundPlayPacket::Sound(_)))));
            let dimension = player.dimension;
            assert!(world
                .level_by_id(&dimension)
                .event_sink
                .recent_game_events()
                .any(|event| event.game_event == VanillaGameEvent::EQUIP));
            let replacement = stack("minecraft:iron_helmet", 1);
            world.levels[PLAYER]
                .inventory
                .set_item_in_hand(hand, replacement.clone());
            use_item(&mut world, &sender, &receiver, hand);
            let player = world.levels.player_at(PLAYER);
            assert_eq!(player.inventory.slot(39), Some(&replacement));
            assert_eq!(runtime_item_in_hand(player, hand), &held);
            let request = world
                .save_request(
                    1_786_000_002_000,
                    202,
                    bbb_server_persistence::RegionCompression::Zlib,
                )
                .unwrap();
            let patch = &request.player_patches[0];
            let document = bbb_server_persistence::created_player_document(
                patch,
                request.item_stack_resolver.as_dyn(),
            )
            .unwrap();
            let parsed =
                bbb_server_persistence::Java26PlayerData::parse(document, patch.uuid).unwrap();
            let equipment = parsed.equipment_input().unwrap();
            let head = bbb_protocol::nbt_summary::value_field(equipment, "head").unwrap();
            assert_eq!(
                RuntimeItemStack::try_read(head, request.item_stack_resolver.as_dyn()).unwrap(),
                replacement
            );
            let saved_hand = if hand == InteractionHand::MainHand {
                let Some(bbb_protocol::NbtSummaryValue::List(items)) = parsed.inventory_input()
                else {
                    panic!("saved inventory list");
                };
                items
                    .iter()
                    .find(|value| {
                        bbb_protocol::nbt_summary::value_field(value, "Slot")
                            == Some(&bbb_protocol::NbtSummaryValue::Byte(0))
                    })
                    .unwrap()
            } else {
                bbb_protocol::nbt_summary::value_field(equipment, "offhand").unwrap()
            };
            assert_eq!(
                RuntimeItemStack::try_read(saved_hand, request.item_stack_resolver.as_dyn())
                    .unwrap(),
                held
            );
        }
    }
    for slot in [
        EquipmentSlot::OffHand,
        EquipmentSlot::Body,
        EquipmentSlot::Saddle,
    ] {
        let (mut world, sender, receiver) = fixture(GameType::Survival);
        let mapping =
            bbb_protocol::equipment_slots::vanilla_equipment_slot_by_ordinal(slot as u8).unwrap();
        let held = custom_equippable(stack("minecraft:diamond_helmet", 1), |component| {
            component.slot_id = mapping.id
        });
        world.levels[PLAYER]
            .inventory
            .set_item_in_hand(InteractionHand::MainHand, held.clone());
        let tick = use_item(&mut world, &sender, &receiver, InteractionHand::MainHand);
        let player = world.levels.player_at(PLAYER);
        assert_eq!(
            player
                .inventory
                .slot(mapping.player_inventory_slot.unwrap() as usize),
            Some(&held)
        );
        assert!(runtime_item_in_hand(player, InteractionHand::MainHand).is_empty());
        assert!(tick.events.iter().any(|event| matches!(event, WorldEvent::Send { connection_id: ConnectionId(2), packet: ClientboundPlayPacket::SetEquipment(update) } if update.slots.iter().any(|update| update.slot == slot && update.item == *held.summary()))));
        assert!(!world
            .level_by_id(&player.dimension)
            .event_sink
            .recent_game_events()
            .any(|event| event.game_event == VanillaGameEvent::EQUIP));
    }
}

#[test]
fn stacked_equipment_displaces_into_inventory_or_drops_when_full() {
    for full in [false, true] {
        let (mut world, sender, receiver) = fixture(GameType::Survival);
        let old = stack("minecraft:iron_helmet", 1);
        let held = custom_equippable(stack("minecraft:carved_pumpkin", 3), |component| {
            component.swappable = true
        });
        let inventory = &mut world.levels[PLAYER].inventory;
        if full {
            for index in 0..36 {
                inventory.set_inventory_slot(index, stack("minecraft:stone", 64));
            }
        }
        inventory.set_inventory_slot(39, old.clone());
        inventory.set_item_in_hand(InteractionHand::MainHand, held.clone());
        use_item(&mut world, &sender, &receiver, InteractionHand::MainHand);
        let player = world.levels.player_at(PLAYER);
        assert_eq!(player.inventory.slot(39), Some(&held.with_count(1)));
        assert_eq!(
            runtime_item_in_hand(player, InteractionHand::MainHand),
            &held.with_count(2)
        );
        assert_eq!(
            (0..36).any(|index| player.inventory.slot(index) == Some(&old)),
            !full
        );
        let entities = &world.level_by_id(&player.dimension).entities;
        assert_eq!(entities.ids().into_iter().any(|id| matches!(entities.get(id).unwrap().data(), crate::entity::EntityData::Item(item) if item.item() == &old)), full);
    }
}

#[test]
fn equip_rejections_and_creative_armor_change() {
    for case in [
        "same",
        "same_default",
        "creative_same_default",
        "different_rarity",
        "removed_rarity",
        "unswappable",
        "disallowed",
        "binding",
        "creative_binding",
    ] {
        let creative = matches!(case, "creative_binding" | "creative_same_default");
        let swaps = matches!(
            case,
            "creative_binding" | "different_rarity" | "removed_rarity"
        );
        let (mut world, sender, receiver) = fixture(if creative {
            GameType::Creative
        } else {
            GameType::Survival
        });
        let mut old = stack("minecraft:iron_helmet", 1);
        let mut held = stack("minecraft:diamond_helmet", 1);
        match case {
            "same" => held = old.clone(),
            "same_default" | "creative_same_default" | "different_rarity" | "removed_rarity" => {
                let mut summary = old.summary().clone();
                if case == "removed_rarity" {
                    summary.component_patch.removed_type_ids =
                        vec![VANILLA_DATA_COMPONENT_RARITY_TYPE_ID];
                } else {
                    summary.component_patch.added_type_ids =
                        vec![VANILLA_DATA_COMPONENT_RARITY_TYPE_ID];
                    summary.component_patch.rarity = Some(if case == "different_rarity" {
                        ItemRaritySummary::Uncommon
                    } else {
                        ItemRaritySummary::Common
                    });
                }
                held = summary.into();
            }
            "unswappable" => {
                held = custom_equippable(held, |component| component.swappable = false)
            }
            "disallowed" => {
                held = custom_equippable(held, |component| {
                    component.allowed_entities = Some(Default::default())
                })
            }
            _ => {
                let registry = world
                    .datapacks()
                    .unwrap()
                    .registries
                    .registry("minecraft:enchantment")
                    .unwrap();
                let holder_id = registry
                    .elements
                    .iter()
                    .position(|element| element.id == "minecraft:binding_curse")
                    .unwrap() as i32;
                let mut summary = old.summary().clone();
                summary.component_patch.added_type_ids =
                    vec![VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID];
                summary.component_patch.enchantments = vec![ItemEnchantmentSummary {
                    holder_id,
                    level: 1,
                }];
                old = summary.into();
            }
        }
        let inventory = &mut world.levels[PLAYER].inventory;
        inventory.set_inventory_slot(39, old.clone());
        inventory.set_item_in_hand(InteractionHand::MainHand, held.clone());
        use_item(&mut world, &sender, &receiver, InteractionHand::MainHand);
        let player = world.levels.player_at(PLAYER);
        assert_eq!(
            player.inventory.slot(39),
            Some(if swaps { &held } else { &old }),
            "{case}"
        );
        assert_eq!(
            runtime_item_in_hand(player, InteractionHand::MainHand),
            if swaps { &old } else { &held },
            "{case}"
        );
        assert_eq!(
            player.cold.stats.value(StatKey::new(
                VANILLA_STAT_TYPE_ITEM_USED_ID,
                held.item_id().unwrap()
            )),
            i32::from(swaps),
            "{case}"
        );
    }
}
