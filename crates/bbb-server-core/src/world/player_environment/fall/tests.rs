use super::super::tests::{block, world};
use super::*;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::Vec3d;

const PLAYER: ConnectionId = ConnectionId(1);

fn land(world: &mut ServerWorld, block_name: &str, distance: f64) -> Vec<WorldEvent> {
    let feet = crate::world::block_position(world.levels[PLAYER].position());
    block(
        world,
        BlockPos {
            y: feet.y - 1,
            ..feet
        },
        block_name,
    );
    world.levels[PLAYER].block_contact.fall_distance = distance;
    let mut events = Vec::new();
    world.check_player_fall_state(PLAYER, Vec3d::default(), true, &mut events);
    events
}

fn distance_stat(world: &ServerWorld) -> i32 {
    world.levels[PLAYER]
        .cold
        .stats
        .value(custom_stat("minecraft:fall_one_cm"))
}

#[test]
fn player_fall_stone_damage_stat_and_reset_use_shared_calculation() {
    let mut world = world(GameType::Survival);
    land(&mut world, "minecraft:stone", 7.0);
    assert_eq!(world.levels[PLAYER].damage.health, 16.0);
    assert_eq!(distance_stat(&world), 700);
    assert_eq!(world.levels[PLAYER].block_contact.fall_distance, 0.0);
}

#[test]
fn player_fall_farmland_lifts_before_mutation_damage_and_ground_event() {
    let mut world = world(GameType::Survival);
    let position = world.levels[PLAYER].position();
    let support = BlockPos {
        x: position.x.floor() as i32,
        y: 79,
        z: position.z.floor() as i32,
    };
    block(&mut world, support, "minecraft:farmland");
    let dimension = world.levels[PLAYER].dimension;
    world
        .level_mut_by_id(&dimension)
        .event_sink
        .drain_game_events();
    let player = world.levels.player_mut(PLAYER).unwrap();
    {
        let mut position = player.position();
        position.y = 79.9375;
        player.set_position(position);
    }
    player.block_contact.fall_distance = 10.0;
    let mut events = Vec::new();
    world.check_player_fall_state(PLAYER, Vec3d::default(), true, &mut events);
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.position().y, 80.0);
    assert!(player.awaiting_teleport.is_some());
    assert_eq!(player.damage.health, 13.0);
    assert_eq!(distance_stat(&world), 1000);
    let entity_id = player.entity_id();
    let dirt = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_DIRT_ID,
    )
    .unwrap();
    assert_eq!(
        stored_block_state(&world.level_by_id(&dimension).chunks, support),
        dirt
    );
    let game_events: Vec<_> = world
        .level_by_id(&dimension)
        .event_sink
        .recent_game_events()
        .copied()
        .collect();
    use crate::level_event::VanillaGameEvent;
    let changed = game_events
        .iter()
        .rposition(|e| e.game_event == VanillaGameEvent::BLOCK_CHANGE)
        .unwrap();
    let ground = game_events
        .iter()
        .rposition(|e| e.game_event == VanillaGameEvent::HIT_GROUND)
        .unwrap();
    assert_eq!(game_events[changed].source_entity_id, Some(entity_id));
    assert_eq!(game_events[changed].affected_state, Some(dirt));
    assert_eq!(game_events[ground].affected_state, Some(dirt));
}

#[test]
fn player_fall_small_pose_does_not_trample_farmland() {
    let mut world = world(GameType::Survival);
    world.levels[PLAYER].pose_id = bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID;
    let dimension = world.levels[PLAYER].dimension;
    land(&mut world, "minecraft:farmland", 10.0);
    let position = world.levels[PLAYER].position();
    let support = BlockPos {
        x: position.x.floor() as i32,
        y: 79,
        z: position.z.floor() as i32,
    };
    let state = stored_block_state(&world.level_by_id(&dimension).chunks, support);
    assert_eq!(
        bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state),
        Some(bbb_protocol::block_types::VANILLA_BLOCK_FARMLAND_ID)
    );
    assert_eq!(world.levels[PLAYER].damage.health, 13.0);
}

