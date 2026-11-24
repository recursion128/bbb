use super::*;
use bbb_server_types::save::Java26LevelKey;

#[test]
fn actual_crystal_cactus_tick_finishes_explosion_before_the_next_crystal_ticks() {
    let (mut world, builtin, dimension, ids) =
        crystal_contact_world(crate::tests::datapack_config());
    let mut events = Vec::new();
    world.tick_level_entities(&world.require_level_id(&dimension), true, &mut events);
    for id in ids {
        assert!(world.level(builtin).entities.get(id).is_none());
    }
    assert!(world
        .level_mut(builtin)
        .entities
        .drain_explosions()
        .is_empty());
    let (game_events, _) = world.game_events(builtin);
    assert_eq!(
        game_events
            .iter()
            .filter(|event| event.game_event == crate::level_event::VanillaGameEvent::EXPLODE)
            .count(),
        1
    );
}

fn crystal_contact_world(
    config: crate::world::ServerWorldConfig,
) -> (
    ServerWorld,
    Java26BuiltinDimension,
    Java26LevelKey,
    [i32; 2],
) {
    let mut world = ServerWorld::new(config).unwrap();
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let builtin = Java26BuiltinDimension::Overworld;
    let dimension = Java26LevelKey::from(builtin);
    for x in -1..=1 {
        for z in -1..=1 {
            world.ensure_entity_transfer_full_chunk_for_test(builtin, (x, z));
        }
    }
    let cactus = BlockPos { x: 8, y: 80, z: 8 };
    let block =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:cactus").unwrap();
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
    assert!(world.set_block(world.require_level_id(&dimension), cactus, state, 2));
    let ids = [8.5, 11.5].map(|x| {
        crate::entity::end_crystal::spawn_end_crystal(
            &mut world.level_mut(builtin).entities,
            Vec3d { x, y: 80.0, z: 8.5 },
            true,
        )
        .unwrap()
    });
    (world, builtin, dimension, ids)
}
