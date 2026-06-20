use super::*;

pub(super) fn apply_crafting_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if slot_num <= CRAFTING_MENU_RESULT_SLOT || slot_num >= CRAFTING_MENU_TOTAL_SLOT_COUNT {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let mut moving = slots[source_index].item.clone();
    let moved = if (CRAFTING_MENU_PLAYER_MAIN_START..CRAFTING_MENU_HOTBAR_END).contains(&slot_num) {
        move_item_stack_to_slots(
            container_id,
            slots,
            source_index,
            &mut moving,
            CRAFTING_MENU_CRAFT_SLOT_START,
            CRAFTING_MENU_CRAFT_SLOT_END,
            false,
            default_item_max_stack_sizes,
        ) || if (CRAFTING_MENU_PLAYER_MAIN_START..CRAFTING_MENU_PLAYER_MAIN_END).contains(&slot_num)
        {
            move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                CRAFTING_MENU_HOTBAR_START,
                CRAFTING_MENU_HOTBAR_END,
                false,
                default_item_max_stack_sizes,
            )
        } else {
            move_item_stack_to_slots(
                container_id,
                slots,
                source_index,
                &mut moving,
                CRAFTING_MENU_PLAYER_MAIN_START,
                CRAFTING_MENU_PLAYER_MAIN_END,
                false,
                default_item_max_stack_sizes,
            )
        }
    } else {
        move_item_stack_to_slots(
            container_id,
            slots,
            source_index,
            &mut moving,
            CRAFTING_MENU_PLAYER_MAIN_START,
            CRAFTING_MENU_HOTBAR_END,
            false,
            default_item_max_stack_sizes,
        )
    };

    if moved {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

pub(super) fn apply_crafting_menu_result_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    default_item_crafting_remainders_known: bool,
    default_item_crafting_remainders: &BTreeMap<i32, i32>,
    recipe_specific_crafting_remainder_item_ids: &BTreeSet<i32>,
    selected_hotbar_slot: u8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    let Some(source_index) = slots
        .iter()
        .position(|slot| slot.slot == CRAFTING_MENU_RESULT_SLOT)
    else {
        return false;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return false;
    }
    let Some(input_slot_nums) = crafting_menu_predictable_input_slot_nums(
        slots,
        default_item_crafting_remainders_known,
        default_item_crafting_remainders,
        recipe_specific_crafting_remainder_item_ids,
    ) else {
        return false;
    };
    let max_crafts = input_slot_nums
        .iter()
        .filter_map(|slot_num| container_slot_item(slots, *slot_num).map(|item| item.count))
        .min()
        .unwrap_or(0)
        .max(0);
    if max_crafts <= 0 {
        return false;
    }

    let result_template = slots[source_index].item.clone();
    let mut trial = slots.to_vec();
    for craft_index in 0..max_crafts {
        let result_still_same =
            container_slot_item(&trial, CRAFTING_MENU_RESULT_SLOT).is_some_and(|item| {
                item_stack_is_non_empty(item) && same_item_same_components(item, &result_template)
            });
        if !result_still_same {
            return false;
        }

        let mut moving = result_template.clone();
        if !move_item_stack_to_slots(
            container_id,
            &mut trial,
            source_index,
            &mut moving,
            CRAFTING_MENU_PLAYER_MAIN_START,
            CRAFTING_MENU_HOTBAR_END,
            true,
            default_item_max_stack_sizes,
        ) || !item_stack_is_empty(&moving)
        {
            return false;
        }

        trial[source_index].item = ProtocolItemStackSummary::empty();
        normalize_container_slot_selection(&mut trial[source_index]);
        let Some(side_effects) = apply_crafting_result_take_side_effects_for_slots(
            container_id,
            &mut trial,
            &input_slot_nums,
            default_item_crafting_remainders,
            None,
            selected_hotbar_slot,
            default_item_max_stack_sizes,
        ) else {
            return false;
        };
        if craft_index + 1 < max_crafts {
            if !side_effects.inputs_can_still_take_result {
                return false;
            }
            trial[source_index].item = result_template.clone();
            normalize_container_slot_selection(&mut trial[source_index]);
        }
    }

    slots.clone_from_slice(&trial);
    true
}

