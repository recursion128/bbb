use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};

use super::normalize_codec_identifier;

const MAX_INT_PROVIDER_DEPTH: usize = 64;

/// Exact retained value shape of vanilla's built-in `IntProvider` codecs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldIntProvider {
    Constant(i32),
    Uniform {
        min_inclusive: i32,
        max_inclusive: i32,
    },
    BiasedToBottom {
        min_inclusive: i32,
        max_inclusive: i32,
    },
    Clamped {
        source: Box<WorldIntProvider>,
        min_inclusive: i32,
        max_inclusive: i32,
    },
    WeightedList(Vec<WorldWeightedIntProvider>),
    ClampedNormal {
        mean_bits: u32,
        deviation_bits: u32,
        min_inclusive: i32,
        max_inclusive: i32,
    },
    Trapezoid {
        min_inclusive: i32,
        max_inclusive: i32,
        plateau: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldWeightedIntProvider {
    pub provider: WorldIntProvider,
    pub weight: i32,
}

impl WorldIntProvider {
    pub fn min_inclusive(&self) -> i32 {
        match self {
            Self::Constant(value) => *value,
            Self::Uniform { min_inclusive, .. }
            | Self::BiasedToBottom { min_inclusive, .. }
            | Self::ClampedNormal { min_inclusive, .. }
            | Self::Trapezoid { min_inclusive, .. } => *min_inclusive,
            Self::Clamped {
                source,
                min_inclusive,
                ..
            } => (*min_inclusive).max(source.min_inclusive()),
            Self::WeightedList(entries) => entries
                .iter()
                .map(|entry| entry.provider.min_inclusive())
                .min()
                .expect("weighted provider decoder rejects empty distributions"),
        }
    }

    pub fn max_inclusive(&self) -> i32 {
        match self {
            Self::Constant(value) => *value,
            Self::Uniform { max_inclusive, .. }
            | Self::BiasedToBottom { max_inclusive, .. }
            | Self::ClampedNormal { max_inclusive, .. }
            | Self::Trapezoid { max_inclusive, .. } => *max_inclusive,
            Self::Clamped {
                source,
                max_inclusive,
                ..
            } => (*max_inclusive).min(source.max_inclusive()),
            Self::WeightedList(entries) => entries
                .iter()
                .map(|entry| entry.provider.max_inclusive())
                .max()
                .expect("weighted provider decoder rejects empty distributions"),
        }
    }
}

pub(super) fn decode_bounded_int_provider(
    value: &NbtSummaryValue,
    min_value: i32,
    max_value: i32,
) -> Result<WorldIntProvider, String> {
    let provider = decode_int_provider(value, 0)?;
    let actual_min = provider.min_inclusive();
    let actual_max = provider.max_inclusive();
    if actual_min < min_value {
        return Err(format!(
            "value provider too low: {min_value} [{actual_min}-{actual_max}]"
        ));
    }
    if actual_max > max_value {
        return Err(format!(
            "value provider too high: {max_value} [{actual_min}-{actual_max}]"
        ));
    }
    Ok(provider)
}

fn decode_int_provider(value: &NbtSummaryValue, depth: usize) -> Result<WorldIntProvider, String> {
    if let Some(value) = numeric_i32(value) {
        return Ok(WorldIntProvider::Constant(value));
    }
    if depth >= MAX_INT_PROVIDER_DEPTH {
        return Err(format!(
            "int provider nesting exceeds {MAX_INT_PROVIDER_DEPTH}"
        ));
    }
    let NbtSummaryValue::Compound(fields) = value else {
        return Err("expected integer or provider compound".to_string());
    };
    let provider_type = required_string(fields, "type")?;
    let provider_type = normalize_codec_identifier(provider_type)
        .ok_or_else(|| format!("type: invalid resource identifier {provider_type}"))?;
    match provider_type.as_str() {
        "minecraft:constant" => Ok(WorldIntProvider::Constant(required_i32(fields, "value")?)),
        "minecraft:uniform" => {
            let (min_inclusive, max_inclusive) = required_range(fields)?;
            Ok(WorldIntProvider::Uniform {
                min_inclusive,
                max_inclusive,
            })
        }
        "minecraft:biased_to_bottom" => {
            let (min_inclusive, max_inclusive) = required_range(fields)?;
            Ok(WorldIntProvider::BiasedToBottom {
                min_inclusive,
                max_inclusive,
            })
        }
        "minecraft:clamped" => {
            let source =
                field(fields, "source").ok_or_else(|| "source: missing field".to_string())?;
            let source = decode_int_provider(source, depth + 1)
                .map_err(|error| format!("source: {error}"))?;
            let (min_inclusive, max_inclusive) = required_range(fields)?;
            Ok(WorldIntProvider::Clamped {
                source: Box::new(source),
                min_inclusive,
                max_inclusive,
            })
        }
        "minecraft:weighted_list" => decode_weighted_list(fields, depth + 1),
        "minecraft:clamped_normal" => {
            let mean_bits = required_f32(fields, "mean")?.to_bits();
            let deviation_bits = required_f32(fields, "deviation")?.to_bits();
            let (min_inclusive, max_inclusive) = required_range(fields)?;
            Ok(WorldIntProvider::ClampedNormal {
                mean_bits,
                deviation_bits,
                min_inclusive,
                max_inclusive,
            })
        }
        "minecraft:trapezoid" => {
            let min_inclusive = required_i32(fields, "min")?;
            let max_inclusive = required_i32(fields, "max")?;
            if max_inclusive < min_inclusive {
                return Err(format!(
                    "max must be at least min [{min_inclusive}, {max_inclusive}]"
                ));
            }
            let plateau = required_i32(fields, "plateau")?;
            if plateau > max_inclusive.wrapping_sub(min_inclusive) {
                return Err(format!(
                    "plateau exceeds span [{min_inclusive}, {max_inclusive}]"
                ));
            }
            Ok(WorldIntProvider::Trapezoid {
                min_inclusive,
                max_inclusive,
                plateau,
            })
        }
        _ => Err(format!("unknown int provider type {provider_type}")),
    }
}

fn decode_weighted_list(
    fields: &[NbtSummaryEntry],
    depth: usize,
) -> Result<WorldIntProvider, String> {
    let distribution =
        field(fields, "distribution").ok_or_else(|| "distribution: missing field".to_string())?;
    let NbtSummaryValue::List(values) = distribution else {
        return Err("distribution: expected list".to_string());
    };
    if values.is_empty() {
        return Err("distribution: weighted list must contain at least one entry".to_string());
    }
    let mut total_weight = 0_i64;
    let mut entries = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let NbtSummaryValue::Compound(fields) = value else {
            return Err(format!("distribution[{index}]: expected compound"));
        };
        let provider = field(fields, "data")
            .ok_or_else(|| format!("distribution[{index}].data: missing field"))?;
        let provider = decode_int_provider(provider, depth)
            .map_err(|error| format!("distribution[{index}].data: {error}"))?;
        let weight = required_i32(fields, "weight")
            .map_err(|error| format!("distribution[{index}].{error}"))?;
        if weight < 0 {
            return Err(format!(
                "distribution[{index}].weight: expected non-negative integer"
            ));
        }
        total_weight += i64::from(weight);
        if total_weight > i64::from(i32::MAX) {
            return Err("distribution: sum of weights exceeds 2147483647".to_string());
        }
        entries.push(WorldWeightedIntProvider { provider, weight });
    }
    Ok(WorldIntProvider::WeightedList(entries))
}

