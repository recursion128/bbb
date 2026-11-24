//! A living mount its rider's client moves through `MoveVehicle`: the fall it
//! accumulates over the client's deltas, the damage that fall deals to the
//! mount and the rider, and the movement game events it raises
//! (`ServerGamePacketListenerImpl.java:470-503`).

use super::horse_interaction::{hold, horse_world, tamed_equine};
use super::saddled_mounts::{mount, spawn_beside};
use super::*;
use bbb_protocol::packets::MoveVehicleCommand;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn vehicle_position(world: &ServerWorld, vehicle: i32) -> Vec3d {
    world
        .stored_entity_for_test(DIM, vehicle)
        .expect("the vehicle is live")
        .position()
}

fn health(world: &ServerWorld, entity: i32) -> f32 {
    world
        .stored_entity_for_test(DIM, entity)
        .and_then(|entity| entity.living().map(|living| living.health()))
        .expect("a living mount")
}

fn ride_to(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    position: Vec3d,
    on_ground: bool,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position,
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground,
        }),
    )
}

/// Lifts the vehicle `height` blocks off the ground in one packet and lands it
/// back on the spot it left in the next.
fn drop_from(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    vehicle: i32,
    height: f64,
) {
    let ground = vehicle_position(world, vehicle);
    let lifted = Vec3d {
        y: ground.y + height,
        ..ground
    };
    ride_to(sender, receiver, world, lifted, false);
    assert_eq!(
        vehicle_position(world, vehicle),
        lifted,
        "the lift is accepted"
    );
    ride_to(sender, receiver, world, ground, true);
    assert_eq!(
        vehicle_position(world, vehicle),
        ground,
        "the landing is accepted"
    );
}

fn rider_health(world: &ServerWorld) -> f32 {
    world.player_health_for_test(ConnectionId(1))
}

/// `LivingEntity.causeFallDamage` on the pig (safe fall 3) and, through
/// `propagateFallToPassengers`, `Player.causeFallDamage` on the rider: eight
/// blocks cost each of them five.
#[test]
fn a_pig_its_rider_drops_hurts_the_pig_and_the_rider() {
    let (sender, receiver, mut world) = horse_world();
    let pig = spawn_beside(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
    );
    hold(&mut world, "minecraft:saddle", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        super::feeding::interact_entity_packet(pig),
    );
    hold(&mut world, "minecraft:carrot_on_a_stick", 1);
    mount(&sender, &receiver, &mut world, pig);
    let (pig_before, rider_before) = (health(&world, pig), rider_health(&world));

    drop_from(&sender, &receiver, &mut world, pig, 8.0);
    assert_eq!(health(&world, pig), pig_before - 5.0);
    assert_eq!(rider_health(&world), rider_before - 5.0);
}

/// `AbstractHorse.causeFallDamage` (`AbstractHorse.java:279-293`) with the
/// horse's safe fall 6 and multiplier 0.5 (`:383-384`): a fall the horse
/// shrugs off spares the rider too, and a longer one hurts the horse two and
/// passes the whole ten blocks to the rider.
#[test]
fn a_horse_absorbs_a_short_fall_and_passes_a_long_one_to_its_rider() {
    let (sender, receiver, mut world) = horse_world();
    let horse = tamed_equine(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID,
        1.5,
    );
    hold(&mut world, "minecraft:saddle", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        super::feeding::interact_entity_packet(horse),
    );
    hold(&mut world, "minecraft:stick", 1);
    mount(&sender, &receiver, &mut world, horse);
    let (horse_before, rider_before) = (health(&world, horse), rider_health(&world));

    drop_from(&sender, &receiver, &mut world, horse, 4.0);
    assert_eq!(health(&world, horse), horse_before);
    assert_eq!(
        rider_health(&world),
        rider_before,
        "the rider alone would take one"
    );

    drop_from(&sender, &receiver, &mut world, horse, 10.0);
    assert_eq!(health(&world, horse), horse_before - 2.0);
    assert_eq!(rider_health(&world), rider_before - 7.0);
}

fn default_state(block: i32) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(block)
        .expect("a built-in block")
}

