use super::*;

pub(super) fn apply_stonecutter_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    stonecutter_recipes: &[ProtocolStonecutterSelectableRecipeSummary],
    profiles: &ItemProfiles,
) {
    if !(0..STONECUTTER_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return;
    }
    if slot_num == STONECUTTER_RESULT_SLOT {
        apply_stonecutter_result_quick_move_to_slots(container_id, slots, profiles);
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
        profiles,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        slots[source_index].normalize_selection();
    }
}

fn apply_stonecutter_result_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    profiles: &ItemProfiles,
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

    // Vanilla `AbstractContainerMenu.doClick` (`AbstractContainerMenu.java:423-427`)
    // loops `StonecutterMenu.quickMoveStack` (`StonecutterMenu.java:180-228`) while
    // the result slot continues holding the same item.
    let source_item = slots[source_index].item.clone();
    let mut trial = slots.to_vec();
    loop {
        if item_stack_is_empty(&trial[source_index].item)
            || item_stack_is_empty(&trial[input_index].item)
            || trial[source_index].item.item_id != source_item.item_id
        {
            break;
        }
        let mut step_trial = trial.clone();
        let mut moving = step_trial[source_index].item.clone();
        let initial_count = moving.count;
        if !move_item_stack_to_slots(
            container_id,
            &mut step_trial,
            source_index,
            &mut moving,
            STONECUTTER_PLAYER_MAIN_START,
            STONECUTTER_HOTBAR_END,
            true,
            profiles,
        ) || moving.count == initial_count
        {
            break;
        }
        let dropped_remainder = !item_stack_is_empty(&moving);
        step_trial[input_index].item.count -= 1;
        normalize_item_stack(&mut step_trial[input_index].item);
        step_trial[input_index].normalize_selection();
        step_trial[source_index].item = if item_stack_is_empty(&step_trial[input_index].item) {
            ProtocolItemStackSummary::empty()
        } else {
            source_item.clone()
        };
        step_trial[source_index].normalize_selection();
        trial = step_trial;
        if dropped_remainder {
            break;
        }
    }
    slots.clone_from_slice(&trial);
}

pub(super) fn apply_stonecutter_result_to_slots(
    slots: &mut [ContainerSlot],
    destination: ResultTakeDestination<'_>,
) -> bool {
    if !destination.may_take() {
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

    destination.take(slots, source_index, &[input_index])
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
