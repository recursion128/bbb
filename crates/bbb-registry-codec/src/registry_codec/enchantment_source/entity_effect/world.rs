//! Exact world-mutation enchantment entity/location effect payloads.
//!
//! These five payload records are shared by `EnchantmentEntityEffect` and
//! `EnchantmentLocationBasedEffect`. Their field order follows the owning
//! `RecordCodecBuilder`; the enclosing dispatch appends `type` last

use crate::{
    enchantment_codec_types::{
        EnchantmentEntityEffectCodecFamily, EnchantmentLocationBasedEffectCodecFamily,
    },
    persistent_component_nbt::source_codec::{
        decode_fixed_holder_reference_source, decode_homogeneous_fixed_holder_set_source,
        decode_sound_event_holder_source, java_float_compare,
    },
    registry_codec::{
        environment_particle_source::{decode_particle_options_source, decode_vec3_source},
        recursive_values::{
            required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
            RegistryCodecRecord,
        },
        value_provider_source::decode_float_provider_source,
        RegistryCodecError, RegistryElementValueRef,
    },
};
use bbb_protocol::{nbt_summary::canonical_float_tag_bits, NbtSummaryEntry, NbtSummaryValue};

use super::super::{
    block_predicate::{decode_block_predicate_source, decode_vec3i},
    block_state_provider::decode_block_state_provider_source,
    value::{child, decode_level_based_value_source},
};

const BLOCK_REGISTRY: &str = "minecraft:block";
const DAMAGE_TYPE_REGISTRY: &str = "minecraft:damage_type";
const GAME_EVENT_REGISTRY: &str = "minecraft:game_event";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorldEffectKind {
    Explode,
    ReplaceBlock,
    ReplaceDisk,
    SetBlockProperties,
    SpawnParticles,
}

