//! `AbstractContainerMenu` remote slot tracking and `broadcastChanges`.

use crate::world::maps::ServerMapStore;
use bbb_facts::{
    PersistentRegistryMembership, PersistentRegistryResolver, PersistentRegistryTagMembers,
};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::packets::HashedStack;
use bbb_registry_codec::item_hash::ItemHashLookup;
use bbb_registry_codec::PersistentItemStackHolderResolver;

use super::{MenuSlot, MenuSync, PlayerInventoryMenu};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result;

/// Adapts the server's live [`PersistentItemStackHolderResolver`] and session
/// [`ContentRegistry`] into [`ItemHashLookup`] for canonical
/// `HashOps.CRC32C_INSTANCE` verification (`ServerPlayer.java:274-286`).
pub(crate) struct ServerItemHashLookup<'a> {
    resolver: &'a dyn PersistentItemStackHolderResolver,
    content_registry: &'a ContentRegistry,
}

impl<'a> ServerItemHashLookup<'a> {
    pub(crate) fn new(
        resolver: &'a dyn PersistentItemStackHolderResolver,
        content_registry: &'a ContentRegistry,
    ) -> Self {
        Self {
            resolver,
            content_registry,
        }
    }

    fn content_registry_contains(&self, registry_id: &str, resource_id: &str) -> bool {
        match registry_id {
            "minecraft:item" => self
                .content_registry
                .item_by_resource_id(resource_id)
                .is_some(),
            "minecraft:block" => self
                .content_registry
                .block_by_resource_id(resource_id)
                .is_some(),
            "minecraft:entity_type" => self
                .content_registry
                .entity_by_resource_id(resource_id)
                .is_some(),
            "minecraft:sound_event" => self
                .content_registry
                .sound_event_by_resource_id(resource_id)
                .is_some(),
            "minecraft:particle_type" => self
                .content_registry
                .particle_by_resource_id(resource_id)
                .is_some(),
            _ => false,
        }
    }
}

impl PersistentRegistryResolver for ServerItemHashLookup<'_> {
    fn registry_available(&self, registry_id: &str) -> bool {
        self.resolver.registry_available(registry_id)
    }

    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        if !self.content_registry.is_vanilla_only()
            && self.content_registry_contains(registry_id, resource_id)
        {
            return PersistentRegistryMembership::Present;
        }
        self.resolver.resource_membership(registry_id, resource_id)
    }

    fn tag_members(&self, registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers {
        self.resolver.tag_members(registry_id, tag_id)
    }

    fn tag_contains(&self, registry_id: &str, tag_id: &str, resource_id: &str) -> Option<bool> {
        self.resolver.tag_contains(registry_id, tag_id, resource_id)
    }
}

impl PersistentItemStackHolderResolver for ServerItemHashLookup<'_> {
    fn holder_id(&self, registry_id: &str, resource_id: &str) -> Option<i32> {
        if let Some(id) = self.resolver.holder_id(registry_id, resource_id) {
            return Some(id);
        }
        if self.content_registry.is_vanilla_only() {
            return None;
        }
        match registry_id {
            "minecraft:item" => self
                .content_registry
                .item_by_resource_id(resource_id)
                .map(|entry| entry.id),
            "minecraft:block" => self
                .content_registry
                .block_by_resource_id(resource_id)
                .map(|entry| entry.block_id),
            "minecraft:entity_type" => self
                .content_registry
                .entity_by_resource_id(resource_id)
                .map(|entry| entry.id),
            "minecraft:sound_event" => self
                .content_registry
                .sound_event_by_resource_id(resource_id)
                .map(|entry| entry.id),
            "minecraft:particle_type" => self
                .content_registry
                .particle_by_resource_id(resource_id)
                .map(|entry| entry.id),
            _ => None,
        }
    }

    fn holder_resource_id(&self, registry_id: &str, holder_id: i32) -> Option<String> {
        if let Some(id) = self.resolver.holder_resource_id(registry_id, holder_id) {
            return Some(id);
        }
        if self.content_registry.is_vanilla_only() {
            return None;
        }
        match registry_id {
            "minecraft:item" => self
                .content_registry
                .item_by_id(holder_id)
                .map(|entry| entry.resource_id.clone()),
            "minecraft:block" => self
                .content_registry
                .block_by_id(holder_id)
                .map(|entry| entry.resource_id.clone()),
            "minecraft:entity_type" => self
                .content_registry
                .entity_by_id(holder_id)
                .map(|entry| entry.resource_id.clone()),
            "minecraft:sound_event" => self
                .content_registry
                .sound_event_by_id(holder_id)
                .map(|entry| entry.resource_id.clone()),
            "minecraft:particle_type" => self
                .content_registry
                .particle_by_id(holder_id)
                .map(|entry| entry.resource_id.clone()),
            _ => None,
        }
    }

    fn cat_sound_variant(
        &self,
        holder_id: i32,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::CatSoundVariantElement> {
        self.resolver.cat_sound_variant(holder_id)
    }

    fn cow_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::CowSoundVariantElement> {
        self.resolver.cow_sound_variant(resource_id)
    }

    fn chicken_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::ChickenSoundVariantElement>
    {
        self.resolver.chicken_sound_variant(resource_id)
    }

    fn pig_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::PigSoundVariantElement> {
        self.resolver.pig_sound_variant(resource_id)
    }

    fn wolf_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::WolfSoundVariantElement> {
        self.resolver.wolf_sound_variant(resource_id)
    }

    fn wolf_sound_variant_count(&self) -> Option<usize> {
        self.resolver.wolf_sound_variant_count()
    }

    fn painting_variant_size(&self, resource_id: &str) -> Option<[i32; 2]> {
        self.resolver.painting_variant_size(resource_id)
    }

    fn jukebox_song(
        &self,
        holder_id: i32,
    ) -> Option<bbb_registry_codec::registry_codec::JukeboxSongRegistryValue> {
        self.resolver.jukebox_song(holder_id)
    }

    fn tag_holder_ids(
        &self,
        registry_id: &str,
        tag_id: &str,
    ) -> std::result::Result<Vec<i32>, String> {
        self.resolver.tag_holder_ids(registry_id, tag_id)
    }
}

