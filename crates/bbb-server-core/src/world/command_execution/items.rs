use super::*;
use crate::world::LevelId;
use bbb_server_types::item_stack::RuntimeItemStack;

impl ServerWorld {
    /// `ClearInventoryCommands.clearInventory` (`ClearInventoryCommands.java:65-68`).
    pub(super) fn clear_items_from_player(
        &mut self,
        connection_id: ConnectionId,
        max_count: i32,
        matches: &mut impl FnMut(&RuntimeItemStack) -> bbb_server_types::Result<bool>,
        events: &mut Vec<WorldEvent>,
    ) -> bbb_server_types::Result<i32> {
        let resolver = self.item_stack_resolver();
        let Some(player) = self.levels.player_mut(connection_id) else {
            return Ok(0);
        };
        let count = player.inventory.clear_matching_items(
            &resolver,
            Some(&mut self.maps),
            max_count,
            matches,
        )?;
        if max_count == 0 {
            return Ok(count);
        }
        let published = player.inventory.take_crafting_sync();
        Self::send_menu_sync(connection_id, published, events);
        self.broadcast_player_menu(
            connection_id,
            crate::inventory::MenuBroadcastMode::Changes,
            events,
        );
        let inventory = &mut self.levels[connection_id].inventory;
        inventory.crafting_slots_changed(&resolver, Some(&mut self.maps))?;
        let published = inventory.take_crafting_sync();
        Self::send_menu_sync(connection_id, published, events);
        Ok(count)
    }

    /// `ItemCommands.setEntityItem` (`ItemCommands.java:384-389`).
    pub(super) fn set_entity_item_from_command(
        &mut self,
        player: crate::commands::CommandPlayerTarget,
        slot: i32,
        stack: RuntimeItemStack,
        events: &mut Vec<WorldEvent>,
    ) -> bbb_server_types::Result<bool> {
        let resolver = self.item_stack_resolver();
        let Some(connection_id) = self.command_player_connection(&player) else {
            return Ok(false);
        };
        let Some(state) = self.levels.player(connection_id) else {
            return Ok(false);
        };
        let Some(target) = crate::commands::player_slot(slot, state.inventory.selected_slot())
        else {
            return Ok(false);
        };
        if !crate::commands::player_slot_accepts(slot, &stack).map_err(|error| {
            bbb_server_types::ServerWorldError::InvalidConfiguration(format!(
                "command slot access: {error:?}"
            ))
        })? {
            return Ok(false);
        }
        let inventory = &mut self.levels[connection_id].inventory;
        match target {
            crate::commands::PlayerSlot::Cursor => inventory.set_carried(stack),
            crate::commands::PlayerSlot::Crafting(index) => {
                if !inventory.set_crafting_slot(&resolver, Some(&mut self.maps), index, stack)? {
                    return Ok(false);
                }
                // Player.java:1729-1732; the container and caller each notify the inventory menu.
                inventory.crafting_slots_changed(&resolver, Some(&mut self.maps))?;
            }
            crate::commands::PlayerSlot::EnderChest(index) => {
                if !inventory.set_ender_chest_runtime_slot(index, stack)? {
                    return Ok(false);
                }
            }
            crate::commands::PlayerSlot::Inventory(index) => {
                if !inventory.set_inventory_slot(index, stack) {
                    return Ok(false);
                }
            }
        }
        let published = self.levels[connection_id].inventory.take_crafting_sync();
        Self::send_menu_sync(connection_id, published, events);
        self.broadcast_player_menu(
            connection_id,
            crate::inventory::MenuBroadcastMode::Changes,
            events,
        );
        Ok(true)
    }