pub(super) fn decode_entity_world_effect_source(
    family: EnchantmentEntityEffectCodecFamily,
    type_id: String,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Option<Result<CanonicalRegistryCodecValue, RegistryCodecError>> {
    entity_kind(family).map(|kind| decode_world_effect(kind, type_id, input, field, context))
}

pub(super) fn decode_location_world_effect_source(
    family: EnchantmentLocationBasedEffectCodecFamily,
    type_id: String,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Option<Result<CanonicalRegistryCodecValue, RegistryCodecError>> {
    location_kind(family).map(|kind| decode_world_effect(kind, type_id, input, field, context))
}

fn entity_kind(family: EnchantmentEntityEffectCodecFamily) -> Option<WorldEffectKind> {
    match family {
        EnchantmentEntityEffectCodecFamily::ExplodeEffect => Some(WorldEffectKind::Explode),
        EnchantmentEntityEffectCodecFamily::ReplaceBlock => Some(WorldEffectKind::ReplaceBlock),
        EnchantmentEntityEffectCodecFamily::ReplaceDisk => Some(WorldEffectKind::ReplaceDisk),
        EnchantmentEntityEffectCodecFamily::SetBlockProperties => {
            Some(WorldEffectKind::SetBlockProperties)
        }
        EnchantmentEntityEffectCodecFamily::SpawnParticlesEffect => {
            Some(WorldEffectKind::SpawnParticles)
        }
        _ => None,
    }
}

fn location_kind(family: EnchantmentLocationBasedEffectCodecFamily) -> Option<WorldEffectKind> {
    match family {
        EnchantmentLocationBasedEffectCodecFamily::ExplodeEffect => Some(WorldEffectKind::Explode),
        EnchantmentLocationBasedEffectCodecFamily::ReplaceBlock => {
            Some(WorldEffectKind::ReplaceBlock)
        }
        EnchantmentLocationBasedEffectCodecFamily::ReplaceDisk => {
            Some(WorldEffectKind::ReplaceDisk)
        }
        EnchantmentLocationBasedEffectCodecFamily::SetBlockProperties => {
            Some(WorldEffectKind::SetBlockProperties)
        }
        EnchantmentLocationBasedEffectCodecFamily::SpawnParticlesEffect => {
            Some(WorldEffectKind::SpawnParticles)
        }
        _ => None,
    }
}

fn decode_world_effect(
    kind: WorldEffectKind,
    type_id: String,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    match kind {
        WorldEffectKind::Explode => decode_explode(type_id, input, field, context),
        WorldEffectKind::ReplaceBlock => decode_replace_block(type_id, input, field, context),
        WorldEffectKind::ReplaceDisk => decode_replace_disk(type_id, input, field, context),
        WorldEffectKind::SetBlockProperties => {
            decode_set_block_properties(type_id, input, field, context)
        }
        WorldEffectKind::SpawnParticles => decode_spawn_particles(type_id, input, field, context),
    }
}

fn decode_explode(
    type_id: String,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let mut output = RegistryCodecRecord::default();
    push_optional_bool(
        &mut output,
        input,
        "attribute_to_user",
        false,
        field,
        context,
    )?;
    push_optional_fixed_holder(
        &mut output,
        input,
        "damage_type",
        field,
        context,
        DAMAGE_TYPE_REGISTRY,
    )?;
    push_optional_level(&mut output, input, "knockback_multiplier", field, context)?;
    push_optional_holder_set(
        &mut output,
        input,
        "immune_blocks",
        field,
        context,
        BLOCK_REGISTRY,
    )?;
    push_optional_vec3(&mut output, input, "offset", field, context)?;
    push_required_level(&mut output, input, "radius", field, context)?;
    push_optional_bool(&mut output, input, "create_fire", false, field, context)?;
    push_required_enum(
        &mut output,
        input,
        "block_interaction",
        &["none", "block", "mob", "tnt", "trigger"],
        field,
        context,
    )?;
    push_required_particle(&mut output, input, "small_particle", field, context)?;
    push_required_particle(&mut output, input, "large_particle", field, context)?;
    push_optional_weighted_particles(&mut output, input, "block_particles", field, context)?;
    push_required_sound(&mut output, input, "sound", field, context)?;
    Ok(output.finish_typed(type_id))
}

fn decode_replace_block(
    type_id: String,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let mut output = RegistryCodecRecord::default();
    push_optional_vec3i(&mut output, input, "offset", field, context)?;
    push_optional_block_predicate(&mut output, input, "predicate", field, context)?;
    push_required_block_state_provider(&mut output, input, "block_state", field, context)?;
    push_optional_fixed_holder(
        &mut output,
        input,
        "trigger_game_event",
        field,
        context,
        GAME_EVENT_REGISTRY,
    )?;
    Ok(output.finish_typed(type_id))
}

fn decode_replace_disk(
    type_id: String,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let mut output = RegistryCodecRecord::default();
    push_required_level(&mut output, input, "radius", field, context)?;
    push_required_level(&mut output, input, "height", field, context)?;
    push_optional_vec3i(&mut output, input, "offset", field, context)?;
    push_optional_block_predicate(&mut output, input, "predicate", field, context)?;
    push_required_block_state_provider(&mut output, input, "block_state", field, context)?;
    push_optional_fixed_holder(
        &mut output,
        input,
        "trigger_game_event",
        field,
        context,
        GAME_EVENT_REGISTRY,
    )?;
    Ok(output.finish_typed(type_id))
}

fn decode_set_block_properties(
    type_id: String,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let mut output = RegistryCodecRecord::default();
    let properties_field = child(field, "properties");
    let properties = required_field(input, "properties", field, context)?;
    let properties = context.with_nested(&properties_field, |context| {
        decode_string_map(properties, &properties_field, context)
    })?;
    output.push("properties", properties);
    push_optional_vec3i(&mut output, input, "offset", field, context)?;
    push_optional_fixed_holder(
        &mut output,
        input,
        "trigger_game_event",
        field,
        context,
        GAME_EVENT_REGISTRY,
    )?;
    Ok(output.finish_typed(type_id))
}

fn decode_spawn_particles(
    type_id: String,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let mut output = RegistryCodecRecord::default();
    push_required_particle(&mut output, input, "particle", field, context)?;
    push_position_source(&mut output, input, "horizontal_position", field, context)?;
    push_position_source(&mut output, input, "vertical_position", field, context)?;
    push_velocity_source(&mut output, input, "horizontal_velocity", field, context)?;
    push_velocity_source(&mut output, input, "vertical_velocity", field, context)?;
    push_optional_float_provider(&mut output, input, "speed", field, context)?;
    Ok(output.finish_typed(type_id))
}

fn push_optional_bool(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    default: bool,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let value_field = child(field, name);
    context.charge_node(&value_field)?;
    let value = value
        .as_bool()
        .ok_or_else(|| context.codec_error(&value_field, "expected Codec.BOOL input"))?;
    if value != default {
        output.push(
            name,
            CanonicalRegistryCodecValue::new(NbtSummaryValue::Byte(i8::from(value))),
        );
    }
    Ok(())
}

fn push_optional_fixed_holder(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    registry: &'static str,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let value_field = child(field, name);
    let value = context.with_nested(&value_field, |context| {
        decode_fixed_holder_reference_source(value, &value_field, context, registry)
    })?;
    output.push(name, value);
    Ok(())
}

fn push_optional_holder_set(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    registry: &'static str,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let value_field = child(field, name);
    let value = context.with_nested(&value_field, |context| {
        decode_homogeneous_fixed_holder_set_source(value, &value_field, context, registry)
    })?;
    output.push(name, value);
    Ok(())
}

fn push_required_level(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let value_field = child(field, name);
    let value = required_field(input, name, field, context)?;
    let value = context.with_nested(&value_field, |context| {
        decode_level_based_value_source(value, &value_field, context)
    })?;
    output.push(name, value);
    Ok(())
}

fn push_optional_level(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let value_field = child(field, name);
    let value = context.with_nested(&value_field, |context| {
        decode_level_based_value_source(value, &value_field, context)
    })?;
    output.push(name, value);
    Ok(())
}

fn push_optional_vec3(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let value_field = child(field, name);
    let value = context.with_nested(&value_field, |context| {
        decode_vec3_source(value, &value_field, context)
    })?;
    if value
        .values
        .iter()
        .any(|component| component.total_cmp(&0.0).is_ne())
    {
        output.push(name, value.canonical);
    }
    Ok(())
}

fn push_optional_vec3i(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let value_field = child(field, name);
    let value = context.with_nested(&value_field, |context| {
        decode_vec3i(value, &value_field, context, false)
    })?;
    if value.network_nbt_unchecked() != &NbtSummaryValue::IntArray(vec![0, 0, 0]) {
        output.push(name, value);
    }
    Ok(())
}

fn push_required_enum(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    allowed: &[&str],
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let value_field = child(field, name);
    context.charge_node(&value_field)?;
    let raw = required_field(input, name, field, context)?
        .java_string_ref()
        .ok_or_else(|| context.codec_error(&value_field, "expected enum string"))?;
    if !allowed.iter().any(|name| raw == *name) {
        return Err(context.codec_error(&value_field, format!("unknown enum value `{raw}`")));
    }
    output.push(
        name,
        CanonicalRegistryCodecValue::new(NbtSummaryValue::String(raw.clone())),
    );
    Ok(())
}

fn push_required_particle(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let value_field = child(field, name);
    let value = required_field(input, name, field, context)?;
    let value = context.with_nested(&value_field, |context| {
        decode_particle_options_source(value, &value_field, context)
    })?;
    output.push(name, value);
    Ok(())
}

fn push_required_sound(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let value_field = child(field, name);
    let value = required_field(input, name, field, context)?;
    let value = context.with_nested(&value_field, |context| {
        decode_sound_event_holder_source(value, &value_field, context)
    })?;
    output.push(name, value);
    Ok(())
}

fn push_optional_block_predicate(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let value_field = child(field, name);
    let value = context.with_nested(&value_field, |context| {
        decode_block_predicate_source(value, &value_field, context)
    })?;
    output.push(name, value);
    Ok(())
}

fn push_required_block_state_provider(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let value_field = child(field, name);
    let value = required_field(input, name, field, context)?;
    let value = context.with_nested(&value_field, |context| {
        decode_block_state_provider_source(value, &value_field, context)
    })?;
    output.push(name, value);
    Ok(())
}

fn decode_string_map(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let len = context.charge_record(input, field, "block properties")?;
    let visible = input.visible_map_entries(len);
    let mut values: Vec<NbtSummaryEntry> = Vec::with_capacity(visible.len());
    for (name, value) in visible {
        let value_field = child(field, &name.to_display_string());
        context.charge_node(&value_field)?;
        let value = value
            .java_string()
            .ok_or_else(|| context.codec_error(&value_field, "property value is not a string"))?;
        values.push(NbtSummaryEntry {
            name: name.clone(),
            value: value.to_nbt(),
        });
    }
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::Compound(
        values,
    )))
}

