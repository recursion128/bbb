use super::WorldBorder;
use crate::geometry::Aabb;
use bbb_protocol::packets::Vec3d;

fn border(center: f64, size: f64) -> WorldBorder {
    WorldBorder::from_settings(center, center, 0.2, 5.0, 5, 300, size, 0, size, 0)
}

fn source(x: f64, z: f64) -> Vec3d {
    Vec3d { x, y: 64.0, z }
}

fn bounds(min_x: f64, max_x: f64, min_z: f64, max_z: f64) -> Aabb {
    Aabb {
        min_x,
        max_x,
        min_y: 64.0,
        max_y: 65.8,
        min_z,
        max_z,
    }
}

#[test]
fn border_collision_strict_edges_and_far_outside_gate() {
    let border = border(0.0, 10.0);
    assert!(border.no_collision_at(source(0.0, 0.0), bounds(-0.3, 0.3, -0.3, 0.3), 0));
    for (box_, actor, expected) in [
        (bounds(4.4, 5.0, -0.3, 0.3), source(4.7, 0.0), true),
        (
            bounds(4.4, 5.0_f64.next_up(), -0.3, 0.3),
            source(4.7, 0.0),
            false,
        ),
        (bounds(-5.0, -4.4, -0.3, 0.3), source(-4.7, 0.0), true),
        (
            bounds(-5.0_f64.next_up(), -4.4, -0.3, 0.3),
            source(-4.7, 0.0),
            false,
        ),
        (bounds(-0.3, 0.3, 4.4, 5.1), source(0.0, 4.7), false),
        (bounds(5.1, 5.7, -0.3, 0.3), source(5.4, 0.0), false),
        (bounds(20.0, 20.6, -0.3, 0.3), source(20.3, 0.0), true),
        (bounds(5.7, 6.3, -0.3, 0.3), source(6.0, 0.0), true),
        (bounds(-6.3, -5.7, -0.3, 0.3), source(-6.0, 0.0), false),
    ] {
        assert_eq!(
            border.no_collision_at(actor, box_, 0),
            expected,
            "{actor:?} {box_:?}"
        );
    }
}

#[test]
fn border_collision_rounds_fractional_shape_outwards() {
    let border = border(0.25, 9.0);
    assert!(border.no_collision_at(source(4.6, 0.0), bounds(4.3, 4.9, -0.3, 0.3), 0));
    assert!(border.no_collision_at(source(4.7, 0.0), bounds(4.4, 5.0, -0.3, 0.3), 0));
    assert!(!border.no_collision_at(source(4.8, 0.0), bounds(4.5, 5.1, -0.3, 0.3), 0));
    assert!(border.no_collision_at(source(-4.7, 0.0), bounds(-5.0, -4.4, -0.3, 0.3), 0));
    assert!(!border.no_collision_at(source(-4.8, 0.0), bounds(-5.1, -4.5, -0.3, 0.3), 0));
}

#[test]
fn border_collision_uses_live_lerp_and_actor_gate() {
    let border = WorldBorder::from_settings(0.0, 0.0, 0.2, 5.0, 5, 300, 10.0, 10, 2.0, 100);
    let actor = source(2.8, 0.0);
    let box_ = bounds(2.5, 3.1, -0.3, 0.3);
    assert!(border.no_collision_at(actor, box_, 100));
    assert!(!border.no_collision_at(actor, box_, 105));
    assert!(border.no_collision_at(actor, box_, 110));

    let border = self::border(0.0, 10.0);
    let crossing = bounds(4.7, 5.3, -0.3, 0.3);
    assert!(border.no_collision_at(source(3.0, 0.0), crossing, 0));
    assert!(!border.no_collision_at(source(3.0_f64.next_up(), 0.0), crossing, 0));
    let wide = bounds(2.5, 5.5, -0.3, 0.3);
    assert!(!border.no_collision_at(source(3.0, 0.0), wide, 0));
}
