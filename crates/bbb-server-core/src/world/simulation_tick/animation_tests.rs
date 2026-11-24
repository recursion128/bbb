use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
use bbb_protocol::packets::InteractionHand;
use bbb_server_types::save::Java26LevelKey;

fn world_with_tracked_mob() -> (ServerWorld, Java26LevelKey, i32) {
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    let mut world = ServerWorld::new(Default::default()).unwrap();
    for (id, name) in [(1, "tracked"), (2, "untracked"), (3, "other_level")] {
        crate::tests::join_loaded(&sender, &receiver, &mut world, id, name, u128::from(id));
    }
    let dimension: Java26LevelKey = Java26BuiltinDimension::Overworld.into();
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    for player in world.levels.players_mut() {
        player.set_position(position);
    }
    let id = world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .entities
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position)
        .unwrap();
    world.apply_level_entity_lifecycle_by_key(&world.require_level_id(&dimension), &mut Vec::new());
    world.flush_level_events(&world.require_level_id(&dimension), &mut Vec::new());
    assert_eq!(
        world
            .level_by_id(&world.require_level_id(&dimension))
            .entity_tracker
            .viewers_of(id),
        vec![ConnectionId(1), ConnectionId(2), ConnectionId(3)]
    );
    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .entity_tracker
        .viewer_left(ConnectionId(2));
    world.levels[ConnectionId(3)].dimension = Java26BuiltinDimension::Nether.into();
    (world, dimension, id)
}

#[test]
fn tracked_animation_has_no_block_destruction_distance_limit() {
    let (mut world, dimension, entity_id) = world_with_tracked_mob();
    {
        let mut position = world.levels[ConnectionId(1)].position();
        position.x = 40.0;
        world.levels[ConnectionId(1)].set_position(position);
    }
    world
        .level_mut_by_id(&world.require_level_id(&dimension))
        .event_sink
        .entity_animation(entity_id, InteractionHand::MainHand);
    let mut events = Vec::new();
    world.flush_level_events(&world.require_level_id(&dimension), &mut events);
    assert!(matches!(events.as_slice(), [WorldEvent::Send {
        connection_id: ConnectionId(1),
        packet: ClientboundPlayPacket::EntityAnimation(animation),
    }] if animation.id == entity_id && animation.action == 0));
}