fn push_optional_weighted_particles(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let value_field = child(field, name);
    let value = context.with_nested(&value_field, |context| {
        decode_weighted_particles(value, &value_field, context)
    })?;
    if value.network_nbt_unchecked() != &NbtSummaryValue::List(Vec::new()) {
        output.push(name, value);
    }
    Ok(())
}

fn decode_weighted_particles(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let len = input
        .list_len()
        .ok_or_else(|| context.codec_error(field, "weighted particle value is not a list"))?;
    context.check_collection(field, len)?;
    let mut entries = Vec::with_capacity(len);
    let mut references = Vec::new();
    let mut total_weight = 0_i64;
    for index in 0..len {
        let item_field = format!("{field}[{index}]");
        let item = input
            .list_element(index)
            .expect("weighted-particle list length and indexed values stay consistent");
        let decoded = context.with_nested(&item_field, |context| {
            decode_weighted_particle(item, &item_field, context)
        })?;
        total_weight += i64::from(decoded.weight);
        if total_weight > i64::from(i32::MAX) {
            return Err(context.codec_error(field, "sum of weights exceeds 2147483647"));
        }
        let (value, nested_references) = decoded.value.into_parts();
        entries.push(value);
        references.extend(nested_references);
    }
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::List(entries),
        references,
    ))
}

struct WeightedParticle {
    value: CanonicalRegistryCodecValue,
    weight: i32,
}

