//! `minecraft:attribute_modifiers` in both directions, transcribed from
//! `ItemAttributeModifiers.CODEC` over
//! `ItemAttributeModifiers.Entry.CODEC`.

use bbb_facts::attribute_modifier_types::{
    attribute_modifier_display_id, attribute_modifier_display_name,
    attribute_modifier_operation_id, attribute_modifier_operation_name,
    VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID, VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
    VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
};
use bbb_facts::attribute_types::{vanilla_attribute_type, vanilla_attribute_type_for_resource_id};
use bbb_facts::equipment_slots::{
    vanilla_equipment_slot_group, vanilla_equipment_slot_group_for_serialized_name,
    VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
};
use bbb_protocol::component::{
    styled_component_runs_from_nbt_value, styled_runs_summary_text, StyledTextRun,
};
use bbb_protocol::nbt_summary::{canonical_double_tag_bits, codec_double_value};
use bbb_protocol::packets::AttributeModifierSummary;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::error::{invalid_persistent_component, ItemStackPersistentProjectionError, Result};
use super::nbt_support::{
    compound_fields, latest_field, list_entries, nbt_string, normalize_resource_id_units,
    MAX_PERSISTENT_LIST_ITEMS,
};

/// `ItemAttributeModifiers.CODEC` is `Entry.CODEC.listOf()`
/// so the save form is a plain list and
/// the empty list is `ItemAttributeModifiers.EMPTY`
///
/// Each `Entry` is
/// `Attribute.CODEC.fieldOf("type")` -- the built-in attribute registry's
/// identifier -- beside the flattened
/// `AttributeModifier.MAP_CODEC`: required
/// `id` and `amount` with the operation name, an optional `slot` defaulting to
/// `any` and an optional `display` defaulting to the `default` unit type.
/// `optionalFieldOf` drops a field that holds its default while encoding, so
/// both defaults are omitted rather than written.
pub(super) fn persistent_attribute_modifiers_nbt(
    type_id: i32,
    resource_id: &'static str,
    modifiers: &[AttributeModifierSummary],
) -> Result<NbtSummaryValue> {
    let entries = modifiers
        .iter()
        .map(|modifier| persistent_attribute_modifier_entry(type_id, resource_id, modifier))
        .collect::<Result<Vec<_>>>()?;
    Ok(NbtSummaryValue::List(entries))
}

fn persistent_attribute_modifier_entry(
    type_id: i32,
    resource_id: &'static str,
    modifier: &AttributeModifierSummary,
) -> Result<NbtSummaryValue> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    // `Attribute.CODEC` is `BuiltInRegistries.ATTRIBUTE.holderByNameCodec()`
    // and `Registries.ATTRIBUTE` is built in, so the
    // generated catalog issues the very ids the wire carried.
    let attribute = vanilla_attribute_type(modifier.attribute_id).ok_or_else(|| {
        invalid(format!(
            "unknown attribute holder id {}",
            modifier.attribute_id
        ))
    })?;
    // `AttributeModifier.Operation` is `StringRepresentable.fromEnum`
    // and the wire decode already folded an
    // out-of-range id onto `ADD_VALUE` the way `ByIdMap.continuous`'s
    // `OutOfBoundsStrategy.ZERO` does.
    let operation = attribute_modifier_operation_name(modifier.operation_id)
        .ok_or_else(|| invalid(format!("unknown operation id {}", modifier.operation_id)))?;
    // `EquipmentSlotGroup.CODEC` is `StringRepresentable.fromEnum`
    // the wire decode folds an out-of-range id
    // onto `ANY` the same way.
    let slot = vanilla_equipment_slot_group(modifier.slot_id).ok_or_else(|| {
        invalid(format!(
            "unknown equipment slot group id {}",
            modifier.slot_id
        ))
    })?;

    let modifier_id = bbb_protocol::normalize_resource_location(modifier.modifier_id.as_str())
        .map_err(|error| invalid(error.to_string()))?;

    let mut fields = vec![
        NbtSummaryEntry {
            name: "type".into(),
            value: NbtSummaryValue::String(attribute.resource_id.into()),
        },
        NbtSummaryEntry {
            name: "id".into(),
            // `Identifier.CODEC` writes the
            // modifier's own resource location, which the wire decode already
            // normalized.
            value: NbtSummaryValue::string(&modifier_id),
        },
        NbtSummaryEntry {
            name: "amount".into(),
            value: NbtSummaryValue::Double(canonical_double_tag_bits(f64::from_bits(
                modifier.amount_bits,
            ))),
        },
        NbtSummaryEntry {
            name: "operation".into(),
            value: NbtSummaryValue::String(operation.into()),
        },
    ];
    if modifier.slot_id != VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID {
        fields.push(NbtSummaryEntry {
            name: "slot".into(),
            value: NbtSummaryValue::String(slot.serialized_name.into()),
        });
    }
    if let Some(display) = persistent_attribute_modifier_display(type_id, resource_id, modifier)? {
        fields.push(display);
    }
    Ok(NbtSummaryValue::Compound(fields))
}

