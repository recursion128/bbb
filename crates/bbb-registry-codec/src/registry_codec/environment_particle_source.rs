//! Source-aware owner for Vanilla 26.1 `ParticleTypes.CODEC`.
//!
//! The built-in type catalog closes dispatch before any payload is decoded.
//! Each selected `MapCodec` writes its payload first and the particle `type`
//! discriminator last, matching DFU `KeyDispatchCodec.encode`.

use crate::persistent_component_nbt::source_codec::decode_item_stack_template_source;
use bbb_facts::particle_types::{
    vanilla_particle_type_for_resource_id, VanillaParticleCodecFamily,
};
use bbb_protocol::{
    nbt_summary::{canonical_double_tag_bits, canonical_float_tag_bits},
    normalize_resource_location, NbtSummaryValue,
};

use super::{
    block_state_source::decode_block_state_source,
    recursive_values::{
        required_field as required, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
        RegistryCodecRecord,
    },
    RegistryCodecError, RegistryElementValueRef,
};

/// Decodes `ParticleTypes.CODEC` from any registry element source.
pub(crate) fn decode_particle_options_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "particle options")?;
    let raw_type = required(input, "type", field, context)?
        .utf8_string()
        .ok_or_else(|| context.codec_error("type", "expected a particle type string"))?;
    let type_id = normalize_resource_location(raw_type.as_str())
        .map_err(|_| context.codec_error("type", "invalid particle type identifier"))?;
    let family = vanilla_particle_type_for_resource_id(&type_id)
        .map(|entry| entry.codec_family)
        .or_else(|| {
            context
                .content_registry()
                .particle_by_resource_id(&type_id)
                .map(|_| VanillaParticleCodecFamily::Simple)
        })
        .ok_or_else(|| {
            context.codec_error("type", format!("unknown particle type `{raw_type}`"))
        })?;

    let mut object = RegistryCodecRecord::default();
    match family {
        VanillaParticleCodecFamily::Simple => {}
        VanillaParticleCodecFamily::Block => decode_block(input, field, context, &mut object)?,
        VanillaParticleCodecFamily::Power => {
            let power = decode_optional_f32(input, "power", 1.0, context)?;
            if power.to_bits() != 1.0_f32.to_bits() {
                object.push("power", f32_value(power, "power", context)?);
            }
        }
        VanillaParticleCodecFamily::Dust => {
            let color = decode_required_color(input, "color", 3, field, context)?;
            object.push("color", i32_value(color, "color", context)?);
            let scale = decode_required_ranged_f32(input, "scale", 0.01, 4.0, field, context)?;
            object.push("scale", f32_value(scale, "scale", context)?);
        }
        VanillaParticleCodecFamily::DustColorTransition => {
            for name in ["from_color", "to_color"] {
                let color = decode_required_color(input, name, 3, field, context)?;
                object.push(name, i32_value(color, name, context)?);
            }
            let scale = decode_required_ranged_f32(input, "scale", 0.01, 4.0, field, context)?;
            object.push("scale", f32_value(scale, "scale", context)?);
        }
        VanillaParticleCodecFamily::Spell => {
            let color = decode_optional_color(input, "color", -1, 3, context)?;
            if color != -1 {
                object.push("color", i32_value(color, "color", context)?);
            }
            let power = decode_optional_f32(input, "power", 1.0, context)?;
            if power.to_bits() != 1.0_f32.to_bits() {
                object.push("power", f32_value(power, "power", context)?);
            }
        }
        VanillaParticleCodecFamily::Color => {
            let color = decode_required_color(input, "color", 4, field, context)?;
            object.push("color", i32_value(color, "color", context)?);
        }
        VanillaParticleCodecFamily::SculkCharge => {
            let roll = decode_f32(required(input, "roll", field, context)?, "roll", context)?;
            object.push("roll", f32_value(roll, "roll", context)?);
        }
        VanillaParticleCodecFamily::Item => {
            let item = required(input, "item", field, context)?;
            let decoded = context.with_nested("item", |context| {
                decode_item_stack_template_source(item, "item", context)
            })?;
            object.push("item", decoded);
        }
        VanillaParticleCodecFamily::Vibration => {
            decode_vibration(input, field, context, &mut object)?
        }
        VanillaParticleCodecFamily::Trail => {
            let target = required(input, "target", field, context)?;
            let target =
                context.with_nested("target", |context| decode_vec3(target, "target", context))?;
            object.push("target", target);
            let color = decode_required_color(input, "color", 3, field, context)?;
            object.push("color", i32_value(color, "color", context)?);
            let duration = required(input, "duration", field, context)?
                .codec_i32()
                .filter(|value| *value > 0)
                .ok_or_else(|| context.codec_error("duration", "expected a positive Codec.INT"))?;
            object.push("duration", i32_value(duration, "duration", context)?);
        }
        VanillaParticleCodecFamily::Shriek => {
            let delay = required(input, "delay", field, context)?
                .codec_i32()
                .ok_or_else(|| context.codec_error("delay", "expected a numeric Codec.INT"))?;
            object.push("delay", i32_value(delay, "delay", context)?);
        }
    }
    object.push("type", string_value(&type_id, "type", context)?);
    Ok(object.finish())
}

