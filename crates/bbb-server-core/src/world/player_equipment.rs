//! `LivingEntity.detectEquipmentUpdates` (`LivingEntity.java:2873-2951`) for a
//! session: what its attribute map takes and its trackers are told when a
//! slot's stack changes.

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{EntityEvent, EquipmentSlotUpdate, SetEquipment};
use bbb_server_types::item_stack::RuntimeItemStack;

use super::{ServerWorld, WorldEvent};

/// `EntityEvent.SWAP_HANDS` (`EntityEvent.java:51`).
const ENTITY_EVENT_SWAP_HANDS: i8 = 55;

impl super::PlayerState {
    pub(crate) fn armor_items(&self) -> impl Iterator<Item = &RuntimeItemStack> {
        self.equipment_slots().filter_map(|(slot, stack)| {
            matches!(
                slot,
                EquipmentSlot::Head
                    | EquipmentSlot::Chest
                    | EquipmentSlot::Legs
                    | EquipmentSlot::Feet
            )
            .then_some(stack)
        })
    }
}

impl ServerWorld {
    pub(in crate::world) fn on_player_equipped_item_broken_checked(
        &mut self,
        connection: ConnectionId,
        slot: EquipmentSlot,
        broken_item_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let player = self.levels.player(connection).ok_or("the session left")?;
        let entity_id = player.entity_id();
        let dimension = player.dimension;
        self.send_to_player_tracking_and_self_in_level(
            connection,
            entity_id,
            &dimension,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id,
                event_id: super::player_fall_flying::entity_event_for_equipment_break(slot),
            }),
            events,
        );
        let player = self.levels.player(connection).ok_or("the session left")?;
        let current = player
            .inventory
            .equipment_slots()
            .into_iter()
            .find(|(at, _)| *at == slot)
            .and_then(|(_, index)| player.inventory.slot(index))
            .cloned()
            .unwrap_or_else(RuntimeItemStack::empty);
        let table = self
            .level_by_id(&player.dimension)
            .entities
            .enchantment_attribute_effects_shared();
        let previous = (!current.is_empty()).then_some(&current);
        let registry = std::sync::Arc::clone(&self.datapack_tables.enchantment_slots);
        crate::entity::equipment_modifiers::apply_equipment_changes(
            &mut self.levels[connection].attributes,
            &[crate::entity::equipment_modifiers::EquipmentChange {
                slot,
                previous,
                current: None,
            }],
            &table,
            &registry,
        )?;
        self.stop_player_slot_location_effects_checked(connection, slot, &current)?;
        self.levels
            .player_mut(connection)
            .ok_or("the session left")?
            .award_stat_once(
                &mut self.scoreboard,
                bbb_server_types::stats::StatKey::new(
                    bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_BROKEN_ID,
                    broken_item_id,
                ),
            );
        Ok(())
    }

    /// `collectEquipmentChanges`' attribute half (`:2895-2915`).
    fn apply_player_equipment_modifiers(
        &mut self,
        connection_id: ConnectionId,
        changed: &[(EquipmentSlot, RuntimeItemStack)],
    ) -> Result<(), String> {
        let table = self
            .level_by_id(&self.levels[connection_id].dimension)
            .entities
            .enchantment_attribute_effects_shared();
        let registry = std::sync::Arc::clone(&self.datapack_tables.enchantment_slots);
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the session was just read");
        let changes = changed
            .iter()
            .map(
                |(slot, current)| crate::entity::equipment_modifiers::EquipmentChange {
                    slot: *slot,
                    previous: player
                        .cold
                        .last_equipment_items
                        .get(slot)
                        .filter(|stack| !stack.is_empty()),
                    current: (!current.is_empty()).then_some(current),
                },
            )
            .collect::<Vec<_>>();
        crate::entity::equipment_modifiers::apply_equipment_changes(
            &mut player.attributes,
            &changes,
            &table,
            &registry,
        )
    }

    /// `detectEquipmentUpdates` from `LivingEntity.tick` (`:2719`) for every
    /// session with a body.
    pub(super) fn detect_player_equipment_updates(&mut self, events: &mut Vec<WorldEvent>) {
        let empty = RuntimeItemStack::empty();
        let mut bodies = Vec::new();
        for (&connection_id, player) in self.levels.player_entries() {
            if player.avatar_removed {
                continue;
            }
            let any_changed = player.inventory.equipment_slots().any(|(slot, index)| {
                let current = player.inventory.slot(index).unwrap_or(&empty);
                let previous = player
                    .cold
                    .last_equipment_items
                    .get(&slot)
                    .unwrap_or(&empty);
                !current.matches(previous)
            });
            if any_changed {
                bodies.push(connection_id);
            }
        }
        for connection_id in bodies {
            self.detect_player_equipment(connection_id, events);
        }
    }

    fn detect_player_equipment(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self.levels.player_at(connection_id);
        let entity_id = player.entity_id();
        let empty = RuntimeItemStack::empty();
        // `collectEquipmentChanges` (`:2883-2922`) in `EquipmentSlot.VALUES` order.
        let mut changed = Vec::new();
        for (slot, index) in player.inventory.equipment_slots() {
            let current = player.inventory.slot(index).unwrap_or(&empty);
            let previous = player
                .cold
                .last_equipment_items
                .get(&slot)
                .unwrap_or(&empty);
            if !current.matches(previous) {
                changed.push((slot, current.clone()));
            }
        }
        if changed.is_empty() {
            return;
        }
        if let Err(detail) = self.apply_player_equipment_modifiers(connection_id, &changed) {
            tracing::warn!("player equipment modifiers: {detail}");
        }
        let location_changes = changed
            .iter()
            .map(|(slot, current)| {
                let previous = self.levels[connection_id]
                    .cold
                    .last_equipment_items
                    .get(slot)
                    .cloned()
                    .unwrap_or_else(RuntimeItemStack::empty);
                (*slot, previous, current.clone())
            })
            .collect::<Vec<_>>();
        self.player_location_equipment_changed(connection_id, &location_changes, events);
        let player = self.levels.player_at(connection_id);
        let last = |slot| {
            player
                .cold
                .last_equipment_items
                .get(&slot)
                .unwrap_or(&empty)
        };
        let changed_in = |slot| changed.iter().find(|(changed, _)| *changed == slot);
        // `handleHandSwap` (`:2928-2941`).
        let swap = match (
            changed_in(EquipmentSlot::MainHand),
            changed_in(EquipmentSlot::OffHand),
        ) {
            (Some((_, main)), Some((_, off)))
                if main.matches(last(EquipmentSlot::OffHand))
                    && off.matches(last(EquipmentSlot::MainHand)) =>
            {
                Some((main.clone(), off.clone()))
            }
            _ => None,
        };
        if let Some((main, off)) = swap {
            self.send_to_player_tracking(
                connection_id,
                entity_id,
                &ClientboundPlayPacket::EntityEvent(EntityEvent {
                    entity_id,
                    event_id: ENTITY_EVENT_SWAP_HANDS,
                }),
                events,
            );
            changed.retain(|(slot, _)| {
                !matches!(slot, EquipmentSlot::MainHand | EquipmentSlot::OffHand)
            });
            let last = &mut self.levels[connection_id].cold.last_equipment_items;
            last.insert(EquipmentSlot::MainHand, main);
            last.insert(EquipmentSlot::OffHand, off);
        }
        if changed.is_empty() {
            return;
        }
        let projected = changed
            .iter()
            .map(|(slot, stack)| EquipmentSlotUpdate {
                slot: *slot,
                item: stack.summary().clone(),
            })
            .collect::<Vec<_>>();
        // `handleEquipmentChanges` (`:2943-2951`).
        let last = &mut self.levels[connection_id].cold.last_equipment_items;
        for (slot, stack) in &changed {
            last.insert(*slot, stack.clone());
        }
        self.send_to_player_tracking(
            connection_id,
            entity_id,
            &ClientboundPlayPacket::SetEquipment(SetEquipment {
                entity_id,
                slots: projected,
            }),
            events,
        );
    }
}
