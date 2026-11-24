//! Session environment counters and ordered base-tick damage.

use super::{PlayerState, ServerWorld, WorldEvent};
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::{living::MAX_AIR_SUPPLY, FluidTag};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_OXYGEN_BONUS_ID;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_BUBBLE_COLUMN_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_ON_FIRE;
use bbb_protocol::packets::{BlockPos, EntityEvent};

impl PlayerState {
    pub(super) fn set_silent(&mut self, silent: bool) {
        self.silent_dirty |= self.silent != silent;
        self.silent = silent;
    }

    pub(super) fn set_air_supply(&mut self, air: i32) {
        self.air_supply_dirty |= self.air_supply != air;
        self.air_supply = air;
    }

    pub(super) fn set_ticks_frozen(&mut self, ticks: i32) {
        self.ticks_frozen_dirty |= self.ticks_frozen != ticks;
        self.ticks_frozen = ticks;
    }

    // Player.java:1754-1756: loading Fire bypasses this setter.
    pub(super) fn set_remaining_fire_ticks(&mut self, ticks: i32) {
        self.remaining_fire_ticks = if self.abilities.invulnerable {
            ticks.min(1)
        } else {
            ticks
        };
    }

    pub(super) fn clear_environment_on_death(&mut self) {
        self.remaining_fire_ticks = self.remaining_fire_ticks.min(0);
        self.set_ticks_frozen(0);
        self.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_ON_FIRE, self.has_visual_fire);
    }
}

impl ServerWorld {
    pub(super) fn tick_player_environment_base(&mut self, events: &mut Vec<WorldEvent>) {
        let connections: Vec<_> = self
            .levels
            .player_entries()
            .filter(|(_, player)| player.body_tick_enabled)
            .map(|(&id, _)| id)
            .collect();
        for id in connections {
            self.refresh_player_nautilus_breath(id, events);
            let player = self.levels.player_at(id);
            if player.remaining_fire_ticks > 0 {
                if player.remaining_fire_ticks % 20 == 0
                    && (player.environment_first_tick
                        || !player.fluid_interaction.is_in_fluid(FluidTag::Lava))
                {
                    self.hurt_connected_player(
                        id,
                        "minecraft:on_fire",
                        1.0,
                        None,
                        None,
                        None,
                        events,
                    );
                }
                // Entity.java:537: a fatal hurt may already have cleared Fire.
                let player = self.levels.player_mut(id).unwrap();
                player.set_remaining_fire_ticks(player.remaining_fire_ticks.wrapping_sub(1));
            }
            // Entity.java:544-546.
            let player = self.levels.player_mut(id).unwrap();
            if !player.environment_first_tick
                && player.fluid_interaction.is_in_fluid(FluidTag::Lava)
            {
                player.block_contact.fall_distance *= 0.5;
            }
            let player = self.levels.player_at(id);
            if player.position().y
                < f64::from(self.level_by_id(&player.dimension).chunks.min_y() - 64)
            {
                self.hurt_connected_player(
                    id,
                    "minecraft:out_of_world",
                    4.0,
                    None,
                    None,
                    None,
                    events,
                );
            }
            let player = self.levels.player_mut(id).unwrap();
            player.set_shared_flag(
                VANILLA_ENTITY_SHARED_FLAG_ON_FIRE,
                player.remaining_fire_ticks > 0 || player.has_visual_fire,
            );
            player.environment_first_tick = false;
            if !player.damage.is_dead_or_dying() {
                self.tick_player_wall_and_border(id, events);
                self.tick_player_air(id, events);
                self.check_player_changed_block(id, events);
            }
        }
    }

