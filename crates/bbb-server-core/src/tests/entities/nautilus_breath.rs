use super::horse_interaction::{hold, horse_world, worn};
use super::nautilus_interaction::tame_with_buckets;
use super::saddled_mounts::{mount, spawn_beside};
use super::*;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::PlayerInput;

const BREATH: &str = "minecraft:breath_of_the_nautilus";

fn answer_keep_alives(sender: &WorldCommandSender, events: &[WorldEvent]) {
    for event in events {
        if let WorldEvent::Send {
            connection_id,
            packet: ClientboundPlayPacket::KeepAlive { id },
        } = event
        {
            enqueue(
                sender,
                WorldCommand::Play {
                    connection_id: *connection_id,
                    packet: PlayServerbound::KeepAlive { id: *id },
                },
            )
            .unwrap();
        }
    }
}

#[test]
fn both_unsaddled_nautiluses_refresh_their_riders_breath_until_dismount() {
    let effect_id = bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id(BREATH)
        .unwrap()
        .id;
    for kind in [
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    ] {
        let (sender, receiver, mut world) = horse_world();
        let nautilus = spawn_beside(&mut world, kind);
        tame_with_buckets(&sender, &receiver, &mut world, nautilus);
        hold(&mut world, "minecraft:air", 0);
        assert_eq!(worn(&world, nautilus, EquipmentSlot::Saddle), None);
        mount(&sender, &receiver, &mut world, nautilus);
        assert!(world
            .player_active_effect_for_test(ConnectionId(1), BREATH)
            .is_some());
        let rider = world.player_entity_id_for_test(ConnectionId(1));
        let mut refreshed = false;
        for _ in 0..100 {
            let tick = world.tick(receiver.drain());
            answer_keep_alives(&sender, &tick.events);
            for event in &tick.events {
                if let WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::UpdateMobEffect(update),
                } = event
                {
                    if update.entity_id == rider && update.effect_id == effect_id {
                        assert_eq!(update.amplifier, 0);
                        assert_eq!(update.duration_ticks, 60);
                        assert!(update.flags.ambient);
                        assert!(update.flags.visible);
                        assert!(update.flags.show_icon);
                        refreshed = true;
                    }
                }
            }
            assert!(world
                .player_active_effect_for_test(ConnectionId(1), BREATH)
                .is_some());
        }
        assert!(refreshed, "the rider receives refreshed breath packets");
        let dismount = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PlayerInput(PlayerInput {
                shift: true,
                ..PlayerInput::default()
            }),
        );
        answer_keep_alives(&sender, &dismount.events);
        assert_eq!(
            world
                .external_player_relation_in_dimension_for_test(
                    Java26BuiltinDimension::Overworld,
                    rider,
                )
                .0,
            None
        );
        let mut removed = false;
        for _ in 0..60 {
            let tick = world.tick(receiver.drain());
            answer_keep_alives(&sender, &tick.events);
            removed |= tick.events.iter().any(|event| {
                matches!(event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::RemoveMobEffect(remove),
                } if remove.entity_id == rider && remove.effect_id == effect_id)
            });
        }
        assert!(removed, "the dismounted rider receives breath removal");
        assert!(world
            .player_active_effect_for_test(ConnectionId(1), BREATH)
            .is_none());
    }
}
