//! A hurt baby hoglin retreats instead of attacking, including across a NoAI pause.

use super::melee::{fixture, health, ALICE, PLAYER_POSITION};
use super::*;
use crate::tests::entities::console_command;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HOGLIN_ID;

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let operation = world.game_time() as u64 + 10000;
    let tick = console_command(sender, receiver, world, operation, text);
    let outcome = admin_outcome(&tick);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{text}: {report:?}");
    tick
}

fn tick(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> TickResult {
    let tick = world.tick(receiver.drain());
    for event in &tick.events {
        if let WorldEvent::Send {
            connection_id: ALICE,
            packet: ClientboundPlayPacket::KeepAlive { id },
        } = event
        {
            enqueue(
                sender,
                WorldCommand::Play {
                    connection_id: ALICE,
                    packet: PlayServerbound::KeepAlive { id: *id },
                },
            )
            .unwrap();
        }
    }
    tick
}

fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .position()
}

fn separation(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}

fn retreat_speed(world: &ServerWorld, id: i32) -> bool {
    let living = world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap();
    (f64::from(living.speed()) - 1.3 * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
        .abs()
        < 1.0e-5
}

#[test]
fn a_hurt_baby_hoglin_retreats_pauses_with_no_ai_and_eventually_attacks_again() {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        "forceload add -128 -64 48 64",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "fill -128 64 -64 48 64 64 minecraft:stone",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "attribute alice minecraft:knockback_resistance base set 1",
    );
    let initial = Vec3d {
        x: PLAYER_POSITION.x - 5.0,
        ..PLAYER_POSITION
    };
    let (hoglin, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_HOGLIN_ID, initial);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{Age:-24000,IsImmuneToZombification:1b}}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:knockback_resistance base set 1"),
    );
    world.tick_rate_mut().set_frozen(false);
    let initial_health = health(&world);
    let mut attacked = false;
    for _ in 0..200 {
        tick(&sender, &receiver, &mut world);
        if health(&world) < initial_health {
            attacked = true;
            break;
        }
    }
    assert!(
        attacked,
        "the baby first approaches and attacks a survival player"
    );
    assert!(position(&world, hoglin).x > initial.x + 2.0);
    let before_hurt = position(&world, hoglin);
    let hurt = command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {uuid} 1 minecraft:player_attack by alice"),
    );
    let mut heard_retreat = false;
    let retreat_sound = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:entity.hoglin.retreat",
    )
    .unwrap();
    let hears_retreat = |tick: &TickResult| {
        tick.events.iter().any(|event| {
            matches!(event,
        WorldEvent::Send { connection_id: ALICE, packet: ClientboundPlayPacket::Sound(sound), .. }
        if sound.sound == SoundEventHolder::Reference { registry_id: retreat_sound }
        && sound.volume == 1.0 && (1.3..=1.7).contains(&sound.pitch))
        })
    };
    heard_retreat |= hears_retreat(&hurt);
    let mut last_health = health(&world);
    let mut ran_at_speed = false;
    for _ in 0..35 {
        let output = tick(&sender, &receiver, &mut world);
        heard_retreat |= hears_retreat(&output);
        ran_at_speed |= retreat_speed(&world, hoglin);
        let now = health(&world);
        assert!(
            now >= last_health,
            "the retreating baby stops hurting its attacker"
        );
        last_health = now;
    }
    assert!(
        separation(position(&world, hoglin), PLAYER_POSITION)
            > separation(before_hurt, PLAYER_POSITION) + 3.0,
        "movement with full knockback resistance proves active retreat"
    );
    assert!(
        ran_at_speed && heard_retreat,
        "the baby runs at its retreat speed and sounds afraid"
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    for _ in 0..450 {
        tick(&sender, &receiver, &mut world);
    }
    let resumed_at = position(&world, hoglin);
    let player = Vec3d {
        x: resumed_at.x + 1.0,
        ..resumed_at
    };
    world.set_player_position_for_test(ALICE, player);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    last_health = health(&world);
    ran_at_speed = false;
    for _ in 0..30 {
        tick(&sender, &receiver, &mut world);
        ran_at_speed |= retreat_speed(&world, hoglin);
        let now = health(&world);
        assert!(
            now >= last_health,
            "NoAI pauses the retreat timer and command loading preserves it"
        );
        last_health = now;
    }
    assert!(
        ran_at_speed && separation(position(&world, hoglin), player) > 3.0,
        "the resumed baby still retreats from the nearby attacker"
    );
    let mut attacked_again = false;
    for _ in 0..400 {
        tick(&sender, &receiver, &mut world);
        let now = health(&world);
        if now < last_health {
            attacked_again = true;
            break;
        }
        last_health = now;
    }
    if !attacked_again {
        let nearby = position(&world, hoglin);
        world.set_player_position_for_test(
            ALICE,
            Vec3d {
                x: nearby.x + 1.0,
                ..nearby
            },
        );
        // Expiry and ordinary target acquisition have separate observable budgets.
        for _ in 0..200 {
            tick(&sender, &receiver, &mut world);
            let now = health(&world);
            if now < last_health {
                attacked_again = true;
                break;
            }
            last_health = now;
        }
    }
    assert!(
        attacked_again,
        "after retreat expiry the baby attacks again: mob={:?}, player={:?}, health={}, target={:?}",
        position(&world, hoglin),
        world.publication().players.first().map(|player| player.position),
        health(&world),
        target_of(&world, hoglin),
    );
}
