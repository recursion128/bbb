use crate::geometry::{
    collect_block_collisions, collect_block_collisions_with_moving_pistons, collide_with_boxes,
    static_block_collision_boxes, Aabb,
};
use bbb_protocol::packets::{BlockPos, Vec3d};
use std::cell::RefCell;

fn query() -> Aabb {
    Aabb {
        min_x: 0.1,
        min_y: 0.1,
        min_z: 0.1,
        max_x: 0.9,
        max_y: 0.9,
        max_z: 0.9,
    }
}

fn boundaries(pos: BlockPos) -> usize {
    [pos.x, pos.y, pos.z]
        .into_iter()
        .filter(|axis| axis.abs() == 1)
        .count()
}

fn overlap(pos: BlockPos) -> Vec<Aabb> {
    vec![Aabb {
        min_x: 0.2 - f64::from(pos.x),
        min_y: 0.2 - f64::from(pos.y),
        min_z: 0.2 - f64::from(pos.z),
        max_x: 0.8 - f64::from(pos.x),
        max_y: 0.8 - f64::from(pos.y),
        max_z: 0.8 - f64::from(pos.z),
    }]
}

#[test]
fn moving_piston_cursor_all_twelve_edges_require_identity_before_shapes() {
    let identities = RefCell::new(Vec::new());
    let boxes = RefCell::new(Vec::new());
    let hits = collect_block_collisions_with_moving_pistons(
        query(),
        |pos| {
            assert_eq!(boundaries(pos), 2);
            identities.borrow_mut().push(pos);
            true
        },
        |pos| {
            boxes.borrow_mut().push(pos);
            overlap(pos)
        },
    );
    assert_eq!(identities.borrow().len(), 12);
    assert_eq!(hits.len(), 19);
    assert!(boxes.borrow().iter().all(|&pos| boundaries(pos) != 3));
    assert_eq!(
        hits.iter().map(|&(pos, _)| pos).collect::<Vec<_>>(),
        *boxes.borrow()
    );
    let mut ordered = boxes.borrow().clone();
    ordered.sort_by_key(|pos| (pos.z, pos.y, pos.x));
    assert_eq!(ordered, *boxes.borrow());
}

#[test]
fn moving_piston_cursor_nonmember_edges_never_query_shapes_and_legacy_is_unchanged() {
    let shapes = |pos| {
        assert!(boundaries(pos) < 2);
        overlap(pos)
    };
    let expected = collect_block_collisions(query(), shapes);
    let actual = collect_block_collisions_with_moving_pistons(query(), |_| false, shapes);
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 7);
}

#[test]
fn moving_piston_cursor_identity_does_not_bypass_overlap_or_touching_boundary() {
    let moving = BlockPos { x: 1, y: -1, z: 0 };
    for (min_x, expected) in [(0.9, 0), (0.8, 1), (1.1, 0)] {
        let hits = collect_block_collisions_with_moving_pistons(
            query(),
            |pos| pos == moving,
            |pos| {
                if pos != moving {
                    return vec![];
                }
                vec![Aabb {
                    min_x: min_x - 1.0,
                    max_x: 0.5,
                    min_y: 1.2,
                    max_y: 1.8,
                    min_z: 0.2,
                    max_z: 0.8,
                }]
            },
        );
        assert_eq!(hits.len(), expected);
    }
}

#[test]
fn moving_piston_cursor_horizontal_fence_fragment_clips_two_boundary_landing() {
    let moving = BlockPos { x: 9, y: 63, z: 8 };
    let fence = super::state("minecraft:oak_fence", &super::cross(&[]));
    let shapes: Vec<_> = static_block_collision_boxes(fence)
        .expect("the built-in fence has a static shape")
        .into_iter()
        .map(|shape| {
            shape.moved(Vec3d {
                x: -0.5,
                y: 0.0,
                z: 0.0,
            })
        })
        .collect();
    let body = Aabb {
        min_x: 8.05,
        max_x: 8.95,
        min_y: 64.6,
        max_y: 66.4,
        min_z: 8.05,
        max_z: 8.95,
    };
    let delta = Vec3d {
        x: 0.0,
        y: -0.5,
        z: 0.0,
    };
    let lookup = |pos| {
        if pos == moving {
            shapes.clone()
        } else {
            vec![]
        }
    };
    let legacy = collect_block_collisions(body.expand_towards(delta), lookup);
    assert!(legacy.is_empty());
    let hits = collect_block_collisions_with_moving_pistons(
        body.expand_towards(delta),
        |pos| pos == moving,
        lookup,
    );
    assert!(!hits.is_empty());
    assert!(hits.iter().all(|&(pos, _)| pos == moving));
    let boxes: Vec<_> = hits.into_iter().map(|(_, shape)| shape).collect();
    assert_eq!(collide_with_boxes(delta, body, &boxes).y, 64.5 - 64.6);
}
