use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::{ServerWorldConfig, WorldCommand};
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID;
use bbb_protocol::item_component_value_types::ItemRaritySummary;
use bbb_protocol::item_types::VANILLA_ITEM_SHIELD_ID;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::packets::{BlockPos, Direction, PlayServerbound, PlayerAction, SoundEventHolder};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;

#[test]
fn player_action_hand_swap_preserves_inventory_and_suppresses_equivalent_equip_sound() {
    let player_id = ConnectionId(1);
    for (silent, case) in [true, false].into_iter().flat_map(|silent| {
        [
            "empty",
            "same_default",
            "different_rarity",
            "removed_rarity",
        ]
        .map(|case| (silent, case))
    }) {
        let mut world = ServerWorld::new(ServerWorldConfig {
            game_type: GameType::Survival,
            ..Default::default()
        })
        .unwrap();
        let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
        crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
        let shield = ItemStackSummary {
            item_id: Some(VANILLA_ITEM_SHIELD_ID),
            count: 1,
            component_patch: Default::default(),
        };
        let mut offhand = shield.clone();
        match case {
            "empty" => offhand = ItemStackSummary::empty(),
            "removed_rarity" => {
                offhand.component_patch.removed_type_ids =
                    vec![VANILLA_DATA_COMPONENT_RARITY_TYPE_ID];
            }
            _ => {
                offhand.component_patch.added_type_ids =
                    vec![VANILLA_DATA_COMPONENT_RARITY_TYPE_ID];
                offhand.component_patch.rarity = Some(if case == "same_default" {
                    ItemRaritySummary::Common
                } else {
                    ItemRaritySummary::Uncommon
                });
            }
        }
        world.stock_menu_slot_for_test(player_id, 36, shield.clone());
        world.levels[player_id]
            .inventory
            .set_item_in_hand(InteractionHand::OffHand, offhand.clone());
        let player = world.levels.player_mut(player_id).unwrap();
        player.set_silent(silent);
        for equips in [true, false] {
            sender
                .try_send(WorldCommand::Play {
                    connection_id: player_id,
                    packet: PlayServerbound::PlayerAction(PlayerAction {
                        action: PlayerActionKind::SwapItemWithOffhand,
                        pos: BlockPos { x: 0, y: 0, z: 0 },
                        direction: Direction::Down,
                        sequence: 0,
                    }),
                })
                .unwrap();
            let tick = world.tick(receiver.drain());
            let player = world.levels.player_at(player_id);
            assert_eq!(
                player
                    .inventory
                    .off_hand_item()
                    .expect("the offhand fixture projects"),
                if equips {
                    shield.clone()
                } else {
                    offhand.clone()
                },
                "{silent} {case}"
            );
            assert_eq!(
                player
                    .inventory
                    .selected_item()
                    .expect("the mainhand fixture projects"),
                if equips {
                    offhand.clone()
                } else {
                    shield.clone()
                },
                "{silent} {case}"
            );
            let sounds = tick
                .events
                .iter()
                .flat_map(WorldEvent::deliveries)
                .filter_map(|(connection_id, packet)| match packet {
                    ClientboundPlayPacket::Sound(sound)
                        if sound.sound
                            == SoundEventHolder::Reference {
                                registry_id: vanilla_sound_event_registry_id(
                                    "minecraft:item.armor.equip_generic",
                                )
                                .unwrap(),
                            } =>
                    {
                        Some((connection_id, sound.seed))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            if !silent && case != "same_default" && (equips || case != "empty") {
                assert_eq!(sounds.len(), 2);
                let seed = sounds[0].1;
                for recipient in [player_id, ConnectionId(2)] {
                    assert!(sounds.contains(&(recipient, seed)));
                }
            } else {
                assert!(sounds.is_empty());
            }
        }
    }
}
