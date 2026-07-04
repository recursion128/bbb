//! Slot-level container and menu machinery behind the local click paths.
//!
//! Free functions (plus their private helper types) shared by
//! [`InventoryState`]'s click/quick-move application methods and by the
//! per-menu submodules; split out of `inventory.rs` to keep the state module
//! focused on the store itself.

use super::*;

pub(super) fn mount_horse_saddle_slot_is_active(
    entity_type_id: i32,
    data_values: &[ProtocolEntityDataValue],
) -> bool {
    if !crate::entities::is_vanilla_can_equip_saddle_type(entity_type_id) {
        return false;
    }
    if crate::entities::is_vanilla_horse_slot_always_active_type(entity_type_id) {
        return true;
    }
    !mount_entity_is_ageable_baby(data_values)
        && (entity_data_byte(data_values, VANILLA_MOUNT_TAME_FLAGS_DATA_ID, 0)
            & VANILLA_ABSTRACT_HORSE_TAME_FLAG)
            != 0
}

pub(super) fn mount_horse_body_slot_kind(entity_type_id: i32) -> Option<MountArmorSlotKind> {
    if crate::entities::is_vanilla_llama_type(entity_type_id) {
        Some(MountArmorSlotKind::Llama)
    } else if crate::entities::is_vanilla_can_wear_horse_armor_type(entity_type_id) {
        Some(MountArmorSlotKind::Horse)
    } else {
        None
    }
}

pub(super) fn mount_nautilus_can_use_equipment_slots(
    data_values: &[ProtocolEntityDataValue],
) -> bool {
    !mount_entity_is_ageable_baby(data_values)
        && (entity_data_byte(data_values, VANILLA_MOUNT_TAME_FLAGS_DATA_ID, 0)
            & VANILLA_TAMABLE_ANIMAL_TAME_FLAG)
            != 0
}

pub(super) fn mount_entity_is_ageable_baby(data_values: &[ProtocolEntityDataValue]) -> bool {
    entity_data_bool(data_values, VANILLA_AGEABLE_MOB_BABY_DATA_ID, false)
}

pub(super) fn entity_data_bool(
    data_values: &[ProtocolEntityDataValue],
    data_id: u8,
    fallback: bool,
) -> bool {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Boolean(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(fallback)
}

pub(super) fn entity_data_byte(
    data_values: &[ProtocolEntityDataValue],
    data_id: u8,
    fallback: i8,
) -> i8 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Byte(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(fallback)
}

pub(super) fn container_click_slot_is_valid(container: &ContainerState, slot_num: i16) -> bool {
    matches!(slot_num, -1 | -999) || container.slots.iter().any(|slot| slot.slot == slot_num)
}

pub(super) fn hashed_stack_from_summary(
    stack: &ProtocolItemStackSummary,
) -> Option<ProtocolHashedStack> {
    let (Some(item_id), true) = (stack.item_id, stack.count > 0) else {
        return Some(ProtocolHashedStack::Empty);
    };
    let components = hashed_component_patch_from_summary(&stack.component_patch)?;
    Some(ProtocolHashedStack::Item(ProtocolHashedItemStack {
        item_id,
        count: stack.count,
        components,
    }))
}

pub(super) fn item_stack_is_non_empty(stack: &ProtocolItemStackSummary) -> bool {
    stack.item_id.is_some() && stack.count > 0
}

pub(super) fn item_stack_has_piercing_weapon(
    stack: &ProtocolItemStackSummary,
    default_piercing_weapon_item_ids: &BTreeSet<i32>,
) -> bool {
    if item_stack_is_empty(stack)
        || stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_PIERCING_WEAPON_COMPONENT_ID)
    {
        return false;
    }

    let Some(item_id) = stack.item_id.filter(|item_id| *item_id >= 0) else {
        return false;
    };

    default_piercing_weapon_item_ids.contains(&item_id)
        || stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_PIERCING_WEAPON_COMPONENT_ID)
}

pub(super) fn item_stack_has_map_id(stack: &ProtocolItemStackSummary) -> bool {
    if item_stack_is_empty(stack)
        || stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_MAP_ID_COMPONENT_ID)
    {
        return false;
    }

    stack.component_patch.map_id.is_some()
        || stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_MAP_ID_COMPONENT_ID)
}

pub(super) fn item_stack_attack_range(
    stack: &ProtocolItemStackSummary,
    default_item_attack_ranges: &BTreeMap<i32, ItemAttackRange>,
) -> Option<ItemAttackRange> {
    if item_stack_is_empty(stack)
        || stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_ATTACK_RANGE_COMPONENT_ID)
    {
        return None;
    }

    if let Some(attack_range) = stack.component_patch.attack_range {
        return Some(item_attack_range_from_protocol(attack_range));
    }

    let item_id = stack.item_id.filter(|item_id| *item_id >= 0)?;
    default_item_attack_ranges.get(&item_id).copied()
}

pub(super) fn item_stack_swing_duration(
    stack: &ProtocolItemStackSummary,
    default_item_swing_animation_durations: &BTreeMap<i32, i32>,
) -> i32 {
    if item_stack_is_empty(stack)
        || stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_SWING_ANIMATION_COMPONENT_ID)
    {
        return ATTACK_SWING_DURATION;
    }

    if let Some(swing_animation) = stack.component_patch.swing_animation {
        return item_swing_animation_duration_from_protocol(swing_animation);
    }

    let Some(item_id) = stack.item_id.filter(|item_id| *item_id >= 0) else {
        return ATTACK_SWING_DURATION;
    };
    default_item_swing_animation_durations
        .get(&item_id)
        .copied()
        .unwrap_or(ATTACK_SWING_DURATION)
}

pub(super) fn item_swing_animation_duration_from_protocol(
    swing_animation: ProtocolSwingAnimationSummary,
) -> i32 {
    if swing_animation.duration > 0 {
        swing_animation.duration
    } else {
        ATTACK_SWING_DURATION
    }
}

pub(super) fn item_attack_range_from_protocol(
    attack_range: ProtocolAttackRangeSummary,
) -> ItemAttackRange {
    ItemAttackRange {
        min_reach: attack_range.min_reach,
        max_reach: attack_range.max_reach,
        min_creative_reach: attack_range.min_creative_reach,
        max_creative_reach: attack_range.max_creative_reach,
        hitbox_margin: attack_range.hitbox_margin,
        mob_factor: attack_range.mob_factor,
    }
}

pub(super) fn item_stack_use_effects(
    stack: &ProtocolItemStackSummary,
    default_item_use_effects: &BTreeMap<i32, ItemUseEffects>,
) -> Option<ItemUseEffects> {
    if item_stack_is_empty(stack) {
        return None;
    }

    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_USE_EFFECTS_COMPONENT_ID)
    {
        return Some(ItemUseEffects::default());
    }

    if let Some(effects) = stack.component_patch.use_effects {
        return Some(ItemUseEffects {
            can_sprint: effects.can_sprint,
            interact_vibrations: effects.interact_vibrations,
            speed_multiplier: effects.speed_multiplier,
        });
    }

    let default_effects = stack
        .item_id
        .filter(|item_id| *item_id >= 0)
        .and_then(|item_id| default_item_use_effects.get(&item_id).copied())
        .unwrap_or_default();
    Some(default_effects)
}

pub(super) fn hashed_component_patch_from_summary(
    patch: &ProtocolDataComponentPatchSummary,
) -> Option<ProtocolHashedComponentPatch> {
    if patch == &ProtocolDataComponentPatchSummary::default() {
        return Some(ProtocolHashedComponentPatch::default());
    }

    if patch.added != patch.added_type_ids.len() {
        return None;
    }

    let removed_components: BTreeSet<_> = patch.removed_type_ids.iter().copied().collect();
    if removed_components.len() != patch.removed_type_ids.len() {
        return None;
    }

    let mut expected = ProtocolDataComponentPatchSummary {
        added: patch.added,
        added_type_ids: patch.added_type_ids.clone(),
        removed_type_ids: patch.removed_type_ids.clone(),
        ..ProtocolDataComponentPatchSummary::default()
    };
    let mut added_components = BTreeMap::new();
    let mut added_type_ids = BTreeSet::new();
    for component_type_id in &patch.added_type_ids {
        if !added_type_ids.insert(*component_type_id) {
            return None;
        }
        let value = match *component_type_id {
            VANILLA_MAX_STACK_SIZE_COMPONENT_ID => {
                let value = patch.max_stack_size?;
                expected.max_stack_size = Some(value);
                value
            }
            VANILLA_MAX_DAMAGE_COMPONENT_ID => {
                let value = patch.max_damage?;
                expected.max_damage = Some(value);
                value
            }
            VANILLA_DAMAGE_COMPONENT_ID => {
                let value = patch.damage?;
                expected.damage = Some(value);
                value
            }
            VANILLA_MAP_ID_COMPONENT_ID => {
                let value = patch.map_id?;
                expected.map_id = Some(value);
                value
            }
            _ => return None,
        };
        added_components.insert(*component_type_id, hash_ops_crc32c_int(value));
    }
    if patch != &expected {
        return None;
    }

    Some(ProtocolHashedComponentPatch {
        added_components,
        removed_components,
    })
}

pub(super) fn component_patch_can_be_hashed_from_summary(
    patch: &ProtocolDataComponentPatchSummary,
) -> bool {
    hashed_component_patch_from_summary(patch).is_some()
}

pub(super) fn hash_ops_crc32c_int(value: i32) -> i32 {
    let mut bytes = [0u8; 5];
    bytes[0] = 8;
    bytes[1..].copy_from_slice(&value.to_le_bytes());
    crc32c(&bytes) as i32
}

pub(super) fn crc32c(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0x82f6_3b78 & mask);
        }
    }
    !crc
}

pub(super) fn set_inventory_slot(slots: &mut Vec<InventorySlot>, mut update: InventorySlot) {
    update.local_selected_bundle_item_index = normalize_local_selected_bundle_item_index(
        update.local_selected_bundle_item_index,
        &update.item,
    );
    if let Some(existing) = slots.iter_mut().find(|slot| slot.slot == update.slot) {
        *existing = update;
    } else {
        slots.push(update);
    }
    slots.sort_by_key(|slot| slot.slot);
}

pub(super) fn set_container_slot(slots: &mut Vec<ContainerSlot>, mut update: ContainerSlot) {
    update.local_selected_bundle_item_index = normalize_local_selected_bundle_item_index(
        update.local_selected_bundle_item_index,
        &update.item,
    );
    if let Some(existing) = slots.iter_mut().find(|slot| slot.slot == update.slot) {
        *existing = update;
    } else {
        slots.push(update);
    }
    slots.sort_by_key(|slot| slot.slot);
}

pub(super) fn changed_hashed_slots(
    before: &[ContainerSlot],
    after: &[ContainerSlot],
) -> Result<BTreeMap<i16, ProtocolHashedStack>, ContainerClickBuildError> {
    let mut changed = BTreeMap::new();
    for slot in after {
        if before
            .iter()
            .find(|before| before.slot == slot.slot)
            .is_some_and(|before| before.item == slot.item)
        {
            continue;
        }
        let hashed = hashed_stack_from_summary(&slot.item)
            .ok_or(ContainerClickBuildError::UnhashableChangedSlot(slot.slot))?;
        changed.insert(slot.slot, hashed);
    }
    Ok(changed)
}

pub(super) fn inventory_menu_result_was_taken(
    before: &[ContainerSlot],
    after: &[ContainerSlot],
) -> bool {
    let Some(before_result) = container_slot_item(before, 0) else {
        return false;
    };
    if item_stack_is_empty(before_result) {
        return false;
    }

    let Some(after_result) = container_slot_item(after, 0) else {
        return true;
    };
    if item_stack_is_empty(after_result) {
        return true;
    }
    !same_item_same_components(before_result, after_result)
        || after_result.count < before_result.count
}

