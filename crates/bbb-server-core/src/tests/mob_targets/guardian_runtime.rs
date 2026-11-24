use super::melee::{fixture, ALICE, PLAYER_POSITION};
use super::*;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID, VANILLA_ENTITY_TYPE_GUARDIAN_ID,
};
use bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID;

#[test]
fn an_elder_guardian_applies_fatigue_and_displays_its_curse_on_real_ticks() {
    let (sender, receiver, mut world) = fixture();
    let id = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID,
        Vec3d {
            y: 90.0,
            ..PLAYER_POSITION
        },
    )
    .0;
    world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_no_gravity(true);
    let mut displayed = false;
    for _ in 0..1300 {
        let tick = world.tick(receiver.drain());
        for event in &tick.events {
            if let WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::KeepAlive { id },
            } = event
            {
                enqueue(
                    &sender,
                    WorldCommand::Play {
                        connection_id: *connection_id,
                        packet: PlayServerbound::KeepAlive { id: *id },
                    },
                )
                .unwrap();
            }
        }
        displayed |= tick.events.iter().any(|event| matches!(event, WorldEvent::Send {
            connection_id: ALICE,
            packet: ClientboundPlayPacket::GameEvent(event),
        } if event.event_id == VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID && event.param == 1.0));
        if displayed {
            break;
        }
    }
    assert!(
        displayed,
        "elder: {:?}",
        world.stored_entity_for_test(DIMENSION, id).map(|entity| (
            entity.tick_count(),
            entity.position(),
            entity.mob_home()
        ))
    );
    let effect = world
        .player_active_effect_for_test(ALICE, "minecraft:mining_fatigue")
        .unwrap();
    assert_eq!(effect.amplifier(), 2);
    assert!(effect.duration() > 5900 && effect.duration() <= 6000);
}

#[test]
fn only_elder_guardians_are_persistent_by_default_beyond_despawn_range() {
    let (_sender, _receiver, mut world) = fixture();
    let position = Vec3d {
        y: 250.0,
        ..PLAYER_POSITION
    };
    let guardian = spawn(&mut world, VANILLA_ENTITY_TYPE_GUARDIAN_ID, position).0;
    let elder = spawn(&mut world, VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID, position).0;
    world.tick(Vec::new());
    assert!(world.stored_entity_for_test(DIMENSION, guardian).is_none());
    assert!(world.stored_entity_for_test(DIMENSION, elder).is_some());
}
