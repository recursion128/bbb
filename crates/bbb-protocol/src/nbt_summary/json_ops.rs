//! Canonical DFU `JsonOps` -> `JavaOps` / `NbtOps` number narrowing.
//!
//! `JsonOps.convertTo` narrows exact integer lexemes to the smallest of
//! `byte`/`short`/`int`/`long` via `BigDecimal.longValueExact`, and otherwise
//! chooses `float` when `f32` preserves the `f64` value or `double` otherwise,
//! subject to Gson's `NumberLimits` (`<= 10_000` characters and `|scale| < 10_000`).

use super::{canonical_double_tag_bits, canonical_float_tag_bits, NbtSummaryValue};

/// Boxed `JavaOps` numeric value before `NbtOps` canonicalizes signed zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JsonOpsNumber {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
}

impl JsonOpsNumber {
    pub fn into_nbt(self) -> NbtSummaryValue {
        match self {
            Self::Byte(value) => NbtSummaryValue::Byte(value),
            Self::Short(value) => NbtSummaryValue::Short(value),
            Self::Int(value) => NbtSummaryValue::Int(value),
            Self::Long(value) => NbtSummaryValue::Long(value),
            Self::Float(value) => NbtSummaryValue::Float(canonical_float_tag_bits(value)),
            Self::Double(value) => NbtSummaryValue::Double(canonical_double_tag_bits(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonOpsNumberError {
    Invalid(&'static str),
    GsonLimit(&'static str),
}

impl From<&'static str> for JsonOpsNumberError {
    fn from(reason: &'static str) -> Self {
        Self::Invalid(reason)
    }
}

pub fn json_number_to_nbtops(value: &str) -> Result<NbtSummaryValue, JsonOpsNumberError> {
    json_number_to_javaops(value).map(JsonOpsNumber::into_nbt)
}

pub fn json_number_to_javaops(value: &str) -> Result<JsonOpsNumber, JsonOpsNumberError> {
    if !is_finite_json_number_lexeme(value) {
        return Err(JsonOpsNumberError::Invalid(
            "invalid finite JSON number lexeme",
        ));
    }
    // Gson NumberLimits rejects before JsonOps' arithmetic narrowing fallback.
    if value.len() > 10_000 {
        return Err(JsonOpsNumberError::GsonLimit(
            "Gson BigDecimal number length exceeds 10000",
        ));
    }
    let (mantissa, exponent) = match value.find(['e', 'E']) {
        Some(index) => (
            &value[..index],
            value[index + 1..].parse::<i64>().map_err(|_| {
                JsonOpsNumberError::GsonLimit(
                    "Gson BigDecimal exponent is outside its representation",
                )
            })?,
        ),
        None => (value, 0),
    };
    let fractional = mantissa
        .find('.')
        .map_or(0, |index| mantissa.len() - index - 1);
    let scale = (fractional as i64)
        .checked_sub(exponent)
        .ok_or(JsonOpsNumberError::GsonLimit(
            "Gson BigDecimal scale is outside its representation",
        ))?;
    if scale.unsigned_abs() >= 10_000 {
        return Err(JsonOpsNumberError::GsonLimit(
            "Gson BigDecimal absolute scale must be below 10000",
        ));
    }
    if let Some(value) = exact_decimal_i64(value)? {
        return Ok(if let Ok(value) = i8::try_from(value) {
            JsonOpsNumber::Byte(value)
        } else if let Ok(value) = i16::try_from(value) {
            JsonOpsNumber::Short(value)
        } else if let Ok(value) = i32::try_from(value) {
            JsonOpsNumber::Int(value)
        } else {
            JsonOpsNumber::Long(value)
        });
    }
    let value = value
        .parse::<f64>()
        .map_err(|_| "JSON number has no BigDecimal double representation")?;
    let narrowed = value as f32;
    Ok(if f64::from(narrowed) == value {
        JsonOpsNumber::Float(narrowed)
    } else {
        JsonOpsNumber::Double(value)
    })
}

/// Strict finite JSON number spelling (RFC 8259 §6), with no surrounding whitespace.
fn is_finite_json_number_lexeme(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    if bytes.get(index) == Some(&b'-') {
        index += 1;
    }
    match bytes.get(index) {
        Some(b'0') => index += 1,
        Some(b'1'..=b'9') => {
            index += 1;
            while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                index += 1;
            }
        }
        _ => return false,
    }
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let frac_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == frac_start {
            return false;
        }
    }
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let exp_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == exp_start {
            return false;
        }
    }
    index == bytes.len()
}

/// Returns the value accepted by `BigDecimal.longValueExact`, or `None` when
/// the decimal is fractional or outside signed-long range.
fn exact_decimal_i64(value: &str) -> Result<Option<i64>, &'static str> {
    let (negative, unsigned) = value
        .strip_prefix('-')
        .map_or((false, value), |value| (true, value));
    let unsigned = unsigned.strip_prefix('+').unwrap_or(unsigned);
    let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
        Some(index) => {
            let exponent = unsigned[index + 1..]
                .parse::<i64>()
                .map_err(|_| "JSON number exponent is outside BigDecimal range")?;
            (&unsigned[..index], exponent)
        }
        None => (unsigned, 0),
    };
    let decimal = mantissa.find('.').unwrap_or(mantissa.len());
    let fractional = mantissa.len().saturating_sub(decimal).saturating_sub(1);
    let digits = mantissa
        .bytes()
        .filter(|value| value.is_ascii_digit())
        .collect::<Vec<_>>();
    let scale = i64::try_from(fractional)
        .map_err(|_| "JSON number scale is outside BigDecimal range")?
        .checked_sub(exponent)
        .ok_or("JSON number scale is outside BigDecimal range")?;
    i32::try_from(scale).map_err(|_| "JSON number scale is outside BigDecimal range")?;
    if digits.iter().all(|value| *value == b'0') {
        return Ok(Some(0));
    }
    let integer = if scale > 0 {
        let scale =
            usize::try_from(scale).map_err(|_| "JSON number scale is outside BigDecimal range")?;
        if scale > digits.len() {
            return Ok(None);
        }
        let split = digits.len() - scale;
        if digits[split..].iter().any(|value| *value != b'0') {
            return Ok(None);
        }
        digits[..split].to_vec()
    } else {
        let zeros = usize::try_from(scale.unsigned_abs())
            .map_err(|_| "JSON number scale is outside BigDecimal range")?;
        let first_significant = digits
            .iter()
            .position(|digit| *digit != b'0')
            .unwrap_or(digits.len());
        let mut integer = digits[first_significant..].to_vec();
        if integer.len().saturating_add(zeros) > 20 {
            return Ok(None);
        }
        integer.resize(integer.len() + zeros, b'0');
        integer
    };
    let leading_zeros = integer
        .iter()
        .position(|&byte| byte != b'0')
        .unwrap_or(integer.len());
    let trimmed = &integer[leading_zeros..];
    if trimmed.is_empty() {
        return Ok(Some(0));
    }
    let digits_str = std::str::from_utf8(trimmed).map_err(|_| "invalid JSON integer")?;
    if !negative {
        return Ok(digits_str.parse::<i64>().ok());
    }
    let mut signed = String::with_capacity(1 + digits_str.len());
    signed.push('-');
    signed.push_str(digits_str);
    Ok(signed.parse::<i64>().ok())
}
