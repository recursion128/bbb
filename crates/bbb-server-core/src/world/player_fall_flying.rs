//! A session's elytra glide: `Player.tryToStartFallFlying` and
//! `LivingEntity.updateFallFlying` over the equipment the session wears.

use crate::connection::ConnectionId;
use crate::entity::FluidTag;
use crate::item_durability::{damage_and_max, hurt_and_break, MineBlockOutcome};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::{PlayerState, ServerWorld, WorldEvent};

const LEVITATION_RESOURCE_ID: &str = "minecraft:levitation";

/// `LivingEntity.canGlideUsing` (`LivingEntity.java:3937-3944`). A stack whose
/// components cannot be read is not a glider (fail-closed).
fn can_glide_using(stack: &RuntimeItemStack, slot: EquipmentSlot) -> bool {
    if stack.is_empty()
        || !stack
            .has_component(VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID)
            .unwrap_or(false)
        || stack.equippable_slot().ok().flatten() != Some(slot)
    {
        return false;
    }
    // `ItemStack.nextDamageWillBreak` (`ItemStack.java:441-443`).
    damage_and_max(stack.summary()).is_none_or(|(damage, max)| damage < max - 1)
}

fn glider_slots(player: &PlayerState) -> Vec<(EquipmentSlot, usize)> {
    player
        .inventory
        .equipment_slots()
        .filter(|(slot, index)| {
            player
                .inventory
                .slot(*index)
                .is_some_and(|stack| can_glide_using(stack, *slot))
        })
        .collect()
}

/// `LivingEntity.entityEventForEquipmentBreak` (`LivingEntity.java:3768-3779`).
pub(in crate::world) fn entity_event_for_equipment_break(slot: EquipmentSlot) -> i8 {
    match slot {
        EquipmentSlot::MainHand => 47,
        EquipmentSlot::OffHand => 48,
        EquipmentSlot::Head => 49,
        EquipmentSlot::Chest => 50,
        EquipmentSlot::Feet => 52,
        EquipmentSlot::Legs => 51,
        EquipmentSlot::Body => 65,
        EquipmentSlot::Saddle => 68,
    }
}

impl ServerWorld {
    pub(in crate::world) fn hurt_and_break_session_reference_checked(
        &mut self,
        connection_id: ConnectionId,
        slot: EquipmentSlot,
        reference: super::combat::EquipmentReference,
        amount: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let super::combat::EquipmentReference::Session { connection, .. } = &reference else {
            return Err("session equipment wear requires a session reference".into());
        };
        if *connection != connection_id {
            return Err("session equipment wear changed its connection".into());
        }
        let has_infinite_materials = self
            .levels
            .player(connection_id)
            .ok_or("the session left")?
            .abilities
            .instabuild;
        let table = std::sync::Arc::clone(&self.datapack_tables.enchantment_item_damage);
        let native_stack = self.referenced_equipment_stack(reference.clone())?;
        if native_stack.is_empty() {
            return Ok(());
        }
        let mut stack = native_stack.summary().clone();
        let enchantments = Self::resolve_value_effects(&stack, &table);
        let mut outcome = MineBlockOutcome::default();
        hurt_and_break(
            &mut stack,
            amount,
            has_infinite_materials,
            &enchantments,
            &self.random,
            &self.value_effect_level(&self.player_level_id(connection_id)),
            &mut outcome,
        );
        drop(enchantments);
        if let Some(raw_new_damage) = outcome.raw_new_damage {
            self.trigger_item_durability_changed(
                connection_id,
                &native_stack,
                raw_new_damage,
                events,
            );
        }
        let edited = RuntimeItemStack::from_summary(stack);
        self.update_session_item_reference(reference, edited)?;
        let Some(broken_item_id) = outcome.broken_item_id else {
            return Ok(());
        };
        self.on_player_equipped_item_broken_checked(connection_id, slot, broken_item_id, events)?;
        Ok(())
    }

    /// `Player.canGlide` (`Player.java:1405-1407`) over `LivingEntity.canGlide`
    /// (`LivingEntity.java:3139-3151`).
    fn player_can_glide(&self, connection_id: ConnectionId) -> bool {
        let player = self.levels.player_at(connection_id);
        !player.abilities.flying
            && !player.on_ground()
            && self
                .level_by_id(&player.dimension)
                .entities
                .external_player_vehicle(player.entity_id())
                .is_none()
            && player.active_effect(LEVITATION_RESOURCE_ID).is_none()
            && !glider_slots(player).is_empty()
    }