pub(super) fn inventory_menu_result_click_requires_server_authority(
    container_id: i32,
    slot_num: i16,
    input: ProtocolContainerInput,
    slots: &[ContainerSlot],
    default_item_crafting_remainders_known: bool,
    default_item_crafting_remainders: &BTreeMap<i32, i32>,
    recipe_specific_crafting_remainder_item_ids: &BTreeSet<i32>,
) -> bool {
    if container_id != INVENTORY_MENU_CONTAINER_ID
        || slot_num != 0
        || !matches!(
            input,
            ProtocolContainerInput::Pickup | ProtocolContainerInput::QuickMove
        )
        || container_slot_item(slots, 0).is_none_or(item_stack_is_empty)
    {
        return false;
    }

    !inventory_menu_non_empty_crafting_slot_nums(slots).is_empty()
        && inventory_menu_predictable_input_slot_nums(
            slots,
            default_item_crafting_remainders_known,
            default_item_crafting_remainders,
            recipe_specific_crafting_remainder_item_ids,
        )
        .is_none()
}

pub(super) fn apply_inventory_menu_result_take_side_effects(
    slots: &mut [ContainerSlot],
    default_item_crafting_remainders: &BTreeMap<i32, i32>,
    selected_hotbar_slot: u8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> Option<CraftingResultTakeSideEffects> {
    let input_slot_nums = inventory_menu_non_empty_crafting_slot_nums(slots);
    apply_crafting_result_take_side_effects_for_slots(
        INVENTORY_MENU_CONTAINER_ID,
        slots,
        &input_slot_nums,
        default_item_crafting_remainders,
        Some(PlayerInventoryAddSlots::inventory_menu()),
        selected_hotbar_slot,
        default_item_max_stack_sizes,
    )
}

pub(super) fn inventory_menu_non_empty_crafting_slot_nums(slots: &[ContainerSlot]) -> Vec<i16> {
    non_empty_slot_nums(slots, 1, 5)
}

pub(super) fn non_empty_slot_nums(
    slots: &[ContainerSlot],
    start_slot: i16,
    end_slot: i16,
) -> Vec<i16> {
    (start_slot..end_slot)
        .filter(|slot_num| {
            slots
                .iter()
                .find(|slot| slot.slot == *slot_num)
                .is_some_and(|slot| item_stack_is_non_empty(&slot.item))
        })
        .collect()
}

pub(super) fn inventory_menu_inputs_can_take_result(
    slots: &[ContainerSlot],
    input_slot_nums: &[i16],
) -> bool {
    !input_slot_nums.is_empty()
        && input_slot_nums.iter().all(|slot_num| {
            slots
                .iter()
                .find(|slot| slot.slot == *slot_num)
                .is_some_and(|slot| item_stack_is_non_empty(&slot.item))
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CraftingResultTakeSideEffects {
    pub(super) inputs_can_still_take_result: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PlayerInventoryAddSlots {
    main_start: i16,
    main_end: i16,
    hotbar_start: i16,
    hotbar_end: i16,
    offhand_slot: Option<i16>,
}

impl PlayerInventoryAddSlots {
    fn inventory_menu() -> Self {
        Self {
            main_start: INVENTORY_MENU_MAIN_START,
            main_end: INVENTORY_MENU_MAIN_END,
            hotbar_start: INVENTORY_MENU_HOTBAR_START,
            hotbar_end: INVENTORY_MENU_HOTBAR_END,
            offhand_slot: Some(INVENTORY_MENU_OFFHAND_SLOT),
        }
    }
}

pub(super) fn apply_crafting_result_take_side_effects_for_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    input_slot_nums: &[i16],
    default_item_crafting_remainders: &BTreeMap<i32, i32>,
    player_inventory_add_slots: Option<PlayerInventoryAddSlots>,
    selected_hotbar_slot: u8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> Option<CraftingResultTakeSideEffects> {
    let mut inputs_can_still_take_result = true;
    for slot_num in input_slot_nums {
        let Some(slot_index) = slots.iter().position(|slot| slot.slot == *slot_num) else {
            continue;
        };
        if item_stack_is_empty(&slots[slot_index].item) {
            continue;
        }
        let input_before = slots[slot_index].item.clone();
        let remainder = input_before
            .item_id
            .and_then(|item_id| default_item_crafting_remainders.get(&item_id).copied())
            .map(simple_item_stack);

        slots[slot_index].item.count -= 1;
        normalize_item_stack(&mut slots[slot_index].item);
        if let Some(mut replacement) = remainder {
            if item_stack_is_empty(&slots[slot_index].item) {
                slots[slot_index].item = replacement;
            } else if same_item_same_components(&slots[slot_index].item, &replacement) {
                replacement.count += slots[slot_index].item.count;
                normalize_item_stack(&mut replacement);
                slots[slot_index].item = replacement;
            } else {
                let player_inventory_add_slots = player_inventory_add_slots?;
                if !add_item_stack_to_visible_player_inventory(
                    container_id,
                    slots,
                    &mut replacement,
                    player_inventory_add_slots,
                    selected_hotbar_slot,
                    default_item_max_stack_sizes,
                ) || item_stack_is_non_empty(&replacement)
                {
                    return None;
                }
            }
        }
        normalize_container_slot_selection(&mut slots[slot_index]);
        if !item_stack_is_non_empty(&slots[slot_index].item)
            || !same_item_same_components(&slots[slot_index].item, &input_before)
        {
            inputs_can_still_take_result = false;
        }
    }
    Some(CraftingResultTakeSideEffects {
        inputs_can_still_take_result,
    })
}

pub(super) fn add_item_stack_to_visible_player_inventory(
    container_id: i32,
    slots: &mut [ContainerSlot],
    moving: &mut ProtocolItemStackSummary,
    player_inventory_add_slots: PlayerInventoryAddSlots,
    selected_hotbar_slot: u8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    let mut changed = false;
    if item_stack_max_stack_size(moving, default_item_max_stack_sizes) > 1 {
        for dest_slot in
            player_inventory_merge_slot_order(player_inventory_add_slots, selected_hotbar_slot)
        {
            if item_stack_is_empty(moving) {
                break;
            }
            let Some(dest_index) = slots.iter().position(|slot| slot.slot == dest_slot) else {
                continue;
            };
            let slot = &mut slots[dest_index];
            if item_stack_is_empty(&slot.item) || !same_item_same_components(moving, &slot.item) {
                continue;
            }
            let max_stack_size = container_slot_max_stack_size(
                container_id,
                dest_slot,
                &slot.item,
                default_item_max_stack_sizes,
            );
            let moved = moving.count.min((max_stack_size - slot.item.count).max(0));
            if moved <= 0 {
                continue;
            }
            slot.item.count += moved;
            moving.count -= moved;
            normalize_item_stack(moving);
            normalize_container_slot_selection(slot);
            changed = true;
        }
    }

    if item_stack_is_non_empty(moving) {
        for dest_slot in player_inventory_free_slot_order(player_inventory_add_slots) {
            let Some(dest_index) = slots.iter().position(|slot| slot.slot == dest_slot) else {
                continue;
            };
            if item_stack_is_non_empty(&slots[dest_index].item) {
                continue;
            }
            let max_stack_size = container_slot_max_stack_size(
                container_id,
                dest_slot,
                moving,
                default_item_max_stack_sizes,
            );
            let amount = moving.count.min(max_stack_size);
            if amount <= 0 {
                continue;
            }
            let slot = &mut slots[dest_index];
            move_stack_count(moving, &mut slot.item, amount);
            normalize_container_slot_selection(slot);
            changed = true;
            break;
        }
    }

    changed
}

pub(super) fn player_inventory_merge_slot_order(
    slots: PlayerInventoryAddSlots,
    selected_hotbar_slot: u8,
) -> Vec<i16> {
    let selected_hotbar_slot = slots.hotbar_start + i16::from(selected_hotbar_slot.min(8));
    let mut order = Vec::with_capacity(38);
    order.push(selected_hotbar_slot);
    if let Some(offhand_slot) = slots.offhand_slot {
        order.push(offhand_slot);
    }
    order.extend(slots.hotbar_start..slots.hotbar_end);
    order.extend(slots.main_start..slots.main_end);
    order
}

pub(super) fn player_inventory_free_slot_order(slots: PlayerInventoryAddSlots) -> Vec<i16> {
    let mut order = Vec::with_capacity(36);
    order.extend(slots.hotbar_start..slots.hotbar_end);
    order.extend(slots.main_start..slots.main_end);
    order
}

pub(super) fn inventory_menu_predictable_input_slot_nums(
    slots: &[ContainerSlot],
    default_item_crafting_remainders_known: bool,
    default_item_crafting_remainders: &BTreeMap<i32, i32>,
    recipe_specific_crafting_remainder_item_ids: &BTreeSet<i32>,
) -> Option<Vec<i16>> {
    crafting_result_predictable_input_slot_nums(
        slots,
        1,
        5,
        default_item_crafting_remainders_known,
        default_item_crafting_remainders,
        recipe_specific_crafting_remainder_item_ids,
    )
}

pub(super) fn sync_inventory_menu_crafting_result_from_recipe_book(
    recipes: &BTreeMap<i32, ProtocolRecipeDisplayEntry>,
    item_tags: Option<&RegistryTagState>,
    slots: &mut [ContainerSlot],
) {
    let Some(result_index) = slots.iter().position(|slot| slot.slot == 0) else {
        return;
    };
    let result = predict_inventory_menu_crafting_result(recipes, item_tags, slots)
        .unwrap_or_else(ProtocolItemStackSummary::empty);
    slots[result_index].item = result;
    normalize_container_slot_selection(&mut slots[result_index]);
}

pub(super) fn predict_inventory_menu_crafting_result(
    recipes: &BTreeMap<i32, ProtocolRecipeDisplayEntry>,
    item_tags: Option<&RegistryTagState>,
    slots: &[ContainerSlot],
) -> Option<ProtocolItemStackSummary> {
    for recipe in recipes.values() {
        let Some(crafting) = &recipe.display.crafting else {
            continue;
        };
        let result = match crafting {
            ProtocolCraftingRecipeDisplaySummary::Shapeless {
                ingredients,
                result,
                ..
            } => {
                if let Some(requirements) = recipe.crafting_requirements.as_deref() {
                    predict_shapeless_inventory_menu_result_from_requirements(
                        requirements,
                        result,
                        item_tags,
                        slots,
                    )
                } else {
                    predict_shapeless_inventory_menu_result(ingredients, result, slots)
                }
            }
            ProtocolCraftingRecipeDisplaySummary::Shaped {
                width,
                height,
                ingredients,
                result,
                ..
            } => {
                if let Some(requirements) = recipe.crafting_requirements.as_deref() {
                    predict_shaped_inventory_menu_result_from_requirements(
                        *width,
                        *height,
                        ingredients,
                        requirements,
                        result,
                        item_tags,
                        slots,
                    )
                } else {
                    predict_shaped_inventory_menu_result(
                        *width,
                        *height,
                        ingredients,
                        result,
                        slots,
                    )
                }
            }
        };
        if result.is_some() {
            return result;
        }
    }
    None
}

pub(super) fn predict_shapeless_inventory_menu_result(
    ingredients: &[bbb_protocol::packets::SlotDisplaySummary],
    result: &bbb_protocol::packets::SlotDisplaySummary,
    slots: &[ContainerSlot],
) -> Option<ProtocolItemStackSummary> {
    if ingredients.is_empty() || ingredients.len() > 4 {
        return None;
    }
    let ingredients = simple_slot_display_item_ids(
        ingredients
            .iter()
            .map(simple_slot_display_item_id)
            .collect::<Option<Vec<_>>>()?,
    );
    let input_item_ids = inventory_menu_crafting_input_item_ids(slots)?;
    if !shapeless_crafting_ingredients_match(&ingredients, &input_item_ids, None) {
        return None;
    }
    simple_slot_display_item_stack(result)
}

pub(super) fn predict_shapeless_inventory_menu_result_from_requirements(
    requirements: &[ProtocolIngredientSummary],
    result: &ProtocolSlotDisplaySummary,
    item_tags: Option<&RegistryTagState>,
    slots: &[ContainerSlot],
) -> Option<ProtocolItemStackSummary> {
    if requirements.is_empty() || requirements.len() > 4 {
        return None;
    }
    let ingredients = requirements
        .iter()
        .map(CraftingIngredientMatch::Requirement)
        .collect::<Vec<_>>();
    let input_item_ids = inventory_menu_crafting_input_item_ids(slots)?;
    if !shapeless_crafting_ingredients_match(&ingredients, &input_item_ids, item_tags) {
        return None;
    }
    simple_slot_display_item_stack(result)
}

pub(super) fn predict_shaped_inventory_menu_result(
    width: i32,
    height: i32,
    ingredients: &[ProtocolSlotDisplaySummary],
    result: &ProtocolSlotDisplaySummary,
    slots: &[ContainerSlot],
) -> Option<ProtocolItemStackSummary> {
    if width <= 0 || height <= 0 || width > 2 || height > 2 {
        return None;
    }
    let width = usize::try_from(width).ok()?;
    let height = usize::try_from(height).ok()?;
    if ingredients.len() != width * height {
        return None;
    }
    let ingredients = simple_shaped_slot_display_ingredients(ingredients)?;
    let grid = inventory_menu_crafting_grid_item_ids(slots);
    for offset_y in 0..=(2 - height) {
        for offset_x in 0..=(2 - width) {
            for x_flipped in [false, true] {
                if shaped_inventory_menu_recipe_matches(
                    width,
                    height,
                    &ingredients,
                    &grid,
                    offset_x,
                    offset_y,
                    x_flipped,
                    None,
                ) {
                    return simple_slot_display_item_stack(result);
                }
            }
        }
    }
    None
}

pub(super) fn predict_shaped_inventory_menu_result_from_requirements(
    width: i32,
    height: i32,
    displays: &[ProtocolSlotDisplaySummary],
    requirements: &[ProtocolIngredientSummary],
    result: &ProtocolSlotDisplaySummary,
    item_tags: Option<&RegistryTagState>,
    slots: &[ContainerSlot],
) -> Option<ProtocolItemStackSummary> {
    if width <= 0 || height <= 0 || width > 2 || height > 2 {
        return None;
    }
    let width = usize::try_from(width).ok()?;
    let height = usize::try_from(height).ok()?;
    if displays.len() != width * height {
        return None;
    }
    let ingredients = shaped_requirement_ingredients(displays, requirements)?;
    let grid = inventory_menu_crafting_grid_item_ids(slots);
    for offset_y in 0..=(2 - height) {
        for offset_x in 0..=(2 - width) {
            for x_flipped in [false, true] {
                if shaped_inventory_menu_recipe_matches(
                    width,
                    height,
                    &ingredients,
                    &grid,
                    offset_x,
                    offset_y,
                    x_flipped,
                    item_tags,
                ) {
                    return simple_slot_display_item_stack(result);
                }
            }
        }
    }
    None
}

pub(super) fn shaped_inventory_menu_recipe_matches(
    width: usize,
    height: usize,
    ingredients: &[CraftingIngredientMatch<'_>],
    grid: &[Option<i32>; 4],
    offset_x: usize,
    offset_y: usize,
    x_flipped: bool,
    item_tags: Option<&RegistryTagState>,
) -> bool {
    for y in 0..2 {
        for x in 0..2 {
            let grid_item_id = grid[x + y * 2];
            let ingredient = if (offset_x..offset_x + width).contains(&x)
                && (offset_y..offset_y + height).contains(&y)
            {
                let recipe_x = x - offset_x;
                let ingredient_x = if x_flipped {
                    width - recipe_x - 1
                } else {
                    recipe_x
                };
                ingredients[ingredient_x + (y - offset_y) * width]
            } else {
                CraftingIngredientMatch::Empty
            };
            match ingredient {
                CraftingIngredientMatch::Empty if grid_item_id.is_none() => {}
                CraftingIngredientMatch::Empty => return false,
                _ => {
                    let Some(item_id) = grid_item_id else {
                        return false;
                    };
                    if !ingredient.accepts_item(item_id, item_tags) {
                        return false;
                    }
                }
            }
        }
    }
    true
}

#[derive(Clone, Copy)]
pub(super) enum CraftingIngredientMatch<'a> {
    Empty,
    OneItem(i32),
    Requirement(&'a ProtocolIngredientSummary),
}

impl<'a> CraftingIngredientMatch<'a> {
    fn accepts_item(self, item_id: i32, item_tags: Option<&RegistryTagState>) -> bool {
        match self {
            Self::Empty => false,
            Self::OneItem(expected) => expected == item_id,
            Self::Requirement(requirement) => {
                ingredient_accepts_item(requirement, item_id, item_tags)
            }
        }
    }
}

pub(super) fn simple_slot_display_item_ids(
    item_ids: Vec<i32>,
) -> Vec<CraftingIngredientMatch<'static>> {
    item_ids
        .into_iter()
        .map(CraftingIngredientMatch::OneItem)
        .collect()
}

pub(super) fn simple_shaped_slot_display_ingredients(
    displays: &[ProtocolSlotDisplaySummary],
) -> Option<Vec<CraftingIngredientMatch<'static>>> {
    displays
        .iter()
        .map(|display| {
            if display.display_type_id == 0 {
                Some(CraftingIngredientMatch::Empty)
            } else {
                Some(CraftingIngredientMatch::OneItem(
                    simple_slot_display_item_id(display)?,
                ))
            }
        })
        .collect()
}

pub(super) fn shaped_requirement_ingredients<'a>(
    displays: &[ProtocolSlotDisplaySummary],
    requirements: &'a [ProtocolIngredientSummary],
) -> Option<Vec<CraftingIngredientMatch<'a>>> {
    let mut requirements = requirements.iter();
    let mut ingredients = Vec::with_capacity(displays.len());
    for display in displays {
        if display.display_type_id == 0 {
            ingredients.push(CraftingIngredientMatch::Empty);
        } else {
            ingredients.push(CraftingIngredientMatch::Requirement(requirements.next()?));
        }
    }
    if requirements.next().is_some() {
        return None;
    }
    Some(ingredients)
}

pub(super) fn simple_slot_display_item_id(
    display: &bbb_protocol::packets::SlotDisplaySummary,
) -> Option<i32> {
    display.item_stack.as_ref()?.item_id
}

pub(super) fn simple_slot_display_item_stack(
    display: &bbb_protocol::packets::SlotDisplaySummary,
) -> Option<ProtocolItemStackSummary> {
    let stack = display.item_stack.clone()?;
    item_stack_is_non_empty(&stack).then_some(stack)
}

pub(super) fn simple_item_stack(item_id: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: ProtocolDataComponentPatchSummary::default(),
    }
}

pub(super) fn shapeless_crafting_ingredients_match(
    ingredients: &[CraftingIngredientMatch<'_>],
    input_item_ids: &[i32],
    item_tags: Option<&RegistryTagState>,
) -> bool {
    if ingredients.len() != input_item_ids.len() {
        return false;
    }
    let mut used_inputs = vec![false; input_item_ids.len()];
    shapeless_crafting_ingredients_match_from(
        ingredients,
        input_item_ids,
        item_tags,
        &mut used_inputs,
        0,
    )
}

pub(super) fn shapeless_crafting_ingredients_match_from(
    ingredients: &[CraftingIngredientMatch<'_>],
    input_item_ids: &[i32],
    item_tags: Option<&RegistryTagState>,
    used_inputs: &mut [bool],
    ingredient_index: usize,
) -> bool {
    let Some(ingredient) = ingredients.get(ingredient_index) else {
        return true;
    };
    if matches!(ingredient, CraftingIngredientMatch::Empty) {
        return false;
    }
    for input_index in 0..input_item_ids.len() {
        if used_inputs[input_index] {
            continue;
        }
        if !ingredient.accepts_item(input_item_ids[input_index], item_tags) {
            continue;
        }
        used_inputs[input_index] = true;
        if shapeless_crafting_ingredients_match_from(
            ingredients,
            input_item_ids,
            item_tags,
            used_inputs,
            ingredient_index + 1,
        ) {
            return true;
        }
        used_inputs[input_index] = false;
    }
    false
}

pub(super) fn ingredient_accepts_item(
    ingredient: &ProtocolIngredientSummary,
    item_id: i32,
    item_tags: Option<&RegistryTagState>,
) -> bool {
    ingredient.item_ids.contains(&item_id)
        || ingredient.tag.as_deref().is_some_and(|tag| {
            item_tags
                .and_then(|registry| registry.tags.get(tag))
                .is_some_and(|entries| entries.contains(&item_id))
        })
}

pub(super) fn inventory_menu_crafting_input_item_ids(slots: &[ContainerSlot]) -> Option<Vec<i32>> {
    let mut item_ids = Vec::new();
    for slot_num in 1..5 {
        let Some(item) = container_slot_item(slots, slot_num) else {
            continue;
        };
        if item_stack_is_empty(item) {
            continue;
        }
        item_ids.push(item.item_id?);
    }
    Some(item_ids)
}

pub(super) fn inventory_menu_crafting_grid_item_ids(slots: &[ContainerSlot]) -> [Option<i32>; 4] {
    std::array::from_fn(|index| {
        let slot_num = 1 + i16::try_from(index).ok()?;
        let item = container_slot_item(slots, slot_num)?;
        item_stack_is_non_empty(item)
            .then_some(item.item_id)
            .flatten()
    })
}

pub(super) fn crafting_result_predictable_input_slot_nums(
    slots: &[ContainerSlot],
    start_slot: i16,
    end_slot: i16,
    default_item_crafting_remainders_known: bool,
    _default_item_crafting_remainders: &BTreeMap<i32, i32>,
    recipe_specific_crafting_remainder_item_ids: &BTreeSet<i32>,
) -> Option<Vec<i16>> {
    if !default_item_crafting_remainders_known {
        return None;
    }
    let input_slot_nums = non_empty_slot_nums(slots, start_slot, end_slot);
    let can_predict = !input_slot_nums.is_empty()
        && input_slot_nums.iter().all(|slot_num| {
            slots
                .iter()
                .find(|slot| slot.slot == *slot_num)
                .is_some_and(|slot| {
                    let item_id = slot.item.item_id;
                    item_stack_is_non_empty(&slot.item)
                        && item_id.is_some()
                        && slot.item.count > 0
                        && !item_id.is_some_and(|item_id| {
                            recipe_specific_crafting_remainder_item_ids.contains(&item_id)
                        })
                })
        });
    can_predict.then_some(input_slot_nums)
}

pub(super) fn container_slot_item(
    slots: &[ContainerSlot],
    slot_num: i16,
) -> Option<&ProtocolItemStackSummary> {
    slots
        .iter()
        .find(|slot| slot.slot == slot_num)
        .map(|slot| &slot.item)
}

pub(super) fn apply_pickup_click_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    slot_num: i16,
    button_num: i8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    match slot_num {
        -999 => {
            apply_outside_pickup_click(cursor, button_num);
        }
        slot_num if slot_num >= 0 => {
            let Some(slot) = slots.iter_mut().find(|slot| slot.slot == slot_num) else {
                return;
            };
            apply_slot_pickup_click(
                container_id,
                slot_num,
                &mut slot.item,
                cursor,
                button_num,
                default_item_max_stack_sizes,
            );
            slot.local_selected_bundle_item_index = normalize_local_selected_bundle_item_index(
                slot.local_selected_bundle_item_index,
                &slot.item,
            );
        }
        _ => {}
    }
}

pub(super) fn apply_quick_craft_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    quick_craft: &mut LocalQuickCraftState,
    slot_num: i16,
    button_num: i8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if container_id != INVENTORY_MENU_CONTAINER_ID {
        return;
    }

    let expected_status = quick_craft.status;
    quick_craft.status = quickcraft_header(button_num);
    if (expected_status != QUICKCRAFT_HEADER_CONTINUE
        || quick_craft.status != QUICKCRAFT_HEADER_END)
        && expected_status != quick_craft.status
    {
        quick_craft.reset();
        return;
    }
    if item_stack_is_empty(cursor) {
        quick_craft.reset();
        return;
    }

    match quick_craft.status {
        QUICKCRAFT_HEADER_START => {
            quick_craft.quickcraft_type = quickcraft_type(button_num);
            if local_survival_quickcraft_type_is_valid(quick_craft.quickcraft_type) {
                quick_craft.status = QUICKCRAFT_HEADER_CONTINUE;
                quick_craft.slots.clear();
            } else {
                quick_craft.reset();
            }
        }
        QUICKCRAFT_HEADER_CONTINUE => {
            let Some(slot) = slots.iter().find(|slot| slot.slot == slot_num) else {
                return;
            };
            if quick_craft_slot_can_accept(
                container_id,
                slot_num,
                &slot.item,
                cursor,
                default_item_max_stack_sizes,
            ) && cursor.count > quick_craft.slots.len() as i32
                && !quick_craft.slots.contains(&slot_num)
            {
                quick_craft.slots.push(slot_num);
            }
        }
        QUICKCRAFT_HEADER_END => {
            finish_quick_craft(
                container_id,
                slots,
                cursor,
                quick_craft,
                default_item_max_stack_sizes,
            );
        }
        _ => quick_craft.reset(),
    }
}

pub(super) fn apply_clone_click_to_slots(
    slots: &[ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    slot_num: i16,
    instabuild: bool,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !instabuild || !item_stack_is_empty(cursor) || slot_num < 0 {
        return;
    }
    let Some(slot_item) = container_slot_item(slots, slot_num) else {
        return;
    };
    if item_stack_is_empty(slot_item) {
        return;
    }

    *cursor = slot_item.clone();
    cursor.count = item_stack_max_stack_size(slot_item, default_item_max_stack_sizes);
    normalize_item_stack(cursor);
}

pub(super) fn finish_quick_craft(
    container_id: i32,
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    quick_craft: &mut LocalQuickCraftState,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    let selected_slots = quick_craft.slots.clone();
    if selected_slots.is_empty() {
        quick_craft.reset();
        return;
    }

    let quickcraft_type = quick_craft.quickcraft_type;
    if selected_slots.len() == 1 {
        quick_craft.reset();
        apply_pickup_click_to_slots(
            container_id,
            slots,
            cursor,
            selected_slots[0],
            quickcraft_type,
            default_item_max_stack_sizes,
        );
        return;
    }
    if !local_survival_quickcraft_type_is_valid(quickcraft_type) {
        quick_craft.reset();
        return;
    }

    let source = cursor.clone();
    if item_stack_is_empty(&source) {
        quick_craft.reset();
        return;
    }

    let slot_count = selected_slots.len() as i32;
    let mut remaining = cursor.count;
    for selected_slot in selected_slots {
        if cursor.count < slot_count {
            continue;
        }
        let Some(slot_index) = slots.iter().position(|slot| slot.slot == selected_slot) else {
            continue;
        };
        if !quick_craft_slot_can_accept(
            container_id,
            selected_slot,
            &slots[slot_index].item,
            cursor,
            default_item_max_stack_sizes,
        ) {
            continue;
        }

        let carry = if item_stack_is_empty(&slots[slot_index].item) {
            0
        } else {
            slots[slot_index].item.count
        };
        let max_size = item_stack_max_stack_size(&source, default_item_max_stack_sizes).min(
            container_slot_max_stack_size(
                container_id,
                selected_slot,
                &source,
                default_item_max_stack_sizes,
            ),
        );
        let place_count = quickcraft_place_count(
            slot_count,
            quickcraft_type,
            &source,
            default_item_max_stack_sizes,
        );
        let new_count = (place_count + carry).min(max_size);
        remaining -= new_count - carry;

        let mut replacement = source.clone();
        replacement.count = new_count;
        normalize_item_stack(&mut replacement);
        slots[slot_index].item = replacement;
        normalize_container_slot_selection(&mut slots[slot_index]);
    }

    cursor.count = remaining;
    normalize_item_stack(cursor);
    quick_craft.reset();
}

pub(super) fn quick_craft_slot_can_accept(
    container_id: i32,
    slot_num: i16,
    slot_item: &ProtocolItemStackSummary,
    cursor: &ProtocolItemStackSummary,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    if container_slot_max_stack_size(container_id, slot_num, cursor, default_item_max_stack_sizes)
        <= 0
    {
        return false;
    }
    if item_stack_is_empty(slot_item) {
        return true;
    }
    same_item_same_components(slot_item, cursor)
        && slot_item.count <= item_stack_max_stack_size(cursor, default_item_max_stack_sizes)
}

pub(super) fn quickcraft_place_count(
    slot_count: i32,
    quickcraft_type: i8,
    source: &ProtocolItemStackSummary,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> i32 {
    match quickcraft_type {
        QUICKCRAFT_TYPE_CHARITABLE => source.count / slot_count,
        QUICKCRAFT_TYPE_GREEDY => 1,
        QUICKCRAFT_TYPE_CLONE => item_stack_max_stack_size(source, default_item_max_stack_sizes),
        _ => source.count,
    }
}

pub(super) fn local_survival_quickcraft_type_is_valid(quickcraft_type: i8) -> bool {
    matches!(
        quickcraft_type,
        QUICKCRAFT_TYPE_CHARITABLE | QUICKCRAFT_TYPE_GREEDY
    )
}

pub(super) fn quickcraft_header(mask: i8) -> i8 {
    mask & 3
}

pub(super) fn quickcraft_type(mask: i8) -> i8 {
    (mask >> 2) & 3
}

pub(super) fn apply_throw_click_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &ProtocolItemStackSummary,
    slot_num: i16,
    button_num: i8,
) {
    if !item_stack_is_empty(cursor) || slot_num < 0 || !matches!(button_num, 0 | 1) {
        return;
    }
    let Some(slot) = slots.iter_mut().find(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slot.item) {
        return;
    }
    if button_num == 0 {
        slot.item.count -= 1;
        normalize_item_stack(&mut slot.item);
    } else {
        slot.item = ProtocolItemStackSummary::empty();
    }
    normalize_container_slot_selection(slot);
}

pub(super) fn apply_pickup_all_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    slot_num: i16,
    button_num: i8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if slot_num < 0 || item_stack_is_empty(cursor) {
        return;
    }
    let Some(clicked_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if !item_stack_is_empty(&slots[clicked_index].item) {
        return;
    }

    let max_stack_size = item_stack_max_stack_size(cursor, default_item_max_stack_sizes);
    if cursor.count >= max_stack_size {
        return;
    }

    let mut indices = (0..slots.len()).collect::<Vec<_>>();
    if button_num != 0 {
        indices.reverse();
    }

    for pass in 0..2 {
        for index in indices.iter().copied() {
            if cursor.count >= max_stack_size {
                return;
            }
            let slot = &mut slots[index];
            if item_stack_is_empty(&slot.item) || !same_item_same_components(&slot.item, cursor) {
                continue;
            }
            if pass == 0
                && slot.item.count
                    == item_stack_max_stack_size(&slot.item, default_item_max_stack_sizes)
            {
                continue;
            }

            let moved = slot.item.count.min(max_stack_size - cursor.count);
            if moved <= 0 {
                continue;
            }
            slot.item.count -= moved;
            cursor.count += moved;
            normalize_item_stack(&mut slot.item);
            normalize_container_slot_selection(slot);
        }
    }
}

pub(super) fn apply_swap_click_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    cursor: &ProtocolItemStackSummary,
    slot_num: i16,
    button_num: i8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if container_id != INVENTORY_MENU_CONTAINER_ID || !item_stack_is_empty(cursor) || slot_num < 0 {
        return;
    }
    let Some(source_slot_num) = swap_button_inventory_menu_slot(button_num) else {
        return;
    };
    if source_slot_num == slot_num {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == source_slot_num) else {
        return;
    };
    let Some(target_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };

    let source_item = slots[source_index].item.clone();
    let target_item = slots[target_index].item.clone();
    if item_stack_is_empty(&source_item) && item_stack_is_empty(&target_item) {
        return;
    }

    if item_stack_is_empty(&source_item) {
        if container_slot_max_stack_size(
            container_id,
            source_slot_num,
            &target_item,
            default_item_max_stack_sizes,
        ) <= 0
        {
            return;
        }
        slots[source_index].item = target_item;
        slots[target_index].item = ProtocolItemStackSummary::empty();
        normalize_container_slot_selection(&mut slots[source_index]);
        normalize_container_slot_selection(&mut slots[target_index]);
        return;
    }

    let target_max_stack_size = container_slot_max_stack_size(
        container_id,
        slot_num,
        &source_item,
        default_item_max_stack_sizes,
    );
    if target_max_stack_size <= 0 {
        return;
    }

    if item_stack_is_empty(&target_item) {
        move_between_container_slots(slots, source_index, target_index, target_max_stack_size);
        return;
    }

    let source_max_stack_size = container_slot_max_stack_size(
        container_id,
        source_slot_num,
        &target_item,
        default_item_max_stack_sizes,
    );
    if source_max_stack_size <= 0 {
        return;
    }

    if source_item.count <= target_max_stack_size && target_item.count <= source_max_stack_size {
        slots[source_index].item = target_item;
        slots[target_index].item = source_item;
        normalize_container_slot_selection(&mut slots[source_index]);
        normalize_container_slot_selection(&mut slots[target_index]);
        return;
    }

    let moved = source_item.count.min(target_max_stack_size);
    let mut target_replacement = source_item.clone();
    target_replacement.count = moved;
    let mut source_remainder = source_item;
    source_remainder.count -= moved;
    normalize_item_stack(&mut source_remainder);
    slots[source_index].item = source_remainder;
    slots[target_index].item = target_replacement;
    normalize_container_slot_selection(&mut slots[source_index]);
    normalize_container_slot_selection(&mut slots[target_index]);

    let mut displaced = target_item;
    move_item_stack_to_slots(
        container_id,
        slots,
        target_index,
        &mut displaced,
        INVENTORY_MENU_MAIN_START,
        INVENTORY_MENU_HOTBAR_END,
        false,
        default_item_max_stack_sizes,
    );
}

pub(super) fn move_between_container_slots(
    slots: &mut [ContainerSlot],
    source_index: usize,
    target_index: usize,
    max_count: i32,
) {
    let mut source = slots[source_index].item.clone();
    let amount = source.count.min(max_count);
    let mut target = source.clone();
    target.count = amount;
    source.count -= amount;
    normalize_item_stack(&mut source);
    slots[source_index].item = source;
    slots[target_index].item = target;
    normalize_container_slot_selection(&mut slots[source_index]);
    normalize_container_slot_selection(&mut slots[target_index]);
}

pub(super) fn swap_button_inventory_menu_slot(button_num: i8) -> Option<i16> {
    match button_num {
        0..=8 => Some(INVENTORY_MENU_HOTBAR_START + i16::from(button_num)),
        40 => Some(INVENTORY_MENU_OFFHAND_SLOT),
        _ => None,
    }
}

pub(super) fn generic_9x_container_slot_count(menu_type_id: Option<i32>) -> Option<i16> {
    let menu_type_id = menu_type_id?;
    (VANILLA_MENU_TYPE_GENERIC_9X1_ID..=VANILLA_MENU_TYPE_GENERIC_9X6_ID)
        .contains(&menu_type_id)
        .then_some((menu_type_id - VANILLA_MENU_TYPE_GENERIC_9X1_ID + 1) as i16)
        .map(|rows| rows * GENERIC_CONTAINER_SLOT_COUNT_PER_ROW)
}

pub(super) fn generic_3x3_container_slot_count(menu_type_id: Option<i32>) -> Option<i16> {
    (menu_type_id == Some(VANILLA_MENU_TYPE_GENERIC_3X3_ID))
        .then_some(GENERIC_3X3_CONTAINER_SLOT_COUNT)
}

pub(super) fn hopper_container_slot_count(menu_type_id: Option<i32>) -> Option<i16> {
    (menu_type_id == Some(VANILLA_MENU_TYPE_HOPPER_ID)).then_some(HOPPER_CONTAINER_SLOT_COUNT)
}

pub(super) fn shulker_box_container_slot_count(menu_type_id: Option<i32>) -> Option<i16> {
    (menu_type_id == Some(VANILLA_MENU_TYPE_SHULKER_BOX_ID))
        .then_some(SHULKER_BOX_CONTAINER_SLOT_COUNT)
}

pub(super) fn menu_result_slot_requires_server_authority(
    menu_type_id: Option<i32>,
    slot_num: i16,
    input: ProtocolContainerInput,
) -> bool {
    if !matches!(
        input,
        ProtocolContainerInput::Pickup | ProtocolContainerInput::QuickMove
    ) {
        return false;
    }
    if matches!(
        (menu_type_id, slot_num, input),
        (
            Some(VANILLA_MENU_TYPE_ANVIL_ID),
            ANVIL_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_ANVIL_ID),
            ANVIL_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        ) | (
            Some(VANILLA_MENU_TYPE_CRAFTING_ID),
            CRAFTING_MENU_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_CRAFTING_ID),
            CRAFTING_MENU_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        ) | (
            Some(VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID),
            CARTOGRAPHY_TABLE_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID),
            CARTOGRAPHY_TABLE_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        ) | (
            Some(VANILLA_MENU_TYPE_STONECUTTER_ID),
            STONECUTTER_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_STONECUTTER_ID),
            STONECUTTER_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        ) | (
            Some(VANILLA_MENU_TYPE_GRINDSTONE_ID),
            GRINDSTONE_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_GRINDSTONE_ID),
            GRINDSTONE_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        ) | (
            Some(VANILLA_MENU_TYPE_LOOM_ID),
            LOOM_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_LOOM_ID),
            LOOM_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        ) | (
            Some(VANILLA_MENU_TYPE_SMITHING_ID),
            SMITHING_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_SMITHING_ID),
            SMITHING_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        ) | (
            Some(VANILLA_MENU_TYPE_MERCHANT_ID),
            MERCHANT_RESULT_SLOT,
            ProtocolContainerInput::QuickMove
        ) | (
            Some(VANILLA_MENU_TYPE_MERCHANT_ID),
            MERCHANT_RESULT_SLOT,
            ProtocolContainerInput::Pickup
        )
    ) {
        return false;
    }
    matches!(
        (menu_type_id, slot_num),
        (Some(VANILLA_MENU_TYPE_ANVIL_ID), ANVIL_RESULT_SLOT)
            | (
                Some(VANILLA_MENU_TYPE_CRAFTING_ID),
                CRAFTING_MENU_RESULT_SLOT
            )
            | (
                Some(VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID),
                CARTOGRAPHY_TABLE_RESULT_SLOT
            )
            | (Some(VANILLA_MENU_TYPE_CRAFTER_ID), CRAFTER_RESULT_SLOT)
            | (Some(VANILLA_MENU_TYPE_LOOM_ID), LOOM_RESULT_SLOT)
            | (
                Some(VANILLA_MENU_TYPE_GRINDSTONE_ID),
                GRINDSTONE_RESULT_SLOT
            )
            | (Some(VANILLA_MENU_TYPE_SMITHING_ID), SMITHING_RESULT_SLOT)
            | (Some(VANILLA_MENU_TYPE_MERCHANT_ID), MERCHANT_RESULT_SLOT)
            | (
                Some(VANILLA_MENU_TYPE_STONECUTTER_ID),
                STONECUTTER_RESULT_SLOT
            )
    )
}

