//! `ServerGamePacketListenerImpl.handlePlayerAction`'s actions that name no
//! block (`ServerGamePacketListenerImpl.java:1265-1304`).

use crate::connection::ConnectionId;
use bbb_facts::interaction_hands::InteractionHand;
use bbb_protocol::equipment_slots::{
    vanilla_equipment_slot_by_ordinal, EquipmentSlot, EquipmentSlotType,
};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{PlayerActionKind, SoundEvent, SoundSource};
use bbb_server_types::item_stack::RuntimeItemStack;

use super::{ServerWorld, WorldEvent};
use crate::level_event::VanillaGameEvent;

impl ServerWorld {
    pub(super) fn apply_player_hand_action(
        &mut self,
        connection_id: ConnectionId,
        action: PlayerActionKind,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        // `ServerGamePacketListenerImpl.java:1260`.
        if !player.client_loaded() {
            return;
        }
        let spectator = player.game_type == GameType::Spectator;
        match action {
            PlayerActionKind::Stab if spectator => {}
            PlayerActionKind::Stab => self.player_stab(connection_id, events),
            // `:1302-1304` has no spectator gate.
            PlayerActionKind::ReleaseUseItem => self.release_using_item(connection_id, events),
            PlayerActionKind::SwapItemWithOffhand
            | PlayerActionKind::DropItem
            | PlayerActionKind::DropAllItems
                if spectator => {}
            PlayerActionKind::SwapItemWithOffhand => {
                self.swap_hand_items(connection_id, events);
                // `:1286`.
                self.stop_using_item(connection_id);
            }
            PlayerActionKind::DropItem => self.drop_selected_item(connection_id, false),
            PlayerActionKind::DropAllItems => self.drop_selected_item(connection_id, true),
            PlayerActionKind::StartDestroyBlock
            | PlayerActionKind::AbortDestroyBlock
            | PlayerActionKind::StopDestroyBlock => {
                unreachable!("block-break actions go through handleBlockBreakAction")
            }
        }
    }

    /// `ServerPlayer.drop(boolean)` (`ServerPlayer.java:2064-2075`).
    fn drop_selected_item(&mut self, connection_id: ConnectionId, all: bool) {
        let inventory = &mut self.levels[connection_id].inventory;
        // `Inventory.removeFromSelected` (`Inventory.java:520-523`).
        let selected = inventory.selected_item_mut();
        let count = if all { selected.count() } else { 1 };
        let removed = selected.split(count);
        // `:2067-2069`: the client took the stack out itself before sending.
        let selected_slot = usize::from(inventory.selected_slot());
        if let Some(menu_slot) = inventory.find_inventory_slot(selected_slot) {
            let held = inventory.selected_stack().clone();
            inventory.set_remote_slot(menu_slot, &held);
        }
        if self.levels[connection_id].use_item_is_empty() {
            self.stop_using_item(connection_id);
        }
        self.drop_player_stack(connection_id, &removed, true);
    }

    /// `ServerGamePacketListenerImpl.java:1283-1285`.
    fn swap_hand_items(&mut self, connection_id: ConnectionId, events: &mut Vec<WorldEvent>) {
        let inventory = &mut self.levels[connection_id].inventory;
        let off_hand = inventory.off_hand_stack().clone();
        let main_hand = inventory.selected_stack().clone();
        inventory.set_item_in_hand(InteractionHand::OffHand, main_hand.clone());
        self.player_equip_item(
            connection_id,
            EquipmentSlot::OffHand,
            &off_hand,
            &main_hand,
            events,
        );
        self.levels[connection_id]
            .inventory
            .set_item_in_hand(InteractionHand::MainHand, off_hand.clone());
        self.player_equip_item(
            connection_id,
            EquipmentSlot::MainHand,
            &main_hand,
            &off_hand,
            events,
        );
    }

    /// `LivingEntity.onEquipItem` (`LivingEntity.java:666-690`) with the
    /// player's armor-only game event (`Player.java:1641-1643`).
    pub(super) fn player_equip_item<S: Clone + Into<RuntimeItemStack>>(
        &mut self,
        connection_id: ConnectionId,
        slot: EquipmentSlot,
        old: &S,
        new: &S,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(sound) = self.player_equip_effects(connection_id, slot, old, new) else {
            return;
        };
        let dimension = self.levels[connection_id].dimension;
        self.broadcast_level_sound_by_key(&dimension, &sound, None, events);
    }

    /// [`Self::player_equip_item`] for a caller with no event list: the
    /// sound joins the level's entity sounds.
    pub(super) fn player_equip_item_queued(
        &mut self,
        connection_id: ConnectionId,
        slot: EquipmentSlot,
        old: &RuntimeItemStack,
        new: &RuntimeItemStack,
    ) {
        let Some(sound) = self.player_equip_effects(connection_id, slot, old, new) else {
            return;
        };
        let dimension = self.levels[connection_id].dimension;
        self.level_mut_by_id(&dimension)
            .entities
            .emit_entity_sound(sound);
    }

    /// Raises the game event and answers the sound `onEquipItem` plays.
    fn player_equip_effects<S: Clone + Into<RuntimeItemStack>>(
        &mut self,
        connection_id: ConnectionId,
        slot: EquipmentSlot,
        old: &S,
        new: &S,
    ) -> Option<SoundEvent> {
        let old: RuntimeItemStack = old.clone().into();
        let new: RuntimeItemStack = new.clone().into();
        let resolver = self.item_stack_resolver();
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        if player.tick_count == 0 || player.is_spectator() {
            return None;
        }
        let Ok(false) = old.same_item_same_effective_components(&new, &resolver) else {
            return None;
        };
        let equippable = crate::entity::armor_stand::equippable_of_persistent(&new).ok()?;
        let position = player.position();
        let silent = player.silent;
        let sound = equippable
            .as_ref()
            .filter(|facts| facts.slot == slot && !silent)
            .and_then(|facts| facts.equip_sound.as_deref())
            .and_then(crate::entity::store::sound_holder_for_id)
            .map(|sound| SoundEvent {
                sound,
                // `Player.getSoundSource` (`Player.java:402-404`).
                source: SoundSource::Players,
                position,
                volume: 1.0,
                pitch: 1.0,
                seed: self.random.next_long(),
            });
        if vanilla_equipment_slot_by_ordinal(slot.ordinal())
            .is_some_and(|entry| entry.slot_type == EquipmentSlotType::HumanoidArmor)
        {
            let event = if equippable.is_some() {
                VanillaGameEvent::EQUIP
            } else {
                VanillaGameEvent::UNEQUIP
            };
            self.player_game_event(connection_id, event);
        }
        sound
    }
}

#[cfg(test)]
mod tests;
