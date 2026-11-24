use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::packets::{SoundEventHolder, SoundSource};

#[test]
fn water_splash_session_fall_refresh_and_base_share_one_entry_and_player_sound() {
    let (mut world, sender, receiver) = world_with_mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let alice = ConnectionId(1);
    let bob = ConnectionId(2);
    let position = world.levels[alice].position();
    let dimension = world.levels[alice].dimension;
    world.set_player_position_for_test(
        bob,
        Vec3d {
            x: position.x + 4.0,
            ..position
        },
    );
    let mut events = Vec::new();
    world.tick_player_fluid_contact(&mut events);
    assert!(!world.levels[alice].was_touching_water);
    world.levels[alice].environment_first_tick = false;
    pool(&mut world, 1);
    let player = world.levels.player_mut(alice).unwrap();
    player.set_silent(true);
    player.set_delta_movement(Vec3d {
        x: 0.0,
        y: -2.0,
        z: 0.0,
    });
    player.block_contact.fall_distance = 8.0;
    events.clear();
    world.check_player_fall_state(
        alice,
        Vec3d {
            y: -0.25,
            ..Vec3d::default()
        },
        false,
        &mut events,
    );
    assert!(world.levels[alice].was_touching_water);
    assert_eq!(world.levels[alice].block_contact.fall_distance, 0.0);
    let sounds: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            crate::world::WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::Sound(sound),
            } => Some((*connection_id, sound)),
            _ => None,
        })
        .collect();
    assert_eq!(sounds.len(), 1);
    let (recipient, sound) = sounds[0];
    assert_eq!(recipient, bob);
    assert_eq!(sound.source, SoundSource::Players);
    assert_eq!(sound.volume, 0.4);
    // `1 + (nextFloat() - nextFloat()) * 0.4`.
    assert!((0.6..=1.4).contains(&sound.pitch));
    assert_eq!(
        sound.sound,
        SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.player.splash.high_speed"
            )
            .unwrap(),
        }
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        crate::world::WorldEvent::Send {
            packet: ClientboundPlayPacket::LevelParticles(_),
            ..
        }
    )));
    events.clear();
    world.tick_player_fluid_contact(&mut events);
    assert!(!events.iter().any(|event| matches!(
        event,
        crate::world::WorldEvent::Send {
            packet: ClientboundPlayPacket::Sound(_),
            ..
        }
    )));
    assert_eq!(
        world
            .level_by_id(&dimension)
            .event_sink
            .recent_game_events()
            .filter(|event| event.game_event == VanillaGameEvent::SPLASH)
            .count(),
        1
    );
}

/// `Player.doWaterSplashEffect` (`Player.java:1454-1458`).
#[test]
fn a_spectator_entering_water_makes_no_splash() {
    let (mut world, sender, receiver) = world_with_mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let alice = ConnectionId(1);
    let position = world.levels[alice].position();
    let dimension = world.levels[alice].dimension;
    world.set_player_position_for_test(
        ConnectionId(2),
        Vec3d {
            x: position.x + 4.0,
            ..position
        },
    );
    let mut events = Vec::new();
    world.tick_player_fluid_contact(&mut events);
    world.levels[alice].environment_first_tick = false;
    world.levels[alice].game_type = GameType::Spectator;
    pool(&mut world, 1);
    events.clear();
    world.tick_player_fluid_contact(&mut events);
    assert!(world.levels[alice].was_touching_water);
    assert!(!events.iter().any(|event| matches!(
        event,
        crate::world::WorldEvent::Send {
            packet: ClientboundPlayPacket::Sound(_),
            ..
        }
    )));
    assert_eq!(
        world
            .level_by_id(&dimension)
            .event_sink
            .recent_game_events()
            .filter(|event| event.game_event == VanillaGameEvent::SPLASH)
            .count(),
        0
    );
}

#[test]
fn player_current_averages_touched_cells_and_flying_skips_current_reads() {
    use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
    use bbb_server_types::fluid::{FluidFamily, FluidState};
    let mut world = world();
    let id = ConnectionId(1);
    let player = world.levels.player_mut(id).unwrap();
    let x = player.position().x.floor() as i32;
    let z = player.position().z.floor() as i32;
    {
        let mut position = player.position();
        position.x = f64::from(x) + 0.95;
        player.set_position(position);
    }
    player
        .attributes
        .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
        .unwrap()
        .set_base_value(2.0);
    player.set_delta_movement(Vec3d {
        x: 0.1,
        ..Vec3d::default()
    });
    let dimension = player.dimension;
    let source = FluidState::source(FluidFamily::Water, false).create_legacy_block();
    let shallow = FluidState::flowing(FluidFamily::Water, 4, false).create_legacy_block();
    let air = bbb_server_types::fluid::air_block_state_id();
    let chunks = &mut world.level_mut_by_id(&dimension).chunks;
    for xx in x - 2..=x + 3 {
        for zz in z - 2..=z + 2 {
            for y in 79..=85 {
                assert!(chunks.set_block_state(xx, y, zz, air).is_some());
            }
        }
    }
    for xx in x - 1..=x + 1 {
        for zz in z - 1..=z + 1 {
            assert!(chunks.set_block_state(xx, 80, zz, source).is_some());
        }
    }
    for zz in z - 1..=z + 1 {
        assert!(chunks.set_block_state(x + 2, 80, zz, shallow).is_some());
    }
    let mut events = Vec::new();
    world.tick_player_fluid_contact(&mut events);
    assert!(world.levels[id].body_tick_enabled);
    assert!((world.levels[id].delta_movement().x - 0.107).abs() < 1e-12);
    assert!(world.levels[id]
        .fluid_interaction
        .is_in_fluid(FluidTag::Water));
    // This neighbour is read only to compute current, outside the touched body cells.
    assert!(world
        .level_mut_by_id(&dimension)
        .chunks
        .set_block_state(x + 2, 80, z, i32::MAX)
        .is_some());
    let player = world.levels.player_mut(id).unwrap();
    player.abilities.flying = true;
    player.set_delta_movement(Vec3d {
        x: 0.1,
        ..Vec3d::default()
    });
    world.tick_player_fluid_contact(&mut events);
    assert!(world.levels[id].body_tick_enabled);
    assert_eq!(world.levels[id].delta_movement().x, 0.1);
    world.levels[id].abilities.flying = false;
    assert!(world
        .level_mut_by_id(&dimension)
        .chunks
        .set_block_state(x + 2, 80, z, shallow)
        .is_some());
    world.tick_player_fluid_contact(&mut events);
    assert!(world.levels[id].body_tick_enabled);
    assert!((world.levels[id].delta_movement().x - 0.107).abs() < 1e-12);
}