pub(super) fn furnace_family_menu_type(menu_type_id: Option<i32>) -> Option<i32> {
    let menu_type_id = menu_type_id?;
    matches!(
        menu_type_id,
        VANILLA_MENU_TYPE_BLAST_FURNACE_ID
            | VANILLA_MENU_TYPE_FURNACE_ID
            | VANILLA_MENU_TYPE_SMOKER_ID
    )
    .then_some(menu_type_id)
}

pub(super) fn apply_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    default_item_equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if container_id != INVENTORY_MENU_CONTAINER_ID || slot_num < 0 {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }
    let source_item = slots[source_index].item.clone();
    let Some((start_slot, end_slot, backwards)) = inventory_menu_quick_move_target_range(
        slot_num,
        &source_item,
        slots,
        default_item_equipment_slots,
    ) else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

pub(super) fn apply_mount_inventory_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    mount_equipment_slots: Option<MountEquipmentSlotVisibility>,
    default_item_equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
    default_mount_body_armor_kinds: &BTreeMap<i32, MountArmorSlotKind>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }
    let Some(player_start) = mount_inventory_player_start_slot(slots) else {
        return;
    };
    let player_end = player_start + MOUNT_PLAYER_SLOT_COUNT;
    if !(0..player_end).contains(&slot_num) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let mut moving = source_item.clone();
    let mut changed = false;
    if slot_num < player_start {
        changed = move_item_stack_to_slots(
            container_id,
            slots,
            source_index,
            &mut moving,
            player_start,
            player_end,
            true,
            default_item_max_stack_sizes,
        );
    } else {
        if let Some(target_slot) = mount_equipment_quick_move_target(
            &source_item,
            slots,
            mount_equipment_slots,
            default_item_equipment_slots,
            default_mount_body_armor_kinds,
        ) {
            changed = move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                target_slot,
                target_slot + 1,
                false,
                default_item_max_stack_sizes,
            );
        }

        if !changed && player_start > MOUNT_INVENTORY_START {
            changed = move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                MOUNT_INVENTORY_START,
                player_start,
                false,
                default_item_max_stack_sizes,
            );
        }

        if !changed {
            let player_main_end = player_start + MOUNT_PLAYER_MAIN_SLOT_COUNT;
            let target = if (player_main_end..player_end).contains(&slot_num) {
                Some((player_start, player_main_end))
            } else if (player_start..player_main_end).contains(&slot_num) {
                Some((player_main_end, player_end))
            } else {
                None
            };
            if let Some((start_slot, end_slot)) = target {
                changed = move_item_stack_to_slots(
                    container_id,
                    slots,
                    source_index,
                    &mut moving,
                    start_slot,
                    end_slot,
                    false,
                    default_item_max_stack_sizes,
                );
            }
        }
    }

    if changed {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

pub(super) fn mount_inventory_quick_move_requires_server_authority(
    slots: &[ContainerSlot],
    slot_num: i16,
    mount_equipment_slots: Option<MountEquipmentSlotVisibility>,
    default_item_equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
    default_mount_body_armor_kinds: &BTreeMap<i32, MountArmorSlotKind>,
) -> bool {
    let Some(player_start) = mount_inventory_player_start_slot(slots) else {
        return false;
    };
    let player_end = player_start + MOUNT_PLAYER_SLOT_COUNT;
    if !(player_start..player_end).contains(&slot_num) {
        return false;
    }
    let Some(source) = slots.iter().find(|slot| slot.slot == slot_num) else {
        return false;
    };
    if item_stack_is_empty(&source.item) {
        return false;
    }
    if default_item_equipment_slots.is_empty() || mount_equipment_slots.is_none() {
        return true;
    }
    if item_stack_has_component_patch(&source.item) {
        return true;
    }

    if let Some(item_id) = source.item.item_id {
        if default_item_equipment_slots
            .get(&item_id)
            .is_some_and(|slot| *slot == ItemEquipmentSlot::Body)
            && !default_mount_body_armor_kinds.contains_key(&item_id)
        {
            return true;
        }
    }

    false
}

pub(super) fn mount_inventory_player_start_slot(slots: &[ContainerSlot]) -> Option<i16> {
    let slot_count = i16::try_from(slots.len()).ok()?;
    (slot_count >= MOUNT_INVENTORY_START + MOUNT_PLAYER_SLOT_COUNT)
        .then_some(slot_count - MOUNT_PLAYER_SLOT_COUNT)
}

pub(super) fn mount_equipment_quick_move_target(
    stack: &ProtocolItemStackSummary,
    slots: &[ContainerSlot],
    mount_equipment_slots: Option<MountEquipmentSlotVisibility>,
    default_item_equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
    default_mount_body_armor_kinds: &BTreeMap<i32, MountArmorSlotKind>,
) -> Option<i16> {
    let item_id = stack.item_id?;
    let visibility = mount_equipment_slots?;
    match default_item_equipment_slots.get(&item_id).copied()? {
        ItemEquipmentSlot::Body
            if visibility
                .body
                .zip(default_mount_body_armor_kinds.get(&item_id).copied())
                .is_some_and(|(slot_kind, item_kind)| slot_kind == item_kind)
                && !inventory_menu_slot_has_item(slots, MOUNT_BODY_ARMOR_SLOT) =>
        {
            Some(MOUNT_BODY_ARMOR_SLOT)
        }
        ItemEquipmentSlot::Saddle
            if visibility.saddle && !inventory_menu_slot_has_item(slots, MOUNT_SADDLE_SLOT) =>
        {
            Some(MOUNT_SADDLE_SLOT)
        }
        _ => None,
    }
}

pub(super) fn apply_inventory_menu_result_quick_move_to_slots(
    slots: &mut [ContainerSlot],
    default_item_crafting_remainders: &BTreeMap<i32, i32>,
    selected_hotbar_slot: u8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    let Some(result_index) = slots.iter().position(|slot| slot.slot == 0) else {
        return;
    };
    if item_stack_is_empty(&slots[result_index].item) {
        return;
    }

    let result_template = slots[result_index].item.clone();
    let input_slot_nums = inventory_menu_non_empty_crafting_slot_nums(slots);
    if input_slot_nums.is_empty() {
        let mut moving = result_template;
        if move_item_stack_to_slots(
            INVENTORY_MENU_CONTAINER_ID,
            slots,
            result_index,
            &mut moving,
            INVENTORY_MENU_MAIN_START,
            INVENTORY_MENU_HOTBAR_END,
            true,
            default_item_max_stack_sizes,
        ) {
            normalize_item_stack(&mut moving);
            slots[result_index].item = moving;
            normalize_container_slot_selection(&mut slots[result_index]);
        }
        return;
    }

    let max_crafts = input_slot_nums
        .iter()
        .filter_map(|slot_num| {
            slots
                .iter()
                .find(|slot| slot.slot == *slot_num)
                .map(|slot| slot.item.count)
        })
        .min()
        .unwrap_or(0)
        .max(0);

    for _ in 0..max_crafts {
        let result_still_same = container_slot_item(slots, 0).is_some_and(|item| {
            item_stack_is_non_empty(item) && same_item_same_components(item, &result_template)
        });
        if !result_still_same || !inventory_menu_inputs_can_take_result(slots, &input_slot_nums) {
            break;
        }

        let mut candidate_slots = slots.to_vec();
        candidate_slots[result_index].item = result_template.clone();
        let mut moving = result_template.clone();
        if !move_item_stack_to_slots(
            INVENTORY_MENU_CONTAINER_ID,
            &mut candidate_slots,
            result_index,
            &mut moving,
            INVENTORY_MENU_MAIN_START,
            INVENTORY_MENU_HOTBAR_END,
            true,
            default_item_max_stack_sizes,
        ) {
            break;
        }

        candidate_slots[result_index].item = ProtocolItemStackSummary::empty();
        normalize_container_slot_selection(&mut candidate_slots[result_index]);
        let Some(side_effects) = apply_crafting_result_take_side_effects_for_slots(
            INVENTORY_MENU_CONTAINER_ID,
            &mut candidate_slots,
            &input_slot_nums,
            default_item_crafting_remainders,
            Some(PlayerInventoryAddSlots::inventory_menu()),
            selected_hotbar_slot,
            default_item_max_stack_sizes,
        ) else {
            break;
        };
        if side_effects.inputs_can_still_take_result {
            candidate_slots[result_index].item = result_template.clone();
            normalize_container_slot_selection(&mut candidate_slots[result_index]);
        }
        slots.clone_from_slice(&candidate_slots);
    }
}

pub(super) fn apply_furnace_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    menu_type_id: Option<i32>,
    recipe_property_sets: &BTreeMap<String, Vec<i32>>,
    furnace_fuel_item_ids: &BTreeSet<i32>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if slot_num < 0
        || slot_num >= FURNACE_CONTAINER_SLOT_COUNT + GENERIC_CONTAINER_PLAYER_SLOT_COUNT
    {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        2 => Some((3, 39, true)),
        0 | 1 => Some((3, 39, false)),
        3..=38 if furnace_can_smelt(menu_type_id, &source_item, recipe_property_sets) => {
            Some((0, 1, false))
        }
        3..=38 if furnace_is_fuel(&source_item, furnace_fuel_item_ids) => Some((1, 2, false)),
        3..=29 => Some((30, 39, false)),
        30..=38 => Some((3, 30, false)),
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

pub(super) fn apply_generic_container_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    container_slot_count: i16,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if slot_num < 0 {
        return;
    }
    let total_slot_count = container_slot_count + GENERIC_CONTAINER_PLAYER_SLOT_COUNT;
    if slot_num >= total_slot_count {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }
    let (start_slot, end_slot, backwards) = if slot_num < container_slot_count {
        (container_slot_count, total_slot_count, true)
    } else {
        (0, container_slot_count, false)
    };

    let mut moving = slots[source_index].item.clone();
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

pub(super) fn anvil_quick_move_requires_server_authority(
    _slots: &[ContainerSlot],
    slot_num: i16,
) -> bool {
    if !(0..ANVIL_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return false;
    }
    if slot_num == ANVIL_RESULT_SLOT {
        return false;
    }
    if matches!(slot_num, ANVIL_INPUT_SLOT | ANVIL_ADDITIONAL_SLOT) {
        return false;
    }
    false
}

pub(super) fn apply_anvil_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    result_may_pickup: bool,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..ANVIL_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    if slot_num == ANVIL_RESULT_SLOT {
        apply_anvil_result_quick_move_to_slots(
            container_id,
            slots,
            result_may_pickup,
            default_item_max_stack_sizes,
        );
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        ANVIL_INPUT_SLOT | ANVIL_ADDITIONAL_SLOT => {
            Some((ANVIL_PLAYER_MAIN_START, ANVIL_HOTBAR_END, false))
        }
        slot if (ANVIL_PLAYER_MAIN_START..ANVIL_HOTBAR_END).contains(&slot) => {
            Some((ANVIL_INPUT_SLOT, ANVIL_RESULT_SLOT, false))
        }
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

pub(super) fn apply_anvil_result_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    result_may_pickup: bool,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !result_may_pickup {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == ANVIL_RESULT_SLOT) else {
        return;
    };
    let Some(input_index) = slots.iter().position(|slot| slot.slot == ANVIL_INPUT_SLOT) else {
        return;
    };
    let Some(additional_index) = slots
        .iter()
        .position(|slot| slot.slot == ANVIL_ADDITIONAL_SLOT)
    else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[input_index].item)
        || slots[input_index].item.count != 1
        || !item_stack_is_empty(&slots[additional_index].item)
    {
        return;
    }

    let mut trial = slots.to_vec();
    let mut moving = slots[source_index].item.clone();
    if !move_item_stack_to_slots(
        container_id,
        &mut trial,
        source_index,
        &mut moving,
        ANVIL_PLAYER_MAIN_START,
        ANVIL_HOTBAR_END,
        true,
        default_item_max_stack_sizes,
    ) || !item_stack_is_empty(&moving)
    {
        return;
    }

    trial[input_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[input_index]);
    trial[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[source_index]);
    slots.clone_from_slice(&trial);
}

pub(super) fn apply_anvil_result_pickup_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    button_num: i8,
    result_may_pickup: bool,
) -> bool {
    if !result_may_pickup || button_num != 0 || !item_stack_is_empty(cursor) {
        return false;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == ANVIL_RESULT_SLOT) else {
        return false;
    };
    let Some(input_index) = slots.iter().position(|slot| slot.slot == ANVIL_INPUT_SLOT) else {
        return false;
    };
    let Some(additional_index) = slots
        .iter()
        .position(|slot| slot.slot == ANVIL_ADDITIONAL_SLOT)
    else {
        return false;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[input_index].item)
        || slots[input_index].item.count != 1
        || !item_stack_is_empty(&slots[additional_index].item)
    {
        return false;
    }

    *cursor = slots[source_index].item.clone();
    slots[input_index].item = ProtocolItemStackSummary::empty();
    slots[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut slots[input_index]);
    normalize_container_slot_selection(&mut slots[source_index]);
    true
}

