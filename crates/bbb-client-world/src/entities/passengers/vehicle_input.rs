use super::*;
use crate::BlockPos;
use crate::LocalPlayerPoseState;

/// Vanilla `AbstractBoat.clampRotation` (`AbstractBoat.java:661-668`): clamps
/// a boat passenger's yaw to within ±105° of the boat's yaw and shifts `yRotO`
/// by the same correction so interpolation stays seamless.
pub(crate) fn clamp_local_player_boat_yaw(
    pose: &mut LocalPlayerPoseState,
    old_pose: Option<&mut LocalPlayerPoseState>,
    boat_y_rot: f32,
) {
    let delta = wrap_degrees_f32(pose.y_rot - boat_y_rot);
    let target_delta = delta.clamp(-105.0, 105.0);
    let correction = target_delta - delta;
    if correction != 0.0 {
        pose.y_rot += correction;
        if let Some(old_pose) = old_pose {
            old_pose.y_rot += correction;
        }
    }
}

impl WorldStore {
    fn vehicle_entity_collision_targets_for(
        &self,
        vehicle_id: i32,
        ticks: u32,
    ) -> Vec<crate::entities::EntityPickTargetState> {
        if ticks == 0 {
            return Vec::new();
        }
        let Some(transform) = self.entities.transform(vehicle_id) else {
            return self.entities.vehicle_entity_collision_targets();
        };
        let bounds = self.entities.pick_bounds(vehicle_id);
        let vel_max = transform
            .delta_movement
            .x
            .abs()
            .max(transform.delta_movement.y.abs())
            .max(transform.delta_movement.z.abs());
        let margin = 16.0 + f64::from(ticks) * (vel_max + 4.0);
        let (min_off, max_off) = bounds.map_or(([0.0; 3], [0.0; 3]), |b| {
            (b.min.map(f64::from), b.max.map(f64::from))
        });
        self.entities.vehicle_entity_collision_targets_in_aabb(
            [
                transform.position.x + min_off[0] - margin,
                transform.position.y + min_off[1] - margin,
                transform.position.z + min_off[2] - margin,
            ],
            [
                transform.position.x + max_off[0] + margin,
                transform.position.y + max_off[1] + margin,
                transform.position.z + max_off[2] + margin,
            ],
        )
    }

