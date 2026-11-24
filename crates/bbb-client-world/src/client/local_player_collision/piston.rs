use super::*;

#[derive(Debug, Clone, Copy)]
pub(super) enum BlockClipShape {
    Collision,
    Visual,
    FallDamageResetting,
}

pub(super) fn block_hit_scale(
    world: &WorldStore,
    from: [f64; 3],
    to: [f64; 3],
    context: LocalPlayerCollisionContext,
    clip_shape: BlockClipShape,
) -> Option<BlockShapeHit> {
    const MAX_DDA_STEPS: usize = 16_384;
    if !from.iter().chain(&to).all(|value| value.is_finite()) {
        return None;
    }
    // `BlockGetter.traverseBlocks` extrapolates both endpoints by -1e-7 away
    // from the opposite endpoint before selecting the first cell.
    let nudged_to: [f64; 3] =
        std::array::from_fn(|axis| to[axis] + (from[axis] - to[axis]) * -1.0e-7);
    let nudged_from: [f64; 3] =
        std::array::from_fn(|axis| from[axis] + (to[axis] - from[axis]) * -1.0e-7);
    if !nudged_from
        .iter()
        .chain(&nudged_to)
        .all(|value| value.is_finite())
    {
        return None;
    }
    let mut current = [
        block_floor(nudged_from[0]),
        block_floor(nudged_from[1]),
        block_floor(nudged_from[2]),
    ];
    if let Some(scale) = block_shape_hit_scale(world, current, from, to, context, clip_shape) {
        return Some(scale);
    }
    let delta = std::array::from_fn::<_, 3, _>(|axis| nudged_to[axis] - nudged_from[axis]);
    let step = delta.map(|value| {
        if value > 0.0 {
            1
        } else if value < 0.0 {
            -1
        } else {
            0
        }
    });
    let t_delta = std::array::from_fn::<_, 3, _>(|axis| {
        if step[axis] == 0 {
            f64::MAX
        } else {
            f64::from(step[axis]) / delta[axis]
        }
    });
    let mut t = std::array::from_fn::<_, 3, _>(|axis| {
        t_delta[axis]
            * if step[axis] > 0 {
                1.0 - (nudged_from[axis] - nudged_from[axis].floor())
            } else {
                nudged_from[axis] - nudged_from[axis].floor()
            }
    });
    for _ in 0..MAX_DDA_STEPS {
        if t[0] > 1.0 && t[1] > 1.0 && t[2] > 1.0 {
            break;
        }
        let axis = if t[0] < t[1] {
            if t[0] < t[2] {
                0
            } else {
                2
            }
        } else if t[1] < t[2] {
            1
        } else {
            2
        };
        let Some(next_cell) = current[axis].checked_add(step[axis]) else {
            break;
        };
        let next_t = t[axis] + t_delta[axis];
        if !next_t.is_finite() || next_t <= t[axis] {
            break;
        }
        current[axis] = next_cell;
        t[axis] = next_t;
        if let Some(scale) = block_shape_hit_scale(world, current, from, to, context, clip_shape) {
            return Some(scale);
        }
    }
    None
}

pub(super) fn block_shape_hit_scale(
    world: &WorldStore,
    cell: [i32; 3],
    from: [f64; 3],
    to: [f64; 3],
    context: LocalPlayerCollisionContext,
    clip_shape: BlockClipShape,
) -> Option<BlockShapeHit> {
    let pos = BlockPos {
        x: cell[0],
        y: cell[1],
        z: cell[2],
    };
    let block = world.block_view(pos)?;
    let shape = match clip_shape {
        BlockClipShape::Collision => {
            block_collision_shape_with_context(world, &block, pos, context)
        }
        BlockClipShape::Visual => block_visual_shape_with_context(world, &block, pos, context),
        BlockClipShape::FallDamageResetting => fall_damage_resetting_shape(world, &block, pos),
    }?;
    let delta = std::array::from_fn::<_, 3, _>(|axis| to[axis] - from[axis]);
    let test_point = std::array::from_fn::<_, 3, _>(|axis| from[axis] + delta[axis] * 0.001);
    shape
        .boxes()
        .filter_map(|shape_box| {
            let collision_box = world_collision_box(pos, shape_box);
            if world_collision_box_contains(collision_box, test_point) {
                Some(0.001)
            } else {
                segment_box_entry_scale(from, to, collision_box)
            }
        })
        .min_by(f64::total_cmp)
        .map(|scale| BlockShapeHit {
            scale,
            pos,
            block_state_id: block.block_state_id,
        })
}

