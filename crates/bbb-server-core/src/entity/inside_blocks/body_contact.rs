//! Shared swept body/block intersections; callbacks retain immediate side effects.

use super::{
    collided_along_vector, component, distance_to_sqr, for_each_block_intersected, subtract,
    with_component_added, EntityMovement, INSIDE_EPSILON, MAX_MOVEMENT_ITERATIONS,
};
use crate::chunk::ChunkStore;
use crate::entity::environment;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{self as block_types, vanilla_block_type};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::fluid::FluidFamily;
use bbb_server_types::geometry::{axis_step_order, Aabb, Vec3Ops};
use rustc_hash::FxHashSet;
use std::collections::VecDeque;

/// Entity.java: merge the oldest pair before admitting movement 101.
pub fn record_body_movement(movements: &mut VecDeque<EntityMovement>, movement: EntityMovement) {
    if movements.len() >= 100 {
        let first = movements
            .pop_front()
            .expect("capped recorder has a first movement");
        let second = movements
            .pop_front()
            .expect("capped recorder has a second movement");
        movements.push_front(EntityMovement::direct(first.from, second.to));
    }
    movements.push_back(movement);
}

/// Entity.java: preserve the collision path and append the final snap.
pub fn take_body_effect_movements(
    pending: &mut VecDeque<EntityMovement>,
    final_movements: &mut Vec<EntityMovement>,
    old_position: Vec3d,
    position: Vec3d,
) {
    final_movements.clear();
    final_movements.extend(pending.drain(..));
    if let Some(last) = final_movements.last() {
        if distance_to_sqr(last.to, position) > f64::from(9.999_999_4e-11_f32) {
            final_movements.push(EntityMovement::direct(last.to, position));
        }
    } else {
        final_movements.push(EntityMovement::direct(old_position, position));
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BlockContactGeometry {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct BodyBlockContact {
    pub pos: BlockPos,
    pub state: i32,
    pub environment_kind: super::effects::EnvironmentBlockKind,
    pub resource_id: &'static str,
    pub step: i32,
    pub precise: bool,
    pub inside_block: bool,
    pub fluid: Option<FluidFamily>,
}

// PowderSnowBlock.java:139-145.
pub(crate) fn can_walk_on_powder_snow(
    entity_type: &str,
    feet_item_id: Option<i32>,
    mut membership: impl FnMut(&str, &str, &str) -> bool,
) -> bool {
    membership(
        "minecraft:entity_type",
        "minecraft:powder_snow_walkable_mobs",
        entity_type,
    ) || feet_item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_LEATHER_BOOTS_ID)
}

/// PowderSnowBlock.java; EntityCollisionContext.java.
pub fn powder_snow_body_collision(
    fall_distance: f64,
    falling_block: bool,
    can_walk: Option<bool>,
    entity_bottom: f64,
    descending: bool,
    pos: BlockPos,
) -> Option<Vec<Aabb>> {
    let height = if fall_distance > 2.5 {
        f64::from(0.9_f32)
    } else if falling_block
        || (can_walk?
            && entity_bottom > f64::from(pos.y) + 1.0 - f64::from(1.0e-5_f32)
            && !descending)
    {
        1.0
    } else {
        return Some(Vec::new());
    };
    Some(vec![Aabb {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: height,
        max_z: 1.0,
    }])
}

pub(crate) struct BodyContactCursor {
    geometry: BlockContactGeometry,
    movements: VecDeque<EntityMovement>,
    segments: VecDeque<(Vec3d, Vec3d)>,
    movement_end: Option<Vec3d>,
    remaining: i32,
    segment: Option<ContactSegment>,
    visited: FxHashSet<(i32, i32, i32)>,
}

struct ContactSegment {
    from: Vec3d,
    to: Vec3d,
    target_box: Aabb,
    moved_far: bool,
    positions: VecDeque<(BlockPos, i32)>,
    last_iteration: i32,
}

impl BodyContactCursor {
    pub(crate) fn new(geometry: BlockContactGeometry, movements: Vec<EntityMovement>) -> Self {
        Self {
            geometry,
            movements: movements.into(),
            segments: VecDeque::new(),
            movement_end: None,
            remaining: MAX_MOVEMENT_ITERATIONS,
            segment: None,
            visited: FxHashSet::default(),
        }
    }

    /// Entity.java: all movements share the visited set and step collector;
    /// the caller stops pulling once the body dies.
    pub(crate) fn next_contact(
        &mut self,
        chunks: &ChunkStore,
        fall_distance: f64,
    ) -> Option<Option<BodyBlockContact>> {
        loop {
            if let Some(segment) = &mut self.segment {
                if let Some((pos, iteration)) = segment.positions.pop_front() {
                    segment.last_iteration = iteration;
                    let from = segment.from;
                    let to = segment.to;
                    if let Some(contact) =
                        self.contact_at(chunks, fall_distance, pos, iteration, from, to)?
                    {
                        return Some(Some(contact));
                    }
                    continue;
                }
                self.remaining -= segment.last_iteration + 1;
                self.segment = None;
            }
            if let Some((from, to)) = self.segments.pop_front() {
                self.start_segment(from, to, self.remaining);
                continue;
            }
            if let Some(end) = self.movement_end.take() {
                if self.remaining <= 0 {
                    self.start_segment(end, end, 1);
                    continue;
                }
            }
            let Some(movement) = self.movements.pop_front() else {
                return Some(None);
            };
            self.remaining = MAX_MOVEMENT_ITERATIONS;
            self.movement_end = Some(movement.to);
            let delta = subtract(movement.to, movement.from);
            if let Some(original) = movement
                .axis_dependent_original
                .filter(|_| delta.length_sqr() > 0.0)
            {
                let mut from = movement.from;
                for axis in axis_step_order(original) {
                    let axis_move = component(delta, axis);
                    if axis_move != 0.0 {
                        let to = with_component_added(from, axis, axis_move);
                        self.segments.push_back((from, to));
                        from = to;
                    }
                }
            } else {
                self.segments.push_back((movement.from, movement.to));
            }
        }
    }

    fn start_segment(&mut self, from: Vec3d, to: Vec3d, max_iterations: i32) {
        let target_box = Aabb::of_entity(to, self.geometry.width, self.geometry.height).inflate(
            -INSIDE_EPSILON,
            -INSIDE_EPSILON,
            -INSIDE_EPSILON,
        );
        let mut positions = VecDeque::new();
        // Only geometry is retained; the next contact reads the current Level after a hurt pause.
        for_each_block_intersected(from, to, target_box, |pos, iteration| {
            if iteration >= max_iterations {
                return false;
            }
            positions.push_back((pos, iteration));
            true
        });
        self.segment = Some(ContactSegment {
            from,
            to,
            target_box,
            moved_far: distance_to_sqr(from, to) > 0.999_990_000_000_252_6_f64.powi(2),
            positions,
            last_iteration: 0,
        });
    }

    fn contact_at(
        &mut self,
        chunks: &ChunkStore,
        fall_distance: f64,
        pos: BlockPos,
        iteration: i32,
        from: Vec3d,
        to: Vec3d,
    ) -> Option<Option<BodyBlockContact>> {
        let segment = self.segment.as_ref()?;
        let state = crate::world::stored_block_state(chunks, pos);
        let Some(block_type_id) = vanilla_block_state_block_type_id(state) else {
            chunks.content_registry().block_by_state_id(state)?;
            return Some(None);
        };
        let block_type = vanilla_block_type(block_type_id)?;
        if matches!(
            block_type_id,
            block_types::VANILLA_BLOCK_AIR_ID
                | block_types::VANILLA_BLOCK_CAVE_AIR_ID
                | block_types::VANILLA_BLOCK_VOID_AIR_ID
        ) {
            return Some(None);
        }

        // `PowderSnowBlock.getEntityInsideCollisionShape`
        // (`PowderSnowBlock.java`).
        let inside_block =
            if block_type_id == block_types::VANILLA_BLOCK_POWDER_SNOW_ID && fall_distance > 2.5 {
                collided_along_vector(
                    Aabb::of_entity(from, self.geometry.width, self.geometry.height),
                    subtract(to, from),
                    Aabb {
                        min_x: f64::from(pos.x),
                        min_y: f64::from(pos.y),
                        min_z: f64::from(pos.z),
                        max_x: f64::from(pos.x) + 1.0,
                        max_y: f64::from(pos.y) + f64::from(0.9_f32),
                        max_z: f64::from(pos.z) + 1.0,
                    },
                )
            } else if block_type_id == block_types::VANILLA_BLOCK_END_PORTAL_ID {
                // `EndPortalBlock.java:34,56-58`: `Block.column(16, 6, 12)`.
                collided_along_vector(
                    Aabb::of_entity(from, self.geometry.width, self.geometry.height),
                    subtract(to, from),
                    Aabb {
                        min_x: f64::from(pos.x),
                        min_y: f64::from(pos.y) + 0.375,
                        min_z: f64::from(pos.z),
                        max_x: f64::from(pos.x) + 1.0,
                        max_y: f64::from(pos.y) + 0.75,
                        max_z: f64::from(pos.z) + 1.0,
                    },
                )
            } else if block_type_id == block_types::VANILLA_BLOCK_TRIPWIRE_ID {
                collided_along_vector(
                    Aabb::of_entity(from, self.geometry.width, self.geometry.height),
                    subtract(to, from),
                    crate::block_behavior::tripwire::entity_box(pos, state),
                )
            } else if matches!(
                block_type_id,
                block_types::VANILLA_BLOCK_WATER_CAULDRON_ID
                    | block_types::VANILLA_BLOCK_POWDER_SNOW_CAULDRON_ID
                    | block_types::VANILLA_BLOCK_LAVA_CAULDRON_ID
            ) {
                let boxes = crate::block_behavior::cauldron::entity_inside_collision_boxes(state)?;
                let origin = Vec3d {
                    x: f64::from(pos.x),
                    y: f64::from(pos.y),
                    z: f64::from(pos.z),
                };
                boxes.into_iter().any(|shape| {
                    collided_along_vector(
                        Aabb::of_entity(from, self.geometry.width, self.geometry.height),
                        subtract(to, from),
                        shape.moved(origin),
                    )
                })
            } else {
                true
            };
        let fluid = environment::fluid_state_at(chunks, pos);
        let fluid = fluid?;
        let inside_fluid = if fluid.is_empty() {
            false
        } else {
            let height = environment::fluid_height_at(chunks, pos, fluid)?;
            collided_along_vector(
                Aabb::of_entity(from, self.geometry.width, self.geometry.height),
                subtract(to, from),
                Aabb {
                    min_x: f64::from(pos.x),
                    min_y: f64::from(pos.y),
                    min_z: f64::from(pos.z),
                    max_x: f64::from(pos.x) + 1.0,
                    max_y: f64::from(pos.y) + f64::from(height),
                    max_z: f64::from(pos.z) + 1.0,
                },
            )
        };
        if !(inside_block || inside_fluid) || !self.visited.insert((pos.x, pos.y, pos.z)) {
            return Some(None);
        }
        Some(Some(BodyBlockContact {
            pos,
            state,
            environment_kind: super::effects::environment_block_kind(block_type_id),
            resource_id: block_type.resource_id,
            step: iteration,
            precise: segment.moved_far || segment.target_box.intersects(Aabb::of_block(pos)),
            inside_block,
            fluid: if inside_fluid { fluid.family() } else { None },
        }))
    }
}

/// Entity.java: narrow each descent to float before double accumulation.
pub fn accumulate_body_fall_distance(fall_distance: f64, delta_y: f64, in_water: bool) -> f64 {
    if !in_water && delta_y < 0.0 {
        fall_distance - f64::from(delta_y as f32)
    } else {
        fall_distance
    }
}
