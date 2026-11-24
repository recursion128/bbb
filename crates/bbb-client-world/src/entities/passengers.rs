use bbb_protocol::packets::{
    EntityDataValue, EntityDataValueKind, EquipmentSlot as ProtocolEquipmentSlot, InteractionHand,
    MoveVehicle as ProtocolMoveVehicle, SetPassengers as ProtocolSetPassengers,
};
use bbb_protocol::{
    attribute_types::{
        VANILLA_ATTRIBUTE_FLYING_SPEED_ID, VANILLA_ATTRIBUTE_GRAVITY_ID,
        VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
        VANILLA_ATTRIBUTE_SNEAKING_SPEED_ID, VANILLA_ATTRIBUTE_STEP_HEIGHT_ID,
        VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID,
    },
    entity_data_serializers::{
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID, VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
    },
    entity_poses::VANILLA_ENTITY_POSE_STANDING_ID,
    entity_types::{
        vanilla_entity_type_can_float_while_ridden, vanilla_entity_type_is_non_controlling_rider,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_MINECART_ID,
        VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
    },
    item_types::{VANILLA_ITEM_CARROT_ON_A_STICK_ID, VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID},
    mob_effect_types::{
        VANILLA_MOB_EFFECT_DOLPHINS_GRACE_ID, VANILLA_MOB_EFFECT_JUMP_BOOST_ID,
        VANILLA_MOB_EFFECT_LEVITATION_ID, VANILLA_MOB_EFFECT_SLOW_FALLING_ID,
    },
};

use crate::{LocalPlayerInputState, WorldStore};

use super::schema::{
    vanilla_entity_metadata_abstract_boat_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_horse_accessors_for_entity_type,
    vanilla_entity_metadata_camel_dash_accessor_for_entity_type,
    vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type,
    vanilla_entity_metadata_controlling_passenger_owner, vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_happy_ghast_accessors_for_entity_type,
    vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type,
    vanilla_entity_metadata_mob_accessors_for_entity_type,
    vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type,
    vanilla_entity_metadata_type_has_custom_inventory_screen,
    vanilla_entity_metadata_type_is_camel, vanilla_entity_metadata_type_is_living,
    vanilla_entity_metadata_type_is_mob, vanilla_entity_metadata_type_is_player_rideable_jumping,
    VanillaEntityOwner,
};
use super::{
    is_vanilla_abstract_horse_type, is_vanilla_abstract_nautilus_type, is_vanilla_boat_type,
    movement::entity_distance_squared, EntityBlockEffectMovement, EntityBoatStatus, EntityVec3,
    VehicleMoveReport, VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
};

const MOVE_VEHICLE_SNAP_EPSILON: f64 = 1.0e-5_f32 as f64;

mod movement_helpers;
mod vehicle_input;

use movement_helpers::*;
pub(crate) use vehicle_input::clamp_local_player_boat_yaw;

