use super::*;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_OAK_FENCE_GATE_ID, VANILLA_BLOCK_STONE_ID};

fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

fn body(x: f64) -> Aabb {
    Aabb {
        min_x: x - 0.6,
        max_x: x + 0.6,
        min_y: 1.0,
        max_y: 2.8,
        min_z: 0.2,
        max_z: 0.8,
    }
}

fn boxes(at: BlockPos, occupied: &[BlockPos]) -> Vec<Aabb> {
    if occupied.contains(&at) {
        vec![Aabb {
            min_x: 0.0,
            min_y: 0.0,
            min_z: 0.0,
            max_x: 1.0,
            max_y: 1.0,
            max_z: 1.0,
        }]
    } else {
        vec![]
    }
}

#[test]
fn supporting_block_nearest_and_equal_distance_choose_greater_position() {
    for (x, expected) in [
        (0.8, pos(0, 0, 0)),
        (1.0, pos(1, 0, 0)),
        (1.2, pos(1, 0, 0)),
    ] {
        let mut supporting = None;
        let mut no_blocks = false;
        check_body_supporting_block(
            Vec3d { x, y: 1.0, z: 0.5 },
            body(x),
            true,
            None,
            &mut supporting,
            &mut no_blocks,
            &|at| boxes(at, &[pos(0, 0, 0), pos(1, 0, 0)]),
        );
        assert_eq!(supporting, Some(expected));
        assert!(!no_blocks);
    }
    assert!(compare_pos(pos(100, 0, 100), pos(0, 1, 0)));
    assert!(compare_pos(pos(100, 0, 0), pos(0, 0, 1)));
}

#[test]
fn supporting_block_retry_uses_horizontal_movement_only_and_no_blocks_suppresses_it() {
    for was_no_blocks in [false, true] {
        let mut supporting = Some(pos(9, 0, 9));
        let mut no_blocks = was_no_blocks;
        check_body_supporting_block(
            Vec3d {
                x: 2.5,
                y: 1.0,
                z: 0.5,
            },
            body(2.5),
            true,
            Some(Vec3d {
                x: 2.0,
                y: 20.0,
                z: 0.0,
            }),
            &mut supporting,
            &mut no_blocks,
            &|at| boxes(at, &[pos(0, 0, 0)]),
        );
        assert_eq!(
            supporting,
            if was_no_blocks {
                None
            } else {
                Some(pos(0, 0, 0))
            }
        );
        assert_eq!(no_blocks, was_no_blocks);
    }
}

#[test]
fn supporting_block_missing_movement_preserves_support_once_then_airborne_clears() {
    let mut supporting = Some(pos(9, 0, 9));
    let mut no_blocks = false;
    let position = Vec3d {
        x: 0.5,
        y: 1.0,
        z: 0.5,
    };
    check_body_supporting_block(
        position,
        body(0.5),
        true,
        None,
        &mut supporting,
        &mut no_blocks,
        &|_| vec![],
    );
    assert_eq!(supporting, Some(pos(9, 0, 9)));
    assert!(no_blocks);
    check_body_supporting_block(
        position,
        body(0.5),
        true,
        None,
        &mut supporting,
        &mut no_blocks,
        &|_| vec![],
    );
    assert_eq!(supporting, None);
    supporting = Some(pos(9, 0, 9));
    check_body_supporting_block(
        position,
        body(0.5),
        false,
        None,
        &mut supporting,
        &mut no_blocks,
        &|_| panic!("airborne query"),
    );
    assert_eq!(supporting, None);
    assert!(!no_blocks);
}

fn facts(is_fence: bool, is_wall: bool, gate: bool) -> SupportingBlockFacts {
    SupportingBlockFacts {
        state_id: vanilla_block_default_state_id(if gate {
            VANILLA_BLOCK_OAK_FENCE_GATE_ID
        } else {
            VANILLA_BLOCK_STONE_ID
        })
        .unwrap(),
        is_fence,
        is_wall,
    }
}

#[test]
fn supporting_block_on_pos_preserves_tags_and_gate_height_at_exact_offset_boundaries() {
    let position = Vec3d {
        x: 4.5,
        y: 2.5,
        z: -3.5,
    };
    let supporting = pos(7, 1, -8);
    for (offset, facts, expected_y) in [
        (0.2, facts(false, false, false), 2),
        (0.2, facts(true, false, false), 1),
        (0.5, facts(true, false, false), 1),
        (0.500001, facts(true, false, false), 1),
        (0.6, facts(false, true, false), 1),
        (0.6, facts(false, false, true), 1),
        (1.6, facts(false, true, false), 1),
        (1.6, facts(false, false, true), 1),
        (1.6, facts(true, false, false), 0),
    ] {
        assert_eq!(
            body_on_pos(position, Some(supporting), offset, |at| {
                assert_eq!(at, supporting);
                facts
            }),
            pos(7, expected_y, -8)
        );
    }
    let high = Vec3d { y: 3.0, ..position };
    assert_eq!(
        body_on_pos(high, Some(supporting), 0.5, |_| facts(true, false, false)).y,
        1
    );
    assert_eq!(
        body_on_pos(high, Some(supporting), 0.500001, |_| facts(
            true, false, false
        ))
        .y,
        2
    );
}

#[test]
fn supporting_block_on_pos_does_not_query_without_support_or_at_small_offset() {
    let position = Vec3d {
        x: -0.2,
        y: 1.0,
        z: -1.2,
    };
    assert_eq!(
        body_on_pos(position, None, 0.2, |_| panic!("no support")),
        pos(-1, 0, -2)
    );
    let supporting = pos(5, 7, 9);
    for offset in [0.0, 1.0e-5_f32, -0.1, f32::NAN] {
        assert_eq!(
            body_on_pos(position, Some(supporting), offset, |_| panic!(
                "small offset"
            )),
            supporting
        );
    }
}
