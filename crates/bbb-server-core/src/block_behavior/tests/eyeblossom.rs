use super::native_world::NativeWorld;
use super::*;

fn open() -> i32 {
    default_state("minecraft:open_eyeblossom")
}

fn closed() -> i32 {
    default_state("minecraft:closed_eyeblossom")
}

fn supported_world(state: i32, game_time: i64) -> NativeWorld {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.world.set_game_time_for_test(game_time);
    world.world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(game_time),
        &mut Vec::new(),
    );
    world.set(at(0, 70, 0), dirt());
    world.set(at(0, 71, 0), state);
    world
}

#[test]
fn random_ticks_flip_both_directions_and_play_the_new_types_long_sound() {
    for (state, game_time, expected, sound, particle_color) in [
        (
            closed(),
            12_600,
            open(),
            "minecraft:block.eyeblossom.open_long",
            16_545_810,
        ),
        (
            open(),
            23_401,
            closed(),
            "minecraft:block.eyeblossom.close_long",
            6_250_335,
        ),
    ] {
        let mut world = supported_world(state, game_time);
        world.join_listener(bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 71.0,
            z: 2.5,
        });
        world.world.set_game_time_for_test(game_time);
        world.world.apply_clock_change(
            "minecraft:overworld",
            crate::commands::CommandClockChange::SetTotalTicks(game_time),
            &mut Vec::new(),
        );
        world.random_tick(at(0, 71, 0), 11);

        assert_eq!(world.state(at(0, 71, 0)), expected);
        assert!(world
            .sounds()
            .iter()
            .any(|entry| entry.pos == at(0, 71, 0) && entry.resource_id == sound));
        let tick = world.world.tick(Vec::new());
        assert!(tick.events.iter().any(|event| {
            let crate::world::WorldEvent::Send {
                packet: crate::connection::ClientboundPlayPacket::LevelParticles(packet),
                ..
            } = event
            else {
                return false;
            };
            if packet.particle.particle_type_id
                != bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_TRAIL_ID
            {
                return false;
            }
            // TrailParticleOption.java:20-28: target, RGB color, duration.
            let mut decoder = bbb_protocol::codec::Decoder::new(&packet.particle.raw_options);
            decoder.read_f64().unwrap();
            decoder.read_f64().unwrap();
            decoder.read_f64().unwrap();
            decoder.read_i32().unwrap() == particle_color
        }));
    }
}

#[test]
fn matching_day_phase_leaves_the_eyeblossom_alone() {
    for (state, game_time) in [(open(), 12_600), (closed(), 23_401)] {
        let mut world = supported_world(state, game_time);
        world.random_tick(at(0, 71, 0), 17);

        assert_eq!(world.state(at(0, 71, 0)), state);
        assert!(world.sounds().is_empty());
    }
}

#[test]
fn a_flipped_eyeblossom_brings_nearby_old_eyeblossoms_into_the_same_state() {
    let center = at(8, 80, 8);
    let nearby = [at(5, 78, 5), at(6, 78, 5), at(5, 79, 7), at(5, 78, 6)];
    let mut world = NativeWorld::ticking(center);
    world.world.set_game_time_for_test(12600);
    world.world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(12600),
        &mut Vec::new(),
    );
    world.set(below(center), dirt());
    world.set(center, closed());
    for pos in nearby {
        world.set(below(pos), dirt());
        world.set(pos, closed());
    }
    world.set(at(7, 77, 5), dirt());
    world.set(at(7, 78, 5), open());
    world.random_tick(center, 36);
    for _ in 0..100 {
        world.run_tick();
    }
    assert_eq!(world.state(center), open());
    for pos in nearby {
        assert_eq!(world.state(pos), open());
    }
    assert_eq!(world.state(at(7, 78, 5)), open());
}

#[test]
fn scheduled_ticks_flip_both_directions_and_use_short_sounds() {
    for (state, game_time, expected, sound) in [
        (closed(), 12_599, open(), "minecraft:block.eyeblossom.open"),
        (open(), 23_400, closed(), "minecraft:block.eyeblossom.close"),
    ] {
        let mut world = supported_world(state, game_time);
        let tick_type = BlockTickType::of_block_state(state).expect("eyeblossom block type");
        world.join_listener(bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 71.0,
            z: 2.5,
        });
        world.world.set_game_time_for_test(game_time);
        world.world.apply_clock_change(
            "minecraft:overworld",
            crate::commands::CommandClockChange::SetTotalTicks(game_time),
            &mut Vec::new(),
        );
        world.with_level(|level| {
            level.schedule_block_tick(at(0, 71, 0), tick_type, 1, TickPriority::Normal)
        });
        let tick = world.world.tick(Vec::new());
        assert_eq!(world.state(at(0, 71, 0)), expected);
        assert!(tick.events.iter().any(|event| matches!(event,
            crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::Sound(packet), .. }
            if matches!(&packet.sound, bbb_protocol::packets::SoundEventHolder::Reference { registry_id } if *registry_id == bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(sound).unwrap())
        )));
    }
}

#[test]
fn vegetation_survival_and_shape_update_remain_inherited() {
    let pos = at(0, 71, 0);
    assert_eq!(
        can_survive(
            closed(),
            pos,
            vanilla_tags(),
            &mut |at| if at == below(pos) { dirt() } else { air() }
        ),
        Some(true)
    );
    assert_eq!(
        can_survive(open(), pos, vanilla_tags(), &mut |_| stone()),
        Some(false)
    );

    let mut world = supported_world(closed(), 23_401);
    world.set(at(0, 70, 0), stone());
    world.shape_update(pos, Direction::Down);
    assert_eq!(world.state(pos), air());
    assert!(world
        .items()
        .iter()
        .any(|(id, count)| id == "minecraft:closed_eyeblossom" && *count > 0));
}
