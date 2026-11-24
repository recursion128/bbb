use bbb_protocol::data_component_types::vanilla_data_component_type;
use bbb_protocol::packets::data_components::fields::component_values_equal;

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

pub(super) fn apply_merchant_selected_offer_payment_autofill(
    inventory: &mut InventoryState,
    container_id: i32,
    buy_a: &ProtocolItemCostSummary,
    buy_b: Option<&ProtocolItemCostSummary>,
    profiles: &ItemProfiles,
) {
    if !merchant_costs_support_local_payment_autofill(buy_a, buy_b, profiles) {
        return;
    }
    for slot in [MERCHANT_PAYMENT_SLOT_1, MERCHANT_PAYMENT_SLOT_2] {
        if !merchant_move_payment_slot_to_player(inventory, container_id, slot, profiles) {
            return;
        }
    }
    if [MERCHANT_PAYMENT_SLOT_1, MERCHANT_PAYMENT_SLOT_2]
        .into_iter()
        .any(|slot| {
            inventory
                .container_slot(inventory.open_container.as_ref().unwrap(), slot)
                .is_some_and(|row| item_stack_is_non_empty(row.item))
        })
    {
        return;
    }
    merchant_move_inventory_items_to_payment_slot(
        inventory,
        container_id,
        MERCHANT_PAYMENT_SLOT_1,
        buy_a,
        profiles,
    );
    if let Some(cost_b) = buy_b {
        merchant_move_inventory_items_to_payment_slot(
            inventory,
            container_id,
            MERCHANT_PAYMENT_SLOT_2,
            cost_b,
            profiles,
        );
    }
}

fn merchant_costs_support_local_payment_autofill(
    buy_a: &ProtocolItemCostSummary,
    buy_b: Option<&ProtocolItemCostSummary>,
    profiles: &ItemProfiles,
) -> bool {
    merchant_item_cost_supports_local_payment_autofill(buy_a, profiles)
        && buy_b
            .is_none_or(|cost| merchant_item_cost_supports_local_payment_autofill(cost, profiles))
}

fn merchant_item_cost_supports_local_payment_autofill(
    cost: &ProtocolItemCostSummary,
    profiles: &ItemProfiles,
) -> bool {
    if cost.item_id < 0 {
        return false;
    }
    let defaults_patch = profiles.default_item_component_patches.get(&cost.item_id);
    let defaults_codec = profiles.default_component_codec_values.get(&cost.item_id);
    cost.component_predicate
        .components
        .added_type_ids
        .iter()
        .all(|component_id| {
            vanilla_data_component_type(*component_id).is_some()
                && !(defaults_codec.is_some_and(|codec| codec.contains_key(component_id))
                    && defaults_patch
                        .is_none_or(|patch| !patch.added_type_ids.contains(component_id)))
        })
}

fn merchant_move_payment_slot_to_player(
    inventory: &mut InventoryState,
    container_id: i32,
    payment_slot: i16,
    profiles: &ItemProfiles,
) -> bool {
    let Some(row) =
        inventory.container_slot(inventory.open_container.as_ref().unwrap(), payment_slot)
    else {
        return false;
    };
    if item_stack_is_empty(row.item) {
        return true;
    }
    let mut moving = row.item.clone();
    let mut changed = false;
    if profiles.item_stack_max_stack_size(&moving) > 1 {
        for slot in (MERCHANT_PLAYER_MAIN_START..MERCHANT_HOTBAR_END).rev() {
            if item_stack_is_empty(&moving) {
                break;
            }
            let Some((item, selection)) = inventory.menu_slot_mut(container_id, slot) else {
                continue;
            };
            if merge_item_stack_into_slot(&mut moving, item, |item| {
                profiles.item_stack_max_stack_size(item)
            }) {
                *selection = normalize_local_selected_bundle_item_index(*selection, item);
                changed = true;
            }
        }
    }
    if item_stack_is_non_empty(&moving) {
        for slot in (MERCHANT_PLAYER_MAIN_START..MERCHANT_HOTBAR_END).rev() {
            let Some((item, selection)) = inventory.menu_slot_mut(container_id, slot) else {
                continue;
            };
            if item_stack_is_non_empty(item) {
                continue;
            }
            let amount = moving
                .count
                .min(profiles.item_stack_max_stack_size(&moving));
            if amount <= 0 {
                continue;
            }
            move_stack_count(&mut moving, item, amount);
            *selection = normalize_local_selected_bundle_item_index(*selection, item);
            changed = true;
            break;
        }
    }
    if changed {
        let (item, selection) = inventory.menu_slot_mut(container_id, payment_slot).unwrap();
        *item = moving;
        *selection = normalize_local_selected_bundle_item_index(*selection, item);
    }
    changed
}