pub(super) fn fall_damage_resetting_shape(
    world: &WorldStore,
    block: &BlockView,
    pos: BlockPos,
) -> Option<BlockCollisionShape> {
    let block_name = block.block_name;
    let synchronized_members = world
        .registry_tags("minecraft:block")
        .and_then(|tags| tags.tags.get(FALL_DAMAGE_RESETTING_BLOCK_TAG));
    let block_resets = synchronized_members.map_or_else(
        || block_name.is_some_and(block_name_resets_fall_damage),
        |members| {
            block_name
                .and_then(|name| {
                    world.resolved_registry_entry_by_resource_id("minecraft:block", name)
                })
                .is_some_and(|entry| members.contains(&entry.holder_id))
        },
    ) || block_name
        .is_some_and(|name| matches!(name, "minecraft:end_gateway" | "minecraft:end_portal"));
    if block_resets {
        return Some(BlockCollisionShape::single(WorldCollisionBox::FULL));
    }

    let fluid = block
        .fluid
        .filter(|fluid| fluid.kind == TerrainFluidKind::Water)?;
    let height = crate::fluid::fluid_height_at(world, pos, fluid);
    (height > 0.0).then(|| {
        BlockCollisionShape::single(WorldCollisionBox {
            min: WorldCollisionBox::FULL.min,
            max: [
                WorldCollisionBox::FULL.max[0],
                height,
                WorldCollisionBox::FULL.max[2],
            ],
        })
    })
}

pub(super) fn world_collision_box_contains(bounds: WorldCollisionBox, point: [f64; 3]) -> bool {
    (0..3).all(|axis| bounds.min[axis] <= point[axis] && point[axis] < bounds.max[axis])
}

pub(super) fn world_collision_boxes_intersect(
    left: WorldCollisionBox,
    right: WorldCollisionBox,
) -> bool {
    left.max[0] > right.min[0]
        && left.min[0] < right.max[0]
        && left.max[1] > right.min[1]
        && left.min[1] < right.max[1]
        && left.max[2] > right.min[2]
        && left.min[2] < right.max[2]
}

pub(super) fn segment_box_entry_scale(
    from: [f64; 3],
    to: [f64; 3],
    bounds: WorldCollisionBox,
) -> Option<f64> {
    let delta = std::array::from_fn::<_, 3, _>(|axis| to[axis] - from[axis]);
    let mut nearest = 1.0;
    let mut hit = false;
    for axis in 0..3 {
        let (plane, range_a, range_b) = if delta[axis] > 1.0e-7 {
            (bounds.min[axis], bounds.min, bounds.max)
        } else if delta[axis] < -1.0e-7 {
            (bounds.max[axis], bounds.min, bounds.max)
        } else {
            continue;
        };
        let scale = (plane - from[axis]) / delta[axis];
        if !(0.0 < scale && scale < nearest) {
            continue;
        }
        let other = [(axis + 1) % 3, (axis + 2) % 3];
        if other.iter().all(|&candidate| {
            let point = from[candidate] + scale * delta[candidate];
            range_a[candidate] - 1.0e-7 < point && point < range_b[candidate] + 1.0e-7
        }) {
            nearest = scale;
            hit = true;
        }
    }
    hit.then_some(nearest)
}

pub(super) fn particle_collision_input_is_finite(position: [f64; 3], movement: [f64; 3]) -> bool {
    position.iter().all(|value| value.is_finite()) && movement.iter().all(|value| value.is_finite())
}

