use super::*;

const SENSOR: BlockPos = BlockPos { x: 5, y: 70, z: 6 };

fn sensor(name: &str, phase: &str, power: i32, waterlogged: bool) -> i32 {
    let power = power.to_string();
    let mut properties = vec![
        ("sculk_sensor_phase", phase),
        ("power", power.as_str()),
        ("waterlogged", if waterlogged { "true" } else { "false" }),
    ];
    if name == "minecraft:calibrated_sculk_sensor" {
        properties.push(("facing", "north"));
    }
    state_of(name, &properties)
}

fn dispatch_tick(world: &mut NativeWorld) {
    let state = world.state(SENSOR);
    world.with_level(|level| tick_block(level, SENSOR, state));
}

#[test]
fn dry_cooldown_tick_becomes_inactive_and_plays_stop_sound() {
    let cooldown = sensor("minecraft:sculk_sensor", "cooldown", 0, false);
    let inactive = sensor("minecraft:sculk_sensor", "inactive", 0, false);
    let mut world = NativeWorld::new();
    world.set(SENSOR, cooldown);

    dispatch_tick(&mut world);

    assert_eq!(world.state(SENSOR), inactive);
    // `SculkSensorBlock.java:88`: pitch `nextFloat() * 0.2 + 0.8`.
    assert!(world.sounds().iter().any(|sound| sound.pos == SENSOR
        && sound.resource_id == "minecraft:block.sculk_sensor.clicking_stop"
        && (0.8..1.0).contains(&sound.pitch)));
}

#[test]
fn waterlogged_cooldown_is_silent() {
    let cooldown = sensor("minecraft:sculk_sensor", "cooldown", 0, true);
    let inactive = sensor("minecraft:sculk_sensor", "inactive", 0, true);
    let mut world = NativeWorld::new();
    world.set(SENSOR, cooldown);

    dispatch_tick(&mut world);

    assert_eq!(world.state(SENSOR), inactive);
    assert!(world.sounds().is_empty());
}

#[test]
fn every_shape_update_preserves_sensor_state_and_saves_its_water_tick() {
    for direction in DIRECTION_VALUES {
        let state = sensor("minecraft:calibrated_sculk_sensor", "inactive", 0, true);
        let mut world = NativeWorld::new();
        world.set(SENSOR, state);
        world.set(relative(SENSOR, direction), stone());
        world.shape_update(SENSOR, direction);
        assert_eq!(world.state(SENSOR), state);
        assert!(world
            .saved_fluid_ticks()
            .iter()
            .any(|tick| tick.id == "minecraft:water"
                && tick.x == SENSOR.x
                && tick.y == SENSOR.y
                && tick.z == SENSOR.z
                && tick.delay == 5));
    }
    let state = sensor("minecraft:sculk_sensor", "inactive", 0, false);
    let mut world = NativeWorld::new();
    world.set(SENSOR, state);
    world.set(relative(SENSOR, Direction::North), water(0));
    world.shape_update(SENSOR, Direction::North);
    assert_eq!(world.state(SENSOR), state);
    assert!(world
        .saved_fluid_ticks()
        .iter()
        .all(|tick| (tick.x, tick.y, tick.z) != (SENSOR.x, SENSOR.y, SENSOR.z)));
}

#[test]
fn a_calibrated_sensor_loses_power_then_returns_to_inactive_on_world_ticks() {
    let active = sensor("minecraft:calibrated_sculk_sensor", "active", 15, false);
    let cooldown = sensor("minecraft:calibrated_sculk_sensor", "cooldown", 0, false);
    let inactive = sensor("minecraft:calibrated_sculk_sensor", "inactive", 0, false);
    let mut world = NativeWorld::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world.world, 1, "listener", 1);
    world.world.set_player_position_for_test(
        crate::connection::ConnectionId(1),
        bbb_protocol::packets::Vec3d {
            x: 5.5,
            y: 64.0,
            z: 7.5,
        },
    );
    world.set(SENSOR, inactive);
    world.set(SENSOR, active);
    world.with_level(|level| {
        level.schedule_block_tick(
            SENSOR,
            BlockTickType::of_block_state(active).unwrap(),
            1,
            TickPriority::Normal,
        )
    });
    world.run_tick();
    assert_eq!(world.state(SENSOR), cooldown);
    let stop_sound = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:block.sculk_sensor.clicking_stop",
    )
    .unwrap();
    let mut heard_stop = false;
    for _ in 0..10 {
        let tick = world.world.tick(receiver.drain());
        heard_stop |= tick.events.iter().any(|event| {
            matches!(event, crate::world::WorldEvent::Send {
                packet: crate::connection::ClientboundPlayPacket::Sound(sound), ..
            } if matches!(&sound.sound, bbb_protocol::packets::SoundEventHolder::Reference { registry_id } if *registry_id == stop_sound))
        });
    }
    assert_eq!(world.state(SENSOR), inactive);
    assert!(heard_stop);
}

#[test]
fn a_placed_active_sensor_recovers_power_unless_its_deactivation_is_pending() {
    let active = sensor("minecraft:calibrated_sculk_sensor", "active", 12, false);
    let unpowered = sensor("minecraft:calibrated_sculk_sensor", "active", 0, false);
    let cooldown = sensor("minecraft:calibrated_sculk_sensor", "cooldown", 0, false);
    let dimension = bbb_server_types::save::Java26BuiltinDimension::Overworld;
    let mut recovered = NativeWorld::new();
    recovered.world.set_block(dimension, SENSOR, active, 2 | 16);
    assert_eq!(recovered.state(SENSOR), unpowered);

    let mut pending = NativeWorld::ticking(SENSOR);
    pending.with_level(|level| {
        level.schedule_block_tick(
            SENSOR,
            BlockTickType::of_block_state(active).unwrap(),
            10,
            TickPriority::Normal,
        )
    });
    pending.world.set_block(dimension, SENSOR, active, 2 | 16);
    assert_eq!(pending.state(SENSOR), active);
    for _ in 0..10 {
        pending.run_tick();
    }
    assert_eq!(pending.state(SENSOR), cooldown);
}
