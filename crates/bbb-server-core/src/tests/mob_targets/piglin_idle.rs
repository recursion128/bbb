//! Unhurt piglin peer interactions, wandering and distant loved-player attention.

use super::melee::{fixture, ALICE};
use super::*;
use crate::tests::entities::console_command;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID,
};

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let operation = world.game_time() as u64 + 10000;
    let output = console_command(sender, receiver, world, operation, text);
    let outcome = admin_outcome(&output);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{text}: {report:?}");
    keep_alive(sender, &output);
    output
}
fn keep_alive(sender: &WorldCommandSender, output: &TickResult) {
    for event in &output.events {
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
}
fn tick(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> TickResult {
    let output = world.tick(receiver.drain());
    keep_alive(sender, &output);
    output
}
fn mob(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    kind: i32,
    x: f64,
    z: f64,
    flags: &str,
) -> (i32, Uuid) {
    let (id, uuid) = spawn(world, kind, Vec3d { x, y: 65.0, z });
    command(
        sender,
        receiver,
        world,
        &format!("data merge entity {uuid} {{IsImmuneToZombification:1b,{flags}}}"),
    );
    command(
        sender,
        receiver,
        world,
        &format!("attribute {uuid} minecraft:knockback_resistance base set 1"),
    );
    (id, uuid)
}
fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .position()
}
fn health(world: &ServerWorld, id: i32) -> f32 {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
        .health()
}
fn separation(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}

fn arena(spectator: bool) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "forceload add -32 -16 48 32",
        "fill -32 64 -16 48 64 32 minecraft:stone",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    if spectator {
        command(&sender, &receiver, &mut world, "gamemode spectator alice");
    }
    (sender, receiver, world)
}
fn speed_matches(body: &crate::entity::Entity) -> bool {
    let living = body.living().unwrap();
    (f64::from(living.speed()) - 0.6 * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
        .abs()
        < 1.0e-5
}
fn looks_at(body: &crate::entity::Entity, target: Vec3d, eye_height: f64) -> bool {
    let from = body.position();
    let yaw = (target.z - from.z).atan2(target.x - from.x).to_degrees() as f32 - 90.0;
    let error = (body.y_head_rot() - yaw + 180.0).rem_euclid(360.0) - 180.0;
    let pitch = (from.y + f64::from(body.eye_height()) - target.y - eye_height)
        .atan2(separation(from, target))
        .to_degrees() as f32;
    error.abs() < 5.0 && (body.x_rot() - pitch).abs() < 3.0
}
fn settled(body: &crate::entity::Entity) -> bool {
    let delta = body.delta_movement();
    body.on_ground() && delta.x.hypot(delta.z) < 0.01
}

#[test]
fn an_unhurt_piglin_looks_and_walks_to_a_peer_then_strolls_alone() {
    let (sender, receiver, mut world) = arena(true);
    let (piglin, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        5.5,
        8.5,
        "CannotHunt:1b,NoAI:0b",
    );
    let (peer, peer_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        11.5,
        8.5,
        "CannotHunt:1b,NoAI:1b",
    );
    let initial = position(&world, piglin);
    let hp = health(&world, piglin);
    let peer_hp = health(&world, peer);
    let mut walked = false;
    let mut looked = 0;
    let mut approached = false;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..900 {
        tick(&sender, &receiver, &mut world);
        let body = world.stored_entity_for_test(DIMENSION, piglin).unwrap();
        let target = world.stored_entity_for_test(DIMENSION, peer).unwrap();
        assert_eq!(health(&world, piglin), hp);
        assert_eq!(health(&world, peer), peer_hp);
        walked |= speed_matches(body) && body.position().x > initial.x + 1.0;
        looked += usize::from(looks_at(
            body,
            target.position(),
            f64::from(target.eye_height()),
        ));
        if walked
            && looked >= 4
            && (1.5..=3.8).contains(&separation(body.position(), target.position()))
        {
            approached = true;
            break;
        }
    }
    assert!(approached, "peer approach: {:?}", position(&world, piglin));
    command(&sender, &receiver, &mut world, &format!("kill {peer_uuid}"));
    let mut quiet = 0;
    let mut baseline = None;
    for _ in 0..450 {
        tick(&sender, &receiver, &mut world);
        let body = world.stored_entity_for_test(DIMENSION, piglin).unwrap();
        quiet = if settled(body) { quiet + 1 } else { 0 };
        if quiet >= 4 && world.stored_entity_for_test(DIMENSION, peer).is_none() {
            baseline = Some(body.position());
            break;
        }
    }
    let baseline = baseline.expect("peer removal and actual navigation settling");
    let mut strolled = false;
    for _ in 0..900 {
        tick(&sender, &receiver, &mut world);
        let body = world.stored_entity_for_test(DIMENSION, piglin).unwrap();
        assert_eq!(health(&world, piglin), hp);
        if speed_matches(body) && body.on_ground() && separation(body.position(), baseline) > 2.0 {
            strolled = true;
            break;
        }
    }
    assert!(
        strolled,
        "fresh solitary stroll: {:?}",
        position(&world, piglin)
    );
}

#[test]
fn an_unhurt_brute_looks_and_walks_toward_a_fixed_brute_peer() {
    let (sender, receiver, mut world) = arena(true);
    command(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:piglin_brute 5.5 65 8.5 {IsImmuneToZombification:1b}",
    );
    let brute = world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIMENSION, *id)
                .unwrap()
                .entity_type_id()
                == VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
        })
        .unwrap();
    let (peer, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        11.5,
        8.5,
        "NoAI:1b",
    );
    let initial = position(&world, brute);
    let hp = health(&world, brute);
    let peer_hp = health(&world, peer);
    let mut walked = false;
    let mut looked = 0;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..900 {
        tick(&sender, &receiver, &mut world);
        let body = world.stored_entity_for_test(DIMENSION, brute).unwrap();
        let target = world.stored_entity_for_test(DIMENSION, peer).unwrap();
        assert_eq!(health(&world, brute), hp);
        assert_eq!(health(&world, peer), peer_hp);
        walked |= speed_matches(body)
            && body.position().x > initial.x + 1.0
            && separation(body.position(), target.position()) < 5.0;
        looked += usize::from(looks_at(
            body,
            target.position(),
            f64::from(target.eye_height()),
        ));
        if walked && looked >= 4 {
            return;
        }
    }
    panic!("brute peer interaction: {:?}", position(&world, brute));
}

#[test]
fn a_safe_player_holding_a_loved_item_is_gazed_at_beyond_eight_blocks() {
    let (sender, receiver, mut world) = arena(false);
    for text in [
        "item replace entity alice armor.head with minecraft:golden_helmet",
        "item replace entity alice weapon.offhand with minecraft:gold_ingot",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    let (piglin, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        20.5,
        8.5,
        "CannotHunt:1b,NoAI:0b",
    );
    let mut looked = 0;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..450 {
        tick(&sender, &receiver, &mut world);
        let body = world.stored_entity_for_test(DIMENSION, piglin).unwrap();
        let player = world.publication().players[0].position;
        let distance = separation(body.position(), player);
        assert_eq!(world.player_health_for_test(ALICE), 20.0);
        if distance > 8.0 && distance <= 14.0 && looks_at(body, player, 1.62) {
            looked += 1;
        }
        if looked >= 4 {
            return;
        }
    }
    panic!("remote loved-player gaze: {:?}", position(&world, piglin));
}
