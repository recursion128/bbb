//! Exact `EnchantmentEffectComponents.CODEC` dispatched-map owner.
//!
//! Vanilla registers 31 persistent component types. Twenty-four are validated
//! conditional/targeted lists, while the remaining seven are direct value,
//! attribute, sound, or unit codecs. The map keeps accepted source encounter
//! order and each payload keeps its owning record-codec order

use std::collections::HashSet;

use crate::{
    enchantment_codec_types::{
        enchantment_effect_component_codec_type, EnchantmentEffectComponentCodecFamily,
        EnchantmentEffectComponentCodecType,
    },
    loot_context_types::{vanilla_loot_context_param_set, VanillaLootContextParamSet},
    persistent_component_nbt::source_codec::decode_sound_event_holder_source,
    registry_codec::{
        recursive_values::{
            decode_list, required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
            RegistryCodecRecord,
        },
        RegistryCodecError, RegistryElementValueRef,
    },
};
use bbb_protocol::{nbt_summary::entry, normalize_resource_location, NbtSummaryValue};

use crate::registry_codec::enchantment_source::{
    entity_effect::{
        decode_enchantment_entity_effect_source, decode_enchantment_location_based_effect_source,
        leaf::decode_enchantment_attribute_effect_source,
    },
    loot_condition_source::decode_loot_condition_source,
    value::{child, decode_enchantment_value_effect_source},
};

/// Decodes the generated 31-entry enchantment effect-component map.
pub(super) fn decode_enchantment_effect_components_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let len = context.charge_record(input, field, "enchantment effects")?;

    // NbtOps exposes a CompoundTag map, whose identical raw keys are already
    // last-wins. Keep the first encounter position while replacing its value,
    // then apply the dispatched-map normalized-key duplicate rule below.
    let visible = input.visible_map_entries(len);

    let mut normalized = HashSet::with_capacity(visible.len());
    let mut fields = Vec::with_capacity(visible.len());
    let mut references = Vec::new();
    for (raw_id, value) in visible {
        let value_field = format!("{field}.{raw_id}");
        let resource_id = normalize_resource_location(raw_id.to_string()).map_err(|_| {
            context.codec_error(&value_field, "invalid effect component identifier")
        })?;
        if !normalized.insert(resource_id.clone()) {
            return Err(context.codec_error(
                &value_field,
                format!("duplicate normalized enchantment effect component {resource_id}"),
            ));
        }
        let descriptor =
            enchantment_effect_component_codec_type(&resource_id).ok_or_else(|| {
                context.codec_error(
                    &value_field,
                    format!("unknown enchantment effect component {resource_id}"),
                )
            })?;
        let decoded = context.with_nested(&value_field, |context| {
            decode_effect_component_value(descriptor, value, &value_field, context)
        })?;
        let (value, nested_references) = decoded.into_parts();
        fields.push(entry(descriptor.resource_id, value));
        references.extend(nested_references);
    }

    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(fields),
        references,
    ))
}

fn decode_effect_component_value(
    descriptor: &EnchantmentEffectComponentCodecType,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    use EnchantmentEffectComponentCodecFamily::*;

    match descriptor.codec_family {
        ConditionalValueEffect => decode_validated_wrapper_list(
            descriptor,
            WrapperKind::Conditional,
            EffectKind::Value,
            input,
            field,
            context,
        ),
        ConditionalDamageImmunity => decode_validated_wrapper_list(
            descriptor,
            WrapperKind::Conditional,
            EffectKind::DamageImmunity,
            input,
            field,
            context,
        ),
        TargetedEntityEffect => decode_validated_wrapper_list(
            descriptor,
            WrapperKind::Targeted,
            EffectKind::Entity,
            input,
            field,
            context,
        ),
        ConditionalEntityEffect => decode_validated_wrapper_list(
            descriptor,
            WrapperKind::Conditional,
            EffectKind::Entity,
            input,
            field,
            context,
        ),
        EquipmentDropsValueEffect => decode_validated_wrapper_list(
            descriptor,
            WrapperKind::EquipmentDrops,
            EffectKind::Value,
            input,
            field,
            context,
        ),
        ConditionalLocationBasedEffect => decode_validated_wrapper_list(
            descriptor,
            WrapperKind::Conditional,
            EffectKind::Location,
            input,
            field,
            context,
        ),
        AttributeEffects => decode_list(
            input,
            field,
            context,
            decode_enchantment_attribute_effect_source,
        ),
        ValueEffect => decode_enchantment_value_effect_source(input, field, context),
        CrossbowChargingSounds => {
            decode_list(input, field, context, decode_crossbow_charging_sound)
        }
        SoundEvents => decode_list(input, field, context, decode_sound_event_holder_source),
        Unit => decode_unit(input, field, context),
    }
}