    // AbstractNautilus.java:260-267: the first passenger receives sixty ticks of breath.
    fn refresh_player_nautilus_breath(&mut self, id: ConnectionId, events: &mut Vec<WorldEvent>) {
        let player = self.levels.player_at(id);
        if player.has_effect_id(
            bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_BREATH_OF_THE_NAUTILUS_ID,
        ) && self.game_time() % 40 != 0
        {
            return;
        }
        let entities = &self.level_by_id(&player.dimension).entities;
        let rider = player.entity_id();
        let Some(vehicle) = entities.vehicle(rider) else {
            return;
        };
        if entities.passengers(vehicle).first() != Some(&rider)
            || !entities.get(vehicle).is_some_and(|entity| {
                matches!(
                    entity.entity_type_id(),
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID
                        | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
                )
            })
        {
            return;
        }
        let effect = crate::entity::living::mob_effects::ActiveMobEffect::with_visibility(
            "minecraft:breath_of_the_nautilus",
            60,
            0,
            true,
            true,
            true,
        )
        .expect("built-in nautilus breath effect");
        self.add_player_mob_effect(id, effect, events);
    }

    fn tick_player_wall_and_border(&mut self, id: ConnectionId, events: &mut Vec<WorldEvent>) {
        let player = self.levels.player_at(id);
        let Ok(eye) = player.eye_position() else {
            return;
        };
        let Ok(bounds) = bbb_server_types::interaction::player_pose_bounding_box(
            player.position(),
            player.pose_id,
            player.pose_scale(),
        ) else {
            return;
        };
        let level = self.level_by_id(&player.dimension);
        let in_wall = crate::entity::body_is_in_wall(
            player.game_type == super::GameType::Spectator
                || player.pose_id == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID,
            eye,
            (bounds.max_x - bounds.min_x) as f32,
            |pos| super::stored_block_state(&level.chunks, pos),
            |pos| super::movement_collision::block_collision_shape(&level.chunks, pos, None),
        );
        if in_wall == Some(true) {
            self.hurt_connected_player(id, "minecraft:in_wall", 1.0, None, None, None, events);
        } else if in_wall == Some(false) {
            let border = level.world_border;
            let time = self.game_time();
            let contains = |x: f64, z: f64| {
                x >= border.min_x_at(time)
                    && x < border.max_x_at(time)
                    && z >= border.min_z_at(time)
                    && z < border.max_z_at(time)
            };
            let epsilon = f64::from(1.0e-5_f32);
            if !contains(bounds.min_x, bounds.min_z)
                || !contains(bounds.max_x - epsilon, bounds.max_z - epsilon)
            {
                let distance =
                    border.distance_to_border_at(player.position().x, player.position().z, time)
                        + border.safe_zone();
                if distance < 0.0 && border.damage_per_block() > 0.0 {
                    let amount =
                        ((-distance * border.damage_per_block()).floor() as i32).max(1) as f32;
                    self.hurt_connected_player(
                        id,
                        "minecraft:outside_border",
                        amount,
                        None,
                        None,
                        None,
                        events,
                    );
                }
            }
        }
    }

