//! The stack reads the pickup decisions make, over the stack's effective
//! components: its patch over the item prototype.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::data_components::EquippableSummary;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::attributes::AttributeOperation;
use crate::entity::item_components::attribute_modifier_applies_to;
pub(super) use crate::entity::item_components::{damage_value, has};
use crate::entity::store::EntityStore;

pub(super) type Read<T> = Result<T, String>;

/// `Holder.is(TagKey)` for the stack's item in the level's tag generation.
pub(super) fn is_in_tag(level: &LevelCtx<'_, '_>, stack: &RuntimeItemStack, tag: &str) -> bool {
    stack
        .item_resource_id()
        .is_some_and(|item| level.registry_tag_membership("minecraft:item", tag, item))
}

pub(super) fn is_item(stack: &RuntimeItemStack, item_id: i32) -> bool {
    stack.item_id() == Some(item_id)
}

pub(super) fn equippable(stack: &RuntimeItemStack) -> Read<Option<&EquippableSummary>> {
    crate::entity::living::valid_equipment::equippable(stack)
        .ok_or_else(|| "equippable is unreadable".into())
}

pub(super) fn compute_attribute(
    stack: &RuntimeItemStack,
    attribute: &str,
    base_value: f64,
    slot: EquipmentSlot,
) -> Read<f64> {
    let mut result = base_value;
    for entry in crate::entity::item_components::attribute_modifiers(stack)? {
        let entry_attribute =
            bbb_protocol::attribute_types::vanilla_attribute_type(entry.attribute_id)
                .expect("validated attribute holder id");
        if entry_attribute.resource_id != attribute || !attribute_modifier_applies_to(entry, slot) {
            continue;
        }
        let amount = f64::from_bits(entry.amount_bits);
        result += match AttributeOperation::by_id(entry.operation_id) {
            AttributeOperation::AddValue => amount,
            AttributeOperation::AddMultipliedBase => amount * base_value,
            AttributeOperation::AddMultipliedTotal => amount * result,
        };
    }
    Ok(result)
}

/// The enchantment ids of `ENCHANTMENTS` (`ItemEnchantments.entrySet`).
pub(super) fn enchantment_ids(stack: &RuntimeItemStack, store: &EntityStore) -> Vec<String> {
    crate::entity::armor_stand::stack_enchantment_ids(stack, store)
}

pub(super) fn has_custom_name(stack: &RuntimeItemStack) -> Read<bool> {
    has(stack, VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID)
}
