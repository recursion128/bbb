//! The fifteen concrete `DataComponentPredicate` payload codecs.
//!
//! Dispatch keys and payload codecs are registered in
//! `DataComponentPredicates`; the outer dispatched-map ordering is
//! owned by the parent module.

use crate::component_source_codec::decode_component_source;
use crate::registry_codec::{
    recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    source_nbt::decode_lenient_flattened_compound_source,
    RegistryCodecError, RegistryElementValueRef,
};
use bbb_facts::{
    attribute_modifier_types::attribute_modifier_operation_id,
    data_component_types::{
        VANILLA_DATA_COMPONENT_PREDICATE_ARMOR_TRIM_TYPE_ID,
        VANILLA_DATA_COMPONENT_PREDICATE_ATTRIBUTE_MODIFIERS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PREDICATE_BUNDLE_CONTENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PREDICATE_CONTAINER_TYPE_ID,
        VANILLA_DATA_COMPONENT_PREDICATE_CUSTOM_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_PREDICATE_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_PREDICATE_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PREDICATE_FIREWORKS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PREDICATE_FIREWORK_EXPLOSION_TYPE_ID,
        VANILLA_DATA_COMPONENT_PREDICATE_JUKEBOX_PLAYABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_PREDICATE_POTIONS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PREDICATE_STORED_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PREDICATE_VILLAGER_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_PREDICATE_WRITABLE_BOOK_TYPE_ID,
        VANILLA_DATA_COMPONENT_PREDICATE_WRITTEN_BOOK_TYPE_ID,
    },
    equipment_slots::vanilla_equipment_slot_group_for_serialized_name,
};

use super::super::holders::decode_homogeneous_fixed_holder_set;
use super::{
    child, decode_bool_value, decode_collection_predicate, decode_double_bounds,
    decode_identifier_value, decode_int_bounds, decode_item_predicate, decode_list,
    decode_named_value, decode_string, RegistryCodecRecord,
};

const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";
const POTION_REGISTRY: &str = "minecraft:potion";
const ATTRIBUTE_REGISTRY: &str = "minecraft:attribute";
const TRIM_MATERIAL_REGISTRY: &str = "minecraft:trim_material";
const TRIM_PATTERN_REGISTRY: &str = "minecraft:trim_pattern";
const JUKEBOX_SONG_REGISTRY: &str = "minecraft:jukebox_song";
const VILLAGER_TYPE_REGISTRY: &str = "minecraft:villager_type";

pub(super) fn decode_concrete_predicate(
    predicate_type_id: i32,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let value = match predicate_type_id {
        VANILLA_DATA_COMPONENT_PREDICATE_DAMAGE_TYPE_ID => {
            decode_damage_predicate(input, field, context)
        }
        VANILLA_DATA_COMPONENT_PREDICATE_ENCHANTMENTS_TYPE_ID
        | VANILLA_DATA_COMPONENT_PREDICATE_STORED_ENCHANTMENTS_TYPE_ID => {
            decode_enchantments_predicate(input, field, context)
        }
        VANILLA_DATA_COMPONENT_PREDICATE_POTIONS_TYPE_ID => {
            decode_homogeneous_fixed_holder_set(input, field, context, POTION_REGISTRY, false)
        }
        VANILLA_DATA_COMPONENT_PREDICATE_CUSTOM_DATA_TYPE_ID => {
            decode_lenient_flattened_compound_source(input, field, context)
        }
        VANILLA_DATA_COMPONENT_PREDICATE_CONTAINER_TYPE_ID
        | VANILLA_DATA_COMPONENT_PREDICATE_BUNDLE_CONTENTS_TYPE_ID => {
            decode_item_collection_owner(input, field, context)
        }
        VANILLA_DATA_COMPONENT_PREDICATE_FIREWORK_EXPLOSION_TYPE_ID => {
            decode_firework_explosion_predicate(input, field, context)
        }
        VANILLA_DATA_COMPONENT_PREDICATE_FIREWORKS_TYPE_ID => {
            decode_fireworks_predicate(input, field, context)
        }
        VANILLA_DATA_COMPONENT_PREDICATE_WRITABLE_BOOK_TYPE_ID => {
            decode_writable_book_predicate(input, field, context)
        }
        VANILLA_DATA_COMPONENT_PREDICATE_WRITTEN_BOOK_TYPE_ID => {
            decode_written_book_predicate(input, field, context)
        }
        VANILLA_DATA_COMPONENT_PREDICATE_ATTRIBUTE_MODIFIERS_TYPE_ID => {
            decode_attribute_modifiers_predicate(input, field, context)
        }
        VANILLA_DATA_COMPONENT_PREDICATE_ARMOR_TRIM_TYPE_ID => {
            decode_trim_predicate(input, field, context)
        }
        VANILLA_DATA_COMPONENT_PREDICATE_JUKEBOX_PLAYABLE_TYPE_ID => {
            decode_jukebox_predicate(input, field, context)
        }
        VANILLA_DATA_COMPONENT_PREDICATE_VILLAGER_VARIANT_TYPE_ID => {
            decode_homogeneous_fixed_holder_set(
                input,
                field,
                context,
                VILLAGER_TYPE_REGISTRY,
                false,
            )
        }
        _ => unreachable!("the generated predicate-type catalog contains exactly fifteen rows"),
    }?;
    Ok(value)
}

