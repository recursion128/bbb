//! Exact value codecs used by `EnvironmentAttributeMap` entries.

use std::cmp::Ordering;

use crate::{
    persistent_component_nbt::source_codec::decode_sound_event_holder_source,
    registry_codec::{
        recursive_values::{
            decode_compact_list, decode_list, required_field, CanonicalRegistryCodecValue,
            RegistryCodecDecodeContext,
        },
        RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
    },
};
use bbb_facts::environment_attribute_types::{
    VanillaEnvironmentAttribute, VanillaEnvironmentAttributeRange, VanillaEnvironmentAttributeType,
    VanillaEnvironmentModifierFamily,
};
use bbb_protocol::{
    nbt_summary::{canonical_double_tag_bits, canonical_float_tag_bits, entry},
    NbtSummaryValue,
};

use super::bed_rule::decode_bed_rule_source;

pub(crate) fn decode_attribute_value_source(
    descriptor: &VanillaEnvironmentAttribute,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let value = decode_unchecked_value_source(descriptor.value_type, input, field, context)?;
    validate_attribute_range(descriptor, &value, field, context)?;
    Ok(value)
}

pub(crate) fn decode_modifier_argument_source(
    descriptor: &VanillaEnvironmentAttribute,
    modifier: &str,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    if modifier == "override" {
        return decode_attribute_value_source(descriptor, input, field, context);
    }
    match descriptor.value_type.modifier_family() {
        VanillaEnvironmentModifierFamily::Boolean => {
            ensure_modifier(
                modifier,
                &["and", "nand", "or", "nor", "xor", "xnor"],
                field,
                context,
            )?;
            decode_boolean(input, field, context)
        }
        VanillaEnvironmentModifierFamily::Float => {
            ensure_modifier(
                modifier,
                &[
                    "alpha_blend",
                    "add",
                    "subtract",
                    "multiply",
                    "minimum",
                    "maximum",
                ],
                field,
                context,
            )?;
            if modifier == "alpha_blend" {
                decode_float_with_alpha(input, field, context)
            } else {
                decode_float(input, field, context)
            }
        }
        VanillaEnvironmentModifierFamily::Rgb => {
            ensure_modifier(
                modifier,
                &[
                    "alpha_blend",
                    "add",
                    "subtract",
                    "multiply",
                    "blend_to_gray",
                ],
                field,
                context,
            )?;
            match modifier {
                "alpha_blend" => decode_string_argb_color(input, field, context),
                "blend_to_gray" => decode_blend_to_gray(input, field, context),
                _ => decode_string_rgb_color(input, field, context),
            }
        }
        VanillaEnvironmentModifierFamily::Argb => {
            ensure_modifier(
                modifier,
                &[
                    "alpha_blend",
                    "add",
                    "subtract",
                    "multiply",
                    "blend_to_gray",
                ],
                field,
                context,
            )?;
            match modifier {
                "alpha_blend" => decode_string_argb_color(input, field, context),
                "blend_to_gray" => decode_blend_to_gray(input, field, context),
                _ => decode_argb_modifier_color(input, field, context),
            }
        }
        VanillaEnvironmentModifierFamily::Integer => {
            ensure_modifier(
                modifier,
                &["add", "subtract", "multiply", "minimum", "maximum"],
                field,
                context,
            )?;
            decode_integer(input, field, context)
        }
        VanillaEnvironmentModifierFamily::OverrideOnly => Err(context.codec_error(
            field,
            format!("modifier `{modifier}` is not allowed for this attribute type"),
        )),
    }
}

fn decode_unchecked_value_source(
    value_type: VanillaEnvironmentAttributeType,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    match value_type {
        VanillaEnvironmentAttributeType::Activity => decode_activity(input, field, context),
        VanillaEnvironmentAttributeType::AmbientParticles => {
            decode_list(input, field, context, decode_ambient_particle)
        }
        VanillaEnvironmentAttributeType::AmbientSounds => {
            decode_ambient_sounds(input, field, context)
        }
        VanillaEnvironmentAttributeType::AngleDegrees | VanillaEnvironmentAttributeType::Float => {
            decode_float(input, field, context)
        }
        VanillaEnvironmentAttributeType::ArgbColor => {
            decode_string_argb_color(input, field, context)
        }
        VanillaEnvironmentAttributeType::BackgroundMusic => {
            decode_background_music(input, field, context)
        }
        VanillaEnvironmentAttributeType::BedRule => decode_bed_rule_source(input, field, context),
        VanillaEnvironmentAttributeType::Boolean => decode_boolean(input, field, context),
        VanillaEnvironmentAttributeType::Integer => decode_integer(input, field, context),
        VanillaEnvironmentAttributeType::MoonPhase => decode_moon_phase(input, field, context),
        VanillaEnvironmentAttributeType::Particle => {
            super::super::environment_particle_source::decode_particle_options_source(
                input, field, context,
            )
        }
        VanillaEnvironmentAttributeType::RgbColor => decode_string_rgb_color(input, field, context),
        VanillaEnvironmentAttributeType::TriState => decode_tri_state(input, field, context),
    }
}