fn required_range(fields: &[NbtSummaryEntry]) -> Result<(i32, i32), String> {
    let min_inclusive = required_i32(fields, "min_inclusive")?;
    let max_inclusive = required_i32(fields, "max_inclusive")?;
    if max_inclusive < min_inclusive {
        return Err(format!(
            "max must be at least min [{min_inclusive}, {max_inclusive}]"
        ));
    }
    Ok((min_inclusive, max_inclusive))
}

fn field<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    fields
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn required_string<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> Result<&'a str, String> {
    match field(fields, name) {
        Some(NbtSummaryValue::String(value)) => Ok(value),
        Some(_) => Err(format!("{name}: expected string")),
        None => Err(format!("{name}: missing field")),
    }
}

fn required_i32(fields: &[NbtSummaryEntry], name: &str) -> Result<i32, String> {
    field(fields, name)
        .ok_or_else(|| format!("{name}: missing field"))
        .and_then(|value| numeric_i32(value).ok_or_else(|| format!("{name}: expected integer")))
}

fn required_f32(fields: &[NbtSummaryEntry], name: &str) -> Result<f32, String> {
    field(fields, name)
        .ok_or_else(|| format!("{name}: missing field"))
        .and_then(|value| numeric_f32(value).ok_or_else(|| format!("{name}: expected number")))
}

