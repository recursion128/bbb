use bbb_protocol::packets::{
    EntityDataValue, EntityDataValueKind, EquipmentSlot as ProtocolEquipmentSlot, InteractionHand,
    MoveVehicle as ProtocolMoveVehicle, SetPassengers as ProtocolSetPassengers,
};
use bbb_protocol::{
    entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
    entity_types::{
        vanilla_entity_type_is_non_controlling_rider, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
    },
    item_types::{VANILLA_ITEM_CARROT_ON_A_STICK_ID, VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID},
};

use crate::{LocalPlayerInputState, WorldStore};

use super::schema::{
    vanilla_entity_metadata_abstract_boat_accessors_for_entity_type,
    vanilla_entity_metadata_happy_ghast_accessors_for_entity_type,
    vanilla_entity_metadata_mob_accessors_for_entity_type,
    vanilla_entity_metadata_type_has_custom_inventory_screen,
    vanilla_entity_metadata_type_is_camel, vanilla_entity_metadata_type_is_living,
    vanilla_entity_metadata_type_is_mob, vanilla_entity_metadata_type_is_player_rideable_jumping,
};
use super::{
    is_vanilla_abstract_horse_type, is_vanilla_abstract_nautilus_type, is_vanilla_boat_type,
    movement::{entity_distance_squared, entity_vec3},
    EntityBoatStatus, VehicleMoveReport,
};