fn decode_activity(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let value = input
        .java_string_ref()
        .and_then(RegistryResourceIdentifier::parse_java_string)
        .and_then(|resource_id| {
            bbb_facts::activity_types::vanilla_activity_for_resource_id(resource_id.as_str())
        })
        .ok_or_else(|| context.codec_error(field, "unknown built-in activity"))?;
    Ok(canonical(NbtSummaryValue::String(value.resource_id.into())))
}

fn decode_boolean(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    Ok(canonical(NbtSummaryValue::Byte(i8::from(
        context.decode_bool(input, field)?,
    ))))
}

fn decode_float(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    Ok(canonical(NbtSummaryValue::Float(canonical_float_tag_bits(
        context.decode_f32(input, field)?,
    ))))
}

fn decode_integer(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    Ok(canonical(NbtSummaryValue::Int(
        context.decode_i32(input, field)?,
    )))
}

fn decode_moon_phase(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let value = input
        .java_string_ref()
        .filter(|value| {
            [
                "full_moon",
                "waning_gibbous",
                "third_quarter",
                "waning_crescent",
                "new_moon",
                "waxing_crescent",
                "first_quarter",
                "waxing_gibbous",
            ]
            .iter()
            .any(|name| *value == *name)
        })
        .ok_or_else(|| context.codec_error(field, "unknown moon phase"))?;
    Ok(canonical(NbtSummaryValue::String(value.clone())))
}

fn decode_tri_state(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    if let Some(value) = input.as_bool() {
        return Ok(canonical(NbtSummaryValue::Byte(i8::from(value))));
    }
    match input.java_string_ref() {
        Some(value) if value == "true" => Ok(canonical(NbtSummaryValue::Byte(1))),
        Some(value) if value == "false" => Ok(canonical(NbtSummaryValue::Byte(0))),
        Some(value) if value == "default" => Ok(canonical(NbtSummaryValue::String(value.clone()))),
        _ => Err(context.codec_error(field, "unknown tri-state value")),
    }
}

fn decode_ambient_particle(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "ambient particle")?;
    let particle = required_field(input, "particle", field, context)?;
    let particle = context.with_nested("particle", |context| {
        super::super::environment_particle_source::decode_particle_options_source(
            particle, "particle", context,
        )
    })?;
    let probability = decode_codec_float_range(
        required_field(input, "probability", field, context)?,
        "probability",
        context,
        0.0,
        1.0,
    )?;
    let (particle, references) = particle.into_parts();
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(vec![
            entry("particle", particle),
            entry(
                "probability",
                NbtSummaryValue::Float(canonical_float_tag_bits(probability)),
            ),
        ]),
        references,
    ))
}

fn decode_background_music(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "background music")?;
    let mut fields = Vec::new();
    let mut references = Vec::new();
    for name in ["default", "creative", "underwater"] {
        let Some(value) = input.field(name) else {
            continue;
        };
        let decoded = context.with_nested(name, |context| decode_music(value, name, context))?;
        let (value, value_references) = decoded.into_parts();
        fields.push(entry(name, value));
        references.extend(value_references);
    }
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(fields),
        references,
    ))
}

fn decode_music(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "music")?;
    let sound = decode_sound_event_holder_source(
        required_field(input, "sound", field, context)?,
        "sound",
        context,
    )?;
    let min_delay = context.decode_i32_range(
        required_field(input, "min_delay", field, context)?,
        "min_delay",
        0,
        i32::MAX,
    )?;
    let max_delay = context.decode_i32_range(
        required_field(input, "max_delay", field, context)?,
        "max_delay",
        0,
        i32::MAX,
    )?;
    let replace_current_music = input
        .field("replace_current_music")
        .map(|value| context.decode_bool(value, "replace_current_music"))
        .transpose()?
        .unwrap_or(false);
    let (sound, references) = sound.into_parts();
    let mut fields = vec![
        entry("sound", sound),
        entry("min_delay", NbtSummaryValue::Int(min_delay)),
        entry("max_delay", NbtSummaryValue::Int(max_delay)),
    ];
    if replace_current_music {
        fields.push(entry("replace_current_music", NbtSummaryValue::Byte(1)));
    }
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(fields),
        references,
    ))
}

