use super::*;

impl WorldStore {
    /// Mirrors `CollisionGetter.getCollisions`: collidable entity AABBs first,
    /// followed by coalesced block `VoxelShape.toAabbs()` boxes.
    pub fn debug_camera_collision_boxes(&self, radius: f64) -> Vec<WorldCollisionBox> {
        if !radius.is_finite() || radius < 0.0 {
            return Vec::new();
        }
        let Some(camera) = self.debug_camera_collision_query() else {
            return Vec::new();
        };
        let mut boxes = self.debug_camera_entity_collision_boxes_for_query(camera, radius);
        boxes.extend(self.debug_camera_block_collision_boxes_for_query(camera, radius));
        boxes
    }

    /// Mirrors the block half of
    /// `level.getCollisions(cameraEntity, cameraEntity.getBoundingBox().inflate(radius))`.
    /// Every coalesced `VoxelShape.toAabbs()` box of a shape that intersects
    /// the query bounds is returned, including boxes extending beyond the
    /// query itself.
    pub fn debug_camera_block_collision_boxes(&self, radius: f64) -> Vec<WorldCollisionBox> {
        if !radius.is_finite() || radius < 0.0 {
            return Vec::new();
        }
        let Some(camera) = self.debug_camera_collision_query() else {
            return Vec::new();
        };
        self.debug_camera_block_collision_boxes_for_query(camera, radius)
    }

    /// Nearest Vanilla `ClipContext.Block.VISUAL` block hit on a camera ray.
    ///
    /// Fluids and entities are intentionally absent. The collision context is
    /// derived from the current camera entity, matching `Camera.getMaxZoom`.
    pub fn camera_visual_block_hit(&self, from: [f64; 3], to: [f64; 3]) -> Option<[f64; 3]> {
        if !from.iter().chain(&to).all(|value| value.is_finite()) || from == to {
            return None;
        }
        let context = self.debug_camera_collision_query()?.block_context;
        let scale = block_hit_scale(self, from, to, context, BlockClipShape::Visual)?.scale;
        Some(std::array::from_fn(|axis| {
            from[axis] + (to[axis] - from[axis]) * scale
        }))
    }