pub(super) fn anvil_result_may_pickup(
    data_values: &[ContainerDataValue],
    abilities: Option<LocalPlayerAbilitiesState>,
    experience: Option<LocalPlayerExperienceState>,
) -> bool {
    let Some(cost) = data_values
        .iter()
        .find_map(|value| (value.id == ANVIL_COST_DATA_ID).then_some(value.value))
    else {
        return false;
    };
    cost > 0
        && (abilities.is_some_and(|abilities| abilities.instabuild)
            || experience.is_some_and(|experience| experience.level >= i32::from(cost)))
}

pub(super) fn set_container_data_value(
    data_values: &mut Vec<ContainerDataValue>,
    id: i16,
    value: i16,
) {
    if let Some(existing) = data_values.iter_mut().find(|value| value.id == id) {
        *existing = ContainerDataValue { id, value };
    } else {
        data_values.push(ContainerDataValue { id, value });
    }
    data_values.sort_by_key(|value| value.id);
}

pub(super) fn beacon_effect_data_value(effect_id: Option<i32>) -> Option<i16> {
    match effect_id {
        Some(effect_id) if effect_id >= 0 => effect_id
            .checked_add(1)
            .and_then(|value| i16::try_from(value).ok()),
        Some(_) => None,
        None => Some(0),
    }
}