/// `ItemAttributeModifiers.Display.CODEC` is
/// `Type.CODEC.dispatch("type", Display::type, type -> type.codec)`
/// the dispatch writes the serialized
/// type name beside the branch codec's own fields, which are none for the two
/// unit branches and a required `value` Component for `override`
///
/// The dispatch has no untyped spelling, so a summary whose display text
/// disagrees with its display id is refused instead of being written as an
/// empty `override`.
fn persistent_attribute_modifier_display(
    type_id: i32,
    resource_id: &'static str,
    modifier: &AttributeModifierSummary,
) -> Result<Option<NbtSummaryEntry>> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    match modifier.display_id {
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID => {
            if modifier.display_text.is_some()
                || modifier.display_text_styled.is_some()
                || modifier.display_text_nbt.is_some()
            {
                return Err(invalid(
                    "the default display carries no override text".to_string(),
                ));
            }
            Ok(None)
        }
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID => {
            if modifier.display_text.is_some()
                || modifier.display_text_styled.is_some()
                || modifier.display_text_nbt.is_some()
            {
                return Err(invalid(
                    "the hidden display carries no override text".to_string(),
                ));
            }
            Ok(Some(NbtSummaryEntry {
                name: "display".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "type".into(),
                    value: NbtSummaryValue::String(
                        attribute_modifier_display_name(
                            VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
                        )
                        .expect("hidden display name")
                        .into(),
                    ),
                }]),
            }))
        }
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID => {
            if modifier.display_text.is_none() || modifier.display_text_styled.is_none() {
                return Err(invalid(
                    "the override display has no decoded text".to_string(),
                ));
            }
            // `ComponentSerialization.CODEC` is the exact Component tree the
            // wire decode already kept; the flattened text cannot rebuild it.
            let value = modifier.display_text_nbt.clone().ok_or(
                ItemStackPersistentProjectionError::UnsupportedComponent {
                    type_id,
                    resource_id,
                },
            )?;
            super::nbt_support::ComponentReader::new(type_id, resource_id)
                .text(&value, "attribute modifier display")?;
            Ok(Some(NbtSummaryEntry {
                name: "display".into(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "type".into(),
                        value: NbtSummaryValue::String(
                            attribute_modifier_display_name(
                                VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                            )
                            .expect("override display name")
                            .into(),
                        ),
                    },
                    NbtSummaryEntry {
                        name: "value".into(),
                        value,
                    },
                ]),
            }))
        }
        other => Err(invalid(format!("unknown display type id {other}"))),
    }
}

/// The wire projection of one `ItemAttributeModifiers.Entry`
/// whose absent `slot` and `display`
/// hold the two `optionalFieldOf` defaults.
pub(super) fn project_attribute_modifiers(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
) -> Result<Vec<AttributeModifierSummary>> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let entries = list_entries(value)
        .ok_or_else(|| invalid("attribute modifiers are not a list".to_string()))?;
    if entries.len() > MAX_PERSISTENT_LIST_ITEMS {
        return Err(invalid(format!(
            "attribute modifiers have {} entries, maximum is {MAX_PERSISTENT_LIST_ITEMS}",
            entries.len()
        )));
    }
    entries
        .iter()
        .map(|entry| project_attribute_modifier_entry(type_id, resource_id, &entry))
        .collect()
}

