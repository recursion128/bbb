//! The attribute half of `LivingEntity.collectEquipmentChanges`
//! (`LivingEntity.java:2883-2922`): an equipped stack's item and enchantment
//! modifiers enter and leave the wearer's attribute map.

use crate::enchantment_facts::{EnchantmentAttributeTable, EnchantmentSlots};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::attributes::{AttributeMap, AttributeModifier, AttributeOperation};
use crate::entity::item_components::{
    attribute_modifier_applies_to, attribute_modifiers, enchantments, is_broken,
};

/// `ItemStack.forEachModifier(EquipmentSlot, BiConsumer)`
/// (`ItemStack.java:1013-1017`): the stack's `ATTRIBUTE_MODIFIERS` that test
/// true for `slot`, then `EnchantmentHelper.forEachModifier`
/// (`EnchantmentHelper.java:395-401`).
pub fn for_each_modifier(
    stack: &RuntimeItemStack,
    slot: EquipmentSlot,
    table: &EnchantmentAttributeTable,
    registry: &[EnchantmentSlots],
    mut consumer: impl FnMut(i32, AttributeModifier),
) -> Result<(), String> {
    for entry in attribute_modifiers(stack)? {
        if attribute_modifier_applies_to(entry, slot) {
            consumer(
                entry.attribute_id,
                AttributeModifier {
                    id: entry.modifier_id.clone(),
                    amount: f64::from_bits(entry.amount_bits),
                    operation: AttributeOperation::by_id(entry.operation_id),
                },
            );
        }
    }
    for (id, level) in enchantments(stack, registry) {
        let Some(effects) = table.get(&id) else {
            continue;
        };
        let effects = effects.as_ref().map_err(Clone::clone)?;
        if !effects.matching_slot(slot) {
            continue;
        }
        for effect in &effects.effects {
            // `EnchantmentAttributeEffect.getModifier` (`EnchantmentAttributeEffect.java:35-37`).
            consumer(
                effect.attribute_id,
                AttributeModifier {
                    id: effect.modifier_id(slot),
                    amount: f64::from(effect.amount.calculate(level)),
                    operation: AttributeOperation::from_serialized_name(&effect.operation)
                        .ok_or_else(|| format!("{id} names operation {}", effect.operation))?,
                },
            );
        }
    }
    Ok(())
}

/// One slot `collectEquipmentChanges` found changed.
pub struct EquipmentChange<'a> {
    pub slot: EquipmentSlot,
    pub previous: Option<&'a RuntimeItemStack>,
    pub current: Option<&'a RuntimeItemStack>,
}

/// Every changed slot's previous stack leaves first (`stopLocationBasedEffects`,
/// `LivingEntity.java:3786-3794`), then each unbroken current stack replaces
/// its modifiers as transient ones (`:2902-2915`). The location-based half is
/// [`crate::entity::location_effects`]'s.
pub fn apply_equipment_changes(
    attributes: &mut AttributeMap,
    changes: &[EquipmentChange<'_>],
    table: &EnchantmentAttributeTable,
    registry: &[EnchantmentSlots],
) -> Result<(), String> {
    let mut removed = Vec::new();
    for change in changes {
        if let Some(previous) = change.previous {
            for_each_modifier(
                previous,
                change.slot,
                table,
                registry,
                |attribute, modifier| {
                    removed.push((attribute, modifier.id));
                },
            )?;
        }
    }
    let mut added = Vec::new();
    for change in changes {
        let Some(current) = change.current else {
            continue;
        };
        if is_broken(current)? {
            continue;
        }
        for_each_modifier(
            current,
            change.slot,
            table,
            registry,
            |attribute, modifier| {
                added.push((attribute, modifier));
            },
        )?;
    }
    for (attribute, id) in removed {
        if let Some(instance) = attributes.get_instance(attribute) {
            instance.remove_modifier(&id);
        }
    }
    for (attribute, modifier) in added {
        if let Some(instance) = attributes.get_instance(attribute) {
            instance.remove_modifier(&modifier.id);
            instance.add_or_update_transient_modifier(modifier);
        }
    }
    Ok(())
}
