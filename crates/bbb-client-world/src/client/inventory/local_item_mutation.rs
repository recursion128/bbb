use super::*;

impl InventoryState {
    pub(crate) fn predict_writable_book_pages(
        &mut self,
        slot_id: i32,
        pages: Vec<bbb_protocol::JavaString>,
    ) -> bool {
        let Some(slot) = self.player_slot_mut(slot_id) else {
            return false;
        };
        let patch = &mut slot.item.component_patch;
        patch.removed_type_ids.retain(|component_id| {
            *component_id != VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID
        });
        if !patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID)
        {
            patch
                .added_type_ids
                .push(VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID);
        }
        patch.writable_book_page_filters = vec![None; pages.len()];
        patch.writable_book_pages = pages;
        slot.local_selected_bundle_item_index = NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX;
        true
    }

    /// Applies the client-side half of `BundleContents.Mutable.removeOne`.
    ///
    /// `BundleContents` defaults to empty, so a non-empty value must be an
    /// explicit stack patch. The transient selected index is kept beside the
    /// inventory slot because vanilla's bundle stream codec does not serialize
    /// it. Removing an invalid selection falls back to the first entry and
    /// always clears the selection.
    pub(crate) fn remove_one_local_bundle_item(
        &mut self,
        selected_hotbar_slot: u8,
        hand: InteractionHand,
    ) -> bool {
        let Some(slot_id) = local_player_hand_inventory_slot(selected_hotbar_slot, hand) else {
            return false;
        };
        let Some(slot) = self.player_slot_mut(slot_id) else {
            return false;
        };
        if !slot.item.item_id.is_some_and(|item_id| {
            vanilla_item_type(item_id).and_then(|item_type| item_type.use_override)
                == Some(VanillaItemUseOverride::BundleItem)
        }) || slot
            .item
            .component_patch
            .explicit_bundle_contents()
            .is_none()
        {
            return false;
        }

        let patch = &mut slot.item.component_patch;
        if patch.bundle_contents_items.is_empty() {
            return false;
        }
        let selected = usize::try_from(slot.local_selected_bundle_item_index)
            .ok()
            .filter(|selected| *selected < patch.bundle_contents_items.len())
            .unwrap_or(0);
        patch.bundle_contents_items.remove(selected);
        if patch.bundle_contents_items.is_empty() {
            patch
                .added_type_ids
                .retain(|type_id| *type_id != VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID);
        }
        slot.local_selected_bundle_item_index = NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX;
        true
    }

    pub(crate) fn local_item_use_prefers_offhand(&self, ctx: InventoryCtx<'_>) -> bool {
        !self
            .selected_hotbar_item(ctx.local_player.selected_hotbar_slot)
            .is_some_and(item_stack_is_non_empty)
            && self
                .local_offhand_item()
                .is_some_and(item_stack_is_non_empty)
    }

    pub(crate) fn local_item_in_hand_is_non_empty(
        &self,
        ctx: InventoryCtx<'_>,
        hand: InteractionHand,
    ) -> bool {
        self.local_item_in_hand(ctx, hand).is_some()
    }

    pub(crate) fn local_item_in_hand(
        &self,
        ctx: InventoryCtx<'_>,
        hand: InteractionHand,
    ) -> Option<&ProtocolItemStackSummary> {
        let slot_id =
            local_player_hand_inventory_slot(ctx.local_player.selected_hotbar_slot, hand)?;
        self.local_player_inventory_item(slot_id)
            .filter(|item| item_stack_is_non_empty(item))
    }

    pub(crate) fn local_player_has_item_in_tag(
        &self,
        ctx: InventoryCtx<'_>,
        tag: &str,
    ) -> Option<bool> {
        let item_ids = ctx.registries.holder_ids_for_tag("minecraft:item", tag)?;
        Some(
            (0..=VANILLA_PLAYER_INVENTORY_SADDLE_SLOT)
                .filter_map(|slot| self.local_player_inventory_item(slot))
                .filter(|item| item_stack_is_non_empty(item))
                .filter_map(|item| item.item_id)
                .any(|item_id| item_ids.contains(&item_id)),
        )
    }

    pub(crate) fn local_selected_main_hand_attack_range(
        &self,
        ctx: InventoryCtx<'_>,
    ) -> Option<ItemAttackRange> {
        let selected_slot = ctx.local_player.selected_hotbar_slot;
        if selected_slot > 8 {
            return None;
        }

        self.local_player_inventory_item(i32::from(selected_slot))
            .and_then(|item| item_stack_attack_range(item, ctx.items))
    }

    pub(crate) fn local_selected_main_hand_minimum_attack_charge(
        &self,
        ctx: InventoryCtx<'_>,
    ) -> f32 {
        self.local_item_in_hand(ctx, InteractionHand::MainHand)
            .map_or(0.0, |item| {
                item_stack_minimum_attack_charge(item, ctx.items)
            })
    }

    pub(crate) fn local_item_in_hand_is_enabled(
        &self,
        ctx: InventoryCtx<'_>,
        hand: InteractionHand,
        vanilla_feature_enabled: bool,
    ) -> bool {
        let Some(item) = self.local_item_in_hand(ctx, hand) else {
            return true;
        };
        let Some(item_id) = item.item_id else {
            return false;
        };
        if vanilla_item_type(item_id).is_some() {
            return vanilla_feature_enabled;
        }
        !ctx.registries.content_registry.0.is_vanilla_only()
            && ctx
                .registries
                .content_registry
                .0
                .item_by_id(item_id)
                .is_some()
    }

    pub(crate) fn local_using_item_use_effects(
        &self,
        ctx: InventoryCtx<'_>,
    ) -> Option<ItemUseEffects> {
        if !ctx.local_player.interaction.using_item {
            return None;
        }

        let hand = ctx
            .local_player
            .interaction
            .using_item_hand
            .unwrap_or(InteractionHand::MainHand);
        let slot_id =
            local_player_hand_inventory_slot(ctx.local_player.selected_hotbar_slot, hand)?;
        let item = self.local_player_inventory_item(slot_id)?;

        item_stack_use_effects(item, ctx.items)
    }

    pub(crate) fn local_using_item_item_id(&self, ctx: InventoryCtx<'_>) -> Option<i32> {
        if !ctx.local_player.interaction.using_item {
            return None;
        }
        let hand = ctx
            .local_player
            .interaction
            .using_item_hand
            .unwrap_or(InteractionHand::MainHand);
        self.local_item_in_hand(ctx, hand)
            .and_then(|item| item.item_id.filter(|item_id| *item_id >= 0))
    }

    pub(crate) fn shrink_local_item_in_hand(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        hand: InteractionHand,
        amount: i32,
    ) -> bool {
        if amount <= 0 {
            return true;
        }
        let Some(slot_id) =
            local_player_hand_inventory_slot(ctx.local_player.selected_hotbar_slot, hand)
        else {
            return false;
        };
        let Some(slot) = self.player_slot_mut(slot_id) else {
            return false;
        };
        if item_stack_is_empty(&slot.item) || slot.item.count < amount {
            return false;
        }
        slot.item.count -= amount;
        normalize_item_stack(&mut slot.item);
        slot.normalize_selection();
        self.update_inventory_slot_count(counters);
        true
    }

    pub(crate) fn consume_local_item_in_hand(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        hand: InteractionHand,
        amount: i32,
    ) -> bool {
        if ctx
            .local_player
            .abilities
            .is_some_and(|abilities| abilities.instabuild)
        {
            return self.local_item_in_hand(ctx, hand).is_some();
        }
        self.shrink_local_item_in_hand(ctx, counters, hand, amount)
    }

    /// Applies the client-observable part of `Mob.usePlayerItem`, including
    /// `DataComponents.USE_REMAINDER`.
    ///
    /// `LocalPlayer` inherits the no-op
    /// `LivingEntity.handleExtraItemsCreatedOnUse`, so a remainder replaces
    /// only a consumed singleton. A remainder created while the original
    /// stack remains nonempty is discarded locally and left to server
    /// inventory correction.
    pub(crate) fn consume_local_mob_player_item(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        hand: InteractionHand,
        remainder: Option<ProtocolItemStackSummary>,
    ) -> bool {
        let Some(slot_id) =
            local_player_hand_inventory_slot(ctx.local_player.selected_hotbar_slot, hand)
        else {
            return false;
        };
        let Some(slot) = self.player_slot_mut(slot_id) else {
            return false;
        };
        if item_stack_is_empty(&slot.item) {
            return false;
        }
        if ctx
            .local_player
            .abilities
            .is_some_and(|abilities| abilities.instabuild)
        {
            return true;
        }

        slot.item.count -= 1;
        if slot.item.count <= 0 {
            slot.item = remainder.unwrap_or_else(ProtocolItemStackSummary::empty);
        }
        normalize_item_stack(&mut slot.item);
        slot.normalize_selection();
        self.update_inventory_slot_count(counters);
        true
    }

    /// Applies the client-side inventory mutation in
    /// `Equippable.swapWithEquipmentSlot`.
    ///
    /// The returned flag is true only when a stacked survival swap cannot add
    /// the displaced equipment to the inventory and vanilla calls
    /// `Player.drop`, which contributes an extra main-hand swing client-side.
    pub(crate) fn swap_local_item_with_equipment_slot(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        hand: InteractionHand,
        equipment_slot: EquipmentSlot,
        creative_mode: bool,
    ) -> Option<bool> {
        let hand_slot_id =
            local_player_hand_inventory_slot(ctx.local_player.selected_hotbar_slot, hand)?;
        let equipment_slot_id = local_player_equipment_inventory_slot(
            ctx.local_player.selected_hotbar_slot,
            equipment_slot,
        )?;
        if hand_slot_id == equipment_slot_id {
            return None;
        }

        let hand_slot = self.player_slot(hand_slot_id)?;
        if item_stack_is_empty(&hand_slot.item) {
            return None;
        }
        let hand_count = hand_slot.item.count;
        let mut equipped = hand_slot.item.clone();
        equipped.count = 1;
        let displaced = match self.player_slot_mut(equipment_slot_id) {
            Some(target_slot) => {
                let displaced = std::mem::replace(&mut target_slot.item, equipped);
                target_slot.normalize_selection();
                displaced
            }
            None => {
                self.set_player_slot(InventorySlot::new(equipment_slot_id, equipped));
                ProtocolItemStackSummary::empty()
            }
        };

        let hand_slot = self.player_slot_mut(hand_slot_id).unwrap();
        if hand_count <= 1 {
            if !creative_mode || !item_stack_is_empty(&displaced) {
                hand_slot.item = displaced;
            }
            normalize_item_stack(&mut hand_slot.item);
            hand_slot.normalize_selection();
            self.update_inventory_slot_count(counters);
            return Some(false);
        }

        let instabuild = ctx
            .local_player
            .abilities
            .is_some_and(|abilities| abilities.instabuild);
        if !instabuild {
            hand_slot.item.count -= 1;
            normalize_item_stack(&mut hand_slot.item);
            hand_slot.normalize_selection();
        }

        let added = item_stack_is_empty(&displaced)
            || self.add_local_item_to_player_inventory(ctx, displaced)
            || instabuild;
        self.update_inventory_slot_count(counters);
        Some(!added)
    }

    /// Applies `ItemUtils.createFilledResult`, or the direct consumption used by
    /// `BeehiveBlock`. The drop flag projects the extra client-side hand swing.
    pub(super) fn replace_local_item_with_filled_result(
        &mut self,
        ctx: InventoryCtx<'_>,
        counters: &mut WorldCounters,
        creative: CreativeFilledResult,
        hand: InteractionHand,
        expected_item_id: i32,
        filled: ProtocolItemStackSummary,
    ) -> Option<bool> {
        let hand_slot_id =
            local_player_hand_inventory_slot(ctx.local_player.selected_hotbar_slot, hand)?;
        let hand_slot = self.player_slot_mut(hand_slot_id)?;
        if item_stack_is_empty(&hand_slot.item)
            || hand_slot.item.item_id != Some(expected_item_id)
            || item_stack_is_empty(&filled)
        {
            return None;
        }

        let instabuild = ctx
            .local_player
            .abilities
            .is_some_and(|abilities| abilities.instabuild);
        if instabuild && creative != CreativeFilledResult::ConsumeInput {
            let already_contains_filled = creative == CreativeFilledResult::AddIfMissing
                && self.player_slots.iter().any(|slot| {
                    (0..=VANILLA_PLAYER_INVENTORY_SADDLE_SLOT).contains(&slot.slot)
                        && !item_stack_is_empty(&slot.item)
                        && same_item_same_components(&slot.item, &filled)
                });
            if !already_contains_filled && self.add_local_item_to_player_inventory(ctx, filled) {
                self.update_inventory_slot_count(counters);
            }
            return Some(false);
        }

        if hand_slot.item.count == 1 {
            hand_slot.item = filled;
            hand_slot.normalize_selection();
            self.update_inventory_slot_count(counters);
            return Some(false);
        }

        hand_slot.item.count -= 1;
        normalize_item_stack(&mut hand_slot.item);
        hand_slot.normalize_selection();
        let added = self.add_local_item_to_player_inventory(ctx, filled);
        self.update_inventory_slot_count(counters);
        Some(!added && !instabuild)
    }

    /// Mirrors `Inventory.add` search order: selected slot, offhand, compatible
    /// non-equipment slots, then empty non-equipment slots. Returns true only
    /// when the complete input stack was inserted.
    pub(super) fn add_local_item_to_player_inventory(
        &mut self,
        ctx: InventoryCtx<'_>,
        mut item: ProtocolItemStackSummary,
    ) -> bool {
        if item_stack_is_empty(&item) {
            return false;
        }
        let selected_slot = i32::from(ctx.local_player.selected_hotbar_slot.min(8));
        for slot_id in [selected_slot, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT]
            .into_iter()
            .chain(0..36)
        {
            let Some(slot) = self.player_slot_mut(slot_id) else {
                continue;
            };
            if item_stack_is_empty(&slot.item) || !same_item_same_components(&slot.item, &item) {
                continue;
            }
            let max_stack_size = ctx.items.item_stack_max_stack_size(&slot.item);
            let moved = item.count.min(max_stack_size - slot.item.count);
            if moved <= 0 {
                continue;
            }
            slot.item.count += moved;
            item.count -= moved;
            slot.normalize_selection();
            if item.count == 0 {
                return true;
            }
        }
        for slot_id in 0..36 {
            if self
                .player_slot(slot_id)
                .is_some_and(|slot| !item_stack_is_empty(&slot.item))
            {
                continue;
            }
            let mut inserted = item.clone();
            inserted.count = item.count.min(ctx.items.item_stack_max_stack_size(&item));
            item.count -= inserted.count;
            self.set_player_slot(InventorySlot::new(slot_id, inserted));
            if item.count == 0 {
                return true;
            }
        }
        false
    }

    pub(super) fn local_offhand_item(&self) -> Option<&ProtocolItemStackSummary> {
        self.local_offhand_slot().map(|(item, _)| item)
    }

    pub(super) fn local_offhand_slot(&self) -> Option<(&ProtocolItemStackSummary, i32)> {
        self.player_slot(VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT)
            .map(|row| (&row.item, row.local_selected_bundle_item_index))
    }

    pub(crate) fn local_player_has_freeze_immune_wearable(&self, ctx: InventoryCtx<'_>) -> bool {
        [
            VANILLA_PLAYER_INVENTORY_FEET_SLOT,
            VANILLA_PLAYER_INVENTORY_LEGS_SLOT,
            VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
            VANILLA_PLAYER_INVENTORY_HEAD_SLOT,
            VANILLA_PLAYER_INVENTORY_BODY_SLOT,
        ]
        .into_iter()
        .filter_map(|slot| self.local_player_inventory_item(slot))
        .filter(|item| item.count > 0)
        .filter_map(|item| item.item_id)
        .any(|item_id| ctx.items.freeze_immune_wearable_item_ids.contains(&item_id))
    }

    pub(crate) fn local_player_can_walk_on_powder_snow(&self, ctx: InventoryCtx<'_>) -> bool {
        self.local_player_inventory_item(VANILLA_PLAYER_INVENTORY_FEET_SLOT)
            .filter(|item| item.count > 0)
            .and_then(|item| item.item_id)
            .is_some_and(|item_id| {
                ctx.items
                    .powder_snow_walkable_foot_item_ids
                    .contains(&item_id)
            })
    }

    pub(super) fn local_player_inventory_item(
        &self,
        slot_id: i32,
    ) -> Option<&ProtocolItemStackSummary> {
        self.player_slot(slot_id).map(|row| &row.item)
    }
}
