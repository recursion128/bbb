use super::*;
use crate::connection::ConnectionId;
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::tests::join_loaded;
use crate::world::ServerWorldConfig;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_STANDING_ID, VANILLA_ENTITY_POSE_SWIMMING_ID,
};
use bbb_protocol::game_types::GameType;
use bbb_server_types::save::Java26BuiltinDimension;

fn dimension() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}

fn loaded_world() -> ServerWorld {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    world
        .level_mut_by_id(&world.require_level_id(&dimension()))
        .chunks
        .try_ensure_served_full_column(0, 0)
        .unwrap();
    world.track_stored_chunk_for_test(Java26BuiltinDimension::Overworld, (0, 0));
    world
}

fn free(world: &ServerWorld, players: &[SpawnPlayer], bounds: Aabb, blocks_building: bool) -> bool {
    let level = world.level_by_id(&world.require_level_id(&dimension()));
    collision_free(
        &level.chunks,
        &level.entities,
        players,
        bounds,
        blocks_building,
    )
}

#[test]
fn actual_player_pose_and_scale_only_block_instance_obstruction() {
    let mut world = loaded_world();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let connection = ConnectionId(1);
    let player = world.levels.player_mut(connection).unwrap();
    player.set_position(Vec3d {
        x: 8.5,
        y: 100.0,
        z: 8.5,
    });
    let upper = Aabb {
        min_x: 8.3,
        min_y: 101.0,
        min_z: 8.3,
        max_x: 8.7,
        max_y: 101.5,
        max_z: 8.7,
    };
    for (pose, scale, clear) in [
        (VANILLA_ENTITY_POSE_STANDING_ID, 1.0, false),
        (VANILLA_ENTITY_POSE_SWIMMING_ID, 1.0, true),
        (VANILLA_ENTITY_POSE_SWIMMING_ID, 3.0, false),
        (VANILLA_ENTITY_POSE_STANDING_ID, 0.1, true),
    ] {
        let player = world.levels.player_mut(connection).unwrap();
        player.set_pose(pose);
        player
            .attributes
            .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
            .unwrap()
            .set_base_value(scale);
        let players = world.spawn_players(&world.require_level_id(&dimension()));
        assert_eq!(players.len(), 1);
        assert!(free(&world, &players, upper, false));
        assert_eq!(
            free(&world, &players, upper, true),
            clear,
            "pose {pose}, scale {scale}"
        );
    }
}

#[test]
fn live_projection_keeps_dead_players_but_excludes_removed_and_other_dimension_sessions() {
    let mut world = loaded_world();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let connection = ConnectionId(1);
    let player = world.levels.player_mut(connection).unwrap();
    player.set_position(Vec3d {
        x: 8.5,
        y: 100.0,
        z: 8.5,
    });
    player.set_pose(VANILLA_ENTITY_POSE_STANDING_ID);
    player.damage.health = 0.0;
    let bounds = spawn_aabb(0.6, 1.8, BlockPos { x: 8, y: 100, z: 8 });
    let dead = world.spawn_players(&world.require_level_id(&dimension()));
    assert_eq!(dead.len(), 1);
    assert!(dead[0].dead_or_dying);
    assert!(free(&world, &dead, bounds, false));
    assert!(!free(&world, &dead, bounds, true));
    let far = spawn_aabb(
        0.6,
        1.8,
        BlockPos {
            x: 10,
            y: 100,
            z: 8,
        },
    );
    assert!(free(&world, &dead, far, true));
    world.levels[connection].game_type = GameType::Spectator;
    let spectators = world.spawn_players(&world.require_level_id(&dimension()));
    assert_eq!(spectators.len(), 1);
    assert!(spectators[0].spectator);
    assert!(free(&world, &spectators, bounds, true));
    world.levels[connection].game_type = GameType::Survival;
    world.levels[connection].avatar_removed = true;
    assert!(world
        .spawn_players(&world.require_level_id(&dimension()))
        .is_empty());
    world.levels[connection].avatar_removed = false;
    world
        .levels
        .move_player(connection, Java26BuiltinDimension::Nether.into());
    assert!(world
        .spawn_players(&world.require_level_id(&dimension()))
        .is_empty());
}

#[test]
fn player_exclusion_preserves_block_store_and_unknown_chunk_collision_gates() {
    let mut world = loaded_world();
    let cow = spawn_aabb(0.6, 1.8, BlockPos { x: 8, y: 100, z: 8 });
    world
        .level_mut_by_id(&world.require_level_id(&dimension()))
        .entities
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 8.5,
                y: 100.0,
                z: 8.5,
            },
        )
        .unwrap();
    assert!(free(&world, &[], cow, false));
    assert!(!free(&world, &[], cow, true));
    let shulker = spawn_aabb(
        0.6,
        1.8,
        BlockPos {
            x: 10,
            y: 100,
            z: 8,
        },
    );
    world
        .level_mut_by_id(&world.require_level_id(&dimension()))
        .entities
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_ID,
            Vec3d {
                x: 10.5,
                y: 100.0,
                z: 8.5,
            },
        )
        .unwrap();
    for blocks_building in [false, true] {
        assert!(!free(&world, &[], shulker, blocks_building));
    }
    let solid = spawn_aabb(
        0.6,
        1.8,
        BlockPos {
            x: 12,
            y: 100,
            z: 8,
        },
    );
    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap();
    assert!(world
        .level_mut_by_id(&world.require_level_id(&dimension()))
        .chunks
        .set_block_state(12, 100, 8, stone)
        .is_some());
    for blocks_building in [false, true] {
        assert!(!free(&world, &[], solid, blocks_building));
    }
    let missing = spawn_aabb(
        0.6,
        1.8,
        BlockPos {
            x: 24,
            y: 100,
            z: 8,
        },
    );
    for blocks_building in [false, true] {
        assert!(!free(&world, &[], missing, blocks_building));
    }
    assert!(!world
        .level_by_id(&world.require_level_id(&dimension()))
        .chunks
        .is_loaded(1, 0));
}

#[test]
fn finalize_source_null_probe_and_actual_instance_obstruction_have_distinct_player_filters() {
    let mut world = loaded_world();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player = world.levels.player_mut(ConnectionId(1)).unwrap();
    player.set_position(Vec3d {
        x: 8.5,
        y: 100.0,
        z: 8.5,
    });
    player.set_pose(VANILLA_ENTITY_POSE_STANDING_ID);
    let dimension = dimension();
    let players = world.spawn_players(&world.require_level_id(&dimension));
    assert_eq!(players.len(), 1);
    let mut entities = EntityStore::new(7, crate::entity::EntityIdCounter::new());
    let id = entities
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            players[0].position,
        )
        .unwrap();
    let entity = entities.detach(id).unwrap();
    let mut environment = environment_for_test(&world, &dimension);
    environment.players = &players;
    assert!(environment.no_collision(&entities, entity.bounding_box()));
    let obstruction = natural_obstruction_probe(&environment, &entities, &entity);
    assert!(!obstruction.is_unobstructed);
    assert!(obstruction.default_dimensions_no_collision);
    environment.players = &[];
    let clear = natural_obstruction_probe(&environment, &entities, &entity);
    assert!(clear.is_unobstructed);
    assert!(clear.default_dimensions_no_collision);
}
