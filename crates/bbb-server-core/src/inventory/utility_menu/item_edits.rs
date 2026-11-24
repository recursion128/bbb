//! The component reads and writes the utility menus make on a stack.

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID, VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use bbb_protocol::item_types::VANILLA_ITEM_ENCHANTED_BOOK_ID;
use bbb_protocol::packets::{DataComponentPatchSummary, ItemEnchantmentSummary, ItemStackSummary};
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_datapack::recipe::ItemTagIndex;
use bbb_server_types::item_stack::rules::{component_i32, has_component};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

/// An enchantment holder id and its level.
pub(super) type Enchantments = Vec<(usize, i32)>;

/// `EnchantmentHelper.getComponentType` (`EnchantmentHelper.java:81-83`).
pub(super) fn enchantment_component(stack: &RuntimeItemStack) -> i32 {
    if stack.item_id() == Some(VANILLA_ITEM_ENCHANTED_BOOK_ID) {
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID
    } else {
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID
    }
}

/// One of the two enchantment maps; every item prototype carries them empty.
pub(super) fn enchantment_list(stack: &RuntimeItemStack, type_id: i32) -> Enchantments {
    let patch = &stack.summary().component_patch;
    if stack.is_empty() || !patch.added_type_ids.contains(&type_id) {
        return Vec::new();
    }
    let list = if type_id == VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID {
        &patch.stored_enchantments
    } else {
        &patch.enchantments
    };
    list.iter()
        .filter_map(|entry| Some((usize::try_from(entry.holder_id).ok()?, entry.level)))
        .collect()
}

/// `EnchantmentHelper.getEnchantmentsForCrafting` (`EnchantmentHelper.java:77-79`).
pub(super) fn crafting_enchantments(stack: &RuntimeItemStack) -> Enchantments {
    enchantment_list(stack, enchantment_component(stack))
}

/// `EnchantmentHelper.canStoreEnchantments` (`EnchantmentHelper.java:69-71`).
pub(super) fn can_store_enchantments(stack: &RuntimeItemStack) -> Result<bool> {
    has_component(stack.summary(), enchantment_component(stack))
}

/// `EnchantmentHelper.hasAnyEnchantments` (`EnchantmentHelper.java:85-88`).
pub(super) fn has_any_enchantments(stack: &RuntimeItemStack) -> bool {
    !enchantment_list(stack, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID).is_empty()
        || !enchantment_list(stack, VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID).is_empty()
}

/// `ItemEnchantments.Mutable.upgrade` (`ItemEnchantments.java:139-143`).
pub(super) fn upgrade(enchantments: &mut Enchantments, holder: usize, level: i32) {
    if level <= 0 {
        return;
    }
    let level = level.min(255);
    match enchantments.iter_mut().find(|(id, _)| *id == holder) {
        Some((_, current)) => *current = (*current).max(level),
        None => enchantments.push((holder, level)),
    }
}

/// `ItemEnchantments.Mutable.set` (`ItemEnchantments.java:131-137`).
pub(super) fn set_level(enchantments: &mut Enchantments, holder: usize, level: i32) {
    if level <= 0 {
        enchantments.retain(|(id, _)| *id != holder);
        return;
    }
    let level = level.min(255);
    match enchantments.iter_mut().find(|(id, _)| *id == holder) {
        Some((_, current)) => *current = level,
        None => enchantments.push((holder, level)),
    }
}

