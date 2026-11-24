use super::*;

fn point(x: f64, y: f64, z: f64) -> Vec3d {
    Vec3d { x, y, z }
}
fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}
const fn cube() -> Aabb {
    Aabb {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    }
}

const CUBE: Aabb = cube();

#[test]
fn shared_block_clip_zero_and_short_segments_miss_without_fabricating_hits() {
    let from = point(0.5, 0.5, 0.5);
    assert_eq!(
        clip_with(from, from, |_| panic!("zero segment visits no cells")),
        Some(None)
    );
    assert_eq!(shape_clip(&[cube()], from, from, pos(0, 0, 0)), None);
    let to = point(0.5001, 0.5, 0.5);
    assert_eq!(
        clip_with(from, to, |_| Some((vec![cube()], None))),
        Some(None)
    );
}

#[test]
fn shared_block_clip_inside_uses_thousandth_point_half_open_bounds_and_native_direction_ties() {
    let from = point(0.5, 0.5, 0.5);
    let hit = shape_clip(&[cube()], from, point(1.5, 1.5, 1.5), pos(0, 0, 0)).unwrap();
    assert_eq!(hit.location, point(0.501, 0.501, 0.501));
    assert_eq!(hit.direction, Direction::Down);
    assert_eq!(
        shape_clip(
            &[cube()],
            point(0.999, 0.5, 0.5),
            point(1.999, 0.5, 0.5),
            pos(0, 0, 0)
        ),
        None
    );
    let hit = shape_clip(
        &[cube()],
        point(-0.001, 0.5, 0.5),
        point(0.999, 0.5, 0.5),
        pos(0, 0, 0),
    )
    .unwrap();
    assert_eq!(hit.location, point(0.0, 0.5, 0.5));
}

#[test]
fn shared_block_clip_water_cell_exit_never_becomes_an_entry_hit() {
    let from = point(0.9999, 0.5, 0.5);
    let to = point(2.0, 0.5, 0.5);
    assert_eq!(shape_clip(&[cube()], from, to, pos(0, 0, 0)), None);
    assert_eq!(
        clip_with(from, to, |cell| Some((
            vec![],
            if cell == pos(0, 0, 0) {
                Some(&CUBE)
            } else {
                None
            }
        ))),
        Some(None)
    );
    assert_eq!(
        shape_clip(
            &[cube()],
            point(0.0001, 0.5, 0.5),
            point(-1.0, 0.5, 0.5),
            pos(0, 0, 0)
        ),
        None
    );
}

#[test]
fn shared_block_clip_entry_faces_and_nearest_box_are_independent_of_box_order() {
    let tests = [
        (
            point(-1.0, 0.5, 0.5),
            point(2.0, 0.5, 0.5),
            Direction::West,
            point(0.0, 0.5, 0.5),
        ),
        (
            point(2.0, 0.5, 0.5),
            point(-1.0, 0.5, 0.5),
            Direction::East,
            point(1.0, 0.5, 0.5),
        ),
        (
            point(0.5, -1.0, 0.5),
            point(0.5, 2.0, 0.5),
            Direction::Down,
            point(0.5, 0.0, 0.5),
        ),
        (
            point(0.5, 2.0, 0.5),
            point(0.5, -1.0, 0.5),
            Direction::Up,
            point(0.5, 1.0, 0.5),
        ),
        (
            point(0.5, 0.5, -1.0),
            point(0.5, 0.5, 2.0),
            Direction::North,
            point(0.5, 0.5, 0.0),
        ),
        (
            point(0.5, 0.5, 2.0),
            point(0.5, 0.5, -1.0),
            Direction::South,
            point(0.5, 0.5, 1.0),
        ),
    ];
    for (from, to, direction, location) in tests {
        let hit = shape_clip(&[cube()], from, to, pos(0, 0, 0)).unwrap();
        assert_eq!(hit.direction, direction);
        assert_eq!(hit.location, location);
    }
    let near = Aabb {
        min_x: 0.25,
        max_x: 0.5,
        ..cube()
    };
    let far = Aabb {
        min_x: 0.75,
        ..cube()
    };
    for boxes in [[near, far], [far, near]] {
        let hit = shape_clip(
            &boxes,
            point(-1.0, 0.5, 0.5),
            point(2.0, 0.5, 0.5),
            pos(0, 0, 0),
        )
        .unwrap();
        assert_eq!(hit.location.x, 0.25);
    }
}

