use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::packets::{BlockPos, Direction};

use super::{default_state, state_of, NativeWorld};
use crate::block_behavior::{can_survive_at, collision_shape_full_block_at};
use bbb_server_types::block_queries::{
    collision_face_full_at, collision_face_nonempty_at, collision_shape_empty_at, face_sturdy_at,
    multiface_can_attach_to_at,
};

const ORIGIN: BlockPos = BlockPos { x: 0, y: 70, z: 0 };

#[test]
fn empty_context_shapes_answer_block_faces() {
    let stone = default_state("minecraft:stone");
    assert!(!collision_shape_empty_at(stone, ORIGIN));
    assert!(collision_shape_full_block_at(stone, ORIGIN));
    assert!(face_sturdy_at(
        stone,
        ORIGIN,
        Direction::Up,
        SupportType::Full
    ));

    let scaffolding = default_state("minecraft:scaffolding");
    assert!(face_sturdy_at(
        scaffolding,
        ORIGIN,
        Direction::Up,
        SupportType::Full
    ));
    assert!(collision_face_nonempty_at(
        scaffolding,
        ORIGIN,
        Direction::Up
    ));
    assert!(!collision_shape_empty_at(scaffolding, ORIGIN));
    assert!(!collision_shape_full_block_at(scaffolding, ORIGIN));
    let powder_snow = default_state("minecraft:powder_snow");
    assert!(!face_sturdy_at(
        powder_snow,
        ORIGIN,
        Direction::Up,
        SupportType::Center,
    ));
    assert!(!collision_face_nonempty_at(
        powder_snow,
        ORIGIN,
        Direction::Up
    ));
    assert!(collision_shape_empty_at(powder_snow, ORIGIN));
    assert!(!collision_shape_full_block_at(powder_snow, ORIGIN));

    let mut bamboo_center = [false; 2];
    let bamboo = default_state("minecraft:bamboo");
    for x in -32..=32 {
        for z in -32..=32 {
            let pos = BlockPos { x, y: 64, z };
            let supported = face_sturdy_at(bamboo, pos, Direction::Up, SupportType::Center);
            bamboo_center[usize::from(supported)] = true;
        }
    }
    assert_eq!(bamboo_center, [true, true]);
    assert!(!collision_face_nonempty_at(
        bamboo,
        ORIGIN,
        Direction::North
    ));
    assert!(!collision_shape_empty_at(bamboo, ORIGIN));
    assert!(!multiface_can_attach_to_at(bamboo, ORIGIN, Direction::Down));
}

#[test]
fn dynamic_shapes_pointed_dripstone_tracks_tip_face_and_center_support() {
    for (vertical, down, up) in [("up", true, false), ("down", false, true)] {
        let state = state_of(
            "minecraft:pointed_dripstone",
            &[
                ("thickness", "tip"),
                ("vertical_direction", vertical),
                ("waterlogged", "false"),
            ],
        );
        assert_eq!(
            collision_face_nonempty_at(state, ORIGIN, Direction::Down),
            down
        );
        assert_eq!(collision_face_nonempty_at(state, ORIGIN, Direction::Up), up);
        assert_eq!(
            face_sturdy_at(state, ORIGIN, Direction::Down, SupportType::Center),
            down
        );
        assert!(!face_sturdy_at(
            state,
            ORIGIN,
            Direction::Up,
            SupportType::Full
        ));
    }
}

#[test]
fn live_can_survive_uses_dynamic_support() {
    let snow = default_state("minecraft:snow");
    let support_pos = BlockPos {
        x: ORIGIN.x,
        y: ORIGIN.y - 1,
        z: ORIGIN.z,
    };
    let mut world = NativeWorld::new();
    world.set(support_pos, default_state("minecraft:scaffolding"));
    assert!(world.with_level(|level| can_survive_at(snow, ORIGIN, level)));
    world.set(support_pos, default_state("minecraft:powder_snow"));
    assert!(!world.with_level(|level| can_survive_at(snow, ORIGIN, level)));
}

#[test]
fn block_entity_dynamic_shapes_answer_their_resting_shape() {
    let moving = default_state("minecraft:moving_piston");
    let shulker = default_state("minecraft:shulker_box");
    assert!(collision_shape_empty_at(moving, ORIGIN));
    assert!(!collision_face_full_at(moving, ORIGIN, Direction::Up));
    assert!(face_sturdy_at(
        shulker,
        ORIGIN,
        Direction::North,
        SupportType::Full
    ));
    assert!(collision_shape_full_block_at(shulker, ORIGIN));
}