fn decode_weighted_particle(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<WeightedParticle, RegistryCodecError> {
    context.charge_record(input, field, "weighted particle entry")?;
    let mut output = RegistryCodecRecord::default();
    push_required_particle(&mut output, input, "particle", field, context)?;
    push_optional_float(&mut output, input, "scaling", 1.0, field, context)?;
    push_optional_float(&mut output, input, "speed", 1.0, field, context)?;
    let weight_field = child(field, "weight");
    context.charge_node(&weight_field)?;
    let weight = required_field(input, "weight", field, context)?
        .codec_i32()
        .ok_or_else(|| context.codec_error(&weight_field, "expected Codec.INT weight"))?;
    if weight < 0 {
        return Err(context.codec_error(&weight_field, "weight must be non-negative"));
    }
    output.push(
        "weight",
        CanonicalRegistryCodecValue::new(NbtSummaryValue::Int(weight)),
    );
    Ok(WeightedParticle {
        value: output.finish(),
        weight,
    })
}

fn push_optional_float(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    default: f32,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let value_field = child(field, name);
    context.charge_node(&value_field)?;
    let value = value
        .codec_f32()
        .ok_or_else(|| context.codec_error(&value_field, "expected Codec.FLOAT input"))?;
    if value.to_bits() != default.to_bits() {
        output.push(
            name,
            CanonicalRegistryCodecValue::new(NbtSummaryValue::Float(canonical_float_tag_bits(
                value,
            ))),
        );
    }
    Ok(())
}

fn push_position_source(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let value_field = child(field, name);
    let value = required_field(input, name, field, context)?;
    let value = context.with_nested(&value_field, |context| {
        decode_position_source(value, &value_field, context)
    })?;
    output.push(name, value);
    Ok(())
}

fn decode_position_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "particle position source")?;
    let mut output = RegistryCodecRecord::default();
    let type_field = child(field, "type");
    context.charge_node(&type_field)?;
    let position_type = required_field(input, "type", field, context)?
        .java_string_ref()
        .ok_or_else(|| context.codec_error(&type_field, "position type is not a string"))?;
    if !(position_type == "entity_position" || position_type == "in_bounding_box") {
        return Err(context.codec_error(
            &type_field,
            format!("unknown position source type `{position_type}`"),
        ));
    }
    output.push(
        "type",
        CanonicalRegistryCodecValue::new(NbtSummaryValue::String(position_type.clone())),
    );
    push_optional_float(&mut output, input, "offset", 0.0, field, context)?;
    let scale = decode_optional_positive_float(input, "scale", 1.0, field, context)?;
    if scale.to_bits() != 1.0_f32.to_bits() {
        output.push(
            "scale",
            CanonicalRegistryCodecValue::new(NbtSummaryValue::Float(canonical_float_tag_bits(
                scale,
            ))),
        );
    }
    if position_type == "entity_position" && scale != 1.0 {
        return Err(context.codec_error(field, "cannot scale an entity position coordinate source"));
    }
    Ok(output.finish())
}

fn decode_optional_positive_float(
    input: RegistryElementValueRef<'_>,
    name: &str,
    default: f32,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<f32, RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(default);
    };
    let value_field = child(field, name);
    context.charge_node(&value_field)?;
    let value = value
        .codec_f32()
        .ok_or_else(|| context.codec_error(&value_field, "expected Codec.FLOAT input"))?;
    if !java_float_compare(value, 0.0).is_gt() || java_float_compare(value, f32::MAX).is_gt() {
        return Err(context.codec_error(&value_field, "expected a positive finite float"));
    }
    Ok(value)
}

fn push_velocity_source(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let value_field = child(field, name);
    let value = required_field(input, name, field, context)?;
    let value = context.with_nested(&value_field, |context| {
        decode_velocity_source(value, &value_field, context)
    })?;
    output.push(name, value);
    Ok(())
}

fn decode_velocity_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "particle velocity source")?;
    let mut output = RegistryCodecRecord::default();
    push_optional_float(&mut output, input, "movement_scale", 0.0, field, context)?;
    if let Some(value) = input.field("base") {
        let value_field = child(field, "base");
        let value = context.with_nested(&value_field, |context| {
            decode_float_provider_source(
                value,
                &value_field,
                context,
                f32::NEG_INFINITY,
                f32::INFINITY,
            )
        })?;
        if value.network_nbt_unchecked() != &NbtSummaryValue::Float(canonical_float_tag_bits(0.0)) {
            output.push("base", value);
        }
    }
    Ok(output.finish())
}

fn push_optional_float_provider(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let value_field = child(field, name);
    let value = context.with_nested(&value_field, |context| {
        decode_float_provider_source(
            value,
            &value_field,
            context,
            f32::NEG_INFINITY,
            f32::INFINITY,
        )
    })?;
    if value.network_nbt_unchecked() != &NbtSummaryValue::Float(canonical_float_tag_bits(0.0)) {
        output.push(name, value);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