#[test]
fn shared_block_clip_aabb_epsilon_axis_gate_edges_and_strict_nearest_ties() {
    let shape = Aabb {
        min_x: 0.5e-7,
        min_y: -1.0,
        max_y: 2.0,
        ..cube()
    };
    assert_eq!(
        shape_clip(
            &[shape],
            point(0.0, 0.0, 0.5),
            point(1.0e-7, 1.0, 0.5),
            pos(0, 0, 0)
        ),
        None
    );
    assert_eq!(
        shape_clip(
            &[shape],
            point(0.0, 0.0, 0.5),
            point(1.00001e-7, 1.0, 0.5),
            pos(0, 0, 0)
        )
        .unwrap()
        .direction,
        Direction::West
    );
    assert!(shape_clip(
        &[cube()],
        point(-1.0, 1.0 + 0.5e-7, 0.5),
        point(2.0, 1.0 + 0.5e-7, 0.5),
        pos(0, 0, 0)
    )
    .is_some());
    assert!(shape_clip(
        &[cube()],
        point(-1.0, 1.0 + 2.0e-7, 0.5),
        point(2.0, 1.0 + 2.0e-7, 0.5),
        pos(0, 0, 0)
    )
    .is_none());
    let hit = shape_clip(
        &[cube()],
        point(-1.0, -1.0, -1.0),
        point(2.0, 2.0, 2.0),
        pos(0, 0, 0),
    )
    .unwrap();
    assert_eq!(hit.direction, Direction::West);
    assert!(shape_clip(
        &[cube()],
        point(-1.0, 0.5, 0.5),
        point(0.0, 0.5, 0.5),
        pos(0, 0, 0)
    )
    .is_none());
}

#[test]
fn shared_block_clip_traverse_nudge_and_strict_axis_tie_order() {
    let mut visits = vec![];
    assert_eq!(
        clip_with(point(0.5, 0.5, 0.5), point(1.5, 1.5, 1.5), |cell| {
            visits.push(cell);
            Some((vec![], None))
        }),
        Some(None)
    );
    assert_eq!(
        visits,
        vec![pos(0, 0, 0), pos(0, 0, 1), pos(0, 1, 1), pos(1, 1, 1)]
    );
    visits.clear();
    assert_eq!(
        clip_with(point(1.0, 0.5, 0.5), point(0.0, 0.5, 0.5), |cell| {
            visits.push(cell);
            Some((vec![], None))
        }),
        Some(None)
    );
    assert_eq!(visits, vec![pos(1, 0, 0), pos(0, 0, 0), pos(-1, 0, 0)]);
}

#[test]
fn shared_block_clip_selects_nearest_block_or_fluid_and_prefers_block_on_ties() {
    let from = point(0.0, 0.0, 0.5);
    let to = point(1.0, 1.0, 0.5);
    let block = Aabb {
        min_x: 0.25,
        ..cube()
    };
    let fluid = Aabb {
        min_y: 0.25,
        ..cube()
    };
    let hit = clip_with(from, to, |_| Some((vec![block], Some(&fluid))))
        .unwrap()
        .unwrap();
    assert_eq!(hit.location, point(0.25, 0.25, 0.5));
    assert_eq!(hit.direction, Direction::West);
    let fluid = Aabb {
        min_y: 0.125,
        ..cube()
    };
    let hit = clip_with(from, to, |_| Some((vec![block], Some(&fluid))))
        .unwrap()
        .unwrap();
    assert_eq!(hit.location, point(0.125, 0.125, 0.5));
    assert_eq!(hit.direction, Direction::Down);
}