impl ItemHashLookup for ServerItemHashLookup<'_> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuBroadcastMode {
    Changes,
    FullState,
    ListenersOnly,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuBroadcastEvent {
    InventoryChanged {
        inventory_slot: usize,
        changed_item: RuntimeItemStack,
    },
    Remote(MenuSync),
}

#[derive(Debug)]
pub struct MenuBroadcastCursor {
    epoch: u64,
    mode: MenuBroadcastMode,
    slot: i16,
    pending_remote: bool,
    done: bool,
}

impl PlayerInventoryMenu {
    /// `AbstractContainerMenu.suppressRemoteUpdates` / `resumeRemoteUpdates`
    /// (:780-786).
    pub fn suppress_remote_updates(&mut self) {
        self.menu.suppress_remote_updates = true;
    }

    pub fn resume_remote_updates(&mut self) {
        self.menu.suppress_remote_updates = false;
    }

    /// `AbstractContainerMenu.setRemoteSlot` (:284-286): a value this server
    /// knows the client already has, so nothing needs to be sent for it.
    pub fn set_remote_slot<S: Clone + Into<RuntimeItemStack>>(&mut self, slot: i16, item: &S) {
        let Ok(index) = usize::try_from(slot) else {
            return;
        };
        if let Some(remote) = self.menu.remote_slots.get_mut(index) {
            remote.force(item);
        }
    }

    /// `AbstractContainerMenu.findSlot(inventory, slotIndex)` (:808-817) over
    /// the player's own `Inventory` in the open menu.
    pub fn find_inventory_slot(&self, inventory_slot: usize) -> Option<i16> {
        (0..self.menu.layout.slot_count())
            .find(|&slot| self.menu.layout.slot(slot) == Some(MenuSlot::Inventory(inventory_slot)))
    }

    /// `AbstractContainerMenu.setRemoteSlotUnsafe` (:288-294), which logs and
    /// ignores a slot the menu does not have.
    pub fn set_remote_slot_unsafe(&mut self, slot: i16, stack: &HashedStack) {
        let Ok(index) = usize::try_from(slot) else {
            return;
        };
        if let Some(remote) = self.menu.remote_slots.get_mut(index) {
            remote.receive(stack);
        }
    }

    /// `AbstractContainerMenu.setRemoteCarried` (:296-298).
    pub fn set_remote_carried(&mut self, stack: &HashedStack) {
        self.menu.remote_carried.receive(stack);
    }

    /// `AbstractContainerMenu.sendAllDataToRemote` (:157-177) through
    /// `ServerPlayer.containerSynchronizer.sendInitialData`
    /// (ServerPlayer.java:289-296).
    pub fn send_all_data_to_remote(&mut self) -> Result<MenuSync> {
        let items = self.menu_items()?;
        let carried_projection = self.menu.carried.summary().clone();
        self.menu.synchronizer_attached = true;
        for index in 0..items.len() {
            let current = self.slot_item(index as i16).into_owned();
            self.menu.remote_slots[index].force(&current);
        }
        let carried = self.menu.carried.clone();
        self.menu.remote_carried.force(&carried);
        Ok(MenuSync::Content {
            container_id: self.menu.container_id,
            state_id: self.increment_state_id(),
            items,
            carried: carried_projection,
        })
    }