impl WorldStore {
    pub fn apply_set_passengers(&mut self, packet: ProtocolSetPassengers) -> bool {
        self.counters.entity_passenger_updates_received += 1;
        self.counters.entity_passenger_ids_received += packet.passenger_ids.len();
        let local_player_id = self.local_player_id;
        // Vanilla asks `vehicle.hasIndirectPassenger(player)` before ejecting
        // the old passenger tree. Preserve that pre-packet fact so repeated
        // SetPassengers packets do not replay first-mount presentation, while
        // tracking direct mount separately so ejecting a nested intermediate
        // vehicle from an outer vehicle does not also dismount the local player
        // from that intermediate vehicle.
        let local_player_was_on_packet_vehicle =
            self.local_player_vehicle_chain_contains(packet.vehicle_id);
        let local_player_was_direct_passenger =
            self.local_player_vehicle_id == Some(packet.vehicle_id);
        if !self.entities.contains(packet.vehicle_id) {
            self.counters.entity_passenger_updates_ignored += 1;
            return false;
        }
        let previous_passengers = self
            .entities
            .mount(packet.vehicle_id)
            .map(|mount| mount.passengers.iter().copied().collect::<Vec<_>>())
            .unwrap_or_default();

        self.entities.for_each_mount_mut(|_, mount| {
            if mount.vehicle_id == Some(packet.vehicle_id) {
                mount.vehicle_id = None;
            }
        });
        self.entities
            .with_mount_mut(packet.vehicle_id, |vehicle| vehicle.passengers.clear());

        let mut mounted = Vec::new();
        let mut mounted_set = rustc_hash::FxHashSet::default();
        let mut local_player_mounted_here = false;
        for passenger_id in packet.passenger_ids {
            if passenger_id == packet.vehicle_id || !mounted_set.insert(passenger_id) {
                continue;
            }
            let is_local_player = local_player_id == Some(passenger_id);
            if is_local_player {
                if let Some(old_vehicle_id) = self.local_player_vehicle_id {
                    if old_vehicle_id != packet.vehicle_id {
                        self.remove_passenger_from_vehicle(old_vehicle_id, passenger_id);
                    }
                }
                self.local_player_vehicle_id = Some(packet.vehicle_id);
                self.removed_player_vehicle_id = None;
                // Vanilla `Entity.updateSwimming` (`Entity.java:1544-1551`) and
                // `LocalPlayer` riding state clear swimming/sneaking when riding.
                self.local_player.swimming = false;
                if let Some(pose) = self.local_player.pose.as_mut() {
                    pose.swimming = false;
                    pose.sneaking = false;
                }
                if !local_player_was_on_packet_vehicle {
                    self.apply_first_local_mount_side_effects(packet.vehicle_id);
                }
                local_player_mounted_here = true;
            }
            match self.entities.mount(passenger_id) {
                Some(passenger_mount) => {
                    if let Some(old_vehicle_id) = passenger_mount.vehicle_id {
                        if old_vehicle_id != packet.vehicle_id {
                            self.remove_passenger_from_vehicle(old_vehicle_id, passenger_id);
                        }
                    }
                    self.entities.with_mount_mut(passenger_id, |passenger| {
                        passenger.vehicle_id = Some(packet.vehicle_id);
                    });
                    mounted.push(passenger_id);
                }
                None => {
                    if is_local_player {
                        mounted.push(passenger_id);
                    } else {
                        mounted_set.remove(&passenger_id);
                    }
                }
            }
        }

        if local_player_was_direct_passenger && !local_player_mounted_here {
            self.local_player_vehicle_id = None;
        }
        for passenger_id in previous_passengers {
            if !mounted_set.contains(&passenger_id) {
                self.entities.shulker_stopped_riding(passenger_id);
            }
        }
        for passenger_id in &mounted {
            self.entities.shulker_started_riding(*passenger_id);
        }
        self.entities.with_mount_mut(packet.vehicle_id, |vehicle| {
            *vehicle.passengers = mounted;
        });
        if !local_player_was_on_packet_vehicle {
            self.correct_local_player_passenger_after_entity_snap(packet.vehicle_id);
        }
        self.counters.entity_passenger_updates_applied += 1;
        true
    }

    fn apply_first_local_mount_side_effects(&mut self, vehicle_id: i32) {
        let Some(vehicle_entity_type_id) = self.entities.entity_type_id(vehicle_id) else {
            return;
        };
        let Some(vehicle_transform) = self.entities.transform(vehicle_id) else {
            return;
        };
        if is_vanilla_boat_type(vehicle_entity_type_id) {
            if let Some(pose) = self.local_player.pose.as_mut() {
                pose.y_rot = vehicle_transform.y_rot;
            }
            if let Some(local_player_id) = self.local_player_id {
                self.entities
                    .with_transform_mut(local_player_id, |transform| {
                        transform.y_rot = vehicle_transform.y_rot;
                        transform.y_head_rot = vehicle_transform.y_rot;
                    });
            }
        }
        self.set_mount_onboard_overlay();
    }

