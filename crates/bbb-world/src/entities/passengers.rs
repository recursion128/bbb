use bbb_protocol::packets::{
    EquipmentSlot as ProtocolEquipmentSlot, InteractionHand, MoveVehicle as ProtocolMoveVehicle,
    SetPassengers as ProtocolSetPassengers,
};
use bbb_protocol::{
    entity_types::{
        vanilla_entity_type_is_non_controlling_rider, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
    },
    item_types::{VANILLA_ITEM_CARROT_ON_A_STICK_ID, VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID},
};

use crate::{LocalPlayerInputState, WorldStore};

use super::schema::{
    vanilla_entity_metadata_happy_ghast_accessors_for_entity_type,
    vanilla_entity_metadata_mob_accessors_for_entity_type,
    vanilla_entity_metadata_type_has_custom_inventory_screen,
    vanilla_entity_metadata_type_is_camel, vanilla_entity_metadata_type_is_living,
    vanilla_entity_metadata_type_is_mob, vanilla_entity_metadata_type_is_player_rideable_jumping,
};
use super::{
    is_vanilla_abstract_horse_type, is_vanilla_abstract_nautilus_type, is_vanilla_boat_type,
    movement::{entity_distance_squared, entity_vec3},
    VehicleMoveReport,
};

const MOVE_VEHICLE_SNAP_EPSILON: f64 = 1.0e-5_f32 as f64;
const LOCAL_BOAT_TICK_SECONDS: f64 = 0.05;
const LOCAL_BOAT_TURN_DEGREES_PER_TICK: f32 = 1.0;
const LOCAL_BOAT_FORWARD_ACCELERATION_PER_TICK: f64 = 0.04;
const LOCAL_BOAT_BACKWARD_ACCELERATION_PER_TICK: f64 = -0.005;
const LOCAL_BOAT_TURN_ONLY_ACCELERATION_PER_TICK: f64 = 0.005;
const LOCAL_BOAT_HORIZONTAL_DAMPING: f64 = 0.9;