pub(super) fn apply_beacon_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    item_tags: Option<&RegistryTagState>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..BEACON_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        BEACON_PAYMENT_SLOT => Some((BEACON_PLAYER_MAIN_START, BEACON_HOTBAR_END, true)),
        slot if (BEACON_PLAYER_MAIN_START..BEACON_HOTBAR_END).contains(&slot)
            && !inventory_menu_slot_has_item(slots, BEACON_PAYMENT_SLOT)
            && source_item.count == 1
            && item_stack_in_item_tag(&source_item, item_tags, BEACON_PAYMENT_ITEM_TAG) =>
        {
            Some((BEACON_PAYMENT_SLOT, BEACON_PAYMENT_SLOT + 1, false))
        }
        slot if (BEACON_PLAYER_MAIN_START..BEACON_PLAYER_MAIN_END).contains(&slot) => {
            Some((BEACON_HOTBAR_START, BEACON_HOTBAR_END, false))
        }
        slot if (BEACON_HOTBAR_START..BEACON_HOTBAR_END).contains(&slot) => {
            Some((BEACON_PLAYER_MAIN_START, BEACON_PLAYER_MAIN_END, false))
        }
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

pub(super) fn enchantment_quick_move_requires_server_authority(
    slots: &[ContainerSlot],
    slot_num: i16,
    enchantment_lapis_lazuli_item_ids: &BTreeSet<i32>,
) -> bool {
    if !(0..ENCHANTMENT_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return false;
    }
    if matches!(slot_num, ENCHANTMENT_INPUT_SLOT | ENCHANTMENT_LAPIS_SLOT) {
        return false;
    }
    let Some(source_item) = container_slot_item(slots, slot_num) else {
        return false;
    };
    if item_stack_is_empty(source_item) {
        return false;
    }
    if enchantment_lapis_lazuli_item_ids.is_empty() {
        return true;
    }
    false
}