fn string_value(
    value: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::String(
        value.into(),
    )))
}

fn i32_value(
    value: i32,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::Int(
        value,
    )))
}

fn f32_value(
    value: f32,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::Float(
        canonical_float_tag_bits(value),
    )))
}

fn decode_f32(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<f32, RegistryCodecError> {
    input
        .codec_f32()
        .ok_or_else(|| context.codec_error(field, "expected a numeric Codec.FLOAT"))
}

fn decode_optional_f32(
    input: RegistryElementValueRef<'_>,
    name: &str,
    default: f32,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<f32, RegistryCodecError> {
    input
        .field(name)
        .map(|value| decode_f32(value, name, context))
        .unwrap_or(Ok(default))
}

fn decode_required_ranged_f32(
    input: RegistryElementValueRef<'_>,
    name: &str,
    min: f32,
    max: f32,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<f32, RegistryCodecError> {
    let value = decode_f32(required(input, name, field, context)?, name, context)?;
    if !(value >= min && value <= max) {
        return Err(context.codec_error(name, format!("value is outside [{min},{max}]")));
    }
    Ok(value)
}

fn decode_required_color(
    input: RegistryElementValueRef<'_>,
    name: &str,
    vector_len: usize,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<i32, RegistryCodecError> {
    let value = required(input, name, field, context)?;
    decode_color(value, vector_len, name, context)
}

fn decode_optional_color(
    input: RegistryElementValueRef<'_>,
    name: &str,
    default: i32,
    vector_len: usize,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<i32, RegistryCodecError> {
    input
        .field(name)
        .map(|value| decode_color(value, vector_len, name, context))
        .unwrap_or(Ok(default))
}

fn decode_color(
    input: RegistryElementValueRef<'_>,
    vector_len: usize,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<i32, RegistryCodecError> {
    if let Some(value) = input.codec_i32() {
        return Ok(value);
    }
    let elements = input
        .list_elements()
        .ok_or_else(|| context.codec_error(field, "expected an integer or color vector"))?;
    let len = elements.len();
    context.check_collection(field, len)?;
    if len != vector_len {
        return Err(context.codec_error(
            field,
            format!("color vector must contain {vector_len} floats"),
        ));
    }
    let mut values = [0; 4];
    for (index, element) in elements.enumerate() {
        let value = element
            .codec_f32()
            .ok_or_else(|| context.codec_error(field, "color vector contains a non-number"))?;
        values[index] = java_color_channel(value);
    }
    let (alpha, red, green, blue) = if vector_len == 4 {
        (values[3], values[0], values[1], values[2])
    } else {
        (255, values[0], values[1], values[2])
    };
    Ok(((alpha & 255) << 24) | ((red & 255) << 16) | ((green & 255) << 8) | (blue & 255))
}

fn java_color_channel(value: f32) -> i32 {
    (value * 255.0).floor() as i32
}

pub(crate) struct DecodedVec3Source {
    pub(crate) canonical: CanonicalRegistryCodecValue,
    pub(crate) values: [f64; 3],
}

fn decode_vec3(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    Ok(decode_vec3_source(input, field, context)?.canonical)
}

pub(crate) fn decode_vec3_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<DecodedVec3Source, RegistryCodecError> {
    context.charge_node(field)?;
    let elements = input
        .list_elements()
        .ok_or_else(|| context.codec_error(field, "Vec3 is not a list"))?;
    let len = elements.len();
    context.check_collection(field, len)?;
    if len != 3 {
        return Err(context.codec_error(field, "Vec3 must contain three doubles"));
    }
    let mut values = [0.0; 3];
    let mut nbt = Vec::with_capacity(3);
    for (index, element) in elements.enumerate() {
        let value = context.decode_f64(element, field)?;
        values[index] = value;
        nbt.push(NbtSummaryValue::Double(canonical_double_tag_bits(value)));
    }
    Ok(DecodedVec3Source {
        canonical: CanonicalRegistryCodecValue::new(NbtSummaryValue::List(nbt)),
        values,
    })
}

fn decode_vibration(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    object: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    let destination = required(input, "destination", field, context)?;
    let decoded = context.with_nested("destination", |context| {
        decode_position_source(destination, "destination", context)
    })?;
    object.push("destination", decoded);
    let arrival = required(input, "arrival_in_ticks", field, context)?
        .codec_i32()
        .ok_or_else(|| context.codec_error("arrival_in_ticks", "expected a numeric Codec.INT"))?;
    object.push(
        "arrival_in_ticks",
        i32_value(arrival, "arrival_in_ticks", context)?,
    );
    Ok(())
}

fn decode_position_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "position source")?;
    let raw = required(input, "type", field, context)?
        .utf8_string()
        .ok_or_else(|| context.codec_error("type", "position source type is not a string"))?;
    let source_type = normalize_resource_location(raw.as_str())
        .map_err(|_| context.codec_error("type", "invalid position source type"))?;
    match source_type.as_str() {
        "minecraft:block" => {
            let pos = required(input, "pos", field, context)?;
            let pos =
                context.with_nested("pos", |context| decode_block_pos(pos, "pos", context))?;
            let mut object = RegistryCodecRecord::default();
            object.push("pos", pos);
            object.push("type", string_value(&source_type, "type", context)?);
            Ok(object.finish())
        }
        "minecraft:entity" => {
            // Decode the selected payload before Vibration's SAFE validator.
            let uuid = required(input, "source_entity", field, context)?;
            decode_fixed_i32_list(uuid, 4, "source_entity", context)?;
            Err(context.codec_error(field, "entity position sources are not allowed"))
        }
        _ => Err(context.codec_error("type", format!("unknown position source type `{raw}`"))),
    }
}

fn decode_block_pos(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let values = decode_fixed_i32_list(input, 3, field, context)?;
    context.charge_nodes(field, 3)?;
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::IntArray(
        values,
    )))
}

fn decode_fixed_i32_list(
    input: RegistryElementValueRef<'_>,
    expected: usize,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<Vec<i32>, RegistryCodecError> {
    let values = input
        .list_elements()
        .ok_or_else(|| context.codec_error(field, "expected an integer list"))?;
    let len = values.len();
    context.check_collection(field, len)?;
    if len != expected {
        return Err(context.codec_error(field, format!("expected {expected} integers")));
    }
    values
        .map(|value| {
            value
                .codec_i32()
                .ok_or_else(|| context.codec_error(field, "expected an integer"))
        })
        .collect()
}

fn decode_block(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    object: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    let source = required(input, "block_state", field, context)?;
    let state = context.with_nested("block_state", |context| {
        decode_block_state_source(source, "block_state", context)
    })?;
    object.push("block_state", state);
    Ok(())
}

#[cfg(test)]
mod tests;
