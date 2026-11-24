use super::*;

const POS: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const POSITION: Vec3d = Vec3d {
    x: -0.3,
    y: 64.2,
    z: 0.5,
};
fn delta(y: f64) -> Vec3d {
    Vec3d { x: 0.2, y, z: -0.4 }
}
fn raw_y(old: f64) -> f64 {
    (old - 0.08) * f64::from(0.98_f32)
}

#[test]
fn honey_contact_slows_descent_and_draws_each_effect() {
    assert!(is_sliding_down(POS, POSITION, 0.6, false, delta(-0.3)));
    assert_eq!(slide_movement(delta(-0.3)).y, raw_y(-0.05));
    let mut draws = [0, 1].into_iter();
    assert_eq!(slide_effects(|| draws.next().unwrap()), (true, false));
}

#[test]
fn honey_contact_does_not_slide_grounded_high_centered_rising_or_wide_bodies() {
    let high = Vec3d {
        y: 64.9375,
        ..POSITION
    };
    let centered = Vec3d { x: 0.5, ..POSITION };
    for (position, width, grounded, delta) in [
        (POSITION, 0.6, true, delta(-0.3)),
        (high, 0.6, false, delta(-0.3)),
        (centered, 0.6, false, delta(-0.3)),
        (POSITION, 0.6, false, delta(raw_y(-0.08))),
        (POSITION, 2.0, false, delta(-0.3)),
    ] {
        assert!(!is_sliding_down(POS, position, width, grounded, delta));
    }
}

#[test]
fn honey_slide_fast_and_slow_boundaries_preserve_horizontal_contract() {
    for old in [-0.130001, -0.13, -0.129999] {
        let input = delta(raw_y(old));
        let actual_old = input.y / f64::from(0.98_f32) + 0.08;
        let expected_factor = if actual_old < -0.13 {
            -0.05 / actual_old
        } else {
            1.0
        };
        let output = slide_movement(input);
        assert_eq!(
            output,
            Vec3d {
                x: input.x * expected_factor,
                y: raw_y(-0.05),
                z: input.z * expected_factor
            }
        );
    }
}