pub(super) fn apply_enchantment_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    enchantment_lapis_lazuli_item_ids: &BTreeSet<i32>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..ENCHANTMENT_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        ENCHANTMENT_INPUT_SLOT | ENCHANTMENT_LAPIS_SLOT => {
            Some((ENCHANTMENT_PLAYER_MAIN_START, ENCHANTMENT_HOTBAR_END, true))
        }
        slot if !enchantment_lapis_lazuli_item_ids.is_empty()
            && (ENCHANTMENT_PLAYER_MAIN_START..ENCHANTMENT_HOTBAR_END).contains(&slot)
            && item_stack_item_id_in_set(&source_item, enchantment_lapis_lazuli_item_ids) =>
        {
            Some((ENCHANTMENT_LAPIS_SLOT, ENCHANTMENT_PLAYER_MAIN_START, true))
        }
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        if enchantment_lapis_lazuli_item_ids.is_empty()
            || !(ENCHANTMENT_PLAYER_MAIN_START..ENCHANTMENT_HOTBAR_END).contains(&slot_num)
            || inventory_menu_slot_has_item(slots, ENCHANTMENT_INPUT_SLOT)
        {
            return;
        }
        let Some(input_index) = slots
            .iter()
            .position(|slot| slot.slot == ENCHANTMENT_INPUT_SLOT)
        else {
            return;
        };
        let mut moving = source_item;
        move_stack_count(&mut moving, &mut slots[input_index].item, 1);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
        normalize_container_slot_selection(&mut slots[input_index]);
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