/// A harnessed happy ghast mounted through the player's two real clicks.
fn ridden_happy_ghast(
    sender: &WorldCommandSender,
    world: &mut ServerWorld,
    receiver: &WorldCommandReceiver,
) -> i32 {
    let ghast = spawn_beside(
        world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    );
    hold(world, "minecraft:white_harness", 1);
    play(
        sender,
        receiver,
        world,
        1,
        super::feeding::interact_entity_packet(ghast),
    );
    assert_eq!(
        world
            .player_inventory_slot_for_test(ConnectionId(1), 0)
            .count,
        0
    );
    mount(sender, receiver, world, ghast);
    ghast
}

/// `HappyGhast.checkFallDamage` is empty (`HappyGhast.java:163-164`).
#[test]
fn a_happy_ghast_its_rider_drops_hurts_nobody() {
    let (sender, receiver, mut world) = horse_world();
    let ghast = ridden_happy_ghast(&sender, &mut world, &receiver);
    let (ghast_before, rider_before) = (health(&world, ghast), rider_health(&world));

    drop_from(&sender, &receiver, &mut world, ghast, 8.0);
    assert_eq!(health(&world, ghast), ghast_before);
    assert_eq!(rider_health(&world), rider_before);
}

/// The client-moved happy ghast settles on top of a shrieker; the next tick's
/// `stepOn` resolves the shrieker's player through the ghast's controlling
/// passenger (`SculkShriekerBlockEntity.java:88-98`).
#[test]
fn a_ridden_happy_ghast_landing_on_a_shrieker_shrieks_for_its_rider() {
    let (sender, receiver, mut world) = horse_world();
    let ghast = ridden_happy_ghast(&sender, &mut world, &receiver);
    let ground = vehicle_position(&world, ghast);
    let shrieker = BlockPos {
        x: (ground.x + 2.5).floor() as i32,
        y: ground.y.floor() as i32,
        z: ground.z.floor() as i32,
    };
    assert!(world.set_block(
        DIM,
        shrieker,
        default_state(bbb_protocol::block_types::VANILLA_BLOCK_SCULK_SHRIEKER_ID),
        SET_BLOCK_UPDATE_ALL
    ));
    ride_to(
        &sender,
        &receiver,
        &mut world,
        // A shrieker is half a block tall.
        Vec3d {
            x: ground.x + 2.5,
            y: ground.y + 0.5,
            z: ground.z,
        },
        true,
    );
    world.tick(receiver.drain());

    let rider = world.player_entity_id_for_test(ConnectionId(1));
    assert!(world
        .positioned_game_events_for_test(DIM)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::SHRIEK
                && event.recorded.source_entity_id == Some(rider)
        ));
}

/// `move(MoverType.PLAYER, ..)` runs the pig's movement emission
/// (`Entity.java:790-795`): two blocks of ground travel raise `STEP` naming the
/// pig, which wakes a sensor in range.
#[test]
fn a_pig_its_rider_walks_steps_and_wakes_a_sculk_sensor() {
    let (sender, receiver, mut world) = horse_world();
    let pig = spawn_beside(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
    );
    hold(&mut world, "minecraft:saddle", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        super::feeding::interact_entity_packet(pig),
    );
    hold(&mut world, "minecraft:carrot_on_a_stick", 1);
    mount(&sender, &receiver, &mut world, pig);
    let ground = vehicle_position(&world, pig);
    let sensor = BlockPos {
        x: ground.x.floor() as i32 + 5,
        y: ground.y.floor() as i32,
        z: ground.z.floor() as i32 + 2,
    };
    let inactive = default_state(bbb_protocol::block_types::VANILLA_BLOCK_SCULK_SENSOR_ID);
    assert!(world.set_block(DIM, sensor, inactive, SET_BLOCK_UPDATE_ALL));
    for _ in 0..3 {
        world.tick(receiver.drain());
    }

    ride_to(
        &sender,
        &receiver,
        &mut world,
        Vec3d {
            x: ground.x + 2.0,
            ..ground
        },
        true,
    );
    assert!(world
        .positioned_game_events_for_test(DIM)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::STEP
            && event.recorded.source_entity_id == Some(pig)));
    let woke = (0..10).any(|_| {
        world.tick(receiver.drain());
        world.block_state(DIM, sensor) != Some(inactive)
    });
    assert!(woke, "the step reached the sensor");
}
