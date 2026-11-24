use super::*;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
};
use bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID;
use bbb_protocol::packets::{Direction, Vec3d};

const POS: BlockPos = BlockPos {
    x: 12,
    y: 17,
    z: -8,
};

fn state(distance: u8, bottom: bool, waterlogged: bool) -> i32 {
    vanilla_block_state_id_by_name_and_properties(
        "minecraft:scaffolding",
        &[
            ("distance".to_owned(), distance.to_string()),
            ("bottom".to_owned(), bottom.to_string()),
            ("waterlogged".to_owned(), waterlogged.to_string()),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap()
}

fn context(bottom: f64, descending: bool, placement: bool) -> Context {
    Context::Entity {
        bottom,
        descending,
        placement,
    }
}

fn eighths(boxes: &[Aabb]) -> Vec<[i32; 6]> {
    boxes
        .iter()
        .map(|shape| {
            [
                shape.min_x,
                shape.min_y,
                shape.min_z,
                shape.max_x,
                shape.max_y,
                shape.max_z,
            ]
            .map(|value| {
                assert_eq!((value * 8.0).fract(), 0.0);
                (value * 8.0) as i32
            })
        })
        .collect()
}

#[test]
fn scaffolding_outline_matches_native_consumed_strip_decomposition() {
    let stable = outline_boxes(state(0, false, false), false).unwrap();
    assert_eq!(
        eighths(&stable),
        vec![
            [0, 0, 0, 1, 8, 1],
            [0, 0, 7, 1, 8, 8],
            [7, 0, 0, 8, 8, 1],
            [7, 0, 7, 8, 8, 8],
            [0, 7, 1, 8, 8, 7],
            [1, 7, 0, 7, 8, 1],
            [1, 7, 7, 7, 8, 8],
        ]
    );
    let unstable = outline_boxes(state(7, true, false), false).unwrap();
    assert_eq!(
        eighths(&unstable),
        vec![
            [0, 0, 0, 8, 1, 8],
            [0, 1, 0, 1, 8, 1],
            [0, 1, 7, 1, 8, 8],
            [7, 1, 0, 8, 8, 1],
            [7, 1, 7, 8, 8, 8],
            [0, 7, 1, 8, 8, 7],
            [1, 7, 0, 7, 8, 1],
            [1, 7, 7, 7, 8, 8],
        ]
    );
}

#[test]
fn scaffolding_partition_is_disjoint_and_covers_the_authored_shape_union() {
    for bottom in [false, true] {
        let shapes = outline_boxes(state(7, bottom, false), false).unwrap();
        let volume: f64 = shapes
            .iter()
            .map(|shape| {
                (shape.max_x - shape.min_x)
                    * (shape.max_y - shape.min_y)
                    * (shape.max_z - shape.min_z)
            })
            .sum();
        assert_eq!(volume, if bottom { 19.0 / 64.0 } else { 23.0 / 128.0 });
        for (i, shape) in shapes.iter().enumerate() {
            for other in &shapes[i + 1..] {
                assert!(!shape.intersects(*other));
            }
        }
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    let point = Vec3d {
                        x: (f64::from(x) + 0.5) / 8.0,
                        y: (f64::from(y) + 0.5) / 8.0,
                        z: (f64::from(z) + 0.5) / 8.0,
                    };
                    let count = shapes
                        .iter()
                        .filter(|shape| {
                            point.x >= shape.min_x
                                && point.x < shape.max_x
                                && point.y >= shape.min_y
                                && point.y < shape.max_y
                                && point.z >= shape.min_z
                                && point.z < shape.max_z
                        })
                        .count();
                    let authored =
                        y == 7 || (bottom && y == 0) || (matches!(x, 0 | 7) && matches!(z, 0 | 7));
                    assert_eq!(
                        count,
                        usize::from(authored),
                        "bottom={bottom}, ({x},{y},{z})"
                    );
                }
            }
        }
    }
}