fn numeric_i32(value: &NbtSummaryValue) -> Option<i32> {
    match value {
        NbtSummaryValue::Byte(value) => Some(i32::from(*value)),
        NbtSummaryValue::Short(value) => Some(i32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value),
        NbtSummaryValue::Long(value) => i32::try_from(*value).ok(),
        _ => None,
    }
}

fn numeric_f32(value: &NbtSummaryValue) -> Option<f32> {
    Some(match value {
        NbtSummaryValue::Byte(value) => f32::from(*value),
        NbtSummaryValue::Short(value) => f32::from(*value),
        NbtSummaryValue::Int(value) => *value as f32,
        NbtSummaryValue::Long(value) => *value as f32,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits),
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as f32,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(json: &str) -> Result<WorldIntProvider, String> {
        let value = super::super::element_value::decode_known_pack_element(json.as_bytes())
            .unwrap()
            .value;
        decode_bounded_int_provider(&value, 0, 15)
    }

    #[test]
    fn decodes_every_builtin_int_provider_shape_and_derived_bounds() {
        let cases = [
            ("7", 7, 7),
            (r#"{"type":"constant","value":3}"#, 3, 3),
            (
                r#"{"type":"uniform","min_inclusive":1,"max_inclusive":5}"#,
                1,
                5,
            ),
            (
                r#"{"type":"biased_to_bottom","min_inclusive":2,"max_inclusive":6}"#,
                2,
                6,
            ),
            (
                r#"{"type":"clamped","source":{"type":"uniform","min_inclusive":1,"max_inclusive":12},"min_inclusive":3,"max_inclusive":8}"#,
                3,
                8,
            ),
            (
                r#"{"type":"weighted_list","distribution":[{"data":2,"weight":0},{"data":{"type":"uniform","min_inclusive":4,"max_inclusive":9},"weight":3}]}"#,
                2,
                9,
            ),
            (
                r#"{"type":"clamped_normal","mean":5.5,"deviation":1.25,"min_inclusive":2,"max_inclusive":10}"#,
                2,
                10,
            ),
            (
                r#"{"type":"trapezoid","min":3,"max":11,"plateau":4}"#,
                3,
                11,
            ),
        ];

        for (json, min, max) in cases {
            let provider = decode(json).unwrap();
            assert_eq!(provider.min_inclusive(), min, "{json}");
            assert_eq!(provider.max_inclusive(), max, "{json}");
        }
    }

    #[test]
    fn matches_provider_specific_and_outer_range_failures() {
        for (json, expected) in [
            (
                r#"{"type":"uniform","min_inclusive":5,"max_inclusive":4}"#,
                "max must be at least min",
            ),
            (
                r#"{"type":"weighted_list","distribution":[]}"#,
                "at least one entry",
            ),
            (
                r#"{"type":"weighted_list","distribution":[{"data":1,"weight":2147483647},{"data":2,"weight":1}]}"#,
                "sum of weights",
            ),
            (
                r#"{"type":"trapezoid","min":0,"max":5,"plateau":6}"#,
                "plateau exceeds span",
            ),
            (
                r#"{"type":"uniform","min_inclusive":-1,"max_inclusive":4}"#,
                "too low",
            ),
            (
                r#"{"type":"uniform","min_inclusive":4,"max_inclusive":16}"#,
                "too high",
            ),
            (r#"{"type":"example:unknown"}"#, "unknown int provider"),
        ] {
            let error = decode(json).unwrap_err();
            assert!(error.contains(expected), "{error}");
        }

        let all_zero =
            decode(r#"{"type":"weighted_list","distribution":[{"data":1,"weight":0}]}"#).unwrap();
        assert_eq!(all_zero.min_inclusive(), 1);
        assert_eq!(all_zero.max_inclusive(), 1);
    }
}