fn decode_ambient_sounds(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "ambient sounds")?;
    let mut fields = Vec::new();
    let mut references = Vec::new();
    if let Some(value) = input.field("loop") {
        let decoded = decode_sound_event_holder_source(value, "loop", context)?;
        let (value, value_references) = decoded.into_parts();
        fields.push(entry("loop", value));
        references.extend(value_references);
    }
    if let Some(value) = input.field("mood") {
        let decoded = context.with_nested("mood", |context| {
            decode_ambient_mood(value, "mood", context)
        })?;
        let (value, value_references) = decoded.into_parts();
        fields.push(entry("mood", value));
        references.extend(value_references);
    }
    if let Some(value) = input.field("additions") {
        let decoded = context.with_nested("additions", |context| {
            decode_compact_list(value, "additions", context, decode_ambient_addition)
        })?;
        let (value, value_references) = decoded.into_parts();
        if !matches!(&value, NbtSummaryValue::List(values) if values.is_empty()) {
            fields.push(entry("additions", value));
        }
        references.extend(value_references);
    }
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(fields),
        references,
    ))
}

fn decode_ambient_mood(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "ambient mood")?;
    let sound = decode_sound_event_holder_source(
        required_field(input, "sound", field, context)?,
        "sound",
        context,
    )?;
    let tick_delay = context.decode_i32(
        required_field(input, "tick_delay", field, context)?,
        "tick_delay",
    )?;
    let block_search_extent = context.decode_i32(
        required_field(input, "block_search_extent", field, context)?,
        "block_search_extent",
    )?;
    let offset = context.decode_f64(required_field(input, "offset", field, context)?, "offset")?;
    let (sound, references) = sound.into_parts();
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(vec![
            entry("sound", sound),
            entry("tick_delay", NbtSummaryValue::Int(tick_delay)),
            entry(
                "block_search_extent",
                NbtSummaryValue::Int(block_search_extent),
            ),
            entry(
                "offset",
                NbtSummaryValue::Double(canonical_double_tag_bits(offset)),
            ),
        ]),
        references,
    ))
}

fn decode_ambient_addition(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "ambient addition")?;
    let sound = decode_sound_event_holder_source(
        required_field(input, "sound", field, context)?,
        "sound",
        context,
    )?;
    let tick_chance = context.decode_f64(
        required_field(input, "tick_chance", field, context)?,
        "tick_chance",
    )?;
    let (sound, references) = sound.into_parts();
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(vec![
            entry("sound", sound),
            entry(
                "tick_chance",
                NbtSummaryValue::Double(canonical_double_tag_bits(tick_chance)),
            ),
        ]),
        references,
    ))
}

fn validate_attribute_range(
    descriptor: &VanillaEnvironmentAttribute,
    value: &CanonicalRegistryCodecValue,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let VanillaEnvironmentAttributeRange::Any = descriptor.value_range else {
        let NbtSummaryValue::Float(bits) = value.network_nbt_unchecked() else {
            return Err(context.codec_error(field, "non-float value uses a float range"));
        };
        let value = f32::from_bits(*bits);
        let valid = descriptor.value_range.accepts_float(value);
        if !valid {
            return Err(
                context.codec_error(field, "value is outside its environment attribute range")
            );
        }
        return Ok(());
    };
    Ok(())
}

fn ensure_modifier(
    modifier: &str,
    allowed: &[&str],
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    if allowed.contains(&modifier) {
        Ok(())
    } else {
        Err(context.codec_error(field, format!("modifier `{modifier}` is not allowed")))
    }
}