    pub fn apply_move_vehicle(&mut self, packet: ProtocolMoveVehicle) -> Option<VehicleMoveReport> {
        self.counters.vehicle_moves_received += 1;
        let Some(root_vehicle_id) = self.local_player_first_passenger_root_vehicle_id() else {
            self.counters.vehicle_moves_ignored += 1;
            return None;
        };
        if !self.entity_is_locally_authoritative(root_vehicle_id) {
            self.counters.vehicle_moves_ignored += 1;
            return None;
        }
        let packet_position = packet.position;
        let Some(current_transform) = self.entities.transform(root_vehicle_id) else {
            self.counters.vehicle_moves_ignored += 1;
            return None;
        };
        let comparison_position = current_transform
            .interpolation_target()
            .map_or(current_transform.position, |(position, _, _)| position);
        let snapped = entity_distance_squared(comparison_position, packet_position).sqrt()
            > MOVE_VEHICLE_SNAP_EPSILON;

        if snapped {
            self.entities
                .with_transform_mut(root_vehicle_id, |transform| {
                    transform.cancel_interpolation();
                    transform.abs_snap_to(packet_position, packet.y_rot, packet.x_rot);
                });
            self.counters.vehicle_moves_snapped += 1;
        }

        let Some(transform) = self.entities.transform(root_vehicle_id) else {
            self.counters.vehicle_moves_ignored += 1;
            return None;
        };
        self.counters.vehicle_moves_applied += 1;
        self.counters.vehicle_moves_acked += 1;
        Some(VehicleMoveReport {
            vehicle_id: root_vehicle_id,
            position: transform.position,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            on_ground: transform.on_ground.unwrap_or(false),
            snapped,
        })
    }

    pub fn local_player_root_vehicle_id(&self) -> Option<i32> {
        self.resolve_root_vehicle_id(self.local_player_vehicle_id?)
    }

    pub fn entity_body_anchor_y_offset(
        &self,
        entity_id: i32,
        is_front: bool,
        partial_ticks: f32,
    ) -> Option<f32> {
        let game_time = self.world_time().map(|time| time.game_time).unwrap_or(0);
        self.entities
            .body_anchor_y_offset(entity_id, game_time, is_front, partial_ticks)
    }

    pub fn local_player_root_boat_vehicle_id(&self) -> Option<i32> {
        let vehicle_id = self.local_player_root_vehicle_id()?;
        self.entities
            .entity_type_id(vehicle_id)
            .filter(|entity_type_id| is_vanilla_boat_type(*entity_type_id))
            .map(|_| vehicle_id)
    }

    pub fn local_player_first_passenger_root_boat_vehicle_id(&self) -> Option<i32> {
        let vehicle_id = self.local_player_first_passenger_root_vehicle_id()?;
        self.entities
            .entity_type_id(vehicle_id)
            .filter(|entity_type_id| is_vanilla_boat_type(*entity_type_id))
            .map(|_| vehicle_id)
    }

    pub fn local_player_rideable_jumping_vehicle_id(&self) -> Option<i32> {
        let (Some(local_player_id), Some(vehicle_id)) =
            (self.local_player_id, self.local_player_vehicle_id)
        else {
            return None;
        };
        let mount = self.entities.mount(vehicle_id)?;
        if mount.passengers.first().copied() != Some(local_player_id) {
            return None;
        }
        let entity_type_id = self.entities.entity_type_id(vehicle_id)?;
        if !is_vanilla_player_rideable_jumping_type(entity_type_id) {
            return None;
        }
        if vanilla_entity_metadata_type_is_camel(entity_type_id)
            && camel_ridden_pose_state(self, vehicle_id)?.refuses_to_move
        {
            return None;
        }
        if !self
            .entities
            .saddle_slot_contains_saddle_item(vehicle_id, &self.items)
        {
            return None;
        }
        Some(vehicle_id)
    }

