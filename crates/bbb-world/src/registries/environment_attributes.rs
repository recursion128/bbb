use std::fmt;

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientEnvironmentAttributes {
    pub fog_color: Option<[u8; 3]>,
    pub fog_start_distance: Option<f32>,
    pub fog_end_distance: Option<f32>,
    pub sky_fog_end_distance: Option<f32>,
    pub cloud_fog_end_distance: Option<f32>,
    pub water_fog_color: Option<[u8; 3]>,
    pub water_fog_start_distance: Option<f32>,
    pub water_fog_end_distance: Option<f32>,
    pub sky_color: Option<[u8; 3]>,
    pub block_light_tint: Option<[u8; 3]>,
    pub sky_light_color: Option<[u8; 3]>,
    pub sky_light_factor: Option<f32>,
    pub night_vision_color: Option<[u8; 3]>,
    pub ambient_light_color: Option<[u8; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EnvironmentAttributeProjectionError {
    pub(crate) field: &'static str,
    pub(crate) reason: &'static str,
}

impl fmt::Display for EnvironmentAttributeProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.reason)
    }
}

pub(crate) fn decode_client_environment_attributes(
    entries: &[NbtSummaryEntry],
) -> Result<ClientEnvironmentAttributes, EnvironmentAttributeProjectionError> {
    Ok(ClientEnvironmentAttributes {
        fog_color: optional_rgb(entries, "minecraft:visual/fog_color")?,
        fog_start_distance: optional_float(
            entries,
            "minecraft:visual/fog_start_distance",
            FloatRange::Any,
        )?,
        fog_end_distance: optional_float(
            entries,
            "minecraft:visual/fog_end_distance",
            FloatRange::NonNegative,
        )?,
        sky_fog_end_distance: optional_float(
            entries,
            "minecraft:visual/sky_fog_end_distance",
            FloatRange::NonNegative,
        )?,
        cloud_fog_end_distance: optional_float(
            entries,
            "minecraft:visual/cloud_fog_end_distance",
            FloatRange::NonNegative,
        )?,
        water_fog_color: optional_rgb(entries, "minecraft:visual/water_fog_color")?,
        water_fog_start_distance: optional_float(
            entries,
            "minecraft:visual/water_fog_start_distance",
            FloatRange::Any,
        )?,
        water_fog_end_distance: optional_float(
            entries,
            "minecraft:visual/water_fog_end_distance",
            FloatRange::NonNegative,
        )?,
        sky_color: optional_rgb(entries, "minecraft:visual/sky_color")?,
        block_light_tint: optional_rgb(entries, "minecraft:visual/block_light_tint")?,
        sky_light_color: optional_rgb(entries, "minecraft:visual/sky_light_color")?,
        sky_light_factor: optional_float(
            entries,
            "minecraft:visual/sky_light_factor",
            FloatRange::Unit,
        )?,
        night_vision_color: optional_rgb(entries, "minecraft:visual/night_vision_color")?,
        ambient_light_color: optional_rgb(entries, "minecraft:visual/ambient_light_color")?,
    })
}

pub(crate) fn parse_rgb(value: &NbtSummaryValue) -> Option<[u8; 3]> {
    let value = match value {
        NbtSummaryValue::String(value) => {
            let hex = value.strip_prefix('#')?;
            if hex.len() != 6 {
                return None;
            }
            u32::from_str_radix(hex, 16).ok()?
        }
        NbtSummaryValue::Byte(value) => i64::from(*value) as u32 & 0x00ff_ffff,
        NbtSummaryValue::Short(value) => i64::from(*value) as u32 & 0x00ff_ffff,
        NbtSummaryValue::Int(value) => i64::from(*value) as u32 & 0x00ff_ffff,
        NbtSummaryValue::Long(value) => *value as u32 & 0x00ff_ffff,
        _ => return None,
    };
    Some([
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ])
}