    /// `ItemStack.enchant` (`ItemStack.java:995-997`) through
    /// `ItemEnchantments.Mutable.upgrade` (`ItemEnchantments.java:139-142`).
    pub(super) fn enchant_item_from_command(
        &mut self,
        player: crate::commands::CommandPlayerTarget,
        slot: i32,
        holder_id: i32,
        level: i32,
        events: &mut Vec<WorldEvent>,
    ) -> bbb_server_types::Result<()> {
        let Some(connection_id) = self.command_player_connection(&player) else {
            return Ok(());
        };
        let Some(state) = self.levels.player_mut(connection_id) else {
            return Ok(());
        };
        let Some(crate::commands::PlayerSlot::Inventory(index)) =
            crate::commands::player_slot(slot, state.inventory.selected_slot())
        else {
            return Ok(());
        };
        let Some(stack) = state.inventory.slot_mut(index) else {
            return Ok(());
        };
        if stack.is_empty() {
            return Ok(());
        }
        if let Some(edited) = enchanted_stack(stack.summary(), holder_id, level) {
            *stack = edited;
        }
        self.broadcast_player_menu(
            connection_id,
            crate::inventory::MenuBroadcastMode::Changes,
            events,
        );
        Ok(())
    }

    /// `/enchant` on a stored living entity's main hand.
    pub(super) fn enchant_stored_item_from_command(
        &mut self,
        level_key: &Java26LevelKey,
        uuid: uuid::Uuid,
        slot: i32,
        holder_id: i32,
        level: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), NbtSummaryValue> {
        let slot = crate::commands::ItemSlot::Stored {
            uuid,
            level: level_key.clone(),
            name: NbtSummaryValue::string(String::new()),
            slot,
        };
        let Some(stack) = self.read_command_item_slot(level_key, &slot, None)? else {
            return Ok(());
        };
        if let Some(edited) = enchanted_stack(stack.summary(), holder_id, level) {
            self.write_command_item_slot(level_key, &slot, edited, events)?;
        }
        Ok(())
    }

    /// `ItemCommands.setBlockItem` (`ItemCommands.java:361-362`).
    pub(super) fn set_container_item_from_command(
        &mut self,
        level: LevelId,
        pos: BlockPos,
        slot: i32,
        stack: RuntimeItemStack,
    ) -> bbb_server_types::Result<()> {
        self.set_command_block_item(&level, pos, slot as usize, stack)
    }

    pub(super) fn give_native_items_to_player(
        &mut self,
        connection_id: ConnectionId,
        prototype: &RuntimeItemStack,
        count: i32,
        events: &mut Vec<WorldEvent>,
    ) -> bbb_server_types::Result<()> {
        let resolver = self.item_stack_resolver();
        let Some(player) = self.levels.player(connection_id) else {
            return Ok(());
        };
        let dimension = player.dimension;
        let sound_position = player.position();
        let eye = player.eye_position()?;
        let drop_position = Vec3d {
            x: eye.x,
            y: eye.y - f64::from(0.3_f32),
            z: eye.z,
        };
        if count <= 0 || prototype.is_empty() {
            return Ok(());
        }
        let mut remaining = count;
        let max_stack = prototype.max_stack_size()?.max(1);
        while remaining > 0 {
            let size = remaining.min(max_stack);
            remaining -= size;
            let mut copy = prototype.with_count(size);
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("the connection was just resolved");
            let creative = player.abilities.instabuild;
            let added = player
                .inventory
                .add_anywhere(&resolver, &mut copy, creative)?;
            if added && copy.is_empty() {
                self.spawn_give_animation(connection_id, &dimension, drop_position, prototype)?;
                self.play_give_pickup_sound(&dimension, sound_position, events);
                self.broadcast_player_menu(
                    connection_id,
                    crate::inventory::MenuBroadcastMode::Changes,
                    events,
                );
            } else if !copy.is_empty() {
                self.spawn_give_leftover(connection_id, &dimension, drop_position, copy)?;
            }
        }
        Ok(())
    }

