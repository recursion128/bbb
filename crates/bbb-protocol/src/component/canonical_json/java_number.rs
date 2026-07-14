use num_bigint::BigUint;
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Decimal {
    significand: u64,
    exponent: i32,
}

#[derive(Debug)]
struct PositiveRational {
    numerator: BigUint,
    denominator: BigUint,
}

pub(super) fn java_float_string(value: f32) -> String {
    java_floating_string(
        value.is_sign_negative(),
        value.is_nan(),
        value.is_infinite(),
        value == 0.0,
        || {
            let magnitude = value.abs();
            let mut buffer = ryu::Buffer::new();
            let decimal = parse_shortest_decimal(buffer.format_finite(magnitude));
            adjust_one_digit_decimal(decimal, binary_rational_f32(magnitude), |candidate| {
                decimal_roundtrips_f32(candidate, magnitude)
            })
        },
    )
}

pub(super) fn java_double_string(value: f64) -> String {
    java_floating_string(
        value.is_sign_negative(),
        value.is_nan(),
        value.is_infinite(),
        value == 0.0,
        || {
            let magnitude = value.abs();
            let mut buffer = ryu::Buffer::new();
            let decimal = parse_shortest_decimal(buffer.format_finite(magnitude));
            adjust_one_digit_decimal(decimal, binary_rational_f64(magnitude), |candidate| {
                decimal_roundtrips_f64(candidate, magnitude)
            })
        },
    )
}

fn java_floating_string(
    negative: bool,
    nan: bool,
    infinite: bool,
    zero: bool,
    decimal: impl FnOnce() -> Decimal,
) -> String {
    if nan {
        return "NaN".to_string();
    }
    if infinite {
        return if negative { "-Infinity" } else { "Infinity" }.to_string();
    }
    if zero {
        return if negative { "-0.0" } else { "0.0" }.to_string();
    }
    format_java_decimal(decimal(), negative)
}

fn parse_shortest_decimal(value: &str) -> Decimal {
    let (mantissa, explicit_exponent) =
        value
            .split_once('e')
            .map_or((value, 0), |(mantissa, exponent)| {
                (
                    mantissa,
                    exponent
                        .parse::<i32>()
                        .expect("Ryū emitted an invalid exponent"),
                )
            });
    let fractional_digits = mantissa
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len()) as i32;
    let digits = mantissa.replace('.', "");
    let digits = digits.trim_start_matches('0');
    let mut significand = digits
        .parse::<u64>()
        .expect("Ryū emitted an invalid significand");
    let mut exponent = explicit_exponent - fractional_digits;
    while significand % 10 == 0 {
        significand /= 10;
        exponent += 1;
    }
    Decimal {
        significand,
        exponent,
    }
}

fn adjust_one_digit_decimal(
    decimal: Decimal,
    value: PositiveRational,
    roundtrips: impl Fn(Decimal) -> bool,
) -> Decimal {
    if decimal.significand >= 10 {
        return decimal;
    }

    // Double.toString/Float.toString deliberately widen the candidate set when the shortest
    // decimal has one digit: Java chooses the closest value among all one- and two-digit decimals
    // that round back to the input. Inspect the three neighboring decimal scales because a value
    // rounded to 1eN can lie just below the power-of-ten boundary (for example 9.9e(N-1)).
    let mut candidates = vec![decimal];
    for exponent in decimal.exponent - 2..=decimal.exponent {
        for significand in nearest_decimal_integers(&value, exponent) {
            if (10..=99).contains(&significand) && significand % 10 != 0 {
                let candidate = Decimal {
                    significand,
                    exponent,
                };
                if roundtrips(candidate) && !candidates.contains(&candidate) {
                    candidates.push(candidate);
                }
            }
        }
    }

    candidates
        .into_iter()
        .min_by(|left, right| compare_decimal_distance(&value, *left, *right))
        .expect("the Ryū decimal always remains a Java candidate")
}

fn nearest_decimal_integers(value: &PositiveRational, exponent: i32) -> Vec<u64> {
    let (numerator, denominator) = if exponent >= 0 {
        (
            value.numerator.clone(),
            &value.denominator * decimal_power(exponent as u32),
        )
    } else {
        (
            &value.numerator * decimal_power((-exponent) as u32),
            value.denominator.clone(),
        )
    };
    let floor = &numerator / &denominator;
    let mut nearest = Vec::with_capacity(2);
    push_small_biguint(&mut nearest, &floor);
    if &floor * &denominator != numerator {
        push_small_biguint(&mut nearest, &(floor + 1u8));
    }
    nearest
}

fn push_small_biguint(values: &mut Vec<u64>, value: &BigUint) {
    let digits = value.to_u64_digits();
    if digits.len() <= 1 {
        values.push(digits.first().copied().unwrap_or(0));
    }
}

fn compare_decimal_distance(value: &PositiveRational, left: Decimal, right: Decimal) -> Ordering {
    let (left_difference, left_denominator) = decimal_difference(value, left);
    let (right_difference, right_denominator) = decimal_difference(value, right);
    let ordering =
        (left_difference * right_denominator).cmp(&(right_difference * left_denominator));
    if ordering != Ordering::Equal {
        return ordering;
    }

    // The JDK specification resolves an exact distance tie with an even decimal significand.
    match (left.significand % 2 == 0, right.significand % 2 == 0) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => (left.exponent, left.significand).cmp(&(right.exponent, right.significand)),
    }
}

