use super::*;

impl InventoryState {
    pub(crate) fn apply_mount_screen_open_container(
        &mut self,
        counters: &mut WorldCounters,
        ui: &mut ClientUiState,
        mount: MountScreenState,
    ) {
        ui.replace_screen();
        ui.push_screen_request(crate::ScreenRequest::Container);
        self.local_inventory_open = false;
        self.local_quick_craft.reset();
        let existing = self
            .open_container
            .take()
            .filter(|container| container.container_id == mount.container_id)
            .unwrap_or_else(|| ContainerState {
                container_id: mount.container_id,
                ..ContainerState::default()
            });
        self.open_container = Some(ContainerState {
            container_id: mount.container_id,
            menu_type_id: None,
            title: existing.title,
            mount: Some(mount),
            state_id: existing.state_id,
            slots: existing.slots,
            data_values: existing.data_values,
            merchant_offers: None,
        });
        let slots = std::mem::take(&mut self.open_container.as_mut().unwrap().slots);
        self.replace_container_slots(mount.container_id, slots);
        self.advance_open_container_revision();
        self.update_merchant_offer_count(counters);
    }

    pub(crate) fn close_for_respawn(&mut self, counters: &mut WorldCounters) -> Option<i32> {
        let server_container_id = self
            .open_container
            .as_ref()
            .map(|container| container.container_id);
        self.local_inventory_open = false;
        self.open_container = None;
        self.cursor_item = ProtocolItemStackSummary::empty();
        if server_container_id.is_some() {
            self.advance_open_container_revision();
        }
        self.local_quick_craft.reset();
        counters.merchant_offers_tracked = 0;
        server_container_id
    }

    pub(crate) fn open_local_inventory(&mut self, ui: &mut ClientUiState) -> bool {
        if self.open_container.is_some() {
            return false;
        }
        ui.replace_screen();
        self.ensure_inventory_menu_slot_shape();
        let was_open = self.local_inventory_open;
        self.local_inventory_open = true;
        self.local_quick_craft.reset();
        !was_open
    }

    pub(crate) fn local_inventory_is_open(&self) -> bool {
        self.local_inventory_open
    }

    pub(crate) fn open_container_id(&self) -> Option<i32> {
        self.open_container
            .as_ref()
            .map(|container| container.container_id)
            .or_else(|| {
                self.local_inventory_open
                    .then_some(INVENTORY_MENU_CONTAINER_ID)
            })
    }

    pub(super) fn advance_open_container_revision(&mut self) {
        self.open_container_revision = self.open_container_revision.saturating_add(1);
    }

    pub(crate) fn recipe_update_listener_context(&self) -> Option<RecipeUpdateListenerContext> {
        if self.local_inventory_open {
            return Some(RecipeUpdateListenerContext {
                container_id: INVENTORY_MENU_CONTAINER_ID,
                layout: RecipeUpdateListenerLayout::Crafting {
                    grid_width: 2,
                    grid_height: 2,
                },
            });
        }
        let container = self.open_container.as_ref()?;
        let layout = match container.menu_type_id? {
            VANILLA_MENU_TYPE_CRAFTING_ID => RecipeUpdateListenerLayout::Crafting {
                grid_width: 3,
                grid_height: 3,
            },
            VANILLA_MENU_TYPE_FURNACE_ID
            | VANILLA_MENU_TYPE_BLAST_FURNACE_ID
            | VANILLA_MENU_TYPE_SMOKER_ID => RecipeUpdateListenerLayout::Furnace {
                fuel_slot_empty: container
                    .slots
                    .iter()
                    .find(|slot| slot.slot == 1)
                    .is_none_or(|slot| item_stack_is_empty(&slot.item)),
            },
            _ => return None,
        };
        Some(RecipeUpdateListenerContext {
            container_id: container.container_id,
            layout,
        })
    }

