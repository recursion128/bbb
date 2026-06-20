use super::*;

impl MerchantOffersState {
    pub(super) fn from_packet(packet: ProtocolMerchantOffers) -> Self {
        Self {
            container_id: packet.container_id,
            offers: packet
                .offers
                .into_iter()
                .map(MerchantOfferState::from_packet)
                .collect(),
            villager_level: packet.villager_level,
            villager_xp: packet.villager_xp,
            show_progress: packet.show_progress,
            can_restock: packet.can_restock,
            local_selected_offer_index: 0,
            local_scroll_offset: 0,
        }
    }
}

impl MerchantOfferState {
    fn from_packet(packet: ProtocolMerchantOffer) -> Self {
        Self {
            buy_a: packet.buy_a,
            sell: packet.sell,
            buy_b: packet.buy_b,
            is_out_of_stock: packet.is_out_of_stock,
            uses: packet.uses,
            max_uses: packet.max_uses,
            xp: packet.xp,
            special_price_diff: packet.special_price_diff,
            price_multiplier: packet.price_multiplier,
            demand: packet.demand,
        }
    }
}

pub(super) fn merchant_max_scroll_offset(offer_count: usize) -> i32 {
    offer_count
        .saturating_sub(MERCHANT_VISIBLE_OFFER_COUNT)
        .try_into()
        .unwrap_or(i32::MAX)
}

pub(super) fn apply_merchant_selected_offer_payment_autofill_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    offer: &MerchantOfferState,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    if !merchant_offer_supports_local_payment_autofill(offer) {
        return;
    }
    if !merchant_move_payment_slot_to_player(
        container_id,
        slots,
        MERCHANT_PAYMENT_SLOT_1,
        default_item_max_stack_sizes,
    ) {
        return;
    }
    if !merchant_move_payment_slot_to_player(
        container_id,
        slots,
        MERCHANT_PAYMENT_SLOT_2,
        default_item_max_stack_sizes,
    ) {
        return;
    }
    if inventory_menu_slot_has_item(slots, MERCHANT_PAYMENT_SLOT_1)
        || inventory_menu_slot_has_item(slots, MERCHANT_PAYMENT_SLOT_2)
    {
        return;
    }

    merchant_move_inventory_items_to_payment_slot(
        slots,
        MERCHANT_PAYMENT_SLOT_1,
        &offer.buy_a,
        default_item_max_stack_sizes,
    );
    if let Some(cost_b) = &offer.buy_b {
        merchant_move_inventory_items_to_payment_slot(
            slots,
            MERCHANT_PAYMENT_SLOT_2,
            cost_b,
            default_item_max_stack_sizes,
        );
    }
}

fn merchant_offer_supports_local_payment_autofill(offer: &MerchantOfferState) -> bool {
    merchant_item_cost_supports_local_payment_autofill(&offer.buy_a)
        && offer
            .buy_b
            .as_ref()
            .is_none_or(merchant_item_cost_supports_local_payment_autofill)
}

fn merchant_item_cost_supports_local_payment_autofill(cost: &ProtocolItemCostSummary) -> bool {
    cost.item_id >= 0
        && cost.component_predicate.component_count == 0
        && cost.component_predicate.component_type_ids.is_empty()
}

fn merchant_move_payment_slot_to_player(
    container_id: i32,
    slots: &mut [ContainerSlot],
    payment_slot: i16,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    let Some(source_index) = slots.iter().position(|slot| slot.slot == payment_slot) else {
        return false;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return true;
    }

    let mut moving = slots[source_index].item.clone();
    if !move_item_stack_to_slots(
        container_id,
        slots,
        source_index,
        &mut moving,
        MERCHANT_PLAYER_MAIN_START,
        MERCHANT_HOTBAR_END,
        true,
        default_item_max_stack_sizes,
    ) {
        return false;
    }
    normalize_item_stack(&mut moving);
    slots[source_index].item = moving;
    normalize_container_slot_selection(&mut slots[source_index]);
    true
}

