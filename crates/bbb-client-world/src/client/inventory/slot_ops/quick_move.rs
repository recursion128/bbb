use super::*;

pub(in crate::client::inventory) fn apply_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    profiles: &ItemProfiles,
) {
    if container_id != INVENTORY_MENU_CONTAINER_ID || slot_num < 0 {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    // Vanilla `AbstractContainerMenu.doClick` (`AbstractContainerMenu.java:423-427`)
    // loops `quickMoveStack` while the slot still holds the same item (e.g. stacked
    // wearable blocks moving 1 to armor slot, then remaining stack to hotbar/main).
    loop {
        if item_stack_is_empty(&slots[source_index].item) {
            return;
        }
        let source_item = slots[source_index].item.clone();
        let Some((start_slot, end_slot, backwards)) =
            inventory_menu_quick_move_target_range(slot_num, &source_item, slots, profiles)
        else {
            return;
        };

        let mut moving = source_item.clone();
        if !move_item_stack_to_slots(
            container_id,
            slots,
            source_index,
            &mut moving,
            start_slot,
            end_slot,
            backwards,
            profiles,
        ) {
            return;
        }
        normalize_item_stack(&mut moving);
        let moved_any = moving.count != source_item.count;
        slots[source_index].item = moving;
        slots[source_index].normalize_selection();
        if !moved_any
            || item_stack_is_empty(&slots[source_index].item)
            || slots[source_index].item.item_id != source_item.item_id
        {
            return;
        }
    }
}

pub(in crate::client::inventory) fn apply_mount_inventory_quick_move_to_slots(
    container_id: i32,
    player_start: i16,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    ctx: InventoryCtx<'_>,
    mount_rules: Option<LocalMountSlotRules>,
) {
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }
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
            ctx.items,
        );
    } else {
        if let Some(target_slot) =
            mount_equipment_quick_move_target(&source_item, slots, ctx, mount_rules)
        {
            changed = move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                target_slot,
                target_slot + 1,
                false,
                ctx.items,
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
                ctx.items,
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
                    ctx.items,
                );
            }
        }
    }

    if changed {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        slots[source_index].normalize_selection();
    }
}

pub(in crate::client::inventory) fn mount_inventory_quick_move_requires_server_authority(
    player_start: i16,
    slots: &[ContainerSlot],
    slot_num: i16,
    ctx: InventoryCtx<'_>,
    mount_rules: Option<LocalMountSlotRules>,
) -> bool {
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
    if item_stack_has_component_patch(&source.item) {
        return true;
    }
    let Some(rules) = mount_rules else {
        return true;
    };
    let Some((slot, _)) = ctx.items.item_stack_equipment(&source.item) else {
        return ctx.items.item_has_effective_component(
            source.item.item_id,
            &source.item.component_patch,
            VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
        );
    };
    mount_stack_equipment_slot(ctx, rules, &source.item, slot).is_none()
}

pub(in crate::client::inventory) fn mount_equipment_quick_move_target(
    stack: &ProtocolItemStackSummary,
    slots: &[ContainerSlot],
    ctx: InventoryCtx<'_>,
    mount_rules: Option<LocalMountSlotRules>,
) -> Option<i16> {
    let rules = mount_rules?;
    let slot = ctx.items.item_stack_equipment(stack)?.0;
    if mount_stack_equipment_slot(ctx, rules, stack, slot)? != Some(slot) {
        return None;
    }
    match slot {
        EquipmentSlot::Body
            if rules.equipment_slots.body.is_some()
                && !inventory_menu_slot_has_item(slots, MOUNT_BODY_ARMOR_SLOT) =>
        {
            Some(MOUNT_BODY_ARMOR_SLOT)
        }
        EquipmentSlot::Saddle
            if rules.equipment_slots.saddle
                && !inventory_menu_slot_has_item(slots, MOUNT_SADDLE_SLOT) =>
        {
            Some(MOUNT_SADDLE_SLOT)
        }
        _ => None,
    }
}

pub(in crate::client::inventory) fn apply_furnace_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    menu_type_id: Option<i32>,
    recipe_property_sets: &BTreeMap<String, Vec<i32>>,
    furnace_fuel_item_ids: &BTreeSet<i32>,
    profiles: &ItemProfiles,
) {
    if !(0..FURNACE_CONTAINER_SLOT_COUNT + GENERIC_CONTAINER_PLAYER_SLOT_COUNT).contains(&slot_num)
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
        profiles,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        slots[source_index].normalize_selection();
    }
}

pub(in crate::client::inventory) fn apply_generic_container_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    container_slot_count: i16,
    profiles: &ItemProfiles,
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
        profiles,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        slots[source_index].normalize_selection();
    }
}

pub(in crate::client::inventory) fn apply_shulker_box_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    container_slot_count: i16,
    profiles: &ItemProfiles,
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
    if slot_num >= container_slot_count
        && !item_can_fit_inside_container_items(&slots[source_index].item)
    {
        return;
    }
    apply_generic_container_quick_move_to_slots(
        container_id,
        slots,
        slot_num,
        container_slot_count,
        profiles,
    );
}