#[test]
fn scaffolding_all_32_states_keep_context_and_outline_branch_order() {
    let stable = outline_boxes(state(0, false, false), false).unwrap();
    let unstable = outline_boxes(state(0, true, false), false).unwrap();
    let floor = vec![Aabb {
        max_y: 0.125,
        ..Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })
    }];
    let full = vec![Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })];
    let mut count = 0;
    for distance in 0..=7 {
        for bottom in [false, true] {
            for waterlogged in [false, true] {
                count += 1;
                let state = state(distance, bottom, waterlogged);
                assert_eq!(
                    collision_boxes(state, POS, Context::Empty),
                    Some(stable.clone())
                );
                assert_eq!(
                    collision_boxes(state, POS, context(19.0, false, false)),
                    Some(stable.clone())
                );
                let bottom_collision = if distance != 0 && bottom {
                    floor.clone()
                } else {
                    vec![]
                };
                assert_eq!(
                    collision_boxes(state, POS, context(19.0, true, false)),
                    Some(bottom_collision.clone())
                );
                assert_eq!(
                    collision_boxes(state, POS, context(17.5, false, false)),
                    Some(bottom_collision)
                );
                assert_eq!(
                    collision_boxes(state, POS, context(16.0, false, false)),
                    Some(vec![])
                );
                for descending in [false, true] {
                    assert_eq!(
                        collision_boxes(state, POS, context(19.0, descending, true)),
                        Some(vec![])
                    );
                }
                assert_eq!(
                    outline_boxes(state, false),
                    Some(if bottom {
                        unstable.clone()
                    } else {
                        stable.clone()
                    })
                );
                assert_eq!(outline_boxes(state, true), Some(full.clone()));
            }
        }
    }
    assert_eq!(count, 32);
}

#[test]
fn scaffolding_entity_bottom_epsilon_is_float_widened_and_strict() {
    let stable = outline_boxes(state(0, false, false), false).unwrap();
    let floor = vec![Aabb {
        max_y: 0.125,
        ..Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })
    }];
    for y in [-17, 0, 17] {
        let pos = BlockPos { y, ..POS };
        let top_limit = f64::from(y) + 1.0 - f64::from(1.0e-5_f32);
        assert_eq!(
            collision_boxes(state(0, true, false), pos, context(top_limit, false, false)),
            Some(vec![])
        );
        assert_eq!(
            collision_boxes(
                state(0, true, false),
                pos,
                context(top_limit.next_down(), false, false)
            ),
            Some(vec![])
        );
        assert_eq!(
            collision_boxes(
                state(0, true, false),
                pos,
                context(top_limit.next_up(), false, false)
            ),
            Some(stable.clone())
        );
        assert_eq!(
            collision_boxes(state(1, true, false), pos, context(top_limit, false, false)),
            Some(floor.clone())
        );
        let lower_limit = f64::from(y) - f64::from(1.0e-5_f32);
        assert_eq!(
            collision_boxes(
                state(1, true, false),
                pos,
                context(lower_limit, false, false)
            ),
            Some(vec![])
        );
        assert_eq!(
            collision_boxes(
                state(1, true, false),
                pos,
                context(lower_limit.next_down(), false, false)
            ),
            Some(vec![])
        );
        assert_eq!(
            collision_boxes(
                state(1, true, false),
                pos,
                context(lower_limit.next_up(), false, false)
            ),
            Some(floor.clone())
        );
    }
    let decimal_boundary = 1.0 - 1.0e-5_f64;
    let native_boundary = 1.0 - f64::from(1.0e-5_f32);
    assert!(native_boundary > decimal_boundary);
    assert_eq!(
        collision_boxes(
            state(0, false, false),
            BlockPos { y: 0, ..POS },
            context((decimal_boundary + native_boundary) / 2.0, false, false)
        ),
        Some(vec![])
    );
}

#[test]
fn scaffolding_block_local_outline_reaches_the_shared_clip_owner() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let stable = outline_boxes(state(7, false, false), false).unwrap();
    let unstable = outline_boxes(state(7, true, false), false).unwrap();
    let from = Vec3d {
        x: 0.5,
        y: 0.0625,
        z: -1.0,
    };
    let to = Vec3d { z: 1.0, ..from };
    assert_eq!(super::super::clip::shape_clip(&stable, from, to, pos), None);
    let hit = super::super::clip::shape_clip(&unstable, from, to, pos).unwrap();
    assert_eq!(hit.direction, Direction::North);
    assert_eq!(hit.location, Vec3d { z: 0.0, ..from });
    let top_from = Vec3d { y: 0.9375, ..from };
    let top_to = Vec3d { y: 0.9375, ..to };
    assert_eq!(
        super::super::clip::shape_clip(&stable, top_from, top_to, pos)
            .unwrap()
            .direction,
        Direction::North
    );
}

#[test]
fn scaffolding_non_scaffolding_and_unknown_state_refuse_before_context_branches() {
    for state in [
        -1,
        i32::MAX,
        vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap(),
    ] {
        assert_eq!(collision_boxes(state, POS, Context::Empty), None);
        assert_eq!(
            collision_boxes(state, POS, context(19.0, false, true)),
            None
        );
        assert_eq!(outline_boxes(state, false), None);
        assert_eq!(outline_boxes(state, true), None);
    }
}
