//! Exact source-aware owners for Vanilla integer and float value providers.
//!
//! Both families accept an inline scalar constant or a registry-dispatched
//! record. Their dispatch membership comes from the generated Vanilla 26.1
//! catalog; this module owns only each registered family's payload semantics.

use crate::{
    codec_dispatch_types::{
        float_provider_codec_type, int_provider_codec_type, FloatProviderCodecFamily,
        IntProviderCodecFamily,
    },
    registry_codec::{
        recursive_values::{
            required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
        },
        RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
    },
};
use bbb_protocol::{
    nbt_summary::{canonical_float_tag_bits, entry},
    NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// `IntProviders.CODEC`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntProvider {
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
        source: Box<IntProvider>,
        min_inclusive: i32,
        max_inclusive: i32,
    },
    /// Entries keep their written order with their weights.
    WeightedList(Vec<WeightedIntProvider>),
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
pub struct WeightedIntProvider {
    pub provider: IntProvider,
    pub weight: i32,
}

impl IntProvider {
    pub fn decode(value: &crate::RegistryJsonValue) -> Result<Self, RegistryCodecError> {
        Self::decode_in(value, i32::MIN, i32::MAX)
    }

    /// `IntProviders.codec(min, max)`.
    pub fn decode_in(
        value: &crate::RegistryJsonValue,
        minimum: i32,
        maximum: i32,
    ) -> Result<Self, RegistryCodecError> {
        let mut context = RegistryCodecDecodeContext::new(
            "minecraft:int_provider",
            "IntProviders.CODEC",
            crate::RegistryElementSource::ServerDatapackJson,
        );
        decode_int_provider_in(
            RegistryElementValueRef::Json(value),
            "",
            &mut context,
            minimum,
            maximum,
        )
    }

    /// `IntProvider.minInclusive`.
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
            Self::WeightedList(distribution) => distribution
                .iter()
                .map(|entry| entry.provider.min_inclusive())
                .min()
                .unwrap_or(i32::MAX),
        }
    }

    /// `IntProvider.maxInclusive`.
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
            Self::WeightedList(distribution) => distribution
                .iter()
                .map(|entry| entry.provider.max_inclusive())
                .max()
                .unwrap_or(i32::MIN),
        }
    }
}

impl IntProvider {
    pub fn into_nbt(self) -> NbtSummaryValue {
        encode_int_provider(Cow::Owned(self))
    }

    pub(crate) fn network_nbt(&self) -> NbtSummaryValue {
        encode_int_provider(Cow::Borrowed(self))
    }
}