    pub(crate) fn open_container_data_value(&self, id: i16) -> Option<i16> {
        self.open_container
            .as_ref()?
            .data_values
            .iter()
            .find_map(|value| (value.id == id).then_some(value.value))
    }

    pub(crate) fn open_mount_inventory_kind(
        &self,
        ctx: InventoryCtx<'_>,
    ) -> Option<MountInventoryKind> {
        let mount = self.open_container.as_ref()?.mount?;
        let entity_type_id = ctx.entities.entity_type_id(mount.entity_id)?;
        if crate::entities::is_vanilla_abstract_horse_type(entity_type_id) {
            Some(MountInventoryKind::Horse)
        } else if crate::entities::is_vanilla_abstract_nautilus_type(entity_type_id) {
            Some(MountInventoryKind::Nautilus)
        } else {
            None
        }
    }

    pub(crate) fn open_mount_armor_slot_kind(
        &self,
        ctx: InventoryCtx<'_>,
    ) -> Option<MountArmorSlotKind> {
        self.open_mount_equipment_slot_visibility(ctx)?.body
    }

    pub(crate) fn open_mount_equipment_slot_visibility(
        &self,
        ctx: InventoryCtx<'_>,
    ) -> Option<MountEquipmentSlotVisibility> {
        let mount = self.open_container.as_ref()?.mount?;
        let entity = ctx.entities.get(mount.entity_id)?;
        let entity_type_id = entity.entity_type_id;
        if crate::entities::is_vanilla_abstract_nautilus_type(entity_type_id) {
            let active =
                mount_nautilus_can_use_equipment_slots(entity_type_id, &entity.data_values);
            Some(MountEquipmentSlotVisibility {
                saddle: active,
                body: active.then_some(MountArmorSlotKind::Nautilus),
            })
        } else if crate::entities::is_vanilla_abstract_horse_type(entity_type_id) {
            Some(MountEquipmentSlotVisibility {
                saddle: mount_horse_saddle_slot_is_active(entity_type_id, &entity.data_values),
                body: mount_horse_body_slot_kind(entity_type_id),
            })
        } else {
            None
        }
    }

    pub(crate) fn scroll_local_merchant_offers(&mut self, delta: i32) -> bool {
        let Some(offers) = self
            .open_container
            .as_mut()
            .and_then(|container| container.merchant_offers.as_mut())
        else {
            return false;
        };
        let max_scroll_offset = merchant_max_scroll_offset(offers.offers.len());
        if max_scroll_offset <= 0 {
            return false;
        }
        offers.local_scroll_offset =
            (offers.local_scroll_offset + delta).clamp(0, max_scroll_offset);
        true
    }

    pub(super) fn active_container_slot_can_quick_craft(
        &self,
        ctx: InventoryCtx<'_>,
        slot_num: i16,
    ) -> bool {
        let Some(container) = self.active_container() else {
            return false;
        };
        let mount_rules = local_mount_slot_rules(
            ctx,
            container,
            container
                .mount
                .and_then(|_| self.open_mount_equipment_slot_visibility(ctx)),
        );
        if container.container_id != INVENTORY_MENU_CONTAINER_ID
            && !menu_type_has_known_slot_rules(container.menu_type_id)
            && mount_rules.is_none()
        {
            return false;
        }
        let Some(slot) = self.container_slot(container, slot_num) else {
            return false;
        };
        let Some(slot_access) = local_container_slot_access(
            ctx,
            container.container_id,
            container.menu_type_id,
            mount_rules,
            &container.data_values,
            slot_num,
            slot.item,
            &self.cursor_item,
        ) else {
            return false;
        };
        quick_craft_slot_can_accept(slot.item, &self.cursor_item, slot_access, ctx.items)
    }