    fn vehicle_move_report(&self, vehicle_id: i32) -> Option<VehicleMoveReport> {
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

    /// Advances the active locally controlled vehicle for `ticks` catch-up
    /// ticks, hoisting the spatial entity-collision query once across the
    /// batch while returning one `VehicleMoveReport` per tick.
    pub fn advance_local_controlled_vehicle_ticks(
        &mut self,
        input: LocalPlayerInputState,
        ticks: u32,
    ) -> Vec<VehicleMoveReport> {
        let mut reports = Vec::with_capacity(ticks as usize);
        if self
            .advance_local_boat_vehicle_input_inner(input, ticks, Some(&mut reports))
            .is_some()
            || self
                .advance_local_abstract_horse_vehicle_input_inner(input, ticks, Some(&mut reports))
                .is_some()
            || self
                .advance_local_nautilus_vehicle_input_inner(input, ticks, Some(&mut reports))
                .is_some()
            || self
                .advance_local_item_based_steering_vehicle_input_inner(ticks, Some(&mut reports))
                .is_some()
            || self
                .advance_local_happy_ghast_vehicle_input_inner(input, ticks, Some(&mut reports))
                .is_some()
        {
            return reports;
        }
        Vec::new()
    }

    pub fn advance_local_boat_vehicle_input(
        &mut self,
        input: LocalPlayerInputState,
        ticks: u32,
    ) -> Option<VehicleMoveReport> {
        self.advance_local_boat_vehicle_input_inner(input, ticks, None)
    }

    fn clear_local_player_riding_swimming_and_sneaking(&mut self) {
        self.local_player.swimming = false;
        if let Some(pose) = self.local_player.pose.as_mut() {
            pose.swimming = false;
            pose.sneaking = false;
        }
    }

    fn advance_local_boat_vehicle_input_inner(
        &mut self,
        input: LocalPlayerInputState,
        ticks: u32,
        mut per_tick_reports: Option<&mut Vec<VehicleMoveReport>>,
    ) -> Option<VehicleMoveReport> {
        let vehicle_id = self.local_player_first_passenger_root_boat_vehicle_id()?;
        self.clear_local_player_riding_swimming_and_sneaking();
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
        let entity_collision_targets = self.vehicle_entity_collision_targets_for(vehicle_id, ticks);
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
            // Vanilla `AbstractBoat.positionRider` (`AbstractBoat.java:618-620`)
            // adds `deltaRotation` to passenger yaw and clamps within ±105° of boat yaw.
            if let Some(pose) = self.local_player.pose.as_mut() {
                pose.y_rot += movement.delta_rotation;
                if let Some(old_pose) = self.local_player_old_pose.as_mut() {
                    old_pose.y_rot += movement.delta_rotation;
                }
                clamp_local_player_boat_yaw(
                    pose,
                    self.local_player_old_pose.as_mut(),
                    next_transform.y_rot,
                );
            }
            if let Some(reports) = per_tick_reports.as_deref_mut() {
                reports.push(self.vehicle_move_report(vehicle_id)?);
            }
        }
        if ticks > 0 {
            let accessors =
                vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(entity_type_id)?;
            self.entities.with_metadata_mut(vehicle_id, |metadata| {
                set_metadata_bool(metadata, accessors.paddle_left_data_id, paddle_left);
                set_metadata_bool(metadata, accessors.paddle_right_data_id, paddle_right);
            })?;
        }

        self.vehicle_move_report(vehicle_id)
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
        self.advance_local_happy_ghast_vehicle_input_inner(input, ticks, None)
    }

    fn advance_local_happy_ghast_vehicle_input_inner(
        &mut self,
        input: LocalPlayerInputState,
        ticks: u32,
        mut per_tick_reports: Option<&mut Vec<VehicleMoveReport>>,
    ) -> Option<VehicleMoveReport> {
        let vehicle_id = self.local_player_first_passenger_root_vehicle_id()?;
        self.clear_local_player_riding_swimming_and_sneaking();
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
        let entity_collision_targets = self.vehicle_entity_collision_targets_for(vehicle_id, ticks);

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
            if let Some(reports) = per_tick_reports.as_deref_mut() {
                reports.push(self.vehicle_move_report(vehicle_id)?);
            }
        }