fn encode_int_provider(input: Cow<'_, IntProvider>) -> NbtSummaryValue {
    let (name, mut fields) = match input.as_ref() {
        IntProvider::Constant(value) => return NbtSummaryValue::Int(*value),
        IntProvider::Uniform {
            min_inclusive,
            max_inclusive,
        } => (
            "minecraft:uniform",
            int_range_entries(*min_inclusive, *max_inclusive),
        ),
        IntProvider::BiasedToBottom {
            min_inclusive,
            max_inclusive,
        } => (
            "minecraft:biased_to_bottom",
            int_range_entries(*min_inclusive, *max_inclusive),
        ),
        IntProvider::Clamped {
            min_inclusive,
            max_inclusive,
            ..
        } => {
            let (min, max) = (*min_inclusive, *max_inclusive);
            let source = match input {
                Cow::Owned(IntProvider::Clamped { source, .. }) => Cow::Owned(*source),
                Cow::Borrowed(IntProvider::Clamped { source, .. }) => {
                    Cow::Borrowed(source.as_ref())
                }
                _ => unreachable!("selected clamped provider"),
            };
            (
                "minecraft:clamped",
                vec![
                    entry("source", encode_int_provider(source)),
                    entry("min_inclusive", NbtSummaryValue::Int(min)),
                    entry("max_inclusive", NbtSummaryValue::Int(max)),
                ],
            )
        }
        IntProvider::WeightedList(_) => {
            let distribution = match input {
                Cow::Owned(IntProvider::WeightedList(entries)) => entries
                    .into_iter()
                    .map(|entry| weighted_int_entry_nbt(Cow::Owned(entry.provider), entry.weight))
                    .collect(),
                Cow::Borrowed(IntProvider::WeightedList(entries)) => entries
                    .iter()
                    .map(|entry| {
                        weighted_int_entry_nbt(Cow::Borrowed(&entry.provider), entry.weight)
                    })
                    .collect(),
                _ => unreachable!("selected weighted provider"),
            };
            (
                "minecraft:weighted_list",
                vec![entry("distribution", NbtSummaryValue::List(distribution))],
            )
        }
        IntProvider::ClampedNormal {
            mean_bits,
            deviation_bits,
            min_inclusive,
            max_inclusive,
        } => (
            "minecraft:clamped_normal",
            vec![
                entry("mean", float(f32::from_bits(*mean_bits))),
                entry("deviation", float(f32::from_bits(*deviation_bits))),
                entry("min_inclusive", NbtSummaryValue::Int(*min_inclusive)),
                entry("max_inclusive", NbtSummaryValue::Int(*max_inclusive)),
            ],
        ),
        IntProvider::Trapezoid {
            min_inclusive,
            max_inclusive,
            plateau,
        } => (
            "minecraft:trapezoid",
            vec![
                entry("min", NbtSummaryValue::Int(*min_inclusive)),
                entry("max", NbtSummaryValue::Int(*max_inclusive)),
                entry("plateau", NbtSummaryValue::Int(*plateau)),
            ],
        ),
    };
    fields.push(entry("type", NbtSummaryValue::String(name.into())));
    NbtSummaryValue::Compound(fields)
}

fn weighted_int_entry_nbt(provider: Cow<'_, IntProvider>, weight: i32) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("data", encode_int_provider(provider)),
        entry("weight", NbtSummaryValue::Int(weight)),
    ])
}

fn int_range_entries(min: i32, max: i32) -> Vec<NbtSummaryEntry> {
    vec![
        entry("min_inclusive", NbtSummaryValue::Int(min)),
        entry("max_inclusive", NbtSummaryValue::Int(max)),
    ]
}

pub(super) fn decode_int_provider_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    minimum: i32,
    maximum: i32,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let provider = decode_int_provider_in(input, field, context, minimum, maximum)?;
    Ok(canonical(provider.into_nbt()))
}

/// Decodes `IntProviders.codec(minimum, maximum)` while retaining one shared
/// recursion/node/collection budget.
pub(super) fn decode_int_provider_in(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    minimum: i32,
    maximum: i32,
) -> Result<IntProvider, RegistryCodecError> {
    let provider = decode_int_provider_value(input, field, context)?;
    if minimum != i32::MIN && provider.min_inclusive() < minimum {
        return Err(context.codec_error(field, format!("Value provider too low: {minimum}")));
    }
    if maximum != i32::MAX && provider.max_inclusive() > maximum {
        return Err(context.codec_error(field, format!("Value provider too high: {maximum}")));
    }
    Ok(provider)
}

