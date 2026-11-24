use super::*;

#[test]
fn union_encloses_both_boxes_with_independent_extents() {
    let left = Aabb {
        min_x: -4.0,
        min_y: 2.0,
        min_z: 8.0,
        max_x: -1.0,
        max_y: 5.0,
        max_z: 9.0,
    };
    let right = Aabb {
        min_x: -2.0,
        min_y: -3.0,
        min_z: 1.0,
        max_x: 6.0,
        max_y: 4.0,
        max_z: 7.0,
    };
    let expected = Aabb {
        min_x: -4.0,
        min_y: -3.0,
        min_z: 1.0,
        max_x: 6.0,
        max_y: 5.0,
        max_z: 9.0,
    };
    assert_eq!(left.minmax(right), expected);
    assert_eq!(right.minmax(left), expected);
    assert_eq!(expected.minmax(left), expected);
}

#[test]
fn zero_bounds_keep_negative_minima_and_positive_maxima_in_both_orders() {
    let positive = Aabb {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 0.0,
        max_y: 0.0,
        max_z: 0.0,
    };
    let negative = Aabb {
        min_x: -0.0,
        min_y: -0.0,
        min_z: -0.0,
        max_x: -0.0,
        max_y: -0.0,
        max_z: -0.0,
    };
    for result in [positive.minmax(negative), negative.minmax(positive)] {
        assert_eq!(
            [
                result.min_x.to_bits(),
                result.min_y.to_bits(),
                result.min_z.to_bits()
            ],
            [0x8000_0000_0000_0000; 3]
        );
        assert_eq!(
            [
                result.max_x.to_bits(),
                result.max_y.to_bits(),
                result.max_z.to_bits()
            ],
            [0; 3]
        );
    }
}

#[test]
fn nan_bounds_propagate_from_either_operand_without_poisoning_other_axes() {
    let nan = f64::from_bits(0x7ff8_0000_0000_1234);
    let finite = Aabb {
        min_x: 1.0,
        min_y: 2.0,
        min_z: 3.0,
        max_x: 4.0,
        max_y: 5.0,
        max_z: 6.0,
    };
    let bad_x = Aabb {
        min_x: nan,
        min_y: -2.0,
        min_z: 4.0,
        max_x: nan,
        max_y: 8.0,
        max_z: 9.0,
    };
    for result in [finite.minmax(bad_x), bad_x.minmax(finite)] {
        assert_eq!(result.min_x.to_bits(), 0x7ff8_0000_0000_1234);
        assert_eq!(result.max_x.to_bits(), 0x7ff8_0000_0000_1234);
        assert_eq!(
            [result.min_y, result.min_z, result.max_y, result.max_z],
            [-2.0, 3.0, 8.0, 9.0]
        );
    }
}

#[test]
fn infinite_outer_limits_preserve_finite_bounds_on_other_axes() {
    let finite = Aabb {
        min_x: -1.0,
        min_y: -2.0,
        min_z: -3.0,
        max_x: 1.0,
        max_y: 2.0,
        max_z: 3.0,
    };
    let infinite = Aabb {
        min_x: f64::NEG_INFINITY,
        min_y: 0.0,
        min_z: 0.0,
        max_x: f64::INFINITY,
        max_y: 1.0,
        max_z: 1.0,
    };
    let result = finite.minmax(infinite);
    assert_eq!(result.min_x.to_bits(), 0xfff0_0000_0000_0000);
    assert_eq!(result.max_x.to_bits(), 0x7ff0_0000_0000_0000);
    assert_eq!(
        [result.min_y, result.min_z, result.max_y, result.max_z],
        [-2.0, -3.0, 2.0, 3.0]
    );
}