pub(in crate::client::inventory) fn apply_anvil_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    result_may_pickup: bool,
    profiles: &ItemProfiles,
) {
    if !(0..ANVIL_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    if slot_num == ANVIL_RESULT_SLOT {
        apply_anvil_result_quick_move_to_slots(container_id, slots, result_may_pickup, profiles);
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
        profiles,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        slots[source_index].normalize_selection();
    }
}

pub(in crate::client::inventory) fn apply_anvil_result_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    result_may_pickup: bool,
    profiles: &ItemProfiles,
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
        profiles,
    ) || !item_stack_is_empty(&moving)
    {
        return;
    }

    trial[input_index].item = ProtocolItemStackSummary::empty();
    trial[input_index].normalize_selection();
    trial[source_index].item = ProtocolItemStackSummary::empty();
    trial[source_index].normalize_selection();
    slots.clone_from_slice(&trial);
}

pub(in crate::client::inventory) fn apply_anvil_result_to_slots(
    slots: &mut [ContainerSlot],
    destination: ResultTakeDestination<'_>,
    result_may_pickup: bool,
) -> bool {
    if !result_may_pickup || !destination.may_take() {
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

    destination.take(slots, source_index, &[input_index])
}

pub(in crate::client::inventory) fn anvil_result_may_pickup(
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

pub(in crate::client::inventory) fn set_container_data_value(
    data_values: &mut Vec<ContainerDataValue>,
    id: i16,
    value: i16,
) {
    match data_values.binary_search_by_key(&id, |entry| entry.id) {
        Ok(index) => data_values[index].value = value,
        Err(index) => data_values.insert(index, ContainerDataValue { id, value }),
    }
}

pub(in crate::client::inventory) fn beacon_effect_data_value(
    effect_id: Option<i32>,
) -> Option<i16> {
    match effect_id {
        Some(effect_id) if effect_id >= 0 => effect_id
            .checked_add(1)
            .and_then(|value| i16::try_from(value).ok()),
        Some(_) => None,
        None => Some(0),
    }
}

pub(in crate::client::inventory) fn apply_beacon_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    item_tags: Option<&RegistryTagState>,
    profiles: &ItemProfiles,
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
        profiles,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        slots[source_index].normalize_selection();
    }
}

pub(in crate::client::inventory) fn enchantment_quick_move_requires_server_authority(
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

pub(in crate::client::inventory) fn apply_enchantment_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    enchantment_lapis_lazuli_item_ids: &BTreeSet<i32>,
    profiles: &ItemProfiles,
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
        slots[source_index].normalize_selection();
        slots[input_index].normalize_selection();
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
        profiles,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        slots[source_index].normalize_selection();
    }
}

pub(in crate::client::inventory) fn apply_brewing_stand_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    item_tags: Option<&RegistryTagState>,
    brewing_potion_item_ids: &BTreeSet<i32>,
    brewing_ingredient_item_ids: &BTreeSet<i32>,
    profiles: &ItemProfiles,
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
            profiles,
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
                profiles,
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
                        profiles,
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
                profiles,
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
                profiles,
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
                profiles,
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
                profiles,
            )
        }
    } else {
        false
    };

    if moved {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        slots[source_index].normalize_selection();
    }
}

pub(in crate::client::inventory) fn brewing_stand_slot_max_stack_size(
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

pub(in crate::client::inventory) fn cartography_table_quick_move_requires_server_authority(
    ctx: InventoryCtx<'_>,
    slots: &[ContainerSlot],
    slot_num: i16,
    cartography_additional_items: &BTreeMap<i32, crate::CartographyAdditionalItemKind>,
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
        return hashed_component_patch_from_summary(ctx, &source_item.component_patch).is_none();
    }
    if cartography_additional_items.is_empty() {
        return true;
    }
    false
}

pub(in crate::client::inventory) fn apply_cartography_table_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    cartography_additional_items: &BTreeMap<i32, crate::CartographyAdditionalItemKind>,
    profiles: &ItemProfiles,
) -> bool {
    if !(0..CARTOGRAPHY_TABLE_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return true;
    }
    if slot_num == CARTOGRAPHY_TABLE_RESULT_SLOT {
        return apply_cartography_table_result_quick_move_to_slots(container_id, slots, profiles);
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return true;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return true;
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
            } else if source_item.item_id.is_some_and(|item_id| {
                source_item.count > 0 && cartography_additional_items.contains_key(&item_id)
            }) {
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
        return true;
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
        profiles,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        slots[source_index].normalize_selection();
    }
    true
}