fn decode_float_with_alpha(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    if input.map_len().is_none() {
        return decode_float(input, field, context);
    }
    context.charge_node(field)?;
    let len = input.map_len().expect("checked above");
    context.check_collection(field, len)?;
    let value = required_field(input, "value", field, context)?;
    let value = decode_float(value, "value", context)?;
    let alpha = match input.field("alpha") {
        Some(alpha) => {
            let alpha = decode_codec_float_range(alpha, "alpha", context, 0.0, 1.0)?;
            Some(alpha)
        }
        None => None,
    };
    if alpha.is_none_or(|alpha| alpha == 1.0) {
        return Ok(value);
    }
    let fields = vec![
        entry("value", value.network_nbt_unchecked().clone()),
        entry(
            "alpha",
            NbtSummaryValue::Float(canonical_float_tag_bits(alpha.expect("checked"))),
        ),
    ];
    Ok(canonical(NbtSummaryValue::Compound(fields)))
}

fn decode_blend_to_gray(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "blend_to_gray argument")?;
    let brightness = decode_codec_float_range(
        required_field(input, "brightness", field, context)?,
        "brightness",
        context,
        0.0,
        1.0,
    )?;
    let factor = decode_codec_float_range(
        required_field(input, "factor", field, context)?,
        "factor",
        context,
        0.0,
        1.0,
    )?;
    Ok(canonical(NbtSummaryValue::Compound(vec![
        entry(
            "brightness",
            NbtSummaryValue::Float(canonical_float_tag_bits(brightness)),
        ),
        entry(
            "factor",
            NbtSummaryValue::Float(canonical_float_tag_bits(factor)),
        ),
    ])))
}

fn decode_codec_float_range(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    min: f32,
    max: f32,
) -> Result<f32, RegistryCodecError> {
    let value = context.decode_f32(input, field)?;
    if java_float_compare(value, min).is_lt() || java_float_compare(value, max).is_gt() {
        return Err(context.codec_error(field, "float is outside the allowed range"));
    }
    Ok(value)
}

fn java_float_compare(left: f32, right: f32) -> Ordering {
    match (left.is_nan(), right.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => left.total_cmp(&right),
    }
}

fn decode_string_rgb_color(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let color = decode_color(input, field, context, false)?;
    if input.codec_i32().is_some() {
        return Ok(canonical(NbtSummaryValue::Int(color as i32)));
    }
    Ok(canonical(NbtSummaryValue::String(
        format!("#{:06x}", color & 0x00ff_ffff).into(),
    )))
}

fn decode_string_argb_color(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let color = decode_color(input, field, context, true)?;
    Ok(canonical(NbtSummaryValue::String(
        format!("#{color:08x}").into(),
    )))
}

fn decode_argb_modifier_color(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let color = decode_color(input, field, context, true)?;
    if color >> 24 == 0xff {
        Ok(canonical(NbtSummaryValue::Int(color as i32)))
    } else {
        Ok(canonical(NbtSummaryValue::String(
            format!("#{color:08x}").into(),
        )))
    }
}

pub(super) fn decode_color(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    argb: bool,
) -> Result<u32, RegistryCodecError> {
    context.charge_node(field)?;
    if let Some(value) = input.utf8_string() {
        let digits = if argb { 8 } else { 6 };
        if value.len() == digits + 1
            && value.starts_with('#')
            && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            let value = u32::from_str_radix(&value[1..], 16)
                .map_err(|_| context.codec_error(field, "invalid hexadecimal color"))?;
            return Ok(if argb { value } else { value | 0xff00_0000 });
        }
        return Err(context.codec_error(field, "invalid hexadecimal color"));
    }
    if let Some(value) = input.codec_i32() {
        return Ok(value as u32);
    }
    let expected = if argb { 4 } else { 3 };
    let elements = input
        .list_elements()
        .ok_or_else(|| context.codec_error(field, "color is not a string, int, or vector"))?;
    let len = elements.len();
    if len != expected {
        return Err(
            context.codec_error(field, format!("color vector must have {expected} entries"))
        );
    }
    context.check_collection(field, len)?;
    let mut channels = [0u32; 4];
    for (index, element) in elements.enumerate() {
        let value = element
            .codec_f32()
            .ok_or_else(|| context.codec_error(field, "color channel is not a float"))?;
        context.charge_node(field)?;
        channels[index] = ((value * 255.0).floor() as i32 as u32) & 0xff;
    }
    let alpha = if argb { channels[3] } else { 0xff };
    Ok(alpha << 24 | channels[0] << 16 | channels[1] << 8 | channels[2])
}

fn canonical(value: NbtSummaryValue) -> CanonicalRegistryCodecValue {
    CanonicalRegistryCodecValue::new(value)
}
