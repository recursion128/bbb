use super::*;
use crate::fluid::with_property;
use crate::world::simulation_level::LevelCtx;
use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;

#[test]
fn door_progress_world_level_preserves_broadcast_order_and_door_source() {
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    let mut world = ServerWorld::new(Default::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "watcher", 1);
    let dimension = Java26BuiltinDimension::Overworld;
    let key: LevelId = dimension.into();
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    world.levels[ConnectionId(1)].set_position(Vec3d {
        x: 0.0,
        y: 64.0,
        z: 0.0,
    });
    world.flush_level_events(&key, &mut Vec::new());
    world.with_level(dimension, |level| {
        LevelCtx::level_event(level, 1019, pos, 0);
        LevelCtx::destroy_block_progress(level, 777, pos, 10);
        LevelCtx::level_event(level, 1021, pos, 0);
        LevelCtx::destroy_block_progress(level, 777, pos, -1);
    });
    let mut events = Vec::new();
    world.flush_level_events(&key, &mut events);
    let state = |kind| vanilla_block_default_state_id(kind).unwrap();
    let lower = state(bbb_protocol::block_types::VANILLA_BLOCK_COPPER_DOOR_ID);
    let upper = with_property(lower, "half", "upper").unwrap();
    world.set_block(
        dimension,
        BlockPos { y: 63, ..pos },
        state(bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID),
        SET_BLOCK_UPDATE_ALL,
    );
    world.set_block(dimension, pos, lower, SET_BLOCK_UPDATE_ALL);
    let top = BlockPos { y: 65, ..pos };
    world.set_block(dimension, top, upper, SET_BLOCK_UPDATE_ALL);
    world.with_level(dimension, |level| {
        assert!(crate::block_behavior::door::set_open(
            level,
            top,
            upper,
            Some(777),
            true
        ));
    });
    for half in [pos, top] {
        let state = world.block_state(dimension, half).unwrap();
        assert!(
            bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                .unwrap()
                .properties
                .iter()
                .any(|(name, value)| name == "open" && value == "true")
        );
    }
    let recorded = world
        .level_by_id(&key)
        .event_sink
        .recent_game_events()
        .last()
        .unwrap();
    assert_eq!(recorded.source_entity_id, Some(777));
    assert_eq!(recorded.affected_state, None);
    assert_eq!(recorded.pos, top);
}