    /// Vanilla `PlayerRideableJumping.getJumpCooldown()` for the local
    /// player's controlled jumpable mount. `local_player_rideable_jumping_vehicle_id`
    /// applies the shared `canJump()` saddle gate. Non-cooldown mount classes
    /// use the interface default of zero; currently tracked cooldown classes
    /// are Camel/CamelHusk and AbstractNautilus.
    pub fn local_player_rideable_jumping_vehicle_cooldown(
        &self,
        partial_ticks: f32,
    ) -> Option<f32> {
        let vehicle_id = self.local_player_rideable_jumping_vehicle_id()?;
        self.entities
            .player_rideable_jumping_cooldown(vehicle_id, partial_ticks)
            .map(|cooldown| cooldown.max(0.0))
    }

    /// Exact integer `PlayerRideableJumping.getJumpCooldown()` used by
    /// `LocalPlayer.aiStep`; unlike the render projection, this is not
    /// partial-tick adjusted.
    pub fn local_player_rideable_jumping_vehicle_cooldown_ticks(&self) -> Option<i32> {
        let vehicle_id = self.local_player_rideable_jumping_vehicle_id()?;
        let entity_type_id = self.entities.entity_type_id(vehicle_id)?;
        if vanilla_entity_metadata_type_is_camel(entity_type_id) {
            return self
                .entities
                .camel_jump_cooldown_ticks(vehicle_id)
                .map(|cooldown| cooldown.max(0));
        }
        if is_vanilla_abstract_nautilus_type(entity_type_id) {
            return self
                .entities
                .nautilus_jump_cooldown_ticks(vehicle_id)
                .map(|cooldown| cooldown.max(0));
        }
        Some(0)
    }

