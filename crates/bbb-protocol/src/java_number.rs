//! Gson numeric narrowing shared by registry source and resource JSON.

// StrictJsonParser reads a number through Gson's 1,024-character buffer.
pub const MAX_GSON_STRICT_JSON_NUMBER_BYTES: usize = 1_023;

// Gson's BigDecimal fallback bounds strings and scale, preventing an untrusted
// exponent from driving an unbounded decimal expansion.
const GSON_MAX_NUMBER_STRING_LENGTH: usize = 10_000;
const GSON_MAX_BIG_DECIMAL_ABS_SCALE: i64 = 10_000;

/// Narrows a numeric token supplied by JSON or NBT.
pub fn json_number_i32(value: &str) -> Option<i32> {
    if let Ok(value) = value.parse::<i32>() {
        return Some(value);
    }
    if let Ok(value) = value.parse::<i64>() {
        return Some(value as i32);
    }
    gson_big_decimal_low_bits(value).map(|value| value as u32 as i32)
}

/// Narrows a numeric token supplied by JSON or NBT.
pub fn json_number_i64(value: &str) -> Option<i64> {
    if let Ok(value) = value.parse::<i64>() {
        return Some(value);
    }
    gson_big_decimal_low_bits(value).map(|value| value as i64)
}

/// Applies Gson's bounded BigDecimal fallback and returns the low 64 bits of
/// the value after truncation toward zero.
fn gson_big_decimal_low_bits(value: &str) -> Option<u64> {
    if value.len() > GSON_MAX_NUMBER_STRING_LENGTH {
        return None;
    }
    let (negative, unsigned) = match value.strip_prefix('-') {
        Some(value) => (true, value),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
        Some(index) => (
            &unsigned[..index],
            unsigned[index + 1..].parse::<i32>().ok()?,
        ),
        None => (unsigned, 0),
    };
    let mut dot_index = None;
    let mut has_digit = false;
    for (idx, byte) in mantissa.bytes().enumerate() {
        if byte.is_ascii_digit() {
            has_digit = true;
        } else if byte == b'.' && dot_index.is_none() {
            dot_index = Some(idx);
        } else {
            return None;
        }
    }
    if !has_digit {
        return None;
    }
    let decimal_index = dot_index.unwrap_or(mantissa.len());
    let fractional_digits = mantissa.len().checked_sub(decimal_index)?.saturating_sub(1);
    let scale = i64::try_from(fractional_digits).ok()? - i64::from(exponent);
    if scale.abs() >= GSON_MAX_BIG_DECIMAL_ABS_SCALE {
        return None;
    }
    let integer_digits = i64::try_from(decimal_index).ok()? + i64::from(exponent);
    if integer_digits <= 0 {
        return Some(0);
    }

    let mut narrowed = 0_u64;
    let mut missing = usize::try_from(integer_digits).ok()?;
    for digit in mantissa
        .bytes()
        .filter(|byte| byte.is_ascii_digit())
        .take(missing)
    {
        narrowed = narrowed
            .wrapping_mul(10)
            .wrapping_add(u64::from(digit - b'0'));
        missing -= 1;
    }
    // Each decimal zero contributes a factor of two to the low 64 bits.
    if missing >= 64 {
        return Some(0);
    }
    narrowed = narrowed.wrapping_mul(10_u64.wrapping_pow(missing as u32));
    if negative {
        narrowed = narrowed.wrapping_neg();
    }
    Some(narrowed)
}

#[cfg(test)]
mod tests {
    use super::{json_number_i32, json_number_i64};

    #[test]
    fn gson_numeric_narrowing_keeps_integer_and_decimal_low_bits() {
        for (raw, i32_value, i64_value) in [
            ("-1", -1, -1),
            ("4294967297", 1, 4_294_967_297),
            ("2147483648", i32::MIN, 2_147_483_648),
            ("18446744073709551615", -1, -1),
            ("1.9", 1, 1),
            ("-1.9", -1, -1),
            ("1e2", 100, 100),
            ("1e400", 0, 0),
        ] {
            assert_eq!(json_number_i32(raw), Some(i32_value), "{raw}");
            assert_eq!(json_number_i64(raw), Some(i64_value), "{raw}");
        }
        assert_eq!(json_number_i32("1e100000"), None);
        assert_eq!(json_number_i64("1e100000"), None);
        for invalid in [
            "NaN",
            "Infinity",
            "-Infinity",
            "abc",
            "",
            "1a2",
            "1.2.3",
            ".",
        ] {
            assert_eq!(json_number_i32(invalid), None, "{invalid}");
            assert_eq!(json_number_i64(invalid), None, "{invalid}");
        }
    }
}
