//! Baby piglins naturally mount, time out and leave a hoglin that becomes adult.

use super::melee::{fixture, ALICE};
use super::*;
use crate::tests::entities::console_command;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID};

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

fn passenger_packet(output: &TickResult, vehicle: i32, rider: Option<i32>) -> bool {
    output.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ALICE, packet: ClientboundPlayPacket::SetPassengers(packet), .. }
        if packet.vehicle_id == vehicle && match rider {
            Some(rider) => packet.passenger_ids == vec![rider],
            None => packet.passenger_ids.is_empty(),
        }))
}
fn attached(world: &ServerWorld, hoglin: i32, piglin: i32) -> bool {
    world.stored_passengers_for_test(DIMENSION, hoglin) == [piglin]
}
fn riding_approach(world: &ServerWorld, piglin: i32, hoglin: i32, before: Vec3d) -> bool {
    let body = world.stored_entity_for_test(DIMENSION, piglin).unwrap();
    let living = body.living().unwrap();
    let base = living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
    (f64::from(living.speed()) - 0.8 * base).abs() < 1.0e-5
        && separation(body.position(), position(world, hoglin))
            < separation(before, position(world, hoglin)) - 0.1
}

fn arena() -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    Uuid,
    i32,
    Uuid,
) {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "gamemode spectator alice",
        "forceload add -32 -16 48 32",
        "fill -32 64 -16 48 64 32 minecraft:stone",
        "fill 3 65 3 3 67 12 minecraft:stone",
        "fill 12 65 3 12 67 12 minecraft:stone",
        "fill 4 65 3 11 67 3 minecraft:stone",
        "fill 4 65 12 11 67 12 minecraft:stone",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    let (piglin, piglin_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        5.5,
        8.5,
        "IsBaby:1b,CannotHunt:1b,NoAI:0b",
    );
    let (hoglin, hoglin_uuid) = mob(
        &sender,
        &receiver,
        &mut world,
        VANILLA_ENTITY_TYPE_HOGLIN_ID,
        8.5,
        8.5,
        "Age:-24000,NoAI:1b",
    );
    (
        sender,
        receiver,
        world,
        piglin,
        piglin_uuid,
        hoglin,
        hoglin_uuid,
    )
}

#[test]
fn a_baby_piglin_naturally_rides_times_out_and_leaves_a_hoglin_that_grows_up() {
    let (sender, receiver, mut world, piglin, _, hoglin, hoglin_uuid) = arena();
    let piglin_hp = health(&world, piglin);
    let hoglin_hp = health(&world, hoglin);
    let fixed = position(&world, hoglin);
    world.tick_rate_mut().set_frozen(false);
    let mut approached = false;
    let mut mount_packet = false;
    let mut first_mount = false;
    let mut before = position(&world, piglin);
    for _ in 0..1200 {
        let output = tick(&sender, &receiver, &mut world);
        assert_eq!(health(&world, piglin), piglin_hp);
        assert_eq!(health(&world, hoglin), hoglin_hp);
        mount_packet |= passenger_packet(&output, hoglin, Some(piglin));
        if !attached(&world, hoglin, piglin) {
            approached |= riding_approach(&world, piglin, hoglin, before);
        } else if mount_packet {
            first_mount = true;
            break;
        }
        before = position(&world, piglin);
    }
    assert!(first_mount && approached,
        "natural .8 approach and observed mounting: approached={approached}, packet={mount_packet}, rider={:?}, hoglin={:?}",
        position(&world, piglin), position(&world, hoglin));
    for _ in 0..4 {
        tick(&sender, &receiver, &mut world);
        assert!(attached(&world, hoglin, piglin));
        let rider = position(&world, piglin);
        let vehicle = position(&world, hoglin);
        assert!(
            rider.y > vehicle.y && separation(rider, vehicle) < 1.0,
            "rider={rider:?}, vehicle={vehicle:?}"
        );
        assert_eq!(vehicle, fixed);
    }
    let mut empty_packet = false;
    let mut expired = false;
    for _ in 0..650 {
        let output = tick(&sender, &receiver, &mut world);
        empty_packet |= passenger_packet(&output, hoglin, None);
        assert_eq!(health(&world, piglin), piglin_hp);
        assert_eq!(health(&world, hoglin), hoglin_hp);
        if !attached(&world, hoglin, piglin) && empty_packet {
            expired = true;
            break;
        }
    }
    assert!(
        expired,
        "an unharmed baby ride naturally expires and publishes empty passengers"
    );
    mount_packet = false;
    let mut remounted = false;
    for _ in 0..1200 {
        let output = tick(&sender, &receiver, &mut world);
        mount_packet |= passenger_packet(&output, hoglin, Some(piglin));
        if attached(&world, hoglin, piglin) && mount_packet {
            remounted = true;
            break;
        }
    }
    assert!(remounted, "the still-baby pair can naturally ride again");
    world.tick_rate_mut().set_frozen(true);
    let adult = command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {hoglin_uuid} {{Age:0}}"),
    );
    empty_packet = passenger_packet(&adult, hoglin, None);
    world.tick_rate_mut().set_frozen(false);
    let mut left_adult = false;
    for _ in 0..30 {
        let output = tick(&sender, &receiver, &mut world);
        empty_packet |= passenger_packet(&output, hoglin, None);
        if !attached(&world, hoglin, piglin) && empty_packet {
            left_adult = true;
            break;
        }
    }
    assert!(
        left_adult,
        "real hoglin maturity ends the baby ride and publishes empty passengers"
    );
    assert_eq!(health(&world, piglin), piglin_hp);
    assert_eq!(health(&world, hoglin), hoglin_hp);
}

