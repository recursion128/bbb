use crate::geometry::{collect_block_collision_shapes_checked, Aabb};
use bbb_protocol::packets::BlockPos;
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

#[test]
fn whole_shapes_preserve_nonintersecting_fragments_of_a_hit_shape() {
    let hit = BlockPos { x: 0, y: 0, z: 0 };
    let inside = Aabb {
        min_x: 0.2,
        min_y: 0.2,
        min_z: 0.2,
        max_x: 0.4,
        max_y: 0.4,
        max_z: 0.4,
    };
    let outside = Aabb {
        min_x: 2.0,
        max_x: 3.0,
        ..inside
    };
    let hits = collect_block_collision_shapes_checked(
        query(),
        |_| Some(false),
        |pos| {
            Some(if pos == hit {
                vec![inside, outside]
            } else {
                Vec::new()
            })
        },
    )
    .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].0, hit);
    assert_eq!(hits[0].1.boxes(), vec![inside, outside]);
    assert_eq!(
        hits[0].1.bounds(),
        Some(Aabb {
            max_x: 3.0,
            ..inside
        })
    );
}

#[test]
fn whole_shapes_use_native_non_cube_epsilon_and_full_cube_strict_overlap() {
    let hit = BlockPos { x: 0, y: 0, z: 0 };
    let unit = Aabb::of_block(hit);
    for (bounds, q, expected) in [
        (
            Aabb { max_x: 0.5, ..unit },
            Aabb {
                min_x: 0.5 - 0.5e-7,
                max_x: 0.9,
                ..query()
            },
            0,
        ),
        (
            Aabb { max_x: 0.5, ..unit },
            Aabb {
                min_x: 0.5 - 2.0e-7,
                max_x: 0.9,
                ..query()
            },
            1,
        ),
        (
            unit,
            Aabb {
                min_x: 1.0 - 0.5e-7,
                max_x: 1.9,
                ..query()
            },
            1,
        ),
        (
            unit,
            Aabb {
                min_x: 1.0,
                max_x: 1.9,
                ..query()
            },
            0,
        ),
    ] {
        let hits = collect_block_collision_shapes_checked(
            q,
            |_| Some(false),
            |pos| Some(if pos == hit { vec![bounds] } else { Vec::new() }),
        )
        .unwrap();
        assert_eq!(hits.len(), expected);
    }
}

#[test]
fn whole_shapes_checked_collection_keeps_unknown_after_a_known_hit() {
    let hit = BlockPos { x: 0, y: 0, z: 0 };
    assert!(collect_block_collision_shapes_checked(
        query(),
        |_| Some(false),
        |pos| {
            if pos == hit {
                Some(vec![Aabb::of_block(hit)])
            } else if pos.z > 0 {
                None
            } else {
                Some(Vec::new())
            }
        }
    )
    .is_none());
    assert!(
        collect_block_collision_shapes_checked(query(), |_| None, |_| Some(Vec::new())).is_none()
    );
    assert!(collect_block_collision_shapes_checked(
        query(),
        |_| Some(false),
        |_| Some(vec![Aabb {
            max_x: f64::NAN,
            ..query()
        }])
    )
    .is_none());
}

#[test]
fn whole_shapes_cursor_skips_corners_and_reads_piston_identity_before_edge_shape() {
    let identities = RefCell::new(Vec::new());
    let shapes = RefCell::new(Vec::new());
    collect_block_collision_shapes_checked(
        query(),
        |pos| {
            assert_eq!(
                [pos.x, pos.y, pos.z]
                    .into_iter()
                    .filter(|v| v.abs() == 1)
                    .count(),
                2
            );
            identities.borrow_mut().push(pos);
            Some(pos.x == -1)
        },
        |pos| {
            let boundaries = [pos.x, pos.y, pos.z]
                .into_iter()
                .filter(|v| v.abs() == 1)
                .count();
            assert_ne!(boundaries, 3);
            if boundaries == 2 {
                assert_eq!(pos.x, -1);
                assert!(identities.borrow().contains(&pos));
            }
            shapes.borrow_mut().push(pos);
            Some(Vec::new())
        },
    )
    .unwrap();
    assert_eq!(identities.borrow().len(), 12);
    assert_eq!(shapes.borrow().len(), 11);
    assert!(shapes
        .borrow()
        .windows(2)
        .all(|pair| (pair[0].z, pair[0].y, pair[0].x) < (pair[1].z, pair[1].y, pair[1].x)));
}

#[test]
fn whole_shapes_cursor_rejects_unbounded_query_before_reading_world() {
    for q in [
        Aabb {
            min_x: f64::NEG_INFINITY,
            ..query()
        },
        Aabb {
            max_x: f64::from(i32::MAX),
            ..query()
        },
        Aabb {
            max_x: 2_000_000.0,
            ..query()
        },
    ] {
        assert!(collect_block_collision_shapes_checked(
            q,
            |_| panic!("identity read"),
            |_| panic!("shape read")
        )
        .is_none());
    }
}

#[test]
fn whole_shapes_keep_provider_coordinates_without_shapes_create_rounding() {
    let hit = BlockPos { x: 0, y: 0, z: 0 };
    let bounds = Aabb {
        min_x: 0.5e-7,
        max_x: 1.0 + 0.5e-7,
        min_y: 0.0,
        max_y: 1.0,
        min_z: 0.0,
        max_z: 1.0,
    };
    let hits = collect_block_collision_shapes_checked(
        query(),
        |_| Some(false),
        |pos| Some(if pos == hit { vec![bounds] } else { Vec::new() }),
    )
    .unwrap();
    assert_eq!(hits[0].1.boxes(), vec![bounds]);
}
