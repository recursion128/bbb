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
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
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
};
use super::{
    is_vanilla_abstract_horse_type, is_vanilla_abstract_nautilus_type, is_vanilla_boat_type,
    movement::{entity_distance_squared, entity_vec3},
    EntityBlockEffectMovement, EntityBoatStatus, EntityVec3, VehicleMoveReport,
    VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
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
        let paddle_left = controls.right && !controls.left || controls.forward;
        let paddle_right = controls.left && !controls.right || controls.forward;
        let entity_collision_targets = (ticks > 0)
            .then(|| self.entities.vehicle_entity_collision_targets())
            .unwrap_or_default();
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
                vehicle_id,
                entity_type_id,
                relative_min,
                relative_max,
                no_gravity,
                first_passenger_is_player,
                &entity_collision_targets,
                &mut movement,
                &mut next_transform,
            );
            control_boat(controls, &mut movement, &mut next_transform);
            let stuck_multiplier = self
                .entities
                .consume_block_effect_stuck_speed_multiplier(vehicle_id);
            if let Some(multiplier) = stuck_multiplier {
                next_transform.delta_movement.x *= multiplier.x;
                next_transform.delta_movement.y *= multiplier.y;
                next_transform.delta_movement.z *= multiplier.z;
            }
            let requested = next_transform.delta_movement;
            let clipped = self.clip_locally_authoritative_entity_collision_movement(
                vehicle_id,
                entity_type_id,
                [
                    next_transform.position.x,
                    next_transform.position.y,
                    next_transform.position.z,
                ],
                [requested.x, requested.y, requested.z],
                relative_min,
                relative_max,
                0.0,
                next_transform.on_ground.unwrap_or(false),
                &entity_collision_targets,
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
            let speed_factor = entity_block_speed_factor(
                self,
                [
                    next_transform.position.x,
                    next_transform.position.y,
                    next_transform.position.z,
                ],
            );
            next_transform.delta_movement.x *= speed_factor;
            next_transform.delta_movement.z *= speed_factor;
            if stuck_multiplier.is_some() {
                next_transform.delta_movement = EntityVec3::default();
            }
            // `AbstractBoat.checkFallDamage` retains `getDeltaMovement().y`,
            // not the collision-clipped displacement supplied by `Entity.move`;
            // a consumed stuck multiplier has already zeroed that delta.
            movement.last_yd = if stuck_multiplier.is_some() {
                0.0
            } else {
                requested.y
            };
            self.entities.with_boat_movement_transform_and_support_mut(
                vehicle_id,
                |stored, canonical, support| {
                    *stored = movement;
                    *canonical = next_transform;
                    if on_ground {
                        support.main = supporting_block;
                        support.on_ground_no_blocks = supporting_block.is_none();
                    } else {
                        support.clear_physical();
                    }
                },
            )?;
            // `AbstractBoat.tick` runs its two `applyEffectsFromBlocks`
            // passes after `move` and before the paddle/sound loop.
            self.apply_local_vehicle_block_effects(
                vehicle_id,
                EntityBlockEffectMovement::clipped(
                    EntityVec3 {
                        x: position[0],
                        y: position[1],
                        z: position[2],
                    },
                    next_transform.position,
                    requested,
                ),
            );
            self.entities
                .advance_deferred_boat_paddles(vehicle_id, paddle_left, paddle_right)?;
        }
        if ticks > 0 {
            let accessors =
                vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(entity_type_id)?;
            self.entities.with_metadata_mut(vehicle_id, |metadata| {
                set_metadata_bool(metadata, accessors.paddle_left_data_id, paddle_left);
                set_metadata_bool(metadata, accessors.paddle_right_data_id, paddle_right);
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

    /// Advances the locally controlled Happy Ghast through Vanilla's
    /// `LivingEntity.travelRidden` -> `HappyGhast.travelFlying` path. The
    /// canonical ECS remains the only mutable owner; publications receive the
    /// resulting transform through the normal immutable snapshot boundary.
    pub fn advance_local_happy_ghast_vehicle_input(
        &mut self,
        input: LocalPlayerInputState,
        ticks: u32,
    ) -> Option<VehicleMoveReport> {
        let vehicle_id = self.local_player_first_passenger_root_vehicle_id()?;
        if self.entities.entity_type_id(vehicle_id)? != VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            || self.entity_controlling_passenger(vehicle_id) != self.local_player_id
        {
            return None;
        }
        let bounds = self.entities.pick_bounds(vehicle_id)?;
        let relative_min = bounds.min.map(f64::from);
        let relative_max = bounds.max.map(f64::from);
        let flying_speed = self
            .entities
            .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_FLYING_SPEED_ID)?;
        let controller = self.local_player.pose?;
        let controller_is_camera = self.local_player.camera.follows_player;
        let slow_factor = if controller.sneaking || controller.swimming {
            self.local_player_id
                .and_then(|id| {
                    self.entities
                        .attribute_value(id, VANILLA_ATTRIBUTE_SNEAKING_SPEED_ID)
                })
                .unwrap_or(0.3)
        } else {
            1.0
        };
        let ridden_input = HappyGhastInput::from_local_input(
            input,
            controller.x_rot,
            flying_speed,
            controller_is_camera,
            slow_factor,
        );
        let travel_speed = (flying_speed as f32 * 5.0_f32 / 3.0_f32) as f64;
        let entity_collision_targets = (ticks > 0)
            .then(|| self.entities.vehicle_entity_collision_targets())
            .unwrap_or_default();

        for _ in 0..ticks {
            let transform = self.entities.transform(vehicle_id)?;
            let previous_supporting_block = self.entities.supporting_block(vehicle_id)?;
            let mut next_transform = transform;
            let next_y_rot = wrap_degrees_f32(
                transform.y_rot + wrap_degrees_f32(controller.y_rot - transform.y_rot) * 0.08_f32,
            );
            next_transform.y_rot = next_y_rot;
            next_transform.old_y_rot = next_y_rot;
            next_transform.x_rot = (controller.x_rot * 0.5_f32).clamp(-90.0, 90.0);
            next_transform.y_head_rot = next_y_rot;

            add_relative_movement(&mut next_transform, ridden_input, travel_speed);
            let position = [
                next_transform.position.x,
                next_transform.position.y,
                next_transform.position.z,
            ];
            let aabb_min = std::array::from_fn(|axis| position[axis] + relative_min[axis]);
            let aabb_max = std::array::from_fn(|axis| position[axis] + relative_max[axis]);
            let eye_y = position[1] + f64::from(self.entities.standing_eye_height(vehicle_id)?);
            let fluid_contact = crate::client::entity_fluid_contact(
                self,
                aabb_min,
                aabb_max,
                position[1],
                [position[0], eye_y, position[2]],
            );
            let stuck_multiplier = self
                .entities
                .consume_block_effect_stuck_speed_multiplier(vehicle_id);
            if let Some(multiplier) = stuck_multiplier {
                next_transform.delta_movement.x *= multiplier.x;
                next_transform.delta_movement.y *= multiplier.y;
                next_transform.delta_movement.z *= multiplier.z;
            }
            let requested = next_transform.delta_movement;
            let clipped = self.clip_locally_authoritative_entity_collision_movement(
                vehicle_id,
                VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
                position,
                [requested.x, requested.y, requested.z],
                relative_min,
                relative_max,
                1.0,
                transform.on_ground.unwrap_or(false),
                &entity_collision_targets,
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
                        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
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
                next_transform.delta_movement.y = living_vertical_collision_response(
                    self,
                    [
                        next_transform.position.x,
                        next_transform.position.y,
                        next_transform.position.z,
                    ],
                    supporting_block,
                    requested.y,
                    self.entities.is_descending(vehicle_id).unwrap_or(false),
                );
            }
            if z_collision {
                next_transform.delta_movement.z = 0.0;
            }
            let speed_factor = entity_block_speed_factor(
                self,
                [
                    next_transform.position.x,
                    next_transform.position.y,
                    next_transform.position.z,
                ],
            );
            next_transform.delta_movement.x *= speed_factor;
            next_transform.delta_movement.z *= speed_factor;
            let drag = if fluid_contact.in_water() {
                f64::from(0.8_f32)
            } else if fluid_contact.in_lava() {
                0.5
            } else {
                f64::from(0.91_f32)
            };
            next_transform.delta_movement.x *= drag;
            next_transform.delta_movement.y *= drag;
            next_transform.delta_movement.z *= drag;
            if stuck_multiplier.is_some() {
                next_transform.delta_movement = EntityVec3::default();
            }

            self.entities
                .with_transform_and_support_mut(vehicle_id, |canonical, support| {
                    *canonical = next_transform;
                    if on_ground {
                        support.main = supporting_block;
                        support.on_ground_no_blocks = supporting_block.is_none();
                    } else {
                        support.clear_physical();
                    }
                })?;
            self.apply_local_vehicle_block_effects(
                vehicle_id,
                EntityBlockEffectMovement::clipped(
                    EntityVec3 {
                        x: position[0],
                        y: position[1],
                        z: position[2],
                    },
                    next_transform.position,
                    requested,
                ),
            );
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

    /// Advances a directly controlled `AbstractHorse` through its retained
    /// jump stage, `LivingEntity.travelRidden`, and the shared air/fluid travel
    /// owner. Camel adds its refusal, dash, and pose-transition subclass work.
    pub fn advance_local_abstract_horse_vehicle_input(
        &mut self,
        input: LocalPlayerInputState,
        ticks: u32,
    ) -> Option<VehicleMoveReport> {
        let vehicle_id = self.local_player_first_passenger_root_vehicle_id()?;
        let entity_type_id = self.entities.entity_type_id(vehicle_id)?;
        if !is_vanilla_abstract_horse_type(entity_type_id)
            || self.entity_controlling_passenger(vehicle_id) != self.local_player_id
        {
            return None;
        }
        let is_camel = vanilla_entity_metadata_type_is_camel(entity_type_id);
        let controller = self.local_player.pose?;
        let slow_factor = if controller.sneaking || controller.swimming {
            self.local_player_id
                .and_then(|id| {
                    self.entities
                        .attribute_value(id, VANILLA_ATTRIBUTE_SNEAKING_SPEED_ID)
                })
                .unwrap_or(0.3)
        } else {
            1.0
        };
        let (strafe, mut forward) = if input.focused && self.local_player.camera.follows_player {
            local_player_move_vector(input, slow_factor)
        } else {
            (0.0, 0.0)
        };
        if forward <= 0.0 {
            forward *= 0.25;
        }
        let ridden_input = HappyGhastInput {
            x: f64::from(strafe * 0.5),
            y: 0.0,
            z: f64::from(forward),
        };
        let base_movement_speed_value = self
            .entities
            .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)?;
        let base_movement_speed = base_movement_speed_value as f32;
        let controller_sprinting = self.local_player_is_sprinting();
        let max_up_step =
            self.entities
                .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_STEP_HEIGHT_ID)? as f32;
        let gravity = if self.entities.no_gravity(vehicle_id).unwrap_or(false) {
            0.0
        } else {
            self.entities
                .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_GRAVITY_ID)?
        };
        let water_movement_efficiency = self
            .entities
            .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID)?
            as f32;
        let horse_accessors =
            vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id)?;
        let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
        let shared_flags = self.entities.metadata_byte(
            vehicle_id,
            entity_accessors.shared_flags_data_id,
            entity_accessors.shared_flags_default,
        )?;
        let sprinting = shared_flags & entity_accessors.sprinting_mask != 0;
        let levitation = self
            .entities
            .mob_effect_amplifier(vehicle_id, VANILLA_MOB_EFFECT_LEVITATION_ID);
        let slow_falling = self
            .entities
            .mob_effect_amplifier(vehicle_id, VANILLA_MOB_EFFECT_SLOW_FALLING_ID)
            .is_some();
        let dolphins_grace = self
            .entities
            .mob_effect_amplifier(vehicle_id, VANILLA_MOB_EFFECT_DOLPHINS_GRACE_ID)
            .is_some();
        let entity_collision_targets = (ticks > 0)
            .then(|| self.entities.vehicle_entity_collision_targets())
            .unwrap_or_default();

        for _ in 0..ticks {
            let transform = self.entities.transform(vehicle_id)?;
            let previous_supporting_block = self.entities.supporting_block(vehicle_id)?;
            let mut next_transform = transform;
            let camel_pose = is_camel
                .then(|| camel_ridden_pose_state(self, vehicle_id))
                .flatten();
            let camel_refuses_to_move = camel_pose.is_some_and(|pose| pose.refuses_to_move);
            if !camel_refuses_to_move {
                next_transform.y_rot = controller.y_rot;
                next_transform.old_y_rot = controller.y_rot;
                next_transform.x_rot = (controller.x_rot * 0.5).clamp(-90.0, 90.0);
                next_transform.y_head_rot = controller.y_rot;
            }
            let mut ridden_jump = self.entities.ridden_jump_state(vehicle_id)?;
            let mut standing = false;
            if !is_camel {
                let mut current_horse_flags = self.entities.metadata_byte(
                    vehicle_id,
                    horse_accessors.flags_data_id,
                    horse_accessors.flags_default,
                )?;
                if ridden_jump.local_stand_ticks_remaining > 0 {
                    ridden_jump.local_stand_ticks_remaining -= 1;
                    if ridden_jump.local_stand_ticks_remaining == 0 {
                        current_horse_flags &= !horse_accessors.standing_mask;
                        let _ = self.entities.with_metadata_mut(vehicle_id, |metadata| {
                            set_metadata_byte(
                                metadata,
                                horse_accessors.flags_data_id,
                                current_horse_flags,
                            );
                        });
                    }
                }
                standing = current_horse_flags & horse_accessors.standing_mask != 0;
                if !standing {
                    ridden_jump.allow_stand_sliding = false;
                    ridden_jump.local_stand_ticks_remaining = 0;
                }
            }
            let on_ground = transform.on_ground.unwrap_or(false);
            let input_this_tick = if camel_refuses_to_move
                || on_ground
                    && ridden_jump.player_jump_pending_scale == 0.0
                    && standing
                    && !ridden_jump.allow_stand_sliding
            {
                HappyGhastInput {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                }
            } else {
                ridden_input
            };
            if is_camel && camel_refuses_to_move && on_ground {
                next_transform.delta_movement.x = 0.0;
                next_transform.delta_movement.z = 0.0;
            }
            if on_ground {
                if ridden_jump.player_jump_pending_scale > 0.0 {
                    let amount = ridden_jump.player_jump_pending_scale;
                    let position = [
                        next_transform.position.x,
                        next_transform.position.y,
                        next_transform.position.z,
                    ];
                    if is_camel {
                        execute_local_camel_dash(
                            self,
                            vehicle_id,
                            entity_type_id,
                            amount,
                            base_movement_speed_value,
                            position,
                            previous_supporting_block.main,
                            &mut next_transform,
                        )?;
                    } else {
                        execute_local_horse_jump(
                            self,
                            vehicle_id,
                            amount,
                            input_this_tick,
                            position,
                            previous_supporting_block.main,
                            &mut next_transform,
                        )?;
                    }
                }
                ridden_jump.player_jump_pending_scale = 0.0;
            }
            let _ = self
                .entities
                .with_ridden_jump_state_mut(vehicle_id, |state| *state = ridden_jump);
            if is_camel
                && forward > 0.0
                && camel_pose.is_some_and(|pose| pose.is_sitting && !pose.is_in_transition)
            {
                self.stand_local_camel(vehicle_id, false)?;
            }
            let movement_speed = base_movement_speed
                + if is_camel
                    && controller_sprinting
                    && self.entities.camel_jump_cooldown_ticks(vehicle_id)? == 0
                {
                    0.1
                } else {
                    0.0
                };
            // `Camel.tickRidden` may have changed SITTING to STANDING above.
            // Vanilla's following `travel` observes the new dimensions, so do
            // not retain the pre-stand AABB across this tick.
            let bounds = self.entities.pick_bounds(vehicle_id)?;
            let relative_min = bounds.min.map(f64::from);
            let relative_max = bounds.max.map(f64::from);
            let position = [
                next_transform.position.x,
                next_transform.position.y,
                next_transform.position.z,
            ];
            let aabb_min = std::array::from_fn(|axis| position[axis] + relative_min[axis]);
            let aabb_max = std::array::from_fn(|axis| position[axis] + relative_max[axis]);
            let eye_y = position[1] + f64::from(self.entities.standing_eye_height(vehicle_id)?);
            let fluid_contact = crate::client::entity_fluid_contact(
                self,
                aabb_min,
                aabb_max,
                position[1],
                [position[0], eye_y, position[2]],
            );
            next_transform.delta_movement = super::client_movement::apply_entity_fluid_currents(
                next_transform.delta_movement,
                fluid_contact,
                crate::client::local_player_lava_current_push_per_tick(self),
            );
            let is_falling = next_transform.delta_movement.y <= 0.0;
            let effective_gravity = if is_falling && slow_falling {
                gravity.min(0.01)
            } else {
                gravity
            };
            let old_y = next_transform.position.y;
            let old_on_ground = transform.on_ground.unwrap_or(false);
            let in_water = fluid_contact.in_water();
            let in_lava = fluid_contact.in_lava();
            if in_water {
                let mut slow_down = if sprinting {
                    0.9_f32
                } else if entity_type_id == VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID {
                    0.96_f32
                } else {
                    0.8_f32
                };
                let mut speed = 0.02_f32;
                let efficiency = if old_on_ground {
                    water_movement_efficiency
                } else {
                    water_movement_efficiency * 0.5
                };
                if efficiency > 0.0 {
                    slow_down += (0.54600006_f32 - slow_down) * efficiency;
                    speed += (movement_speed - speed) * efficiency;
                }
                if dolphins_grace {
                    slow_down = 0.96;
                }
                add_relative_movement(&mut next_transform, input_this_tick, f64::from(speed));
                let outcome = move_local_living_entity_consuming_stuck(
                    self,
                    vehicle_id,
                    entity_type_id,
                    relative_min,
                    relative_max,
                    max_up_step,
                    previous_supporting_block,
                    &entity_collision_targets,
                    next_transform,
                );
                next_transform = outcome.transform;
                next_transform.delta_movement.x *= f64::from(slow_down);
                next_transform.delta_movement.y *= f64::from(0.8_f32);
                next_transform.delta_movement.z *= f64::from(slow_down);
                next_transform.delta_movement.y = fluid_falling_adjusted_velocity(
                    next_transform.delta_movement.y,
                    effective_gravity,
                    is_falling,
                    sprinting,
                );
                jump_out_of_fluid_if_free(
                    self,
                    vehicle_id,
                    entity_type_id,
                    relative_min,
                    relative_max,
                    &entity_collision_targets,
                    old_y,
                    outcome.horizontal_collision,
                    &mut next_transform,
                );
                if vanilla_entity_type_can_float_while_ridden(entity_type_id)
                    && fluid_contact.water_height > 0.4
                {
                    next_transform.delta_movement.y += f64::from(0.04_f32);
                }
                store_living_move(
                    &mut self.entities,
                    vehicle_id,
                    next_transform,
                    outcome.supporting_block,
                )?;
            } else if in_lava {
                add_relative_movement(&mut next_transform, input_this_tick, f64::from(0.02_f32));
                let outcome = move_local_living_entity_consuming_stuck(
                    self,
                    vehicle_id,
                    entity_type_id,
                    relative_min,
                    relative_max,
                    max_up_step,
                    previous_supporting_block,
                    &entity_collision_targets,
                    next_transform,
                );
                next_transform = outcome.transform;
                if fluid_contact.lava_height <= 0.4 {
                    next_transform.delta_movement.x *= 0.5;
                    next_transform.delta_movement.y *= f64::from(0.8_f32);
                    next_transform.delta_movement.z *= 0.5;
                    next_transform.delta_movement.y = fluid_falling_adjusted_velocity(
                        next_transform.delta_movement.y,
                        effective_gravity,
                        is_falling,
                        sprinting,
                    );
                } else {
                    next_transform.delta_movement.x *= 0.5;
                    next_transform.delta_movement.y *= 0.5;
                    next_transform.delta_movement.z *= 0.5;
                }
                if effective_gravity != 0.0 {
                    next_transform.delta_movement.y -= effective_gravity / 4.0;
                }
                jump_out_of_fluid_if_free(
                    self,
                    vehicle_id,
                    entity_type_id,
                    relative_min,
                    relative_max,
                    &entity_collision_targets,
                    old_y,
                    outcome.horizontal_collision,
                    &mut next_transform,
                );
                store_living_move(
                    &mut self.entities,
                    vehicle_id,
                    next_transform,
                    outcome.supporting_block,
                )?;
            } else {
                let block_friction = if old_on_ground {
                    entity_movement_block_friction(self, position, previous_supporting_block.main)
                } else {
                    1.0
                };
                let friction = block_friction * 0.91_f32;
                let speed = if old_on_ground {
                    movement_speed
                        * (0.21600002_f32 / (block_friction * block_friction * block_friction))
                } else {
                    movement_speed * 0.1_f32
                };
                add_relative_movement(&mut next_transform, input_this_tick, f64::from(speed));
                let outcome = move_local_living_entity_consuming_stuck(
                    self,
                    vehicle_id,
                    entity_type_id,
                    relative_min,
                    relative_max,
                    max_up_step,
                    previous_supporting_block,
                    &entity_collision_targets,
                    next_transform,
                );
                next_transform = outcome.transform;
                next_transform.delta_movement.y = if let Some(amplifier) = levitation {
                    next_transform.delta_movement.y
                        + (0.05 * f64::from(amplifier + 1) - next_transform.delta_movement.y) * 0.2
                } else {
                    next_transform.delta_movement.y - effective_gravity
                };
                next_transform.delta_movement.x *= f64::from(friction);
                next_transform.delta_movement.y *= f64::from(0.98_f32);
                next_transform.delta_movement.z *= f64::from(friction);
                store_living_move(
                    &mut self.entities,
                    vehicle_id,
                    next_transform,
                    outcome.supporting_block,
                )?;
            }
            if is_camel {
                let current = self.entities.transform(vehicle_id)?;
                let camel_is_passenger = self
                    .entities
                    .mount(vehicle_id)
                    .is_some_and(|mount| mount.vehicle_id.is_some());
                let (dash_data_id, dash_default) =
                    vanilla_entity_metadata_camel_dash_accessor_for_entity_type(entity_type_id)?;
                let is_dashing = self.entities.metadata_bool_for_entity(
                    vehicle_id,
                    dash_data_id,
                    dash_default,
                )?;
                if is_dashing
                    && self.entities.camel_jump_cooldown_ticks(vehicle_id)? <= 48
                    && (current.on_ground.unwrap_or(false)
                        || fluid_contact.in_water()
                        || fluid_contact.in_lava()
                        || camel_is_passenger)
                {
                    let _ = self.entities.with_metadata_mut(vehicle_id, |metadata| {
                        set_metadata_bool(metadata, dash_data_id, false);
                    });
                    self.entities.stop_local_camel_dash(vehicle_id)?;
                }
                if camel_pose.is_some_and(|pose| pose.is_sitting) && fluid_contact.in_water() {
                    self.stand_local_camel(vehicle_id, true)?;
                }
            }
            self.apply_local_vehicle_block_effects_from_recorder(vehicle_id);
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

    /// Advances a directly controlled `AbstractNautilus` through its exact
    /// `tickRidden` dash stage, overridden water travel, shared lava/air travel,
    /// and post-movement subclass cooldown/sound/bubble tick.
    pub fn advance_local_nautilus_vehicle_input(
        &mut self,
        input: LocalPlayerInputState,
        ticks: u32,
    ) -> Option<VehicleMoveReport> {
        let vehicle_id = self.local_player_first_passenger_root_vehicle_id()?;
        let entity_type_id = self.entities.entity_type_id(vehicle_id)?;
        if !is_vanilla_abstract_nautilus_type(entity_type_id)
            || self.entity_controlling_passenger(vehicle_id) != self.local_player_id
        {
            return None;
        }
        let controller = self.local_player.pose?;
        let slow_factor = if controller.sneaking || controller.swimming {
            self.local_player_id
                .and_then(|id| {
                    self.entities
                        .attribute_value(id, VANILLA_ATTRIBUTE_SNEAKING_SPEED_ID)
                })
                .unwrap_or(0.3)
        } else {
            1.0
        };
        let (strafe, forward_axis) = if input.focused && self.local_player.camera.follows_player {
            local_player_move_vector(input, slow_factor)
        } else {
            (0.0, 0.0)
        };
        let mut forward = 0.0_f32;
        let mut up = 0.0_f32;
        if forward_axis != 0.0 {
            let pitch = controller.x_rot * std::f32::consts::PI / 180.0;
            forward = super::client_movement::vanilla_mth_cos(f64::from(pitch));
            up = -super::client_movement::vanilla_mth_sin(f64::from(pitch));
            if forward_axis < 0.0 {
                forward *= -0.5;
                up *= -0.5;
            }
        }
        let ridden_input = HappyGhastInput {
            x: f64::from(strafe),
            y: f64::from(up),
            z: f64::from(forward),
        };
        let movement_speed = self
            .entities
            .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)?;
        let max_up_step =
            self.entities
                .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_STEP_HEIGHT_ID)? as f32;
        let gravity = if self.entities.no_gravity(vehicle_id).unwrap_or(false) {
            0.0
        } else {
            self.entities
                .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_GRAVITY_ID)?
        };
        let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
        let shared_flags = self.entities.metadata_byte(
            vehicle_id,
            entity_accessors.shared_flags_data_id,
            entity_accessors.shared_flags_default,
        )?;
        let sprinting = shared_flags & entity_accessors.sprinting_mask != 0;
        let levitation = self
            .entities
            .mob_effect_amplifier(vehicle_id, VANILLA_MOB_EFFECT_LEVITATION_ID);
        let slow_falling = self
            .entities
            .mob_effect_amplifier(vehicle_id, VANILLA_MOB_EFFECT_SLOW_FALLING_ID)
            .is_some();
        let entity_collision_targets = (ticks > 0)
            .then(|| self.entities.vehicle_entity_collision_targets())
            .unwrap_or_default();

        for _ in 0..ticks {
            let transform = self.entities.transform(vehicle_id)?;
            let previous_supporting_block = self.entities.supporting_block(vehicle_id)?;
            let mut next_transform = transform;
            let y_rot =
                transform.y_rot + wrap_degrees_f32(controller.y_rot - transform.y_rot) * 0.5_f32;
            next_transform.y_rot = y_rot;
            next_transform.old_y_rot = y_rot;
            next_transform.x_rot = controller.x_rot * 0.5;
            next_transform.y_head_rot = y_rot;

            let bounds = self.entities.pick_bounds(vehicle_id)?;
            let relative_min = bounds.min.map(f64::from);
            let relative_max = bounds.max.map(f64::from);
            let position = [
                next_transform.position.x,
                next_transform.position.y,
                next_transform.position.z,
            ];
            let aabb_min = std::array::from_fn(|axis| position[axis] + relative_min[axis]);
            let aabb_max = std::array::from_fn(|axis| position[axis] + relative_max[axis]);
            let eye_y = position[1] + f64::from(self.entities.standing_eye_height(vehicle_id)?);
            let fluid_contact = crate::client::entity_fluid_contact(
                self,
                aabb_min,
                aabb_max,
                position[1],
                [position[0], eye_y, position[2]],
            );

            let mut ridden_jump = self.entities.ridden_jump_state(vehicle_id)?;
            if ridden_jump.player_jump_pending_scale > 0.0 {
                let amount = ridden_jump.player_jump_pending_scale;
                let look = super::calculate_view_vector(controller.x_rot, controller.y_rot);
                let momentum = f64::from(if fluid_contact.in_water() {
                    1.2_f32
                } else {
                    0.5_f32
                }) * f64::from(amount)
                    * movement_speed
                    * entity_block_speed_factor(self, position);
                next_transform.delta_movement.x += look.x * momentum;
                next_transform.delta_movement.y += look.y * momentum;
                next_transform.delta_movement.z += look.z * momentum;
                let _ = self.entities.with_metadata_mut(vehicle_id, |metadata| {
                    set_metadata_bool(metadata, VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID, true);
                });
                self.entities.start_local_nautilus_dash(vehicle_id)?;
            }
            ridden_jump.player_jump_pending_scale = 0.0;
            let _ = self
                .entities
                .with_ridden_jump_state_mut(vehicle_id, |state| *state = ridden_jump);

            let is_falling = next_transform.delta_movement.y <= 0.0;
            let effective_gravity = if is_falling && slow_falling {
                gravity.min(0.01)
            } else {
                gravity
            };
            let old_y = next_transform.position.y;
            let old_on_ground = transform.on_ground.unwrap_or(false);
            if fluid_contact.in_water() {
                add_relative_movement(
                    &mut next_transform,
                    ridden_input,
                    f64::from(0.0325_f32) * movement_speed,
                );
                let outcome = move_local_living_entity_consuming_stuck(
                    self,
                    vehicle_id,
                    entity_type_id,
                    relative_min,
                    relative_max,
                    max_up_step,
                    previous_supporting_block,
                    &entity_collision_targets,
                    next_transform,
                );
                next_transform = outcome.transform;
                next_transform.delta_movement.x *= f64::from(0.9_f32);
                next_transform.delta_movement.y *= f64::from(0.9_f32);
                next_transform.delta_movement.z *= f64::from(0.9_f32);
                store_living_move(
                    &mut self.entities,
                    vehicle_id,
                    next_transform,
                    outcome.supporting_block,
                )?;
            } else if fluid_contact.in_lava() {
                add_relative_movement(&mut next_transform, ridden_input, f64::from(0.02_f32));
                let outcome = move_local_living_entity_consuming_stuck(
                    self,
                    vehicle_id,
                    entity_type_id,
                    relative_min,
                    relative_max,
                    max_up_step,
                    previous_supporting_block,
                    &entity_collision_targets,
                    next_transform,
                );
                next_transform = outcome.transform;
                if fluid_contact.lava_height <= 0.4 {
                    next_transform.delta_movement.x *= 0.5;
                    next_transform.delta_movement.y *= f64::from(0.8_f32);
                    next_transform.delta_movement.z *= 0.5;
                    next_transform.delta_movement.y = fluid_falling_adjusted_velocity(
                        next_transform.delta_movement.y,
                        effective_gravity,
                        is_falling,
                        sprinting,
                    );
                } else {
                    next_transform.delta_movement.x *= 0.5;
                    next_transform.delta_movement.y *= 0.5;
                    next_transform.delta_movement.z *= 0.5;
                }
                if effective_gravity != 0.0 {
                    next_transform.delta_movement.y -= effective_gravity / 4.0;
                }
                jump_out_of_fluid_if_free(
                    self,
                    vehicle_id,
                    entity_type_id,
                    relative_min,
                    relative_max,
                    &entity_collision_targets,
                    old_y,
                    outcome.horizontal_collision,
                    &mut next_transform,
                );
                store_living_move(
                    &mut self.entities,
                    vehicle_id,
                    next_transform,
                    outcome.supporting_block,
                )?;
            } else {
                let block_friction = if old_on_ground {
                    entity_movement_block_friction(self, position, previous_supporting_block.main)
                } else {
                    1.0
                };
                let friction = block_friction * 0.91_f32;
                let ridden_speed = f64::from(0.02_f32) * movement_speed;
                let speed = if old_on_ground {
                    ridden_speed
                        * f64::from(
                            0.21600002_f32 / (block_friction * block_friction * block_friction),
                        )
                } else {
                    ridden_speed * f64::from(0.1_f32)
                };
                add_relative_movement(&mut next_transform, ridden_input, speed);
                let outcome = move_local_living_entity_consuming_stuck(
                    self,
                    vehicle_id,
                    entity_type_id,
                    relative_min,
                    relative_max,
                    max_up_step,
                    previous_supporting_block,
                    &entity_collision_targets,
                    next_transform,
                );
                next_transform = outcome.transform;
                next_transform.delta_movement.y = if let Some(amplifier) = levitation {
                    next_transform.delta_movement.y
                        + (0.05 * f64::from(amplifier + 1) - next_transform.delta_movement.y) * 0.2
                } else {
                    next_transform.delta_movement.y - effective_gravity
                };
                next_transform.delta_movement.x *= f64::from(friction);
                next_transform.delta_movement.y *= f64::from(0.98_f32);
                next_transform.delta_movement.z *= f64::from(friction);
                store_living_move(
                    &mut self.entities,
                    vehicle_id,
                    next_transform,
                    outcome.supporting_block,
                )?;
            }
            self.apply_local_vehicle_block_effects_from_recorder(vehicle_id);
            self.entities.advance_deferred_nautilus_subclass_tick(
                vehicle_id,
                fluid_contact.in_water(),
                fluid_contact.eye_in_water,
            )?;
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

    /// Advances a directly controlled Pig or Strider through
    /// `ItemBasedSteering.tickBoost`, its fixed forward ridden input, and the
    /// shared `LivingEntity.travel` path. The synchronized metadata owns the
    /// total boost duration; the canonical entity animation owner retains only
    /// the client-local elapsed cursor.
    pub fn advance_local_item_based_steering_vehicle_input(
        &mut self,
        ticks: u32,
    ) -> Option<VehicleMoveReport> {
        let vehicle_id = self.local_player_first_passenger_root_vehicle_id()?;
        let entity_type_id = self.entities.entity_type_id(vehicle_id)?;
        if vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
            entity_type_id,
        )
        .is_none()
            || self.entity_controlling_passenger(vehicle_id) != self.local_player_id
        {
            return None;
        }
        let controller = self.local_player.pose?;
        let max_up_step =
            self.entities
                .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_STEP_HEIGHT_ID)? as f32;
        let gravity = if self.entities.no_gravity(vehicle_id).unwrap_or(false) {
            0.0
        } else {
            self.entities
                .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_GRAVITY_ID)?
        };
        let water_movement_efficiency = self
            .entities
            .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID)?
            as f32;
        let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
        let shared_flags = self.entities.metadata_byte(
            vehicle_id,
            entity_accessors.shared_flags_data_id,
            entity_accessors.shared_flags_default,
        )?;
        let sprinting = shared_flags & entity_accessors.sprinting_mask != 0;
        let levitation = self
            .entities
            .mob_effect_amplifier(vehicle_id, VANILLA_MOB_EFFECT_LEVITATION_ID);
        let slow_falling = self
            .entities
            .mob_effect_amplifier(vehicle_id, VANILLA_MOB_EFFECT_SLOW_FALLING_ID)
            .is_some();
        let dolphins_grace = self
            .entities
            .mob_effect_amplifier(vehicle_id, VANILLA_MOB_EFFECT_DOLPHINS_GRACE_ID)
            .is_some();
        let strider_suffocating =
            vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type(entity_type_id);
        let entity_collision_targets = (ticks > 0)
            .then(|| self.entities.vehicle_entity_collision_targets())
            .unwrap_or_default();
        let ridden_input = HappyGhastInput {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };

        for _ in 0..ticks {
            let transform = self.entities.transform(vehicle_id)?;
            let previous_supporting_block = self.entities.supporting_block(vehicle_id)?;
            let mut next_transform = transform;
            next_transform.y_rot = controller.y_rot;
            next_transform.old_y_rot = controller.y_rot;
            next_transform.x_rot = controller.x_rot * 0.5;
            next_transform.y_head_rot = controller.y_rot;

            let bounds = self.entities.pick_bounds(vehicle_id)?;
            let relative_min = bounds.min.map(f64::from);
            let relative_max = bounds.max.map(f64::from);
            let position = [
                next_transform.position.x,
                next_transform.position.y,
                next_transform.position.z,
            ];
            let aabb_min = std::array::from_fn(|axis| position[axis] + relative_min[axis]);
            let aabb_max = std::array::from_fn(|axis| position[axis] + relative_max[axis]);
            let eye_y = position[1] + f64::from(self.entities.standing_eye_height(vehicle_id)?);
            let fluid_contact = crate::client::entity_fluid_contact(
                self,
                aabb_min,
                aabb_max,
                position[1],
                [position[0], eye_y, position[2]],
            );
            if strider_suffocating.is_some() {
                let mob_accessors =
                    vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id)?;
                let mob_flags = self.entities.metadata_byte(
                    vehicle_id,
                    mob_accessors.flags_data_id,
                    mob_accessors.flags_default,
                )?;
                if mob_flags & mob_accessors.no_ai_mask == 0 {
                    self.entities
                        .set_strider_suffocating(vehicle_id, !fluid_contact.in_lava())?;
                }
            }
            let base_movement_speed = self
                .entities
                .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)?;
            let boost_factor = self.entities.advance_item_based_steering_tick(vehicle_id)?;
            let steering_modifier = if let Some((data_id, default)) = strider_suffocating {
                if self
                    .entities
                    .metadata_bool_for_entity(vehicle_id, data_id, default)?
                {
                    0.35_f32
                } else {
                    0.55_f32
                }
            } else {
                0.225_f32
            };
            let movement_speed = (base_movement_speed
                * f64::from(steering_modifier)
                * f64::from(boost_factor)) as f32 as f64;
            next_transform.delta_movement = super::client_movement::apply_entity_fluid_currents(
                next_transform.delta_movement,
                fluid_contact,
                crate::client::local_player_lava_current_push_per_tick(self),
            );
            let is_falling = next_transform.delta_movement.y <= 0.0;
            let effective_gravity = if is_falling && slow_falling {
                gravity.min(0.01)
            } else {
                gravity
            };
            let old_y = next_transform.position.y;
            let old_on_ground = transform.on_ground.unwrap_or(false);
            if fluid_contact.in_water() {
                let mut slow_down = if sprinting { 0.9_f32 } else { 0.8_f32 };
                let mut speed = 0.02_f32;
                let efficiency = if old_on_ground {
                    water_movement_efficiency
                } else {
                    water_movement_efficiency * 0.5
                };
                if efficiency > 0.0 {
                    slow_down += (0.54600006_f32 - slow_down) * efficiency;
                    speed += (movement_speed as f32 - speed) * efficiency;
                }
                if dolphins_grace {
                    slow_down = 0.96;
                }
                add_relative_movement(&mut next_transform, ridden_input, f64::from(speed));
                let outcome = move_local_living_entity_consuming_stuck(
                    self,
                    vehicle_id,
                    entity_type_id,
                    relative_min,
                    relative_max,
                    max_up_step,
                    previous_supporting_block,
                    &entity_collision_targets,
                    next_transform,
                );
                next_transform = outcome.transform;
                next_transform.delta_movement.x *= f64::from(slow_down);
                next_transform.delta_movement.y *= f64::from(0.8_f32);
                next_transform.delta_movement.z *= f64::from(slow_down);
                next_transform.delta_movement.y = fluid_falling_adjusted_velocity(
                    next_transform.delta_movement.y,
                    effective_gravity,
                    is_falling,
                    sprinting,
                );
                jump_out_of_fluid_if_free(
                    self,
                    vehicle_id,
                    entity_type_id,
                    relative_min,
                    relative_max,
                    &entity_collision_targets,
                    old_y,
                    outcome.horizontal_collision,
                    &mut next_transform,
                );
                store_living_move(
                    &mut self.entities,
                    vehicle_id,
                    next_transform,
                    outcome.supporting_block,
                )?;
            } else if fluid_contact.in_lava() {
                add_relative_movement(&mut next_transform, ridden_input, f64::from(0.02_f32));
                let outcome = move_local_living_entity_consuming_stuck(
                    self,
                    vehicle_id,
                    entity_type_id,
                    relative_min,
                    relative_max,
                    max_up_step,
                    previous_supporting_block,
                    &entity_collision_targets,
                    next_transform,
                );
                next_transform = outcome.transform;
                if fluid_contact.lava_height <= 0.4 {
                    next_transform.delta_movement.x *= 0.5;
                    next_transform.delta_movement.y *= f64::from(0.8_f32);
                    next_transform.delta_movement.z *= 0.5;
                    next_transform.delta_movement.y = fluid_falling_adjusted_velocity(
                        next_transform.delta_movement.y,
                        effective_gravity,
                        is_falling,
                        sprinting,
                    );
                } else {
                    next_transform.delta_movement.x *= 0.5;
                    next_transform.delta_movement.y *= 0.5;
                    next_transform.delta_movement.z *= 0.5;
                }
                if effective_gravity != 0.0 {
                    next_transform.delta_movement.y -= effective_gravity / 4.0;
                }
                jump_out_of_fluid_if_free(
                    self,
                    vehicle_id,
                    entity_type_id,
                    relative_min,
                    relative_max,
                    &entity_collision_targets,
                    old_y,
                    outcome.horizontal_collision,
                    &mut next_transform,
                );
                store_living_move(
                    &mut self.entities,
                    vehicle_id,
                    next_transform,
                    outcome.supporting_block,
                )?;
            } else {
                let block_friction = if old_on_ground {
                    entity_movement_block_friction(self, position, previous_supporting_block.main)
                } else {
                    1.0
                };
                let friction = block_friction * 0.91_f32;
                let speed = if old_on_ground {
                    movement_speed
                        * f64::from(
                            0.21600002_f32 / (block_friction * block_friction * block_friction),
                        )
                } else {
                    movement_speed * f64::from(0.1_f32)
                };
                add_relative_movement(&mut next_transform, ridden_input, speed);
                let outcome = move_local_living_entity_consuming_stuck(
                    self,
                    vehicle_id,
                    entity_type_id,
                    relative_min,
                    relative_max,
                    max_up_step,
                    previous_supporting_block,
                    &entity_collision_targets,
                    next_transform,
                );
                next_transform = outcome.transform;
                next_transform.delta_movement.y = if let Some(amplifier) = levitation {
                    next_transform.delta_movement.y
                        + (0.05 * f64::from(amplifier + 1) - next_transform.delta_movement.y) * 0.2
                } else {
                    next_transform.delta_movement.y - effective_gravity
                };
                next_transform.delta_movement.x *= f64::from(friction);
                next_transform.delta_movement.y *= f64::from(0.98_f32);
                next_transform.delta_movement.z *= f64::from(friction);
                store_living_move(
                    &mut self.entities,
                    vehicle_id,
                    next_transform,
                    outcome.supporting_block,
                )?;
            }

            self.apply_local_vehicle_block_effects_from_recorder(vehicle_id);
            if strider_suffocating.is_some() && fluid_contact.in_lava() {
                self.advance_local_strider_float_after_tick(
                    vehicle_id,
                    relative_min,
                    fluid_contact,
                )?;
            }
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

    fn advance_local_strider_float_after_tick(
        &mut self,
        vehicle_id: i32,
        relative_min: [f64; 3],
        fluid_contact: crate::client::EntityFluidContactState,
    ) -> Option<()> {
        debug_assert!(fluid_contact.in_lava());
        let mut transform = self.entities.transform(vehicle_id)?;
        let block_pos = crate::BlockPos {
            x: transform.position.x.floor() as i32,
            y: transform.position.y.floor() as i32,
            z: transform.position.z.floor() as i32,
        };
        let entity_bottom = transform.position.y + relative_min[1];
        let above_lava = block_pos.y.checked_add(1).is_some_and(|y| {
            self.fluid_state_and_height_at(crate::BlockPos {
                x: block_pos.x,
                y,
                z: block_pos.z,
            })
            .is_some_and(|(fluid, _)| fluid.kind == crate::TerrainFluidKind::Lava)
        });
        if entity_bottom > f64::from(block_pos.y) + 0.5 - f64::from(1.0e-5_f32) && !above_lava {
            transform.on_ground = Some(true);
        } else {
            transform.delta_movement.x *= 0.5;
            transform.delta_movement.y = transform.delta_movement.y * 0.5 + 0.05;
            transform.delta_movement.z *= 0.5;
        }
        self.entities
            .with_transform_mut(vehicle_id, |canonical| *canonical = transform)
    }

    /// `LivingEntity.aiStep` runs `applyEffectsFromBlocks` after
    /// `travelRidden`; the ridden travel owners call this once their recorded
    /// move is complete.
    fn apply_local_vehicle_block_effects_from_recorder(&mut self, vehicle_id: i32) {
        let Some(movement) = self.entities.take_local_vehicle_move(vehicle_id) else {
            return;
        };
        self.apply_local_vehicle_block_effects(vehicle_id, movement);
    }

    /// Locally authoritative `applyEffectsFromBlocks` for the directly
    /// controlled vehicle, borrowing the immutable terrain owner beside the
    /// mutable entity store. Pass count and family gating come from the
    /// generated block-effect tick policy.
    fn apply_local_vehicle_block_effects(
        &mut self,
        vehicle_id: i32,
        movement: EntityBlockEffectMovement,
    ) {
        if self.chunks.is_empty() {
            return;
        }
        let mut block_effect_world = crate::client::EntityBlockEffectWorld::new(
            self.dimension,
            &self.registries,
            &self.chunks,
            self.entity_block_effect_weather(),
        );
        let empty_collisions = std::collections::HashMap::new();
        let _ = self.entities.apply_local_vehicle_block_effect_tick(
            vehicle_id,
            movement,
            &empty_collisions,
            &mut block_effect_world,
        );
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

    fn stand_local_camel(&mut self, vehicle_id: i32, instantly: bool) -> Option<()> {
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
        if !instantly && !self.entities.is_silent(vehicle_id).unwrap_or(false) {
            let position = self.entities.transform(vehicle_id)?.position;
            let pitch = self.entities.next_camel_voice_pitch(vehicle_id)?;
            self.entities.pending_entity_client_movement_effects.push(
                super::EntityClientMovementEffectState::Sound(
                    super::EntityClientMovementSoundState {
                        entity_id: vehicle_id,
                        kind: super::EntityClientMovementSoundKind::CamelStand {
                            husk: entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
                            pitch,
                        },
                        position,
                    },
                ),
            );
        }
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

        if controller_owner == "net.minecraft.world.entity.vehicle.boat.AbstractBoat" {
            return self
                .entity_id_is_living(first_passenger_id)
                .then_some(first_passenger_id);
        }

        let has_saddle = self
            .entities
            .saddle_slot_contains_saddle_item(entity_id, &self.items.default_item_equipment_slots);
        if matches!(
            controller_owner,
            "net.minecraft.world.entity.animal.equine.AbstractHorse"
                | "net.minecraft.world.entity.animal.nautilus.AbstractNautilus"
        ) {
            if has_saddle && self.entity_id_is_player(first_passenger_id) {
                return Some(first_passenger_id);
            }
        } else if controller_owner == "net.minecraft.world.entity.animal.pig.Pig" {
            if has_saddle
                && self.entity_id_is_player(first_passenger_id)
                && self.player_holds_item(first_passenger_id, VANILLA_ITEM_CARROT_ON_A_STICK_ID)
            {
                return Some(first_passenger_id);
            }
        } else if controller_owner == "net.minecraft.world.entity.monster.Strider" {
            if has_saddle
                && self.entity_id_is_player(first_passenger_id)
                && self
                    .player_holds_item(first_passenger_id, VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID)
            {
                return Some(first_passenger_id);
            }
        } else if controller_owner == "net.minecraft.world.entity.animal.happyghast.HappyGhast" {
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
        } else if controller_owner != "net.minecraft.world.entity.Mob" {
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

    pub(crate) fn local_player_holds_item(&self, item_id: i32) -> bool {
        [InteractionHand::MainHand, InteractionHand::OffHand]
            .into_iter()
            .any(|hand| {
                self.local_item_in_hand(hand).and_then(|item| item.item_id) == Some(item_id)
            })
    }

    fn player_holds_item(&self, player_id: i32, item_id: i32) -> bool {
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

#[derive(Debug, Clone, Copy)]
struct HappyGhastInput {
    x: f64,
    y: f64,
    z: f64,
}

impl HappyGhastInput {
    fn from_local_input(
        input: LocalPlayerInputState,
        controller_x_rot: f32,
        flying_speed: f64,
        controller_is_camera: bool,
        slow_factor: f64,
    ) -> Self {
        let accepts_input = input.focused && controller_is_camera;
        let (strafe, forward_axis) = if accepts_input {
            local_player_move_vector(input, slow_factor)
        } else {
            (0.0, 0.0)
        };
        let mut forward = 0.0_f32;
        let mut up = 0.0_f32;
        if forward_axis != 0.0 {
            let pitch = controller_x_rot * (std::f64::consts::PI / 180.0) as f32;
            forward = super::client_movement::vanilla_mth_cos(f64::from(pitch));
            up = -super::client_movement::vanilla_mth_sin(f64::from(pitch));
            if forward_axis < 0.0 {
                forward *= -0.5;
                up *= -0.5;
            }
        }
        if accepts_input && input.jump {
            up += 0.5;
        }
        let scale = f64::from(3.9_f32) * flying_speed;
        Self {
            x: f64::from(strafe) * scale,
            y: f64::from(up) * scale,
            z: f64::from(forward) * scale,
        }
    }
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

fn bool_axis(positive: bool, negative: bool) -> f32 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

fn local_player_move_vector(input: LocalPlayerInputState, slow_factor: f64) -> (f32, f32) {
    let mut x = bool_axis(input.left, input.right);
    let mut y = bool_axis(input.forward, input.backward);
    let raw_length = (x * x + y * y).sqrt();
    if raw_length < 1.0e-4_f32 {
        return (0.0, 0.0);
    }
    x /= raw_length;
    y /= raw_length;
    let scale = 0.98_f32 * slow_factor as f32;
    x *= scale;
    y *= scale;

    let length = (x * x + y * y).sqrt();
    if length <= 0.0 {
        return (x, y);
    }
    let direction_x = x / length;
    let direction_y = y / length;
    let abs_x = direction_x.abs();
    let abs_y = direction_y.abs();
    let tan = if abs_y > abs_x {
        abs_x / abs_y
    } else {
        abs_y / abs_x
    };
    let distance_to_square = (1.0_f32 + tan * tan).sqrt();
    let modified_length = (length * distance_to_square).min(1.0);
    (direction_x * modified_length, direction_y * modified_length)
}

fn add_relative_movement(
    transform: &mut super::components::EntityTransform,
    input: HappyGhastInput,
    speed: f64,
) {
    let length_squared = input.x * input.x + input.y * input.y + input.z * input.z;
    if length_squared < 1.0e-7 {
        return;
    }
    let normalizer = if length_squared > 1.0 {
        length_squared.sqrt().recip()
    } else {
        1.0
    };
    let movement_x = input.x * normalizer * speed;
    let movement_y = input.y * normalizer * speed;
    let movement_z = input.z * normalizer * speed;
    let yaw = transform.y_rot * (std::f64::consts::PI / 180.0) as f32;
    let sin = super::client_movement::vanilla_mth_sin(f64::from(yaw));
    let cos = super::client_movement::vanilla_mth_cos(f64::from(yaw));
    transform.delta_movement.x += movement_x * f64::from(cos) - movement_z * f64::from(sin);
    transform.delta_movement.y += movement_y;
    transform.delta_movement.z += movement_z * f64::from(cos) + movement_x * f64::from(sin);
}

#[derive(Debug, Clone, Copy)]
struct LocalLivingMoveOutcome {
    transform: super::components::EntityTransform,
    supporting_block: Option<crate::BlockPos>,
    horizontal_collision: bool,
    /// The `Entity.Movement` this move records for `applyEffectsFromBlocks`:
    /// pre-move origin, collision-clipped destination, and the requested
    /// vector that drives Vanilla's axis-dependent traversal order.
    movement: EntityBlockEffectMovement,
}

#[allow(clippy::too_many_arguments)]
fn move_local_living_entity(
    world: &WorldStore,
    entity_id: i32,
    entity_type_id: i32,
    relative_min: [f64; 3],
    relative_max: [f64; 3],
    max_up_step: f32,
    previous_supporting_block: super::components::EntitySupportingBlock,
    entity_collision_targets: &[super::EntityPickTargetState],
    mut transform: super::components::EntityTransform,
) -> LocalLivingMoveOutcome {
    let position = [
        transform.position.x,
        transform.position.y,
        transform.position.z,
    ];
    let requested = transform.delta_movement;
    let clipped = world.clip_locally_authoritative_entity_collision_movement(
        entity_id,
        entity_type_id,
        position,
        [requested.x, requested.y, requested.z],
        relative_min,
        relative_max,
        max_up_step,
        transform.on_ground.unwrap_or(false),
        entity_collision_targets,
    );
    transform.position.x += clipped[0];
    transform.position.y += clipped[1];
    transform.position.z += clipped[2];
    let x_collision = (requested.x - clipped[0]).abs() >= f64::from(1.0e-5_f32);
    let y_collision = requested.y != clipped[1];
    let z_collision = (requested.z - clipped[2]).abs() >= f64::from(1.0e-5_f32);
    let on_ground = y_collision && requested.y < 0.0;
    transform.on_ground = Some(on_ground);
    let supporting_block = on_ground
        .then(|| {
            world.entity_supporting_block_after_collision(
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
        transform.delta_movement.x = 0.0;
    }
    if y_collision {
        transform.delta_movement.y = living_vertical_collision_response(
            world,
            [
                transform.position.x,
                transform.position.y,
                transform.position.z,
            ],
            supporting_block,
            requested.y,
            world.entities.is_descending(entity_id).unwrap_or(false),
        );
    }
    if z_collision {
        transform.delta_movement.z = 0.0;
    }
    let speed_factor = entity_block_speed_factor(
        world,
        [
            transform.position.x,
            transform.position.y,
            transform.position.z,
        ],
    );
    transform.delta_movement.x *= speed_factor;
    transform.delta_movement.z *= speed_factor;
    LocalLivingMoveOutcome {
        movement: EntityBlockEffectMovement::clipped(
            EntityVec3 {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            transform.position,
            requested,
        ),
        transform,
        supporting_block,
        horizontal_collision: x_collision || z_collision,
    }
}

/// `Entity.move` for the directly controlled Living vehicle: consume the
/// pending stuck-speed multiplier into the requested movement, run the shared
/// clipped move, and zero the post-move delta exactly as Vanilla does when a
/// multiplier was pending.
#[allow(clippy::too_many_arguments)]
fn move_local_living_entity_consuming_stuck(
    world: &mut WorldStore,
    entity_id: i32,
    entity_type_id: i32,
    relative_min: [f64; 3],
    relative_max: [f64; 3],
    max_up_step: f32,
    previous_supporting_block: super::components::EntitySupportingBlock,
    entity_collision_targets: &[super::EntityPickTargetState],
    mut transform: super::components::EntityTransform,
) -> LocalLivingMoveOutcome {
    let stuck_multiplier = world
        .entities
        .consume_block_effect_stuck_speed_multiplier(entity_id);
    if let Some(multiplier) = stuck_multiplier {
        transform.delta_movement.x *= multiplier.x;
        transform.delta_movement.y *= multiplier.y;
        transform.delta_movement.z *= multiplier.z;
    }
    let mut outcome = move_local_living_entity(
        world,
        entity_id,
        entity_type_id,
        relative_min,
        relative_max,
        max_up_step,
        previous_supporting_block,
        entity_collision_targets,
        transform,
    );
    if stuck_multiplier.is_some() {
        outcome.transform.delta_movement = EntityVec3::default();
    }
    world
        .entities
        .record_local_vehicle_move(entity_id, outcome.movement);
    outcome
}

fn store_living_move(
    entities: &mut super::EntityStore,
    entity_id: i32,
    transform: super::components::EntityTransform,
    supporting_block: Option<crate::BlockPos>,
) -> Option<()> {
    entities.with_transform_and_support_mut(entity_id, |canonical, support| {
        *canonical = transform;
        if transform.on_ground.unwrap_or(false) {
            support.main = supporting_block;
            support.on_ground_no_blocks = supporting_block.is_none();
        } else {
            support.clear_physical();
        }
    })
}

fn fluid_falling_adjusted_velocity(
    movement_y: f64,
    gravity: f64,
    is_falling: bool,
    sprinting: bool,
) -> f64 {
    if gravity == 0.0 || sprinting {
        return movement_y;
    }
    let gravity_step = gravity / 16.0;
    if is_falling
        && (movement_y - 0.005).abs() >= 0.003
        && (movement_y - gravity_step).abs() < 0.003
    {
        -0.003
    } else {
        movement_y - gravity_step
    }
}

#[allow(clippy::too_many_arguments)]
fn jump_out_of_fluid_if_free(
    world: &WorldStore,
    entity_id: i32,
    entity_type_id: i32,
    relative_min: [f64; 3],
    relative_max: [f64; 3],
    entity_collision_targets: &[super::EntityPickTargetState],
    old_y: f64,
    horizontal_collision: bool,
    transform: &mut super::components::EntityTransform,
) {
    if !horizontal_collision {
        return;
    }
    let movement = transform.delta_movement;
    let test_position = [
        transform.position.x + movement.x,
        transform.position.y + movement.y + f64::from(0.6_f32) - transform.position.y + old_y,
        transform.position.z + movement.z,
    ];
    let collision_free = world.entity_collision_bounds_are_free(
        entity_id,
        entity_type_id,
        test_position,
        relative_min,
        relative_max,
        transform.on_ground.unwrap_or(false),
        entity_collision_targets,
    );
    let aabb_min = std::array::from_fn(|axis| test_position[axis] + relative_min[axis]);
    let aabb_max = std::array::from_fn(|axis| test_position[axis] + relative_max[axis]);
    if collision_free && !crate::client::entity_bounds_contains_any_fluid(world, aabb_min, aabb_max)
    {
        transform.delta_movement.y = f64::from(0.3_f32);
    }
}

fn entity_movement_block_friction(
    world: &WorldStore,
    position: [f64; 3],
    supporting_block: Option<crate::BlockPos>,
) -> f32 {
    let pos = entity_movement_effect_block_pos(world, position, supporting_block);
    let block_name = world.probe_block(pos).and_then(|block| block.block_name);
    crate::terrain::vanilla_block_friction(block_name.as_deref())
}

fn entity_block_jump_factor(
    world: &WorldStore,
    position: [f64; 3],
    supporting_block: Option<crate::BlockPos>,
) -> f32 {
    let here = crate::BlockPos {
        x: position[0].floor() as i32,
        y: position[1].floor() as i32,
        z: position[2].floor() as i32,
    };
    let here_name = world.probe_block(here).and_then(|block| block.block_name);
    let here_factor = crate::terrain::vanilla_block_jump_factor(here_name.as_deref());
    if here_factor != 1.0 {
        return here_factor;
    }
    let below = entity_movement_effect_block_pos(world, position, supporting_block);
    let below_name = world.probe_block(below).and_then(|block| block.block_name);
    crate::terrain::vanilla_block_jump_factor(below_name.as_deref())
}

#[allow(clippy::too_many_arguments)]
fn execute_local_horse_jump(
    world: &WorldStore,
    vehicle_id: i32,
    amount: f32,
    input: HappyGhastInput,
    position: [f64; 3],
    supporting_block: Option<crate::BlockPos>,
    transform: &mut super::components::EntityTransform,
) -> Option<()> {
    let mut impulse = world
        .entities
        .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID)?
        as f32
        * amount
        * entity_block_jump_factor(world, position, supporting_block);
    if let Some(amplifier) = world
        .entities
        .mob_effect_amplifier(vehicle_id, VANILLA_MOB_EFFECT_JUMP_BOOST_ID)
    {
        impulse += 0.1_f32 * (amplifier + 1) as f32;
    }
    transform.delta_movement.y = f64::from(impulse);
    if input.z > 0.0 {
        let yaw = transform.y_rot * std::f32::consts::PI / 180.0;
        let sin = super::client_movement::vanilla_mth_sin(f64::from(yaw));
        let cos = super::client_movement::vanilla_mth_cos(f64::from(yaw));
        transform.delta_movement.x += f64::from(-0.4_f32 * sin * amount);
        transform.delta_movement.z += f64::from(0.4_f32 * cos * amount);
    }
    Some(())
}

#[allow(clippy::too_many_arguments)]
fn execute_local_camel_dash(
    world: &mut WorldStore,
    vehicle_id: i32,
    entity_type_id: i32,
    amount: f32,
    movement_speed: f64,
    position: [f64; 3],
    supporting_block: Option<crate::BlockPos>,
    transform: &mut super::components::EntityTransform,
) -> Option<()> {
    let mut jump_momentum = world
        .entities
        .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID)?
        as f32
        * entity_block_jump_factor(world, position, supporting_block);
    if let Some(amplifier) = world
        .entities
        .mob_effect_amplifier(vehicle_id, VANILLA_MOB_EFFECT_JUMP_BOOST_ID)
    {
        jump_momentum += 0.1_f32 * (amplifier + 1) as f32;
    }
    let yaw = transform.y_rot * std::f32::consts::PI / 180.0;
    let sin = super::client_movement::vanilla_mth_sin(f64::from(yaw));
    let cos = super::client_movement::vanilla_mth_cos(f64::from(yaw));
    let horizontal_momentum = f64::from(22.2222_f32 * amount)
        * movement_speed
        * entity_block_speed_factor(world, position);
    transform.delta_movement.x += f64::from(-sin) * horizontal_momentum;
    transform.delta_movement.y += f64::from(1.4285_f32 * amount) * f64::from(jump_momentum);
    transform.delta_movement.z += f64::from(cos) * horizontal_momentum;
    let (dash_data_id, _) =
        vanilla_entity_metadata_camel_dash_accessor_for_entity_type(entity_type_id)?;
    let _ = world.entities.with_metadata_mut(vehicle_id, |metadata| {
        set_metadata_bool(metadata, dash_data_id, true);
    });
    world
        .entities
        .start_local_camel_dash_after_tick(vehicle_id)?;
    Some(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CamelRiddenPoseState {
    is_sitting: bool,
    is_in_transition: bool,
    refuses_to_move: bool,
}

fn camel_ridden_pose_state(world: &WorldStore, entity_id: i32) -> Option<CamelRiddenPoseState> {
    let entity_type_id = world.entities.entity_type_id(entity_id)?;
    let (data_id, default) =
        vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
            entity_type_id,
        )?;
    let last_pose_change_tick = world.entities.metadata_long(entity_id, data_id, default)?;
    let game_time = world.world_time().map_or(0, |time| time.game_time);
    let is_sitting = last_pose_change_tick < 0;
    let pose_time = game_time.saturating_sub(last_pose_change_tick.saturating_abs());
    let is_in_transition = pose_time < if is_sitting { 40 } else { 52 };
    Some(CamelRiddenPoseState {
        is_sitting,
        is_in_transition,
        refuses_to_move: is_sitting || is_in_transition,
    })
}

fn entity_movement_effect_block_pos(
    world: &WorldStore,
    position: [f64; 3],
    supporting_block: Option<crate::BlockPos>,
) -> crate::BlockPos {
    let position_y = (position[1] - f64::from(0.500001_f32)).floor() as i32;
    if let Some(mut support) = supporting_block {
        let keeps_support_y = world
            .probe_block(support)
            .and_then(|block| block.block_name)
            .is_some_and(|name| {
                crate::terrain::block_name_is_fence(&name)
                    || crate::terrain::block_name_is_wall(&name)
                    || crate::terrain::block_name_is_fence_gate(&name)
            });
        if !keeps_support_y {
            support.y = position_y;
        }
        support
    } else {
        crate::BlockPos {
            x: position[0].floor() as i32,
            y: position_y,
            z: position[2].floor() as i32,
        }
    }
}

fn float_boat(
    world: &WorldStore,
    entity_id: i32,
    entity_type_id: i32,
    relative_min: [f64; 3],
    relative_max: [f64; 3],
    no_gravity: bool,
    first_passenger_is_player: bool,
    entity_collision_targets: &[super::EntityPickTargetState],
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
            entity_id,
            entity_type_id,
            target_position,
            relative_min,
            relative_max,
            transform.on_ground.unwrap_or(false),
            entity_collision_targets,
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

fn living_vertical_collision_response(
    world: &WorldStore,
    position: [f64; 3],
    supporting_block: Option<crate::BlockPos>,
    requested_y: f64,
    suppressing_bounce: bool,
) -> f64 {
    let effect_pos = crate::BlockPos {
        x: supporting_block.map_or(position[0].floor() as i32, |pos| pos.x),
        y: (position[1] - 0.2).floor() as i32,
        z: supporting_block.map_or(position[2].floor() as i32, |pos| pos.z),
    };
    let block_name = world
        .probe_block(effect_pos)
        .and_then(|block| block.block_name);
    if suppressing_bounce {
        return 0.0;
    }
    match block_name.as_deref() {
        Some("minecraft:slime_block") if requested_y < 0.0 => -requested_y,
        Some(name) if crate::terrain::block_name_is_bed(name) && requested_y < 0.0 => {
            -requested_y * f64::from(0.66_f32)
        }
        Some("minecraft:slime_block") => requested_y,
        Some(name) if crate::terrain::block_name_is_bed(name) => requested_y,
        _ => 0.0,
    }
}

fn entity_block_speed_factor(world: &WorldStore, position: [f64; 3]) -> f64 {
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
    set_metadata_value(
        metadata,
        EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(value),
        },
    );
}

fn set_metadata_byte(metadata: &mut super::components::EntityMetadata, data_id: u8, value: i8) {
    set_metadata_value(
        metadata,
        EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(value),
        },
    );
}

fn set_metadata_long(metadata: &mut super::components::EntityMetadata, data_id: u8, value: i64) {
    set_metadata_value(
        metadata,
        EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
            value: EntityDataValueKind::Long(value),
        },
    );
}

fn set_metadata_pose(metadata: &mut super::components::EntityMetadata, data_id: u8, value: i32) {
    set_metadata_value(
        metadata,
        EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
            value: EntityDataValueKind::Pose(value),
        },
    );
}

fn set_metadata_value(metadata: &mut super::components::EntityMetadata, value: EntityDataValue) {
    if metadata
        .data_values
        .iter()
        .any(|existing| *existing == value)
    {
        return;
    }
    let data_id = value.data_id;
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