#[test]
fn shared_block_clip_first_hit_and_unknown_cell_stop_before_later_fluid() {
    let mut visits = vec![];
    let from = point(0.5, 0.5, 0.5);
    let to = point(3.5, 0.5, 0.5);
    let hit = clip_with(from, to, |cell| {
        visits.push(cell);
        Some((
            if cell.x == 1 { vec![cube()] } else { vec![] },
            if cell.x == 2 { Some(&CUBE) } else { None },
        ))
    })
    .unwrap()
    .unwrap();
    assert_eq!(hit.pos, pos(1, 0, 0));
    assert_eq!(visits, vec![pos(0, 0, 0), pos(1, 0, 0)]);
    visits.clear();
    assert_eq!(
        clip_with(from, to, |cell| {
            visits.push(cell);
            if cell.x == 1 {
                None
            } else {
                Some((vec![], Some(&CUBE)))
            }
        }),
        Some(Some(ClipHit {
            pos: pos(0, 0, 0),
            direction: Direction::West,
            location: point(0.503, 0.5, 0.5)
        }))
    );
    visits.clear();
    assert_eq!(
        clip_with(from, to, |cell| {
            visits.push(cell);
            if cell.x == 1 {
                None
            } else {
                Some((vec![], if cell.x == 2 { Some(&CUBE) } else { None }))
            }
        }),
        None
    );
    assert_eq!(visits, vec![pos(0, 0, 0), pos(1, 0, 0)]);
}

#[test]
fn shared_block_clip_invalid_coordinates_and_derived_steps_refuse_without_visits() {
    for (from, to) in [
        (point(f64::NAN, 0.5, 0.5), point(2.0, 0.5, 0.5)),
        (
            point(f64::INFINITY, 0.5, 0.5),
            point(f64::INFINITY, 0.5, 0.5),
        ),
        (point(0.5, 0.5, 0.5), point(f64::NEG_INFINITY, 0.5, 0.5)),
        (point(f64::MAX, 0.5, 0.5), point(0.5, 0.5, 0.5)),
        (
            point(f64::from(i32::MIN) - 1.0, 0.5, 0.5),
            point(0.5, 0.5, 0.5),
        ),
        (
            point(0.5, 0.5, 0.5),
            point(f64::from(i32::MAX) + 1.0, 0.5, 0.5),
        ),
        (
            point(f64::from(i32::MIN), 0.5, 0.5),
            point(f64::from(i32::MAX), 0.5, 0.5),
        ),
        (point(0.0, 0.5, 0.0), point(2.0, 0.5, f64::from_bits(1))),
    ] {
        assert_eq!(
            clip_with(from, to, |_| panic!("invalid segment must not visit cells")),
            None
        );
    }
    assert_eq!(
        clip_with(point(0.5, 0.5, 0.5), point(2.5, 0.5, 0.5), |_| Some((
            vec![Aabb {
                max_x: f64::NAN,
                ..cube()
            }],
            None
        ))),
        None
    );
}

#[test]
fn shared_block_clip_has_no_normal_ray_distance_cap_and_accepts_safe_integer_edges() {
    let hit = clip_with(point(0.5, 0.5, 0.5), point(256.5, 0.5, 0.5), |cell| {
        Some((if cell.x == 255 { vec![cube()] } else { vec![] }, None))
    })
    .unwrap()
    .unwrap();
    assert_eq!(hit.pos, pos(255, 0, 0));
    for x in [f64::from(i32::MIN) + 0.5, f64::from(i32::MAX) + 0.25] {
        assert_eq!(
            clip_with(point(x, 0.5, 0.5), point(x + 0.125, 0.5, 0.5), |_| Some((
                vec![],
                None
            ))),
            Some(None)
        );
    }
}
