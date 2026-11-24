use super::*;

macro_rules! inventory_ctx {
    ($store:expr) => {
        InventoryCtx {
            items: &$store.items,
            local_player: &$store.local_player,
            entities: &$store.entities,
            recipes: &$store.recipes,
            registries: &$store.registries,
        }
    };
}

/// Facade delegation: the inventory method group lives on
/// [`InventoryState`] (and the item default tables on `ItemProfiles`);
/// `WorldStore` keeps the public signatures and forwards, splitting off
/// cross-domain borrows explicitly.
impl WorldStore {
    /// Decodes Vanilla's registry-aware `ItemStack.OPTIONAL_CODEC` NBT shape.
    /// An empty compound is the canonical empty-stack representation.
    pub fn persistent_item_stack_from_nbt(
        &self,
        value: &bbb_protocol::NbtSummaryValue,
    ) -> Result<ProtocolItemStackSummary, String> {
        if matches!(
            value,
            bbb_protocol::NbtSummaryValue::Compound(entries) if entries.is_empty()
        ) {
            return Ok(ProtocolItemStackSummary::empty());
        }
        crate::client::decode_command_item_stack_template_summary(
            value,
            self.frozen_registry_access(),
        )
    }

    /// Encodes Vanilla's registry-aware `ItemStack.OPTIONAL_CODEC` NBT shape.
    /// Transient components are filtered by the persistent codec.
    pub fn persistent_item_stack_to_nbt(
        &self,
        stack: &ProtocolItemStackSummary,
    ) -> Result<bbb_protocol::NbtSummaryValue, String> {
        if stack.item_id.is_none() {
            return Ok(bbb_protocol::NbtSummaryValue::Compound(Vec::new()));
        }
        if !(1..=99).contains(&stack.count) {
            return Err("item stack count is out of range".to_string());
        }
        bbb_protocol::packets::data_components::validate_persistent_data_component_patch_summary_with_registry(
            &stack.component_patch,
            self.content_registry(),
        )
        .map_err(|error| error.to_string())?;
        bbb_registry_codec::item_stack_summary_to_persistent_nbt_with_resolver(
            stack,
            &self.frozen_registry_access(),
        )
        .map_err(|error| error.to_string())
    }

    pub fn apply_set_player_inventory(&mut self, packet: ProtocolSetPlayerInventory) -> bool {
        self.tutorial_on_get_item(&packet.item);
        self.inventory
            .apply_set_player_inventory(&mut self.counters, packet)
    }

    pub fn apply_set_cursor_item(&mut self, packet: ProtocolSetCursorItem) {
        self.apply_creative_aware_set_cursor_item(packet, false)
    }

    pub fn apply_open_screen(&mut self, packet: ProtocolOpenScreen) {
        self.counters.container_open_updates_received += 1;
        if !self.preflight_packet_components(&packet, "open screen") {
            return;
        }
        self.inventory
            .apply_open_screen(&mut self.counters, &mut self.client_ui, packet)
    }

    pub fn apply_container_set_content(&mut self, packet: ProtocolContainerSetContent) -> bool {
        self.inventory
            .apply_container_set_content(&mut self.counters, packet)
    }

    pub fn apply_merchant_offers(&mut self, packet: ProtocolMerchantOffers) -> bool {
        self.inventory
            .apply_merchant_offers(&mut self.counters, packet)
    }

    pub(crate) fn apply_mount_screen_open_container(&mut self, mount: MountScreenState) {
        self.inventory.apply_mount_screen_open_container(
            &mut self.counters,
            &mut self.client_ui,
            mount,
        )
    }

    pub fn apply_container_set_data(&mut self, packet: ProtocolContainerSetData) -> bool {
        self.inventory
            .apply_container_set_data(&mut self.counters, packet)
    }

    pub fn apply_container_close(&mut self, packet: ProtocolContainerClose) -> bool {
        let applied = self
            .inventory
            .apply_container_close(&mut self.counters, packet);
        self.push_screen_request(crate::ScreenRequest::CloseScreen);
        applied
    }

    pub fn close_local_container(&mut self, container_id: i32) -> bool {
        let closed = self
            .inventory
            .close_local_container(&mut self.counters, container_id);
        if closed {
            self.push_screen_request(crate::ScreenRequest::CloseContainer);
        }
        closed
    }