fn merchant_move_inventory_items_to_payment_slot(
    inventory: &mut InventoryState,
    container_id: i32,
    payment_slot: i16,
    cost: &ProtocolItemCostSummary,
    profiles: &ItemProfiles,
) {
    let Some(row) =
        inventory.container_slot(inventory.open_container.as_ref().unwrap(), payment_slot)
    else {
        return;
    };
    let mut payment = row.item.clone();
    for slot in MERCHANT_PLAYER_MAIN_START..MERCHANT_HOTBAR_END {
        let Some((item, selection)) = inventory.menu_slot_mut(container_id, slot) else {
            continue;
        };
        if !merchant_item_cost_matches_stack(cost, item, profiles)
            || (item_stack_is_non_empty(&payment) && !same_item_same_components(item, &payment))
        {
            continue;
        }
        let max_stack_size = profiles.item_stack_max_stack_size(item);
        let moved = (max_stack_size - payment.count).min(item.count).max(0);
        if moved <= 0 {
            continue;
        }
        let current_count = payment.count;
        move_stack_count(item, &mut payment, moved);
        payment.count += current_count;
        *selection = normalize_local_selected_bundle_item_index(*selection, item);
        if payment.count >= max_stack_size {
            break;
        }
    }
    let (item, selection) = inventory.menu_slot_mut(container_id, payment_slot).unwrap();
    *item = payment;
    *selection = normalize_local_selected_bundle_item_index(*selection, item);
}

/// Mirrors Vanilla `ItemCost.test` (`ItemCost.java:51-53`) and
/// `DataComponentExactPredicate.test` (`DataComponentExactPredicate.java:77-86`).
fn merchant_item_cost_matches_stack(
    cost: &ProtocolItemCostSummary,
    stack: &ProtocolItemStackSummary,
    profiles: &ItemProfiles,
) -> bool {
    if !item_stack_is_non_empty(stack) || stack.item_id != Some(cost.item_id) {
        return false;
    }
    if cost.component_predicate.is_empty() {
        return true;
    }
    let expected = &*cost.component_predicate.components;
    let stack_patch = &*stack.component_patch;
    let defaults_patch = profiles.default_item_component_patches.get(&cost.item_id);
    expected.added_type_ids.iter().all(|&component_id| {
        if stack_patch.removed_type_ids.contains(&component_id) {
            return false;
        }
        let actual_source = if stack_patch.added_type_ids.contains(&component_id) {
            stack_patch
        } else if let Some(defaults) =
            defaults_patch.filter(|defaults| defaults.added_type_ids.contains(&component_id))
        {
            defaults
        } else {
            return false;
        };
        component_values_equal(component_id, expected, actual_source).unwrap_or(false)
    })
}