fn decode_int_provider_value(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<IntProvider, RegistryCodecError> {
    if input.is_compound() {
        decode_int_provider_dispatch(input, field, context)
    } else {
        decode_constant_int_provider(input, field, context)
    }
}

fn decode_constant_int_provider(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<IntProvider, RegistryCodecError> {
    let value = context.decode_i32(input, field)?;
    Ok(IntProvider::Constant(value))
}

fn decode_int_provider_dispatch(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<IntProvider, RegistryCodecError> {
    context.charge_record(input, field, "IntProvider dispatch")?;
    let type_field = "type";
    context.charge_node(type_field)?;
    let raw_type = required_field(input, "type", field, context)?
        .utf8_string()
        .ok_or_else(|| context.codec_error(type_field, "type is not a string"))?;
    let type_id = normalize_minecraft_identifier(&raw_type, type_field, context)?;
    let family = int_provider_codec_type(&type_id)
        .ok_or_else(|| {
            context.codec_error(
                type_field,
                format!("unknown int provider type `{raw_type}`"),
            )
        })?
        .codec_family;

    match family {
        IntProviderCodecFamily::ConstantInt => {
            decode_constant_int_provider_record(input, field, context)
        }
        IntProviderCodecFamily::UniformInt | IntProviderCodecFamily::BiasedToBottomInt => {
            let (min_inclusive, max_inclusive) = decode_int_range(input, field, context)?;
            Ok(if family == IntProviderCodecFamily::UniformInt {
                IntProvider::Uniform {
                    min_inclusive,
                    max_inclusive,
                }
            } else {
                IntProvider::BiasedToBottom {
                    min_inclusive,
                    max_inclusive,
                }
            })
        }
        IntProviderCodecFamily::ClampedInt => decode_clamped_int_provider(input, field, context),
        IntProviderCodecFamily::WeightedListInt => {
            decode_weighted_list_int_provider(input, field, context)
        }
        IntProviderCodecFamily::ClampedNormalInt => {
            decode_clamped_normal_int_provider(input, field, context)
        }
        IntProviderCodecFamily::TrapezoidInt => {
            decode_trapezoid_int_provider(input, field, context)
        }
    }
}

fn decode_constant_int_provider_record(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<IntProvider, RegistryCodecError> {
    let value = context.decode_i32(required_field(input, "value", field, context)?, "value")?;
    Ok(IntProvider::Constant(value))
}

fn decode_int_range(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(i32, i32), RegistryCodecError> {
    let min = context.decode_i32(
        required_field(input, "min_inclusive", field, context)?,
        "min_inclusive",
    )?;
    let max = context.decode_i32(
        required_field(input, "max_inclusive", field, context)?,
        "max_inclusive",
    )?;
    validate_int_max_at_least_min(field, context, min, max)?;
    Ok((min, max))
}

fn decode_clamped_int_provider(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<IntProvider, RegistryCodecError> {
    let source_field = "source";
    let source = context.with_nested(source_field, |context| {
        decode_int_provider_value(
            required_field(input, "source", field, context)?,
            source_field,
            context,
        )
    })?;
    let (min, max) = decode_int_range(input, field, context)?;
    Ok(IntProvider::Clamped {
        source: Box::new(source),
        min_inclusive: min,
        max_inclusive: max,
    })
}

fn decode_weighted_list_int_provider(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<IntProvider, RegistryCodecError> {
    // The provider uses `WeightedList.nonEmptyCodec` and derives its extrema
    // from every retained item. A list whose
    // weights total zero is empty in that codec (`WeightedList`,
    // `WeightedList`), and construction also rejects a total
    // above signed-int range.
    let distribution_field = "distribution";
    let distribution = required_field(input, "distribution", field, context)?;
    context.charge_node(distribution_field)?;
    let distribution = distribution
        .list_elements()
        .ok_or_else(|| context.codec_error(distribution_field, "distribution is not a list"))?;
    let len = distribution.len();
    context.check_collection(distribution_field, len)?;
    if len == 0 {
        return Err(context.codec_error(
            distribution_field,
            "Weighted list must contain at least one entry with non-zero weight",
        ));
    }

    let mut output = Vec::with_capacity(len);
    let mut total_weight = 0_i64;
    for item in distribution {
        let weighted = context.with_nested(distribution_field, |context| {
            decode_weighted_int_provider_entry(item, distribution_field, context)
        })?;
        total_weight += i64::from(weighted.weight);
        if total_weight > i64::from(i32::MAX) {
            return Err(
                context.codec_error(distribution_field, "sum of weights must be <= 2147483647")
            );
        }
        output.push(weighted);
    }
    if total_weight == 0 {
        return Err(context.codec_error(
            distribution_field,
            "Weighted list must contain at least one entry with non-zero weight",
        ));
    }

    Ok(IntProvider::WeightedList(output))
}

fn decode_weighted_int_provider_entry(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<WeightedIntProvider, RegistryCodecError> {
    context.charge_record(input, field, "weighted int provider entry")?;

    let data_field = "data";
    let data = context.with_nested(data_field, |context| {
        decode_int_provider_value(
            required_field(input, "data", field, context)?,
            data_field,
            context,
        )
    })?;
    let weight = context.decode_i32_range(
        required_field(input, "weight", field, context)?,
        "weight",
        0,
        i32::MAX,
    )?;
    Ok(WeightedIntProvider {
        provider: data,
        weight,
    })
}

fn decode_clamped_normal_int_provider(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<IntProvider, RegistryCodecError> {
    let mean = context.decode_f32(required_field(input, "mean", field, context)?, "mean")?;
    let deviation = context.decode_f32(
        required_field(input, "deviation", field, context)?,
        "deviation",
    )?;
    let (min, max) = decode_int_range(input, field, context)?;
    Ok(IntProvider::ClampedNormal {
        mean_bits: canonical_float_tag_bits(mean),
        deviation_bits: canonical_float_tag_bits(deviation),
        min_inclusive: min,
        max_inclusive: max,
    })
}

fn decode_trapezoid_int_provider(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<IntProvider, RegistryCodecError> {
    let min = context.decode_i32(required_field(input, "min", field, context)?, "min")?;
    let max = context.decode_i32(required_field(input, "max", field, context)?, "max")?;
    let plateau =
        context.decode_i32(required_field(input, "plateau", field, context)?, "plateau")?;
    validate_int_max_at_least_min(field, context, min, max)?;
    // Java evaluates the subtraction as wrapping `int` arithmetic
    if plateau > max.wrapping_sub(min) {
        return Err(context.codec_error(
            field,
            format!("Plateau can at most be the full span: [{min}, {max}]"),
        ));
    }
    Ok(IntProvider::Trapezoid {
        min_inclusive: min,
        max_inclusive: max,
        plateau,
    })
}

fn validate_int_max_at_least_min(
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
    min: i32,
    max: i32,
) -> Result<(), RegistryCodecError> {
    if max < min {
        return Err(context.codec_error(
            field,
            format!("Max must be at least min, min_inclusive: {min}, max_inclusive: {max}"),
        ));
    }
    Ok(())
}

/// `FloatProviders.CODEC`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FloatProvider {
    Constant(f32),
    Uniform {
        min_inclusive: f32,
        max_exclusive: f32,
    },
    ClampedNormal {
        mean: f32,
        deviation: f32,
        min: f32,
        max: f32,
    },
    Trapezoid {
        min: f32,
        max: f32,
        plateau: f32,
    },
}

impl FloatProvider {
    pub fn decode(value: &crate::RegistryJsonValue) -> Result<Self, RegistryCodecError> {
        Self::decode_source(RegistryElementValueRef::Json(value))
    }

    pub fn decode_source(input: RegistryElementValueRef<'_>) -> Result<Self, RegistryCodecError> {
        let source = match input {
            RegistryElementValueRef::Json(_) => crate::RegistryElementSource::ServerDatapackJson,
            _ => crate::RegistryElementSource::NetworkNbt,
        };
        let mut context = RegistryCodecDecodeContext::new(
            "minecraft:float_provider",
            "FloatProviders.CODEC",
            source,
        );
        decode_float_provider_in(input, "", &mut context, f32::NEG_INFINITY, f32::INFINITY)
    }

    /// `FloatProviders.codec(min, max)`.
    pub fn decode_in(
        value: &crate::RegistryJsonValue,
        min: f32,
        max: f32,
    ) -> Result<Self, RegistryCodecError> {
        let mut context = RegistryCodecDecodeContext::new(
            "minecraft:float_provider",
            "FloatProviders.CODEC",
            crate::RegistryElementSource::ServerDatapackJson,
        );
        decode_float_provider_in(
            RegistryElementValueRef::Json(value),
            "",
            &mut context,
            min,
            max,
        )
    }

    /// `FloatProvider.min()` / `max()`.
    const fn bounds(self) -> (f32, f32) {
        match self {
            Self::Constant(value) => (value, value),
            Self::Uniform {
                min_inclusive,
                max_exclusive,
            } => (min_inclusive, max_exclusive),
            Self::ClampedNormal { min, max, .. } | Self::Trapezoid { min, max, .. } => (min, max),
        }
    }

    fn into_nbt(self) -> NbtSummaryValue {
        let (name, mut fields) = match self {
            Self::Constant(value) => return float(value),
            Self::Uniform {
                min_inclusive,
                max_exclusive,
            } => (
                "minecraft:uniform",
                vec![
                    entry("min_inclusive", float(min_inclusive)),
                    entry("max_exclusive", float(max_exclusive)),
                ],
            ),
            Self::ClampedNormal {
                mean,
                deviation,
                min,
                max,
            } => (
                "minecraft:clamped_normal",
                vec![
                    entry("mean", float(mean)),
                    entry("deviation", float(deviation)),
                    entry("min", float(min)),
                    entry("max", float(max)),
                ],
            ),
            Self::Trapezoid { min, max, plateau } => (
                "minecraft:trapezoid",
                vec![
                    entry("min", float(min)),
                    entry("max", float(max)),
                    entry("plateau", float(plateau)),
                ],
            ),
        };
        fields.push(entry("type", NbtSummaryValue::String(name.into())));
        NbtSummaryValue::Compound(fields)
    }
}

pub(super) fn decode_float_provider_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    minimum: f32,
    maximum: f32,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let provider = decode_float_provider_in(input, field, context, minimum, maximum)?;
    Ok(canonical(provider.into_nbt()))
}

/// Decodes `FloatProviders.codec(minimum, maximum)`. Vanilla's outer checks
/// are primitive comparisons, so NaN passes them.
fn decode_float_provider_in(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    minimum: f32,
    maximum: f32,
) -> Result<FloatProvider, RegistryCodecError> {
    let decoded = if input.is_compound() {
        decode_float_provider_dispatch(input, field, context)?
    } else {
        decode_constant_float_provider(input, field, context)?
    };
    let (min, max) = decoded.bounds();
    if min < minimum {
        return Err(context.codec_error(
            field,
            format!("Value provider too low: {minimum} [{}-{}]", min, max),
        ));
    }
    if max > maximum {
        return Err(context.codec_error(
            field,
            format!("Value provider too high: {maximum} [{}-{}]", min, max),
        ));
    }
    Ok(decoded)
}

fn decode_constant_float_provider(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<FloatProvider, RegistryCodecError> {
    let value = constant_float(context.decode_f32(input, field)?);
    Ok(FloatProvider::Constant(value))
}

fn decode_float_provider_dispatch(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<FloatProvider, RegistryCodecError> {
    context.charge_record(input, field, "FloatProvider dispatch")?;
    let type_field = "type";
    context.charge_node(type_field)?;
    let raw_type = required_field(input, "type", field, context)?
        .utf8_string()
        .ok_or_else(|| context.codec_error(type_field, "type is not a string"))?;
    let type_id = normalize_minecraft_identifier(&raw_type, type_field, context)?;
    let family = float_provider_codec_type(&type_id)
        .ok_or_else(|| {
            context.codec_error(
                type_field,
                format!("unknown float provider type `{raw_type}`"),
            )
        })?
        .codec_family;

    match family {
        FloatProviderCodecFamily::ConstantFloat => {
            decode_constant_float_provider_record(input, field, context)
        }
        FloatProviderCodecFamily::UniformFloat => {
            decode_uniform_float_provider(input, field, context)
        }
        FloatProviderCodecFamily::ClampedNormalFloat => {
            decode_clamped_normal_float_provider(input, field, context)
        }
        FloatProviderCodecFamily::TrapezoidFloat => {
            decode_trapezoid_float_provider(input, field, context)
        }
    }
}

fn decode_constant_float_provider_record(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<FloatProvider, RegistryCodecError> {
    let value = constant_float(
        context.decode_f32(required_field(input, "value", field, context)?, "value")?,
    );
    Ok(FloatProvider::Constant(value))
}

fn decode_uniform_float_provider(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<FloatProvider, RegistryCodecError> {
    let min = context.decode_f32(
        required_field(input, "min_inclusive", field, context)?,
        "min_inclusive",
    )?;
    let max = context.decode_f32(
        required_field(input, "max_exclusive", field, context)?,
        "max_exclusive",
    )?;
    // The record validator uses primitive `max <= min`; NaN therefore passes
    // and equal signed zeros fail.
    if max <= min {
        return Err(context.codec_error(
            field,
            format!("Max must be larger than min, min: {min}, max: {max}"),
        ));
    }
    Ok(FloatProvider::Uniform {
        min_inclusive: min,
        max_exclusive: max,
    })
}

fn decode_clamped_normal_float_provider(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<FloatProvider, RegistryCodecError> {
    let mean = context.decode_f32(required_field(input, "mean", field, context)?, "mean")?;
    let deviation = context.decode_f32(
        required_field(input, "deviation", field, context)?,
        "deviation",
    )?;
    let min = context.decode_f32(required_field(input, "min", field, context)?, "min")?;
    let max = context.decode_f32(required_field(input, "max", field, context)?, "max")?;
    // This is the record's primitive comparison, not boxed Float ordering;
    // equal endpoints pass while NaN makes the comparison false
    if max < min {
        return Err(context.codec_error(
            field,
            format!("Max must be larger than min: [{min}, {max}]"),
        ));
    }
    Ok(FloatProvider::ClampedNormal {
        mean,
        deviation,
        min,
        max,
    })
}

fn decode_trapezoid_float_provider(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<FloatProvider, RegistryCodecError> {
    let min = context.decode_f32(required_field(input, "min", field, context)?, "min")?;
    let max = context.decode_f32(required_field(input, "max", field, context)?, "max")?;
    let plateau =
        context.decode_f32(required_field(input, "plateau", field, context)?, "plateau")?;
    // Negative plateaus are accepted; only max/min and the primitive f32
    // subtraction constrain this field.
    if max < min {
        return Err(context.codec_error(
            field,
            format!("Max must be larger than min: [{min}, {max}]"),
        ));
    }
    if plateau > max - min {
        return Err(context.codec_error(
            field,
            format!("Plateau can at most be the full span: [{min}, {max}]"),
        ));
    }
    Ok(FloatProvider::Trapezoid { min, max, plateau })
}

// `ConstantFloat.of` interns either signed zero as its `ZERO` singleton
// before the outer codec writes the scalar.
fn constant_float(value: f32) -> f32 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

fn normalize_minecraft_identifier(
    raw: &str,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<String, RegistryCodecError> {
    RegistryResourceIdentifier::parse(raw)
        .map(RegistryResourceIdentifier::into_string)
        .ok_or_else(|| context.codec_error(field, format!("invalid resource identifier `{raw}`")))
}

fn canonical(value: NbtSummaryValue) -> CanonicalRegistryCodecValue {
    CanonicalRegistryCodecValue::new(value)
}

fn float(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(canonical_float_tag_bits(value))
}

#[cfg(test)]
mod tests;
