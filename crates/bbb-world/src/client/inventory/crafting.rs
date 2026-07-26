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
    external_offhand: Option<&mut ProtocolItemStackSummary>,
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
    let Some(input_slot_nums) =
        crafting_menu_predictable_input_slot_nums(slots, default_item_crafting_remainders_known)
    else {
        return false;
    };

    let mut trial = slots.to_vec();
    let mut moving = trial[source_index].item.clone();
    if !move_item_stack_to_slots(
        container_id,
        &mut trial,
        source_index,
        &mut moving,
        CRAFTING_MENU_PLAYER_MAIN_START,
        CRAFTING_MENU_HOTBAR_END,
        true,
        default_item_max_stack_sizes,
    ) {
        return false;
    }
    normalize_item_stack(&mut moving);
    trial[source_index].item = moving;
    normalize_container_slot_selection(&mut trial[source_index]);
    apply_crafting_result_take_side_effects(
        container_id,
        &mut trial,
        &input_slot_nums,
        default_item_crafting_remainders,
        PlayerInventoryAddSlots::crafting_menu(),
        external_offhand,
        selected_hotbar_slot,
        default_item_max_stack_sizes,
    );

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
    external_offhand: Option<&mut ProtocolItemStackSummary>,
    selected_hotbar_slot: u8,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    if !matches!(button_num, 0 | 1) {
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
    let amount = if item_stack_is_empty(cursor) {
        if button_num == 0 {
            slots[source_index].item.count
        } else {
            (slots[source_index].item.count + 1) / 2
        }
    } else if same_item_same_components(&slots[source_index].item, cursor) {
        let cursor_max_stack_size = item_stack_max_stack_size(cursor, default_item_max_stack_sizes);
        slots[source_index]
            .item
            .count
            .min((cursor_max_stack_size - cursor.count).max(0))
    } else {
        0
    };
    if amount <= 0 {
        return true;
    }
    let Some(input_slot_nums) =
        crafting_menu_predictable_input_slot_nums(slots, default_item_crafting_remainders_known)
    else {
        return false;
    };

    let mut trial = slots.to_vec();
    if item_stack_is_empty(cursor) {
        move_stack_count(&mut trial[source_index].item, cursor, amount);
    } else {
        trial[source_index].item.count -= amount;
        cursor.count += amount;
        normalize_item_stack(&mut trial[source_index].item);
    }
    normalize_container_slot_selection(&mut trial[source_index]);
    apply_crafting_result_take_side_effects(
        container_id,
        &mut trial,
        &input_slot_nums,
        default_item_crafting_remainders,
        PlayerInventoryAddSlots::crafting_menu(),
        external_offhand,
        selected_hotbar_slot,
        default_item_max_stack_sizes,
    );
    slots.clone_from_slice(&trial);
    true
}

fn crafting_menu_predictable_input_slot_nums(
    slots: &[ContainerSlot],
    default_item_crafting_remainders_known: bool,
) -> Option<Vec<i16>> {
    crafting_result_predictable_input_slot_nums(
        slots,
        CRAFTING_MENU_CRAFT_SLOT_START,
        CRAFTING_MENU_CRAFT_SLOT_END,
        default_item_crafting_remainders_known,
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