pub(super) fn particle_collision_bounds(
    position: [f64; 3],
    half_width: f64,
    height: f64,
) -> WorldCollisionBox {
    let half_width = half_width.max(0.0);
    let height = height.max(0.0);
    WorldCollisionBox {
        min: [
            position[0] - half_width,
            position[1],
            position[2] - half_width,
        ],
        max: [
            position[0] + half_width,
            position[1] + height,
            position[2] + half_width,
        ],
    }
}

pub(super) fn particle_collision_axis_order(movement: [f64; 3]) -> [CollisionAxis; 3] {
    bbb_facts::collision::axis_step_order(movement[0], movement[2])
}

pub(super) fn clipped_particle_axis_against_box(
    bounds: WorldCollisionBox,
    axis: CollisionAxis,
    _requested: f64,
    current_clipped: f64,
    pos: BlockPos,
    shape_box: WorldCollisionBox,
) -> f64 {
    bbb_facts::collision::collide_axis(
        axis,
        bounds,
        &[world_collision_box(pos, shape_box)],
        current_clipped,
    )
}

pub(super) fn block_collides_with_local_player_bounds(
    world: &WorldStore,
    block: &BlockView,
    pos: BlockPos,
    bounds: WorldCollisionBox,
    context: LocalPlayerCollisionContext,
) -> bool {
    if let Some(shape) = block_collision_shape_with_context(world, block, pos, context) {
        return bounds_intersects_block_shape(bounds, pos, shape);
    }
    false
}

pub(super) fn block_collision_shape_with_context(
    world: &WorldStore,
    block: &BlockView,
    pos: BlockPos,
    context: LocalPlayerCollisionContext,
) -> Option<BlockCollisionShape> {
    if block.block_name == Some("minecraft:moving_piston") {
        return moving_piston_collision_shape(world, block, pos, context, false);
    }
    if block.block_name.is_some_and(is_shulker_box_block_name) {
        return shulker_box_collision_shape(world, block, pos);
    }
    if context.can_stand_on_lava
        && block.fluid.is_some_and(|fluid| {
            fluid.kind == TerrainFluidKind::Lava && fluid.amount == 8 && !fluid.falling
        })
        && context.is_above(pos, 0.5)
        && !world
            .block_view(BlockPos {
                y: pos.y.saturating_add(1),
                ..pos
            })
            .and_then(|above| above.fluid)
            .is_some_and(|fluid| fluid.kind == TerrainFluidKind::Lava)
    {
        return Some(BlockCollisionShape::single(WorldCollisionBox::BOTTOM_SLAB));
    }
    if block.block_name == Some("minecraft:powder_snow") {
        return powder_snow_collision_shape(pos, context);
    }
    if block.block_name == Some("minecraft:scaffolding") {
        return scaffolding_collision_shape(block.block_properties, pos, context);
    }
    block_collision_shape_with_registry(block, pos, world.content_registry())
}

pub(super) fn moving_piston_collision_shape(
    world: &WorldStore,
    template: &BlockView,
    pos: BlockPos,
    context: LocalPlayerCollisionContext,
    collision_related_only: bool,
) -> Option<BlockCollisionShape> {
    let moving = world.moving_piston(pos)?;
    let mut shape = BlockCollisionShapeBuilder::new();
    if !collision_related_only && !moving.extending && moving.is_source {
        // `PistonMovingBlockEntity.getCollisionShape` extends the source base.
        let extended_state_id = bbb_protocol::block_state_types::vanilla_block_state_with_property(
            moving.moved_block_state_id,
            "extended",
            "true",
        )
        .unwrap_or(moving.moved_block_state_id);
        let base = block_probe_for_state(world, template, extended_state_id)?;
        if let Some(base_shape) = block_collision_shape_with_context(world, &base, pos, context) {
            shape.extend(base_shape);
        }
    }

    let state_id = if !collision_related_only && moving.is_source {
        moving_piston_source_head_state_id(world, moving)?
    } else {
        moving_piston_collision_related_state_id(world, moving)?
    };
    let moved = block_probe_for_state(world, template, state_id)?;
    let moved_shape = block_collision_shape_with_context(world, &moved, pos, context)?;
    let [dx, dy, dz] = moving.direction.step();
    let extended = f64::from(moving.extended_progress(moving.progress));
    shape.extend(moved_shape.offset(
        f64::from(dx) * extended,
        f64::from(dy) * extended,
        f64::from(dz) * extended,
    ));
    Some(shape.build())
}

