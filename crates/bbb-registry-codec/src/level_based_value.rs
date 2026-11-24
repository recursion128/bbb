//! `LevelBasedValue`, the enchantment-level
//! number every enchantment effect component spells its amounts in.

use bbb_protocol::{nbt_summary::compound_field as nbt_field, NbtSummaryValue};

use crate::{RegistryElementValueRef, RegistryJsonValue};

/// `LevelBasedValue.CODEC`: a bare float is a
/// `Constant`, anything else dispatches on `type`.
#[derive(Debug, Clone, PartialEq)]
pub enum LevelBasedValue {
    Constant(f32),
    Clamped {
        value: Box<LevelBasedValue>,
        min: f32,
        max: f32,
    },
    Exponent {
        base: Box<LevelBasedValue>,
        power: Box<LevelBasedValue>,
    },
    Fraction {
        numerator: Box<LevelBasedValue>,
        denominator: Box<LevelBasedValue>,
    },
    LevelsSquared {
        added: f32,
    },
    Linear {
        base: f32,
        per_level_above_first: f32,
    },
    Lookup {
        values: Vec<f32>,
        fallback: Box<LevelBasedValue>,
    },
}

impl LevelBasedValue {
    pub fn parse(value: &RegistryJsonValue) -> Result<Self, String> {
        Self::parse_source(RegistryElementValueRef::Json(value))
    }

    pub fn parse_source(value: RegistryElementValueRef<'_>) -> Result<Self, String> {
        if let Some(constant) = value.codec_f32() {
            return Ok(Self::Constant(constant));
        }
        let value_type = value
            .field("type")
            .and_then(RegistryElementValueRef::utf8_string)
            .map(bbb_protocol::normalize_resource_location)
            .transpose()
            .map_err(|error| error.to_string())?;
        let float = |name: &str| -> Result<f32, String> {
            value
                .field(name)
                .and_then(RegistryElementValueRef::codec_f32)
                .ok_or_else(|| format!("a level-based value has no float {name}"))
        };
        let nested = |name: &str| -> Result<Box<Self>, String> {
            value
                .field(name)
                .ok_or_else(|| format!("a level-based value has no {name}"))
                .and_then(Self::parse_source)
                .map(Box::new)
        };
        match value_type.as_deref() {
            Some("minecraft:clamped") => {
                let (min, max) = (float("min")?, float("max")?);
                // `Clamped.CODEC.validate`.
                if max <= min {
                    return Err(format!("clamped max {max} is not above min {min}"));
                }
                Ok(Self::Clamped {
                    value: nested("value")?,
                    min,
                    max,
                })
            }
            Some("minecraft:exponent") => Ok(Self::Exponent {
                base: nested("base")?,
                power: nested("power")?,
            }),
            Some("minecraft:fraction") => Ok(Self::Fraction {
                numerator: nested("numerator")?,
                denominator: nested("denominator")?,
            }),
            Some("minecraft:levels_squared") => Ok(Self::LevelsSquared {
                added: float("added")?,
            }),
            Some("minecraft:linear") => Ok(Self::Linear {
                base: float("base")?,
                per_level_above_first: float("per_level_above_first")?,
            }),
            Some("minecraft:lookup") => Ok(Self::Lookup {
                values: {
                    let values = value
                        .field("values")
                        .ok_or("a lookup level-based value has no values")?;
                    (0..values
                        .list_len()
                        .ok_or("a lookup level-based value has no values")?)
                        .map(|index| {
                            values
                                .list_element(index)
                                .and_then(|value| value.codec_f32())
                                .ok_or_else(|| "a lookup value is not a float".to_string())
                        })
                        .collect::<Result<_, _>>()?
                },
                fallback: nested("fallback")?,
            }),
            other => Err(format!(
                "level-based value type {other:?} is not registered"
            )),
        }
    }

    /// Projects a value produced by the exact level-based source codec.
    pub fn from_canonical(value: &NbtSummaryValue) -> Self {
        if let NbtSummaryValue::Float(bits) = value {
            return Self::Constant(f32::from_bits(*bits));
        }
        let NbtSummaryValue::Compound(fields) = value else {
            unreachable!("canonical level-based value");
        };
        let field = |name| nbt_field(fields, name).expect("canonical level-based value field");
        let NbtSummaryValue::String(value_type) = field("type") else {
            unreachable!("canonical level-based value type");
        };
        value_type
            .with_utf8(|value_type| match value_type {
                "minecraft:clamped" => Self::Clamped {
                    value: Box::new(Self::from_canonical(field("value"))),
                    min: nbt_f32(field("min")),
                    max: nbt_f32(field("max")),
                },
                "minecraft:exponent" => Self::Exponent {
                    base: Box::new(Self::from_canonical(field("base"))),
                    power: Box::new(Self::from_canonical(field("power"))),
                },
                "minecraft:fraction" => Self::Fraction {
                    numerator: Box::new(Self::from_canonical(field("numerator"))),
                    denominator: Box::new(Self::from_canonical(field("denominator"))),
                },
                "minecraft:levels_squared" => Self::LevelsSquared {
                    added: nbt_f32(field("added")),
                },
                "minecraft:linear" => Self::Linear {
                    base: nbt_f32(field("base")),
                    per_level_above_first: nbt_f32(field("per_level_above_first")),
                },
                "minecraft:lookup" => Self::Lookup {
                    values: {
                        let NbtSummaryValue::List(values) = field("values") else {
                            unreachable!("canonical enchantment lookup values");
                        };
                        values.iter().map(nbt_f32).collect()
                    },
                    fallback: Box::new(Self::from_canonical(field("fallback"))),
                },
                _ => unreachable!("canonical level-based value type"),
            })
            .expect("canonical level-based value identifier")
    }

    #[must_use]
    #[inline]
    pub fn calculate(&self, level: i32) -> f32 {
        match self {
            Self::Constant(value) => *value,
            Self::Clamped { value, min, max } => {
                let value = value.calculate(level);
                if value < *min {
                    *min
                } else {
                    java_float_min(value, *max)
                }
            }
            Self::Exponent { base, power } => {
                f64::from(base.calculate(level)).powf(f64::from(power.calculate(level))) as f32
            }
            // A zero denominator answers zero.
            Self::Fraction {
                numerator,
                denominator,
            } => {
                let denominator = denominator.calculate(level);
                if denominator == 0.0 {
                    0.0
                } else {
                    numerator.calculate(level) / denominator
                }
            }
            Self::LevelsSquared { added } => level.wrapping_mul(level) as f32 + added,
            Self::Linear {
                base,
                per_level_above_first,
            } => base + per_level_above_first * (level - 1) as f32,
            Self::Lookup { values, fallback } => usize::try_from(level - 1)
                .ok()
                .and_then(|index| values.get(index))
                .copied()
                .unwrap_or_else(|| fallback.calculate(level)),
        }
    }
}

fn nbt_f32(value: &NbtSummaryValue) -> f32 {
    let NbtSummaryValue::Float(bits) = value else {
        unreachable!("canonical enchantment float");
    };
    f32::from_bits(*bits)
}

#[inline]
pub fn java_float_min(left: f32, right: f32) -> f32 {
    if left.is_nan() {
        left
    } else if right.is_nan() {
        right
    } else {
        left.min(right)
    }
}
