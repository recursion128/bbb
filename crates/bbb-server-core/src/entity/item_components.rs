//! A stack's effective components: its patch over the item prototype.

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{AttributeModifierSummary, SoundEventHolder, SoundEventSummary};
use bbb_server_types::item_stack::{rules, RuntimeItemStack};

pub(crate) type Read<T> = Result<T, String>;

pub(crate) fn is_consumable_food(stack: &RuntimeItemStack) -> Read<bool> {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
    };
    let read = || -> bbb_server_types::Result<bool> {
        Ok(stack.has_component(VANILLA_DATA_COMPONENT_FOOD_TYPE_ID)?
            && stack.has_component(VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID)?)
    };
    read().map_err(|error| error.to_string())
}

pub(crate) fn sound_holder(sound: &SoundEventSummary) -> Option<SoundEventHolder> {
    if let Some(registry_id) = sound.registry_id {
        return Some(SoundEventHolder::Reference { registry_id });
    }
    Some(SoundEventHolder::Direct {
        location: sound.sound_id.clone()?,
        fixed_range: sound.fixed_range_bits.map(f32::from_bits),
    })
}

pub(crate) fn sound_resource_id(sound: &SoundEventSummary) -> Option<&str> {
    sound.sound_id.as_deref().or_else(|| {
        sound
            .registry_id
            .and_then(bbb_protocol::sound_event_types::vanilla_sound_event_resource_id)
    })
}

pub(crate) fn has(stack: &RuntimeItemStack, type_id: i32) -> Read<bool> {
    stack
        .has_component(type_id)
        .map_err(|error| format!("component {type_id}: {error}"))
}

/// `Equippable.damageOnHurt` (`Equippable.java:54`, default true), `false`
/// for a stack without `EQUIPPABLE`.
pub fn equippable_damage_on_hurt(stack: &RuntimeItemStack) -> Read<bool> {
    bbb_server_types::item_stack::static_item_component(
        stack.summary(),
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
    )
    .map(|patch| {
        patch
            .and_then(|patch| patch.equippable.as_ref())
            .is_some_and(|equippable| equippable.damage_on_hurt)
    })
    .map_err(|error| format!("component {VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID}: {error}"))
}

/// `ItemAttributeModifiers.Entry` (`ItemAttributeModifiers.java:305-315`).
pub(crate) fn attribute_modifier_applies_to(
    modifier: &AttributeModifierSummary,
    slot: EquipmentSlot,
) -> bool {
    let group = bbb_protocol::equipment_slots::vanilla_equipment_slot_group(modifier.slot_id)
        .expect("validated equipment slot group id");
    crate::item_durability::equipment_slot_group_matches(group.serialized_name, slot)
}

/// The stack's `ATTRIBUTE_MODIFIERS`, defaulting to `EMPTY`, in list order.
pub(crate) fn attribute_modifiers(stack: &RuntimeItemStack) -> Read<&[AttributeModifierSummary]> {
    bbb_server_types::item_stack::static_item_component(
        stack.summary(),
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
    )
    .map(|patch| patch.map_or(&[][..], |patch| patch.attribute_modifiers.as_slice()))
    .map_err(|error| {
        format!("component {VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID}: {error}")
    })
}

pub(crate) fn enchantment_count(stack: &RuntimeItemStack) -> usize {
    enchantment_holders(stack).len()
}

fn enchantment_holders(
    stack: &RuntimeItemStack,
) -> &[bbb_protocol::packets::ItemEnchantmentSummary] {
    let patch = &stack.summary().component_patch;
    if stack.is_empty()
        || !patch.added_type_ids.contains(
            &bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        )
    {
        return &[];
    }
    &patch.enchantments
}

/// The stack's `ENCHANTMENTS` as `(id, level)` (`ItemEnchantments.entrySet`),
/// naming holder ids through the generation's enchantment registry.
pub(crate) fn enchantments(
    stack: &RuntimeItemStack,
    registry: &[crate::enchantment_facts::EnchantmentSlots],
) -> Vec<(String, i32)> {
    enchantment_holders(stack)
        .iter()
        .filter_map(|entry| {
            let enchantment = registry.get(usize::try_from(entry.holder_id).ok()?)?;
            Some((enchantment.resource_id.clone(), entry.level))
        })
        .collect()
}

/// `ItemStack.getDamageValue` (`ItemStack.java:425-427`).
pub(crate) fn damage_value(stack: &RuntimeItemStack) -> Read<i32> {
    Ok(rules::current_damage(stack.summary(), max_damage(stack)?))
}

/// `ItemStack.getMaxDamage` (`ItemStack.java:433-435`).
pub(crate) fn max_damage(stack: &RuntimeItemStack) -> Read<i32> {
    let summary = stack.summary();
    rules::component_i32(
        summary,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        summary.component_patch.max_damage,
        0,
    )
    .map_err(|error| format!("component {VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID}: {error}"))
}

/// `ItemStack.isDamageableItem` (`ItemStack.java:417-419`) with
/// `getDamageValue` and `getMaxDamage` (`:425-435`) when it is.
pub(crate) fn damage_and_max(stack: &RuntimeItemStack) -> Read<Option<(i32, i32)>> {
    Ok(rules::damage_and_max(stack.summary()))
}

/// `ItemStack.isBroken` (`ItemStack.java:437-439`) over `isDamageableItem`
/// (`:417-419`).
pub(crate) fn is_broken(stack: &RuntimeItemStack) -> Read<bool> {
    Ok(damage_and_max(stack)?.is_some_and(|(damage, max)| damage >= max))
}