pub(super) fn moving_piston_source_head_state_id(
    world: &WorldStore,
    moving: &crate::MovingPistonState,
) -> Option<i32> {
    let moved = world.registries.block_state(moving.moved_block_state_id)?;
    let sticky = moved.name == "minecraft:sticky_piston"
        || moved
            .properties
            .get("type")
            .is_some_and(|kind| kind == "sticky");
    let short = moving.extending != (1.0 - moving.progress < 0.25);
    let properties = BTreeMap::from([
        ("facing".to_string(), moving.direction.as_name().to_string()),
        ("short".to_string(), short.to_string()),
        (
            "type".to_string(),
            if sticky { "sticky" } else { "normal" }.to_string(),
        ),
    ]);
    world
        .registries
        .block_state_id_by_name_and_properties("minecraft:piston_head", &properties)
}

pub(super) fn moving_piston_collision_related_state_id(
    world: &WorldStore,
    moving: &crate::MovingPistonState,
) -> Option<i32> {
    if !moving.extending && moving.is_source {
        let moved = world.registries.block_state(moving.moved_block_state_id)?;
        let facing = moved
            .properties
            .get("facing")
            .map(String::as_str)
            .unwrap_or_else(|| moving.direction.as_name());
        let properties = BTreeMap::from([
            ("facing".to_string(), facing.to_string()),
            ("short".to_string(), (moving.progress > 0.25).to_string()),
            (
                "type".to_string(),
                if moved.name == "minecraft:sticky_piston" {
                    "sticky".to_string()
                } else {
                    "normal".to_string()
                },
            ),
        ]);
        world
            .registries
            .block_state_id_by_name_and_properties("minecraft:piston_head", &properties)
    } else {
        Some(moving.moved_block_state_id)
    }
}

pub(super) fn block_probe_for_state(
    _world: &WorldStore,
    template: &BlockView,
    state_id: i32,
) -> Option<BlockView> {
    vanilla_block_state_name(state_id)?;
    Some(BlockView {
        fluid: None,
        ..BlockView::of_state(template.pos, state_id)
    })
}

impl WorldStore {
    /// Mirrors `Level.isUnobstructed(state, pos,
    /// CollisionContext.placementContext(player))` for a prospective built-in
    /// state. Placement context makes powder snow and scaffolding collision
    /// shapes empty; every other supported state uses the shared canonical
    /// collision-shape projection.
    pub(crate) fn local_block_state_placement_is_unobstructed(
        &self,
        pos: BlockPos,
        block_state_id: i32,
    ) -> Option<bool> {
        let template = self.block_view(pos)?;
        let block = block_probe_for_state(self, &template, block_state_id)?;
        if matches!(
            block.block_name,
            Some("minecraft:powder_snow" | "minecraft:scaffolding")
        ) {
            return Some(true);
        }
        let Some(shape) = block_collision_shape_with_registry(&block, pos, self.content_registry())
        else {
            return Some(true);
        };
        for shape_box in shape.boxes() {
            let bounds = world_collision_box(pos, shape_box);
            if !self.local_block_placement_is_unobstructed(bounds.min, bounds.max)? {
                return Some(false);
            }
        }
        Some(true)
    }