fn optional_rgb(
    entries: &[NbtSummaryEntry],
    field: &'static str,
) -> Result<Option<[u8; 3]>, EnvironmentAttributeProjectionError> {
    match field_value(entries, field) {
        Some(NbtSummaryValue::Compound(_)) => {
            Err(error(field, "attribute modifiers are not yet supported"))
        }
        Some(value) => parse_rgb(value)
            .map(Some)
            .ok_or_else(|| error(field, "expected #RRGGBB string or RGB int")),
        None => Ok(None),
    }
}

#[derive(Debug, Clone, Copy)]
enum FloatRange {
    Any,
    NonNegative,
    Unit,
}

fn optional_float(
    entries: &[NbtSummaryEntry],
    field: &'static str,
    range: FloatRange,
) -> Result<Option<f32>, EnvironmentAttributeProjectionError> {
    match field_value(entries, field) {
        Some(NbtSummaryValue::Compound(_)) => {
            Err(error(field, "attribute modifiers are not yet supported"))
        }
        Some(value) => match numeric_f32(value) {
            Some(value) if float_in_range(value, range) => Ok(Some(value)),
            Some(_) => Err(error(field, float_range_reason(range))),
            None => Err(error(field, "expected number")),
        },
        None => Ok(None),
    }
}

fn field_value<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
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

fn float_in_range(value: f32, range: FloatRange) -> bool {
    value.is_finite()
        && match range {
            FloatRange::Any => true,
            FloatRange::NonNegative => value >= 0.0,
            FloatRange::Unit => (0.0..=1.0).contains(&value),
        }
}

fn float_range_reason(range: FloatRange) -> &'static str {
    match range {
        FloatRange::Any => "float must be finite",
        FloatRange::NonNegative => "float must be finite and non-negative",
        FloatRange::Unit => "float must be finite and within 0..=1",
    }
}

fn error(field: &'static str, reason: &'static str) -> EnvironmentAttributeProjectionError {
    EnvironmentAttributeProjectionError { field, reason }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_direct_visual_environment_attributes() {
        let entries = vec![
            entry(
                "minecraft:visual/fog_color",
                NbtSummaryValue::String("#123456".to_string()),
            ),
            entry(
                "minecraft:visual/fog_start_distance",
                NbtSummaryValue::Double((-4.0_f64).to_bits()),
            ),
            entry(
                "minecraft:visual/fog_end_distance",
                NbtSummaryValue::Double(192.0_f64.to_bits()),
            ),
            entry(
                "minecraft:visual/sky_light_factor",
                NbtSummaryValue::Double(0.25_f64.to_bits()),
            ),
            entry(
                "minecraft:visual/ambient_light_color",
                NbtSummaryValue::Long(0x203040),
            ),
        ];

        let projection = decode_client_environment_attributes(&entries).unwrap();
        assert_eq!(projection.fog_color, Some([0x12, 0x34, 0x56]));
        assert_eq!(projection.fog_start_distance, Some(-4.0));
        assert_eq!(projection.fog_end_distance, Some(192.0));
        assert_eq!(projection.sky_light_factor, Some(0.25));
        assert_eq!(projection.ambient_light_color, Some([0x20, 0x30, 0x40]));
    }

    #[test]
    fn rejects_relevant_modifier_compounds_and_invalid_ranges() {
        let modifier = vec![entry(
            "minecraft:visual/fog_color",
            NbtSummaryValue::Compound(Vec::new()),
        )];
        assert_eq!(
            decode_client_environment_attributes(&modifier).unwrap_err(),
            EnvironmentAttributeProjectionError {
                field: "minecraft:visual/fog_color",
                reason: "attribute modifiers are not yet supported",
            }
        );

        let invalid = vec![entry(
            "minecraft:visual/sky_light_factor",
            NbtSummaryValue::Double(1.5_f64.to_bits()),
        )];
        assert_eq!(
            decode_client_environment_attributes(&invalid).unwrap_err(),
            EnvironmentAttributeProjectionError {
                field: "minecraft:visual/sky_light_factor",
                reason: "float must be finite and within 0..=1",
            }
        );
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.to_string(),
            value,
        }
    }
}