pub(super) fn apply_brewing_stand_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    item_tags: Option<&RegistryTagState>,
    brewing_potion_item_ids: &BTreeSet<i32>,
    brewing_ingredient_item_ids: &BTreeSet<i32>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..BREWING_STAND_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let mut moving = source_item.clone();
    let moved = if (BREWING_STAND_BOTTLE_SLOT_START..=BREWING_STAND_FUEL_SLOT).contains(&slot_num) {
        move_item_stack_to_slots(
            container_id,
            slots,
            source_index,
            &mut moving,
            BREWING_STAND_PLAYER_MAIN_START,
            BREWING_STAND_HOTBAR_END,
            true,
            default_item_max_stack_sizes,
        )
    } else if (BREWING_STAND_PLAYER_MAIN_START..BREWING_STAND_HOTBAR_END).contains(&slot_num) {
        let is_ingredient = item_stack_item_id_in_set(&source_item, brewing_ingredient_item_ids);
        if item_stack_in_item_tag(&source_item, item_tags, BREWING_STAND_FUEL_ITEM_TAG) {
            let fuel_moved = move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                BREWING_STAND_FUEL_SLOT,
                BREWING_STAND_FUEL_SLOT + 1,
                false,
                default_item_max_stack_sizes,
            );
            fuel_moved
                || (is_ingredient
                    && move_item_stack_to_slots(
                        container_id,
                        slots,
                        source_index,
                        &mut moving,
                        BREWING_STAND_INGREDIENT_SLOT,
                        BREWING_STAND_INGREDIENT_SLOT + 1,
                        false,
                        default_item_max_stack_sizes,
                    ))
        } else if is_ingredient {
            move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                BREWING_STAND_INGREDIENT_SLOT,
                BREWING_STAND_INGREDIENT_SLOT + 1,
                false,
                default_item_max_stack_sizes,
            )
        } else if item_stack_item_id_in_set(&source_item, brewing_potion_item_ids) {
            move_item_stack_to_slots_where_with_limit(
                container_id,
                slots,
                source_index,
                &mut moving,
                BREWING_STAND_BOTTLE_SLOT_START,
                BREWING_STAND_BOTTLE_SLOT_END,
                false,
                |_| true,
                brewing_stand_slot_max_stack_size,
                default_item_max_stack_sizes,
            )
        } else if (BREWING_STAND_PLAYER_MAIN_START..BREWING_STAND_PLAYER_MAIN_END)
            .contains(&slot_num)
        {
            move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                BREWING_STAND_HOTBAR_START,
                BREWING_STAND_HOTBAR_END,
                false,
                default_item_max_stack_sizes,
            )
        } else {
            move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                BREWING_STAND_PLAYER_MAIN_START,
                BREWING_STAND_PLAYER_MAIN_END,
                false,
                default_item_max_stack_sizes,
            )
        }
    } else {
        false
    };

    if moved {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

pub(super) fn brewing_stand_slot_max_stack_size(
    slot_num: i16,
    _stack: &ProtocolItemStackSummary,
    base_max_stack_size: i32,
) -> i32 {
    if (BREWING_STAND_BOTTLE_SLOT_START..BREWING_STAND_BOTTLE_SLOT_END).contains(&slot_num) {
        base_max_stack_size.min(1)
    } else {
        base_max_stack_size
    }
}

pub(super) fn cartography_table_quick_move_requires_server_authority(
    slots: &[ContainerSlot],
    slot_num: i16,
    cartography_additional_item_ids: &BTreeSet<i32>,
) -> bool {
    if !(0..CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return false;
    }
    if matches!(
        slot_num,
        CARTOGRAPHY_TABLE_MAP_SLOT | CARTOGRAPHY_TABLE_ADDITIONAL_SLOT
    ) {
        return false;
    }
    if slot_num == CARTOGRAPHY_TABLE_RESULT_SLOT {
        return false;
    }
    let Some(source_item) = container_slot_item(slots, slot_num) else {
        return false;
    };
    if item_stack_is_empty(source_item) {
        return false;
    }
    if item_stack_has_map_id(source_item) {
        return hashed_component_patch_from_summary(&source_item.component_patch).is_none();
    }
    if cartography_additional_item_ids.is_empty() {
        return true;
    }
    false
}

pub(super) fn apply_cartography_table_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    cartography_additional_item_ids: &BTreeSet<i32>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    if slot_num == CARTOGRAPHY_TABLE_RESULT_SLOT {
        apply_cartography_table_result_quick_move_to_slots(
            container_id,
            slots,
            default_item_max_stack_sizes,
        );
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        CARTOGRAPHY_TABLE_MAP_SLOT | CARTOGRAPHY_TABLE_ADDITIONAL_SLOT => Some((
            CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
            CARTOGRAPHY_TABLE_HOTBAR_END,
            false,
        )),
        slot if (CARTOGRAPHY_TABLE_PLAYER_MAIN_START..CARTOGRAPHY_TABLE_HOTBAR_END)
            .contains(&slot) =>
        {
            if item_stack_has_map_id(&source_item) {
                Some((
                    CARTOGRAPHY_TABLE_MAP_SLOT,
                    CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
                    false,
                ))
            } else if item_stack_item_id_in_set(&source_item, cartography_additional_item_ids) {
                Some((
                    CARTOGRAPHY_TABLE_ADDITIONAL_SLOT,
                    CARTOGRAPHY_TABLE_RESULT_SLOT,
                    false,
                ))
            } else if (CARTOGRAPHY_TABLE_PLAYER_MAIN_START..CARTOGRAPHY_TABLE_PLAYER_MAIN_END)
                .contains(&slot)
            {
                Some((
                    CARTOGRAPHY_TABLE_HOTBAR_START,
                    CARTOGRAPHY_TABLE_HOTBAR_END,
                    false,
                ))
            } else {
                Some((
                    CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
                    CARTOGRAPHY_TABLE_PLAYER_MAIN_END,
                    false,
                ))
            }
        }
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

pub(super) fn apply_cartography_table_result_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    let Some(source_index) = slots
        .iter()
        .position(|slot| slot.slot == CARTOGRAPHY_TABLE_RESULT_SLOT)
    else {
        return;
    };
    let Some(map_index) = slots
        .iter()
        .position(|slot| slot.slot == CARTOGRAPHY_TABLE_MAP_SLOT)
    else {
        return;
    };
    let Some(additional_index) = slots
        .iter()
        .position(|slot| slot.slot == CARTOGRAPHY_TABLE_ADDITIONAL_SLOT)
    else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[map_index].item)
        || item_stack_is_empty(&slots[additional_index].item)
        || slots[map_index].item.count != 1
        || slots[additional_index].item.count != 1
    {
        return;
    }

    let mut trial = slots.to_vec();
    let mut moving = slots[source_index].item.clone();
    if !move_item_stack_to_slots(
        container_id,
        &mut trial,
        source_index,
        &mut moving,
        CARTOGRAPHY_TABLE_PLAYER_MAIN_START,
        CARTOGRAPHY_TABLE_HOTBAR_END,
        true,
        default_item_max_stack_sizes,
    ) || !item_stack_is_empty(&moving)
    {
        return;
    }

    trial[map_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[map_index]);
    trial[additional_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[additional_index]);
    trial[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[source_index]);
    slots.clone_from_slice(&trial);
}

pub(super) fn apply_cartography_table_result_pickup_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    button_num: i8,
) -> bool {
    if button_num != 0 || !item_stack_is_empty(cursor) {
        return false;
    }
    let Some(source_index) = slots
        .iter()
        .position(|slot| slot.slot == CARTOGRAPHY_TABLE_RESULT_SLOT)
    else {
        return false;
    };
    let Some(map_index) = slots
        .iter()
        .position(|slot| slot.slot == CARTOGRAPHY_TABLE_MAP_SLOT)
    else {
        return false;
    };
    let Some(additional_index) = slots
        .iter()
        .position(|slot| slot.slot == CARTOGRAPHY_TABLE_ADDITIONAL_SLOT)
    else {
        return false;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[map_index].item)
        || item_stack_is_empty(&slots[additional_index].item)
        || slots[map_index].item.count != 1
        || slots[additional_index].item.count != 1
    {
        return false;
    }

    *cursor = slots[source_index].item.clone();
    slots[map_index].item = ProtocolItemStackSummary::empty();
    slots[additional_index].item = ProtocolItemStackSummary::empty();
    slots[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut slots[map_index]);
    normalize_container_slot_selection(&mut slots[additional_index]);
    normalize_container_slot_selection(&mut slots[source_index]);
    true
}

pub(super) fn apply_loom_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    item_tags: Option<&RegistryTagState>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..LOOM_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    if slot_num == LOOM_RESULT_SLOT {
        apply_loom_result_quick_move_to_slots(container_id, slots, default_item_max_stack_sizes);
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        LOOM_BANNER_SLOT | LOOM_DYE_SLOT | LOOM_PATTERN_SLOT => {
            Some((LOOM_PLAYER_MAIN_START, LOOM_HOTBAR_END, false))
        }
        slot if (LOOM_PLAYER_MAIN_START..LOOM_HOTBAR_END).contains(&slot) => {
            if item_stack_in_item_tag(&source_item, item_tags, LOOM_BANNER_ITEM_TAG) {
                Some((LOOM_BANNER_SLOT, LOOM_DYE_SLOT, false))
            } else if item_stack_in_item_tag(&source_item, item_tags, LOOM_DYE_ITEM_TAG) {
                Some((LOOM_DYE_SLOT, LOOM_PATTERN_SLOT, false))
            } else if item_stack_in_item_tag(&source_item, item_tags, LOOM_PATTERN_ITEM_TAG) {
                Some((LOOM_PATTERN_SLOT, LOOM_RESULT_SLOT, false))
            } else if (LOOM_PLAYER_MAIN_START..LOOM_PLAYER_MAIN_END).contains(&slot) {
                Some((LOOM_HOTBAR_START, LOOM_HOTBAR_END, false))
            } else {
                Some((LOOM_PLAYER_MAIN_START, LOOM_PLAYER_MAIN_END, false))
            }
        }
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        return;
    };

    let mut moving = source_item;
    if move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

pub(super) fn apply_loom_result_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    let Some(source_index) = slots.iter().position(|slot| slot.slot == LOOM_RESULT_SLOT) else {
        return;
    };
    let Some(banner_index) = slots.iter().position(|slot| slot.slot == LOOM_BANNER_SLOT) else {
        return;
    };
    let Some(dye_index) = slots.iter().position(|slot| slot.slot == LOOM_DYE_SLOT) else {
        return;
    };
    let Some(pattern_index) = slots.iter().position(|slot| slot.slot == LOOM_PATTERN_SLOT) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[banner_index].item)
        || item_stack_is_empty(&slots[dye_index].item)
        || slots[banner_index].item.count != 1
        || slots[dye_index].item.count != 1
        || !item_stack_is_empty(&slots[pattern_index].item)
    {
        return;
    }

    let mut trial = slots.to_vec();
    let mut moving = slots[source_index].item.clone();
    if !move_item_stack_to_slots(
        container_id,
        &mut trial,
        source_index,
        &mut moving,
        LOOM_PLAYER_MAIN_START,
        LOOM_HOTBAR_END,
        true,
        default_item_max_stack_sizes,
    ) || !item_stack_is_empty(&moving)
    {
        return;
    }

    trial[banner_index].item.count -= 1;
    normalize_item_stack(&mut trial[banner_index].item);
    normalize_container_slot_selection(&mut trial[banner_index]);
    trial[dye_index].item.count -= 1;
    normalize_item_stack(&mut trial[dye_index].item);
    normalize_container_slot_selection(&mut trial[dye_index]);
    trial[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[source_index]);
    slots.clone_from_slice(&trial);
}

pub(super) fn apply_loom_result_pickup_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    button_num: i8,
) -> bool {
    if button_num != 0 || !item_stack_is_empty(cursor) {
        return false;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == LOOM_RESULT_SLOT) else {
        return false;
    };
    let Some(banner_index) = slots.iter().position(|slot| slot.slot == LOOM_BANNER_SLOT) else {
        return false;
    };
    let Some(dye_index) = slots.iter().position(|slot| slot.slot == LOOM_DYE_SLOT) else {
        return false;
    };
    let Some(pattern_index) = slots.iter().position(|slot| slot.slot == LOOM_PATTERN_SLOT) else {
        return false;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[banner_index].item)
        || item_stack_is_empty(&slots[dye_index].item)
        || slots[banner_index].item.count != 1
        || slots[dye_index].item.count != 1
        || !item_stack_is_empty(&slots[pattern_index].item)
    {
        return false;
    }

    *cursor = slots[source_index].item.clone();
    slots[banner_index].item = ProtocolItemStackSummary::empty();
    slots[dye_index].item = ProtocolItemStackSummary::empty();
    slots[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut slots[banner_index]);
    normalize_container_slot_selection(&mut slots[dye_index]);
    normalize_container_slot_selection(&mut slots[source_index]);
    true
}

pub(super) fn furnace_can_smelt(
    menu_type_id: Option<i32>,
    stack: &ProtocolItemStackSummary,
    recipe_property_sets: &BTreeMap<String, Vec<i32>>,
) -> bool {
    let Some(item_id) = stack.item_id else {
        return false;
    };
    let Some(property_set) = furnace_input_property_set(menu_type_id) else {
        return false;
    };
    recipe_property_sets
        .get(property_set)
        .is_some_and(|items| items.contains(&item_id))
}

pub(super) fn furnace_input_property_set(menu_type_id: Option<i32>) -> Option<&'static str> {
    match menu_type_id {
        Some(VANILLA_MENU_TYPE_BLAST_FURNACE_ID) => Some("minecraft:blast_furnace_input"),
        Some(VANILLA_MENU_TYPE_FURNACE_ID) => Some("minecraft:furnace_input"),
        Some(VANILLA_MENU_TYPE_SMOKER_ID) => Some("minecraft:smoker_input"),
        _ => None,
    }
}

pub(super) fn furnace_is_fuel(
    stack: &ProtocolItemStackSummary,
    furnace_fuel_item_ids: &BTreeSet<i32>,
) -> bool {
    stack
        .item_id
        .is_some_and(|item_id| furnace_fuel_item_ids.contains(&item_id))
}

pub(super) fn item_stack_in_item_tag(
    stack: &ProtocolItemStackSummary,
    item_tags: Option<&RegistryTagState>,
    tag: &str,
) -> bool {
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_tags
        .and_then(|registry| registry.tags.get(tag))
        .is_some_and(|entries| entries.contains(&item_id))
}

pub(super) fn item_stack_item_id_in_set(
    stack: &ProtocolItemStackSummary,
    item_ids: &BTreeSet<i32>,
) -> bool {
    stack
        .item_id
        .is_some_and(|item_id| item_ids.contains(&item_id))
}