fn decode_damage_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "damage predicate")?;
    let mut output = RegistryCodecRecord::default();
    push_default_int_bounds(input, "durability", field, context, &mut output)?;
    push_default_int_bounds(input, "damage", field, context, &mut output)?;
    Ok(output.finish())
}

fn decode_enchantments_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_list(input, field, context, |input, item_field, context| {
        context.charge_record(input, item_field, "enchantment predicate")?;
        let mut output = RegistryCodecRecord::default();
        if let Some(enchantments) = input.field("enchantments") {
            let child = child(item_field, "enchantments");
            let decoded = context.with_nested(&child, |context| {
                decode_homogeneous_fixed_holder_set(
                    enchantments,
                    &child,
                    context,
                    ENCHANTMENT_REGISTRY,
                    false,
                )
            })?;
            output.push("enchantments", decoded);
        }
        push_default_int_bounds(input, "levels", item_field, context, &mut output)?;
        Ok(output.finish())
    })
}

fn decode_item_collection_owner(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "item collection predicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(items) = input.field("items") {
        let items_field = child(field, "items");
        let decoded = context.with_nested(&items_field, |context| {
            decode_collection_predicate(items, &items_field, context, decode_item_predicate)
        })?;
        output.push("items", decoded);
    }
    Ok(output.finish())
}

fn decode_firework_explosion_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "firework explosion predicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(shape) = input.field("shape") {
        let shape_field = child(field, "shape");
        let decoded = context.with_nested(&shape_field, |context| {
            decode_named_value(shape, &shape_field, context, "firework shape", |shape| {
                matches!(
                    shape,
                    "small_ball" | "large_ball" | "star" | "creeper" | "burst"
                )
            })
        })?;
        output.push("shape", decoded);
    }
    push_optional_bool(input, "has_twinkle", field, context, &mut output)?;
    push_optional_bool(input, "has_trail", field, context, &mut output)?;
    Ok(output.finish())
}

fn decode_fireworks_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "fireworks predicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(explosions) = input.field("explosions") {
        let explosions_field = child(field, "explosions");
        let decoded = context.with_nested(&explosions_field, |context| {
            decode_collection_predicate(
                explosions,
                &explosions_field,
                context,
                decode_firework_explosion_predicate,
            )
        })?;
        output.push("explosions", decoded);
    }
    push_default_int_bounds(input, "flight_duration", field, context, &mut output)?;
    Ok(output.finish())
}

fn decode_writable_book_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "writable book predicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(pages) = input.field("pages") {
        let pages_field = child(field, "pages");
        let decoded = context.with_nested(&pages_field, |context| {
            decode_collection_predicate(
                pages,
                &pages_field,
                context,
                |page, page_field, context| {
                    decode_string(page, page_field, context, "writable book page")
                },
            )
        })?;
        output.push("pages", decoded);
    }
    Ok(output.finish())
}

fn decode_written_book_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "written book predicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(pages) = input.field("pages") {
        let pages_field = child(field, "pages");
        let decoded = context.with_nested(&pages_field, |context| {
            decode_collection_predicate(pages, &pages_field, context, decode_component_source)
        })?;
        output.push("pages", decoded);
    }
    push_optional_string(input, "author", "author", field, context, &mut output)?;
    push_optional_string(input, "title", "title", field, context, &mut output)?;
    push_default_int_bounds(input, "generation", field, context, &mut output)?;
    push_optional_bool(input, "resolved", field, context, &mut output)?;
    Ok(output.finish())
}