fn merchant_move_inventory_items_to_payment_slot(
    slots: &mut [ContainerSlot],
    payment_slot: i16,
    cost: &ProtocolItemCostSummary,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    let Some(payment_index) = slots.iter().position(|slot| slot.slot == payment_slot) else {
        return false;
    };
    let mut changed = false;
    for source_slot in MERCHANT_PLAYER_MAIN_START..MERCHANT_HOTBAR_END {
        let Some(source_index) = slots.iter().position(|slot| slot.slot == source_slot) else {
            continue;
        };
        if !merchant_item_cost_matches_stack(cost, &slots[source_index].item) {
            continue;
        }
        let current_payment = slots[payment_index].item.clone();
        if item_stack_is_non_empty(&current_payment)
            && !same_item_same_components(&slots[source_index].item, &current_payment)
        {
            continue;
        }

        let max_stack_size =
            item_stack_max_stack_size(&slots[source_index].item, default_item_max_stack_sizes);
        let current_count = if item_stack_is_empty(&current_payment) {
            0
        } else {
            current_payment.count
        };
        let moved = (max_stack_size - current_count)
            .min(slots[source_index].item.count)
            .max(0);
        if moved <= 0 {
            continue;
        }

        let mut new_payment = slots[source_index].item.clone();
        new_payment.count = current_count + moved;
        slots[payment_index].item = new_payment;
        normalize_container_slot_selection(&mut slots[payment_index]);

        slots[source_index].item.count -= moved;
        normalize_item_stack(&mut slots[source_index].item);
        normalize_container_slot_selection(&mut slots[source_index]);
        changed = true;

        if slots[payment_index].item.count >= max_stack_size {
            break;
        }
    }
    changed
}

fn merchant_item_cost_matches_stack(
    cost: &ProtocolItemCostSummary,
    stack: &ProtocolItemStackSummary,
) -> bool {
    item_stack_is_non_empty(stack) && stack.item_id == Some(cost.item_id)
}

pub(super) fn apply_merchant_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    selected_offer: Option<&MerchantOfferState>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    if !(0..MERCHANT_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return false;
    }
    if slot_num == MERCHANT_RESULT_SLOT {
        return apply_merchant_result_quick_move_to_slots(
            container_id,
            slots,
            selected_offer,
            default_item_max_stack_sizes,
        );
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return false;
    };
    if item_stack_is_empty(&slots[source_index].item) {
        return false;
    }

    let source_item = slots[source_index].item.clone();
    let target = match slot_num {
        MERCHANT_PAYMENT_SLOT_1 | MERCHANT_PAYMENT_SLOT_2 => {
            Some((MERCHANT_PLAYER_MAIN_START, MERCHANT_HOTBAR_END, false))
        }
        slot if (MERCHANT_PLAYER_MAIN_START..MERCHANT_PLAYER_MAIN_END).contains(&slot) => {
            Some((MERCHANT_HOTBAR_START, MERCHANT_HOTBAR_END, false))
        }
        slot if (MERCHANT_HOTBAR_START..MERCHANT_HOTBAR_END).contains(&slot) => {
            Some((MERCHANT_PLAYER_MAIN_START, MERCHANT_PLAYER_MAIN_END, false))
        }
        _ => None,
    };
    let Some((start_slot, end_slot, backwards)) = target else {
        return false;
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
    false
}

fn apply_merchant_result_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    selected_offer: Option<&MerchantOfferState>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    let Some(offer) = selected_offer else {
        return false;
    };
    if merchant_offer_result_take_requires_server_authority(offer) {
        return false;
    }
    let Some(source_index) = slots
        .iter()
        .position(|slot| slot.slot == MERCHANT_RESULT_SLOT)
    else {
        return false;
    };
    let Some(payment_1_index) = slots
        .iter()
        .position(|slot| slot.slot == MERCHANT_PAYMENT_SLOT_1)
    else {
        return false;
    };
    let Some(payment_2_index) = slots
        .iter()
        .position(|slot| slot.slot == MERCHANT_PAYMENT_SLOT_2)
    else {
        return false;
    };
    let payment_take = merchant_payment_slots_satisfy_offer(
        &slots[payment_1_index].item,
        &slots[payment_2_index].item,
        offer,
        default_item_max_stack_sizes,
    );
    if item_stack_is_empty(&slots[source_index].item)
        || !merchant_result_matches_offer(&slots[source_index].item, offer)
        || payment_take.is_none()
    {
        return false;
    }
    let payment_take = payment_take.expect("checked payment take");

    let mut trial = slots.to_vec();
    let mut moving = slots[source_index].item.clone();
    if !move_item_stack_to_slots(
        container_id,
        &mut trial,
        source_index,
        &mut moving,
        MERCHANT_PLAYER_MAIN_START,
        MERCHANT_HOTBAR_END,
        true,
        default_item_max_stack_sizes,
    ) || !item_stack_is_empty(&moving)
    {
        return false;
    }

    merchant_apply_payment_take_and_update_result(
        &mut trial,
        payment_1_index,
        payment_2_index,
        source_index,
        payment_take,
        offer,
        default_item_max_stack_sizes,
    );
    slots.clone_from_slice(&trial);
    true
}

