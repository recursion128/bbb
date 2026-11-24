use super::*;

pub(super) fn apply_grindstone_menu_quick_move_to_slots(
    ctx: InventoryCtx<'_>,
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    profiles: &ItemProfiles,
) {
    if !(0..GRINDSTONE_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    if slot_num == GRINDSTONE_RESULT_SLOT {
        apply_grindstone_result_to_slots(
            slots,
            ResultTakeDestination::PlayerInventory {
                container_id,
                player_slots: GRINDSTONE_PLAYER_MAIN_START..GRINDSTONE_HOTBAR_END,
                profiles,
            },
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
            && grindstone_stack_can_quick_move(ctx, &source_item, profiles) =>
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
        profiles,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        slots[source_index].normalize_selection();
    }
}

pub(super) fn apply_grindstone_result_to_slots(
    slots: &mut [ContainerSlot],
    destination: ResultTakeDestination<'_>,
) -> bool {
    if !destination.may_take() {
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

    destination.take(slots, source_index, &[input_index, additional_index])
}

pub(super) fn grindstone_quick_move_requires_server_authority(
    ctx: InventoryCtx<'_>,
    slots: &[ContainerSlot],
    slot_num: i16,
    profiles: &ItemProfiles,
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
    !grindstone_stack_can_quick_move(ctx, &source.item, profiles)
}

fn grindstone_stack_can_quick_move(
    ctx: InventoryCtx<'_>,
    stack: &ProtocolItemStackSummary,
    profiles: &ItemProfiles,
) -> bool {
    component_patch_can_be_hashed_from_summary(ctx, &stack.component_patch)
        && item_stack_is_damageable_or_enchanted(stack, profiles)
}
