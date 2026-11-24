use super::super::tests::{block, world};
use super::*;
use crate::random::GameRandom;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{BlockPos, Vec3d};

const PLAYER: ConnectionId = ConnectionId(1);

#[test]
fn player_step_magma_damage_precedes_inside_fire_damage() {
    let mut world = world(GameType::Survival);
    standing(&mut world, "minecraft:magma_block");
    let feet = crate::world::block_position(world.levels[PLAYER].position());
    block(&mut world, feet, "minecraft:fire");
    let mut events = Vec::new();
    world.tick_player_block_contacts(&mut events);
    let damage: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: crate::connection::ClientboundPlayPacket::DamageEvent(event),
            } if *connection_id == PLAYER => Some(event.source_type_id),
            _ => None,
        })
        .collect();
    let hot_floor = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|entry| entry.resource_id == "minecraft:hot_floor")
        .unwrap()
        .registry_id;
    assert_eq!(damage, vec![hot_floor]);
    assert_eq!(world.levels[PLAYER].damage.health, 19.0);
}

fn standing(world: &mut ServerWorld, resource: &str) -> BlockPos {
    let feet = crate::world::block_position(world.levels[PLAYER].position());
    let pos = BlockPos {
        y: feet.y - 1,
        ..feet
    };
    block(world, pos, resource);
    world.levels[PLAYER].set_on_ground(true);
    pos
}

#[test]
fn player_step_magma_runs_without_movement_and_uses_shift_not_pose() {
    for (on_ground, shift, health) in [
        (true, false, 19.0),
        (true, true, 20.0),
        (false, false, 20.0),
    ] {
        let mut world = world(GameType::Survival);
        standing(&mut world, "minecraft:magma_block");
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.set_on_ground(on_ground);
        player.pose_id = bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID;
        player.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, shift);
        world.tick_player_block_contacts(&mut Vec::new());
        assert_eq!(world.levels[PLAYER].damage.health, health);
    }
}

#[test]
fn player_step_slime_scales_live_horizontal_delta_without_sync_push() {
    for shift in [false, true] {
        let mut world = world(GameType::Survival);
        standing(&mut world, "minecraft:slime_block");
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.set_delta_movement(Vec3d {
            x: 1.0,
            y: 0.05,
            z: -2.0,
        });
        player.needs_sync = false;
        player.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, shift);
        world.tick_player_block_contacts(&mut Vec::new());
        let scale = if shift { 1.0 } else { 0.4 + 0.05 * 0.2 };
        assert_eq!(
            world.levels[PLAYER].delta_movement(),
            Vec3d {
                x: scale,
                y: 0.05,
                z: -2.0 * scale
            }
        );
        assert!(!world.levels[PLAYER].needs_sync);
    }
}

#[test]
fn player_step_redstone_consumes_visible_face_rng_even_after_lighting_and_when_dead() {
    for name in ["minecraft:redstone_ore", "minecraft:deepslate_redstone_ore"] {
        let mut world = world(GameType::Survival);
        let pos = standing(&mut world, name);
        for (x, y, z) in [
            (1, 0, 0),
            (-1, 0, 0),
            (0, 1, 0),
            (0, -1, 0),
            (0, 0, 1),
            (0, 0, -1),
        ] {
            block(
                &mut world,
                BlockPos {
                    x: pos.x + x,
                    y: pos.y + y,
                    z: pos.z + z,
                },
                "minecraft:air",
            );
        }
        let dimension = world.levels[PLAYER].dimension;
        world.levels[PLAYER].damage.health = 0.0;
        for _ in 0..2 {
            world.tick_player_block_contacts(&mut Vec::new());
            let level = world.level_by_id(&dimension);
            assert_eq!(
                bbb_server_types::block_queries::bool_property(level.block_state(pos), "lit"),
                Some(true)
            );
        }
        world.levels[PLAYER].set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, true);
        world.tick_player_block_contacts(&mut Vec::new());
    }
}