pub(super) fn apply_merchant_result_pickup_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    button_num: i8,
    selected_offer: Option<&MerchantOfferState>,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> bool {
    if button_num != 0 || item_stack_is_non_empty(cursor) {
        return false;
    }
    let Some(offer) = selected_offer else {
        return false;
    };
    if merchant_offer_result_take_requires_server_authority(offer) {
        return false;
    }
    let Some(source_index) = slots
        .iter()
        .position(|slot| slot.slot == MERCHANT_RESULT_SLOT)
    else {
        return false;
    };
    let Some(payment_1_index) = slots
        .iter()
        .position(|slot| slot.slot == MERCHANT_PAYMENT_SLOT_1)
    else {
        return false;
    };
    let Some(payment_2_index) = slots
        .iter()
        .position(|slot| slot.slot == MERCHANT_PAYMENT_SLOT_2)
    else {
        return false;
    };
    let payment_take = merchant_payment_slots_satisfy_offer(
        &slots[payment_1_index].item,
        &slots[payment_2_index].item,
        offer,
        default_item_max_stack_sizes,
    );
    if item_stack_is_empty(&slots[source_index].item)
        || !merchant_result_matches_offer(&slots[source_index].item, offer)
        || payment_take.is_none()
    {
        return false;
    }
    let payment_take = payment_take.expect("checked payment take");

    let mut trial = slots.to_vec();
    merchant_apply_payment_take_and_update_result(
        &mut trial,
        payment_1_index,
        payment_2_index,
        source_index,
        payment_take,
        offer,
        default_item_max_stack_sizes,
    );
    *cursor = slots[source_index].item.clone();
    slots.clone_from_slice(&trial);
    true
}

fn merchant_offer_result_take_requires_server_authority(offer: &MerchantOfferState) -> bool {
    offer.is_out_of_stock
        || offer.uses >= offer.max_uses
        || !merchant_offer_supports_local_payment_autofill(offer)
}

fn merchant_apply_payment_take_and_update_result(
    slots: &mut [ContainerSlot],
    payment_1_index: usize,
    payment_2_index: usize,
    source_index: usize,
    payment_take: MerchantPaymentTake,
    offer: &MerchantOfferState,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) {
    merchant_shrink_payment_slot(&mut slots[payment_1_index], payment_take.payment_1_count);
    normalize_container_slot_selection(&mut slots[payment_1_index]);
    merchant_shrink_payment_slot(&mut slots[payment_2_index], payment_take.payment_2_count);
    normalize_container_slot_selection(&mut slots[payment_2_index]);
    slots[source_index].item = if merchant_offer_remains_in_stock_after_take(offer)
        && merchant_payment_slots_satisfy_offer(
            &slots[payment_1_index].item,
            &slots[payment_2_index].item,
            offer,
            default_item_max_stack_sizes,
        )
        .is_some()
    {
        offer.sell.clone()
    } else {
        ProtocolItemStackSummary::empty()
    };
    normalize_container_slot_selection(&mut slots[source_index]);
}