#[derive(Debug, Clone, Copy)]
enum WrapperKind {
    Conditional,
    Targeted,
    EquipmentDrops,
}

#[derive(Debug, Clone, Copy)]
enum EffectKind {
    Value,
    DamageImmunity,
    Entity,
    Location,
}

fn decode_validated_wrapper_list(
    descriptor: &EnchantmentEffectComponentCodecType,
    wrapper: WrapperKind,
    effect: EffectKind,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let validation_context = descriptor.validation_context.ok_or_else(|| {
        context.codec_error(
            field,
            format!(
                "generated effect component {} has no validation context",
                descriptor.resource_id
            ),
        )
    })?;
    let param_set_id = validation_context.loot_context_param_set_resource_id();
    let param_set = vanilla_loot_context_param_set(param_set_id).ok_or_else(|| {
        context.codec_error(
            field,
            format!("unknown generated loot context parameter set {param_set_id}"),
        )
    })?;

    context.charge_node(field)?;
    let len = input
        .list_len()
        .ok_or_else(|| context.codec_error(field, "expected validated enchantment effect list"))?;
    context.check_collection(field, len)?;
    let mut values = Vec::with_capacity(len);
    let mut references = Vec::new();
    for index in 0..len {
        let item_field = format!("{field}[{index}]");
        let item = input
            .list_element(index)
            .expect("validated effect list length and entries stay consistent");
        let decoded = context.with_nested(&item_field, |context| {
            decode_effect_wrapper(wrapper, effect, param_set, item, &item_field, context)
        })?;
        let (value, nested_references) = decoded.into_parts();
        values.push(value);
        references.extend(nested_references);
    }
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::List(values),
        references,
    ))
}

fn decode_effect_wrapper(
    wrapper: WrapperKind,
    effect: EffectKind,
    param_set: &VanillaLootContextParamSet,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "conditional enchantment effect")?;
    let mut output = RegistryCodecRecord::default();
    match wrapper {
        WrapperKind::Conditional => {}
        WrapperKind::Targeted => {
            push_target(&mut output, input, "enchanted", field, context, false)?;
            push_target(&mut output, input, "affected", field, context, false)?;
        }
        WrapperKind::EquipmentDrops => {
            push_target(&mut output, input, "enchanted", field, context, true)?;
        }
    }
    let effect_field = child(field, "effect");
    let effect_input = required_field(input, "effect", field, context)?;
    let effect_value = context.with_nested(&effect_field, |context| {
        decode_effect_value(effect, effect_input, &effect_field, context)
    })?;
    output.push("effect", effect_value);

    if let Some(requirements) = input.field("requirements") {
        let requirements_field = child(field, "requirements");
        let decoded = context.with_nested(&requirements_field, |context| {
            decode_loot_condition_source(requirements, &requirements_field, context)
        })?;
        decoded.validate_without_references(param_set, &requirements_field, context)?;
        output.push("requirements", decoded.value.canonical);
    }
    Ok(output.finish())
}

fn decode_effect_value(
    kind: EffectKind,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    match kind {
        EffectKind::Value => decode_enchantment_value_effect_source(input, field, context),
        EffectKind::DamageImmunity => decode_unit(input, field, context),
        EffectKind::Entity => decode_enchantment_entity_effect_source(input, field, context),
        EffectKind::Location => {
            decode_enchantment_location_based_effect_source(input, field, context)
        }
    }
}

fn push_target(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    non_damage: bool,
) -> Result<(), RegistryCodecError> {
    let target_field = child(field, name);
    context.charge_node(&target_field)?;
    let target = required_field(input, name, field, context)?
        .java_string_ref()
        .filter(|target| {
            *target == "attacker"
                || *target == "victim"
                || (!non_damage && *target == "damaging_entity")
        })
        .ok_or_else(|| {
            context.codec_error(
                &target_field,
                if non_damage {
                    "unknown non-damage enchantment target"
                } else {
                    "unknown enchantment target"
                },
            )
        })?;
    output.push(
        name,
        CanonicalRegistryCodecValue::new(NbtSummaryValue::String(target.clone())),
    );
    Ok(())
}

fn decode_crossbow_charging_sound(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "crossbow charging sound")?;
    let mut output = RegistryCodecRecord::default();
    for name in ["start", "mid", "end"] {
        let Some(value) = input.field(name) else {
            continue;
        };
        let value_field = child(field, name);
        let decoded = context.with_nested(&value_field, |context| {
            decode_sound_event_holder_source(value, &value_field, context)
        })?;
        output.push(name, decoded);
    }
    Ok(output.finish())
}

fn decode_unit(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "unit effect")?;
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::Compound(
        Vec::new(),
    )))
}

#[cfg(test)]
#[path = "effect_components/tests.rs"]
mod tests;