        self.vehicle_move_report(vehicle_id)
    }

    /// Advances a directly controlled `AbstractHorse` through its retained
    /// jump stage, `LivingEntity.travelRidden`, and the shared air/fluid travel
    /// owner. Camel adds its refusal, dash, and pose-transition subclass work.
    pub fn advance_local_abstract_horse_vehicle_input(
        &mut self,
        input: LocalPlayerInputState,
        ticks: u32,
    ) -> Option<VehicleMoveReport> {
        self.advance_local_abstract_horse_vehicle_input_inner(input, ticks, None)
    }

    fn advance_local_abstract_horse_vehicle_input_inner(
        &mut self,
        input: LocalPlayerInputState,
        ticks: u32,
        mut per_tick_reports: Option<&mut Vec<VehicleMoveReport>>,
    ) -> Option<VehicleMoveReport> {
        let vehicle_id = self.local_player_first_passenger_root_vehicle_id()?;
        let entity_type_id = self.entities.entity_type_id(vehicle_id)?;
        if !is_vanilla_abstract_horse_type(entity_type_id)
            || self.entity_controlling_passenger(vehicle_id) != self.local_player_id
        {
            return None;
        }
        self.clear_local_player_riding_swimming_and_sneaking();
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
        let entity_collision_targets = self.vehicle_entity_collision_targets_for(vehicle_id, ticks);

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
            next_transform.delta_movement =
                crate::entities::client_movement::apply_entity_fluid_currents(
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
                let (friction, speed_scale) = vehicle_air_friction_and_ground_scale(
                    self,
                    position,
                    old_on_ground,
                    previous_supporting_block.main,
                );
                let speed = movement_speed * speed_scale;
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
            if let Some(reports) = per_tick_reports.as_deref_mut() {
                reports.push(self.vehicle_move_report(vehicle_id)?);
            }
        }

        self.vehicle_move_report(vehicle_id)
    }

    /// Advances a directly controlled `AbstractNautilus` through its exact
    /// `tickRidden` dash stage, overridden water travel, shared lava/air travel,
    /// and post-movement subclass cooldown/sound/bubble tick.
    pub fn advance_local_nautilus_vehicle_input(
        &mut self,
        input: LocalPlayerInputState,
        ticks: u32,
    ) -> Option<VehicleMoveReport> {
        self.advance_local_nautilus_vehicle_input_inner(input, ticks, None)
    }

    fn advance_local_nautilus_vehicle_input_inner(
        &mut self,
        input: LocalPlayerInputState,
        ticks: u32,
        mut per_tick_reports: Option<&mut Vec<VehicleMoveReport>>,
    ) -> Option<VehicleMoveReport> {
        let vehicle_id = self.local_player_first_passenger_root_vehicle_id()?;
        let entity_type_id = self.entities.entity_type_id(vehicle_id)?;
        if !is_vanilla_abstract_nautilus_type(entity_type_id)
            || self.entity_controlling_passenger(vehicle_id) != self.local_player_id
        {
            return None;
        }
        self.clear_local_player_riding_swimming_and_sneaking();
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
            forward = crate::entities::client_movement::vanilla_mth_cos(f64::from(pitch));
            up = -crate::entities::client_movement::vanilla_mth_sin(f64::from(pitch));
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
        let entity_collision_targets = self.vehicle_entity_collision_targets_for(vehicle_id, ticks);

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
                let look =
                    crate::entities::calculate_view_vector(controller.x_rot, controller.y_rot);
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
                let (friction, speed_scale) = vehicle_air_friction_and_ground_scale(
                    self,
                    position,
                    old_on_ground,
                    previous_supporting_block.main,
                );
                let ridden_speed = f64::from(0.02_f32) * movement_speed;
                let speed = ridden_speed * f64::from(speed_scale);
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
            self.entities
                .advance_deferred_nautilus_subclass_tick(vehicle_id, fluid_contact.in_water())?;
            if let Some(reports) = per_tick_reports.as_deref_mut() {
                reports.push(self.vehicle_move_report(vehicle_id)?);
            }
        }

        self.vehicle_move_report(vehicle_id)
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
        self.advance_local_item_based_steering_vehicle_input_inner(ticks, None)
    }

    fn advance_local_item_based_steering_vehicle_input_inner(
        &mut self,
        ticks: u32,
        mut per_tick_reports: Option<&mut Vec<VehicleMoveReport>>,
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
        let entity_collision_targets = self.vehicle_entity_collision_targets_for(vehicle_id, ticks);
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
            if let Some((strider_data_id, strider_default)) = strider_suffocating {
                let mob_accessors =
                    vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id)?;
                let mob_flags = self.entities.metadata_byte(
                    vehicle_id,
                    mob_accessors.flags_data_id,
                    mob_accessors.flags_default,
                )?;
                if mob_flags & mob_accessors.no_ai_mask == 0 {
                    // Vanilla `Strider.tick` (`Strider.java:295-303`): warm when
                    // inside or standing on `#minecraft:strider_warm_blocks`,
                    // touching lava, or riding a non-suffocating Strider.
                    let inside_pos = BlockPos {
                        x: position[0].floor() as i32,
                        y: position[1].floor() as i32,
                        z: position[2].floor() as i32,
                    };
                    let on_legacy_pos = BlockPos {
                        x: position[0].floor() as i32,
                        y: (position[1] - 0.2).floor() as i32,
                        z: position[2].floor() as i32,
                    };
                    let in_warm_blocks = fluid_contact.in_lava()
                        || strider_block_is_warm(self, inside_pos)
                        || strider_block_is_warm(self, on_legacy_pos);
                    let on_warm_strider = self
                        .entities
                        .mount(vehicle_id)
                        .and_then(|mount| mount.vehicle_id)
                        .is_some_and(|mount_id| {
                            self.entities.entity_type_id(mount_id)
                                == Some(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID)
                                && !self
                                    .entities
                                    .metadata_bool_for_entity(
                                        mount_id,
                                        strider_data_id,
                                        strider_default,
                                    )
                                    .unwrap_or(strider_default)
                        });
                    self.entities
                        .set_strider_suffocating(vehicle_id, !in_warm_blocks && !on_warm_strider)?;
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
            next_transform.delta_movement =
                crate::entities::client_movement::apply_entity_fluid_currents(
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
            // Vanilla `LivingEntity.shouldTravelInFluid` (`LivingEntity.java:2384,2398`)
            // and `Strider.canStandOnFluid` (`Strider.java:174-176`): bypass
            // `travelInFluid` when a Strider stands on lava at `blockPosition()`.
            let strider_stands_on_lava = strider_suffocating.is_some()
                && fluid_contact.block_fluid_kind == Some(crate::TerrainFluidKind::Lava);
            if !strider_stands_on_lava && fluid_contact.in_water() {
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
            } else if !strider_stands_on_lava && fluid_contact.in_lava() {
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
                let (friction, speed_scale) = vehicle_air_friction_and_ground_scale(
                    self,
                    position,
                    old_on_ground,
                    previous_supporting_block.main,
                );
                let speed = movement_speed * f64::from(speed_scale);
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
            if let Some(reports) = per_tick_reports.as_deref_mut() {
                reports.push(self.vehicle_move_report(vehicle_id)?);
            }
        }

        self.vehicle_move_report(vehicle_id)
    }

    pub(super) fn advance_local_strider_float_after_tick(
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
    pub(super) fn apply_local_vehicle_block_effects_from_recorder(&mut self, vehicle_id: i32) {
        let Some(movement) = self.entities.take_local_vehicle_move(vehicle_id) else {
            return;
        };
        self.apply_local_vehicle_block_effects(vehicle_id, movement);
    }

    /// Locally authoritative `applyEffectsFromBlocks` for the directly
    /// controlled vehicle, borrowing the immutable terrain owner beside the
    /// mutable entity store. Pass count and family gating come from the
    /// generated block-effect tick policy.
    pub(super) fn apply_local_vehicle_block_effects(
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
        let empty_collisions = rustc_hash::FxHashMap::default();
        let _ = self.entities.apply_local_vehicle_block_effect_tick(
            vehicle_id,
            movement,
            &empty_collisions,
            &mut block_effect_world,
        );
    }
}

fn strider_block_is_warm(world: &WorldStore, pos: BlockPos) -> bool {
    let Some(block) = world.block_view(pos) else {
        return false;
    };
    if let Some(block_id) =
        bbb_protocol::block_state_types::vanilla_block_state_block_type_id(block.block_state_id)
            .or_else(|| {
                world
                    .content_registry()
                    .block_by_state_id(block.block_state_id)
                    .map(|(spec, _)| spec.block_id)
            })
    {
        if world.registry_holder_is_in_tag(
            "minecraft:block",
            "minecraft:strider_warm_blocks",
            block_id,
        ) {
            return true;
        }
    }
    block.block_name == Some("minecraft:lava")
}

fn vehicle_air_friction_and_ground_scale(
    world: &WorldStore,
    position: [f64; 3],
    old_on_ground: bool,
    main_supporting: Option<crate::BlockPos>,
) -> (f32, f32) {
    let block_friction = if old_on_ground {
        entity_movement_block_friction(world, position, main_supporting)
    } else {
        1.0
    };
    let friction = bbb_facts::movement::air_horizontal_friction(block_friction);
    let speed_scale = if old_on_ground {
        bbb_facts::movement::ground_friction_speed_scale(block_friction)
    } else {
        0.1_f32
    };
    (friction, speed_scale)
}