#[test]
fn real_damage_interrupts_a_baby_piglins_natural_ride() {
    let (sender, receiver, mut world, piglin, piglin_uuid, hoglin, _) = arena();
    let hp = health(&world, piglin);
    let hoglin_hp = health(&world, hoglin);
    let mut mount_packet = false;
    let mut mounted = false;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..1200 {
        let output = tick(&sender, &receiver, &mut world);
        mount_packet |= passenger_packet(&output, hoglin, Some(piglin));
        if attached(&world, hoglin, piglin) && mount_packet {
            mounted = true;
            break;
        }
    }
    assert!(
        mounted,
        "a real natural ride precedes the damage interruption"
    );
    for _ in 0..4 {
        tick(&sender, &receiver, &mut world);
        assert!(attached(&world, hoglin, piglin));
        let rider = position(&world, piglin);
        let vehicle = position(&world, hoglin);
        assert!(
            rider.y > vehicle.y && separation(rider, vehicle) < 1.0,
            "rider={rider:?}, vehicle={vehicle:?}"
        );
    }
    let mounted_at = position(&world, piglin);
    world.tick_rate_mut().set_frozen(true);
    let hurt = command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {piglin_uuid} 2 minecraft:generic"),
    );
    assert!(health(&world, piglin) < hp, "the actual rider takes damage");
    let mut empty_packet = passenger_packet(&hurt, hoglin, None);
    let mut left = false;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..40 {
        let output = tick(&sender, &receiver, &mut world);
        empty_packet |= passenger_packet(&output, hoglin, None);
        let rider = position(&world, piglin);
        let dx = rider.x - mounted_at.x;
        let dy = rider.y - mounted_at.y;
        let dz = rider.z - mounted_at.z;
        let moved_off_seat = dx * dx + dy * dy + dz * dz > 1.0e-8;
        if !attached(&world, hoglin, piglin) && empty_packet && moved_off_seat {
            left = true;
            break;
        }
    }
    assert!(
        left,
        "hurt rider leaves the real passenger relation, packet and seat: baseline={mounted_at:?}, current={:?}, passengers={:?}, empty_packet={empty_packet}",
        position(&world, piglin),
        world.stored_passengers_for_test(DIMENSION, hoglin)
    );
    assert_eq!(health(&world, hoglin), hoglin_hp);
}