pub(super) fn inventory_menu_quick_move_target_range(
    slot_num: i16,
    source_item: &ProtocolItemStackSummary,
    slots: &[ContainerSlot],
    default_item_equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
) -> Option<(i16, i16, bool)> {
    match slot_num {
        0 => Some((INVENTORY_MENU_MAIN_START, INVENTORY_MENU_HOTBAR_END, true)),
        1..=8 => Some((INVENTORY_MENU_MAIN_START, INVENTORY_MENU_HOTBAR_END, false)),
        INVENTORY_MENU_MAIN_START..=35 => inventory_menu_equipment_quick_move_target(
            source_item,
            slots,
            default_item_equipment_slots,
        )
        .or(Some((
            INVENTORY_MENU_HOTBAR_START,
            INVENTORY_MENU_HOTBAR_END,
            false,
        ))),
        INVENTORY_MENU_HOTBAR_START..=44 => inventory_menu_equipment_quick_move_target(
            source_item,
            slots,
            default_item_equipment_slots,
        )
        .or(Some((
            INVENTORY_MENU_MAIN_START,
            INVENTORY_MENU_MAIN_END,
            false,
        ))),
        INVENTORY_MENU_OFFHAND_SLOT => inventory_menu_equipment_quick_move_target(
            source_item,
            slots,
            default_item_equipment_slots,
        )
        .or(Some((
            INVENTORY_MENU_MAIN_START,
            INVENTORY_MENU_HOTBAR_END,
            false,
        ))),
        _ => None,
    }
}

pub(super) fn inventory_menu_equipment_quick_move_target(
    source_item: &ProtocolItemStackSummary,
    slots: &[ContainerSlot],
    default_item_equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
) -> Option<(i16, i16, bool)> {
    let item_id = source_item.item_id?;
    let target_slot =
        inventory_menu_equipment_slot(default_item_equipment_slots.get(&item_id).copied()?)?;
    if inventory_menu_slot_has_item(slots, target_slot) {
        return None;
    }
    Some((target_slot, target_slot + 1, false))
}

pub(super) fn inventory_menu_equipment_slot(equipment_slot: ItemEquipmentSlot) -> Option<i16> {
    match equipment_slot {
        ItemEquipmentSlot::Head => Some(5),
        ItemEquipmentSlot::Chest => Some(6),
        ItemEquipmentSlot::Legs => Some(7),
        ItemEquipmentSlot::Feet => Some(8),
        ItemEquipmentSlot::OffHand => Some(INVENTORY_MENU_OFFHAND_SLOT),
        ItemEquipmentSlot::MainHand | ItemEquipmentSlot::Body | ItemEquipmentSlot::Saddle => None,
    }
}

pub(super) fn inventory_menu_slot_has_item(slots: &[ContainerSlot], slot_num: i16) -> bool {
    slots
        .iter()
        .find(|slot| slot.slot == slot_num)
        .is_some_and(|slot| item_stack_is_non_empty(&slot.item))
}

pub(super) fn move_item_stack_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    source_index: usize,
    moving: &mut ProtocolItemStackSummary,
    start_slot: i16,
    end_slot: i16,
    backwards: bool,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    move_item_stack_to_slots_where(
        container_id,
        slots,
        source_index,
        moving,
        start_slot,
        end_slot,
        backwards,
        |_| true,
        default_item_max_stack_sizes,
    )
}

pub(super) fn move_item_stack_to_slots_where(
    container_id: i32,
    slots: &mut [ContainerSlot],
    source_index: usize,
    moving: &mut ProtocolItemStackSummary,
    start_slot: i16,
    end_slot: i16,
    backwards: bool,
    may_use_slot: impl FnMut(i16) -> bool,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    move_item_stack_to_slots_where_with_limit(
        container_id,
        slots,
        source_index,
        moving,
        start_slot,
        end_slot,
        backwards,
        may_use_slot,
        |_, _, max_stack_size| max_stack_size,
        default_item_max_stack_sizes,
    )
}

pub(super) fn move_item_stack_to_slots_where_with_limit(
    container_id: i32,
    slots: &mut [ContainerSlot],
    source_index: usize,
    moving: &mut ProtocolItemStackSummary,
    start_slot: i16,
    end_slot: i16,
    backwards: bool,
    mut may_use_slot: impl FnMut(i16) -> bool,
    mut slot_max_stack_size: impl FnMut(i16, &ProtocolItemStackSummary, i32) -> i32,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    let mut changed = false;
    if item_stack_max_stack_size(moving, default_item_max_stack_sizes) > 1 {
        for dest_slot in quick_move_slot_ids(start_slot, end_slot, backwards) {
            if !may_use_slot(dest_slot) {
                continue;
            }
            if item_stack_is_empty(moving) {
                break;
            }
            let Some(dest_index) = slots.iter().position(|slot| slot.slot == dest_slot) else {
                continue;
            };
            if dest_index == source_index {
                continue;
            }
            let slot = &mut slots[dest_index];
            if item_stack_is_empty(&slot.item) || !same_item_same_components(moving, &slot.item) {
                continue;
            }
            let base_max_stack_size = container_slot_max_stack_size(
                container_id,
                dest_slot,
                &slot.item,
                default_item_max_stack_sizes,
            );
            let max_stack_size =
                slot_max_stack_size(dest_slot, &slot.item, base_max_stack_size).max(0);
            let moved = moving.count.min((max_stack_size - slot.item.count).max(0));
            if moved <= 0 {
                continue;
            }
            slot.item.count += moved;
            moving.count -= moved;
            normalize_item_stack(moving);
            normalize_container_slot_selection(slot);
            changed = true;
        }
    }

    if !item_stack_is_empty(moving) {
        for dest_slot in quick_move_slot_ids(start_slot, end_slot, backwards) {
            if !may_use_slot(dest_slot) {
                continue;
            }
            let Some(dest_index) = slots.iter().position(|slot| slot.slot == dest_slot) else {
                continue;
            };
            if dest_index == source_index || !item_stack_is_empty(&slots[dest_index].item) {
                continue;
            }
            let base_max_stack_size = container_slot_max_stack_size(
                container_id,
                dest_slot,
                moving,
                default_item_max_stack_sizes,
            );
            let max_stack_size = slot_max_stack_size(dest_slot, moving, base_max_stack_size).max(0);
            let amount = moving.count.min(max_stack_size);
            if amount <= 0 {
                continue;
            }
            let slot = &mut slots[dest_index];
            move_stack_count(moving, &mut slot.item, amount);
            normalize_container_slot_selection(slot);
            changed = true;
            break;
        }
    }

    changed
}

pub(super) fn quick_move_slot_ids(start_slot: i16, end_slot: i16, backwards: bool) -> Vec<i16> {
    if backwards {
        (start_slot..end_slot).rev().collect()
    } else {
        (start_slot..end_slot).collect()
    }
}

pub(super) fn normalize_container_slot_selection(slot: &mut ContainerSlot) {
    slot.local_selected_bundle_item_index = normalize_local_selected_bundle_item_index(
        slot.local_selected_bundle_item_index,
        &slot.item,
    );
}

pub(super) fn apply_outside_pickup_click(cursor: &mut ProtocolItemStackSummary, button_num: i8) {
    if item_stack_is_empty(cursor) {
        return;
    }
    if button_num == 0 {
        *cursor = ProtocolItemStackSummary::empty();
    } else if button_num == 1 {
        cursor.count -= 1;
        normalize_item_stack(cursor);
    }
}

pub(super) fn apply_slot_pickup_click(
    container_id: i32,
    slot_num: i16,
    slot: &mut ProtocolItemStackSummary,
    cursor: &mut ProtocolItemStackSummary,
    button_num: i8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !matches!(button_num, 0 | 1) {
        return;
    }

    if item_stack_is_empty(slot) {
        if item_stack_is_empty(cursor) {
            return;
        }
        let amount = if button_num == 0 { cursor.count } else { 1 };
        let amount = amount.min(container_slot_max_stack_size(
            container_id,
            slot_num,
            cursor,
            default_item_max_stack_sizes,
        ));
        move_stack_count(cursor, slot, amount);
        return;
    }

    if item_stack_is_empty(cursor) {
        let amount = if button_num == 0 {
            slot.count
        } else {
            (slot.count + 1) / 2
        };
        move_stack_count(slot, cursor, amount);
        return;
    }

    if same_item_same_components(slot, cursor) {
        let amount = if button_num == 0 { cursor.count } else { 1 };
        let max_stack_size = container_slot_max_stack_size(
            container_id,
            slot_num,
            slot,
            default_item_max_stack_sizes,
        );
        let moved = amount.min((max_stack_size - slot.count).max(0));
        if moved > 0 {
            slot.count += moved;
            cursor.count -= moved;
            normalize_item_stack(cursor);
        }
    } else if cursor.count
        <= container_slot_max_stack_size(
            container_id,
            slot_num,
            cursor,
            default_item_max_stack_sizes,
        )
    {
        std::mem::swap(slot, cursor);
    }
}

pub(super) fn move_stack_count(
    source: &mut ProtocolItemStackSummary,
    target: &mut ProtocolItemStackSummary,
    amount: i32,
) {
    let moved = amount.min(source.count).max(0);
    if moved <= 0 {
        return;
    }
    *target = source.clone();
    target.count = moved;
    source.count -= moved;
    normalize_item_stack(source);
}

pub(super) fn normalize_item_stack(stack: &mut ProtocolItemStackSummary) {
    if item_stack_is_empty(stack) {
        *stack = ProtocolItemStackSummary::empty();
    }
}

pub(super) fn item_stack_is_empty(stack: &ProtocolItemStackSummary) -> bool {
    stack.item_id.is_none() || stack.count <= 0
}

pub(super) fn item_stack_has_component_patch(stack: &ProtocolItemStackSummary) -> bool {
    stack.component_patch != Default::default()
}

pub(super) fn same_item_same_components(
    left: &ProtocolItemStackSummary,
    right: &ProtocolItemStackSummary,
) -> bool {
    left.item_id == right.item_id && left.component_patch == right.component_patch
}

pub(super) fn container_slot_max_stack_size(
    container_id: i32,
    slot_num: i16,
    stack: &ProtocolItemStackSummary,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> i32 {
    let item_max_stack_size = item_stack_max_stack_size(stack, default_item_max_stack_sizes);
    let slot_max_stack_size = if container_id == INVENTORY_MENU_CONTAINER_ID {
        match slot_num {
            0 => 0,
            5..=8 => 1,
            _ => VANILLA_DEFAULT_MAX_STACK_SIZE,
        }
    } else {
        VANILLA_DEFAULT_MAX_STACK_SIZE
    };
    item_max_stack_size.min(slot_max_stack_size)
}

pub(super) fn item_stack_max_stack_size(
    stack: &ProtocolItemStackSummary,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> i32 {
    if item_stack_is_empty(stack) {
        return 0;
    }
    if let Some(size) = stack.component_patch.max_stack_size {
        return clamp_vanilla_item_max_stack_size(size);
    }
    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_MAX_STACK_SIZE_COMPONENT_ID)
    {
        return 1;
    }
    if stack.component_patch.max_damage.is_some() || stack.component_patch.damage.is_some() {
        return 1;
    }
    stack
        .item_id
        .and_then(|item_id| default_item_max_stack_sizes.get(&item_id).copied())
        .map(clamp_vanilla_item_max_stack_size)
        .unwrap_or(VANILLA_DEFAULT_MAX_STACK_SIZE)
}

pub(super) fn apply_local_selected_bundle_item_index(
    item: &ProtocolItemStackSummary,
    current_selected_item_index: &mut i32,
    selected_item_index: i32,
) -> bool {
    let Some(bundle_item_count) = item.component_patch.bundle_contents_item_count else {
        return false;
    };

    *current_selected_item_index = if selected_item_index == NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        || selected_item_index == *current_selected_item_index
        || usize::try_from(selected_item_index)
            .map(|index| index >= bundle_item_count)
            .unwrap_or(true)
    {
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    } else {
        selected_item_index
    };
    true
}

pub(super) fn normalize_local_selected_bundle_item_index(
    selected_item_index: i32,
    item: &ProtocolItemStackSummary,
) -> i32 {
    let Some(bundle_item_count) = item.component_patch.bundle_contents_item_count else {
        return NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX;
    };
    let Ok(selected_item_index_usize) = usize::try_from(selected_item_index) else {
        return NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX;
    };
    if selected_item_index_usize < bundle_item_count {
        selected_item_index
    } else {
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    }
}
