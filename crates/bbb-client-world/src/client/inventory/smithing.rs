use super::*;

pub(super) fn smithing_quick_move_requires_server_authority(
    slots: &[ContainerSlot],
    slot_num: i16,
    recipe_property_sets: &BTreeMap<String, Vec<i32>>,
) -> bool {
    if !(0..SMITHING_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return false;
    }
    if slot_num == SMITHING_RESULT_SLOT {
        return false;
    }
    if matches!(
        slot_num,
        SMITHING_TEMPLATE_SLOT | SMITHING_BASE_SLOT | SMITHING_ADDITIONAL_SLOT
    ) {
        return false;
    }
    if !(SMITHING_PLAYER_MAIN_START..SMITHING_HOTBAR_END).contains(&slot_num) {
        return false;
    }
    if !inventory_menu_slot_has_item(slots, slot_num) {
        return false;
    }
    !smithing_recipe_property_sets_available(recipe_property_sets)
}

pub(super) fn apply_smithing_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    recipe_property_sets: &BTreeMap<String, Vec<i32>>,
    profiles: &ItemProfiles,
) -> bool {
    if !(0..SMITHING_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return true;
    }
    if slot_num == SMITHING_RESULT_SLOT {
        let Some(source_index) = slots
            .iter()
            .position(|slot| slot.slot == SMITHING_RESULT_SLOT)
        else {
            return true;
        };
        if item_stack_is_empty(&slots[source_index].item) {
            return true;
        }
        let inputs_single = [
            SMITHING_TEMPLATE_SLOT,
            SMITHING_BASE_SLOT,
            SMITHING_ADDITIONAL_SLOT,
        ]
        .into_iter()
        .all(|input_slot| {
            slots
                .iter()
                .find(|slot| slot.slot == input_slot)
                .is_some_and(|slot| slot.item.count == 1)
        });
        if !inputs_single {
            // Multi-craft smithing loops in Vanilla using server-computed results.
            return false;
        }
        let _ = apply_smithing_result_to_slots(
            slots,
            ResultTakeDestination::PlayerInventory {
                container_id,
                player_slots: SMITHING_PLAYER_MAIN_START..SMITHING_HOTBAR_END,
                profiles,
            },
        );
        return true;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return true;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return true;
    }

    let source_item = slots[source_index].item.clone();
    let mut moving = source_item.clone();
    let moved = if matches!(
        slot_num,
        SMITHING_TEMPLATE_SLOT | SMITHING_BASE_SLOT | SMITHING_ADDITIONAL_SLOT
    ) {
        move_item_stack_to_slots(
            container_id,
            slots,
            source_index,
            &mut moving,
            SMITHING_PLAYER_MAIN_START,
            SMITHING_HOTBAR_END,
            false,
            profiles,
        )
    } else if !smithing_recipe_property_sets_available(recipe_property_sets) {
        false
    } else if smithing_can_move_into_input_slots(&source_item, slots, recipe_property_sets) {
        move_item_stack_to_slots_where(
            container_id,
            slots,
            source_index,
            &mut moving,
            SMITHING_TEMPLATE_SLOT,
            SMITHING_RESULT_SLOT,
            false,
            |dest_slot| {
                smithing_input_slot_accepts_stack(dest_slot, &source_item, recipe_property_sets)
            },
            profiles,
        )
    } else if (SMITHING_PLAYER_MAIN_START..SMITHING_PLAYER_MAIN_END).contains(&slot_num) {
        move_item_stack_to_slots(
            container_id,
            slots,
            source_index,
            &mut moving,
            SMITHING_HOTBAR_START,
            SMITHING_HOTBAR_END,
            false,
            profiles,
        )
    } else {
        move_item_stack_to_slots(
            container_id,
            slots,
            source_index,
            &mut moving,
            SMITHING_PLAYER_MAIN_START,
            SMITHING_PLAYER_MAIN_END,
            false,
            profiles,
        )
    };

    if moved {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        slots[source_index].normalize_selection();
    }
    true
}

pub(super) fn apply_smithing_result_to_slots(
    slots: &mut [ContainerSlot],
    destination: ResultTakeDestination<'_>,
) -> bool {
    if !destination.may_take() {
        return false;
    }
    let Some(source_index) = slots
        .iter()
        .position(|slot| slot.slot == SMITHING_RESULT_SLOT)
    else {
        return false;
    };
    let Some(template_index) = slots
        .iter()
        .position(|slot| slot.slot == SMITHING_TEMPLATE_SLOT)
    else {
        return false;
    };
    let Some(base_index) = slots
        .iter()
        .position(|slot| slot.slot == SMITHING_BASE_SLOT)
    else {
        return false;
    };
    let Some(additional_index) = slots
        .iter()
        .position(|slot| slot.slot == SMITHING_ADDITIONAL_SLOT)
    else {
        return false;
    };
    if item_stack_is_empty(&slots[source_index].item)
        || item_stack_is_empty(&slots[template_index].item)
        || item_stack_is_empty(&slots[base_index].item)
        || item_stack_is_empty(&slots[additional_index].item)
        || slots[template_index].item.count != 1
        || slots[base_index].item.count != 1
        || slots[additional_index].item.count != 1
    {
        return false;
    }

    let inputs = [template_index, base_index, additional_index];
    destination.take(slots, source_index, &inputs)
}

fn smithing_recipe_property_sets_available(
    recipe_property_sets: &BTreeMap<String, Vec<i32>>,
) -> bool {
    [
        SMITHING_TEMPLATE_PROPERTY_SET,
        SMITHING_BASE_PROPERTY_SET,
        SMITHING_ADDITION_PROPERTY_SET,
    ]
    .into_iter()
    .all(|property_set| recipe_property_sets.contains_key(property_set))
}

fn smithing_can_move_into_input_slots(
    stack: &ProtocolItemStackSummary,
    slots: &[ContainerSlot],
    recipe_property_sets: &BTreeMap<String, Vec<i32>>,
) -> bool {
    [
        SMITHING_TEMPLATE_SLOT,
        SMITHING_BASE_SLOT,
        SMITHING_ADDITIONAL_SLOT,
    ]
    .into_iter()
    .any(|slot_num| {
        !inventory_menu_slot_has_item(slots, slot_num)
            && smithing_input_slot_accepts_stack(slot_num, stack, recipe_property_sets)
    })
}

pub(super) fn smithing_input_slot_accepts_stack(
    slot_num: i16,
    stack: &ProtocolItemStackSummary,
    recipe_property_sets: &BTreeMap<String, Vec<i32>>,
) -> bool {
    let Some(item_id) = stack.item_id else {
        return false;
    };
    let Some(property_set) = smithing_input_slot_property_set(slot_num) else {
        return false;
    };
    recipe_property_sets
        .get(property_set)
        .is_some_and(|items| items.contains(&item_id))
}

fn smithing_input_slot_property_set(slot_num: i16) -> Option<&'static str> {
    match slot_num {
        SMITHING_TEMPLATE_SLOT => Some(SMITHING_TEMPLATE_PROPERTY_SET),
        SMITHING_BASE_SLOT => Some(SMITHING_BASE_PROPERTY_SET),
        SMITHING_ADDITIONAL_SLOT => Some(SMITHING_ADDITION_PROPERTY_SET),
        _ => None,
    }
}