#[test]
fn player_fall_turtle_eggs_mutate_before_damage_with_old_state_context() {
    use crate::random::GameRandom;
    for eggs in [1, 2] {
        let mut world = world(GameType::Survival);
        let position = world.levels[PLAYER].position();
        let support = BlockPos {
            x: position.x.floor() as i32,
            y: 79,
            z: position.z.floor() as i32,
        };
        block(&mut world, support, "minecraft:turtle_egg");
        let dimension = world.levels[PLAYER].dimension;
        let old = stored_block_state(&world.level_by_id(&dimension).chunks, support);
        let old = crate::fluid::with_property(old, "eggs", &eggs.to_string()).unwrap();
        world
            .level_mut_by_id(&dimension)
            .chunks
            .set_block_state(support.x, support.y, support.z, old)
            .unwrap();
        let seed = (0..1000)
            .find(|seed| GameRandom::with_seed(*seed).next_int(3) == 0)
            .unwrap();
        world.level_mut_by_id(&dimension).random.reseed(seed);
        world
            .level_mut_by_id(&dimension)
            .event_sink
            .drain_game_events();
        world.levels[PLAYER].block_contact.fall_distance = 10.0;
        world.check_player_fall_state(PLAYER, Vec3d::default(), true, &mut Vec::new());
        let state = stored_block_state(&world.level_by_id(&dimension).chunks, support);
        if eggs == 1 {
            assert!(bbb_server_types::fluid::block_state_is_air(state));
        } else {
            assert_eq!(
                state,
                crate::fluid::with_property(old, "eggs", "1").unwrap()
            );
        }
        assert_eq!(world.levels[PLAYER].damage.health, 13.0);
        let game_events: Vec<_> = world
            .level_by_id(&dimension)
            .event_sink
            .recent_game_events()
            .copied()
            .collect();
        use crate::level_event::VanillaGameEvent;
        let destroyed = game_events
            .iter()
            .rposition(|e| e.game_event == VanillaGameEvent::BLOCK_DESTROY)
            .unwrap();
        assert_eq!(game_events[destroyed].source_entity_id, None);
        assert_eq!(game_events[destroyed].affected_state, Some(old));
    }
}

#[test]
fn player_fall_mutating_block_does_not_carry_landed_distance_to_next_surface() {
    for name in ["minecraft:farmland", "minecraft:turtle_egg"] {
        let mut world = world(GameType::Survival);
        land(&mut world, name, 10.0);
        assert_eq!(world.levels[PLAYER].block_contact.fall_distance, 0.0);
        let feet = crate::world::block_position(world.levels[PLAYER].position());
        block(
            &mut world,
            BlockPos {
                y: feet.y - 1,
                ..feet
            },
            "minecraft:stone",
        );
        world.check_player_fall_state(PLAYER, Vec3d::default(), true, &mut Vec::new());
        assert_eq!(world.levels[PLAYER].damage.health, 13.0);
        assert_eq!(distance_stat(&world), 1000);
    }
}

#[test]
fn player_fall_bed_and_hay_change_damage_but_only_bed_changes_stat_distance() {
    for (name, health, stat) in [
        ("minecraft:red_bed", 18.0, 500),
        ("minecraft:hay_block", 19.0, 1000),
        ("minecraft:slime_block", 20.0, 1000),
        ("minecraft:powder_snow", 20.0, 0),
    ] {
        let mut world = world(GameType::Survival);
        land(&mut world, name, 10.0);
        assert_eq!(world.levels[PLAYER].damage.health, health, "{name}");
        assert_eq!(distance_stat(&world), stat, "{name}");
    }
}

#[test]
fn player_fall_mayfly_and_suppressed_slime_skip_statistics() {
    let mut creative = world(GameType::Creative);
    land(&mut creative, "minecraft:stone", 10.0);
    assert_eq!(distance_stat(&creative), 0);
    let mut survival = world(GameType::Survival);
    survival.levels[PLAYER].set_shared_flag(
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
        true,
    );
    land(&mut survival, "minecraft:slime_block", 10.0);
    assert_eq!(distance_stat(&survival), 0);
    assert_eq!(survival.levels[PLAYER].damage.health, 20.0);
}

#[test]
fn player_fall_invulnerable_hurt_still_reports_damage_and_statistics() {
    let mut world = world(GameType::Survival);
    world.levels[PLAYER].abilities.invulnerable = true;
    assert_eq!(
        world.cause_player_fall_damage(PLAYER, 10.0, 1.0, "minecraft:fall", &mut Vec::new(),),
        Some(true)
    );
    assert_eq!(world.levels[PLAYER].damage.health, 20.0);
    assert_eq!(distance_stat(&world), 1000);
}

#[test]
fn player_fall_refreshes_water_before_packet_distance_accumulation() {
    let mut world = world(GameType::Survival);
    let feet = crate::world::block_position(world.levels[PLAYER].position());
    block(&mut world, feet, "minecraft:water");
    assert!(!world.levels[PLAYER]
        .fluid_interaction
        .is_in_fluid(crate::entity::FluidTag::Water));
    world.levels[PLAYER].block_contact.fall_distance = 10.0;
    world.check_player_fall_state(
        PLAYER,
        Vec3d {
            y: -2.0,
            ..Vec3d::default()
        },
        true,
        &mut Vec::new(),
    );
    assert_eq!(world.levels[PLAYER].damage.health, 20.0);
    assert_eq!(distance_stat(&world), 0);
    assert_eq!(world.levels[PLAYER].block_contact.fall_distance, 0.0);
}