pub(super) fn apply_merchant_menu_quick_move_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    selected_offer: Option<&MerchantOfferState>,
    profiles: &ItemProfiles,
) -> bool {
    if !(0..MERCHANT_TOTAL_SLOT_COUNT).contains(&slot_num) {
        return false;
    }
    if slot_num == MERCHANT_RESULT_SLOT {
        return apply_merchant_result_to_slots(
            slots,
            ResultTakeDestination::PlayerInventory {
                container_id,
                player_slots: MERCHANT_PLAYER_MAIN_START..MERCHANT_HOTBAR_END,
                profiles,
            },
            selected_offer,
            profiles,
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
        profiles,
    ) {
        normalize_item_stack(&mut moving);
        slots[source_index].item = moving;
        slots[source_index].normalize_selection();
    }
    false
}

pub(super) fn apply_merchant_result_to_slots(
    slots: &mut [ContainerSlot],
    destination: ResultTakeDestination<'_>,
    selected_offer: Option<&MerchantOfferState>,
    profiles: &ItemProfiles,
) -> bool {
    if !destination.may_take() {
        return false;
    }
    let Some(offer) = selected_offer else {
        return false;
    };
    if merchant_offer_result_take_requires_server_authority(offer, profiles) {
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
        profiles,
    );
    if item_stack_is_empty(&slots[source_index].item)
        || !merchant_result_matches_offer(&slots[source_index].item, offer)
        || payment_take.is_none()
    {
        return false;
    }
    let payment_take = payment_take.expect("checked payment take");

    let mut trial = slots.to_vec();
    match &destination {
        ResultTakeDestination::PlayerInventory {
            container_id,
            player_slots,
            profiles,
        } => {
            let mut moving = slots[source_index].item.clone();
            if !move_item_stack_to_slots(
                *container_id,
                &mut trial,
                source_index,
                &mut moving,
                player_slots.start,
                player_slots.end,
                true,
                profiles,
            ) || !item_stack_is_empty(&moving)
            {
                return false;
            }
        }
        ResultTakeDestination::Slot(dest_slot_num) => {
            let Some(dest_index) = trial.iter().position(|slot| slot.slot == *dest_slot_num) else {
                return false;
            };
            if dest_index == source_index || !item_stack_is_empty(&trial[dest_index].item) {
                return false;
            }
            trial[dest_index].item = slots[source_index].item.clone();
            trial[dest_index].normalize_selection();
        }
        ResultTakeDestination::Cursor(_, _) | ResultTakeDestination::Drop => {}
    }

    merchant_apply_payment_take_and_update_result(
        &mut trial,
        payment_1_index,
        payment_2_index,
        source_index,
        payment_take,
        offer,
        profiles,
    );
    if let ResultTakeDestination::Cursor(cursor, _) = destination {
        *cursor = slots[source_index].item.clone();
    }
    slots.clone_from_slice(&trial);
    true
}

pub(super) fn merchant_offer_result_take_requires_server_authority(
    offer: &MerchantOfferState,
    profiles: &ItemProfiles,
) -> bool {
    offer.is_out_of_stock
        || offer.uses >= offer.max_uses
        || !merchant_costs_support_local_payment_autofill(
            &offer.buy_a,
            offer.buy_b.as_ref(),
            profiles,
        )
}

fn merchant_apply_payment_take_and_update_result(
    slots: &mut [ContainerSlot],
    payment_1_index: usize,
    payment_2_index: usize,
    source_index: usize,
    payment_take: MerchantPaymentTake,
    offer: &MerchantOfferState,
    profiles: &ItemProfiles,
) {
    merchant_shrink_payment_slot(&mut slots[payment_1_index], payment_take.payment_1_count);
    slots[payment_1_index].normalize_selection();
    merchant_shrink_payment_slot(&mut slots[payment_2_index], payment_take.payment_2_count);
    slots[payment_2_index].normalize_selection();
    slots[source_index].item = if merchant_offer_remains_in_stock_after_take(offer)
        && merchant_payment_slots_satisfy_offer(
            &slots[payment_1_index].item,
            &slots[payment_2_index].item,
            offer,
            profiles,
        )
        .is_some()
    {
        offer.sell.clone()
    } else {
        ProtocolItemStackSummary::empty()
    };
    slots[source_index].normalize_selection();
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
    profiles: &ItemProfiles,
) -> Option<MerchantPaymentTake> {
    let cost_a_count = merchant_modified_cost_a_count(offer, profiles)?;
    if merchant_payment_slot_satisfies_cost(payment_a, &offer.buy_a, cost_a_count, profiles) {
        match &offer.buy_b {
            Some(cost_b)
                if merchant_payment_slot_satisfies_cost(
                    payment_b,
                    cost_b,
                    cost_b.count,
                    profiles,
                ) =>
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
            if merchant_payment_slot_satisfies_cost(payment_a, cost_b, cost_b.count, profiles)
                && merchant_payment_slot_satisfies_cost(
                    payment_b,
                    &offer.buy_a,
                    cost_a_count,
                    profiles,
                ) =>
        {
            Some(MerchantPaymentTake {
                payment_1_count: cost_b.count,
                payment_2_count: cost_a_count,
            })
        }
        None if item_stack_is_empty(payment_a)
            && merchant_payment_slot_satisfies_cost(
                payment_b,
                &offer.buy_a,
                cost_a_count,
                profiles,
            ) =>
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
    profiles: &ItemProfiles,
) -> bool {
    required_count > 0
        && payment.count >= required_count
        && merchant_item_cost_matches_stack(cost, payment, profiles)
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
    profiles: &ItemProfiles,
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
    let max_stack_size = profiles.item_max_stack_size_for_protocol_id(cost.item_id);
    let modified_count = i64::from(cost.count) + demand_diff + i64::from(offer.special_price_diff);
    Some(modified_count.clamp(1, i64::from(max_stack_size)) as i32)
}