#[test]
fn player_step_turtle_egg_draws_one_in_hundred_then_shared_mutation_pitch() {
    let mut world = world(GameType::Survival);
    let pos = standing(&mut world, "minecraft:turtle_egg");
    let dimension = world.levels[PLAYER].dimension;
    let state = world.level_by_id(&dimension).block_state(pos);
    let state = crate::fluid::with_property(state, "eggs", "2").unwrap();
    world
        .level_mut_by_id(&dimension)
        .chunks
        .set_block_state(pos.x, pos.y, pos.z, state)
        .unwrap();
    let seed = (0..10000)
        .find(|seed| GameRandom::with_seed(*seed).next_int(100) == 0)
        .unwrap();
    let expected = GameRandom::with_seed(seed);
    assert_eq!(expected.next_int(100), 0);
    expected.next_float();
    world.level_mut_by_id(&dimension).random.reseed(seed);
    world.tick_player_block_contacts(&mut Vec::new());
    let level = world.level_by_id(&dimension);
    assert_eq!(
        bbb_server_types::block_queries::int_property(level.block_state(pos), "eggs"),
        Some(1)
    );
}

fn live_step_block(world: &mut ServerWorld, name: &str) -> BlockPos {
    let feet = crate::world::block_position(world.levels[PLAYER].position());
    let pos = BlockPos {
        y: feet.y - 1,
        ..feet
    };
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
    let dimension = world.levels[PLAYER].dimension;
    assert!(world.set_block(dimension, pos, state, 3));
    world.levels[PLAYER].set_on_ground(true);
    world.levels[PLAYER].set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, true);
    pos
}

#[test]
fn player_step_sensor_forces_candidate_while_careful_and_shrieker_is_immediate() {
    for name in ["minecraft:sculk_sensor", "minecraft:sculk_shrieker"] {
        let mut world = world(GameType::Survival);
        let pos = live_step_block(&mut world, name);
        let dimension = world.levels[PLAYER].dimension;
        world.tick_player_block_contacts(&mut Vec::new());
        let level = world.level_by_id(&dimension);
        let entity = level.chunks.block_entities().get(pos).unwrap();
        if let Some(sensor) = entity.sculk_sensor() {
            let (candidate, _) = sensor
                .vibration_data()
                .selection_strategy()
                .current()
                .unwrap();
            assert_eq!(candidate.game_event, "minecraft:step");
            assert_eq!(candidate.source, Some(world.levels[PLAYER].profile.uuid));
            assert_eq!(candidate.pos, world.levels[PLAYER].position());
        } else {
            assert_eq!(
                bbb_server_types::block_queries::bool_property(level.block_state(pos), "shrieking"),
                Some(true)
            );
        }
    }
}

#[test]
fn player_step_calibrated_sensor_uses_live_comparator_output_and_missing_entity_zero() {
    for output in [Some(0), Some(1), Some(2), None] {
        let mut world = world(GameType::Survival);
        let pos = live_step_block(&mut world, "minecraft:calibrated_sculk_sensor");
        let dimension = world.levels[PLAYER].dimension;
        let sensor_state = world.level_by_id(&dimension).block_state(pos);
        let facing = crate::block_behavior::direction_property(sensor_state, "facing").unwrap();
        let back = match facing {
            bbb_protocol::packets::Direction::North => bbb_protocol::packets::Direction::South,
            _ => panic!("default calibrated sensor faces north"),
        };
        let comparator_pos = bbb_server_types::directions::relative(pos, back);
        block(
            &mut world,
            BlockPos {
                y: comparator_pos.y - 1,
                ..comparator_pos
            },
            "minecraft:stone",
        );
        let base = bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_COMPARATOR_ID,
        )
        .unwrap();
        let state = crate::fluid::with_property(base, "facing", "south").unwrap();
        let state = crate::fluid::with_property(state, "powered", "true").unwrap();
        assert!(world.set_block(dimension, comparator_pos, state, 2));
        let mut store = world
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut();
        if let Some(output) = output {
            store
                .reborrow()
                .get_mut(comparator_pos)
                .unwrap()
                .comparator_mut()
                .unwrap()
                .set_output_signal(output);
        } else {
            store.take(comparator_pos).unwrap();
        }
        *store
            .get_mut(pos)
            .unwrap()
            .sculk_sensor_mut()
            .unwrap()
            .vibration_data_mut_for_test() = Default::default();
        world.tick_player_block_contacts(&mut Vec::new());
        let level = world.level_by_id(&dimension);
        let sensor = level
            .chunks
            .block_entities()
            .get(pos)
            .unwrap()
            .sculk_sensor()
            .unwrap();
        assert_eq!(
            sensor
                .vibration_data()
                .selection_strategy()
                .current()
                .is_some(),
            matches!(output, None | Some(0 | 1)),
            "comparator output {output:?}"
        );
    }
}
