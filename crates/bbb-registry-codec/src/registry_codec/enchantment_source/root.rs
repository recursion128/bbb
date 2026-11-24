//! Exact source-aware owner for `Enchantment.DIRECT_CODEC`.
//!
//! The root record flattens `EnchantmentDefinition` between its description
//! and optional effect fields. Fixed-registry holder sets remain unresolved
//! placeholders until the synchronized registry generation freezes; the
//! canonical writer preserves Vanilla's record-codec order and default
//! omission.

use crate::component_source_codec::decode_component_source;
use crate::{
    persistent_component_nbt::source_codec::decode_homogeneous_fixed_holder_set_source,
    registry_codec::{
        recursive_values::{
            required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
            RegistryCodecRecord,
        },
        RegistryCodecError, RegistryElementData, RegistryElementValueRef,
    },
};
use bbb_facts::equipment_slots::vanilla_equipment_slot_group_for_serialized_name;
use bbb_protocol::{nbt_summary::entry, NbtSummaryValue};

use super::effect_components::decode_enchantment_effect_components_source;

const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";
const ITEM_REGISTRY: &str = "minecraft:item";
const ENCHANTMENT_CODEC: &str = "Enchantment.DIRECT_CODEC";

const MIN_WEIGHT: i32 = 1;
const MAX_WEIGHT: i32 = 1024;
const MIN_LEVEL: i32 = 1;
const MAX_LEVEL: i32 = 255;

/// Decodes one synchronized enchantment element and writes the canonical NBT
/// shape consumed by the configuration registry packet.
pub(in crate::registry_codec) fn decode_enchantment_element_with_content_registry(
    data: &RegistryElementData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let mut context =
        RegistryCodecDecodeContext::new(ENCHANTMENT_REGISTRY, ENCHANTMENT_CODEC, data.source)
            .with_content_registry(content_registry);
    decode_enchantment_source(data.root(), "<root>", &mut context)
}

#[cfg(test)]
pub(in crate::registry_codec) fn decode_enchantment_element(
    data: &RegistryElementData,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_enchantment_element_with_content_registry(
        data,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

fn decode_enchantment_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "Enchantment record")?;
    let mut output = RegistryCodecRecord::default();

    let description_input = required_field(input, "description", field, context)?;
    let description = context.with_nested("description", |context| {
        decode_component_source(description_input, "description", context)
    })?;
    output.push("description", description);

    output.push(
        "supported_items",
        decode_holder_set_field(input, "supported_items", field, context, ITEM_REGISTRY)?,
    );

    if let Some(primary_items) = input.field("primary_items") {
        let primary_items = context.with_nested("primary_items", |context| {
            decode_homogeneous_fixed_holder_set_source(
                primary_items,
                "primary_items",
                context,
                ITEM_REGISTRY,
            )
        })?;
        output.push("primary_items", primary_items);
    }

    output.push_value(
        "weight",
        NbtSummaryValue::Int(context.decode_i32_range(
            required_field(input, "weight", field, context)?,
            "weight",
            MIN_WEIGHT,
            MAX_WEIGHT,
        )?),
    );
    output.push_value(
        "max_level",
        NbtSummaryValue::Int(context.decode_i32_range(
            required_field(input, "max_level", field, context)?,
            "max_level",
            MIN_LEVEL,
            MAX_LEVEL,
        )?),
    );

    for name in ["min_cost", "max_cost"] {
        let cost_input = required_field(input, name, field, context)?;
        let cost = context.with_nested(name, |context| decode_cost(cost_input, name, context))?;
        output.push(name, cost);
    }

    output.push_value(
        "anvil_cost",
        NbtSummaryValue::Int(context.decode_i32_range(
            required_field(input, "anvil_cost", field, context)?,
            "anvil_cost",
            0,
            i32::MAX,
        )?),
    );

    let slots_input = required_field(input, "slots", field, context)?;
    let slots = context.with_nested("slots", |context| {
        decode_equipment_slot_groups(slots_input, "slots", context)
    })?;
    output.push("slots", slots);

    if let Some(exclusive_set) = input.field("exclusive_set") {
        let exclusive_set = context.with_nested("exclusive_set", |context| {
            decode_homogeneous_fixed_holder_set_source(
                exclusive_set,
                "exclusive_set",
                context,
                ENCHANTMENT_REGISTRY,
            )
        })?;
        if !is_empty_direct_holder_set(exclusive_set.network_nbt_unchecked()) {
            output.push("exclusive_set", exclusive_set);
        } else {
            output.references.extend(exclusive_set.into_parts().1);
        }
    }

    if let Some(effects) = input.field("effects") {
        let effects = context.with_nested("effects", |context| {
            decode_enchantment_effect_components_source(effects, "effects", context)
        })?;
        if !is_empty_component_map(effects.network_nbt_unchecked()) {
            output.push("effects", effects);
        } else {
            output.references.extend(effects.into_parts().1);
        }
    }

    Ok(output.finish())
}

fn decode_holder_set_field(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    target_registry: &'static str,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let holder_input = required_field(input, name, field, context)?;
    context.with_nested(name, |context| {
        decode_homogeneous_fixed_holder_set_source(holder_input, name, context, target_registry)
    })
}

fn decode_cost(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "Enchantment.Cost record")?;
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::Compound(
        vec![
            entry(
                "base",
                NbtSummaryValue::Int(
                    context.decode_i32(required_field(input, "base", field, context)?, "base")?,
                ),
            ),
            entry(
                "per_level_above_first",
                NbtSummaryValue::Int(context.decode_i32(
                    required_field(input, "per_level_above_first", field, context)?,
                    "per_level_above_first",
                )?),
            ),
        ],
    )))
}

fn decode_equipment_slot_groups(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let len = input
        .list_len()
        .ok_or_else(|| context.codec_error(field, "equipment slots are not a list"))?;
    context.check_collection(field, len)?;
    let mut slots = Vec::with_capacity(len);
    for index in 0..len {
        let slot_input = input
            .list_element(index)
            .expect("slot-list length and indexed values stay consistent");
        let slot = context.with_nested(field, |context| {
            context.charge_node(field)?;
            let raw = slot_input.java_string_ref().ok_or_else(|| {
                context.codec_error(field, "equipment slot group is not a string")
            })?;
            raw.with_utf8(vanilla_equipment_slot_group_for_serialized_name)
                .flatten()
                .ok_or_else(|| {
                    context.codec_error(field, format!("unknown equipment slot group `{raw}`"))
                })?;
            Ok::<_, RegistryCodecError>(NbtSummaryValue::String(raw.into()))
        })?;
        slots.push(slot);
    }
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::List(
        slots,
    )))
}

fn is_empty_direct_holder_set(value: &NbtSummaryValue) -> bool {
    matches!(value, NbtSummaryValue::List(values) if values.is_empty())
}

fn is_empty_component_map(value: &NbtSummaryValue) -> bool {
    matches!(value, NbtSummaryValue::Compound(fields) if fields.is_empty())
}

#[cfg(test)]
mod tests;