impl WorldStore {
    pub fn apply_set_passengers(&mut self, packet: ProtocolSetPassengers) -> bool {
        self.counters.entity_passenger_updates_received += 1;
        self.counters.entity_passenger_ids_received += packet.passenger_ids.len();
        let local_player_id = self.local_player_id;
        // Vanilla asks `vehicle.hasIndirectPassenger(player)` before ejecting
        // the old passenger tree. Preserve that pre-packet fact so repeated
        // SetPassengers packets do not replay first-mount presentation.
        let local_player_was_on_packet_vehicle =
            self.local_player_vehicle_chain_contains(packet.vehicle_id);
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
        let mut local_player_mounted_here = false;
        for passenger_id in packet.passenger_ids {
            if passenger_id == packet.vehicle_id || mounted.contains(&passenger_id) {
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
                    }
                }
            }
        }

        if local_player_was_on_packet_vehicle && !local_player_mounted_here {
            self.local_player_vehicle_id = None;
        }
        for passenger_id in previous_passengers {
            if !mounted.contains(&passenger_id) {
                self.entities.shulker_stopped_riding(passenger_id);
            }
        }
        for passenger_id in &mounted {
            self.entities.shulker_started_riding(*passenger_id);
        }
        self.entities.with_mount_mut(packet.vehicle_id, |vehicle| {
            *vehicle.passengers = mounted;
        });
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
        let packet_position = entity_vec3(packet.position);
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

    pub fn advance_local_boat_vehicle_input(
        &mut self,
        input: LocalPlayerInputState,
        dt_seconds: f64,
    ) -> Option<VehicleMoveReport> {
        let vehicle_id = self.local_player_first_passenger_root_boat_vehicle_id()?;
        let ticks = (dt_seconds.max(0.0) / LOCAL_BOAT_TICK_SECONDS).max(0.0);
        self.entities.with_transform_mut(vehicle_id, |transform| {
            if ticks > 0.0 {
                let turn = if input.focused {
                    axis(input.right, input.left)
                } else {
                    0.0
                };
                transform.y_rot = wrap_degrees_f32(
                    transform.y_rot + turn as f32 * LOCAL_BOAT_TURN_DEGREES_PER_TICK * ticks as f32,
                );

                let mut acceleration = 0.0;
                if input.focused {
                    if input.forward {
                        acceleration += LOCAL_BOAT_FORWARD_ACCELERATION_PER_TICK;
                    }
                    if input.backward {
                        acceleration += LOCAL_BOAT_BACKWARD_ACCELERATION_PER_TICK;
                    }
                    if turn != 0.0 && !input.forward && !input.backward {
                        acceleration += LOCAL_BOAT_TURN_ONLY_ACCELERATION_PER_TICK;
                    }
                }
                acceleration *= ticks;

                let yaw = f64::from(transform.y_rot).to_radians();
                transform.delta_movement.x += (-yaw.sin()) * acceleration;
                transform.delta_movement.z += yaw.cos() * acceleration;
                let damping = LOCAL_BOAT_HORIZONTAL_DAMPING.powf(ticks);
                transform.delta_movement.x *= damping;
                transform.delta_movement.z *= damping;
                transform.position.x += transform.delta_movement.x * ticks;
                transform.position.y += transform.delta_movement.y * ticks;
                transform.position.z += transform.delta_movement.z * ticks;
                transform.position_base = transform.position;
            }
        })?;

        let transform = self.entities.transform(vehicle_id)?;
        Some(VehicleMoveReport {
            vehicle_id,
            position: transform.position,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            on_ground: transform.on_ground.unwrap_or(false),
            snapped: false,
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
        if !self
            .entities
            .saddle_slot_contains_saddle_item(vehicle_id, &self.items.default_item_equipment_slots)
        {
            return None;
        }
        Some(vehicle_id)
    }

    /// Vanilla `PlayerRideableJumping.getJumpCooldown()` for the local
    /// player's controlled jumpable mount. `local_player_rideable_jumping_vehicle_id`
    /// applies the shared `canJump()` saddle gate. Non-cooldown mount classes
    /// use the interface default of zero; currently tracked cooldown classes
    /// are camel and camel husk. Nautilus dash cooldown is not yet
    /// reconstructed.
    pub fn local_player_rideable_jumping_vehicle_cooldown(
        &self,
        partial_ticks: f32,
    ) -> Option<f32> {
        let vehicle_id = self.local_player_rideable_jumping_vehicle_id()?;
        self.entities
            .player_rideable_jumping_cooldown(vehicle_id, partial_ticks)
            .map(|cooldown| cooldown.max(0.0))
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

    fn remove_passenger_from_vehicle(&mut self, vehicle_id: i32, passenger_id: i32) {
        self.entities.with_mount_mut(vehicle_id, |vehicle| {
            vehicle
                .passengers
                .retain(|existing| *existing != passenger_id);
        });
    }

    fn resolve_root_vehicle_id(&self, vehicle_id: i32) -> Option<i32> {
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

    fn local_player_first_passenger_root_vehicle_id(&self) -> Option<i32> {
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

    fn entity_controlling_passenger(&self, entity_id: i32) -> Option<i32> {
        let entity_type_id = self.entities.entity_type_id(entity_id)?;
        let first_passenger_id = self
            .entities
            .mount(entity_id)?
            .passengers
            .first()
            .copied()?;

        if is_vanilla_boat_type(entity_type_id) {
            return self
                .entity_id_is_living(first_passenger_id)
                .then_some(first_passenger_id);
        }

        let has_saddle = self
            .entities
            .saddle_slot_contains_saddle_item(entity_id, &self.items.default_item_equipment_slots);
        if is_vanilla_abstract_horse_type(entity_type_id)
            || is_vanilla_abstract_nautilus_type(entity_type_id)
        {
            if has_saddle && self.entity_id_is_player(first_passenger_id) {
                return Some(first_passenger_id);
            }
        } else if entity_type_id == VANILLA_ENTITY_TYPE_PIG_ID {
            if has_saddle
                && self.entity_id_is_player(first_passenger_id)
                && first_passenger_id == self.local_player_id?
                && self.local_player_holds_item(VANILLA_ITEM_CARROT_ON_A_STICK_ID)
            {
                return Some(first_passenger_id);
            }
        } else if entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID {
            if has_saddle
                && self.entity_id_is_player(first_passenger_id)
                && first_passenger_id == self.local_player_id?
                && self.local_player_holds_item(VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID)
            {
                return Some(first_passenger_id);
            }
        } else if entity_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
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

    fn entity_id_is_living(&self, entity_id: i32) -> bool {
        self.local_player_id == Some(entity_id)
            || self
                .entities
                .entity_type_id(entity_id)
                .is_some_and(vanilla_entity_metadata_type_is_living)
    }

    fn entity_id_is_player(&self, entity_id: i32) -> bool {
        self.local_player_id == Some(entity_id)
            || self.entities.entity_type_id(entity_id) == Some(VANILLA_ENTITY_TYPE_PLAYER_ID)
    }

    fn local_player_holds_item(&self, item_id: i32) -> bool {
        [InteractionHand::MainHand, InteractionHand::OffHand]
            .into_iter()
            .any(|hand| {
                self.local_item_in_hand(hand).and_then(|item| item.item_id) == Some(item_id)
            })
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

        if self.local_player_vehicle_id == Some(vehicle_id) {
            let game_time = self.world_time().map_or(0, |time| time.game_time);
            let current_pose = self.local_player.pose;
            let position = if self.entities.contains(local_player_id) {
                self.entities
                    .passenger_position(local_player_id, vehicle_id, game_time)
            } else {
                self.entities.local_player_passenger_position(
                    vehicle_id,
                    local_player_id,
                    current_pose.is_some_and(|pose| pose.swimming),
                    game_time,
                )
            };
            if let (Some(mut pose), Some(position)) = (current_pose, position) {
                pose.position.x = position.x;
                pose.position.y = position.y;
                pose.position.z = position.z;
                self.local_player.pose = Some(pose);
            }
            if let Some(position) = position {
                let _ = self
                    .entities
                    .with_transform_mut(local_player_id, |transform| {
                        transform.position = position;
                    });
            }
        }

        if let Some(pose) = self.local_player.pose {
            self.local_player_old_pose = Some(pose);
        }
        let _ = self
            .entities
            .with_transform_mut(local_player_id, |transform| transform.set_old_pos_and_rot());
        true
    }
}

fn axis(positive: bool, negative: bool) -> f64 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

fn wrap_degrees_f32(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

fn is_vanilla_player_rideable_jumping_type(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_is_player_rideable_jumping(entity_type_id)
}

fn is_vanilla_sprintable_vehicle_type(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_is_camel(entity_type_id)
}

fn is_vanilla_custom_inventory_screen_type(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_has_custom_inventory_screen(entity_type_id)
}
