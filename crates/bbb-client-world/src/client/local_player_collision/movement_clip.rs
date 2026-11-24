use super::*;

pub(crate) type LocalBrushBlockHit = (BlockPos, i32, [f64; 3], ProtocolDirection, [f64; 3]);

impl WorldStore {
    pub(crate) fn local_player_pose_collides_with_block(
        &self,
        pos: BlockPos,
        pose: LocalPlayerPoseState,
    ) -> bool {
        let Some(block) = self.block_view(pos) else {
            return false;
        };
        let bounds = self.local_player_bounds(pose);
        block_collides_with_local_player_bounds(
            self,
            &block,
            pos,
            bounds,
            LocalPlayerCollisionContext::for_pose(self, pose, false),
        )
    }

    pub fn particle_block_fluid_surface_sample(
        &self,
        position: [f64; 3],
    ) -> ParticleBlockFluidSurfaceSample {
        if !position.iter().all(|value| value.is_finite()) {
            return ParticleBlockFluidSurfaceSample::default();
        }
        let pos = BlockPos {
            x: block_floor(position[0]),
            y: block_floor(position[1]),
            z: block_floor(position[2]),
        };
        let Some(block) = self.block_view(pos) else {
            return ParticleBlockFluidSurfaceSample::default();
        };
        let local_x = position[0] - f64::from(pos.x);
        let local_z = position[2] - f64::from(pos.z);
        let block_height =
            block_collision_shape_with_registry(&block, pos, self.content_registry())
                .map(|shape| shape.max_y_at(local_x, local_z))
                .unwrap_or(0.0);
        let fluid_height = block
            .fluid
            .map(|fluid| crate::fluid::fluid_height_at(self, pos, fluid))
            .unwrap_or(0.0);
        ParticleBlockFluidSurfaceSample {
            block_collision_height: block_height,
            fluid_height,
            fluid_kind: block.fluid.map(|fluid| fluid.kind),
            block_is_air: block.block_name.is_none_or(block_name_is_air),
        }
    }

    /// Loaded block-state facts consumed by `ClientLevel.doAnimateTick` fluid
    /// animation and dripping-particle placement.
    pub fn environment_animate_block_sample(
        &self,
        pos: BlockPos,
    ) -> Option<EnvironmentAnimateBlockSample> {
        let block = self.block_view(pos)?;
        let shape = block_collision_shape_empty_context(&block, pos);
        let block_name = block.block_name?;
        let full_face_sturdy = self
            .block_full_face_sturdy_faces(&block)
            .unwrap_or([false; 6]);
        let collision_full_face = shape.map_or([false; 6], |shape| {
            VANILLA_PROTOCOL_DIRECTIONS
                .map(|direction| shape.has_full_face(block_direction(direction)))
        });
        let support_facts = vanilla_block_state_support_facts(block.block_state_id);
        Some(EnvironmentAnimateBlockSample {
            fluid: block.fluid,
            block_is_air: block_name_is_air(block_name),
            can_occlude: support_facts.is_some_and(|facts| facts.can_occlude()),
            solid_render: support_facts.is_some_and(|facts| facts.solid_render()),
            full_face_sturdy,
            collision_full_face,
            fire_flammable: block_name_is_fire_flammable(block_name),
            triggers_dried_ghast_ambient_sound: block_name_triggers_dried_ghast_ambient_sound(
                block_name,
            ),
            enchantment_power_provider: block_name_is_enchantment_power_provider(block_name),
            enchantment_power_transmitter: block_name_is_enchantment_power_transmitter(block_name),
            impermeable: block_name_is_impermeable(block_name),
            collision_bounds: shape.and_then(BlockCollisionShape::environment_animate_bounds),
        })
    }

    /// Mirrors `FallingBlock.isFree` for a loaded client block. Positions
    /// outside the dimension's build height are Vanilla void air; unloaded
    /// in-range positions remain unknown so ambient effects stay conservative.
    pub fn falling_block_is_free(&self, pos: BlockPos) -> Option<bool> {
        if !self.dimension().contains_y(pos.y) {
            return Some(true);
        }
        let block = self.block_view(pos)?;
        let block_name = block.block_name?;
        let facts = vanilla_block_state_support_facts(block.block_state_id)?;
        Some(
            block_name_is_air(block_name)
                || block_name_is_fire(block_name)
                || facts.liquid()
                || facts.replaceable(),
        )
    }

    pub fn particle_block_fluid_surface_height(&self, position: [f64; 3]) -> f64 {
        self.particle_block_fluid_surface_sample(position)
            .max_surface_height()
    }

    pub fn clip_particle_collision_movement(
        &self,
        position: [f64; 3],
        movement: [f64; 3],
        half_width: f64,
        height: f64,
    ) -> [f64; 3] {
        if !particle_collision_input_is_finite(position, movement) {
            return movement;
        }
        self.clip_collision_bounds_movement(
            particle_collision_bounds(position, half_width, height),
            movement,
        )
    }

