//! Session fluid contact and active swimming over the shared entity scan.

use super::{GameType, ServerWorld};
use crate::connection::ConnectionId;
use crate::entity::{body_swimming_state, scan_body_fluid_interaction_with_current, FluidTag};
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAG_SPRINTING, VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
};
use bbb_protocol::packets::BlockPos;
use bbb_server_types::interaction::player_pose_bounding_box;

impl ServerWorld {
    pub(super) fn tick_player_fluid_contact(&mut self, events: &mut Vec<super::WorldEvent>) {
        let connections: Vec<_> = self.levels.player_keys().copied().collect();
        for id in connections {
            let player = self.levels.player_at(id);
            let Ok(bounds) =
                player_pose_bounding_box(player.position(), player.pose_id, player.pose_scale())
            else {
                continue;
            };
            let level = self.level_by_id(&player.dimension);
            // Entity.java:3587-3594: this gate differs from the fluid scan's margin.
            let expanded = bounds.inflate(1.0, 1.0, 1.0);
            let body_tick_enabled = !player.avatar_removed
                && (player.game_type != GameType::Spectator
                    || ((expanded.min_x.floor() as i32 >> 4)
                        ..=(expanded.max_x.ceil() as i32 >> 4))
                        .all(|x| {
                            ((expanded.min_z.floor() as i32 >> 4)
                                ..=(expanded.max_z.ceil() as i32 >> 4))
                                .all(|z| level.chunks.is_ticking_full(x, z))
                        }));
            if !body_tick_enabled {
                self.levels[id].body_tick_enabled = false;
                continue;
            }
            if player.environment_first_tick {
                if let Some(pos) = player.cold.sleeping_pos {
                    self.levels[id].set_position(super::player_sleep::sleep_position(pos));
                    self.update_player_query_section(id);
                }
            }
            self.levels[id].body_tick_enabled = true;
            self.tick_player_portal(id, events);
            let _ = self.refresh_player_fluid_and_swimming(id, events);
        }
    }

    fn refresh_player_fluid_and_swimming(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<super::WorldEvent>,
    ) -> Option<()> {
        let old_eye = self
            .levels
            .player(id)?
            .fluid_interaction
            .is_eye_in_fluid(FluidTag::Water);
        self.refresh_player_fluid_contact(id, events)?;
        let player = self.levels.player(id)?;
        let level = self.level_by_id(&player.dimension);
        let swimming = if player.abilities.flying
            || level
                .entities
                .external_player_vehicle(player.entity_id())
                .is_some()
        {
            false
        } else {
            body_swimming_state(
                player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SWIMMING),
                player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING),
                player.fluid_interaction.is_in_fluid(FluidTag::Water),
                old_eye,
                BlockPos {
                    x: player.position().x.floor() as i32,
                    y: player.position().y.floor() as i32,
                    z: player.position().z.floor() as i32,
                },
                &level.chunks,
            )?
        };
        self.levels
            .player_mut(id)?
            .set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SWIMMING, swimming);
        Some(())
    }

    pub(in crate::world) fn refresh_player_fluid_contact(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<super::WorldEvent>,
    ) -> Option<()> {
        use crate::level_event::VanillaGameEvent;
        use bbb_protocol::packets::{SoundEventHolder, SoundSource};
        let player = self.levels.player(id)?;
        let bounds =
            player_pose_bounding_box(player.position(), player.pose_id, player.pose_scale())
                .ok()?;
        let level = self.level_by_id(&player.dimension);
        let vehicle = match level.entities.external_player_vehicle(player.entity_id()) {
            Some(vehicle) => Some(level.entities.get(vehicle)?),
            None => None,
        };
        let contact = scan_body_fluid_interaction_with_current(
            bounds,
            player.eye_position().ok()?,
            vehicle,
            &level.chunks,
            player.abilities.flying,
        )?;
        let water = contact.is_in_fluid(FluidTag::Water);
        // Player.java:1454-1458: spectators make no splash.
        let splash = water
            && !player.was_touching_water
            && !player.environment_first_tick
            && player.game_type != GameType::Spectator;
        let position = player.position();
        let player = self.levels.player_mut(id)?;
        player.fluid_interaction = contact;
        if water {
            self.reset_player_fall_distance(id, events);
        }
        if splash {
            let player = self.levels.player(id)?;
            let (volume, pitch) =
                crate::entity::body_water_splash(player.delta_movement(), false, &self.random);
            let sound = if volume < 0.25 {
                "minecraft:entity.player.splash"
            } else {
                "minecraft:entity.player.splash.high_speed"
            };
            let registry_id =
                bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(sound)?;
            self.play_player_sound(
                id,
                Some(id),
                SoundEventHolder::Reference { registry_id },
                SoundSource::Players,
                position,
                volume,
                pitch,
                events,
            );
            self.player_game_event_at(id, VanillaGameEvent::SPLASH, position, None);
        }
        let player = self.levels.player_mut(id)?;
        player.was_touching_water = water;
        if !player.abilities.flying {
            if water {
                player.set_delta_movement(contact.movement_after_current(
                    FluidTag::Water,
                    player.delta_movement(),
                    0.014,
                    true,
                ));
            }
            if contact.is_in_fluid(FluidTag::Lava) {
                let dimension = player.dimension;
                let scale = if self.fast_lava_dimension_value(&dimension)? {
                    0.007
                } else {
                    0.0023333333333333335
                };
                let player = self.levels.player_mut(id)?;
                player.set_delta_movement(contact.movement_after_current(
                    FluidTag::Lava,
                    player.delta_movement(),
                    scale,
                    true,
                ));
            }
        }
        Some(())
    }
}

#[cfg(test)]
mod tests;
