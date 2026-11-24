//! The container menus [`ServerWorld`] opens, syncs and closes: the menu
//! packet arms, the block-entity inventories behind them, and the open-count
//! signal and sounds a viewer's arrival and departure produce.

use crate::level_event::VanillaGameEvent;
use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{
    BlockPos, ContainerButtonClick, ContainerClick, ContainerClose, ContainerSetContent,
    ContainerSetSlot, ContainerSlotStateChanged, Direction, OpenScreen, SetCursorItem,
    SetPlayerInventory, SoundEvent, SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use std::collections::BTreeSet;

use crate::block_behavior::barrel as barrel_block;
use crate::block_behavior::chest as chest_block;
use crate::block_behavior::dispenser as dispenser_block;
use crate::block_behavior::lectern as lectern_block;
use crate::block_behavior::shulker_box as shulker_box_block;
use crate::block_entity::{lectern, shulker_box, BlockEntity};
use crate::block_events::BlockEventData;
use crate::inventory::{
    ContainerAnchor, DroppedMenuStack, MenuLayout, MenuSync, CHEST_MENU_ROWS,
    DOUBLE_CHEST_MENU_ROWS,
};
use crate::loot_runtime::LootEntity;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_CRAFTED_ID;
use bbb_server_types::directions::relative;
use bbb_server_types::interaction::is_within_block_interaction_range;
use bbb_server_types::item_stack::RuntimeItemStack;

use bbb_server_types::Result as CoreResult;

use super::simulation_level::block_entity_loot::ContainerLootOpener;

use super::{chest_menu_title, send, ServerWorld, WorldEvent, CONTAINER_DISTANCE_BUFFER};
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use bbb_server_types::stats::{custom_stat, StatKey};

mod ender_chest;
mod recipe_placement;
mod vehicle;

impl ServerWorld {
    pub(super) fn apply_container_slot_state_changed(
        &mut self,
        connection_id: ConnectionId,
        change: ContainerSlotStateChanged,
    ) {
        let player = self.levels.player_at(connection_id);
        if player.game_type == GameType::Spectator
            || player.inventory.container_id() != change.container_id
            || !player.inventory.is_crafter_menu()
        {
            return;
        }
        let [original] = player.inventory.container_instances() else {
            return;
        };
        let original = original.clone();
        let dimension = player.dimension;
        self.with_level(dimension, |level| {
            level.set_crafter_slot_state(&original, change.slot_id, change.new_state);
        });
    }

    pub(in crate::world) fn begin_container_open(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Option<i32> {
        if self.levels[connection_id].inventory.container_id() != 0 {
            self.close_container(connection_id, true, events);
            if self.levels[connection_id].inventory.container_id() != 0 {
                return None;
            }
        }
        Some(
            self.levels[connection_id]
                .inventory
                .next_container_counter(),
        )
    }

    pub(in crate::world) fn container_menu_may_open(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        instances: &[BlockEntity],
        title_key: &str,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let player = self.levels.player_at(connection_id);
        let spectator = player.game_type == GameType::Spectator;
        let key = player.inventory.selected_stack();
        let registries = self.item_stack_resolver();
        for original in instances {
            let pos = original.block_pos();
            let entity =
                original.resolve_instance(self.level_by_id(&dimension).chunks.block_entities());
            let allowed = entity.can_open_container(spectator, key, &registries);
            match allowed {
                Ok(true) => continue,
                Err(error) => {
                    self.with_level(dimension, |level| level.record_item_failure(pos, error));
                    return false;
                }
                Ok(false) => {}
            }
            if !spectator || instances.len() == 2 {
                if let Some(title) =
                    self.container_menu_title_from_instances(dimension, instances, title_key)
                {
                    let message = bbb_protocol::component_translatable_nbt(
                        "container.isLocked",
                        vec![title.component().clone()],
                        &bbb_protocol::ComponentStyle::default(),
                    )
                    .and_then(|component| {
                        bbb_protocol::RichTextComponent::try_from_component_nbt_with_registry(
                            component,
                            self.plugins().content_registry(),
                        )
                    });
                    if let Ok(message) = message {
                        events.push(send(
                            connection_id,
                            ClientboundPlayPacket::SystemChat(
                                bbb_protocol::packets::SystemChat::from_component(message, true),
                            ),
                        ));
                    }
                }
                if let Some(registry_id) =
                    vanilla_sound_event_registry_id("minecraft:block.chest.locked")
                {
                    let first = instances[0].block_pos();
                    let last = instances
                        .last()
                        .expect("refused container has a position")
                        .block_pos();
                    let sound = SoundEvent {
                        sound: SoundEventHolder::Reference { registry_id },
                        source: SoundSource::Blocks,
                        position: Vec3d {
                            x: (f64::from(first.x) + f64::from(last.x)) * 0.5 + 0.5,
                            y: f64::from(first.y) + 0.5,
                            z: (f64::from(first.z) + f64::from(last.z)) * 0.5 + 0.5,
                        },
                        volume: 1.0,
                        pitch: 1.0,
                        seed: self.random.next_long(),
                    };
                    self.broadcast_level_sound_by_key(&dimension, &sound, None, events);
                }
            }
            // ServerPlayer.java:1330-1333: a refused menu tells a spectator why.
            if spectator {
                let message = bbb_protocol::component_translatable_nbt(
                    "container.spectatorCantOpen",
                    Vec::new(),
                    &bbb_protocol::ComponentStyle {
                        color: Some(0x00FF_5555),
                        ..Default::default()
                    },
                )
                .and_then(bbb_protocol::RichTextComponent::try_from_component_nbt);
                if let Ok(message) = message {
                    events.push(send(
                        connection_id,
                        ClientboundPlayPacket::SystemChat(
                            bbb_protocol::packets::SystemChat::from_component(message, true),
                        ),
                    ));
                }
            }
            return false;
        }
        true
    }

    pub(super) fn unpack_menu_container(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> CoreResult<()> {
        let dimension = dimension.into();
        let Some(original) = self.get_block_entity(dimension, pos).cloned() else {
            return Ok(());
        };
        self.unpack_original_menu_container(connection_id, dimension, &original, events)
    }

    fn unpack_original_menu_container(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        original: &BlockEntity,
        events: &mut Vec<WorldEvent>,
    ) -> CoreResult<()> {
        let dimension = dimension.into();
        let body = original.resolve_instance(self.level_by_id(&dimension).chunks.block_entities());
        let Some(table) = body.container_loot_table_id().map(str::to_owned) else {
            return Ok(());
        };
        self.trigger_generate_loot(connection_id, &table, events);
        let player = self.levels.player_at(connection_id);
        let view = self.loot_view_for_player(player);
        let luck = player
            .attributes
            .value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_LUCK_ID)
            .unwrap_or(0.0) as f32;
        self.with_level(dimension, |level| {
            level.unpack_original_container_loot(
                original,
                Some(ContainerLootOpener {
                    luck,
                    entity: LootEntity::Facts(&view),
                }),
            )
        })
    }

    /// `ServerGamePacketListenerImpl.handleContainerClose` (:1909-1912).
    ///
    ///
    pub(super) fn apply_container_close(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        self.close_container(connection_id, false, events);
    }

    pub(super) fn close_container(
        &mut self,
        connection_id: ConnectionId,
        announce: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        self.close_container_items(connection_id, announce, false, events);
    }

    /// `Player.remove` for a killed body (Player.java:1244-1249): the
    /// inventory menu and any open menu are removed with every stack dropped.
    pub(super) fn drop_removed_player_menus(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.levels[connection_id].inventory.container_id() != 0 {
            self.close_container_items(connection_id, false, true, events);
        }
        self.close_container_items(connection_id, false, true, events);
    }

    fn close_container_items(
        &mut self,
        connection_id: ConnectionId,
        announce: bool,
        removed: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let resolver = self.item_stack_resolver();
        let player = self.levels.player_at(connection_id);
        let closing_id = player.inventory.container_id();
        let instances = player
            .inventory
            .container_instances()
            .iter()
            .map(BlockEntity::clone)
            .collect::<Vec<_>>();
        let vehicle = player.inventory.vehicle_container();
        let merchant_trader = player.inventory.merchant_trader();
        let ender_chest_menu = player.inventory.is_ender_chest_menu();
        let is_spectator = player.game_type == GameType::Spectator;
        let instabuild = player.abilities.instabuild;
        let prepared = (|| {
            let inventory = &mut self.levels[connection_id].inventory;
            let closed = if removed {
                inventory.close_container_for_removed_player(&resolver)?
            } else {
                inventory.do_close_container(&resolver, instabuild)?
            };
            let mut updates = Vec::new();
            for &slot in &closed.filled {
                updates.push(send(
                    connection_id,
                    ClientboundPlayPacket::SetPlayerInventory(SetPlayerInventory {
                        slot: i32::try_from(slot).expect("inventory slots are small"),
                        item: inventory.inventory_items()[slot].summary().clone(),
                    }),
                ));
            }
            Ok::<_, bbb_server_types::ServerWorldError>((closed, updates))
        })();
        let (closed, updates) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                tracing::warn!("container close: {error}");
                return;
            }
        };
        if announce {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::ContainerClose(ContainerClose {
                    container_id: closing_id,
                }),
            ));
        }
        events.extend(updates);
        for stack in closed.dropped {
            if self
                .drop_player_original_stack(connection_id, stack, false)
                .is_none()
            {
                tracing::warn!("container close: original stack publication refused");
            }
        }
        if let Some((dimension, entity_id)) = vehicle {
            self.vehicle_container_event(connection_id, dimension, entity_id, false);
        } else if let Some(trader_id) = merchant_trader {
            let key = self.levels[connection_id].dimension;
            self.release_merchant_trader(&key, trader_id);
        } else if ender_chest_menu {
            self.change_ender_chest_openers(connection_id, false);
            self.levels[connection_id]
                .inventory
                .set_active_ender_chest(None);
        } else if closed.closed_container.is_some() {
            let dimension = self.player_level_id(connection_id);
            self.container_stop_open(connection_id, dimension, &instances, is_spectator, events);
        }
    }

    /// `ServerGamePacketListenerImpl.handleContainerClick` (:1915-1951).
    ///
    /// The click runs with remote updates suppressed, then the client's own
    /// prediction is stamped into the remote slots, and only then does the
    /// menu broadcast — so a client that predicted correctly is told nothing.
    /// A stale state id skips that agreement and resends the whole menu.
    ///
    ///
    pub(super) fn apply_container_click(
        &mut self,
        connection_id: ConnectionId,
        click: ContainerClick,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        if click.container_id != player.inventory.container_id() {
            return;
        }
        // `:1919-1920`: a spectator's click only resends the menu.
        if player.game_type == GameType::Spectator {
            self.refresh_open_container(connection_id);
            self.broadcast_player_menu(
                connection_id,
                crate::inventory::MenuBroadcastMode::FullState,
                events,
            );
            self.sync_menu_data(connection_id, true, events);
            return;
        }
        if !player.inventory.is_valid_slot_index(click.slot_num) {
            return;
        }
        match self.container_still_valid(connection_id) {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                tracing::warn!("container validity: {error}");
                return;
            }
        }
        self.refresh_open_container(connection_id);
        let resolver = self.item_stack_resolver();
        let entity_tags = self.registry_tags_for(connection_id);
        let armor_change_locks = std::sync::Arc::clone(&self.datapack_tables.armor_change_locks);
        let content_registry = std::sync::Arc::clone(self.plugins().content_registry());
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        player.inventory.set_armor_change_locks(armor_change_locks);
        player.inventory.set_entity_tags(entity_tags);
        player.inventory.set_content_registry(content_registry);
        let full_resync_needed = click.state_id != player.inventory.state_id();
        let crafted_item = (click.slot_num >= 0)
            .then(|| player.inventory.slot_item(click.slot_num).item_id())
            .flatten();
        let crafted_before = crafted_item.map_or(0, |id| player.inventory.crafted_count(id));
        // A double-click collecting into the cursor takes from the result
        // slot without clicking it.
        let furnace_result_item = player.inventory.furnace_result_item_id();
        player.inventory.suppress_remote_updates();
        let dropped = match player.inventory.clicked(
            &resolver,
            Some(&mut self.maps),
            click.slot_num,
            click.button_num,
            click.input,
            player.abilities.instabuild,
        ) {
            Ok(dropped) => dropped,
            Err(_) => {
                // A missing catalog fact cannot be recovered from mid-click;
                // the menu is resent so the client stops showing a state this
                // server never reached.
                player.inventory.resume_remote_updates();
                self.play_equip_changes(connection_id, events);
                self.refresh_open_container(connection_id);
                self.broadcast_player_menu(
                    connection_id,
                    crate::inventory::MenuBroadcastMode::FullState,
                    events,
                );
                self.sync_menu_data(connection_id, true, events);
                return;
            }
        };
        Self::send_menu_sync(connection_id, player.inventory.take_crafting_sync(), events);
        let bundle_sounds = player.inventory.take_bundle_sounds();
        for (slot, stack) in &click.changed_slots {
            player.inventory.set_remote_slot_unsafe(*slot, stack);
        }
        player.inventory.set_remote_carried(&click.carried_item);
        player.inventory.resume_remote_updates();
        if let Some(item_id) = crafted_item {
            let crafted = player.inventory.crafted_count(item_id) - crafted_before;
            if crafted > 0 {
                player.award_stat(
                    &mut self.scoreboard,
                    StatKey::new(VANILLA_STAT_TYPE_ITEM_CRAFTED_ID, item_id),
                    crafted,
                );
            }
        }
        let brewed_potion_take = player.inventory.take_brewed_potion_take();
        let furnace_take = player.inventory.take_furnace_result_take();
        if let Some((_takes, remove_count)) = furnace_take {
            if remove_count > 0 {
                if let Some(item_id) = furnace_result_item {
                    player.award_stat(
                        &mut self.scoreboard,
                        StatKey::new(VANILLA_STAT_TYPE_ITEM_CRAFTED_ID, item_id),
                        remove_count,
                    );
                }
            }
        }
        if let Some(potion) = brewed_potion_take {
            self.trigger_brewed_potion(connection_id, potion, events);
        }
        self.award_crafted_recipes(connection_id, events);
        self.settle_merchant_menu(connection_id, events);
        self.settle_utility_menu(connection_id, events);
        self.play_bundle_sounds(connection_id, &bundle_sounds, events);
        self.play_equip_changes(connection_id, events);
        // `AbstractContainerMenu.doClick` drops inside the click, so the
        // entities exist before the menu broadcast that follows it.
        if !self.drop_clicked_items(connection_id, dropped) {
            self.refresh_open_container(connection_id);
            tracing::warn!("container click: original stack publication refused");
            self.broadcast_player_menu(
                connection_id,
                crate::inventory::MenuBroadcastMode::FullState,
                events,
            );
            self.sync_menu_data(connection_id, true, events);
            return;
        }
        let mode = if full_resync_needed {
            crate::inventory::MenuBroadcastMode::FullState
        } else {
            crate::inventory::MenuBroadcastMode::Changes
        };
        if let Err(error) = self.flush_open_container(connection_id) {
            self.refresh_open_container(connection_id);
            tracing::warn!("container click: {error}");
            self.broadcast_player_menu(
                connection_id,
                crate::inventory::MenuBroadcastMode::FullState,
                events,
            );
            self.sync_menu_data(connection_id, true, events);
            return;
        }
        if furnace_take.is_some() {
            self.furnace_award_used_recipes(connection_id, events);
        }
        self.broadcast_player_menu(connection_id, mode, events);
        self.sync_menu_data(
            connection_id,
            mode == crate::inventory::MenuBroadcastMode::FullState,
            events,
        );
    }

    /// The `onEquipItem` calls the inventory menu's equipment slots made.
    pub(super) fn play_equip_changes(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let changes = self.levels[connection_id].inventory.take_equip_changes();
        for change in changes {
            self.player_equip_item(connection_id, change.slot, &change.old, &change.new, events);
        }
    }

    fn drop_clicked_items(
        &mut self,
        connection_id: ConnectionId,
        dropped: Vec<DroppedMenuStack>,
    ) -> bool {
        for DroppedMenuStack {
            stack,
            thrown_from_hand,
        } in dropped
        {
            if stack.is_empty() {
                continue;
            }
            if self
                .drop_player_original_stack(connection_id, stack, thrown_from_hand)
                .is_none()
            {
                return false;
            }
        }
        true
    }

    pub(super) fn apply_container_button_click(
        &mut self,
        connection_id: ConnectionId,
        click: ContainerButtonClick,
        events: &mut Vec<WorldEvent>,
    ) {
        let resolver = self.item_stack_resolver();
        let player = self.levels.player_at(connection_id);
        if click.container_id != player.inventory.container_id() {
            return;
        }
        if player.game_type == GameType::Spectator {
            return;
        }
        match self.container_still_valid(connection_id) {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                tracing::warn!("container validity: {error}");
                return;
            }
        }
        if self.apply_utility_button_click(connection_id, &click, events) {
            return;
        }
        let player = self.levels.player_at(connection_id);
        let dimension = self.player_level_id(connection_id);
        let Some(pos) = player
            .inventory
            .open_container_anchor()
            .and_then(|anchor| anchor.positions.first().copied())
        else {
            return;
        };
        let Some(current_page) = self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::lectern)
            .map(lectern::LecternBlockEntity::page)
        else {
            return;
        };
        let player = self.levels.player_at(connection_id);
        let may_build = player.may_build();
        let instabuild = player.abilities.instabuild;
        let inventory = &mut self.levels[connection_id].inventory;
        let next_page = inventory.lectern_button_page(click.button_id, current_page);
        let (accepted, taken) = inventory.click_menu_button(click.button_id, may_build);
        if !accepted {
            return;
        }
        if taken.is_some() {
            let Some(lectern) = self
                .level_mut_by_id(&dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(BlockEntity::lectern_mut)
            else {
                return;
            };
            let mut actual_book = lectern.remove_book();
            self.level_mut_by_id(&dimension)
                .chunks
                .block_entities_mut()
                .set_changed(pos);
            let Some(state) = self.block_state(dimension, pos) else {
                return;
            };
            self.with_level(dimension, |level| {
                lectern_block::reset_book_state(level, pos, state, None, false)
            });
            let added = (|| -> CoreResult<()> {
                self.levels[connection_id].inventory.add_anywhere(
                    &resolver,
                    &mut actual_book,
                    instabuild,
                )?;
                Ok(())
            })();
            if let Err(error) = added {
                self.with_level(dimension, |level| level.record_item_failure(pos, error));
                return;
            }
            if !actual_book.is_empty()
                && self
                    .drop_player_original_stack(connection_id, actual_book, false)
                    .is_none()
            {
                tracing::warn!("lectern book drop refused");
            }
            return;
        }
        let Some(next_page) = next_page else {
            return;
        };
        let Some(lectern) = self
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::lectern_mut)
        else {
            return;
        };
        if !lectern.set_page(next_page) {
            return;
        }
        self.level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .set_changed(pos);
        let Some(block_state_id) = self.block_state(dimension, pos) else {
            return;
        };
        self.with_level(dimension, |level| {
            lectern_block::signal_page_change(level, pos, block_state_id);
        });
        // `handleContainerButtonClick` broadcasts the clicker's menu at once;
        // other readers see the page on their next per-tick broadcast.
        self.sync_menu_data(connection_id, false, events);
    }

    /// Re-reads the block entities behind the open menu into the copy the menu
    /// clicks against.
    pub(super) fn refresh_open_container(&mut self, connection_id: ConnectionId) {
        if self.levels[connection_id].inventory.is_ender_chest_menu() {
            let inventory = &mut self.levels[connection_id].inventory;
            inventory.refresh_container_items(inventory.ender_chest_slots());
            return;
        }
        if self.levels[connection_id].inventory.is_merchant_menu() {
            self.refresh_merchant_menu(connection_id);
            return;
        }
        if self.levels[connection_id]
            .inventory
            .vehicle_container()
            .is_some()
        {
            self.refresh_vehicle_container(connection_id);
            return;
        }
        if self.levels[connection_id].inventory.is_mount_menu() {
            self.refresh_mount_menu(connection_id);
            return;
        }
        if self.levels[connection_id]
            .inventory
            .utility_menu_kind()
            .is_some()
        {
            self.refresh_utility_menu(connection_id);
            return;
        }
        let Some(positions) = self.levels[connection_id]
            .inventory
            .open_container_anchor()
            .map(|anchor| anchor.positions.clone())
        else {
            return;
        };
        let dimension = self.player_level_id(connection_id);
        let instances = self.levels[connection_id]
            .inventory
            .container_instances()
            .iter()
            .map(|original| {
                original.resolve_instance(self.level_by_id(&dimension).chunks.block_entities())
            })
            .collect::<Vec<_>>();
        if positions.len() == 1 {
            if self.levels[connection_id].inventory.is_crafter_menu() {
                if let Some((items, disabled)) = instances
                    .first()
                    .and_then(BlockEntity::crafter)
                    .map(|crafter| {
                        (
                            crafter
                                .original_items()
                                .iter()
                                .map(RuntimeItemStack::clone)
                                .collect(),
                            std::array::from_fn(|slot| crafter.is_slot_disabled(slot)),
                        )
                    })
                {
                    self.levels[connection_id]
                        .inventory
                        .refresh_crafter_state(items, disabled);
                }
                return;
            }
            if let Some(book) = instances
                .first()
                .and_then(BlockEntity::lectern)
                .map(lectern::LecternBlockEntity::original_book)
            {
                self.levels[connection_id]
                    .inventory
                    .refresh_container_items(vec![book]);
                return;
            }
        }
        let Some(items) = self.container_originals_from_instances(dimension, &instances) else {
            return;
        };
        self.levels[connection_id]
            .inventory
            .refresh_container_items(items);
    }

    pub(super) fn flush_open_container(&mut self, connection_id: ConnectionId) -> CoreResult<()> {
        if self.levels[connection_id].inventory.is_ender_chest_menu() {
            return Ok(());
        }
        if self.levels[connection_id]
            .inventory
            .vehicle_container()
            .is_some()
        {
            return self.flush_vehicle_container(connection_id);
        }
        if self.levels[connection_id].inventory.is_mount_menu() {
            self.flush_mount_menu(connection_id);
            return Ok(());
        }
        let player = self.levels.player_at(connection_id);
        if player.inventory.is_merchant_menu() || player.inventory.utility_menu_kind().is_some() {
            return Ok(());
        }
        let dimension = self.player_level_id(connection_id);
        let instances = player
            .inventory
            .container_instances()
            .iter()
            .map(|original| {
                let current = self
                    .level_by_id(&dimension)
                    .chunks
                    .block_entities()
                    .get(original.block_pos())
                    .filter(|current| current.same_position_and_type(original))
                    .unwrap_or(original);
                (
                    original.block_pos(),
                    original.type_id(),
                    original.block_state(),
                    current.container_slots().map(|items| items.len()),
                    current.lectern().is_some(),
                )
            })
            .collect::<Vec<_>>();
        let items = player
            .inventory
            .container_items()
            .iter()
            .map(RuntimeItemStack::clone)
            .collect::<Vec<_>>();
        if let [(pos, type_id, _, _, true)] = instances.as_slice() {
            return self.write_lectern_book(dimension, *pos, *type_id, items.first().cloned());
        }
        let mut items = items.into_iter();
        for (pos, type_id, block_state, slots, _) in instances {
            let Some(slots) = slots else {
                continue;
            };
            let slots = items.by_ref().take(slots).collect::<Vec<_>>();
            self.write_container_items_by_key(&dimension, pos, type_id, slots)?;
            self.with_level(dimension, |level| {
                level.block_entity_set_changed(pos, block_state)
            });
        }
        Ok(())
    }

    pub(super) fn write_lectern_book(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        type_id: i32,
        book: Option<RuntimeItemStack>,
    ) -> CoreResult<()> {
        let dimension = dimension.into();
        let empty = book.as_ref().is_none_or(RuntimeItemStack::is_empty);
        let Some(body) = self
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .filter(|body| body.type_id() == type_id)
        else {
            return Ok(());
        };
        let Some(lectern) = body.lectern_mut() else {
            return Ok(());
        };
        if let Some(book) = book {
            lectern.set_original_book(book)?;
        }
        if empty {
            lectern.on_book_removed();
        }
        let block_state_id = body.block_state();
        self.level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .set_changed(pos);
        self.with_level(dimension, |level| {
            lectern_block::reset_book_state(level, pos, block_state_id, None, !empty);
        });
        Ok(())
    }

    pub(super) fn send_menu_sync(
        connection_id: ConnectionId,
        sync: impl IntoIterator<Item = MenuSync>,
        events: &mut Vec<WorldEvent>,
    ) {
        for update in sync {
            let packet = match update {
                MenuSync::Content {
                    container_id,
                    state_id,
                    items,
                    carried,
                } => ClientboundPlayPacket::ContainerSetContent(ContainerSetContent {
                    container_id,
                    state_id,
                    items,
                    carried_item: carried,
                }),
                MenuSync::Slot {
                    container_id,
                    state_id,
                    slot,
                    item,
                } => ClientboundPlayPacket::ContainerSetSlot(ContainerSetSlot {
                    container_id,
                    state_id,
                    slot,
                    item,
                }),
                MenuSync::Cursor { item } => {
                    ClientboundPlayPacket::SetCursorItem(SetCursorItem { item })
                }
            };
            events.push(send(connection_id, packet));
        }
    }

    ///
    /// The answer is `InteractionResult.SUCCESS` whatever happened
    /// (`:267`), so a chest that refuses to open still consumes the click and
    /// the held item's `useOn` never runs.
    ///
    pub(super) fn use_chest_without_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let has_provider = !matches!(
            self.chest_combine(dimension, pos, block_state_id),
            chest_block::ChestCombine::None
        );
        self.open_chest_menu(connection_id, dimension, pos, block_state_id, events);
        if has_provider {
            let stat = if chest_block::is_trapped_chest(block_state_id) {
                "minecraft:trigger_trapped_chest"
            } else {
                "minecraft:open_chest"
            };
            self.award_opener_stat(connection_id, stat);
            self.anger_nearby_piglins(connection_id, true);
        }
        // `:267`: `InteractionResult.SUCCESS` whatever happened.
        true
    }

    /// `Player.awardStat(Identifier)` for one opener, which every
    /// `useWithoutItem` below runs immediately after `openMenu`.
    pub(in crate::world) fn award_opener_stat(
        &mut self,
        connection_id: ConnectionId,
        resource_id: &str,
    ) {
        self.levels[connection_id].award_stat_once(&mut self.scoreboard, custom_stat(resource_id));
    }

    /// Whether the position holds the block entity a `useWithoutItem` arm tests
    /// for with `instanceof` before it opens and counts.
    pub(in crate::world) fn has_container_block_entity(
        &self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        of_kind: fn(&BlockEntity) -> bool,
    ) -> bool {
        let dimension = dimension.into();
        self.level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .is_some_and(of_kind)
    }

    ///
    /// The counter is guarded by the `instanceof BarrelBlockEntity` test alone
    /// (`:46`, `:48`), not by whether a menu came back: a barrel whose
    /// `createMenu` answers null still counts.
    pub(super) fn use_barrel_without_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let is_barrel =
            self.has_container_block_entity(dimension, pos, |at| BlockEntity::barrel(at).is_some());
        self.open_barrel_menu(connection_id, dimension, pos, events);
        if is_barrel {
            self.award_opener_stat(connection_id, "minecraft:open_barrel");
            self.anger_nearby_piglins(connection_id, true);
        }
        true
    }

    pub(super) fn open_barrel_menu(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        if !self.has_container_block_entity(dimension, pos, |at| BlockEntity::barrel(at).is_some())
        {
            return false;
        }
        let Some(instances) = self.capture_container_instances(dimension, &[pos]) else {
            return false;
        };
        self.open_plain_container_menu(
            connection_id,
            dimension,
            instances,
            MenuLayout::Chest {
                rows: CHEST_MENU_ROWS,
            },
            "container.barrel",
            events,
        )
    }

    ///
    /// `InteractionResult.SUCCESS` whatever happened (`:85`): a lid that cannot
    /// open still consumes the click.
    pub(super) fn use_shulker_without_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let can_open = self.shulker_menu_may_open(
            dimension,
            pos,
            self.levels[connection_id].game_type == GameType::Spectator,
        );
        self.open_shulker_menu(connection_id, dimension, pos, events);
        if can_open {
            // `:81`, behind the same `canOpen` the menu is behind (`:79`).
            self.award_opener_stat(connection_id, "minecraft:open_shulker_box");
            self.anger_nearby_piglins(connection_id, true);
        }
        true
    }

    pub(super) fn open_shulker_menu(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let is_spectator = self.levels[connection_id].game_type == GameType::Spectator;
        if !self.shulker_menu_may_open(dimension, pos, is_spectator) {
            return false;
        }
        let Some(instances) = self.capture_container_instances(dimension, &[pos]) else {
            return false;
        };
        self.open_plain_container_menu(
            connection_id,
            dimension,
            instances,
            MenuLayout::ShulkerBox,
            "container.shulkerBox",
            events,
        )
    }

    pub(super) fn shulker_menu_may_open(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        is_spectator: bool,
    ) -> bool {
        let dimension = dimension.into();
        let Some(block_state_id) = self.block_state(dimension, pos) else {
            return false;
        };
        let Some(shulker) = self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::shulker_box)
        else {
            return false;
        };
        if is_spectator {
            return true;
        }
        if !shulker.is_closed() {
            return true;
        }
        // `level.noCollision(lidOpenBoundingBox)`: block shapes and the
        // entities that can be collided with (boats, minecarts, shulkers).
        let lid = shulker_box_block::lid_open_box(pos, block_state_id);
        !super::entity_placement::has_block_collision(self, dimension, lid)
            && self
                .level_by_id(&dimension)
                .entities
                .first_entity_in_box(lid.inflate(1.0e-7, 1.0e-7, 1.0e-7), |entity| {
                    entity.can_be_collided_with()
                })
                .is_none()
    }

    ///
    /// `:80` picks the counter off the block entity's own subclass, so the
    /// dropper is the same arm rather than a second one, and like the barrel it
    /// is guarded by the `instanceof` alone (`:78`).
    pub(super) fn use_dispenser_without_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let is_dispenser = self
            .has_container_block_entity(dimension, pos, |at| BlockEntity::dispenser(at).is_some());
        let dropper =
            dispenser_block::is_dropper(self.block_state(dimension, pos).unwrap_or_default());
        self.open_dispenser_menu(connection_id, dimension, pos, events);
        if is_dispenser {
            self.award_opener_stat(
                connection_id,
                if dropper {
                    "minecraft:inspect_dropper"
                } else {
                    "minecraft:inspect_dispenser"
                },
            );
        }
        true
    }

    pub(super) fn open_dispenser_menu(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        if !self
            .has_container_block_entity(dimension, pos, |at| BlockEntity::dispenser(at).is_some())
        {
            return false;
        }
        let title_key =
            if dispenser_block::is_dropper(self.block_state(dimension, pos).unwrap_or_default()) {
                "container.dropper"
            } else {
                "container.dispenser"
            };
        let Some(instances) = self.capture_container_instances(dimension, &[pos]) else {
            return false;
        };
        self.open_plain_container_menu(
            connection_id,
            dimension,
            instances,
            MenuLayout::Generic3x3,
            title_key,
            events,
        )
    }

    pub(super) fn open_hopper_menu(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        if !self.has_container_block_entity(dimension, pos, |at| BlockEntity::hopper(at).is_some())
        {
            return false;
        }
        let Some(instances) = self.capture_container_instances(dimension, &[pos]) else {
            return false;
        };
        self.open_plain_container_menu(
            connection_id,
            dimension,
            instances,
            MenuLayout::Hopper,
            "container.hopper",
            events,
        )
    }

    pub(super) fn open_crafter_menu(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        if !self.has_container_block_entity(dimension, pos, |at| BlockEntity::crafter(at).is_some())
        {
            return false;
        }
        let Some(instances) = self.capture_container_instances(dimension, &[pos]) else {
            return false;
        };
        self.open_plain_container_menu(
            connection_id,
            dimension,
            instances,
            MenuLayout::Crafter,
            "container.crafter",
            events,
        )
    }

    pub(in crate::world) fn open_plain_container_menu(
        &mut self,
        connection_id: ConnectionId,
        dimension: LevelId,
        instances: Vec<BlockEntity>,
        layout: MenuLayout,
        title_key: &str,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Some(container_id) = self.begin_container_open(connection_id, events) else {
            return false;
        };
        if !self.container_menu_may_open(connection_id, dimension, &instances, title_key, events) {
            return false;
        }
        if !matches!(
            layout,
            MenuLayout::Furnace { .. } | MenuLayout::BrewingStand
        ) {
            for original in &instances {
                if let Err(error) =
                    self.unpack_original_menu_container(connection_id, dimension, original, events)
                {
                    self.with_level(dimension, |level| {
                        level.record_item_failure(original.block_pos(), error)
                    });
                    return false;
                }
            }
        }
        let Some(items) = self.container_originals_from_instances(dimension, &instances) else {
            return false;
        };
        let resolver = self.item_stack_resolver();
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        let is_spectator = player.game_type == GameType::Spectator;
        let opened = player.inventory.prepare_reserved_menu(
            &resolver,
            if matches!(
                layout,
                MenuLayout::Furnace { .. } | MenuLayout::BrewingStand
            ) {
                None
            } else {
                Some(&mut self.maps)
            },
            container_id,
            layout,
            ContainerAnchor {
                positions: instances.iter().map(BlockEntity::block_pos).collect(),
            },
            items,
        );
        let opened = match opened {
            Ok(Some(opened)) => opened,
            Ok(None) => return false,
            Err(error) => {
                self.with_level(dimension, |level| {
                    level.record_item_failure(instances[0].block_pos(), error)
                });
                return false;
            }
        };

        let Some(title) =
            self.container_menu_title_from_instances(dimension, &instances, title_key)
        else {
            return false;
        };
        if matches!(layout, MenuLayout::Chest { .. } | MenuLayout::ShulkerBox) {
            self.levels[connection_id]
                .inventory
                .bind_container_instances(instances.clone());
            self.container_start_open(connection_id, dimension, &instances, is_spectator, events);
        } else {
            self.levels[connection_id]
                .inventory
                .bind_container_instances(instances);
        }
        if layout == MenuLayout::Crafter {
            self.refresh_open_container(connection_id);
        }

        // `:1337`: the open-screen packet goes out before `initMenu`, so the
        // contents below always land in a screen the client already has.
        events.push(send(
            connection_id,
            ClientboundPlayPacket::OpenScreen(OpenScreen {
                container_id: opened.container_id,
                menu_type_id: opened.menu_type_id,
                title,
            }),
        ));
        self.initialize_player_menu(connection_id, events);
        true
    }

    ///
    /// Answers whether a menu opened, which is what the spectator arm turns
    /// into `CONSUME` rather than `PASS`.
    pub(super) fn open_chest_menu(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let instances = match self.chest_combine(dimension, pos, block_state_id) {
            chest_block::ChestCombine::None => return false,
            chest_block::ChestCombine::Single(single) => {
                self.capture_container_instances(dimension, &[single])
            }
            chest_block::ChestCombine::Double(first, second) => {
                self.capture_container_instances(dimension, &[first, second])
            }
        };
        let Some(instances) = instances else {
            return false;
        };
        let title_key = if instances.len() == 1 {
            "container.chest"
        } else {
            "container.chestDouble"
        };
        let layout = MenuLayout::Chest {
            rows: if instances.len() == 1 {
                CHEST_MENU_ROWS
            } else {
                DOUBLE_CHEST_MENU_ROWS
            },
        };
        self.open_plain_container_menu(
            connection_id,
            dimension,
            instances,
            layout,
            title_key,
            events,
        )
    }

    pub(super) fn chest_combine(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
    ) -> chest_block::ChestCombine {
        let dimension = dimension.into();
        // Gather the two candidates before the state reader borrows the world.
        let mut known = BTreeSet::new();
        let candidates = [
            Some(pos),
            chest_block::connected_direction(block_state_id)
                .map(|direction| relative(pos, direction)),
        ];
        let mut cat_blocked = [false; 2];
        for (index, candidate) in candidates.into_iter().enumerate() {
            let Some(candidate) = candidate else {
                continue;
            };
            cat_blocked[index] = chest_block::is_cat_sitting_on_chest(
                candidate,
                crate::entity::store::EntityBodyReadView::new(
                    &self.level_by_id(&dimension).entities,
                    None,
                    None,
                ),
            );
            if self
                .level_by_id(&dimension)
                .chunks
                .block_entities()
                .get(candidate)
                .and_then(BlockEntity::chest)
                .is_some()
            {
                known.insert((candidate.x, candidate.y, candidate.z));
            }
        }
        chest_block::combine(
            pos,
            block_state_id,
            &mut |at| self.block_state_or_air(dimension, at),
            &mut |at| known.contains(&(at.x, at.y, at.z)),
            &mut |at| {
                candidates
                    .iter()
                    .position(|candidate| *candidate == Some(at))
                    .is_some_and(|index| cat_blocked[index])
            },
        )
    }

    pub(super) fn container_start_open(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        instances: &[BlockEntity],
        is_spectator: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        self.change_container_openers(
            connection_id,
            dimension,
            instances,
            is_spectator,
            true,
            events,
        );
    }

    pub(super) fn container_stop_open(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        instances: &[BlockEntity],
        is_spectator: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        self.change_container_openers(
            connection_id,
            dimension,
            instances,
            is_spectator,
            false,
            events,
        );
    }

    fn change_container_openers(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        instances: &[BlockEntity],
        is_spectator: bool,
        opening: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        if is_spectator {
            return;
        }
        let player = self.levels.player_at(connection_id);
        let source_entity_id = player.entity_id();
        let interaction_range = player.block_interaction_range();
        for original in instances {
            let pos = original.block_pos();
            let Some(body) = self
                .level_mut_by_id(&dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .filter(|current| current.same_position_and_type(original))
            else {
                continue;
            };
            if body.chest().is_some() {
                self.with_level(dimension, |level| {
                    level.change_chest_openers(pos, source_entity_id, interaction_range, opening);
                });
                continue;
            }
            if body.barrel().is_some() {
                self.with_level(dimension, |level| {
                    let state = level.block_state(pos);
                    barrel_block::change_openers(
                        level,
                        pos,
                        state,
                        source_entity_id,
                        interaction_range,
                        opening,
                    );
                });
                continue;
            }
            let Some(shulker) = body.shulker_box_mut() else {
                continue;
            };
            let count = if opening {
                shulker.start_open()
            } else {
                shulker.stop_open()
            };
            self.signal_shulker_open_count(dimension, pos, count);
            if (opening && count == 1) || (!opening && count <= 0) {
                // ShulkerBoxBlockEntity.java:176,189.
                self.container_game_event(dimension, pos, source_entity_id, opening);
                self.play_shulker_sound(
                    dimension,
                    pos,
                    if opening {
                        shulker_box_block::SHULKER_BOX_OPEN_SOUND
                    } else {
                        shulker_box_block::SHULKER_BOX_CLOSE_SOUND
                    },
                    events,
                );
            }
        }
    }

    fn container_game_event(
        &mut self,
        dimension: LevelId,
        pos: BlockPos,
        source_entity_id: i32,
        opening: bool,
    ) {
        let event = if opening {
            VanillaGameEvent::CONTAINER_OPEN
        } else {
            VanillaGameEvent::CONTAINER_CLOSE
        };
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(event, pos, Some(source_entity_id), None);
    }

    pub(super) fn signal_shulker_open_count(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        current: i32,
    ) {
        let dimension = dimension.into();
        let Some(block_state_id) = self.block_state(dimension, pos) else {
            return;
        };
        let Some(block_type_id) = vanilla_block_state_block_type_id(block_state_id) else {
            return;
        };
        self.level_mut_by_id(&dimension)
            .block_events
            .block_event(BlockEventData::new(
                pos,
                block_type_id,
                shulker_box::EVENT_SET_OPEN_COUNT,
                current,
            ));
    }

    /// `ShulkerBoxBlockEntity.startOpen` / `stopOpen` (`:177-178`, `:190-191`):
    /// `playSound` at the block's centre, volume 0.5, pitch
    /// `random.nextFloat() * 0.1F + 0.9F`.
    pub(super) fn play_shulker_sound(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        sound_resource_id: &str,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        let Some(registry_id) = vanilla_sound_event_registry_id(sound_resource_id) else {
            return;
        };
        let pitch = self.random.next_float() * 0.1 + 0.9;
        let sound = SoundEvent {
            sound: SoundEventHolder::Reference { registry_id },
            source: SoundSource::Blocks,
            position: Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
            volume: 0.5,
            pitch,
            seed: self.random.next_long(),
        };
        self.broadcast_level_sound_by_key(&dimension, &sound, None, events);
    }

    pub(super) fn container_still_valid(&self, connection_id: ConnectionId) -> CoreResult<bool> {
        if self.levels[connection_id].inventory.is_ender_chest_menu() {
            return self.ender_chest_still_valid(connection_id);
        }
        let player = self.levels.player_at(connection_id);

        if player.inventory.is_merchant_menu() {
            return Ok(self.merchant_menu_still_valid(connection_id));
        }
        if player.inventory.vehicle_container().is_some() {
            return Ok(self.vehicle_container_still_valid(connection_id));
        }
        if player.inventory.is_mount_menu() {
            return Ok(self.mount_menu_still_valid(connection_id));
        }
        if player.inventory.utility_menu_kind().is_some() {
            return Ok(self.utility_menu_still_valid(connection_id));
        }
        let Some(anchor) = player.inventory.open_container_anchor() else {
            return Ok(true);
        };
        let Ok(eye) = player.eye_position() else {
            return Ok(false);
        };
        // `AbstractContainerMenu.stillValid(access, player, Blocks.CRAFTING_TABLE)`
        // (`AbstractContainerMenu.java:93-95`, `CraftingMenu.java:104-106`).
        if player.inventory.is_crafting_table_menu() {
            return Ok(anchor.positions.iter().all(|&pos| {
                super::block_use::crafting_table::is_crafting_table(
                    self.level_by_id(&player.dimension).block_state(pos),
                ) && is_within_block_interaction_range(
                    eye,
                    pos,
                    player.block_interaction_range(),
                    CONTAINER_DISTANCE_BUFFER,
                )
            }));
        }
        if player
            .inventory
            .container_instances()
            .iter()
            .any(|original| {
                self.level_by_id(&player.dimension)
                    .chunks
                    .block_entities()
                    .get(original.block_pos())
                    .is_none_or(|current| !current.same_position_and_type(original))
            })
        {
            return Ok(false);
        }
        for &pos in &anchor.positions {
            let Some(entity) = self
                .level_by_id(&player.dimension)
                .chunks
                .block_entities()
                .get(pos)
            else {
                return Ok(false);
            };
            let valid = if player.inventory.is_crafter_menu() {
                entity.crafter().is_some()
            } else if let Some(lectern) = entity.lectern() {
                lectern.has_book()?
            } else {
                entity.container_slots().is_some()
            };
            if !valid
                || !is_within_block_interaction_range(
                    eye,
                    pos,
                    player.block_interaction_range(),
                    CONTAINER_DISTANCE_BUFFER,
                )
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub(in crate::world) fn capture_container_instances(
        &self,
        dimension: impl Into<LevelId>,
        positions: &[BlockPos],
    ) -> Option<Vec<BlockEntity>> {
        let dimension = dimension.into();
        positions
            .iter()
            .map(|pos| self.get_block_entity(dimension, *pos).cloned())
            .collect()
    }

    fn container_originals_from_instances(
        &self,
        dimension: impl Into<LevelId>,
        instances: &[BlockEntity],
    ) -> Option<Vec<RuntimeItemStack>> {
        let dimension = dimension.into();
        let mut items = Vec::new();
        for original in instances {
            items.extend(
                original
                    .resolve_instance(self.level_by_id(&dimension).chunks.block_entities())
                    .container_slots()?
                    .iter()
                    .cloned(),
            );
        }
        Some(items)
    }

    pub(in crate::world) fn container_menu_title_from_instances(
        &self,
        dimension: impl Into<LevelId>,
        instances: &[BlockEntity],
        default_title_key: &str,
    ) -> Option<bbb_protocol::RichTextComponent> {
        let dimension = dimension.into();
        instances
            .iter()
            .find_map(|original| {
                original
                    .resolve_instance(self.level_by_id(&dimension).chunks.block_entities())
                    .container_custom_name()
                    .and_then(|name| {
                        bbb_protocol::RichTextComponent::try_from_component_nbt(name.clone()).ok()
                    })
            })
            .or_else(|| chest_menu_title(default_title_key))
    }

    pub(super) fn container_items_at_by_key(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
    ) -> Option<Vec<RuntimeItemStack>> {
        self.level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::container_items)
    }

    pub(super) fn write_container_items_at_by_key(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        items: Vec<RuntimeItemStack>,
    ) -> CoreResult<()> {
        let Some(type_id) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .map(BlockEntity::type_id)
        else {
            return Ok(());
        };
        self.write_container_items_by_key(dimension, pos, type_id, items)
    }

    fn write_container_items_by_key(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        type_id: i32,
        items: Vec<RuntimeItemStack>,
    ) -> CoreResult<()> {
        let Some(body) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .filter(|body| body.type_id() == type_id)
        else {
            return Ok(());
        };
        if let Some(crafter) = body.crafter() {
            if items.len() != crafter.items().len() {
                return Ok(());
            }
            let previous = body.container_slots().expect("crafter slots");
            let changed = items
                .into_iter()
                .enumerate()
                .filter(|(slot, item)| previous[*slot] != *item)
                .collect::<Vec<_>>();
            return self.with_level(dimension, |level| {
                for (slot, item) in changed {
                    if level
                        .chunks
                        .block_entities()
                        .get(pos)
                        .is_some_and(|body| body.type_id() == type_id)
                    {
                        level.set_randomizable_container_item(pos, slot, item)?;
                    }
                }
                Ok(())
            });
        }
        let block_state = body.block_state();
        let furnace_cooking_total_time = if body.furnace().is_some() {
            super::block_use::furnace_input_cooking_time_for_items(
                block_state,
                items.first(),
                self.datapacks(),
                self.item_tags(),
            )
            .unwrap_or(200)
        } else {
            200
        };
        let item_tags = self
            .level_by_id(dimension)
            .chunks
            .registry_tags_shared()
            .map_or(
                crate::entity::EntityRegistryTagView::Builtin26_1,
                crate::entity::EntityRegistryTagView::Frozen,
            );
        let resolver = self.item_stack_resolver();
        let result = (|| -> CoreResult<bool> {
            let Some(body) = self
                .level_mut_by_id(dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .filter(|body| body.type_id() == type_id)
            else {
                return Ok(false);
            };
            let occupancy = if let Some(furnace) = body.furnace_mut() {
                furnace.set_items(&resolver, &items, furnace_cooking_total_time)?;
                false
            } else if let Some(stand) = body.brewing_stand_mut() {
                stand.set_items(&items);
                false
            } else if let Some(pot) = body.decorated_pot_mut() {
                pot.overwrite_items(&items);
                false
            } else if let Some(shelf) = body.shelf_mut() {
                shelf.overwrite_items(&items);
                false
            } else if let Some(bookshelf) = body.chiseled_bookshelf_mut() {
                bookshelf
                    .overwrite_items(&items, |item| {
                        item.item_resource_id().is_some_and(|id| {
                            item_tags.contains("minecraft:item", "minecraft:bookshelf_books", id)
                        })
                    })
                    .is_some()
            } else {
                false
            };
            if body.chiseled_bookshelf().is_none() {
                body.install_container_originals(items);
            }
            Ok(occupancy)
        })();
        if result.is_err() {
            self.with_level(dimension, |level| {
                level.block_entity_set_changed(pos, block_state)
            });
        }
        if result? {
            self.level_mut_by_id(dimension)
                .chunks
                .block_entities_mut()
                .set_changed(pos);
            self.sync_bookshelf_occupancy_by_key(dimension, pos);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "containers/tests.rs"]
mod tests;