const MOVE_VEHICLE_SNAP_EPSILON: f64 = 1.0e-5_f32 as f64;
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
        ticks: u32,
    ) -> Option<VehicleMoveReport> {
        let vehicle_id = self.local_player_first_passenger_root_boat_vehicle_id()?;
        let entity_type_id = self.entities.entity_type_id(vehicle_id)?;
        let bounds = self.entities.pick_bounds(vehicle_id)?;
        let no_gravity = self.entities.no_gravity(vehicle_id).unwrap_or(false);
        let first_passenger_is_player = self
            .entities
            .mount(vehicle_id)
            .and_then(|mount| mount.passengers.first().copied())
            .is_some_and(|passenger| self.entity_id_is_player(passenger));
        let controls = BoatInput::from_local_input(input);
        for _ in 0..ticks {
            let transform = self.entities.transform(vehicle_id)?;
            let mut movement = self.entities.boat_movement(vehicle_id)?;
            let previous_supporting_block = self.entities.supporting_block(vehicle_id)?;
            let relative_min = bounds.min.map(f64::from);
            let relative_max = bounds.max.map(f64::from);
            let position = [
                transform.position.x,
                transform.position.y,
                transform.position.z,
            ];
            let aabb_min = std::array::from_fn(|axis| position[axis] + relative_min[axis]);
            let aabb_max = std::array::from_fn(|axis| position[axis] + relative_max[axis]);
            movement.old_status = movement.status;
            let (water_status, water_level) =
                crate::fluid::boat_water_status(self, aabb_min, aabb_max);
            movement.water_level = water_level;
            movement.status = Some(if let Some(status) = water_status {
                status
            } else {
                let friction = self.boat_ground_friction(position, relative_min, relative_max);
                if friction > 0.0 {
                    movement.land_friction = friction;
                    EntityBoatStatus::OnLand
                } else {
                    EntityBoatStatus::InAir
                }
            });
            if matches!(
                movement.status,
                Some(EntityBoatStatus::UnderWater | EntityBoatStatus::UnderFlowingWater)
            ) {
                movement.out_of_control_ticks += 1.0;
            } else {
                movement.out_of_control_ticks = 0.0;
            }

            let mut next_transform = transform;
            float_boat(
                self,
                entity_type_id,
                relative_min,
                relative_max,
                no_gravity,
                first_passenger_is_player,
                &mut movement,
                &mut next_transform,
            );
            control_boat(controls, &mut movement, &mut next_transform);
            let requested = next_transform.delta_movement;
            let clipped = self.clip_entity_collision_movement(
                [
                    next_transform.position.x,
                    next_transform.position.y,
                    next_transform.position.z,
                ],
                [requested.x, requested.y, requested.z],
                relative_min,
                relative_max,
            );
            next_transform.position.x += clipped[0];
            next_transform.position.y += clipped[1];
            next_transform.position.z += clipped[2];
            let x_collision = (requested.x - clipped[0]).abs() >= f64::from(1.0e-5_f32);
            let y_collision = requested.y != clipped[1];
            let z_collision = (requested.z - clipped[2]).abs() >= f64::from(1.0e-5_f32);
            let on_ground = y_collision && requested.y < 0.0;
            next_transform.on_ground = Some(on_ground);
            let supporting_block = on_ground
                .then(|| {
                    self.entity_supporting_block_after_collision(
                        entity_type_id,
                        position,
                        clipped,
                        relative_min,
                        relative_max,
                        previous_supporting_block,
                    )
                })
                .flatten();
            if x_collision {
                next_transform.delta_movement.x = 0.0;
            }
            if y_collision {
                next_transform.delta_movement.y = boat_vertical_collision_response(
                    self,
                    [
                        next_transform.position.x,
                        next_transform.position.y,
                        next_transform.position.z,
                    ],
                    supporting_block,
                    requested.y,
                );
            }
            if z_collision {
                next_transform.delta_movement.z = 0.0;
            }
            let speed_factor = boat_block_speed_factor(
                self,
                [
                    next_transform.position.x,
                    next_transform.position.y,
                    next_transform.position.z,
                ],
            );
            next_transform.delta_movement.x *= speed_factor;
            next_transform.delta_movement.z *= speed_factor;
            // `AbstractBoat.checkFallDamage` retains `getDeltaMovement().y`,
            // not the collision-clipped displacement supplied by `Entity.move`.
            movement.last_yd = requested.y;
            self.entities.with_boat_movement_transform_and_support_mut(
                vehicle_id,
                |stored, canonical, support| {
                    *stored = movement;
                    *canonical = next_transform;
                    if on_ground {
                        support.main = supporting_block;
                        support.on_ground_no_blocks = supporting_block.is_none();
                    } else {
                        *support = super::components::EntitySupportingBlock::default();
                    }
                },
            )?;
        }
        if ticks > 0 {
            let accessors =
                vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(entity_type_id)?;
            self.entities.with_metadata_mut(vehicle_id, |metadata| {
                set_metadata_bool(
                    metadata,
                    accessors.paddle_left_data_id,
                    controls.right && !controls.left || controls.forward,
                );
                set_metadata_bool(
                    metadata,
                    accessors.paddle_right_data_id,
                    controls.left && !controls.right || controls.forward,
                );
            })?;
        }

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

#[derive(Debug, Clone, Copy)]
struct BoatInput {
    left: bool,
    right: bool,
    forward: bool,
    backward: bool,
}

impl BoatInput {
    fn from_local_input(input: LocalPlayerInputState) -> Self {
        Self {
            left: input.focused && input.left,
            right: input.focused && input.right,
            forward: input.focused && input.forward,
            backward: input.focused && input.backward,
        }
    }
}

fn float_boat(
    world: &WorldStore,
    entity_type_id: i32,
    relative_min: [f64; 3],
    relative_max: [f64; 3],
    no_gravity: bool,
    first_passenger_is_player: bool,
    movement: &mut super::components::EntityBoatMovement,
    transform: &mut super::components::EntityTransform,
) {
    if movement.old_status == Some(EntityBoatStatus::InAir)
        && !matches!(
            movement.status,
            Some(EntityBoatStatus::InAir | EntityBoatStatus::OnLand)
        )
    {
        movement.water_level = transform.position.y + relative_max[1];
        let position = [
            transform.position.x,
            transform.position.y,
            transform.position.z,
        ];
        let aabb_min = std::array::from_fn(|axis| position[axis] + relative_min[axis]);
        let aabb_max = std::array::from_fn(|axis| position[axis] + relative_max[axis]);
        let target_y =
            crate::fluid::boat_water_level_above(world, aabb_min, aabb_max, movement.last_yd)
                - (relative_max[1] - relative_min[1])
                + 0.101;
        let target_position = [transform.position.x, target_y, transform.position.z];
        if world.entity_collision_bounds_are_free(
            entity_type_id,
            target_position,
            relative_min,
            relative_max,
            transform.on_ground.unwrap_or(false),
        ) {
            transform.position.y = target_y;
            transform.delta_movement.y = 0.0;
            movement.last_yd = 0.0;
        }
        movement.status = Some(EntityBoatStatus::InWater);
        return;
    }

    let mut vertical_speed = if no_gravity { 0.0 } else { -0.04 };
    let mut buoyancy = 0.0;
    let friction = match movement.status {
        Some(EntityBoatStatus::InWater) => {
            buoyancy =
                (movement.water_level - transform.position.y) / (relative_max[1] - relative_min[1]);
            0.9_f32
        }
        Some(EntityBoatStatus::UnderFlowingWater) => {
            vertical_speed = -7.0e-4;
            0.9
        }
        Some(EntityBoatStatus::UnderWater) => {
            buoyancy = f64::from(0.01_f32);
            0.45
        }
        Some(EntityBoatStatus::InAir) => 0.9,
        Some(EntityBoatStatus::OnLand) => {
            let friction = movement.land_friction;
            if first_passenger_is_player {
                movement.land_friction /= 2.0;
            }
            friction
        }
        None => 0.05,
    };
    let friction = f64::from(friction);
    transform.delta_movement.x *= friction;
    transform.delta_movement.y += vertical_speed;
    transform.delta_movement.z *= friction;
    movement.delta_rotation *= friction as f32;
    if buoyancy > 0.0 {
        transform.delta_movement.y = (transform.delta_movement.y + buoyancy * (0.04 / 0.65)) * 0.75;
    }
}

fn control_boat(
    input: BoatInput,
    movement: &mut super::components::EntityBoatMovement,
    transform: &mut super::components::EntityTransform,
) {
    if input.left {
        movement.delta_rotation -= 1.0;
    }
    if input.right {
        movement.delta_rotation += 1.0;
    }
    let mut acceleration = 0.0_f32;
    if input.right != input.left && !input.forward && !input.backward {
        acceleration += 0.005;
    }
    transform.y_rot = wrap_degrees_f32(transform.y_rot + movement.delta_rotation);
    if input.forward {
        acceleration += 0.04;
    }
    if input.backward {
        acceleration -= 0.005;
    }
    let yaw = transform.y_rot * (std::f64::consts::PI / 180.0) as f32;
    transform.delta_movement.x +=
        f64::from(super::client_movement::vanilla_mth_sin(-f64::from(yaw)) * acceleration);
    transform.delta_movement.z +=
        f64::from(super::client_movement::vanilla_mth_cos(f64::from(yaw)) * acceleration);
}

fn boat_vertical_collision_response(
    world: &WorldStore,
    position: [f64; 3],
    supporting_block: Option<crate::BlockPos>,
    requested_y: f64,
) -> f64 {
    let effect_pos = crate::BlockPos {
        x: supporting_block.map_or(position[0].floor() as i32, |pos| pos.x),
        y: (position[1] - 0.2).floor() as i32,
        z: supporting_block.map_or(position[2].floor() as i32, |pos| pos.z),
    };
    let block_name = world
        .probe_block(effect_pos)
        .and_then(|block| block.block_name);
    if requested_y >= 0.0 {
        return if block_name.as_deref().is_some_and(|name| {
            name == "minecraft:slime_block" || crate::terrain::block_name_is_bed(name)
        }) {
            requested_y
        } else {
            0.0
        };
    }
    match block_name.as_deref() {
        Some("minecraft:slime_block") => -requested_y * 0.8,
        Some(name) if crate::terrain::block_name_is_bed(name) => {
            -requested_y * f64::from(0.66_f32) * 0.8
        }
        _ => 0.0,
    }
}

fn boat_block_speed_factor(world: &WorldStore, position: [f64; 3]) -> f64 {
    let block_pos = crate::BlockPos {
        x: position[0].floor() as i32,
        y: position[1].floor() as i32,
        z: position[2].floor() as i32,
    };
    let here = world
        .probe_block(block_pos)
        .and_then(|block| block.block_name)
        .unwrap_or_default();
    if matches!(here.as_str(), "minecraft:water" | "minecraft:bubble_column") {
        return 1.0;
    }
    let here_factor = f64::from(crate::terrain::vanilla_block_speed_factor(Some(&here)));
    if here_factor != 1.0 {
        return here_factor;
    }
    let below = crate::BlockPos {
        y: (position[1] - f64::from(0.500001_f32)).floor() as i32,
        ..block_pos
    };
    world
        .probe_block(below)
        .and_then(|block| block.block_name)
        .as_deref()
        .map_or(1.0, |name| {
            f64::from(crate::terrain::vanilla_block_speed_factor(Some(name)))
        })
}

fn set_metadata_bool(metadata: &mut super::components::EntityMetadata, data_id: u8, value: bool) {
    if metadata.data_values.iter().any(|existing| {
        existing.data_id == data_id
            && existing.serializer_id == VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID
            && existing.value == EntityDataValueKind::Boolean(value)
    }) {
        return;
    }
    let value = EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(value),
    };
    if let Some(existing) = metadata
        .data_values
        .iter_mut()
        .find(|existing| existing.data_id == data_id)
    {
        *existing = value;
    } else {
        metadata.data_values.push(value);
        metadata.data_values.sort_by_key(|value| value.data_id);
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