    fn spawn_give_animation(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        position: Vec3d,
        prototype: &RuntimeItemStack,
    ) -> bbb_server_types::Result<()> {
        let item = prototype.clone();
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("give retains its player");
        let velocity =
            crate::world::advancement_loot::reward_drop_velocity(player.rotation(), &self.random);
        crate::entity::item::spawn_fake_command_item(
            &mut self.level_mut_by_id(dimension).entities,
            position,
            item,
            velocity,
        )
        .ok_or_else(|| {
            bbb_server_types::ServerWorldError::InvalidConfiguration(
                "give animation entity spawn refused its drawn UUID".into(),
            )
        })?;
        Ok(())
    }

    /// `GiveCommand.giveItem` (`GiveCommand.java:71-81`): `ITEM_PICKUP` at the
    /// player's feet, `PLAYERS`, volume `0.2`.
    fn play_give_pickup_sound(
        &mut self,
        dimension: &LevelId,
        position: Vec3d,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(registry_id) = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
            "minecraft:entity.item.pickup",
        ) else {
            return;
        };
        let pitch = {
            let random = &self.random;
            ((random.next_float() - random.next_float()) * 0.7 + 1.0) * 2.0
        };
        let seed = self.random.next_long();
        let sound = SoundEvent {
            sound: SoundEventHolder::Reference { registry_id },
            source: SoundSource::Players,
            position,
            volume: 0.2,
            pitch,
            seed,
        };
        self.broadcast_level_sound_by_key(dimension, &sound, None, events);
    }

    /// `GiveCommand.giveItem` leftover arm (`GiveCommand.java:84-88`) through
    /// `LivingEntity.drop` (`LivingEntity.java:3372-3378`): eye Y minus `0.3`.
    fn spawn_give_leftover(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        position: Vec3d,
        leftover: RuntimeItemStack,
    ) -> bbb_server_types::Result<()> {
        let item = leftover;
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("give retains its player");
        let velocity =
            crate::world::advancement_loot::reward_drop_velocity(player.rotation(), &self.random);
        let target = player.profile.uuid;
        if crate::entity::item::spawn_advancement_reward_item(
            &mut self.level_mut_by_id(dimension).entities,
            position,
            item,
            velocity,
            target,
        )
        .is_none()
        {
            return Err(bbb_server_types::ServerWorldError::InvalidConfiguration(
                "give item entity spawn refused its drawn UUID".into(),
            ));
        }
        Ok(())
    }
}

/// The stack with `holder_id` upgraded to `level`, or `None` when unchanged.
fn enchanted_stack(
    before: &ItemStackSummary,
    holder_id: i32,
    level: i32,
) -> Option<RuntimeItemStack> {
    let mut edited = before.clone();
    upgrade_enchantments(&mut edited, holder_id, level);
    if edited == *before {
        return None;
    }
    let type_id =
        if edited.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_ENCHANTED_BOOK_ID) {
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID
        } else {
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID
        };
    let patch = &mut edited.component_patch;
    if !patch.added_type_ids.contains(&type_id) {
        patch.added_type_ids.push(type_id);
    }
    patch.removed_type_ids.retain(|id| *id != type_id);
    Some(RuntimeItemStack::from_summary(edited))
}

/// `ItemEnchantments.Mutable.upgrade` (`ItemEnchantments.java:139-142`).
fn upgrade_enchantments(stack: &mut ItemStackSummary, holder_id: i32, level: i32) {
    if level <= 0 {
        return;
    }
    let level = level.min(255);
    let enchanted_book =
        bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:enchanted_book")
            .is_some_and(|entry| stack.item_id == Some(entry.id));
    let entries = if enchanted_book {
        &mut stack.component_patch.stored_enchantments
    } else {
        &mut stack.component_patch.enchantments
    };
    if let Some(existing) = entries
        .iter_mut()
        .find(|entry| entry.holder_id == holder_id)
    {
        existing.level = existing.level.max(level);
    } else {
        entries.push(bbb_protocol::packets::ItemEnchantmentSummary { holder_id, level });
    }
}