    // Player.java:278-280,322-335: after effects tick, before cooldowns and pose.
    pub(super) fn tick_player_turtle_helmets(&mut self, events: &mut Vec<WorldEvent>) {
        let wearers: Vec<_> = self
            .levels
            .player_entries()
            .filter(|(_, player)| {
                player.body_tick_enabled
                    && !player.fluid_interaction.is_eye_in_fluid(FluidTag::Water)
            })
            .map(|(&id, player)| {
                let equipped = (|| -> bbb_server_types::Result<bool> {
                    for (slot, index) in player.inventory.equipment_slots() {
                        let Some(item) = player.inventory.slot(index) else {
                            continue;
                        };
                        if item.item_id()
                            == Some(bbb_protocol::item_types::VANILLA_ITEM_TURTLE_HELMET_ID)
                            && item.equippable_slot()? == Some(slot)
                        {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                })();
                (id, equipped)
            })
            .collect();
        for (id, equipped) in wearers {
            match equipped {
                Ok(true) => {}
                Ok(false) => continue,
                Err(error) => {
                    tracing::warn!("player turtle helmet tick: {error}");
                    continue;
                }
            }
            let effect = crate::entity::living::mob_effects::ActiveMobEffect::with_visibility(
                "minecraft:water_breathing",
                200,
                0,
                false,
                false,
                true,
            )
            .expect("built-in turtle helmet effect");
            self.add_player_mob_effect(id, effect, events);
        }
    }

    // LivingEntity.java:417-439: this precedes mob-effect duration updates.
    fn tick_player_air(&mut self, id: ConnectionId, events: &mut Vec<WorldEvent>) {
        let player = self.levels.player_at(id);
        let Ok(eye) = player.eye_position() else {
            return;
        };
        let eye_state = super::stored_block_state(
            &self.level_by_id(&player.dimension).chunks,
            BlockPos {
                x: eye.x.floor() as i32,
                y: eye.y.floor() as i32,
                z: eye.z.floor() as i32,
            },
        );
        let submerged = player.fluid_interaction.is_eye_in_fluid(FluidTag::Water)
            && vanilla_block_state_block_type_id(eye_state) != Some(VANILLA_BLOCK_BUBBLE_COLUMN_ID);
        let can_breathe_underwater = if submerged {
            match self.player_registry_tags(id).map(|tags| {
                tags.contains(
                    "minecraft:entity_type",
                    "minecraft:can_breathe_under_water",
                    "minecraft:player",
                )
            }) {
                Some(true) => true,
                Some(false) => false,
                _ => return,
            }
        } else {
            false
        };
        let water_breathing = player
            .has_effect_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_WATER_BREATHING_ID);
        let conduit = player
            .has_effect_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_CONDUIT_POWER_ID);
        let nautilus = player.has_effect_id(
            bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_BREATH_OF_THE_NAUTILUS_ID,
        );
        let player = self.levels.player_mut(id).unwrap();
        if submerged
            && !can_breathe_underwater
            && !player.abilities.invulnerable
            && !water_breathing
            && !conduit
            && !nautilus
        {
            let oxygen = player
                .attributes
                .value(VANILLA_ATTRIBUTE_OXYGEN_BONUS_ID)
                .unwrap_or(0.0);
            if oxygen <= 0.0 || self.random.next_double() < 1.0 / (oxygen + 1.0) {
                player.set_air_supply(player.air_supply.wrapping_sub(1));
            }
            if player.air_supply <= -20 {
                player.set_air_supply(0);
                let entity_id = player.entity_id();
                self.send_to_player_tracking_and_self(
                    id,
                    entity_id,
                    ClientboundPlayPacket::EntityEvent(EntityEvent {
                        entity_id,
                        event_id: 67,
                    }),
                    events,
                );
                self.hurt_connected_player(id, "minecraft:drown", 2.0, None, None, None, events);
            }
        } else if player.air_supply < MAX_AIR_SUPPLY
            && (!submerged || !nautilus || water_breathing || conduit)
        {
            player.set_air_supply(player.air_supply.wrapping_add(4).min(MAX_AIR_SUPPLY));
        }
        if submerged {
            let player = self.levels.player_at(id);
            let dimension = player.dimension;
            let level = self.level_by_id(&dimension);
            let dismounts = level
                .entities
                .external_player_vehicle(player.entity_id())
                .and_then(|vehicle| level.entities.get(vehicle))
                .and_then(|vehicle| {
                    bbb_protocol::entity_types::vanilla_entity_type(vehicle.entity_type_id())
                })
                .and_then(|kind| {
                    self.player_registry_tags(id).map(|tags| {
                        tags.contains(
                            "minecraft:entity_type",
                            "minecraft:dismounts_underwater",
                            kind.resource_id,
                        )
                    })
                })
                == Some(true);
            if dismounts
                && self
                    .stop_external_player_ride(
                        &dimension,
                        id,
                        super::external_player_ride_events::DismountLanding::Search,
                        events,
                    )
                    .unwrap_or(false)
            {
                self.apply_level_entity_lifecycle_by_key(&dimension, events);
            }
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod lifecycle_tests;

pub(super) mod contact;

mod fall;

mod impulse;

mod step;

#[cfg(test)]
mod silent_tests;

#[cfg(test)]
mod ground_jump_tests;