    pub(super) fn debug_camera_block_collision_boxes_for_query(
        &self,
        camera: EntityCollisionQuery,
        radius: f64,
    ) -> Vec<WorldCollisionBox> {
        const MAX_DEBUG_CAMERA_RADIUS: f64 = 64.0;
        const MAX_DEBUG_CAMERA_EXTENT: f64 = 16.0;
        const MAX_DEBUG_CAMERA_BOXES: usize = 65_536;
        let radius = radius.min(MAX_DEBUG_CAMERA_RADIUS);
        let raw_query = camera.bounds.inflated(radius);
        let max_half = radius + MAX_DEBUG_CAMERA_EXTENT;
        let query = WorldCollisionBox {
            min: std::array::from_fn(|axis| {
                raw_query.min[axis].max(camera.source_position[axis] - max_half)
            }),
            max: std::array::from_fn(|axis| {
                raw_query.max[axis].min(camera.source_position[axis] + max_half)
            }),
        };
        let min_x = block_floor(query.min[0] - COLLISION_EPSILON).saturating_sub(1);
        let max_x = block_floor(query.max[0] + COLLISION_EPSILON).saturating_add(1);
        let min_y = block_floor(query.min[1] - COLLISION_EPSILON).saturating_sub(1);
        let max_y = block_floor(query.max[1] + COLLISION_EPSILON).saturating_add(1);
        let min_z = block_floor(query.min[2] - COLLISION_EPSILON).saturating_sub(1);
        let max_z = block_floor(query.max[2] + COLLISION_EPSILON).saturating_add(1);
        let dimension = self.dimension();
        let min_y = min_y.max(dimension.min_y);
        let max_y = max_y.min(
            dimension
                .min_y
                .saturating_add(dimension.height)
                .saturating_sub(1),
        );
        if min_x > max_x || min_y > max_y || min_z > max_z {
            return Vec::new();
        }

        let mut boxes = Vec::new();
        // Vanilla Cursor3D advances X fastest, then Y, then Z.
        'outer: for z in min_z..=max_z {
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let pos = BlockPos { x, y, z };
                    let Some(block) = self.block_view(pos) else {
                        continue;
                    };
                    let Some(shape) =
                        block_collision_shape_with_context(self, &block, pos, camera.block_context)
                    else {
                        continue;
                    };
                    let start = boxes.len();
                    let mut intersects = false;
                    shape.visit_aabbs(|shape_box| {
                        intersects |= bounds_intersects_block_box(query, pos, shape_box);
                        boxes.push(world_collision_box(pos, shape_box));
                    });
                    if !intersects {
                        boxes.truncate(start);
                    } else if boxes.len() >= MAX_DEBUG_CAMERA_BOXES {
                        break 'outer;
                    }
                }
            }
        }
        boxes
    }

    pub(super) fn debug_camera_entity_collision_boxes_for_query(
        &self,
        camera: EntityCollisionQuery,
        radius: f64,
    ) -> Vec<WorldCollisionBox> {
        let query = camera.bounds.inflated(radius + COLLISION_EPSILON);
        self.entity_pick_targets_in_aabb_at_partial_tick(query.min, query.max, 1.0)
            .into_iter()
            .filter_map(|target| {
                if camera.source_entity_id == Some(target.entity_id)
                    || self.debug_entities_share_root_vehicle(
                        camera.source_entity_id,
                        target.entity_id,
                    )
                {
                    return None;
                }
                let target_type = self.entity_type_id(target.entity_id)?;
                let target_box = WorldCollisionBox {
                    min: [
                        target.position.x + f64::from(target.bounds.min[0]),
                        target.position.y + f64::from(target.bounds.min[1]),
                        target.position.z + f64::from(target.bounds.min[2]),
                    ],
                    max: [
                        target.position.x + f64::from(target.bounds.max[0]),
                        target.position.y + f64::from(target.bounds.max[1]),
                        target.position.z + f64::from(target.bounds.max[2]),
                    ],
                };
                if !self.debug_entities_can_collide(
                    camera,
                    target.entity_id,
                    target_type,
                    [target.position.x, target.position.y, target.position.z],
                    target_box,
                ) || !world_collision_boxes_intersect(target_box, query)
                {
                    return None;
                }
                Some(target_box)
            })
            .collect()
    }

    pub(super) fn debug_entities_can_collide(
        &self,
        camera: EntityCollisionQuery,
        target_id: i32,
        target_type: i32,
        target_position: [f64; 3],
        target_box: WorldCollisionBox,
    ) -> bool {
        let can_be_collided_with =
            self.debug_entity_can_be_collided_with(camera, target_id, target_type, target_box);
        if !is_vanilla_boat_type(camera.source_entity_type_id)
            && !is_vanilla_minecart_type(camera.source_entity_type_id)
        {
            return can_be_collided_with;
        }
        can_be_collided_with
            || self.debug_entity_is_pushable(target_id, target_type, target_position)
    }

    pub(super) fn debug_entity_can_be_collided_with(
        &self,
        camera: EntityCollisionQuery,
        target_id: i32,
        target_type: i32,
        target_box: WorldCollisionBox,
    ) -> bool {
        match vanilla_entity_metadata_type_collidability_kind(target_type) {
            Some(VanillaEntityCollidabilityKind::Always) => return true,
            Some(VanillaEntityCollidabilityKind::Alive) => {
                return self
                    .entities
                    .living_entity_health(target_id)
                    .is_some_and(|health| health > 0.0);
            }
            Some(VanillaEntityCollidabilityKind::HappyGhast) => {}
            Some(VanillaEntityCollidabilityKind::Never) | None => return false,
        }
        if self.entities.is_baby(target_id).unwrap_or(false)
            || !self
                .entities
                .living_entity_health(target_id)
                .is_some_and(|health| health > 0.0)
        {
            return false;
        }

        let player_is_above = camera.source_entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            && camera.source_position[1] >= target_box.max[1];
        let happy_ghast_vehicle_pair = camera.source_entity_type_id
            == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            && self
                .entities
                .mount(target_id)
                .is_some_and(|mount| !mount.passengers.is_empty());
        let stays_still = vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(
            target_type,
        )
        .is_some_and(|accessors| {
            self.entities
                .metadata_bool_for_entity(
                    target_id,
                    accessors.stays_still_data_id,
                    accessors.stays_still_default,
                )
                .unwrap_or(accessors.stays_still_default)
        });
        player_is_above || happy_ghast_vehicle_pair || stays_still
    }

    pub(crate) fn entity_is_pushable(
        &self,
        target_id: i32,
        target_type: i32,
        target_position: [f64; 3],
    ) -> bool {
        let Some(kind) = vanilla_entity_metadata_type_pushability_kind(target_type).or_else(|| {
            crate::entities::content_facts::mod_entity_spec(target_type, self.content_registry())
                .filter(|spec| spec.is_living)
                .map(|_| VanillaEntityPushabilityKind::Living)
        }) else {
            return false;
        };
        match kind {
            VanillaEntityPushabilityKind::Never => return false,
            VanillaEntityPushabilityKind::Always => return true,
            VanillaEntityPushabilityKind::NotVehicle => {
                return self
                    .entities
                    .mount(target_id)
                    .is_some_and(|mount| mount.passengers.is_empty());
            }
            VanillaEntityPushabilityKind::Living
            | VanillaEntityPushabilityKind::LivingCanMove
            | VanillaEntityPushabilityKind::LivingNotDiggingOrEmerging => {}
        }
        if target_type == VANILLA_ENTITY_TYPE_PLAYER_ID
            && self
                .entities
                .identity(target_id)
                .and_then(|identity| self.player_info_entry(identity.uuid))
                .is_some_and(|player| player.is_spectator())
        {
            return false;
        }
        if !self
            .entities
            .living_entity_health(target_id)
            .is_some_and(|health| health > 0.0)
            || self.debug_living_entity_on_climbable(target_id, target_position)
        {
            return false;
        }
        if kind == VanillaEntityPushabilityKind::LivingNotDiggingOrEmerging
            && self.entities.pose(target_id).is_some_and(|pose| {
                matches!(
                    pose,
                    VANILLA_ENTITY_POSE_EMERGING_ID | VANILLA_ENTITY_POSE_DIGGING_ID
                )
            })
        {
            return false;
        }
        if kind == VanillaEntityPushabilityKind::LivingCanMove {
            let can_move =
                vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type(target_type)
                    .is_some_and(|(data_id, default)| {
                        self.entities
                            .metadata_bool_for_entity(target_id, data_id, default)
                            .unwrap_or(default)
                    });
            if !can_move {
                return false;
            }
        }
        true
    }

    pub(super) fn debug_entity_is_pushable(
        &self,
        target_id: i32,
        target_type: i32,
        target_position: [f64; 3],
    ) -> bool {
        self.entity_is_pushable(target_id, target_type, target_position)
    }

    pub(super) fn debug_living_entity_on_climbable(
        &self,
        entity_id: i32,
        position: [f64; 3],
    ) -> bool {
        let pos = BlockPos {
            x: block_floor(position[0]),
            y: block_floor(position[1]),
            z: block_floor(position[2]),
        };
        let Some(block) = self.block_view(pos) else {
            return false;
        };
        let Some(block_name) = block.block_name else {
            return false;
        };
        if debug_climbable_block_name(block_name) {
            return !(self.entities.is_fall_flying(entity_id).unwrap_or(false)
                && debug_can_glide_through_block_name(block_name));
        }
        if !block_name_is_trapdoor(block_name)
            || block.block_properties.get("open").map(String::as_str) != Some("true")
        {
            return false;
        }
        let Some(below) = self.block_view(BlockPos {
            y: pos.y.saturating_sub(1),
            ..pos
        }) else {
            return false;
        };
        below.block_name == Some("minecraft:ladder")
            && below.block_properties.get("facing") == block.block_properties.get("facing")
    }

    pub(super) fn debug_entities_share_root_vehicle(
        &self,
        source_id: Option<i32>,
        target_id: i32,
    ) -> bool {
        let Some(source_id) = source_id else {
            return false;
        };
        matches!(
            (
                self.debug_entity_root_vehicle_id(source_id),
                self.debug_entity_root_vehicle_id(target_id),
            ),
            (Some(source_root), Some(target_root)) if source_root == target_root
        )
    }

    pub(super) fn debug_entity_root_vehicle_id(&self, entity_id: i32) -> Option<i32> {
        let mut root = entity_id;
        for _ in 0..=self.entity_count() {
            let mount = self.entities.mount(root)?;
            let Some(vehicle_id) = mount.vehicle_id else {
                return Some(root);
            };
            root = vehicle_id;
        }
        None
    }

    pub(super) fn debug_camera_collision_query(&self) -> Option<EntityCollisionQuery> {
        let camera = self.local_player().camera;
        if !camera.follows_player {
            let camera_id = camera.entity_id?;
            let pose = self.probe_entity_camera_pose(camera_id)?;
            let entity_type_id = self.entity_type_id(camera_id)?;
            let bounds = self.entities.pick_bounds(camera_id)?;
            let world_bounds = WorldCollisionBox {
                min: [
                    pose.position.x + f64::from(bounds.min[0]),
                    pose.position.y + f64::from(bounds.min[1]),
                    pose.position.z + f64::from(bounds.min[2]),
                ],
                max: [
                    pose.position.x + f64::from(bounds.max[0]),
                    pose.position.y + f64::from(bounds.max[1]),
                    pose.position.z + f64::from(bounds.max[2]),
                ],
            };
            return Some(EntityCollisionQuery {
                source_entity_id: Some(camera_id),
                source_entity_type_id: entity_type_id,
                source_position: [pose.position.x, pose.position.y, pose.position.z],
                bounds: world_bounds,
                block_context: LocalPlayerCollisionContext::new(world_bounds.min[1], false),
            });
        }

        let pose = self.local_player_pose()?;
        Some(EntityCollisionQuery {
            source_entity_id: self.local_player_id(),
            source_entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            source_position: [pose.position.x, pose.position.y, pose.position.z],
            bounds: self.local_player_bounds(pose),
            block_context: LocalPlayerCollisionContext::for_pose(self, pose, false),
        })
    }
}
