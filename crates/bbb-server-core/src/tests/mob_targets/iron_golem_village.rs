//! A naturally occupied bed village draws a distant iron golem across live ground.

use super::melee::{fixture, ALICE};
use super::*;
use crate::tests::entities::console_command;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
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
fn separation(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}

fn health(world: &ServerWorld, id: i32) -> f32 {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
        .health()
}

#[test]
fn a_naturally_occupied_village_draws_an_iron_golem_toward_its_bed_section() {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "gamemode spectator alice",
        "forceload add -32 -32 80 48",
        "fill -16 64 -24 64 64 40 minecraft:stone",
        "gamerule advance_time false",
        "time set midnight",
        "weather clear",
        "setblock 7 65 8 minecraft:red_bed[part=foot,facing=east]",
        "setblock 8 65 8 minecraft:red_bed[part=head,facing=east]",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    let bed = BlockPos { x: 8, y: 65, z: 8 };
    let (resident, resident_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 10.5,
        },
    );
    let resident_health = health(&world, resident);
    let mut slept = false;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..1200 {
        tick(&sender, &receiver, &mut world);
        let body = world.stored_entity_for_test(DIMENSION, resident).unwrap();
        let sleeping =
            body.sleeping_pos() == Some(bed) && body.pose_id() == VANILLA_ENTITY_POSE_SLEEPING_ID;
        let occupied = world
            .block_state(DIMENSION, bed)
            .and_then(|state| bbb_server_types::block_queries::bool_property(state, "occupied"));
        if sleeping && occupied == Some(true) {
            slept = true;
            break;
        }
    }
    assert!(
        slept,
        "the real resident claims and sleeps in the actual bed"
    );
    let dimension = level_key(DIMENSION);
    assert_eq!(
        world
            .poi_record(&dimension, [8, 65, 8])
            .expect("actual bed POI")
            .free_tickets,
        0
    );
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {resident_uuid} {{NoAI:1b}}"),
    );
    command(&sender, &receiver, &mut world, "time set 3000");
    let initial = Vec3d {
        x: 40.5,
        y: 65.0,
        z: 8.5,
    };
    let home = Vec3d {
        x: 8.5,
        y: 65.0,
        z: 8.5,
    };
    let (golem, _) = spawn(&mut world, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, initial);
    assert_eq!(
        world.is_close_to_village(&dimension, [40, 65, 8], 1),
        Some(false)
    );
    assert_eq!(world.sections_to_village(&dimension, [2, 4, 0]), Some(2));
    let golem_health = health(&world, golem);
    let initial_distance = separation(initial, home);
    let mut approached = false;
    let mut best_distance = initial_distance;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..1200 {
        tick(&sender, &receiver, &mut world);
        assert_eq!(health(&world, resident), resident_health);
        assert_eq!(health(&world, golem), golem_health);
        let body = world.stored_entity_for_test(DIMENSION, golem).unwrap();
        let distance = separation(body.position(), home);
        best_distance = best_distance.min(distance);
        let living = body.living().unwrap();
        let speed_matches = (f64::from(living.speed())
            - 0.6 * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
        .abs()
            < 1.0e-5;
        if speed_matches
            && body.on_ground()
            && !body.is_in_water()
            && !body.is_in_lava()
            && distance < initial_distance - 4.0
        {
            approached = true;
            break;
        }
    }
    assert!(approached,
        "the golem walks toward the occupied village at .6: best_distance={best_distance}, current={:?}",
        world.stored_entity_for_test(DIMENSION, golem).unwrap().position());
    assert_eq!(
        world
            .poi_record(&dimension, [8, 65, 8])
            .expect("the resident keeps its actual bed POI")
            .free_tickets,
        0
    );
}