fn project_attribute_modifier_entry(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
) -> Result<AttributeModifierSummary> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let fields = compound_fields(value)
        .ok_or_else(|| invalid("attribute modifier entry is not a compound".to_string()))?;
    let attribute_name = nbt_string(
        latest_field(fields, "type")
            .ok_or_else(|| invalid("attribute modifier entry carries no type".to_string()))?,
    )
    .ok_or_else(|| invalid("attribute modifier type is not a string".to_string()))?;
    let attribute_id = vanilla_attribute_type_for_resource_id(&normalize_resource_id_units(
        attribute_name.units(),
    )?)
    .ok_or_else(|| invalid(format!("unknown attribute type {attribute_name}")))?
    .id;
    let modifier_id = nbt_string(
        latest_field(fields, "id")
            .ok_or_else(|| invalid("attribute modifier entry carries no id".to_string()))?,
    )
    .ok_or_else(|| invalid("attribute modifier id is not a string".to_string()))?;
    let amount = latest_field(fields, "amount")
        .and_then(codec_double_value)
        .ok_or_else(|| invalid("attribute modifier amount is not numeric".to_string()))?;
    let operation_name = nbt_string(
        latest_field(fields, "operation")
            .ok_or_else(|| invalid("attribute modifier entry carries no operation".to_string()))?,
    )
    .ok_or_else(|| invalid("attribute modifier operation is not a string".to_string()))?;
    let operation_id = operation_name
        .with_utf8(attribute_modifier_operation_id)
        .flatten()
        .ok_or_else(|| invalid(format!("unknown operation {operation_name}")))?;
    let slot_id = match latest_field(fields, "slot") {
        Some(slot) => {
            let slot_name = nbt_string(slot)
                .ok_or_else(|| invalid("attribute modifier slot is not a string".to_string()))?;
            slot_name
                .with_utf8(vanilla_equipment_slot_group_for_serialized_name)
                .flatten()
                .ok_or_else(|| invalid(format!("unknown equipment slot group {slot_name}")))?
                .id
        }
        None => VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
    };
    let display = project_attribute_modifier_display(type_id, resource_id, fields)?;
    Ok(AttributeModifierSummary {
        attribute_id,
        modifier_id: normalize_resource_id_units(modifier_id.units())?,
        amount_bits: amount.to_bits(),
        operation_id,
        slot_id,
        display_id: display.display_id,
        display_text: display.display_text,
        display_text_styled: display.display_text_styled,
        display_text_nbt: display.display_text_nbt,
    })
}

struct AttributeModifierDisplay {
    display_id: i32,
    display_text: Option<String>,
    display_text_styled: Option<Vec<StyledTextRun>>,
    display_text_nbt: Option<NbtSummaryValue>,
}

fn project_attribute_modifier_display(
    type_id: i32,
    resource_id: &'static str,
    fields: &[NbtSummaryEntry],
) -> Result<AttributeModifierDisplay> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let Some(display) = latest_field(fields, "display") else {
        return Ok(AttributeModifierDisplay {
            display_id: VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
            display_text: None,
            display_text_styled: None,
            display_text_nbt: None,
        });
    };
    let display_fields = compound_fields(display)
        .ok_or_else(|| invalid("attribute modifier display is not a compound".to_string()))?;
    let type_name = nbt_string(
        latest_field(display_fields, "type")
            .ok_or_else(|| invalid("attribute modifier display carries no type".to_string()))?,
    )
    .ok_or_else(|| invalid("attribute modifier display type is not a string".to_string()))?;
    match type_name.with_utf8(attribute_modifier_display_id).flatten() {
        Some(VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID) => Ok(AttributeModifierDisplay {
            display_id: VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
            display_text: None,
            display_text_styled: None,
            display_text_nbt: None,
        }),
        Some(VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID) => Ok(AttributeModifierDisplay {
            display_id: VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
            display_text: None,
            display_text_styled: None,
            display_text_nbt: None,
        }),
        Some(VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID) => {
            let component = latest_field(display_fields, "value")
                .ok_or_else(|| invalid("override display carries no value".to_string()))?;
            bbb_protocol::component::validate_component_nbt(
                component,
                "attribute modifier display",
            )
            .map_err(invalid)?;
            let styled = styled_component_runs_from_nbt_value(component);
            Ok(AttributeModifierDisplay {
                display_id: VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                display_text: Some(styled_runs_summary_text(&styled)),
                display_text_styled: Some(styled),
                display_text_nbt: Some(component.clone()),
            })
        }
        _ => Err(invalid(format!("unknown display type {type_name}"))),
    }
}