fn merchant_result_matches_offer(
    result: &ProtocolItemStackSummary,
    offer: &MerchantOfferState,
) -> bool {
    item_stack_is_non_empty(&offer.sell)
        && result.count == offer.sell.count
        && same_item_same_components(result, &offer.sell)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MerchantPaymentTake {
    payment_1_count: i32,
    payment_2_count: i32,
}

fn merchant_payment_slots_satisfy_offer(
    payment_a: &ProtocolItemStackSummary,
    payment_b: &ProtocolItemStackSummary,
    offer: &MerchantOfferState,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> Option<MerchantPaymentTake> {
    let Some(cost_a_count) = merchant_modified_cost_a_count(offer, default_item_max_stack_sizes)
    else {
        return None;
    };
    if merchant_payment_slot_satisfies_cost(payment_a, &offer.buy_a, cost_a_count) {
        match &offer.buy_b {
            Some(cost_b)
                if merchant_payment_slot_satisfies_cost(payment_b, cost_b, cost_b.count) =>
            {
                return Some(MerchantPaymentTake {
                    payment_1_count: cost_a_count,
                    payment_2_count: cost_b.count,
                });
            }
            None if item_stack_is_empty(payment_b) => {
                return Some(MerchantPaymentTake {
                    payment_1_count: cost_a_count,
                    payment_2_count: 0,
                });
            }
            _ => {}
        }
    }

    match &offer.buy_b {
        Some(cost_b)
            if merchant_payment_slot_satisfies_cost(payment_a, cost_b, cost_b.count)
                && merchant_payment_slot_satisfies_cost(payment_b, &offer.buy_a, cost_a_count) =>
        {
            Some(MerchantPaymentTake {
                payment_1_count: cost_b.count,
                payment_2_count: cost_a_count,
            })
        }
        None if item_stack_is_empty(payment_a)
            && merchant_payment_slot_satisfies_cost(payment_b, &offer.buy_a, cost_a_count) =>
        {
            Some(MerchantPaymentTake {
                payment_1_count: 0,
                payment_2_count: cost_a_count,
            })
        }
        _ => None,
    }
}

fn merchant_payment_slot_satisfies_cost(
    payment: &ProtocolItemStackSummary,
    cost: &ProtocolItemCostSummary,
    required_count: i32,
) -> bool {
    required_count > 0
        && item_stack_is_non_empty(payment)
        && payment.item_id == Some(cost.item_id)
        && payment.count >= required_count
}

fn merchant_shrink_payment_slot(slot: &mut ContainerSlot, count: i32) {
    if count <= 0 {
        return;
    }
    slot.item.count -= count;
    normalize_item_stack(&mut slot.item);
}

fn merchant_offer_remains_in_stock_after_take(offer: &MerchantOfferState) -> bool {
    offer.uses.saturating_add(1) < offer.max_uses
}

pub(super) fn merchant_selected_offer_index(offers: &MerchantOffersState) -> Result<usize, ()> {
    let index = usize::try_from(offers.local_selected_offer_index).map_err(|_| ())?;
    (index < offers.offers.len()).then_some(index).ok_or(())
}

pub(super) fn merchant_increment_selected_offer_use(offers: &mut Option<MerchantOffersState>) {
    let Some(offers) = offers.as_mut() else {
        return;
    };
    let Ok(index) = merchant_selected_offer_index(offers) else {
        return;
    };
    let offer = &mut offers.offers[index];
    offer.uses = offer.uses.saturating_add(1);
    if offer.uses >= offer.max_uses {
        offer.is_out_of_stock = true;
    }
}

fn merchant_modified_cost_a_count(
    offer: &MerchantOfferState,
    default_item_max_stack_sizes: &BTreeMap<i32, i32>,
) -> Option<i32> {
    let cost = &offer.buy_a;
    if cost.item_id < 0 || cost.count <= 0 || !offer.price_multiplier.is_finite() {
        return None;
    }
    let demand_diff =
        ((cost.count as f32) * (offer.demand as f32) * offer.price_multiplier).floor();
    if !demand_diff.is_finite() {
        return None;
    }
    let demand_diff = (demand_diff as i64).max(0);
    let max_stack_size = default_item_max_stack_sizes
        .get(&cost.item_id)
        .copied()
        .map(clamp_vanilla_item_max_stack_size)
        .unwrap_or(VANILLA_DEFAULT_MAX_STACK_SIZE);
    let modified_count = i64::from(cost.count) + demand_diff + i64::from(offer.special_price_diff);
    Some(modified_count.clamp(1, i64::from(max_stack_size)) as i32)
}