    pub fn suspend_menu_synchronizer(&mut self) {
        self.menu.synchronizer_attached = false;
    }

    pub fn begin_menu_broadcast(&self, mode: MenuBroadcastMode) -> MenuBroadcastCursor {
        MenuBroadcastCursor {
            epoch: self.menu_epoch,
            mode,
            slot: 0,
            pending_remote: false,
            done: false,
        }
    }

    /// Listener state commits before callbacks (`AbstractContainerMenu.java:237-257`).
    pub(crate) fn next_menu_broadcast(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        cursor: &mut MenuBroadcastCursor,
    ) -> Result<Option<MenuBroadcastEvent>> {
        if cursor.done || cursor.epoch != self.menu_epoch {
            cursor.done = true;
            return Ok(None);
        }
        loop {
            if std::mem::take(&mut cursor.pending_remote) {
                let slot = cursor.slot;
                cursor.slot += 1;
                if cursor.mode != MenuBroadcastMode::Changes
                    || !self.menu.synchronizer_attached
                    || self.menu.suppress_remote_updates
                {
                    continue;
                }
                // `AbstractContainerMenu.java:194-199`: a slot listener may have changed the slot.
                let current = self.slot_item(slot);
                if self.menu.remote_slots[slot as usize].matches_in_place(&current) == Some(true) {
                    continue;
                }
                let current = current.into_owned();
                let lookup = ServerItemHashLookup::new(resolver, &self.content_registry);
                if !self.menu.remote_slots[slot as usize].matches(&lookup, &current) {
                    let projected = current.summary().clone();
                    self.menu.remote_slots[slot as usize].force(&current);
                    return Ok(Some(MenuBroadcastEvent::Remote(MenuSync::Slot {
                        container_id: self.menu.container_id,
                        state_id: self.increment_state_id(),
                        slot,
                        item: projected,
                    })));
                }
            }
            if cursor.slot >= self.menu.layout.slot_count() {
                cursor.done = true;
                if !self.menu.synchronizer_attached {
                    return Ok(None);
                }
                match cursor.mode {
                    MenuBroadcastMode::FullState => {
                        return Ok(Some(MenuBroadcastEvent::Remote(
                            self.send_all_data_to_remote()?,
                        )))
                    }
                    MenuBroadcastMode::Changes
                        if !self.menu.suppress_remote_updates
                            && !self.menu.remote_carried.matches(
                                &ServerItemHashLookup::new(resolver, &self.content_registry),
                                &self.menu.carried,
                            ) =>
                    {
                        let projected = self.menu.carried.summary().clone();
                        self.menu.remote_carried.force(&self.menu.carried);
                        return Ok(Some(MenuBroadcastEvent::Remote(MenuSync::Cursor {
                            item: projected,
                        })));
                    }
                    _ => {}
                }
                return Ok(None);
            }
            let slot = cursor.slot;
            let item = self.slot_item(slot);
            cursor.pending_remote = true;
            if !self.menu.last_slots[slot as usize].matches(&item) {
                let item = item.into_owned();
                self.menu.last_slots[slot as usize] = item.clone();
                self.rematch_crafter_preview(resolver, maps.as_deref_mut())?;
                if let Some(MenuSlot::Inventory(inventory_slot)) = self.menu.layout.slot(slot) {
                    return Ok(Some(MenuBroadcastEvent::InventoryChanged {
                        inventory_slot,
                        changed_item: item,
                    }));
                }
            }
        }
    }

    pub(crate) fn broadcast_changes(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut ServerMapStore>,
    ) -> Result<Vec<MenuSync>> {
        self.broadcast_remote(resolver, maps, MenuBroadcastMode::Changes)
    }

    fn broadcast_remote(
        &mut self,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        mut maps: Option<&mut ServerMapStore>,
        mode: MenuBroadcastMode,
    ) -> Result<Vec<MenuSync>> {
        let mut cursor = self.begin_menu_broadcast(mode);
        let mut sync = Vec::new();
        while let Some(event) =
            self.next_menu_broadcast(resolver, maps.as_deref_mut(), &mut cursor)?
        {
            if let MenuBroadcastEvent::Remote(remote) = event {
                sync.push(remote);
            }
        }
        Ok(sync)
    }

    /// `AbstractContainerMenu.getStateId` (:819-821).
    pub fn state_id(&self) -> i32 {
        self.menu.state_id
    }
}

#[cfg(test)]
mod tests;