    /// Mirrors `Level.isUnobstructed(state, pos, CollisionContext.empty())`
    /// for placement paths such as `StandingAndWallBlockItem` that explicitly
    /// discard the placing player's collision context.
    pub(crate) fn local_block_state_empty_context_is_unobstructed(
        &self,
        pos: BlockPos,
        block_state_id: i32,
    ) -> Option<bool> {
        let template = self.block_view(pos)?;
        let block = block_probe_for_state(self, &template, block_state_id)?;
        let Some(shape) = block_collision_shape_empty_context(&block, pos) else {
            return Some(true);
        };
        for shape_box in shape.boxes() {
            let bounds = world_collision_box(pos, shape_box);
            if !self.local_block_placement_is_unobstructed(bounds.min, bounds.max)? {
                return Some(false);
            }
        }
        Some(true)
    }

    pub(crate) fn move_entities_for_piston_tick(
        &mut self,
        moving: &crate::MovingPistonState,
        new_progress: f32,
    ) {
        let delta_progress = f64::from(new_progress - moving.progress);
        if delta_progress <= 0.0 {
            return;
        }
        let Some(template) = self.block_view(moving.pos) else {
            return;
        };
        let context = LocalPlayerCollisionContext::new(f64::NEG_INFINITY, false);
        let Some(shape) = moving_piston_collision_shape(self, &template, moving.pos, context, true)
        else {
            return;
        };
        let movement = moving.movement_direction();
        let movement_step = movement.step();
        let shape_boxes = shape
            .boxes()
            .map(|shape_box| world_collision_box(moving.pos, shape_box))
            .collect::<Vec<_>>();
        let cause_bounce =
            vanilla_block_state_name(moving.moved_block_state_id) == Some("minecraft:slime_block");

        let candidates = piston_entity_candidates(self);
        for candidate in &candidates {
            if piston_entity_ignores_movement(candidate.entity_type_id) {
                continue;
            }
            let mut delta = 0.0_f64;
            for shape_box in &shape_boxes {
                let movement_area = swept_piston_box(*shape_box, movement_step, delta_progress);
                if world_collision_boxes_intersect(movement_area, candidate.bounds) {
                    delta = delta.max(piston_separation(movement_area, movement, candidate.bounds));
                    if delta >= delta_progress {
                        break;
                    }
                }
            }
            if delta > 0.0 {
                let movement_delta = delta.min(delta_progress) + 0.01;
                move_piston_entity(
                    self,
                    candidate.entity_id,
                    movement,
                    movement_delta,
                    cause_bounce,
                );
            }
        }

        if !matches!(movement, PistonDirection::Up | PistonDirection::Down)
            && vanilla_block_state_name(moving.moved_block_state_id)
                == Some("minecraft:honey_block")
        {
            let sticky_top = shape_boxes
                .iter()
                .map(|shape_box| shape_box.max[1])
                .fold(f64::NEG_INFINITY, f64::max);
            if sticky_top.is_finite() {
                let [direction_x, direction_y, direction_z] = moving.direction.step();
                let extended = f64::from(moving.extended_progress(moving.progress));
                let offset_x = f64::from(direction_x) * extended;
                let offset_y = f64::from(direction_y) * extended;
                let offset_z = f64::from(direction_z) * extended;
                let sticky = WorldCollisionBox {
                    min: [
                        f64::from(moving.pos.x) + offset_x,
                        sticky_top,
                        f64::from(moving.pos.z) + offset_z,
                    ],
                    max: [
                        f64::from(moving.pos.x) + offset_x + 1.0,
                        f64::from(moving.pos.y) + offset_y + 1.500001,
                        f64::from(moving.pos.z) + offset_z + 1.0,
                    ],
                };
                for candidate in candidates {
                    if candidate.on_ground
                        && !piston_entity_ignores_movement(candidate.entity_type_id)
                        && world_collision_boxes_intersect(sticky, candidate.bounds)
                    {
                        move_piston_entity(
                            self,
                            candidate.entity_id,
                            movement,
                            delta_progress,
                            false,
                        );
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PistonEntityCandidate {
    pub(super) entity_id: Option<i32>,
    pub(super) entity_type_id: i32,
    pub(super) bounds: WorldCollisionBox,
    pub(super) on_ground: bool,
}

pub(super) fn piston_entity_candidates(world: &WorldStore) -> Vec<PistonEntityCandidate> {
    let mut candidates = Vec::new();
    if let Some(pose) = world.local_player_pose() {
        let bounds = world.local_player_bounds(pose);
        candidates.push(PistonEntityCandidate {
            entity_id: None,
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            bounds,
            on_ground: pose.on_ground,
        });
    }
    let local_id = world.local_player_id();
    for transform in world.entity_transforms() {
        if local_id == Some(transform.id) {
            continue;
        }
        let Some(relative) = world.probe_entity_pick_bounds(transform.id) else {
            continue;
        };
        candidates.push(PistonEntityCandidate {
            entity_id: Some(transform.id),
            entity_type_id: transform.entity_type_id,
            bounds: WorldCollisionBox {
                min: [
                    transform.position.x + f64::from(relative.min[0]),
                    transform.position.y + f64::from(relative.min[1]),
                    transform.position.z + f64::from(relative.min[2]),
                ],
                max: [
                    transform.position.x + f64::from(relative.max[0]),
                    transform.position.y + f64::from(relative.max[1]),
                    transform.position.z + f64::from(relative.max[2]),
                ],
            },
            on_ground: transform.on_ground.unwrap_or(false),
        });
    }
    candidates
}

pub(super) fn piston_entity_ignores_movement(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID
            | VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID
            | VANILLA_ENTITY_TYPE_INTERACTION_ID
            | VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID
            | VANILLA_ENTITY_TYPE_MARKER_ID
            | VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID
            | VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID
    )
}

pub(super) fn swept_piston_box(
    bounds: WorldCollisionBox,
    movement: [i32; 3],
    delta: f64,
) -> WorldCollisionBox {
    let moved = WorldCollisionBox {
        min: std::array::from_fn(|axis| bounds.min[axis] + f64::from(movement[axis]) * delta),
        max: std::array::from_fn(|axis| bounds.max[axis] + f64::from(movement[axis]) * delta),
    };
    WorldCollisionBox {
        min: std::array::from_fn(|axis| bounds.min[axis].min(moved.min[axis])),
        max: std::array::from_fn(|axis| bounds.max[axis].max(moved.max[axis])),
    }
}

pub(super) fn piston_separation(
    piston: WorldCollisionBox,
    movement: PistonDirection,
    entity: WorldCollisionBox,
) -> f64 {
    match movement {
        PistonDirection::East => piston.max[0] - entity.min[0],
        PistonDirection::West => entity.max[0] - piston.min[0],
        PistonDirection::Up => piston.max[1] - entity.min[1],
        PistonDirection::Down => entity.max[1] - piston.min[1],
        PistonDirection::South => piston.max[2] - entity.min[2],
        PistonDirection::North => entity.max[2] - piston.min[2],
    }
}

pub(super) fn move_piston_entity(
    world: &mut WorldStore,
    entity_id: Option<i32>,
    movement: PistonDirection,
    delta: f64,
    cause_bounce: bool,
) {
    let [dx, dy, dz] = movement.step();
    if let Some(entity_id) = entity_id {
        let _ = world.entities.with_transform_mut(entity_id, |transform| {
            transform.position.x += f64::from(dx) * delta;
            transform.position.y += f64::from(dy) * delta;
            transform.position.z += f64::from(dz) * delta;
            if cause_bounce {
                if dx != 0 {
                    transform.delta_movement.x = f64::from(dx);
                } else if dy != 0 {
                    transform.delta_movement.y = f64::from(dy);
                } else {
                    transform.delta_movement.z = f64::from(dz);
                }
            }
        });
    } else if let Some(pose) = world.local_player.pose.as_mut() {
        pose.position.x += f64::from(dx) * delta;
        pose.position.y += f64::from(dy) * delta;
        pose.position.z += f64::from(dz) * delta;
        if cause_bounce {
            if dx != 0 {
                pose.delta_movement.x = f64::from(dx);
            } else if dy != 0 {
                pose.delta_movement.y = f64::from(dy);
            } else {
                pose.delta_movement.z = f64::from(dz);
            }
        }
    }
}