pub(super) fn apply_crafting_menu_result_pickup_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    button_num: i8,
    default_item_crafting_remainders_known: bool,
    default_item_crafting_remainders: &BTreeMap<i32, i32>,
    recipe_specific_crafting_remainder_item_ids: &BTreeSet<i32>,
    selected_hotbar_slot: u8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    if button_num != 0 || !item_stack_is_empty(cursor) {
        return false;
    }
    let Some(source_index) = slots
        .iter()
        .position(|slot| slot.slot == CRAFTING_MENU_RESULT_SLOT)
    else {
        return false;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return false;
    }
    let Some(input_slot_nums) = crafting_menu_predictable_input_slot_nums(
        slots,
        default_item_crafting_remainders_known,
        default_item_crafting_remainders,
        recipe_specific_crafting_remainder_item_ids,
    ) else {
        return false;
    };

    let result_template = slots[source_index].item.clone();
    let mut trial = slots.to_vec();
    trial[source_index].item = ProtocolItemStackSummary::empty();
    normalize_container_slot_selection(&mut trial[source_index]);
    let Some(side_effects) = apply_crafting_result_take_side_effects_for_slots(
        container_id,
        &mut trial,
        &input_slot_nums,
        default_item_crafting_remainders,
        None,
        selected_hotbar_slot,
        default_item_max_stack_sizes,
    ) else {
        return false;
    };
    if side_effects.inputs_can_still_take_result {
        trial[source_index].item = result_template.clone();
        normalize_container_slot_selection(&mut trial[source_index]);
    }
    *cursor = result_template;
    slots.clone_from_slice(&trial);
    true
}

fn crafting_menu_predictable_input_slot_nums(
    slots: &[ContainerSlot],
    default_item_crafting_remainders_known: bool,
    default_item_crafting_remainders: &BTreeMap<i32, i32>,
    recipe_specific_crafting_remainder_item_ids: &BTreeSet<i32>,
) -> Option<Vec<i16>> {
    crafting_result_predictable_input_slot_nums(
        slots,
        CRAFTING_MENU_CRAFT_SLOT_START,
        CRAFTING_MENU_CRAFT_SLOT_END,
        default_item_crafting_remainders_known,
        default_item_crafting_remainders,
        recipe_specific_crafting_remainder_item_ids,
    )
}

pub(super) fn apply_crafter_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    disabled_slots: &BTreeSet<i16>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !(0..CRAFTER_TOTAL_SLOT_COUNT).contains(&slot_num) || slot_num == CRAFTER_RESULT_SLOT {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return;
    }

    let source_item = slots[source_index].item.clone();
    let (start_slot, end_slot, backwards) = if slot_num < CRAFTER_GRID_SLOT_COUNT {
        (CRAFTER_PLAYER_MAIN_START, CRAFTER_HOTBAR_END, true)
    } else {
        (0, CRAFTER_GRID_SLOT_COUNT, false)
    };

    let mut moving = source_item;
    if move_item_stack_to_slots_where(
        container_id,
        slots,
        source_index,
        &mut moving,
        start_slot,
        end_slot,
        backwards,
        |slot| !disabled_slots.contains(&slot),
        default_item_max_stack_sizes,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        normalize_container_slot_selection(&mut slots[source_index]);
    }
}

pub(super) fn crafter_disabled_slots(data_values: &[ContainerDataValue]) -> BTreeSet<i16> {
    data_values
        .iter()
        .filter_map(|value| {
            ((0..CRAFTER_GRID_SLOT_COUNT).contains(&value.id) && value.value == 1)
                .then_some(value.id)
        })
        .collect()
}