    pub(crate) fn clip_entity_collision_movement(
        &self,
        position: [f64; 3],
        movement: [f64; 3],
        relative_min: [f64; 3],
        relative_max: [f64; 3],
    ) -> [f64; 3] {
        // The generated entity bounds are relative to the canonical feet/origin
        // position, matching `Entity.getBoundingBox()` before `Entity.move`.
        if !particle_collision_input_is_finite(position, movement) {
            return movement;
        }

        let bounds = WorldCollisionBox {
            min: [
                position[0] + relative_min[0],
                position[1] + relative_min[1],
                position[2] + relative_min[2],
            ],
            max: [
                position[0] + relative_max[0],
                position[1] + relative_max[1],
                position[2] + relative_max[2],
            ],
        };
        self.clip_collision_bounds_movement(bounds, movement)
    }

    /// Vanilla `Entity.collideBoundingBox` for a locally authoritative entity:
    /// resolve generated entity colliders and the nearby world-border shape in
    /// the same axis pass as block collision. The caller supplies one narrow
    /// collision-target batch for the whole client-tick catch-up instead of
    /// rebuilding full `EntityState` payloads for every movement substep.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn clip_locally_authoritative_entity_collision_movement(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        position: [f64; 3],
        movement: [f64; 3],
        relative_min: [f64; 3],
        relative_max: [f64; 3],
        max_up_step: f32,
        on_ground: bool,
        entity_collision_targets: &[crate::EntityPickTargetState],
    ) -> [f64; 3] {
        if !particle_collision_input_is_finite(position, movement) {
            return movement;
        }
        let bounds = WorldCollisionBox {
            min: std::array::from_fn(|axis| position[axis] + relative_min[axis]),
            max: std::array::from_fn(|axis| position[axis] + relative_max[axis]),
        };
        let block_context = LocalPlayerCollisionContext::for_remote_entity(
            bounds.min[1],
            self.entities.is_descending(entity_id).unwrap_or(false),
            self.remote_entity_can_walk_on_powder_snow(entity_id),
            entity_type_id == VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        )
        .with_can_stand_on_lava(entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID);
        let query = EntityCollisionQuery {
            source_entity_id: Some(entity_id),
            source_entity_type_id: entity_type_id,
            source_position: position,
            bounds,
            block_context,
        };
        self.clip_locally_authoritative_collision_movement(
            query,
            movement,
            max_up_step,
            on_ground,
            entity_collision_targets,
        )
    }

    /// Local-player `Entity.move`: use the live player collision context while
    /// sharing exact block, entity, border, axis-order, and step-candidate
    /// resolution with the other locally authoritative movement paths.
    pub(in crate::client) fn clip_local_player_collision_movement(
        &self,
        pose: LocalPlayerPoseState,
        movement: [f64; 3],
        max_up_step: f32,
        descending: bool,
        entity_collision_targets: &[crate::EntityPickTargetState],
    ) -> [f64; 3] {
        let position = [pose.position.x, pose.position.y, pose.position.z];
        if !particle_collision_input_is_finite(position, movement)
            || self.local_player_is_spectator()
        {
            return movement;
        }
        let bounds = self.local_player_bounds(pose);
        let block_context = LocalPlayerCollisionContext::for_pose(self, pose, descending);
        let query = EntityCollisionQuery {
            source_entity_id: self.local_player_id,
            source_entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            source_position: position,
            bounds,
            block_context,
        };
        self.clip_locally_authoritative_collision_movement(
            query,
            movement,
            max_up_step,
            pose.on_ground,
            entity_collision_targets,
        )
    }

    /// `CollisionGetter.noCollision(player, bounds)` for pose selection and
    /// sneak-edge support probes. The query retains the current player as its
    /// collision context/source while testing the supplied prospective box.
    pub(in crate::client) fn local_player_collision_bounds_are_free(
        &self,
        pose: LocalPlayerPoseState,
        bounds: WorldCollisionBox,
        descending: bool,
        entity_collision_targets: &[crate::EntityPickTargetState],
    ) -> bool {
        if self.local_player_is_spectator() {
            return true;
        }
        let block_context = LocalPlayerCollisionContext::for_pose(self, pose, descending);
        if local_player_collides_with_context(self, bounds, block_context) {
            return false;
        }
        let query = EntityCollisionQuery {
            source_entity_id: self.local_player_id,
            source_entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            source_position: [pose.position.x, pose.position.y, pose.position.z],
            bounds,
            block_context,
        };
        self.entity_collision_boxes_for_targets(
            query,
            bounds.inflated(COLLISION_EPSILON),
            entity_collision_targets,
        )
        .is_empty()
            && self.world_border_collision_boxes(query, bounds).is_empty()
    }

    /// Vanilla `LocalPlayer.suffocatesAt`: test the player's vertical span
    /// inside one block-cell column against only states whose live
    /// `isSuffocating` predicate is true.
    pub(in crate::client) fn local_player_suffocates_at(
        &self,
        pose: LocalPlayerPoseState,
        pos: BlockPos,
        descending: bool,
    ) -> bool {
        let bounds = WorldCollisionBox {
            min: [
                f64::from(pos.x) + COLLISION_EPSILON,
                pose.position.y + COLLISION_EPSILON,
                f64::from(pos.z) + COLLISION_EPSILON,
            ],
            max: [
                f64::from(pos.x) + 1.0 - COLLISION_EPSILON,
                pose.position.y + self.local_player_dimensions(pose).height - COLLISION_EPSILON,
                f64::from(pos.z) + 1.0 - COLLISION_EPSILON,
            ],
        };
        local_player_collides_with_suffocating_block(
            self,
            bounds,
            LocalPlayerCollisionContext::for_pose(self, pose, descending),
        )
    }

    /// Highest world-space Y in the live local-player collision shape at one
    /// block cell. `LocalPlayer.updateAutoJump` uses this both for its ceiling
    /// probes and for stacked obstacle-height refinement.
    pub(in crate::client) fn local_player_block_collision_shape_max_y(
        &self,
        pose: LocalPlayerPoseState,
        pos: BlockPos,
        descending: bool,
    ) -> Option<f64> {
        let block = self.block_view(pos)?;
        block_collision_shape_with_context(
            self,
            &block,
            pos,
            LocalPlayerCollisionContext::for_pose(self, pose, descending),
        )?
        .to_aabbs()
        .into_iter()
        .map(|shape_box| f64::from(pos.y) + shape_box.max[1])
        .reduce(f64::max)
    }

    /// Vanilla `CollisionGetter.getCollisions(player, bounds)` as consumed by
    /// auto-jump: collidable entities first, then block `VoxelShape.toAabbs`
    /// order. The world border is a separate collision source and is
    /// deliberately absent from this query.
    pub(in crate::client) fn local_player_auto_jump_collision_boxes(
        &self,
        pose: LocalPlayerPoseState,
        bounds: WorldCollisionBox,
        descending: bool,
        entity_collision_targets: &[crate::EntityPickTargetState],
    ) -> Vec<WorldCollisionBox> {
        let block_context = LocalPlayerCollisionContext::for_pose(self, pose, descending);
        let query = EntityCollisionQuery {
            source_entity_id: self.local_player_id,
            source_entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            source_position: [pose.position.x, pose.position.y, pose.position.z],
            bounds: self.local_player_bounds(pose),
            block_context,
        };
        let mut boxes = self.entity_collision_boxes_for_targets(
            query,
            bounds.inflated(COLLISION_EPSILON),
            entity_collision_targets,
        );

        let min_x = block_floor(bounds.min[0] - COLLISION_EPSILON).saturating_sub(1);
        let max_x = block_floor(bounds.max[0] + COLLISION_EPSILON).saturating_add(1);
        let min_y = block_floor(bounds.min[1] - COLLISION_EPSILON).saturating_sub(1);
        let max_y = block_floor(bounds.max[1] + COLLISION_EPSILON).saturating_add(1);
        let min_z = block_floor(bounds.min[2] - COLLISION_EPSILON).saturating_sub(1);
        let max_z = block_floor(bounds.max[2] + COLLISION_EPSILON).saturating_add(1);
        self.visit_loaded_chunk_block_positions(
            [min_x, min_y, min_z],
            [max_x, max_y, max_z],
            |pos| {
                let Some(block) = self.block_view(pos) else {
                    return;
                };
                let Some(shape) =
                    block_collision_shape_with_context(self, &block, pos, block_context)
                else {
                    return;
                };
                shape.visit_aabbs(|shape_box| {
                    let collision_box = world_collision_box(pos, shape_box);
                    if world_collision_boxes_intersect(collision_box, bounds) {
                        boxes.push(collision_box);
                    }
                });
            },
        );
        boxes
    }

    /// Vanilla's bounded `ClipContext.Block.FALLDAMAGE_RESETTING` /
    /// `ClipContext.Fluid.WATER` ray used by `Entity.move` before committing a
    /// long local-player displacement.
    pub(in crate::client) fn local_player_fall_reset_ray_hits(
        &self,
        pose: LocalPlayerPoseState,
        movement: [f64; 3],
    ) -> bool {
        let movement_length_squared = movement.iter().map(|value| value * value).sum::<f64>();
        if !movement_length_squared.is_finite() || movement_length_squared < 1.0 {
            return false;
        }
        let movement_length = movement_length_squared.sqrt();
        let scale = (8.0 / movement_length).min(1.0);
        let from = [pose.position.x, pose.position.y, pose.position.z];
        let to = std::array::from_fn(|axis| from[axis] + movement[axis] * scale);
        block_hit_scale(
            self,
            from,
            to,
            LocalPlayerCollisionContext::for_pose(self, pose, false),
            BlockClipShape::FallDamageResetting,
        )
        .is_some()
    }

    /// Current local-player `Entity.getOnPosLegacy()` block after collision
    /// support selection. Landing callbacks and the movement-after-fall tail
    /// consume this effect block rather than a separate feet sample.
    pub(in crate::client) fn local_player_legacy_effect_block(
        &self,
        pose: LocalPlayerPoseState,
        descending: bool,
    ) -> Option<BlockView> {
        let supporting_pos = pose
            .on_ground
            .then(|| self.local_player_supporting_block(pose, descending))?;
        let effect_pos = self.legacy_on_pos(
            supporting_pos,
            crate::EntityVec3 {
                x: pose.position.x,
                y: pose.position.y,
                z: pose.position.z,
            },
        );
        self.block_view(effect_pos)
    }

    pub(in crate::client) fn local_player_supporting_block(
        &self,
        pose: LocalPlayerPoseState,
        descending: bool,
    ) -> Option<BlockPos> {
        find_supporting_block(
            self,
            [pose.position.x, pose.position.y, pose.position.z],
            self.local_player_bounds(pose),
            LocalPlayerCollisionContext::for_pose(self, pose, descending),
        )
    }

    pub(super) fn clip_locally_authoritative_collision_movement(
        &self,
        query: EntityCollisionQuery,
        movement: [f64; 3],
        max_up_step: f32,
        on_ground: bool,
        entity_collision_targets: &[crate::EntityPickTargetState],
    ) -> [f64; 3] {
        let bounds = query.bounds;
        let block_context = query.block_context;
        let swept = bounds
            .swept_axis(CollisionAxis::X, movement[0])
            .swept_axis(CollisionAxis::Y, movement[1])
            .swept_axis(CollisionAxis::Z, movement[2]);
        let entity_colliders = self.entity_collision_boxes_for_targets(
            query,
            swept.inflated(COLLISION_EPSILON),
            entity_collision_targets,
        );
        let mut base_colliders = entity_colliders.clone();
        base_colliders.extend(self.world_border_collision_boxes(query, swept));
        base_colliders.extend(self.block_collision_boxes_for_bounds(swept, block_context));
        let clipped = bbb_facts::collision::collide_with_boxes(movement, bounds, &base_colliders);
        bbb_facts::collision::collide_with_step(
            bounds,
            movement,
            clipped,
            on_ground,
            max_up_step,
            |step_box| {
                let step_bounds = WorldCollisionBox::from(step_box);
                let mut step_colliders = entity_colliders.clone();
                step_colliders.extend(self.world_border_collision_boxes(query, step_bounds));
                step_colliders
                    .extend(self.block_collision_boxes_for_bounds(step_bounds, block_context));
                Some(step_colliders)
            },
        )
        .unwrap_or(clipped)
    }

    pub(super) fn block_collision_boxes_for_bounds(
        &self,
        bounds: WorldCollisionBox,
        context: LocalPlayerCollisionContext,
    ) -> Vec<WorldCollisionBox> {
        let min_x = block_floor(bounds.min[0] + COLLISION_EPSILON).saturating_sub(1);
        let max_x = block_floor(bounds.max[0] - COLLISION_EPSILON).saturating_add(1);
        let min_y = block_floor(bounds.min[1] + COLLISION_EPSILON).saturating_sub(1);
        let max_y = block_floor(bounds.max[1] - COLLISION_EPSILON).saturating_add(1);
        let min_z = block_floor(bounds.min[2] + COLLISION_EPSILON).saturating_sub(1);
        let max_z = block_floor(bounds.max[2] - COLLISION_EPSILON).saturating_add(1);
        let mut boxes = Vec::new();
        self.visit_loaded_chunk_block_positions(
            [min_x, min_y, min_z],
            [max_x, max_y, max_z],
            |pos| {
                let Some(block) = self.block_view(pos) else {
                    return;
                };
                let Some(shape) = block_collision_shape_with_context(self, &block, pos, context)
                else {
                    return;
                };
                boxes.extend(shape.boxes().filter_map(|shape_box| {
                    let collision_box = world_collision_box(pos, shape_box);
                    world_collision_boxes_intersect(collision_box, bounds).then_some(collision_box)
                }));
            },
        );
        boxes
    }

    /// Vanilla `AbstractBoat.getGroundFriction`: average the friction of each
    /// non-lily-pad collision shape intersecting the one-millimetre slice
    /// directly below the boat.
    pub(crate) fn boat_ground_friction(
        &self,
        position: [f64; 3],
        relative_min: [f64; 3],
        relative_max: [f64; 3],
    ) -> f32 {
        use bbb_facts::collision::{CollisionBounds, CollisionBox};

        let bounds = CollisionBox::from_arrays(
            [
                position[0] + relative_min[0],
                position[1] + relative_min[1],
                position[2] + relative_min[2],
            ],
            [
                position[0] + relative_max[0],
                position[1] + relative_max[1],
                position[2] + relative_max[2],
            ],
        );
        bbb_facts::movement::boat_ground_friction(bounds, |[x, y, z], intersects| {
            let pos = BlockPos { x, y, z };
            let block = self.block_view(pos)?;
            if block.block_name == Some("minecraft:lily_pad") {
                return None;
            }
            let shape = block_collision_shape_with_registry(&block, pos, self.content_registry())?;
            shape
                .boxes()
                .any(|shape_box| {
                    intersects(
                        world_collision_box(BlockPos { x: 0, y: 0, z: 0 }, shape_box)
                            .to_collision_box(),
                    )
                })
                .then(|| crate::terrain::vanilla_block_friction(block.block_name))
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn entity_collision_bounds_are_free(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        position: [f64; 3],
        relative_min: [f64; 3],
        relative_max: [f64; 3],
        on_ground: bool,
        entity_collision_targets: &[crate::EntityPickTargetState],
    ) -> bool {
        self.entity_collision_bounds_are_free_for_source(
            Some(entity_id),
            entity_type_id,
            position,
            relative_min,
            relative_max,
            on_ground,
            entity_collision_targets,
        )
    }

    pub(crate) fn prospective_entity_collision_bounds_are_free(
        &self,
        entity_type_id: i32,
        position: [f64; 3],
        relative_min: [f64; 3],
        relative_max: [f64; 3],
        on_ground: bool,
        entity_collision_targets: &[crate::EntityPickTargetState],
    ) -> bool {
        self.entity_collision_bounds_are_free_for_source(
            None,
            entity_type_id,
            position,
            relative_min,
            relative_max,
            on_ground,
            entity_collision_targets,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn entity_collision_bounds_are_free_for_source(
        &self,
        source_entity_id: Option<i32>,
        entity_type_id: i32,
        position: [f64; 3],
        relative_min: [f64; 3],
        relative_max: [f64; 3],
        on_ground: bool,
        entity_collision_targets: &[crate::EntityPickTargetState],
    ) -> bool {
        let bounds = WorldCollisionBox {
            min: std::array::from_fn(|axis| position[axis] + relative_min[axis]),
            max: std::array::from_fn(|axis| position[axis] + relative_max[axis]),
        };
        let block_context = LocalPlayerCollisionContext::for_remote_entity(
            position[1],
            on_ground,
            false,
            entity_type_id == VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        )
        .with_can_stand_on_lava(entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID);
        if local_player_collides_with_context(self, bounds, block_context) {
            return false;
        }
        let query = EntityCollisionQuery {
            source_entity_id,
            source_entity_type_id: entity_type_id,
            source_position: position,
            bounds,
            block_context,
        };
        self.entity_collision_boxes_for_targets(
            query,
            bounds.inflated(COLLISION_EPSILON),
            entity_collision_targets,
        )
        .is_empty()
            && self.world_border_collision_boxes(query, bounds).is_empty()
    }

    pub(super) fn entity_collision_boxes_for_targets(
        &self,
        query: EntityCollisionQuery,
        test_bounds: WorldCollisionBox,
        targets: &[crate::EntityPickTargetState],
    ) -> Vec<WorldCollisionBox> {
        targets
            .iter()
            .filter_map(|target| {
                if query.source_entity_id == Some(target.entity_id)
                    || !self.entity_passes_spectator_filter(target.entity_id)
                    || self
                        .debug_entities_share_root_vehicle(query.source_entity_id, target.entity_id)
                {
                    return None;
                }
                let target_type = self.entity_type_id(target.entity_id)?;
                let target_position = [target.position.x, target.position.y, target.position.z];
                let target_box = WorldCollisionBox {
                    min: std::array::from_fn(|axis| {
                        target_position[axis] + f64::from(target.bounds.min[axis])
                    }),
                    max: std::array::from_fn(|axis| {
                        target_position[axis] + f64::from(target.bounds.max[axis])
                    }),
                };
                (self.debug_entities_can_collide(
                    query,
                    target.entity_id,
                    target_type,
                    target_position,
                    target_box,
                ) && world_collision_boxes_intersect(target_box, test_bounds))
                .then_some(target_box)
            })
            .collect()
    }

    pub(super) fn world_border_collision_boxes(
        &self,
        query: EntityCollisionQuery,
        test_bounds: WorldCollisionBox,
    ) -> Vec<WorldCollisionBox> {
        let horizontal_size = (test_bounds.max[0] - test_bounds.min[0])
            .abs()
            .max((test_bounds.max[2] - test_bounds.min[2]).abs())
            .max(1.0);
        let border = self.world_border();
        let source_is_close = border
            .distance_to_border(query.source_position[0], query.source_position[2])
            < horizontal_size * 2.0
            && query.source_position[0] >= border.min_x_at(0.0) - horizontal_size
            && query.source_position[0] < border.max_x_at(0.0) + horizontal_size
            && query.source_position[2] >= border.min_z_at(0.0) - horizontal_size
            && query.source_position[2] < border.max_z_at(0.0) + horizontal_size;
        if !source_is_close {
            return Vec::new();
        }
        let min_x = border.min_x_at(0.0).floor();
        let min_z = border.min_z_at(0.0).floor();
        let max_x = border.max_x_at(0.0).ceil();
        let max_z = border.max_z_at(0.0).ceil();
        [
            WorldCollisionBox {
                min: [f64::NEG_INFINITY; 3],
                max: [min_x, f64::INFINITY, f64::INFINITY],
            },
            WorldCollisionBox {
                min: [max_x, f64::NEG_INFINITY, f64::NEG_INFINITY],
                max: [f64::INFINITY; 3],
            },
            WorldCollisionBox {
                min: [f64::NEG_INFINITY; 3],
                max: [f64::INFINITY, f64::INFINITY, min_z],
            },
            WorldCollisionBox {
                min: [f64::NEG_INFINITY, f64::NEG_INFINITY, max_z],
                max: [f64::INFINITY; 3],
            },
        ]
        .into_iter()
        .filter(|collision_box| world_collision_boxes_intersect(*collision_box, test_bounds))
        .collect()
    }

    /// Vanilla `ProjectileUtil.getHitResultOnMoveVector` block phase for a
    /// remote projectile. Returns the original direction shortened to the
    /// nearest collision-shape surface; fluid shapes are intentionally absent
    /// because FishingHook uses `ClipContext.Fluid.NONE`.
    pub(crate) fn clip_projectile_block_movement(
        &self,
        entity_type_id: i32,
        position: [f64; 3],
        movement: [f64; 3],
        on_ground: bool,
    ) -> ProjectileBlockMovementClip {
        self.clip_projectile_block_movement_with_context_y(
            entity_type_id,
            position,
            movement,
            on_ground,
            position[1],
        )
    }

    pub(crate) fn clip_projectile_block_movement_with_context_y(
        &self,
        entity_type_id: i32,
        position: [f64; 3],
        movement: [f64; 3],
        on_ground: bool,
        collision_context_y: f64,
    ) -> ProjectileBlockMovementClip {
        if !particle_collision_input_is_finite(position, movement) || movement == [0.0, 0.0, 0.0] {
            return ProjectileBlockMovementClip {
                movement,
                world_border_hit: false,
                block_state_id: None,
            };
        }
        let to = [
            position[0] + movement[0],
            position[1] + movement[1],
            position[2] + movement[2],
        ];
        let context = LocalPlayerCollisionContext::for_remote_entity(
            collision_context_y,
            on_ground,
            false,
            entity_type_id == VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        )
        .with_can_stand_on_lava(entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID);
        let shape_hit = block_hit_scale(self, position, to, context, BlockClipShape::Collision);
        let block_scale = shape_hit.map_or(1.0, |hit| hit.scale);
        let block_hit_location: [f64; 3] =
            std::array::from_fn(|axis| position[axis] + movement[axis] * block_scale);
        let border = self.world_border();
        let from_is_inside = position[0] >= border.min_x_at(0.0)
            && position[0] < border.max_x_at(0.0)
            && position[2] >= border.min_z_at(0.0)
            && position[2] < border.max_z_at(0.0);
        let block_hit_is_inside = block_hit_location[0] >= border.min_x_at(0.0)
            && block_hit_location[0] < border.max_x_at(0.0)
            && block_hit_location[2] >= border.min_z_at(0.0)
            && block_hit_location[2] < border.max_z_at(0.0);
        let world_border_hit = from_is_inside && !block_hit_is_inside;
        let scale = if world_border_hit {
            // `CollisionGetter.clipIncludingBorder` clamps the block hit (or
            // miss endpoint) to the border. Preserve that distance for the
            // entity-vs-border comparison; the caller uses the retained flag
            // to run FishingHook's reverse deflection instead of `onHitBlock`.
            let max_edge_offset = f64::from(1.0e-5_f32);
            let border_hit = [
                crate::client::mth_clamp_f64(
                    block_hit_location[0],
                    border.min_x_at(0.0),
                    border.max_x_at(0.0) - max_edge_offset,
                ),
                block_hit_location[1],
                crate::client::mth_clamp_f64(
                    block_hit_location[2],
                    border.min_z_at(0.0),
                    border.max_z_at(0.0) - max_edge_offset,
                ),
            ];
            let border_distance_squared = (border_hit[0] - position[0]).powi(2)
                + (border_hit[1] - position[1]).powi(2)
                + (border_hit[2] - position[2]).powi(2);
            let movement_length_squared = movement.iter().map(|value| value * value).sum::<f64>();
            (border_distance_squared / movement_length_squared).sqrt()
        } else {
            block_scale
        };
        ProjectileBlockMovementClip {
            movement: [
                movement[0] * scale,
                movement[1] * scale,
                movement[2] * scale,
            ],
            world_border_hit,
            block_state_id: if world_border_hit {
                None
            } else {
                shape_hit.map(|hit| hit.block_state_id)
            },
        }
    }

    /// `BrushItem.calculateHitResult` (`BrushItem.java:98-100`): clips the local
    /// player's view vector against block collision shapes within interaction range.
    pub(crate) fn clip_local_player_brush_block_hit(&self) -> Option<LocalBrushBlockHit> {
        let pose = self.local_player().pose?;
        let eye_height = self.local_player_eye_height(pose);
        let from = [
            pose.position.x,
            pose.position.y + eye_height,
            pose.position.z,
        ];
        let x_rad = f64::from(pose.x_rot).to_radians();
        let y_rad = f64::from(-pose.y_rot).to_radians();
        let cos_x = x_rad.cos();
        let view = [y_rad.sin() * cos_x, -x_rad.sin(), y_rad.cos() * cos_x];
        let range = self
            .local_player_attribute_value(
                bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
            )
            .unwrap_or(4.5_f64)
            .clamp(0.0_f64, 64.0_f64);
        let to = [
            from[0] + view[0] * range,
            from[1] + view[1] * range,
            from[2] + view[2] * range,
        ];
        let context = LocalPlayerCollisionContext::for_pose(self, pose, pose.sneaking);
        let hit = block_hit_scale(self, from, to, context, BlockClipShape::Collision)?;
        let hit_loc = [
            from[0] + (to[0] - from[0]) * hit.scale,
            from[1] + (to[1] - from[1]) * hit.scale,
            from[2] + (to[2] - from[2]) * hit.scale,
        ];
        // Infer hit face from the axis where `hit_loc` is closest to a face of `hit.pos`.
        let rx = hit_loc[0] - f64::from(hit.pos.x);
        let ry = hit_loc[1] - f64::from(hit.pos.y);
        let rz = hit_loc[2] - f64::from(hit.pos.z);
        let faces = [
            (rx.abs(), ProtocolDirection::West),
            ((1.0 - rx).abs(), ProtocolDirection::East),
            (ry.abs(), ProtocolDirection::Down),
            ((1.0 - ry).abs(), ProtocolDirection::Up),
            (rz.abs(), ProtocolDirection::North),
            ((1.0 - rz).abs(), ProtocolDirection::South),
        ];
        let face = faces
            .into_iter()
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map_or(ProtocolDirection::Up, |(_, dir)| dir);
        Some((hit.pos, hit.block_state_id, hit_loc, face, view))
    }

    /// `AbstractArrow.tick`'s pre-flight containment check at
    /// `blockPosition()`. This is intentionally separate from segment clipping:
    /// Vanilla grounds an arrow at its exact current position before applying
    /// water, crit, rotation, or movement.
    pub(crate) fn projectile_position_inside_block_collision(
        &self,
        entity_type_id: i32,
        position: [f64; 3],
        on_ground: bool,
    ) -> Option<i32> {
        if !position.iter().all(|value| value.is_finite()) {
            return None;
        }
        let pos = BlockPos {
            x: block_floor(position[0]),
            y: block_floor(position[1]),
            z: block_floor(position[2]),
        };
        let block = self.block_view(pos)?;
        let context = LocalPlayerCollisionContext::for_remote_entity(
            position[1],
            on_ground,
            false,
            entity_type_id == VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        )
        .with_can_stand_on_lava(entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID);
        let shape = block_collision_shape_with_context(self, &block, pos, context)?;
        shape
            .boxes()
            .any(|shape_box| {
                world_collision_box_contains(world_collision_box(pos, shape_box), position)
            })
            .then_some(block.block_state_id)
    }

    pub(crate) fn entity_supporting_block_after_collision(
        &self,
        entity_type_id: i32,
        position: [f64; 3],
        clipped_movement: [f64; 3],
        relative_min: [f64; 3],
        relative_max: [f64; 3],
        previous: EntitySupportingBlock,
    ) -> Option<BlockPos> {
        let bounds = WorldCollisionBox {
            min: [
                position[0] + relative_min[0] + clipped_movement[0],
                position[1] + relative_min[1] + clipped_movement[1],
                position[2] + relative_min[2] + clipped_movement[2],
            ],
            max: [
                position[0] + relative_max[0] + clipped_movement[0],
                position[1] + relative_max[1] + clipped_movement[1],
                position[2] + relative_max[2] + clipped_movement[2],
            ],
        };
        let entity_position = [
            position[0] + clipped_movement[0],
            position[1] + clipped_movement[1],
            position[2] + clipped_movement[2],
        ];
        let context = LocalPlayerCollisionContext::for_remote_entity(
            bounds.min[1],
            clipped_movement[1] < 0.0,
            false,
            entity_type_id == VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        )
        .with_can_stand_on_lava(entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID);
        let current = find_supporting_block(self, entity_position, bounds, context);
        if current.is_some() || previous.on_ground_no_blocks {
            return current;
        }
        find_supporting_block(
            self,
            entity_position,
            bounds.moved(-clipped_movement[0], 0.0, -clipped_movement[2]),
            context,
        )
    }

    pub(super) fn clip_collision_bounds_movement(
        &self,
        bounds: WorldCollisionBox,
        movement: [f64; 3],
    ) -> [f64; 3] {
        let mut clipped = [0.0; 3];
        for axis in particle_collision_axis_order(movement) {
            let idx = axis.index();
            let requested = movement[idx];
            if requested == 0.0 {
                continue;
            }
            let moved_bounds = bounds.moved(clipped[0], clipped[1], clipped[2]);
            let mut axis_clipped = self.clip_axis_collision(moved_bounds, axis, requested);
            if axis_clipped.abs() < COLLISION_EPSILON {
                axis_clipped = 0.0;
            }
            clipped[idx] = axis_clipped;
        }
        clipped
    }

    pub(super) fn clip_axis_collision(
        &self,
        bounds: WorldCollisionBox,
        axis: CollisionAxis,
        requested: f64,
    ) -> f64 {
        let swept = bounds.swept_axis(axis, requested);
        let min_x = block_floor(swept.min[0] + COLLISION_EPSILON).saturating_sub(1);
        let max_x = block_floor(swept.max[0] - COLLISION_EPSILON).saturating_add(1);
        let min_y = block_floor(swept.min[1] + COLLISION_EPSILON).saturating_sub(1);
        let max_y = block_floor(swept.max[1] - COLLISION_EPSILON).saturating_add(1);
        let min_z = block_floor(swept.min[2] + COLLISION_EPSILON).saturating_sub(1);
        let max_z = block_floor(swept.max[2] - COLLISION_EPSILON).saturating_add(1);

        let mut clipped = requested;
        let content_registry = self.content_registry();
        self.visit_collision_block_states(
            [min_x, min_y, min_z],
            [max_x, max_y, max_z],
            |pos, state_id| {
                visit_block_collision_boxes_with_registry(
                    pos,
                    state_id,
                    content_registry,
                    |shape_box| {
                        clipped = clipped_particle_axis_against_box(
                            bounds, axis, requested, clipped, pos, shape_box,
                        );
                    },
                );
            },
        );
        if clipped.abs() < COLLISION_EPSILON {
            0.0
        } else {
            clipped
        }
    }

    pub(super) fn visit_loaded_chunk_block_positions(
        &self,
        [min_x, min_y, min_z]: [i32; 3],
        [max_x, max_y, max_z]: [i32; 3],
        mut visit: impl FnMut(BlockPos),
    ) {
        self.visit_collision_block_states(
            [min_x, min_y, min_z],
            [max_x, max_y, max_z],
            |pos, _| {
                visit(pos);
            },
        );
    }

    pub(super) fn visit_collision_block_states(
        &self,
        [min_x, min_y, min_z]: [i32; 3],
        [max_x, max_y, max_z]: [i32; 3],
        mut visit: impl FnMut(BlockPos, i32),
    ) {
        if self.chunks.is_empty() || min_x > max_x || min_z > max_z {
            return;
        }
        let dimension = self.dimension();
        let min_y = min_y.max(dimension.min_y);
        let max_y = max_y.min(
            dimension
                .min_y
                .saturating_add(dimension.height)
                .saturating_sub(1),
        );
        if min_y > max_y {
            return;
        }
        let min_cx = min_x.div_euclid(16);
        let max_cx = max_x.div_euclid(16);
        let min_cz = min_z.div_euclid(16);
        let max_cz = max_z.div_euclid(16);
        let chunk_span_x = (i64::from(max_cx) - i64::from(min_cx)).saturating_add(1);
        let chunk_span_z = (i64::from(max_cz) - i64::from(min_cz)).saturating_add(1);
        let rect_chunks = chunk_span_x.saturating_mul(chunk_span_z);
        let min_sy = min_y.div_euclid(16);
        let max_sy = max_y.div_euclid(16);
        let min_section_y = dimension.min_section_y();

        let mut visit_column = |column: &crate::ChunkColumn| {
            let cx = column.pos.x;
            let cz = column.pos.z;
            let x_start = min_x.max(cx.saturating_mul(16));
            let x_end = max_x.min(cx.saturating_mul(16).saturating_add(15));
            let z_start = min_z.max(cz.saturating_mul(16));
            let z_end = max_z.min(cz.saturating_mul(16).saturating_add(15));
            for sy in min_sy..=max_sy {
                let Some(section) = column.sections.get((sy - min_section_y) as usize) else {
                    continue;
                };
                if section.non_empty_block_count == 0 && section.fluid_count == 0 {
                    continue;
                }
                let y_start = min_y.max(sy.saturating_mul(16));
                let y_end = max_y.min(sy.saturating_mul(16).saturating_add(15));
                for y in y_start..=y_end {
                    for z in z_start..=z_end {
                        for x in x_start..=x_end {
                            let index = crate::section_block_index(
                                (x & 15) as u8,
                                (y & 15) as u8,
                                (z & 15) as u8,
                            );
                            if let Some(value) = section.block_states.value_at(index) {
                                visit(BlockPos { x, y, z }, value.global_id);
                            }
                        }
                    }
                }
            }
        };

        if rect_chunks > self.chunks.len() as i64 {
            for column in self.chunks.iter() {
                if column.pos.x >= min_cx
                    && column.pos.x <= max_cx
                    && column.pos.z >= min_cz
                    && column.pos.z <= max_cz
                {
                    visit_column(column);
                }
            }
        } else {
            for cz in min_cz..=max_cz {
                for cx in min_cx..=max_cx {
                    if let Some(column) = self.probe_chunk(crate::ChunkPos { x: cx, z: cz }) {
                        visit_column(column);
                    }
                }
            }
        }
    }
}
