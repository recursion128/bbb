use super::*;

pub(super) fn apply_grindstone_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    default_damageable_item_ids: &BTreeSet<i32>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..GRINDSTONE_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    if slot_num == GRINDSTONE_RESULT_SLOT {
        apply_grindstone_result_quick_move_to_slots(
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
    let inputs_full = inventory_menu_slot_has_item(slots, GRINDSTONE_INPUT_SLOT)
        && inventory_menu_slot_has_item(slots, GRINDSTONE_ADDITIONAL_SLOT);
    let target = match slot_num {
        GRINDSTONE_INPUT_SLOT | GRINDSTONE_ADDITIONAL_SLOT => {
            Some((GRINDSTONE_PLAYER_MAIN_START, GRINDSTONE_HOTBAR_END, false))
        }
        slot if !inputs_full
            && (GRINDSTONE_PLAYER_MAIN_START..GRINDSTONE_HOTBAR_END).contains(&slot)
            && item_stack_is_default_damageable(&source_item, default_damageable_item_ids) =>
        {
            Some((GRINDSTONE_INPUT_SLOT, GRINDSTONE_RESULT_SLOT, false))
        }
        slot if inputs_full
            && (GRINDSTONE_PLAYER_MAIN_START..GRINDSTONE_PLAYER_MAIN_END).contains(&slot) =>
        {
            Some((GRINDSTONE_HOTBAR_START, GRINDSTONE_HOTBAR_END, false))
        }
        slot if inputs_full && (GRINDSTONE_HOTBAR_START..GRINDSTONE_HOTBAR_END).contains(&slot) => {
            Some((
                GRINDSTONE_PLAYER_MAIN_START,
                GRINDSTONE_PLAYER_MAIN_END,
                false,
            ))
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

fn apply_grindstone_result_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    let Some(source_index) = slots
        .iter()
        .position(|slot| slot.slot == GRINDSTONE_RESULT_SLOT)
    else {
        return;
    };
    let Some(input_index) = slots
        .iter()
        .position(|slot| slot.slot == GRINDSTONE_INPUT_SLOT)
    else {
        return;
    };
    let Some(additional_index) = slots
        .iter()
        .position(|slot| slot.slot == GRINDSTONE_ADDITIONAL_SLOT)
    else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || (item_stack_is_empty(&slots[input_index].item)
            && item_stack_is_empty(&slots[additional_index].item))
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
        GRINDSTONE_PLAYER_MAIN_START,
        GRINDSTONE_HOTBAR_END,
        true,
        default_item_max_stack_sizes,
    ) || !item_stack_is_empty(&moving)
    {
        return;
    }

    trial[input_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[input_index]);
    trial[additional_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[additional_index]);
    trial[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[source_index]);
    slots.clone_from_slice(&trial);
}

pub(super) fn apply_grindstone_result_pickup_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    button_num: i8,
) -> bool {
    if button_num != 0 || !item_stack_is_empty(cursor) {
        return false;
    }
    let Some(source_index) = slots
        .iter()
        .position(|slot| slot.slot == GRINDSTONE_RESULT_SLOT)
    else {
        return false;
    };
    let Some(input_index) = slots
        .iter()
        .position(|slot| slot.slot == GRINDSTONE_INPUT_SLOT)
    else {
        return false;
    };
    let Some(additional_index) = slots
        .iter()
        .position(|slot| slot.slot == GRINDSTONE_ADDITIONAL_SLOT)
    else {
        return false;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || (item_stack_is_empty(&slots[input_index].item)
            && item_stack_is_empty(&slots[additional_index].item))
    {
        return false;
    }

    *cursor = slots[source_index].item.clone();
    slots[input_index].item = ProtocolItemStackSummary::empty();
    slots[additional_index].item = ProtocolItemStackSummary::empty();
    slots[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut slots[input_index]);
    normalize_container_slot_selection(&mut slots[additional_index]);
    normalize_container_slot_selection(&mut slots[source_index]);
    true
}

pub(super) fn grindstone_quick_move_requires_server_authority(
    slots: &[ContainerSlot],
    slot_num: i16,
    default_damageable_item_ids: &BTreeSet<i32>,
) -> bool {
    if !(0..GRINDSTONE_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return false;
    }
    if slot_num == GRINDSTONE_RESULT_SLOT {
        return false;
    }
    if matches!(slot_num, GRINDSTONE_INPUT_SLOT | GRINDSTONE_ADDITIONAL_SLOT) {
        return false;
    }
    if !(GRINDSTONE_PLAYER_MAIN_START..GRINDSTONE_HOTBAR_END).contains(&slot_num) {
        return false;
    }
    let inputs_full = inventory_menu_slot_has_item(slots, GRINDSTONE_INPUT_SLOT)
        && inventory_menu_slot_has_item(slots, GRINDSTONE_ADDITIONAL_SLOT);
    if inputs_full {
        return false;
    }
    let Some(source) = slots.iter().find(|slot| slot.slot == slot_num) else {
        return false;
    };
    !item_stack_is_default_damageable(&source.item, default_damageable_item_ids)
}

fn item_stack_is_default_damageable(
    stack: &ProtocolItemStackSummary,
    default_damageable_item_ids: &BTreeSet<i32>,
) -> bool {
    if item_stack_is_empty(stack)
        || !component_patch_can_be_hashed_from_summary(&stack.component_patch)
        || stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_MAX_DAMAGE_COMPONENT_ID)
    {
        return false;
    }

    let Some(item_id) = stack.item_id.filter(|item_id| *item_id >= 0) else {
        return false;
    };
    default_damageable_item_ids.contains(&item_id)
}