pub(in crate::client::inventory) fn apply_cartography_table_result_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    profiles: &ItemProfiles,
) -> bool {
    let Some(source_index) = slots
        .iter()
        .position(|slot| slot.slot == CARTOGRAPHY_TABLE_RESULT_SLOT)
    else {
        return true;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return true;
    }
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
    if item_stack_is_empty(&slots[map_index].item)
        || item_stack_is_empty(&slots[additional_index].item)
        || slots[map_index].item.count != 1
        || slots[additional_index].item.count != 1
    {
        // Multi-craft `QuickMove` repopulates the result via server-side logic.
        return false;
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
        profiles,
    ) || !item_stack_is_empty(&moving)
    {
        return true;
    }

    trial[map_index].item = ProtocolItemStackSummary::empty();
    trial[map_index].normalize_selection();
    trial[additional_index].item = ProtocolItemStackSummary::empty();
    trial[additional_index].normalize_selection();
    trial[source_index].item = ProtocolItemStackSummary::empty();
    trial[source_index].normalize_selection();
    slots.clone_from_slice(&trial);
    true
}

pub(in crate::client::inventory) fn apply_cartography_table_result_to_slots(
    slots: &mut [ContainerSlot],
    destination: ResultTakeDestination<'_>,
) -> bool {
    if !destination.may_take() {
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

    destination.take(slots, source_index, &[map_index, additional_index])
}

pub(in crate::client::inventory) fn apply_loom_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    item_tags: Option<&RegistryTagState>,
    profiles: &ItemProfiles,
) -> bool {
    if !(0..LOOM_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return true;
    }
    if slot_num == LOOM_RESULT_SLOT {
        return apply_loom_result_quick_move_to_slots(container_id, slots, profiles);
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return true;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return true;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        LOOM_BANNER_SLOT | LOOM_DYE_SLOT | LOOM_PATTERN_SLOT => {
            Some((LOOM_PLAYER_MAIN_START, LOOM_HOTBAR_END, false))
        }
        slot if (LOOM_PLAYER_MAIN_START..LOOM_HOTBAR_END).contains(&slot) => {
            // `LoomMenu.java:111-117`: `isDyeItem` and `isPatternItem` require both tag and component.
            if item_stack_in_item_tag(&source_item, item_tags, LOOM_BANNER_ITEM_TAG) {
                Some((LOOM_BANNER_SLOT, LOOM_DYE_SLOT, false))
            } else if item_stack_in_item_tag(&source_item, item_tags, LOOM_DYE_ITEM_TAG)
                && profiles.item_has_effective_component(
                    source_item.item_id,
                    &source_item.component_patch,
                    VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
                )
            {
                Some((LOOM_DYE_SLOT, LOOM_PATTERN_SLOT, false))
            } else if item_stack_in_item_tag(&source_item, item_tags, LOOM_PATTERN_ITEM_TAG)
                && profiles.item_has_effective_component(
                    source_item.item_id,
                    &source_item.component_patch,
                    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
                )
            {
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
        return true;
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
        profiles,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        slots[source_index].normalize_selection();
    }
    true
}

pub(in crate::client::inventory) fn apply_loom_result_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    profiles: &ItemProfiles,
) -> bool {
    let Some(source_index) = slots.iter().position(|slot| slot.slot == LOOM_RESULT_SLOT) else {
        return true;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return true;
    }
    let Some(banner_index) = slots.iter().position(|slot| slot.slot == LOOM_BANNER_SLOT) else {
        return false;
    };
    let Some(dye_index) = slots.iter().position(|slot| slot.slot == LOOM_DYE_SLOT) else {
        return false;
    };
    let Some(pattern_index) = slots.iter().position(|slot| slot.slot == LOOM_PATTERN_SLOT) else {
        return false;
    };
    if item_stack_is_empty(&slots[banner_index].item)
        || item_stack_is_empty(&slots[dye_index].item)
        || slots[banner_index].item.count != 1
        || slots[dye_index].item.count != 1
        || !item_stack_is_empty(&slots[pattern_index].item)
    {
        return false;
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
        profiles,
    ) || !item_stack_is_empty(&moving)
    {
        return true;
    }

    trial[banner_index].item.count -= 1;
    normalize_item_stack(&mut trial[banner_index].item);
    trial[banner_index].normalize_selection();
    trial[dye_index].item.count -= 1;
    normalize_item_stack(&mut trial[dye_index].item);
    trial[dye_index].normalize_selection();
    trial[source_index].item = ProtocolItemStackSummary::empty();
    trial[source_index].normalize_selection();
    slots.clone_from_slice(&trial);
    true
}

pub(in crate::client::inventory) fn apply_loom_result_to_slots(
    slots: &mut [ContainerSlot],
    destination: ResultTakeDestination<'_>,
) -> bool {
    if !destination.may_take() {
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
        || (matches!(destination, ResultTakeDestination::Drop)
            && slots[source_index].item.count != 1)
        || item_stack_is_empty(&slots[banner_index].item)
        || item_stack_is_empty(&slots[dye_index].item)
        || slots[banner_index].item.count != 1
        || slots[dye_index].item.count != 1
        || !item_stack_is_empty(&slots[pattern_index].item)
    {
        return false;
    }

    destination.take(slots, source_index, &[banner_index, dye_index])
}