fn decode_attribute_modifiers_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "attribute modifiers predicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(modifiers) = input.field("modifiers") {
        let modifiers_field = child(field, "modifiers");
        let decoded = context.with_nested(&modifiers_field, |context| {
            decode_collection_predicate(
                modifiers,
                &modifiers_field,
                context,
                decode_attribute_modifier_entry_predicate,
            )
        })?;
        output.push("modifiers", decoded);
    }
    Ok(output.finish())
}

fn decode_attribute_modifier_entry_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "attribute modifier entry predicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(attribute) = input.field("attribute") {
        let attribute_field = child(field, "attribute");
        let decoded = context.with_nested(&attribute_field, |context| {
            decode_homogeneous_fixed_holder_set(
                attribute,
                &attribute_field,
                context,
                ATTRIBUTE_REGISTRY,
                false,
            )
        })?;
        output.push("attribute", decoded);
    }
    if let Some(id) = input.field("id") {
        let id_field = child(field, "id");
        let decoded = context.with_nested(&id_field, |context| {
            decode_identifier_value(id, &id_field, context)
        })?;
        output.push("id", decoded);
    }
    if let Some(amount) = input.field("amount") {
        let amount_field = child(field, "amount");
        let decoded = context.with_nested(&amount_field, |context| {
            decode_double_bounds(amount, &amount_field, context)
        })?;
        if !decoded.is_any {
            output.push("amount", decoded.value);
        }
    }
    if let Some(operation) = input.field("operation") {
        let operation_field = child(field, "operation");
        let decoded = context.with_nested(&operation_field, |context| {
            decode_named_value(
                operation,
                &operation_field,
                context,
                "attribute modifier operation",
                |name| attribute_modifier_operation_id(name).is_some(),
            )
        })?;
        output.push("operation", decoded);
    }
    if let Some(slot) = input.field("slot") {
        let slot_field = child(field, "slot");
        let decoded = context.with_nested(&slot_field, |context| {
            decode_named_value(slot, &slot_field, context, "equipment slot group", |name| {
                vanilla_equipment_slot_group_for_serialized_name(name).is_some()
            })
        })?;
        output.push("slot", decoded);
    }
    Ok(output.finish())
}

fn decode_trim_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "armor trim predicate")?;
    let mut output = RegistryCodecRecord::default();
    push_optional_holder_set(
        input,
        "material",
        field,
        context,
        TRIM_MATERIAL_REGISTRY,
        &mut output,
    )?;
    push_optional_holder_set(
        input,
        "pattern",
        field,
        context,
        TRIM_PATTERN_REGISTRY,
        &mut output,
    )?;
    Ok(output.finish())
}

fn decode_jukebox_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "jukebox playable predicate")?;
    let mut output = RegistryCodecRecord::default();
    push_optional_holder_set(
        input,
        "song",
        field,
        context,
        JUKEBOX_SONG_REGISTRY,
        &mut output,
    )?;
    Ok(output.finish())
}

fn push_default_int_bounds(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    output: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let value_field = child(field, name);
    let decoded = context.with_nested(&value_field, |context| {
        decode_int_bounds(value, &value_field, context)
    })?;
    if !decoded.is_any {
        output.push(name, decoded.value);
    }
    Ok(())
}

fn push_optional_bool(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    output: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let value_field = child(field, name);
    let decoded = context.with_nested(&value_field, |context| {
        decode_bool_value(value, &value_field, context)
    })?;
    output.push(name, decoded);
    Ok(())
}

fn push_optional_string(
    input: RegistryElementValueRef<'_>,
    name: &str,
    expected: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    output: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let value_field = child(field, name);
    let decoded = context.with_nested(&value_field, |context| {
        decode_string(value, &value_field, context, expected)
    })?;
    output.push(name, decoded);
    Ok(())
}

fn push_optional_holder_set(
    input: RegistryElementValueRef<'_>,
    name: &'static str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    registry: &'static str,
    output: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let value_field = child(field, name);
    let decoded = context.with_nested(&value_field, |context| {
        decode_homogeneous_fixed_holder_set(value, &value_field, context, registry, false)
    })?;
    output.push(name, decoded);
    Ok(())
}