fn decimal_difference(value: &PositiveRational, decimal: Decimal) -> (BigUint, BigUint) {
    let decimal = decimal_rational(decimal);
    let value_scaled = &value.numerator * &decimal.denominator;
    let decimal_scaled = &decimal.numerator * &value.denominator;
    let difference = if value_scaled >= decimal_scaled {
        value_scaled - decimal_scaled
    } else {
        decimal_scaled - value_scaled
    };
    (difference, decimal.denominator)
}

fn decimal_rational(decimal: Decimal) -> PositiveRational {
    if decimal.exponent >= 0 {
        PositiveRational {
            numerator: BigUint::from(decimal.significand) * decimal_power(decimal.exponent as u32),
            denominator: BigUint::from(1u8),
        }
    } else {
        PositiveRational {
            numerator: BigUint::from(decimal.significand),
            denominator: decimal_power((-decimal.exponent) as u32),
        }
    }
}

fn binary_rational_f32(value: f32) -> PositiveRational {
    let bits = value.to_bits();
    let fraction = bits & 0x007f_ffff;
    let biased_exponent = ((bits >> 23) & 0xff) as i32;
    let (significand, exponent) = if biased_exponent == 0 {
        (u64::from(fraction), -149)
    } else {
        (u64::from((1 << 23) | fraction), biased_exponent - 127 - 23)
    };
    binary_rational(significand, exponent)
}

fn binary_rational_f64(value: f64) -> PositiveRational {
    let bits = value.to_bits();
    let fraction = bits & 0x000f_ffff_ffff_ffff;
    let biased_exponent = ((bits >> 52) & 0x7ff) as i32;
    let (significand, exponent) = if biased_exponent == 0 {
        (fraction, -1074)
    } else {
        ((1u64 << 52) | fraction, biased_exponent - 1023 - 52)
    };
    binary_rational(significand, exponent)
}

fn binary_rational(significand: u64, exponent: i32) -> PositiveRational {
    if exponent >= 0 {
        PositiveRational {
            numerator: BigUint::from(significand) << exponent as usize,
            denominator: BigUint::from(1u8),
        }
    } else {
        PositiveRational {
            numerator: BigUint::from(significand),
            denominator: BigUint::from(1u8) << (-exponent) as usize,
        }
    }
}

fn decimal_power(exponent: u32) -> BigUint {
    BigUint::from(10u8).pow(exponent)
}

fn decimal_roundtrips_f32(decimal: Decimal, value: f32) -> bool {
    format!("{}e{}", decimal.significand, decimal.exponent)
        .parse::<f32>()
        .is_ok_and(|parsed| parsed == value)
}

fn decimal_roundtrips_f64(decimal: Decimal, value: f64) -> bool {
    format!("{}e{}", decimal.significand, decimal.exponent)
        .parse::<f64>()
        .is_ok_and(|parsed| parsed == value)
}

fn format_java_decimal(decimal: Decimal, negative: bool) -> String {
    let digits = decimal.significand.to_string();
    let digit_count = digits.len() as i32;
    let scientific_exponent = digit_count + decimal.exponent - 1;
    let mut output = String::new();
    if negative {
        output.push('-');
    }

    match scientific_exponent {
        -3..=-1 => {
            output.push_str("0.");
            output.extend(std::iter::repeat('0').take((-scientific_exponent - 1) as usize));
            output.push_str(&digits);
        }
        0..=6 if decimal.exponent >= 0 => {
            output.push_str(&digits);
            output.extend(std::iter::repeat('0').take(decimal.exponent as usize));
            output.push_str(".0");
        }
        0..=6 => {
            let split = (digit_count + decimal.exponent) as usize;
            output.push_str(&digits[..split]);
            output.push('.');
            output.push_str(&digits[split..]);
        }
        _ => {
            output.push_str(&digits[..1]);
            output.push('.');
            if digits.len() == 1 {
                output.push('0');
            } else {
                output.push_str(&digits[1..]);
            }
            output.push('E');
            output.push_str(&scientific_exponent.to_string());
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_java_special_values_subnormals_and_notation_boundaries() {
        assert_eq!(java_float_string(0.0), "0.0");
        assert_eq!(java_float_string(-0.0), "-0.0");
        assert_eq!(java_float_string(f32::INFINITY), "Infinity");
        assert_eq!(java_float_string(f32::NEG_INFINITY), "-Infinity");
        assert_eq!(java_float_string(f32::NAN), "NaN");
        assert_eq!(java_float_string(f32::from_bits(1)), "1.4E-45");
        assert_eq!(java_float_string(f32::MIN_POSITIVE), "1.1754944E-38");
        assert_eq!(java_float_string(0.001), "0.001");
        assert_eq!(java_float_string(10_000_000.0), "1.0E7");
        assert_eq!(java_float_string(9_999_999.0), "9999999.0");

        assert_eq!(java_double_string(f64::from_bits(1)), "4.9E-324");
        assert_eq!(java_double_string(f64::from_bits(2)), "9.9E-324");
        assert_eq!(java_double_string(1_234_000_000_000_000.0), "1.234E15");
        assert_eq!(java_double_string(20_000_000_000_000_000.0), "2.0E16");
    }
}