    /// `handlePlayerCommand`'s `START_FALL_FLYING` arm
    /// (`ServerGamePacketListenerImpl.java:1724-1728`) over
    /// `Player.tryToStartFallFlying` (`Player.java:1440-1447`).
    pub(super) fn start_player_fall_flying(&mut self, connection_id: ConnectionId) {
        let player = self.levels.player_at(connection_id);
        let starts = !player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING)
            && self.player_can_glide(connection_id)
            && !player.fluid_interaction.is_in_fluid(FluidTag::Water);
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        if starts {
            player.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, true);
        } else {
            player.stop_fall_flying();
        }
    }

    /// `LivingEntity.aiStep`'s `updateFallFlying` call (`LivingEntity.java:3051-3053`)
    /// and the `fallFlyTicks` step `LivingEntity.tick` takes after it
    /// (`:2790-2794`).
    pub(super) fn tick_player_fall_flying(&mut self, events: &mut Vec<WorldEvent>) {
        let bodies = self
            .levels
            .player_entries()
            .filter(|(_, player)| !player.avatar_removed && player.body_tick_enabled)
            .map(|(connection_id, _)| *connection_id)
            .collect::<Vec<_>>();
        for connection_id in bodies {
            if !self.levels[connection_id].shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING) {
                self.levels[connection_id].cold.fall_fly_ticks = 0;
                continue;
            }
            if let Err(detail) = self.update_player_fall_flying(connection_id, events) {
                tracing::warn!(%detail, "fall flying update failed");
                continue;
            }
            // `travelFallFlying` (`LivingEntity.java:2520-2522`): a climbable
            // ends the glide, and `travelInAir`'s climb resets the fall.
            if self.levels[connection_id].shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING)
                && self.player_on_climbable(connection_id)
            {
                self.levels[connection_id].stop_fall_flying();
                self.reset_player_fall_distance(connection_id, events);
            }
            let Some(player) = self.levels.player_mut(connection_id) else {
                continue;
            };
            player.cold.fall_fly_ticks =
                if player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING) {
                    player.cold.fall_fly_ticks + 1
                } else {
                    0
                };
        }
    }

    /// `LivingEntity.updateFallFlying` (`LivingEntity.java:3117-3137`).
    fn update_player_fall_flying(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the session was just read");
        // `Entity.checkFallDistanceAccumulation` (`Entity.java:2824-2828`). The
        // client owns the session's motion here, so its reported displacement
        // stands in for the `deltaMovement` vanilla's own glide travel computes.
        if player.known_movement.y > -0.5 && player.block_contact.fall_distance > 1.0 {
            player.block_contact.fall_distance = 1.0;
        }
        if !self.player_can_glide(connection_id) {
            self.levels[connection_id]
                .set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, false);
            return Ok(());
        }
        let check_fall_fly_ticks = self.levels[connection_id].cold.fall_fly_ticks + 1;
        if check_fall_fly_ticks % 10 != 0 {
            return Ok(());
        }
        if (check_fall_fly_ticks / 10) % 2 == 0 {
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("the session was just read");
            let gliders = glider_slots(player);
            // `Util.getRandom` (`Util.java:750-752`) on the entity's random.
            let pick = self.random.next_int(gliders.len() as i32) as usize;
            let (slot, index) = gliders[pick];
            self.hurt_and_break_equipment(connection_id, slot, index, 1, events)?;
        }
        self.player_game_event(connection_id, VanillaGameEvent::ELYTRA_GLIDE);
        Ok(())
    }

    /// `ItemStack.hurtAndBreak(int, LivingEntity, EquipmentSlot)`
    /// (`ItemStack.java:491-497`) on one worn slot, breaking through
    /// `ServerPlayer.onEquippedItemBroken` (`ServerPlayer.java:2200-2203`).
    pub(in crate::world) fn hurt_and_break_equipment(
        &mut self,
        connection_id: ConnectionId,
        slot: EquipmentSlot,
        index: usize,
        amount: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let target = connection_id;
        let player = self
            .levels
            .player(target)
            .ok_or("the original equipment owner disappeared")?;
        let connection = player.connection_id;
        let dimension = player.dimension;
        if !player
            .inventory
            .equipment_slots()
            .any(|entry| entry == (slot, index))
        {
            return Err("the equipment slot changed its inventory owner".into());
        }
        let Some(reference) = self.capture_equipment_reference(
            &dimension,
            super::combat::EnchantmentEffectEntity::Session(target),
            slot,
        )?
        else {
            return Ok(());
        };
        self.hurt_and_break_session_reference_checked(connection, slot, reference, amount, events)
    }
}
