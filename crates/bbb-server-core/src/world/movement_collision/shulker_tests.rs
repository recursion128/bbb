use super::*;
use crate::connection::ConnectionId;
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::{ServerWorld, ServerWorldConfig};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::save::Java26BuiltinDimension;

const POS: BlockPos = BlockPos { x: 8, y: 70, z: 8 };
const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn install(world: &mut ServerWorld, name: &str, facing: &str) {
    let state = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        name,
        &[("facing".to_string(), facing.to_string())]
            .into_iter()
            .collect(),
    )
    .unwrap();
    assert!(world.set_block(DIM, POS, state, 3));
}

fn animate(world: &mut ServerWorld, opening: bool, ticks: usize) {
    let shulker = world
        .level_mut(DIM)
        .chunks
        .block_entities_mut()
        .get_mut(POS)
        .unwrap()
        .shulker_box_mut()
        .unwrap();
    assert!(shulker.trigger_event(1, i32::from(opening)));
    for _ in 0..ticks {
        shulker.tick_animation();
    }
}

fn shapes(world: &mut ServerWorld, expected: Aabb) {
    let level = world.level(DIM);
    assert_eq!(
        block_collision_shape(&level.chunks, POS, None),
        vec![expected]
    );
    assert_eq!(
        player_block_collision_shape_checked(
            &level.chunks,
            world.levels.player_at(ConnectionId(1)),
            POS
        )
        .unwrap(),
        vec![expected]
    );
    assert_eq!(
        world.with_stored_hurt_level(&DIM.into(), |_, level| level.block_collision_boxes(POS)),
        vec![expected]
    );
}

#[test]
fn shulker_box_collision_reaches_stored_session_and_swept_queries() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let cube = Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 });
    for (facing, direction, probe) in [
        (
            "down",
            Direction::Down,
            Vec3d {
                x: 8.5,
                y: 69.7,
                z: 8.5,
            },
        ),
        (
            "up",
            Direction::Up,
            Vec3d {
                x: 8.5,
                y: 71.3,
                z: 8.5,
            },
        ),
        (
            "north",
            Direction::North,
            Vec3d {
                x: 8.5,
                y: 70.5,
                z: 7.7,
            },
        ),
        (
            "south",
            Direction::South,
            Vec3d {
                x: 8.5,
                y: 70.5,
                z: 9.3,
            },
        ),
        (
            "west",
            Direction::West,
            Vec3d {
                x: 7.7,
                y: 70.5,
                z: 8.5,
            },
        ),
        (
            "east",
            Direction::East,
            Vec3d {
                x: 9.3,
                y: 70.5,
                z: 8.5,
            },
        ),
    ] {
        install(&mut world, "minecraft:shulker_box", facing);
        shapes(&mut world, cube);
        let bounds = Aabb::of_size(probe, 0.1, 0.1, 0.1);
        assert!(block_collision_boxes(&world.level(DIM).chunks, bounds).is_empty());
        animate(&mut world, true, 10);
        let expected = world
            .level(DIM)
            .chunks
            .block_entities()
            .get(POS)
            .unwrap()
            .shulker_box()
            .unwrap()
            .collision_box(direction);
        shapes(&mut world, expected);
        assert_eq!(
            block_collision_boxes(&world.level(DIM).chunks, bounds),
            vec![expected.moved(Vec3d {
                x: 8.0,
                y: 70.0,
                z: 8.0
            })]
        );
        animate(&mut world, false, 10);
        shapes(&mut world, cube);
        assert!(block_collision_boxes(&world.level(DIM).chunks, bounds).is_empty());
    }
}

#[test]
fn shulker_box_collision_covers_colours_and_absent_payload_fallback() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let mut count = 0;
    for block in bbb_protocol::block_types::VANILLA_BLOCK_TYPES {
        if block.block_entity_type_id
            != Some(bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SHULKER_BOX_ID)
        {
            continue;
        }
        count += 1;
        let name = block.resource_id;
        install(&mut world, name, "up");
        animate(&mut world, true, 10);
        assert_eq!(
            block_collision_shape(&world.level(DIM).chunks, POS, None)[0].max_y,
            1.5
        );
        assert!(world.level_mut(DIM).chunks.block_entities_mut().remove(POS));
        assert_eq!(
            block_collision_shape(&world.level(DIM).chunks, POS, None),
            vec![Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })]
        );
    }
    assert_eq!(count, 17);
}