    pub(crate) fn close_container_for_respawn(&mut self) -> Option<i32> {
        let closed = self.inventory.close_for_respawn(&mut self.counters);
        self.push_screen_request(crate::ScreenRequest::CloseContainer);
        closed
    }

    pub fn open_local_inventory(&mut self) -> bool {
        match self.open_local_inventory_with_operator_tab(false) {
            Ok(opened) => opened,
            Err(error) => {
                self.record_apply_error("creative_inventory", error.to_string());
                let opened = self.inventory.open_local_inventory(&mut self.client_ui);
                self.push_screen_request(crate::ScreenRequest::Container);
                opened
            }
        }
    }

    pub fn local_inventory_is_open(&self) -> bool {
        self.inventory.local_inventory_is_open()
    }

    pub fn open_container_id(&self) -> Option<i32> {
        self.inventory.open_container_id()
    }

    pub(crate) fn recipe_update_listener_context(&self) -> Option<RecipeUpdateListenerContext> {
        self.inventory.recipe_update_listener_context()
    }

    pub fn open_container_data_value(&self, id: i16) -> Option<i16> {
        self.inventory.open_container_data_value(id)
    }

    /// Plans `CrafterScreen.updateSlotState` before the ordinary container
    /// click. Empty grid slots toggle on pickup; a disabled slot also enables
    /// before a hotbar/offhand swap when that player slot is nonempty.
    ///
    /// The returned change supplies the payload for
    /// `ServerboundContainerSlotStateChangedPacket`. A later matching
    /// `ContainerSetData` remains authoritative and overwrites this local
    /// projection.
    pub fn plan_local_crafter_slot_state_change(
        &self,
        slot_id: i16,
        input: ProtocolContainerInput,
        button_num: i8,
    ) -> Option<LocalCrafterSlotStateChange> {
        if self.local_player_is_spectator() || !(0..CRAFTER_GRID_SLOT_COUNT).contains(&slot_id) {
            return None;
        }
        let (container_id, disabled) = {
            let container = self.inventory.open_container.as_ref()?;
            if container.menu_type_id != Some(VANILLA_MENU_TYPE_CRAFTER_3X3_ID) {
                return None;
            }
            let slot = container.slots.iter().find(|slot| slot.slot == slot_id)?;
            if !item_stack_is_empty(&slot.item) {
                return None;
            }
            (
                container.container_id,
                container
                    .data_values
                    .iter()
                    .find_map(|value| (value.id == slot_id).then_some(value.value))
                    .unwrap_or_default()
                    == 1,
            )
        };

        let enabled = match input {
            ProtocolContainerInput::Pickup if disabled => true,
            ProtocolContainerInput::Pickup if item_stack_is_empty(&self.inventory.cursor_item) => {
                false
            }
            ProtocolContainerInput::Swap if disabled => {
                let player_slot = i32::from(button_num);
                if !((0..PLAYER_HOTBAR_SIZE as i32).contains(&player_slot)
                    || player_slot == VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT)
                    || self
                        .inventory
                        .local_player_inventory_item(player_slot)
                        .is_none_or(item_stack_is_empty)
                {
                    return None;
                }
                true
            }
            _ => return None,
        };

        Some(LocalCrafterSlotStateChange {
            container_id,
            slot_id,
            enabled,
        })
    }