/// `EnchantmentHelper.setEnchantments` (`EnchantmentHelper.java:73-75`).
pub(super) fn with_crafting_enchantments(
    stack: &RuntimeItemStack,
    enchantments: &[(usize, i32)],
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<RuntimeItemStack> {
    let type_id = enchantment_component(stack);
    let entries = enchantments
        .iter()
        .map(|&(holder, level)| ItemEnchantmentSummary {
            holder_id: holder as i32,
            level,
        })
        .collect();
    let mut patch = DataComponentPatchSummary {
        added_type_ids: vec![type_id],
        ..Default::default()
    };
    if type_id == VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID {
        patch.stored_enchantments = entries;
    } else {
        patch.enchantments = entries;
    }
    stack.clone().with_component_patch(&patch, resolver)
}

/// `ItemStack.isDamageableItem` (`ItemStack.java:417-419`).
pub(super) fn is_damageable(stack: &RuntimeItemStack) -> Result<bool> {
    crate::inventory::item_rules::is_damageable_item(stack.summary())
}

/// `ItemStack.getMaxDamage` (`ItemStack.java:433-435`).
pub(super) fn max_damage(stack: &RuntimeItemStack) -> Result<i32> {
    let summary = stack.summary();
    component_i32(
        summary,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        summary.component_patch.max_damage,
        0,
    )
}

/// `ItemStack.getDamageValue` (`ItemStack.java:425-427`).
pub(super) fn damage_value(stack: &RuntimeItemStack) -> Result<i32> {
    let summary = stack.summary();
    let damage = component_i32(
        summary,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        summary.component_patch.damage,
        0,
    )?;
    Ok(damage.clamp(0, max_damage(stack)?.max(0)))
}

/// `ItemStack.setDamageValue` (`ItemStack.java:429-431`).
pub(super) fn with_damage(
    stack: &RuntimeItemStack,
    damage: i32,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<RuntimeItemStack> {
    let damage = damage.clamp(0, max_damage(stack)?.max(0));
    with_int(
        stack,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        damage,
        resolver,
    )
}

pub(super) fn with_max_damage(
    stack: &RuntimeItemStack,
    max: i32,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<RuntimeItemStack> {
    with_int(
        stack,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        max,
        resolver,
    )
}

/// `getOrDefault(DataComponents.REPAIR_COST, 0)`.
pub(super) fn repair_cost(stack: &RuntimeItemStack) -> Result<i32> {
    let summary = stack.summary();
    component_i32(
        summary,
        VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
        summary.component_patch.repair_cost,
        0,
    )
}

pub(super) fn with_repair_cost(
    stack: &RuntimeItemStack,
    cost: i32,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<RuntimeItemStack> {
    with_int(
        stack,
        VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
        cost,
        resolver,
    )
}

fn with_int(
    stack: &RuntimeItemStack,
    type_id: i32,
    value: i32,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<RuntimeItemStack> {
    stack
        .clone()
        .with_component_nbt(type_id, Some(&NbtSummaryValue::Int(value)), resolver)
}

/// `AnvilMenu.calculateIncreasedRepairCost` (`AnvilMenu.java:276-278`).
pub(super) fn increased_repair_cost(cost: i32) -> i32 {
    (i64::from(cost) * 2 + 1).min(i64::from(i32::MAX)) as i32
}

/// `ItemStack.getHoverName().getString()` (`ItemStack.java:804-828`,
/// `Item.java:341-343`): `CUSTOM_NAME`, else a non-blank written-book title,
/// else `ITEM_NAME` (from patch or item prototype), else empty.
pub(super) fn hover_name(
    stack: &RuntimeItemStack,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<bbb_protocol::JavaString> {
    if let Some(custom) =
        stack.component_nbt(VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, resolver)?
    {
        return Ok(bbb_server_datapack::language::component_plain_text(&custom).into());
    }
    if let Some(title) = stack
        .effective_component(
            VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
            resolver,
        )?
        .and_then(|patch| patch.written_book.as_ref().map(|book| book.title.clone()))
    {
        if !String::from_utf16_lossy(title.units())
            .chars()
            .all(char::is_whitespace)
        {
            return Ok(title);
        }
    }
    if let Some(item_name) =
        stack.component_nbt(VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID, resolver)?
    {
        return Ok(bbb_server_datapack::language::component_plain_text(&item_name).into());
    }
    Ok(bbb_protocol::JavaString::default())
}

/// `ItemStack.set(CUSTOM_NAME, Component.literal(name))` or `remove`.
pub(super) fn with_custom_name(
    stack: &RuntimeItemStack,
    name: Option<&str>,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<RuntimeItemStack> {
    let value = name.map(NbtSummaryValue::string);
    stack.clone().with_component_nbt(
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
        value.as_ref(),
        resolver,
    )
}

/// `ItemStack.isValidRepairItem` (`ItemStack.java:1117-1120`) over
/// `Repairable.isValidRepairItem`.
pub(super) fn is_valid_repair_item(
    stack: &RuntimeItemStack,
    repair: &RuntimeItemStack,
    tags: &ItemTagIndex,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<bool> {
    let Some(repair_id) = repair.item_id() else {
        return Ok(false);
    };
    let Some(repairable) =
        stack.effective_component(VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID, resolver)?
    else {
        return Ok(false);
    };
    if repairable.repairable_item_registry_ids.contains(&repair_id) {
        return Ok(true);
    }
    Ok(repairable
        .repairable_item_tag
        .as_deref()
        .is_some_and(|tag| tags.contains(tag.trim_start_matches('#'), repair_id)))
}

/// `ItemStack.transmuteCopy` (`ItemStack.java:600-608`): applies the stack's
/// component patch against the target item's prototype via
/// `PatchedDataComponentMap` (`PatchedDataComponentMap.java:139-148`).
pub(crate) fn transmute(
    stack: &RuntimeItemStack,
    item_id: i32,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<RuntimeItemStack> {
    if stack.is_empty() {
        return Ok(RuntimeItemStack::empty());
    }
    RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(item_id),
        count: stack.count(),
        component_patch: Default::default(),
    })
    .with_component_patch(&stack.summary().component_patch, resolver)
}