#[test]
fn player_fall_accepted_packet_emits_damage_then_ground_event_with_support_state() {
    let mut world = world(GameType::Survival);
    let from = world.levels[PLAYER].position();
    let feet = crate::world::block_position(from);
    let support = BlockPos {
        y: feet.y - 1,
        ..feet
    };
    block(&mut world, support, "minecraft:stone");
    let dimension = world.levels[PLAYER].dimension;
    world
        .level_mut_by_id(&dimension)
        .event_sink
        .drain_game_events();
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.first_good = from;
    player.last_good = from;
    player.block_contact.fall_distance = 7.0;
    world.apply_command(
        crate::world::WorldCommand::Play {
            connection_id: PLAYER,
            packet: bbb_protocol::packets::PlayServerbound::MovePlayer(
                bbb_protocol::packets::MovePlayer {
                    position: Some(from),
                    rotation: None,
                    on_ground: true,
                    horizontal_collision: false,
                },
            ),
        },
        &mut Vec::new(),
    );
    assert_eq!(world.levels[PLAYER].damage.health, 16.0);
    assert_eq!(distance_stat(&world), 700);
    let state = stored_block_state(&world.level_by_id(&dimension).chunks, support);
    let game_events: Vec<_> = world
        .level_by_id(&dimension)
        .event_sink
        .recent_game_events()
        .copied()
        .collect();
    let ground = game_events
        .iter()
        .position(|event| event.game_event == crate::level_event::VanillaGameEvent::HIT_GROUND)
        .unwrap();
    assert_eq!(game_events[ground].affected_state, Some(state));
    assert_eq!(
        game_events[ground].source_entity_id,
        Some(world.levels[PLAYER].entity_id())
    );
}

#[test]
fn player_fall_honey_refused_hurt_keeps_ordered_sounds_and_excludes_self() {
    let mut world = ServerWorld::new(crate::world::ServerWorldConfig {
        game_type: GameType::Survival,
        ..crate::world::ServerWorldConfig::default()
    })
    .unwrap();
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    {
        let mut position = world.levels[PLAYER].position();
        position.y = 80.0;
        world.levels[PLAYER].set_position(position);
    }
    let position = world.levels[PLAYER].position();
    world.levels[ConnectionId(2)].set_position(position);
    world.levels[PLAYER].abilities.invulnerable = true;
    let events = land(&mut world, "minecraft:honey_block", 10.0);
    let sounds: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::Sound(sound),
            } => {
                assert_eq!(*connection_id, ConnectionId(2));
                assert_eq!(sound.source, SoundSource::Players);
                Some(sound.sound.clone())
            }
            _ => None,
        })
        .collect();
    let expected: Vec<_> = [
        "minecraft:block.honey_block.slide",
        "minecraft:entity.player.small_fall",
        "minecraft:block.honey_block.fall",
        "minecraft:block.honey_block.fall",
    ]
    .into_iter()
    .map(|name| SoundEventHolder::Reference {
        registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(name)
            .unwrap(),
    })
    .collect();
    assert_eq!(sounds, expected);
    assert_eq!(world.levels[PLAYER].damage.health, 20.0);
    assert_eq!(distance_stat(&world), 1000);
    assert!(events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::EntityEvent(EntityEvent { event_id: 54, .. }),
            ..
        }
    )));
}

#[test]
fn a_falling_cow_tramples_farmland_takes_fall_damage_and_is_lifted() {
    let mut world = world(GameType::Spectator);
    let dimension = world.levels[PLAYER].dimension;
    let position = world.levels[PLAYER].position();
    let support = BlockPos {
        x: position.x.floor() as i32 + 3,
        y: 79,
        z: position.z.floor() as i32,
    };
    block(&mut world, support, "minecraft:farmland");
    let start = Vec3d {
        x: f64::from(support.x) + 0.5,
        y: 80.25,
        z: f64::from(support.z) + 0.5,
    };
    let entities = &mut world.level_mut_by_id(&dimension).entities;
    let cow = entities
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            start,
        )
        .unwrap();
    let body = entities.get_mut(cow).unwrap();
    body.set_fall_distance(8.8);
    body.set_delta_movement(Vec3d {
        x: 0.0,
        y: -0.5,
        z: 0.0,
    });
    let health = body.living().unwrap().health();
    world.tick(Vec::new());
    let dirt = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_DIRT_ID,
    )
    .unwrap();
    assert_eq!(
        stored_block_state(&world.level_by_id(&dimension).chunks, support),
        dirt
    );
    let body = world.level_by_id(&dimension).entities.get(cow).unwrap();
    assert_eq!(body.position().y, 80.0);
    assert_eq!(body.fall_distance(), 0.0);
    assert!(body.living().unwrap().health() < health);
}
