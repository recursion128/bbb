//! Session adapters for the shared impulse landing context.

use super::{PlayerState, ServerWorld};
use crate::connection::ConnectionId;
use crate::entity::FluidTag;
use crate::explosion::ExplosionSourceEntity;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING;
use bbb_protocol::game_types::GameType;
use bbb_protocol::NbtSummaryValue;
use std::sync::Arc;

impl PlayerState {
    pub(in crate::world) fn reset_fall_impulse(&mut self) {
        self.cold.fall_impulse.reset();
        self.cold.impulse_explosion_cause = None;
    }

    pub(in crate::world) fn try_reset_fall_impulse(&mut self) {
        if self.cold.fall_impulse.try_reset() {
            self.cold.impulse_explosion_cause = None;
        }
    }

    pub(super) fn adjusted_impulse_fall_distance(&mut self, distance: f64) -> f64 {
        let (distance, reset) = self
            .cold
            .fall_impulse
            .adjust_fall_distance(distance, self.position().y);
        if reset {
            self.cold.impulse_explosion_cause = None;
        }
        distance
    }

    // ServerPlayer.java:1290-1293: only the player's wind charge establishes an impact point.
    pub(in crate::world) fn on_explosion_impulse(
        &mut self,
        source: Option<&ExplosionSourceEntity>,
        nbt: Option<Arc<NbtSummaryValue>>,
    ) {
        self.cold.impulse_explosion_cause = source.map(|source| (source.clone(), nbt));
        self.cold.fall_impulse.set_ignore_fall_damage(
            source.is_some_and(|source| source.entity_resource_id == "minecraft:wind_charge"),
            self.position(),
        );
    }
}

impl ServerWorld {
    // ServerGamePacketListenerImpl.java:1166-1172; LivingEntity.java:385-387.
    pub(in crate::world) fn try_reset_player_impulse_after_movement(
        &mut self,
        id: ConnectionId,
        on_ground: bool,
        on_climbable: bool,
    ) {
        let player = self.levels.player_mut(id).unwrap();
        let landed_in_liquid = player.delta_movement().y < f64::from(1.0e-5_f32)
            && (player.fluid_interaction.is_in_fluid(FluidTag::Water)
                || player.fluid_interaction.is_in_fluid(FluidTag::Lava));
        if on_ground
            || landed_in_liquid
            || on_climbable
            || player.game_type == GameType::Spectator
            || player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING)
            || player.using_item.is_auto_spin_attack()
        {
            player.try_reset_fall_impulse();
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod climbable_packet_tests;
