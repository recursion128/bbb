use super::*;

pub(super) fn apply_stonecutter_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    stonecutter_recipes: &[ProtocolStonecutterSelectableRecipeSummary],
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..STONECUTTER_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    if slot_num == STONECUTTER_RESULT_SLOT {
        apply_stonecutter_result_quick_move_to_slots(
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
        STONECUTTER_INPUT_SLOT => {
            Some((STONECUTTER_PLAYER_MAIN_START, STONECUTTER_HOTBAR_END, false))
        }
        slot if (STONECUTTER_PLAYER_MAIN_START..STONECUTTER_HOTBAR_END).contains(&slot)
            && stonecutter_accepts_input(&source_item, stonecutter_recipes) =>
        {
            Some((STONECUTTER_INPUT_SLOT, STONECUTTER_RESULT_SLOT, false))
        }
        slot if (STONECUTTER_PLAYER_MAIN_START..STONECUTTER_PLAYER_MAIN_END).contains(&slot) => {
            Some((STONECUTTER_HOTBAR_START, STONECUTTER_HOTBAR_END, false))
        }
        slot if (STONECUTTER_HOTBAR_START..STONECUTTER_HOTBAR_END).contains(&slot) => Some((
            STONECUTTER_PLAYER_MAIN_START,
            STONECUTTER_PLAYER_MAIN_END,
            false,
        )),
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

fn apply_stonecutter_result_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    let Some(source_index) = slots
        .iter()
        .position(|slot| slot.slot == STONECUTTER_RESULT_SLOT)
    else {
        return;
    };
    let Some(input_index) = slots
        .iter()
        .position(|slot| slot.slot == STONECUTTER_INPUT_SLOT)
    else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[input_index].item)
    {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let mut trial = slots.to_vec();
    let mut moving = source_item.clone();
    if !move_item_stack_to_slots(
        container_id,
        &mut trial,
        source_index,
        &mut moving,
        STONECUTTER_PLAYER_MAIN_START,
        STONECUTTER_HOTBAR_END,
        true,
        default_item_max_stack_sizes,
    ) || !item_stack_is_empty(&moving)
    {
        return;
    }

    trial[input_index].item.count -= 1;
    normalize_item_stack(&mut trial[input_index].item);
    normalize_container_slot_selection(&mut trial[input_index]);
    trial[source_index].item = if item_stack_is_empty(&trial[input_index].item) {
        ProtocolItemStackSummary::empty()
    } else {
        source_item
    };
    normalize_container_slot_selection(&mut trial[source_index]);
    slots.clone_from_slice(&trial);
}

pub(super) fn apply_stonecutter_result_pickup_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    button_num: i8,
) -> bool {
    if button_num != 0 || !item_stack_is_empty(cursor) {
        return false;
    }
    let Some(source_index) = slots
        .iter()
        .position(|slot| slot.slot == STONECUTTER_RESULT_SLOT)
    else {
        return false;
    };
    let Some(input_index) = slots
        .iter()
        .position(|slot| slot.slot == STONECUTTER_INPUT_SLOT)
    else {
        return false;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[input_index].item)
        || slots[input_index].item.count != 1
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

fn stonecutter_accepts_input(
    stack: &ProtocolItemStackSummary,
    stonecutter_recipes: &[ProtocolStonecutterSelectableRecipeSummary],
) -> bool {
    let Some(item_id) = stack.item_id else {
        return false;
    };
    stonecutter_recipes
        .iter()
        .any(|recipe| recipe.input.item_ids.contains(&item_id))
}