    /// Applies the local `PlayerRideableJumping.onPlayerJump` call that
    /// Vanilla performs immediately before sending `START_RIDING_JUMP`.
    /// Horse-family state is retained canonically until its ridden tick;
    /// Camel and Nautilus subclasses own their distinct dash stages.
    pub fn apply_local_player_riding_jump(&mut self, jump_amount: i32) -> bool {
        let Some(vehicle_id) = self.local_player_rideable_jumping_vehicle_id() else {
            return false;
        };
        let Some(entity_type_id) = self.entities.entity_type_id(vehicle_id) else {
            return false;
        };
        let is_nautilus = is_vanilla_abstract_nautilus_type(entity_type_id);
        if !is_vanilla_abstract_horse_type(entity_type_id) && !is_nautilus {
            return false;
        }
        let should_stand = jump_amount >= 0;
        let jump_amount = jump_amount.max(0);
        let pending_scale = if jump_amount >= 90 {
            1.0
        } else {
            0.4 + 0.4 * jump_amount as f32 / 90.0
        };
        let is_camel = vanilla_entity_metadata_type_is_camel(entity_type_id);
        if is_camel {
            let Some(cooldown) = self.entities.camel_jump_cooldown_ticks(vehicle_id) else {
                return false;
            };
            let Some(transform) = self.entities.transform(vehicle_id) else {
                return false;
            };
            if cooldown > 0 || !transform.on_ground.unwrap_or(false) {
                return false;
            }
        }
        if is_nautilus
            && self
                .entities
                .nautilus_jump_cooldown_ticks(vehicle_id)
                .is_none_or(|cooldown| cooldown > 0)
        {
            return false;
        }
        let Some(()) = self
            .entities
            .with_ridden_jump_state_mut(vehicle_id, |state| {
                state.player_jump_pending_scale = pending_scale;
                if should_stand && !is_camel && !is_nautilus {
                    state.allow_stand_sliding = true;
                    state.local_stand_ticks_remaining = 20;
                } else if is_camel {
                    state.allow_stand_sliding = true;
                }
            })
        else {
            return false;
        };
        let Some(accessors) =
            vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id)
        else {
            return is_nautilus;
        };
        let flags = self
            .entities
            .metadata_byte(vehicle_id, accessors.flags_data_id, accessors.flags_default)
            .unwrap_or(accessors.flags_default);
        if should_stand && !is_camel && !is_nautilus {
            let flags = (flags & !accessors.eating_mask) | accessors.standing_mask;
            let _ = self.entities.with_metadata_mut(vehicle_id, |metadata| {
                set_metadata_byte(metadata, accessors.flags_data_id, flags);
            });
        }
        true
    }

    pub(super) fn stand_local_camel(&mut self, vehicle_id: i32, instantly: bool) -> Option<()> {
        let entity_type_id = self.entities.entity_type_id(vehicle_id)?;
        if !vanilla_entity_metadata_type_is_camel(entity_type_id) {
            return None;
        }
        if !camel_ridden_pose_state(self, vehicle_id)?.is_sitting {
            return Some(());
        }
        let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
        let (last_pose_data_id, _) =
            vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
                entity_type_id,
            )?;
        let game_time = self.world_time().map_or(0, |time| time.game_time);
        let last_pose_change_tick = if instantly {
            game_time.saturating_sub(53).max(0)
        } else {
            game_time
        };
        let _ = self.entities.with_metadata_mut(vehicle_id, |metadata| {
            set_metadata_pose(
                metadata,
                entity_accessors.pose_data_id,
                VANILLA_ENTITY_POSE_STANDING_ID,
            );
            set_metadata_long(metadata, last_pose_data_id, last_pose_change_tick);
        });
        Some(())
    }

    pub fn local_player_sprintable_vehicle_id(&self) -> Option<i32> {
        let (Some(local_player_id), Some(vehicle_id)) =
            (self.local_player_id, self.local_player_vehicle_id)
        else {
            return None;
        };
        let mount = self.entities.mount(vehicle_id)?;
        if mount.passengers.first().copied() != Some(local_player_id) {
            return None;
        }
        self.entities
            .entity_type_id(vehicle_id)
            .filter(|entity_type_id| is_vanilla_sprintable_vehicle_type(*entity_type_id))
            .map(|_| vehicle_id)
    }

    pub fn local_player_server_controlled_inventory_vehicle_id(&self) -> Option<i32> {
        let vehicle_id = self.local_player_vehicle_id?;
        self.entities
            .entity_type_id(vehicle_id)
            .filter(|entity_type_id| is_vanilla_custom_inventory_screen_type(*entity_type_id))
            .map(|_| vehicle_id)
    }

    pub(crate) fn clear_local_player_mount(&mut self, local_player_id: i32) {
        self.local_player_vehicle_id = None;
        self.entities.for_each_mount_mut(|entity_id, mount| {
            if entity_id == local_player_id {
                mount.vehicle_id = None;
            }
            mount
                .passengers
                .retain(|passenger_id| *passenger_id != local_player_id);
        });
    }

    pub(super) fn remove_passenger_from_vehicle(&mut self, vehicle_id: i32, passenger_id: i32) {
        self.entities.with_mount_mut(vehicle_id, |vehicle| {
            vehicle
                .passengers
                .retain(|existing| *existing != passenger_id);
        });
    }

    pub(super) fn resolve_root_vehicle_id(&self, vehicle_id: i32) -> Option<i32> {
        let mut root_vehicle_id = vehicle_id;
        for _ in 0..self.entities.len() {
            let mount = self.entities.mount(root_vehicle_id)?;
            let Some(parent_vehicle_id) = mount.vehicle_id else {
                return Some(root_vehicle_id);
            };
            root_vehicle_id = parent_vehicle_id;
        }
        None
    }

    pub(crate) fn entity_shares_local_player_root_vehicle(&self, entity_id: i32) -> bool {
        let Some(local_player_id) = self.local_player_id else {
            return false;
        };
        let local_root = self
            .local_player_vehicle_id
            .and_then(|vehicle_id| self.resolve_root_vehicle_id(vehicle_id))
            .unwrap_or(local_player_id);
        self.resolve_root_vehicle_id(entity_id) == Some(local_root)
    }

    pub(crate) fn local_player_vehicle_chain_contains(&self, entity_id: i32) -> bool {
        let Some(mut vehicle_id) = self.local_player_vehicle_id else {
            return false;
        };
        for _ in 0..=self.entities.len() {
            if vehicle_id == entity_id {
                return true;
            }
            let Some(parent) = self
                .entities
                .mount(vehicle_id)
                .and_then(|mount| mount.vehicle_id)
            else {
                return false;
            };
            vehicle_id = parent;
        }
        false
    }

    pub(crate) fn local_player_first_passenger_root_vehicle_id(&self) -> Option<i32> {
        let mut passenger_id = self.local_player_id?;
        let mut vehicle_id = self.local_player_vehicle_id?;
        for _ in 0..self.entities.len() {
            let mount = self.entities.mount(vehicle_id)?;
            if mount.passengers.first().copied() != Some(passenger_id) {
                return None;
            }
            passenger_id = vehicle_id;
            let Some(parent_vehicle_id) = mount.vehicle_id else {
                return Some(vehicle_id);
            };
            vehicle_id = parent_vehicle_id;
        }
        None
    }

    pub(super) fn entity_is_locally_authoritative(&self, entity_id: i32) -> bool {
        let Some(local_player_id) = self.local_player_id else {
            return false;
        };
        if entity_id == local_player_id {
            return true;
        }

        let mut controlled_id = entity_id;
        for _ in 0..=self.entities.len() {
            let Some(controller_id) = self.entity_controlling_passenger(controlled_id) else {
                return false;
            };
            if controller_id == local_player_id {
                return true;
            }
            controlled_id = controller_id;
        }
        false
    }

    pub(crate) fn entity_controlling_passenger(&self, entity_id: i32) -> Option<i32> {
        let entity_type_id = self.entities.entity_type_id(entity_id)?;
        let controller_owner = vanilla_entity_metadata_controlling_passenger_owner(entity_type_id)?;
        let first_passenger_id = self
            .entities
            .mount(entity_id)?
            .passengers
            .first()
            .copied()?;

        if controller_owner == VanillaEntityOwner::AbstractBoat {
            return self
                .entity_id_is_living(first_passenger_id)
                .then_some(first_passenger_id);
        }

        let has_saddle = self
            .entities
            .saddle_slot_contains_saddle_item(entity_id, &self.items);
        if matches!(
            controller_owner,
            VanillaEntityOwner::AbstractHorse | VanillaEntityOwner::AbstractNautilus
        ) {
            if has_saddle && self.entity_id_is_player(first_passenger_id) {
                return Some(first_passenger_id);
            }
        } else if controller_owner == VanillaEntityOwner::Pig {
            if has_saddle
                && self.entity_id_is_player(first_passenger_id)
                && self.player_holds_item(first_passenger_id, VANILLA_ITEM_CARROT_ON_A_STICK_ID)
            {
                return Some(first_passenger_id);
            }
        } else if controller_owner == VanillaEntityOwner::Strider {
            if has_saddle
                && self.entity_id_is_player(first_passenger_id)
                && self
                    .player_holds_item(first_passenger_id, VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID)
            {
                return Some(first_passenger_id);
            }
        } else if controller_owner == VanillaEntityOwner::HappyGhast {
            let accessors =
                vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(entity_type_id)?;
            let stays_still = self.entities.metadata_bool_for_entity(
                entity_id,
                accessors.stays_still_data_id,
                accessors.stays_still_default,
            )?;
            if !stays_still
                && self
                    .entities
                    .equipment_slot_contains_nonempty_item(entity_id, ProtocolEquipmentSlot::Body)
                && self.entity_id_is_player(first_passenger_id)
            {
                return Some(first_passenger_id);
            }
        } else if controller_owner != VanillaEntityOwner::Mob {
            return None;
        }

        // Vanilla `Mob.getControllingPassenger`: subclass overrides above fall
        // back to this exact generated ancestry/metadata/tag decision.
        let mob_accessors = vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id)?;
        let mob_flags = self.entities.metadata_byte(
            entity_id,
            mob_accessors.flags_data_id,
            mob_accessors.flags_default,
        )?;
        let no_ai = mob_flags & mob_accessors.no_ai_mask != 0;
        let first_passenger_type_id = self.entities.entity_type_id(first_passenger_id)?;
        (!no_ai
            && vanilla_entity_metadata_type_is_mob(first_passenger_type_id)
            && !vanilla_entity_type_is_non_controlling_rider(first_passenger_type_id))
        .then_some(first_passenger_id)
    }

    pub(super) fn entity_id_is_living(&self, entity_id: i32) -> bool {
        self.local_player_id == Some(entity_id)
            || self
                .entities
                .entity_type_id(entity_id)
                .is_some_and(vanilla_entity_metadata_type_is_living)
    }

    pub(super) fn entity_id_is_player(&self, entity_id: i32) -> bool {
        self.local_player_id == Some(entity_id)
            || self.entities.entity_type_id(entity_id) == Some(VANILLA_ENTITY_TYPE_PLAYER_ID)
    }

    pub(crate) fn local_player_holds_item(&self, item_id: i32) -> bool {
        [InteractionHand::MainHand, InteractionHand::OffHand]
            .into_iter()
            .any(|hand| {
                self.local_item_in_hand(hand).and_then(|item| item.item_id) == Some(item_id)
            })
    }

    pub(super) fn player_holds_item(&self, player_id: i32, item_id: i32) -> bool {
        if self.local_player_id == Some(player_id) {
            return self.local_player_holds_item(item_id);
        }
        [false, true].into_iter().any(|off_hand| {
            self.entities
                .held_item(player_id, off_hand)
                .and_then(|item| item.item_id)
                == Some(item_id)
        })
    }

    pub(crate) fn sync_local_player_passenger_position(&mut self) {
        let (Some(local_player_id), Some(_)) = (self.local_player_id, self.local_player_vehicle_id)
        else {
            return;
        };
        let game_time = self.world_time().map_or(0, |time| time.game_time);
        let Some(position) =
            self.entities
                .render_position_at_partial_tick(local_player_id, 1.0, game_time, false)
        else {
            return;
        };
        if let Some(pose) = self.local_player.pose.as_mut() {
            pose.position = position;
            pose.delta_movement = EntityVec3::default();
        }
        let _ = self
            .entities
            .with_transform_mut(local_player_id, |transform| {
                transform.position = position;
            });
    }

    pub(crate) fn local_player_minecart_render_position(
        &self,
        partial_tick: f32,
    ) -> Option<EntityVec3> {
        if !self.is_feature_enabled("minecraft:minecart_improvements") {
            return None;
        }
        let vehicle_id = self.local_player_vehicle_id?;
        if self.entities.entity_type_id(vehicle_id) != Some(VANILLA_ENTITY_TYPE_MINECART_ID)
            || !self.entities.minecart_has_render_steps(vehicle_id)
        {
            return None;
        }
        let game_time = self.world_time().map_or(0, |time| time.game_time);
        self.entities.render_position_at_partial_tick(
            self.local_player_id?,
            partial_tick,
            game_time,
            true,
        )
    }

    pub(super) fn correct_local_player_passenger_after_entity_snap(
        &mut self,
        vehicle_id: i32,
    ) -> bool {
        if !self.local_player_vehicle_chain_contains(vehicle_id) {
            return false;
        }
        let Some(local_player_id) = self.local_player_id else {
            return false;
        };

        self.sync_local_player_passenger_position();

        if let Some(pose) = self.local_player.pose {
            self.local_player_old_pose = Some(pose);
        }
        let _ = self
            .entities
            .with_transform_mut(local_player_id, |transform| transform.set_old_pos_and_rot());
        true
    }
}
