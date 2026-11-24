//! Hoglin family following, ordinary piglin avoidance and idle attention on live ticks.

use super::melee::{fixture, ALICE};
use super::*;
use crate::tests::entities::console_command;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID,
};

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) {
    let operation = world.game_time() as u64 + 10000;
    let output = console_command(sender, receiver, world, operation, text);
    let outcome = admin_outcome(&output);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{text}: {report:?}");
}

fn arena() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "gamemode spectator alice",
        "forceload add -32 -16 48 32",
        "fill -32 64 -16 48 64 32 minecraft:stone",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    (sender, receiver, world)
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
    (id, uuid)
}

fn tick(sender: &WorldCommandSender, receiver: &WorldCommandReceiver, world: &mut ServerWorld) {
    let output = world.tick(receiver.drain());
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

#[track_caller]
fn until(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    limit: usize,
    label: &str,
    mut done: impl FnMut(&ServerWorld) -> bool,
) {
    for _ in 0..limit {
        tick(sender, receiver, world);
        if done(world) {
            return;
        }
    }
    panic!("{label} did not occur within {limit} ticks");
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
fn speed_matches(body: &crate::entity::Entity, multiplier: f64) -> bool {
    let living = body.living().unwrap();
    (f64::from(living.speed())
        - multiplier * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
    .abs()
        < 1.0e-5
}
fn looks_at(body: &crate::entity::Entity, target: &crate::entity::Entity) -> bool {
    let from = body.position();
    let to = target.position();
    let yaw = (to.z - from.z).atan2(to.x - from.x).to_degrees() as f32 - 90.0;
    let yaw_error = (body.y_head_rot() - yaw + 180.0).rem_euclid(360.0) - 180.0;
    let pitch = (from.y + f64::from(body.eye_height()) - to.y - f64::from(target.eye_height()))
        .atan2(separation(from, to))
        .to_degrees() as f32;
    yaw_error.abs() < 5.0 && (body.x_rot() - pitch).abs() < 3.0
}

#[test]
fn a_baby_hoglin_follows_a_fixed_adult_at_its_family_speed_and_four_block_reach() {
    let (sender, receiver, mut world) = arena();
    let (adult, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        3.5,
        8.5,
        "NoAI:1b",
    );
    let (baby, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        11.5,
        8.5,
        "Age:-24000",
    );
    let initial = position(&world, baby);
    let mut followed = false;
    world.tick_rate_mut().set_frozen(false);
    until(
        &sender,
        &receiver,
        &mut world,
        450,
        "baby following",
        |world| {
            let body = world.stored_entity_for_test(DIMENSION, baby).unwrap();
            followed |= speed_matches(body, 0.6) && body.position().x < initial.x - 0.5;
            followed
                && body.on_ground()
                && (3.0..=4.6).contains(&separation(body.position(), position(world, adult)))
                && body.delta_movement().x.hypot(body.delta_movement().z) < 0.02
        },
    );
    let close = position(&world, baby);
    assert!(close.x < initial.x - 3.0 && (close.y - 65.0).abs() < 0.001);
    assert_eq!(
        position(&world, adult),
        Vec3d {
            x: 3.5,
            y: 65.0,
            z: 8.5
        }
    );
}

#[test]
fn an_unhurt_adult_hoglin_walks_away_from_a_nearby_adult_piglin_at_idle_speed() {
    let (sender, receiver, mut world) = arena();
    let (hoglin, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        8.5,
        8.5,
        "NoAI:0b",
    );
    let (piglin, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        12.5,
        8.5,
        "NoAI:1b",
    );
    let initial_health = world
        .stored_entity_for_test(DIMENSION, hoglin)
        .unwrap()
        .living()
        .unwrap()
        .health();
    let mut walked = false;
    world.tick_rate_mut().set_frozen(false);
    until(
        &sender,
        &receiver,
        &mut world,
        450,
        "ordinary piglin avoidance",
        |world| {
            let body = world.stored_entity_for_test(DIMENSION, hoglin).unwrap();
            assert_eq!(body.living().unwrap().health(), initial_health);
            assert!(
                !speed_matches(body, 1.3),
                "ordinary idle avoidance uses no hurt retreat"
            );
            walked |= speed_matches(body, 0.4) && body.position().x < 8.0;
            walked
                && body.on_ground()
                && separation(body.position(), position(world, piglin)) >= 8.0
        },
    );
    assert!(position(&world, hoglin).x < 6.0);
    assert_eq!(
        position(&world, piglin),
        Vec3d {
            x: 12.5,
            y: 65.0,
            z: 8.5
        }
    );
}

#[test]
fn a_lone_hoglin_looks_and_walks_to_a_cow_then_strolls_after_the_cow_is_removed() {
    let (sender, receiver, mut world) = arena();
    let (hoglin, _) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        5.5,
        8.5,
        "NoAI:0b",
    );
    let (cow, cow_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        11.5,
        8.5,
        "NoAI:1b",
    );
    let cow_health = world
        .stored_entity_for_test(DIMENSION, cow)
        .unwrap()
        .living()
        .unwrap()
        .health();
    let initial = position(&world, hoglin);
    let mut walked = false;
    let mut looking_ticks = 0;
    world.tick_rate_mut().set_frozen(false);
    until(
        &sender,
        &receiver,
        &mut world,
        900,
        "living attention and approach",
        |world| {
            let body = world.stored_entity_for_test(DIMENSION, hoglin).unwrap();
            let cow = world.stored_entity_for_test(DIMENSION, cow).unwrap();
            assert_eq!(cow.living().unwrap().health(), cow_health);
            walked |= speed_matches(body, 0.4) && body.position().x > initial.x + 1.0;
            looking_ticks = if looks_at(body, cow) {
                looking_ticks + 1
            } else {
                0
            };
            walked
                && looking_ticks >= 4
                && body.on_ground()
                && (2.5..=3.7).contains(&separation(body.position(), cow.position()))
                && body.delta_movement().x.hypot(body.delta_movement().z) < 0.02
        },
    );
    command(&sender, &receiver, &mut world, &format!("kill {cow_uuid}"));
    for _ in 0..30 {
        tick(&sender, &receiver, &mut world);
    }
    let after_cow_left = position(&world, hoglin);
    until(
        &sender,
        &receiver,
        &mut world,
        900,
        "idle land strolling",
        |world| {
            let body = world.stored_entity_for_test(DIMENSION, hoglin).unwrap();
            body.on_ground()
                && speed_matches(body, 0.4)
                && separation(body.position(), after_cow_left) > 2.0
        },
    );
    let body = world.stored_entity_for_test(DIMENSION, hoglin).unwrap();
    assert!((body.position().y - 65.0).abs() < 0.001 && !body.is_in_water() && !body.is_in_lava());
}
