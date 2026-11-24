use super::*;

fn context(grace: i32, height: f64) -> FallImpulseContext {
    FallImpulseContext {
        reset_grace_time: grace,
        impact_position: Some(Vec3d {
            x: 1.0,
            y: height,
            z: 2.0,
        }),
    }
}

#[test]
fn impulse_ignore_replaces_impact_extends_grace_and_false_retains_impact() {
    let mut state = FallImpulseContext::default();
    let first = Vec3d {
        x: 1.0,
        y: 40.0,
        z: 2.0,
    };
    state.set_ignore_fall_damage(true, first);
    assert_eq!(state, context(40, 40.0));
    state.apply_grace(80);
    state.apply_grace(10);
    let next = Vec3d { y: 60.0, ..first };
    state.set_ignore_fall_damage(true, next);
    assert_eq!(state, context(80, 60.0));
    state.set_ignore_fall_damage(false, first);
    assert_eq!(state, context(0, 60.0));
    assert!(!state.is_in_grace());
    assert!(state.try_reset());
    assert_eq!(state, FallImpulseContext::default());
}

#[test]
fn impulse_tick_counts_grace_down_and_does_not_clear_at_zero() {
    for (initial, expected) in [(0, 0), (1, 0), (40, 39)] {
        let mut ticks = initial;
        tick_impulse_grace(&mut ticks);
        assert_eq!(ticks, expected);
        let mut state = context(initial, 40.0);
        state.tick();
        assert_eq!(state, context(expected, 40.0));
        assert_eq!(state.is_in_grace(), expected > 0);
    }
}

#[test]
fn impulse_distance_adjustment_retains_grace_or_resets_after_its_expiry() {
    for grace in [40, -1, 0] {
        let mut state = context(grace, 100.0);
        assert_eq!(state.adjust_fall_distance(80.0, 90.0), (10.0, grace == 0));
        assert_eq!(
            state,
            if grace == 0 {
                FallImpulseContext::default()
            } else {
                context(grace, 100.0)
            }
        );
    }
    let mut state = context(40, 100.0);
    assert_eq!(state.adjust_fall_distance(2.0, 90.0), (2.0, false));
    for current_y in [100.0, 105.0] {
        let mut state = context(40, 100.0);
        assert_eq!(
            state.adjust_fall_distance(80.0, current_y),
            (100.0 - current_y, true)
        );
        assert_eq!(state, FallImpulseContext::default());
    }
}

#[test]
fn impulse_without_impact_keeps_distance_and_does_not_execute_reset() {
    for grace in [-1, 0, 40] {
        let mut state = FallImpulseContext {
            reset_grace_time: grace,
            impact_position: None,
        };
        assert_eq!(state.adjust_fall_distance(-2.0, 40.0), (-2.0, false));
        assert_eq!(state.reset_grace_time, grace);
    }
}
