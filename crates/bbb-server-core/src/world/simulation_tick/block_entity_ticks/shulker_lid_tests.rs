use super::*;

#[test]
fn an_opening_shulker_box_lid_lifts_the_item_resting_on_it() {
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    let mut world = ServerWorld::new(Default::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let dimension = world.require_level_id(&Java26BuiltinDimension::Overworld.into());
    let pos = BlockPos { x: 0, y: 100, z: 0 };
    let state = crate::fluid::with_property(
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_SHULKER_BOX_ID,
        )
        .unwrap(),
        "facing",
        "up",
    )
    .unwrap();
    assert!(world.set_block(dimension, pos, state, SET_BLOCK_UPDATE_ALL));
    let id = crate::entity::spawn_item_entity(
        &mut world.level_mut_by_id(&dimension).entities,
        Vec3d {
            x: 0.5,
            y: 101.0,
            z: 0.5,
        },
        crate::entity::item::plain_stack("minecraft:stone", 1).unwrap(),
    )
    .unwrap();
    let shulker = world
        .level_mut_by_id(&dimension)
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .and_then(BlockEntity::shulker_box_mut)
        .unwrap();
    assert!(shulker.trigger_event(1, 1));

    world.tick_shulker_box(&dimension, pos, &mut Vec::new());

    let y = world
        .level_by_id(&dimension)
        .entities
        .get(id)
        .unwrap()
        .position()
        .y;
    // The band spans the raw progress 0 -> 0.1, so the push is 0.1 + 0.01.
    assert!(
        (y - (101.0 + f64::from(0.1_f32) + 0.01)).abs() < 1e-6,
        "{y}"
    );
}

/// `BlockBehaviour.java:1003`: an opening shulker box stops conducting, so
/// the wire beside it reaches down past it (`RedStoneWireBlock.java:254-257`).
#[test]
fn a_wire_reaches_down_past_an_open_shulker_box() {
    let mut world = ServerWorld::new(Default::default()).unwrap();
    let dimension = world.require_level_id(&Java26BuiltinDimension::Overworld.into());
    let state_of = |name: &str, properties: &[(&str, &str)]| {
        let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
        let mut state =
            bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
        for (key, value) in properties {
            state = crate::fluid::with_property(state, key, value).unwrap();
        }
        state
    };
    let wire = |north: &str, east: &str, south: &str, west: &str| {
        state_of(
            "minecraft:redstone_wire",
            &[
                ("north", north),
                ("east", east),
                ("south", south),
                ("west", west),
            ],
        )
    };
    let stone = state_of("minecraft:stone", &[]);
    let here = BlockPos { x: 0, y: 100, z: 0 };
    let shulker_pos = BlockPos { x: 1, y: 100, z: 0 };
    let lower = BlockPos { x: 1, y: 99, z: 0 };
    for (pos, state) in [
        (BlockPos { x: 0, y: 99, z: 0 }, stone),
        (BlockPos { x: 1, y: 98, z: 0 }, stone),
        (lower, wire("side", "side", "side", "side")),
        (
            shulker_pos,
            state_of("minecraft:shulker_box", &[("facing", "up")]),
        ),
        (here, wire("side", "none", "side", "none")),
    ] {
        assert!(world.set_block(dimension, pos, state, SET_BLOCK_UPDATE_ALL));
    }
    let east_side = |world: &ServerWorld| {
        let state = world
            .level_by_id(&dimension)
            .chunks
            .get_block_state(here.x, here.y, here.z)
            .unwrap();
        bbb_protocol::block_state_types::vanilla_block_state_property(state, "east")
    };
    assert_eq!(east_side(&world), Some("none"));
    let shulker = world
        .level_mut_by_id(&dimension)
        .chunks
        .block_entities_mut()
        .get_mut(shulker_pos)
        .and_then(BlockEntity::shulker_box_mut)
        .unwrap();
    assert!(shulker.trigger_event(1, 1));
    for _ in 0..3 {
        world.tick_shulker_box(&dimension, shulker_pos, &mut Vec::new());
    }
    assert_eq!(east_side(&world), Some("side"));
}