    /// Commits a still-current Crafter slot-state plan and queues its
    /// client-local button sound. Menu replacement, correction, or a newly
    /// occupied slot invalidates the plan without mutating state.
    pub fn apply_local_crafter_slot_state_change(
        &mut self,
        change: LocalCrafterSlotStateChange,
    ) -> bool {
        if self.local_player_is_spectator()
            || !(0..CRAFTER_GRID_SLOT_COUNT).contains(&change.slot_id)
        {
            return false;
        }
        let container = self.inventory.open_container.as_mut().filter(|container| {
            container.container_id == change.container_id
                && container.menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTER_3X3_ID)
        });
        let Some(container) = container else {
            return false;
        };
        let Some(slot) = container
            .slots
            .iter()
            .find(|slot| slot.slot == change.slot_id)
        else {
            return false;
        };
        if !item_stack_is_empty(&slot.item) {
            return false;
        }
        let disabled = container
            .data_values
            .iter()
            .find_map(|value| (value.id == change.slot_id).then_some(value.value))
            .unwrap_or_default()
            == 1;
        if change.enabled != disabled {
            return false;
        }
        set_container_data_value(
            &mut container.data_values,
            change.slot_id,
            if change.enabled { 0 } else { 1 },
        );
        self.queue_local_interaction_sound_action(
            LocalInteractionSoundAction::CrafterSlotStateChanged {
                enabled: change.enabled,
            },
        );
        true
    }

    /// Vanilla `LoomMenu.getSelectablePatterns`: banner and dye inputs gate the
    /// list; an empty pattern slot resolves `BannerPatternTags.NO_ITEM_REQUIRED`,
    /// while a pattern item resolves its effective `PROVIDES_BANNER_PATTERNS`
    /// holder set in current session order. Missing or invalid holder state
    /// fails closed to an empty list.
    pub fn loom_selectable_banner_pattern_ids(&self) -> Option<Vec<i32>> {
        let container = self.inventory.open_container.as_ref()?;
        if container.menu_type_id != Some(VANILLA_MENU_TYPE_LOOM_ID) {
            return None;
        }
        let slot_item = |slot_num| {
            container
                .slots
                .iter()
                .find(|slot| slot.slot == slot_num)
                .map(|slot| &slot.item)
        };
        if slot_item(LOOM_BANNER_SLOT).is_none_or(item_stack_is_empty)
            || slot_item(LOOM_DYE_SLOT).is_none_or(item_stack_is_empty)
        {
            return None;
        }

        let access = self.registries.frozen_access();
        let pattern_stack = slot_item(LOOM_PATTERN_SLOT);
        let resolved = if pattern_stack.is_none_or(item_stack_is_empty) {
            access.holder_set_by_tag(BANNER_PATTERN_REGISTRY, LOOM_NO_ITEM_REQUIRED_PATTERN_TAG)
        } else {
            let pattern_stack = pattern_stack?;
            if !self.item_stack_has_effective_component(
                pattern_stack,
                VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
            ) {
                return Some(Vec::new());
            }
            let explicitly_patched = pattern_stack
                .component_patch
                .added_type_ids
                .contains(&VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID);
            let patch = self.items.component_source(
                pattern_stack.item_id,
                &pattern_stack.component_patch,
                VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
            );
            match patch.provides_banner_pattern_tag.as_deref() {
                Some(_)
                    if explicitly_patched
                        && !patch.provides_banner_pattern_registry_ids.is_empty() =>
                {
                    return Some(Vec::new());
                }
                Some(tag) => access.holder_set_by_tag(BANNER_PATTERN_REGISTRY, tag),
                None => access.holder_set_by_holder_ids(
                    BANNER_PATTERN_REGISTRY,
                    &patch.provides_banner_pattern_registry_ids,
                ),
            }
        };

        Some(
            resolved
                .map(|set| set.entries.iter().map(|entry| entry.holder_id).collect())
                .unwrap_or_default(),
        )
    }

    pub fn apply_local_beacon_confirm_effects(
        &mut self,
        primary_effect: i32,
        secondary_effect: Option<i32>,
    ) -> bool {
        self.inventory
            .apply_local_beacon_confirm_effects(primary_effect, secondary_effect)
    }

    pub fn open_mount_inventory_kind(&self) -> Option<MountInventoryKind> {
        self.inventory
            .open_mount_inventory_kind(inventory_ctx!(self))
    }

    pub fn open_mount_armor_slot_kind(&self) -> Option<MountArmorSlotKind> {
        self.inventory
            .open_mount_armor_slot_kind(inventory_ctx!(self))
    }

    pub fn open_mount_equipment_slot_visibility(&self) -> Option<MountEquipmentSlotVisibility> {
        self.inventory
            .open_mount_equipment_slot_visibility(inventory_ctx!(self))
    }

    pub fn apply_local_select_bundle_item(
        &mut self,
        slot_id: i32,
        selected_item_index: i32,
    ) -> bool {
        self.inventory
            .apply_local_select_bundle_item(slot_id, selected_item_index)
    }

    pub fn inventory(&self) -> &InventoryState {
        &self.inventory
    }

    pub fn local_item_use_prefers_offhand(&self) -> bool {
        self.inventory
            .local_item_use_prefers_offhand(inventory_ctx!(self))
    }

    pub(crate) fn local_item_in_hand_is_non_empty(&self, hand: InteractionHand) -> bool {
        self.inventory
            .local_item_in_hand_is_non_empty(inventory_ctx!(self), hand)
    }

    /// The non-empty stack currently held in the requested local-player hand.
    ///
    /// Main hand follows the selected hotbar slot, while off hand follows the
    /// player-inventory offhand slot, matching vanilla `LocalPlayer.getItemInHand`.
    pub fn local_item_in_hand(&self, hand: InteractionHand) -> Option<&ProtocolItemStackSummary> {
        self.inventory
            .local_item_in_hand(inventory_ctx!(self), hand)
    }

    pub(crate) fn local_player_has_item_in_tag(&self, tag: &str) -> Option<bool> {
        self.inventory
            .local_player_has_item_in_tag(inventory_ctx!(self), tag)
    }

    /// Vanilla `ClientLevel.getMarkerParticleTarget`: only a creative player
    /// holding barrier/light in the main hand yields a marker target block.
    pub fn local_marker_particle_target_block(&self) -> Option<&'static str> {
        if !self.local_player_is_creative() {
            return None;
        }
        match self
            .local_item_in_hand(InteractionHand::MainHand)?
            .item_id?
        {
            VANILLA_ITEM_BARRIER_ID => Some("minecraft:barrier"),
            VANILLA_ITEM_LIGHT_ID => Some("minecraft:light"),
            _ => None,
        }
    }

    /// Vanilla `Consumable.canConsume`: FOOD only gates an effective
    /// CONSUMABLE, creative/invulnerable players and `can_always_eat` bypass
    /// the full-food check, and an absent health packet uses FoodData's
    /// initial full level.
    pub fn local_item_in_hand_can_start_consuming(&self, hand: InteractionHand) -> bool {
        let Some(stack) = self.local_item_in_hand(hand) else {
            return true;
        };
        if !self
            .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID)
            || !self.item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID)
        {
            return true;
        }

        let effective = self.effective_item_stack(stack);
        self.local_player
            .abilities
            .is_some_and(|abilities| abilities.invulnerable)
            || effective.component_patch.food_can_always_eat
            || self
                .local_player
                .health
                .is_some_and(|health| health.food < 20)
    }

    pub fn local_selected_main_hand_has_piercing_weapon(&self) -> bool {
        let selected_slot = self.local_player.selected_hotbar_slot;
        if selected_slot > 8 {
            return false;
        }
        self.inventory
            .local_player_inventory_item(i32::from(selected_slot))
            .is_some_and(|item| {
                !item_stack_is_empty(item)
                    && item.item_id.unwrap() >= 0
                    && self.items.item_has_effective_component(
                        item.item_id,
                        &item.component_patch,
                        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
                    )
            })
    }

    pub fn local_selected_main_hand_attack_range(&self) -> Option<ItemAttackRange> {
        self.inventory
            .local_selected_main_hand_attack_range(inventory_ctx!(self))
    }

    /// Vanilla `LivingEntity.getActiveItem()` followed by
    /// `DataComponents.ATTACK_RANGE`: spectators expose an empty active item;
    /// active use selects the use hand; otherwise the selected main hand wins.
    pub fn local_active_item_attack_range(&self) -> Option<ItemAttackRange> {
        if self.local_player_is_spectator() {
            return None;
        }
        let hand = if self.local_player.interaction.using_item {
            self.local_player
                .interaction
                .using_item_hand
                .unwrap_or(InteractionHand::MainHand)
        } else {
            InteractionHand::MainHand
        };
        self.local_item_in_hand(hand)
            .and_then(|item| item_stack_attack_range(item, &self.items))
    }

    pub fn local_selected_main_hand_minimum_attack_charge(&self) -> f32 {
        self.inventory
            .local_selected_main_hand_minimum_attack_charge(inventory_ctx!(self))
    }

    pub fn local_selected_main_hand_is_enabled(&self) -> bool {
        self.local_item_in_hand_is_enabled(InteractionHand::MainHand)
    }

    pub fn local_item_in_hand_is_enabled(&self, hand: InteractionHand) -> bool {
        self.inventory.local_item_in_hand_is_enabled(
            inventory_ctx!(self),
            hand,
            self.is_feature_enabled("minecraft:vanilla"),
        )
    }

    pub(crate) fn entity_held_item_swing_duration(&self, id: i32, off_hand: bool) -> i32 {
        self.held_item(id, off_hand)
            .map(|item| {
                self.entity_swing_duration_with_effects(
                    id,
                    self.item_stack_swing_animation(item).duration,
                )
            })
            .unwrap_or(ATTACK_SWING_DURATION)
    }

    pub fn item_stack_swing_animation(
        &self,
        stack: &ProtocolItemStackSummary,
    ) -> ProtocolSwingAnimationSummary {
        item_stack_swing_animation(stack, &self.items)
    }

    pub fn trigger_local_player_swing(&mut self, hand: InteractionHand) -> bool {
        let Some(id) = self.local_player_id else {
            return false;
        };
        let off_hand = hand == InteractionHand::OffHand;
        let duration = self.entity_held_item_swing_duration(id, off_hand);
        self.entities
            .trigger_client_animation_swing(id, off_hand, duration)
            .is_some()
    }

    pub(crate) fn refresh_entity_active_swing_duration(&mut self, id: i32) {
        let Some(off_hand) = self.entities.active_swing_off_hand(id) else {
            return;
        };
        let duration = self.entity_held_item_swing_duration(id, off_hand);
        let _ = self
            .entities
            .refresh_client_animation_swing_duration(id, duration);
    }

    pub(crate) fn local_using_item_use_effects(&self) -> Option<ItemUseEffects> {
        self.inventory
            .local_using_item_use_effects(inventory_ctx!(self))
    }

    pub fn local_using_item_item_id(&self) -> Option<i32> {
        self.inventory
            .local_using_item_item_id(inventory_ctx!(self))
    }

    pub fn drop_local_selected_hotbar_item(&mut self, all: bool) -> bool {
        self.inventory.drop_local_selected_hotbar_item(
            inventory_ctx!(self),
            &mut self.counters,
            all,
        )
    }

    pub(crate) fn shrink_local_item_in_hand(&mut self, hand: InteractionHand, amount: i32) -> bool {
        self.inventory.shrink_local_item_in_hand(
            inventory_ctx!(self),
            &mut self.counters,
            hand,
            amount,
        )
    }

    pub(crate) fn consume_local_item_in_hand(
        &mut self,
        hand: InteractionHand,
        amount: i32,
    ) -> bool {
        self.inventory.consume_local_item_in_hand(
            inventory_ctx!(self),
            &mut self.counters,
            hand,
            amount,
        )
    }

    pub(crate) fn consume_local_mob_player_item(
        &mut self,
        hand: InteractionHand,
        remainder: Option<ProtocolItemStackSummary>,
    ) -> bool {
        self.inventory.consume_local_mob_player_item(
            inventory_ctx!(self),
            &mut self.counters,
            hand,
            remainder,
        )
    }

    /// Vanilla `Gui.extractCameraOverlays` (Gui.java:264-270): every equipment
    /// slot whose item carries an `Equippable` for that same slot with a
    /// `cameraOverlay`, in `EquipmentSlot.values()` order. The identifiers are
    /// resolved to their `textures/<path>.png` locations the way vanilla's
    /// `withPath` call does.
    pub fn local_player_camera_overlay_textures(&self) -> Vec<String> {
        self.inventory
            .local_player_equipment(self.local_player.selected_hotbar_slot)
            .filter_map(|(entry, item)| {
                let equippable = if item
                    .component_patch
                    .removed_type_ids
                    .contains(&VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
                {
                    None
                } else {
                    item.component_patch.equippable.as_ref().or_else(|| {
                        self.item_stack_has_effective_component(
                            item,
                            VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                        )
                        .then(|| {
                            self.items
                                .component_source(
                                    item.item_id,
                                    &item.component_patch,
                                    VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                                )
                                .equippable
                                .as_ref()
                        })
                        .flatten()
                    })
                }?;
                if equippable.slot_id != entry.id {
                    return None;
                }
                let overlay = equippable.camera_overlay.as_ref()?;
                let (namespace, path) = overlay
                    .split_once(':')
                    .unwrap_or(("minecraft", overlay.as_str()));
                Some(format!("{namespace}:textures/{path}.png"))
            })
            .collect()
    }

    pub(crate) fn swap_local_item_with_equipment_slot(
        &mut self,
        hand: InteractionHand,
        equipment_slot: EquipmentSlot,
    ) -> Option<bool> {
        let creative_mode = self.local_player_is_creative();
        self.inventory.swap_local_item_with_equipment_slot(
            inventory_ctx!(self),
            &mut self.counters,
            hand,
            equipment_slot,
            creative_mode,
        )
    }

    pub(super) fn replace_local_item_with_filled_result(
        &mut self,
        creative: CreativeFilledResult,
        hand: InteractionHand,
        expected_item_id: i32,
        filled: ProtocolItemStackSummary,
    ) -> Option<bool> {
        self.inventory.replace_local_item_with_filled_result(
            inventory_ctx!(self),
            &mut self.counters,
            creative,
            hand,
            expected_item_id,
            filled,
        )
    }

    pub(crate) fn add_local_item_from_block_interaction(
        &mut self,
        item: ProtocolItemStackSummary,
    ) -> bool {
        let added = self
            .inventory
            .add_local_item_to_player_inventory(inventory_ctx!(self), item);
        self.inventory
            .update_inventory_slot_count(&mut self.counters);
        added
    }

    /// Vanilla `LivingEntity.canGlideUsing`: any equipment slot may carry the
    /// usable glider, but the effective EQUIPPABLE slot must match and the
    /// next durability point must not break the stack.
    pub fn local_player_has_usable_glider(&self) -> bool {
        VANILLA_EQUIPMENT_SLOTS.iter().any(|slot| {
            let stack = match slot.slot {
                EquipmentSlot::MainHand => self.local_item_in_hand(InteractionHand::MainHand),
                EquipmentSlot::OffHand => self.local_item_in_hand(InteractionHand::OffHand),
                _ => slot
                    .player_inventory_slot
                    .and_then(|slot| self.inventory.local_player_inventory_item(slot))
                    .filter(|stack| item_stack_is_non_empty(stack)),
            };
            let Some(stack) = stack else {
                return false;
            };
            self.item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID)
                && self.effective_item_equipment_slot(stack) == Some(slot.slot)
                && !self.item_stack_next_damage_will_break(stack)
        })
    }

    pub(super) fn effective_item_equipment_slot(
        &self,
        stack: &ProtocolItemStackSummary,
    ) -> Option<EquipmentSlot> {
        self.item_stack_equipment(stack).map(|(slot, _)| slot)
    }

    pub(super) fn item_stack_next_damage_will_break(
        &self,
        stack: &ProtocolItemStackSummary,
    ) -> bool {
        ItemProfiles::damage_state(Some(&self.items), stack.item_id, &stack.component_patch)
            .is_some_and(|(damage, max_damage)| damage >= max_damage - 1)
    }

    pub fn set_local_merchant_selected_offer(&mut self, index: i32) -> bool {
        self.inventory
            .set_local_merchant_selected_offer(inventory_ctx!(self), index)
    }

    pub fn scroll_local_merchant_offers(&mut self, delta: i32) -> bool {
        self.inventory.scroll_local_merchant_offers(delta)
    }

    pub fn furnace_fuel_item_ids(&self) -> &BTreeSet<i32> {
        &self.items.furnace_fuel_item_ids
    }

    pub fn furnace_burn_ticks(&self, item_id: i32) -> Option<u32> {
        use bbb_facts::fuel_burn_times::{
            BurnTime, FuelRow, VANILLA_BURN_TIME_ROWS, VANILLA_FUEL_BASE_UNIT,
        };
        if !self.furnace_fuel_item_ids().contains(&item_id) {
            return None;
        }
        let resource_id = vanilla_item_type(item_id)
            .map(|entry| entry.resource_id)
            .or_else(|| {
                self.content_registry()
                    .item_by_id(item_id)
                    .map(|entry| entry.resource_id.as_str())
            });
        let mut matched: Option<BurnTime> = None;
        for row in VANILLA_BURN_TIME_ROWS {
            match row {
                FuelRow::Item(id, burn_time) => {
                    if resource_id == Some(*id) {
                        matched = Some(*burn_time);
                    }
                }
                FuelRow::Tag(tag, burn_time) => {
                    if self
                        .registry_holder_ids_for_tag("minecraft:item", tag)
                        .is_some_and(|ids| ids.contains(&item_id))
                    {
                        matched = Some(*burn_time);
                    }
                }
                FuelRow::RemoveTag(tag) => {
                    if self
                        .registry_holder_ids_for_tag("minecraft:item", tag)
                        .is_some_and(|ids| ids.contains(&item_id))
                    {
                        matched = None;
                    }
                }
            }
        }
        matched
            .map(|burn_time| burn_time.ticks(VANILLA_FUEL_BASE_UNIT))
            .filter(|&ticks| ticks > 0)
            .map(|ticks| ticks as u32)
    }

    fn entity_swing_duration_with_effects(&self, entity_id: i32, duration: i32) -> i32 {
        if let Some(amplifier) = self.entity_dig_speed_amplifier(entity_id) {
            return duration.saturating_sub(amplifier.saturating_add(1));
        }
        if let Some(amplifier) =
            self.entity_mob_effect_amplifier(entity_id, VANILLA_MOB_EFFECT_MINING_FATIGUE_ID)
        {
            return duration.saturating_add(amplifier.saturating_add(1).saturating_mul(2));
        }
        duration
    }

    fn entity_mob_effect_amplifier(&self, entity_id: i32, effect_id: i32) -> Option<i32> {
        self.entity_effect(entity_id, effect_id)
            .map(|effect| effect.amplifier)
    }

    fn entity_dig_speed_amplifier(&self, entity_id: i32) -> Option<i32> {
        [
            VANILLA_MOB_EFFECT_HASTE_ID,
            VANILLA_MOB_EFFECT_CONDUIT_POWER_ID,
        ]
        .into_iter()
        .filter_map(|effect_id| self.entity_mob_effect_amplifier(entity_id, effect_id))
        .max()
    }

    pub fn build_container_click_slot(
        &self,
        request: ContainerClickSlotRequest,
    ) -> Result<ProtocolContainerClick, ContainerClickBuildError> {
        self.inventory
            .build_container_click_slot(inventory_ctx!(self), request)
    }

    pub fn active_container_slot_can_quick_craft(&self, slot_num: i16) -> bool {
        self.inventory
            .active_container_slot_can_quick_craft(inventory_ctx!(self), slot_num)
    }

    pub fn active_container_slot_max_stack_size(
        &self,
        slot_num: i16,
        stack: &ProtocolItemStackSummary,
    ) -> Option<i32> {
        self.inventory
            .active_container_slot_max_stack_size(inventory_ctx!(self), slot_num, stack)
    }

    pub fn active_container_slot_may_pickup(&self, slot_num: i16) -> bool {
        self.inventory
            .active_container_slot_may_pickup(inventory_ctx!(self), slot_num)
    }

    pub fn active_container_slot_can_take_for_pick_all(&self, slot_num: i16) -> bool {
        self.inventory
            .active_container_slot_can_take_for_pick_all(slot_num)
    }

    pub fn active_container_slots_share_container(&self, left: i16, right: i16) -> bool {
        self.inventory
            .active_container_slots_share_container(left, right)
    }

    pub fn active_container_slot_ids(&self) -> Vec<i16> {
        let container = self
            .inventory
            .open_container
            .as_ref()
            .unwrap_or(&self.inventory.inventory_menu);
        container.slots.iter().map(|slot| slot.slot).collect()
    }

    pub fn apply_local_container_click_slot(
        &mut self,
        request: ContainerClickSlotRequest,
    ) -> Result<ProtocolContainerClick, ContainerClickBuildError> {
        let (packet, sound_action) =
            self.inventory
                .predict_container_click_slot(inventory_ctx!(self), request, true)?;
        if let Some(action) = sound_action {
            self.queue_local_interaction_sound_action(action);
        }
        self.inventory
            .update_inventory_slot_count(&mut self.counters);
        Ok(packet.unwrap())
    }

    pub(crate) fn preview_creative_inventory_click(
        &self,
        inventory: &mut InventoryState,
        request: ContainerClickSlotRequest,
    ) -> Result<(), ContainerClickBuildError> {
        inventory
            .predict_container_click_slot(inventory_ctx!(self), request, false)
            .map(|_| ())
    }

    pub(crate) fn local_player_has_freeze_immune_wearable(&self) -> bool {
        self.inventory
            .local_player_has_freeze_immune_wearable(inventory_ctx!(self))
    }

    pub(crate) fn local_player_can_walk_on_powder_snow(&self) -> bool {
        self.inventory
            .local_player_can_walk_on_powder_snow(inventory_ctx!(self))
    }
}
