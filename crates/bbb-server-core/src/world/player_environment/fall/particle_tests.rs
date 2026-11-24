use super::super::tests::{block, world};
use super::*;
use bbb_protocol::game_types::GameType;
const PLAYER: ConnectionId = ConnectionId(1);

#[test]
fn player_fall_particles_use_viewer_block_center_range_and_dimension() {
    let mut world = ServerWorld::new(crate::world::ServerWorldConfig {
        game_type: GameType::Survival,
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let origin = Vec3d {
        x: 0.5,
        y: 80.5,
        z: 0.5,
    };
    world.levels[PLAYER].set_position(origin);
    let dimension = world.levels[PLAYER].dimension;
    let packet = LevelParticles {
        override_limiter: false,
        always_show: false,
        position: origin,
        offset: Vec3d::default(),
        max_speed: 0.15,
        count: 1,
        particle: ParticlePayload {
            particle_type_id: bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_BLOCK_ID,
            raw_options: vec![0],
        },
    };
    for (x, other_dimension, reaches) in [
        (31.999, false, true),
        (32.001, false, false),
        (0.5, true, false),
    ] {
        world.levels.move_player(
            ConnectionId(2),
            if other_dimension {
                bbb_server_types::save::Java26BuiltinDimension::Nether.into()
            } else {
                dimension
            },
        );
        world.levels[ConnectionId(2)].set_position(Vec3d { x, ..origin });
        let mut events = Vec::new();
        world.publish_player_particles(PLAYER, packet.clone(), &mut events);
        assert_eq!(particles(&events).len(), 1);
        assert_eq!(
            events.iter().any(|event| matches!(
                event.packet_for(ConnectionId(2)),
                Some(ClientboundPlayPacket::LevelParticles(_))
            )),
            reaches
        );
    }
}
fn particles(events: &[WorldEvent]) -> Vec<&LevelParticles> {
    events
        .iter()
        .filter_map(|event| match event.packet_for(PLAYER) {
            Some(ClientboundPlayPacket::LevelParticles(packet)) => Some(packet),
            _ => None,
        })
        .collect()
}
fn support(world: &mut ServerWorld) -> BlockPos {
    let feet = crate::world::block_position(world.levels[PLAYER].position());
    let pos = BlockPos {
        y: feet.y - 1,
        ..feet
    };
    block(world, pos, "minecraft:stone");
    pos
}
#[test]
fn player_fall_particles_read_old_distance_before_packet_accumulation() {
    for (old, count) in [(0.0, None), (6.0, Some(60))] {
        let mut world = world(GameType::Survival);
        support(&mut world);
        world.levels[PLAYER].block_contact.fall_distance = old;
        let mut events = Vec::new();
        world.check_player_fall_state(
            PLAYER,
            Vec3d {
                y: -4.0,
                ..Default::default()
            },
            true,
            &mut events,
        );
        assert_eq!(particles(&events).first().map(|packet| packet.count), count);
        assert_eq!(
            world.levels[PLAYER].damage.health,
            if old == 0.0 { 19.0 } else { 13.0 }
        );
    }
}
#[test]
fn player_fall_particles_precede_hurt_and_ignore_creative_damage_immunity() {
    for game_type in [GameType::Survival, GameType::Creative] {
        let mut world = world(game_type);
        support(&mut world);
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.block_contact.fall_distance = 8.0;
        player.cold.spawn_extra_particles_on_fall = true;
        let mut events = Vec::new();
        world.check_player_fall_state(PLAYER, Vec3d::default(), true, &mut events);
        let packets = particles(&events);
        assert_eq!(packets.len(), 2);
        assert_eq!(packets[0].count, 200);
        assert_eq!(packets[0].offset.x, f64::from(0.3_f32));
        assert_eq!(packets[1].count, 80);
        assert_eq!(packets[1].offset, Vec3d::default());
        assert!(!world.levels[PLAYER].cold.spawn_extra_particles_on_fall);
        assert!(packets
            .iter()
            .all(|p| !p.override_limiter && !p.always_show && p.max_speed == 0.15));
        if game_type == GameType::Survival {
        } else {
            assert_eq!(world.levels[PLAYER].damage.health, 20.0);
        }
    }
}
#[test]
fn player_fall_extra_particles_precede_water_reset_and_clear_only_on_positive_landing() {
    let mut world = world(GameType::Survival);
    support(&mut world);
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.cold.spawn_extra_particles_on_fall = true;
    let mut events = Vec::new();
    world.check_player_fall_state(PLAYER, Vec3d::default(), true, &mut events);
    assert!(particles(&events).is_empty());
    assert!(world.levels[PLAYER].cold.spawn_extra_particles_on_fall);
    let feet = crate::world::block_position(world.levels[PLAYER].position());
    block(&mut world, feet, "minecraft:water");
    world.levels[PLAYER].block_contact.fall_distance = 8.0;
    world.check_player_fall_state(PLAYER, Vec3d::default(), false, &mut events);
    assert!(world.levels[PLAYER].cold.spawn_extra_particles_on_fall);
    world.levels[PLAYER].block_contact.fall_distance = 8.0;
    world.levels[PLAYER].fluid_interaction = Default::default();
    world.check_player_fall_state(PLAYER, Vec3d::default(), true, &mut events);
    assert_eq!(particles(&events).len(), 1);
    assert_eq!(particles(&events)[0].count, 200);
    assert!(!world.levels[PLAYER].cold.spawn_extra_particles_on_fall);
    assert_eq!(world.levels[PLAYER].damage.health, 20.0);
}