    pub(super) fn active_container_slot_max_stack_size(
        &self,
        ctx: InventoryCtx<'_>,
        slot_num: i16,
        stack: &ProtocolItemStackSummary,
    ) -> Option<i32> {
        let container = self.active_container()?;
        let mount_rules = local_mount_slot_rules(
            ctx,
            container,
            container
                .mount
                .and_then(|_| self.open_mount_equipment_slot_visibility(ctx)),
        );
        let slot = self.container_slot(container, slot_num)?;
        local_container_slot_access(
            ctx,
            container.container_id,
            container.menu_type_id,
            mount_rules,
            &container.data_values,
            slot_num,
            slot.item,
            stack,
        )
        .map(|access| access.max_stack_size)
    }

    pub(super) fn active_container_slot_may_pickup(
        &self,
        ctx: InventoryCtx<'_>,
        slot_num: i16,
    ) -> bool {
        let Some(container) = self.active_container() else {
            return false;
        };
        let mount_rules = local_mount_slot_rules(
            ctx,
            container,
            container
                .mount
                .and_then(|_| self.open_mount_equipment_slot_visibility(ctx)),
        );
        let Some(slot) = self.container_slot(container, slot_num) else {
            return false;
        };
        local_container_slot_access(
            ctx,
            container.container_id,
            container.menu_type_id,
            mount_rules,
            &container.data_values,
            slot_num,
            slot.item,
            &ProtocolItemStackSummary::empty(),
        )
        .is_some_and(|access| access.may_pickup)
    }

    pub(super) fn active_container_slot_can_take_for_pick_all(&self, slot_num: i16) -> bool {
        let Some(container) = self.active_container() else {
            return false;
        };
        menu_slot_can_take_item_for_pick_all(
            container.container_id,
            container.menu_type_id,
            slot_num,
        )
    }

    /// Returns whether two slots in the active menu share the same backing
    /// `Container`, matching Vanilla `AbstractContainerScreen.mouseReleased`
    /// (`AbstractContainerScreen.java:465`).
    pub(super) fn active_container_slots_share_container(&self, left: i16, right: i16) -> bool {
        let Some(container) = self.active_container() else {
            return false;
        };
        let group = |slot_num: i16| -> u8 {
            if container.container_id == INVENTORY_MENU_CONTAINER_ID {
                return match slot_num {
                    0 => 0,
                    1..=4 => 1,
                    _ => 2,
                };
            }
            let player_start = slot_state::player_section_start(container).unwrap_or(i16::MAX);
            if slot_num >= player_start {
                return 10;
            }
            if container.mount.is_some() {
                return match slot_num {
                    0 => 0,
                    1 => 1,
                    _ => 2,
                };
            }
            if is_interactive_menu_result_slot(container.menu_type_id, slot_num)
                || (container.menu_type_id == Some(VANILLA_MENU_TYPE_CRAFTER_3X3_ID)
                    && slot_num == CRAFTER_RESULT_SLOT)
            {
                0
            } else {
                1
            }
        };
        group(left) == group(right)
    }

    pub(super) fn active_container(&self) -> Option<&ContainerState> {
        self.open_container
            .as_ref()
            .or_else(|| self.local_inventory_open.then_some(&self.inventory_menu))
    }

    pub(super) fn ensure_inventory_menu_slot_shape(&mut self) {
        for slot in 0..=INVENTORY_MENU_OFFHAND_SLOT {
            if self.container_slot(&self.inventory_menu, slot).is_none() {
                self.set_menu_slot(
                    INVENTORY_MENU_CONTAINER_ID,
                    ContainerSlot::new(slot, ProtocolItemStackSummary::empty()),
                );
            }
        }
    }

    pub(crate) fn update_inventory_slot_count(&mut self, counters: &mut WorldCounters) {
        counters.inventory_slots_tracked = self.player_slots.len();
    }

    pub(super) fn update_merchant_offer_count(&mut self, counters: &mut WorldCounters) {
        counters.merchant_offers_tracked = self
            .open_container
            .as_ref()
            .and_then(|container| container.merchant_offers.as_ref())
            .map(|offers| offers.offers.len())
            .unwrap_or(0);
    }
}
