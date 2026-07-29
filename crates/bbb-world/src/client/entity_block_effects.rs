use std::cell::Cell;
use std::collections::BTreeMap;

use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID;

use crate::chunks::ChunkHeightmapSampler;
use crate::entities::{
    EntityBlockEffectEvaluation, EntityBlockEffectMovement, EntityBlockEffectMovementIter,
    EntityBlockEffectQuery,
};
use crate::terrain::{
    block_name_has_empty_collision_shape, block_name_is_air, block_name_is_fence,
    block_name_is_fence_gate, block_name_is_wall, terrain_fluid_state,
};
use crate::{
    section_block_index, BlockPos, ChunkColumn, ChunkPos, EntityVec3, PublicationShared,
    RegistrySet, TerrainFluidKind, TerrainFluidState, WorldDimension,
};

const INTERSECTION_DEFLATE: f64 = 1.0e-5;
const SHAPE_INFLATE_EPSILON: f64 = 1.0e-7;
const FAR_MOVEMENT_THRESHOLD_SQUARED: f64 = 0.999_980_000_100_505_2;
const MAX_MOVEMENT_ITERATIONS: i32 = 16;
const CHUNK_LOOKUP_CACHE_SLOTS: usize = 16;

/// `Level.isRaining()` plus the sea level `Level.precipitationAt` feeds into
/// `Biome.getPrecipitationAt`, resolved once per pass by the store owner.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EntityBlockEffectWeather {
    pub(crate) raining: bool,
    pub(crate) sea_level: i32,
}

/// Borrowed terrain reader for one entity tick pass.
///
/// The canonical chunk and registry owners remain in `WorldStore`; this view
/// exists only while `EntityStore` is mutably advanced, so the callback cannot
/// escape or become a second world owner. Its two traversal buffers reuse
/// their high-water capacity across every entity in the pass.
pub(crate) struct EntityBlockEffectWorld<'a> {
    dimension: WorldDimension,
    registries: &'a RegistrySet,
    chunks: &'a [PublicationShared<ChunkColumn>],
    weather: EntityBlockEffectWeather,
    // Vanilla uses primitive hash sets here. The client-side bounds exercised
    // by this policy visit only a handful of cells in the common case; reused
    // linear scratch avoids hashing and still grows for long swept segments.
    visited_blocks: Vec<BlockPos>,
    travel_blocks: Vec<BlockPos>,
    // Vanilla's powder-snow snowflakes read `entity.getX()/getZ()` at
    // `entityInside` time and gate on the tick's `xOld/zOld` horizontal
    // delta; the current movement's end anchors both while cells are visited.
    snowflake_anchor: Option<EntityVec3>,
    chunk_lookup_cache: [Cell<Option<(ChunkPos, usize)>>; CHUNK_LOOKUP_CACHE_SLOTS],
}

impl<'a> EntityBlockEffectWorld<'a> {
    pub(crate) fn new(
        dimension: WorldDimension,
        registries: &'a RegistrySet,
        chunks: &'a [PublicationShared<ChunkColumn>],
        weather: EntityBlockEffectWeather,
    ) -> Self {
        Self {
            dimension,
            registries,
            chunks,
            weather,
            visited_blocks: Vec::new(),
            travel_blocks: Vec::new(),
            snowflake_anchor: None,
            chunk_lookup_cache: std::array::from_fn(|_| Cell::new(None)),
        }
    }

    pub(crate) fn evaluate(
        &mut self,
        mut query: EntityBlockEffectQuery<'_>,
    ) -> EntityBlockEffectEvaluation {
        self.visited_blocks.clear();
        self.travel_blocks.clear();
        self.snowflake_anchor = None;
        let mut evaluation = EntityBlockEffectEvaluation::from_query(&query);
        self.apply_step_on(&query, &mut evaluation);
        let mut collector = InsideEffectCollector::default();

        let movements =
            std::mem::replace(&mut query.movements, EntityBlockEffectMovementIter::Empty);
        for movement in movements {
            self.check_movement(&query, movement, &mut collector, &mut evaluation);
        }
        collector.flush(&query, &mut evaluation);
        // Vanilla `applyEffectsFromBlocks` follows `applyAndClear` with
        // `if (this.isInRain()) this.clearFire();`. The extinguish sound
        // compares the shared on-fire flag, which local simulation never
        // flips, so only the private timer changes client-side.
        if evaluation.remaining_fire_ticks != 0
            && self.is_in_rain(query.position, f64::from(query.bounds.max[1]))
        {
            evaluation.remaining_fire_ticks = 0;
        }
        evaluation.visited_blocks = self.visited_blocks.len();
        evaluation
    }

    pub(crate) fn has_loaded_chunks(&self) -> bool {
        !self.chunks.is_empty()
    }

    /// Vanilla `Entity.isInRain`: `Level.isRainingAt` at the entity's block
    /// position and at that column sampled at the bounding-box top.
    pub(crate) fn is_in_rain(&self, position: EntityVec3, bounds_max_y: f64) -> bool {
        let block_pos = BlockPos {
            x: floor_i32(position.x),
            y: floor_i32(position.y),
            z: floor_i32(position.z),
        };
        self.is_raining_at(block_pos)
            || self.is_raining_at(BlockPos {
                y: floor_i32(position.y + bounds_max_y),
                ..block_pos
            })
    }

    /// Vanilla `Level.isRainingAt`, including the `isRaining` gate. Sky exposure uses
    /// the synced sky-light data (`canSeeSky` needs full brightness) and the
    /// packet `MOTION_BLOCKING` heightmap; positions above the build height
    /// stay exposed like vanilla's light engine padding.
    pub(crate) fn is_raining_at(&self, pos: BlockPos) -> bool {
        if !self.weather.raining {
            return false;
        }
        let top_y = self.dimension.min_y + self.dimension.height;
        if pos.y < self.dimension.min_y {
            return false;
        }
        let chunk_pos = ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        };
        let Some(chunk) = self.column(chunk_pos) else {
            return false;
        };
        let local_x = pos.x.rem_euclid(16);
        let local_z = pos.z.rem_euclid(16);
        if pos.y < top_y {
            let light = crate::chunks::sample_terrain_light(
                &chunk.light,
                self.dimension,
                local_x as usize,
                pos.y,
                local_z as usize,
            );
            if light.sky < 15 {
                return false;
            }
        }
        let motion_blocking_height = ChunkHeightmapSampler::new(chunk, self.dimension)
            .first_available(
                VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
                local_x as u8,
                local_z as u8,
            );
        match motion_blocking_height {
            Some(height) if height > pos.y => return false,
            Some(_) => {}
            None => return false,
        }
        let biome_y = pos.y.clamp(self.dimension.min_y, top_y - 1);
        let section_index =
            usize::try_from(biome_y.div_euclid(16) - self.dimension.min_section_y()).ok();
        let biome_id = section_index
            .and_then(|section_index| chunk.sections.get(section_index))
            .and_then(|section| {
                section
                    .biomes
                    .value_at(crate::section_biome_index(
                        local_x as u8 / 4,
                        biome_y.rem_euclid(16) as u8 / 4,
                        local_z as u8 / 4,
                    ))
                    .map(|value| value.global_id)
            });
        let Some(biome_id) = biome_id else {
            return false;
        };
        let Ok(Some(projection)) = self.registries.biome_client_projection(biome_id) else {
            return false;
        };
        if !projection.has_precipitation {
            return false;
        }
        // `Biome.getPrecipitationAt` returns RAIN exactly when the
        // height-adjusted temperature stays at or above `warmEnoughToRain`'s
        // 0.15 threshold.
        crate::climate::biome_height_adjusted_temperature(
            projection.temperature,
            projection.temperature_modifier,
            pos,
            self.weather.sea_level,
        ) >= 0.15
    }

    fn apply_step_on(
        &self,
        query: &EntityBlockEffectQuery<'_>,
        evaluation: &mut EntityBlockEffectEvaluation,
    ) {
        if !query.on_ground || query.stepping_carefully {
            return;
        }
        let pos = if let Some(supporting) = query.supporting_block {
            if self
                .cell(supporting)
                .is_some_and(|cell| cell.legacy_support_preserves_y)
            {
                supporting
            } else {
                BlockPos {
                    y: floor_i32(query.position.y - 0.2),
                    ..supporting
                }
            }
        } else {
            BlockPos {
                x: floor_i32(query.position.x),
                y: floor_i32(query.position.y - 0.2),
                z: floor_i32(query.position.z),
            }
        };
        let Some(cell) = self.cell(pos) else {
            return;
        };
        if cell.kind != BlockEffectKind::Slime {
            return;
        }
        let abs_y = evaluation.delta_movement.y.abs();
        if abs_y < 0.1 {
            let scale = 0.4 + abs_y * 0.2;
            evaluation.delta_movement.x *= scale;
            evaluation.delta_movement.z *= scale;
            evaluation.applied_callbacks += 1;
        }
    }

    fn check_movement(
        &mut self,
        query: &EntityBlockEffectQuery<'_>,
        movement: EntityBlockEffectMovement,
        collector: &mut InsideEffectCollector,
        evaluation: &mut EntityBlockEffectEvaluation,
    ) {
        self.snowflake_anchor = (movement.from.x != movement.to.x
            || movement.from.z != movement.to.z)
            .then_some(movement.to);
        let delta = subtract(movement.to, movement.from);
        let mut iterations_left = MAX_MOVEMENT_ITERATIONS;
        if let Some(original) = movement
            .axis_dependent_original
            .filter(|_| length_squared(delta) > 0.0)
        {
            let mut from = movement.from;
            for axis in axis_step_order(original) {
                let distance = coordinate(delta, axis);
                if distance == 0.0 {
                    continue;
                }
                let mut to = from;
                *coordinate_mut(&mut to, axis) += distance;
                let used =
                    self.check_segment(query, from, to, iterations_left, collector, evaluation);
                iterations_left -= used;
                from = to;
            }
        } else {
            iterations_left -= self.check_segment(
                query,
                movement.from,
                movement.to,
                MAX_MOVEMENT_ITERATIONS,
                collector,
                evaluation,
            );
        }

        if iterations_left <= 0 {
            self.check_segment(query, movement.to, movement.to, 1, collector, evaluation);
        }
    }

    fn check_segment(
        &mut self,
        query: &EntityBlockEffectQuery<'_>,
        from: EntityVec3,
        to: EntityVec3,
        max_iterations: i32,
        collector: &mut InsideEffectCollector,
        evaluation: &mut EntityBlockEffectEvaluation,
    ) -> i32 {
        let target_bounds = entity_bounds_at(query, to).deflate(INTERSECTION_DEFLATE);
        let moved_far = length_squared(subtract(to, from)) > FAR_MOVEMENT_THRESHOLD_SQUARED;
        let mut last_iteration = 0;
        let mut stopped = false;
        self.for_each_intersected_block(from, to, target_bounds, |world, pos, iteration| {
            if iteration >= max_iterations {
                stopped = true;
                return false;
            }
            last_iteration = iteration;
            let Some(cell) = world.cell(pos) else {
                return true;
            };
            if cell.kind == BlockEffectKind::Air {
                return true;
            }
            let block_shapes = inside_block_shapes(cell, pos, query);
            let inside_block = block_shapes.collided_along_vector(query, from, to);
            let inside_fluid = cell.fluid.is_some_and(|fluid| {
                collided_along_vector(query, from, to, world.fluid_box(pos, fluid))
            });
            if (!inside_block && !inside_fluid) || !insert_unique(&mut world.visited_blocks, pos) {
                return true;
            }

            if inside_block {
                let precise = moved_far || target_bounds.intersects(Aabb::unit(pos));
                collector.advance_step(iteration, query, evaluation);
                apply_block_callback(cell, pos, precise, query, collector, evaluation, world);
            }
            if inside_fluid {
                collector.advance_step(iteration, query, evaluation);
                match cell.fluid.expect("inside fluid implies fluid").kind {
                    TerrainFluidKind::Water => collector.effects.extinguish = true,
                    TerrainFluidKind::Lava => {
                        collector.effects.clear_freeze = true;
                        collector.effects.lava_ignite = true;
                    }
                }
            }
            true
        });
        if stopped {
            max_iterations
        } else {
            last_iteration + 1
        }
    }

    fn for_each_intersected_block(
        &mut self,
        from: EntityVec3,
        to: EntityVec3,
        target: Aabb,
        mut visitor: impl FnMut(&mut Self, BlockPos, i32) -> bool,
    ) {
        let travel = subtract(to, from);
        if length_squared(travel) < INTERSECTION_DEFLATE * INTERSECTION_DEFLATE {
            visit_between_closed(target, |pos| visitor(self, pos, 0));
            return;
        }

        self.travel_blocks.clear();
        let completed =
            visit_between_corners_in_direction(target.moved(scale(travel, -1.0)), travel, |pos| {
                if !visitor(self, pos, 0) {
                    return false;
                }
                insert_unique(&mut self.travel_blocks, pos);
                true
            });
        if !completed {
            return;
        }

        let Some(iterations) = self.add_collisions_along_travel(travel, target, &mut visitor)
        else {
            return;
        };
        visit_between_corners_in_direction(target, travel, |pos| {
            !insert_unique(&mut self.travel_blocks, pos) || visitor(self, pos, iterations + 1)
        });
    }

    fn add_collisions_along_travel(
        &mut self,
        travel: EntityVec3,
        target: Aabb,
        visitor: &mut impl FnMut(&mut Self, BlockPos, i32) -> bool,
    ) -> Option<i32> {
        let size = target.size();
        let corner_dir = [sign(travel.x), sign(travel.y), sign(travel.z)];
        let center = target.center();
        let to_corner = EntityVec3 {
            x: center.x + size.x * 0.5 * f64::from(corner_dir[0]),
            y: center.y + size.y * 0.5 * f64::from(corner_dir[1]),
            z: center.z + size.z * 0.5 * f64::from(corner_dir[2]),
        };
        let from_corner = subtract(to_corner, travel);
        let mut cell = [
            floor_i32(from_corner.x),
            floor_i32(from_corner.y),
            floor_i32(from_corner.z),
        ];
        let sign = corner_dir;
        let delta = [travel.x, travel.y, travel.z];
        let t_delta: [f64; 3] = std::array::from_fn(|axis| {
            if sign[axis] == 0 {
                f64::MAX
            } else {
                f64::from(sign[axis]) / delta[axis]
            }
        });
        let mut t: [f64; 3] = std::array::from_fn(|axis| {
            t_delta[axis]
                * if sign[axis] > 0 {
                    1.0 - frac([from_corner.x, from_corner.y, from_corner.z][axis])
                } else {
                    frac([from_corner.x, from_corner.y, from_corner.z][axis])
                }
        });
        let mut iterations = 0;
        while t[0] <= 1.0 || t[1] <= 1.0 || t[2] <= 1.0 {
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
            cell[axis] += sign[axis];
            t[axis] += t_delta[axis];
            let block = Aabb::from_arrays(
                [f64::from(cell[0]), f64::from(cell[1]), f64::from(cell[2])],
                [
                    f64::from(cell[0]) + 1.0,
                    f64::from(cell[1]) + 1.0,
                    f64::from(cell[2]) + 1.0,
                ],
            );
            let Some(hit) = block.clip(from_corner, to_corner) else {
                continue;
            };
            iterations += 1;
            let hit = [
                hit.x.clamp(
                    f64::from(cell[0]) + INTERSECTION_DEFLATE,
                    f64::from(cell[0]) + 1.0 - INTERSECTION_DEFLATE,
                ),
                hit.y.clamp(
                    f64::from(cell[1]) + INTERSECTION_DEFLATE,
                    f64::from(cell[1]) + 1.0 - INTERSECTION_DEFLATE,
                ),
                hit.z.clamp(
                    f64::from(cell[2]) + INTERSECTION_DEFLATE,
                    f64::from(cell[2]) + 1.0 - INTERSECTION_DEFLATE,
                ),
            ];
            let opposite = [
                floor_i32(hit[0] - size.x * f64::from(corner_dir[0])),
                floor_i32(hit[1] - size.y * f64::from(corner_dir[1])),
                floor_i32(hit[2] - size.z * f64::from(corner_dir[2])),
            ];
            let completed = visit_between_corners_in_direction_raw(cell, opposite, travel, |pos| {
                !insert_unique(&mut self.travel_blocks, pos) || visitor(self, pos, iterations)
            });
            if !completed {
                return None;
            }
        }
        Some(iterations)
    }

    fn column(&self, chunk_pos: ChunkPos) -> Option<&ChunkColumn> {
        let cache_slot = chunk_lookup_cache_slot(chunk_pos);
        let chunk_index = self.chunk_lookup_cache[cache_slot]
            .get()
            .filter(|(cached_pos, _)| *cached_pos == chunk_pos)
            .map(|(_, index)| index)
            .or_else(|| {
                let index = self
                    .chunks
                    .iter()
                    .position(|chunk| chunk.pos == chunk_pos)?;
                self.chunk_lookup_cache[cache_slot].set(Some((chunk_pos, index)));
                Some(index)
            })?;
        self.chunks.get(chunk_index).map(|chunk| &**chunk)
    }

    fn cell(&self, pos: BlockPos) -> Option<BlockEffectCell> {
        if !self.dimension.contains_y(pos.y) {
            return None;
        }
        let chunk_pos = ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        };
        let chunk = self.column(chunk_pos)?;
        let section_index =
            usize::try_from(pos.y.div_euclid(16) - self.dimension.min_section_y()).ok()?;
        let section = chunk.sections.get(section_index)?;
        let value = section.block_states.value_at(section_block_index(
            pos.x.rem_euclid(16) as u8,
            pos.y.rem_euclid(16) as u8,
            pos.z.rem_euclid(16) as u8,
        ))?;
        let state = self.registries.block_state(value.global_id)?;
        let collision_empty = state.name == "minecraft:powder_snow"
            || block_name_has_empty_collision_shape(&state.name);
        let legacy_support_preserves_y = block_name_is_fence(&state.name)
            || block_name_is_wall(&state.name)
            || block_name_is_fence_gate(&state.name);
        let kind = match state.name.as_str() {
            name if block_name_is_air(name) => BlockEffectKind::Air,
            "minecraft:slime_block" => BlockEffectKind::Slime,
            "minecraft:cobweb" => BlockEffectKind::Cobweb,
            "minecraft:sweet_berry_bush" => BlockEffectKind::SweetBerryBush,
            "minecraft:powder_snow" => BlockEffectKind::PowderSnow,
            "minecraft:bubble_column" => BlockEffectKind::BubbleColumn {
                drag_down: property_bool(&state.properties, "drag").unwrap_or(true),
            },
            "minecraft:honey_block" => BlockEffectKind::Honey,
            "minecraft:fire" | "minecraft:soul_fire" => BlockEffectKind::Fire,
            "minecraft:water_cauldron" => BlockEffectKind::WaterCauldron {
                level: property_i32(&state.properties, "level")
                    .unwrap_or(1)
                    .clamp(1, 3),
            },
            "minecraft:powder_snow_cauldron" => BlockEffectKind::PowderSnowCauldron {
                level: property_i32(&state.properties, "level")
                    .unwrap_or(1)
                    .clamp(1, 3),
            },
            "minecraft:lava_cauldron" => BlockEffectKind::LavaCauldron,
            "minecraft:end_portal" => BlockEffectKind::EndPortal,
            "minecraft:tripwire" => BlockEffectKind::Tripwire {
                attached: property_bool(&state.properties, "attached").unwrap_or(false),
            },
            _ => BlockEffectKind::Other,
        };
        Some(BlockEffectCell {
            kind,
            fluid: terrain_fluid_state(Some(&state.name), &state.properties),
            collision_empty,
            legacy_support_preserves_y,
        })
    }

    fn fluid_box(&self, pos: BlockPos, fluid: TerrainFluidState) -> Aabb {
        let height = if self
            .cell(BlockPos {
                y: pos.y + 1,
                ..pos
            })
            .and_then(|cell| cell.fluid)
            .is_some_and(|above| above.kind == fluid.kind)
        {
            1.0
        } else {
            fluid.own_height()
        };
        Aabb::local_box(pos, [0.0, 0.0, 0.0], [1.0, height, 1.0])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockEffectKind {
    Air,
    Other,
    Slime,
    Cobweb,
    SweetBerryBush,
    PowderSnow,
    BubbleColumn { drag_down: bool },
    Honey,
    Fire,
    WaterCauldron { level: i32 },
    PowderSnowCauldron { level: i32 },
    LavaCauldron,
    EndPortal,
    Tripwire { attached: bool },
}

#[derive(Debug, Clone, Copy)]
struct BlockEffectCell {
    kind: BlockEffectKind,
    fluid: Option<TerrainFluidState>,
    collision_empty: bool,
    legacy_support_preserves_y: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct PendingInsideEffects {
    freeze: bool,
    clear_freeze: bool,
    fire_ignite: bool,
    lava_ignite: bool,
    extinguish: bool,
}

#[derive(Debug, Default)]
struct InsideEffectCollector {
    last_step: Option<i32>,
    effects: PendingInsideEffects,
}

impl InsideEffectCollector {
    fn advance_step(
        &mut self,
        step: i32,
        query: &EntityBlockEffectQuery<'_>,
        evaluation: &mut EntityBlockEffectEvaluation,
    ) {
        if self.last_step != Some(step) {
            self.flush(query, evaluation);
            self.last_step = Some(step);
        }
    }

    fn flush(
        &mut self,
        query: &EntityBlockEffectQuery<'_>,
        evaluation: &mut EntityBlockEffectEvaluation,
    ) {
        // `InsideBlockEffectType.values()` order in Vanilla 26.1.
        if self.effects.freeze {
            evaluation.in_powder_snow = true;
            if !query.freeze_immune {
                evaluation.ticks_frozen = (evaluation.ticks_frozen + 1).min(140);
            }
        }
        if self.effects.clear_freeze {
            evaluation.ticks_frozen = 0;
        }
        if self.effects.fire_ignite && !query.fire_immune {
            if evaluation.remaining_fire_ticks < 0 {
                evaluation.remaining_fire_ticks += 1;
            }
            if evaluation.remaining_fire_ticks >= 0 {
                evaluation.remaining_fire_ticks = evaluation.remaining_fire_ticks.max(160);
                evaluation.ticks_frozen = 0;
            }
        }
        if self.effects.lava_ignite && !query.fire_immune {
            evaluation.remaining_fire_ticks = evaluation.remaining_fire_ticks.max(300);
            evaluation.ticks_frozen = 0;
        }
        if self.effects.extinguish {
            evaluation.remaining_fire_ticks = evaluation.remaining_fire_ticks.min(0);
        }
        self.effects = PendingInsideEffects::default();
    }
}

fn apply_block_callback(
    cell: BlockEffectCell,
    pos: BlockPos,
    precise: bool,
    query: &EntityBlockEffectQuery<'_>,
    collector: &mut InsideEffectCollector,
    evaluation: &mut EntityBlockEffectEvaluation,
    world: &EntityBlockEffectWorld<'_>,
) {
    match cell.kind {
        BlockEffectKind::Cobweb => {
            evaluation.stuck_speed_multiplier = if query.has_weaving {
                EntityVec3 {
                    x: 0.5,
                    y: 0.25,
                    z: 0.5,
                }
            } else {
                EntityVec3 {
                    x: 0.25,
                    y: 0.05,
                    z: 0.25,
                }
            };
            evaluation.fall_distance = 0.0;
            evaluation.applied_callbacks += 1;
        }
        BlockEffectKind::SweetBerryBush if query.is_living && !query.is_fox_or_bee => {
            evaluation.stuck_speed_multiplier = EntityVec3 {
                x: 0.8,
                y: 0.75,
                z: 0.8,
            };
            evaluation.fall_distance = 0.0;
            evaluation.applied_callbacks += 1;
        }
        BlockEffectKind::PowderSnow => {
            let current_block = BlockPos {
                x: floor_i32(query.position.x),
                y: floor_i32(query.position.y),
                z: floor_i32(query.position.z),
            };
            if !query.is_living
                || world
                    .cell(current_block)
                    .is_some_and(|cell| cell.kind == BlockEffectKind::PowderSnow)
            {
                evaluation.stuck_speed_multiplier = EntityVec3 {
                    x: 0.9,
                    y: 1.5,
                    z: 0.9,
                };
                evaluation.fall_distance = 0.0;
                if let Some(anchor) = world.snowflake_anchor {
                    evaluation.snowflakes.push(EntityVec3 {
                        x: anchor.x,
                        y: f64::from(pos.y + 1),
                        z: anchor.z,
                    });
                }
            }
            collector.effects.freeze = true;
            collector.effects.extinguish = true;
            evaluation.applied_callbacks += 1;
        }
        BlockEffectKind::BubbleColumn { drag_down } if precise => {
            let above = world.cell(BlockPos {
                y: pos.y + 1,
                ..pos
            });
            let open_above = above.is_none_or(|cell| cell.collision_empty && cell.fluid.is_none());
            if open_above {
                evaluation.delta_movement.y = if drag_down {
                    (-0.9_f64).max(evaluation.delta_movement.y - 0.03)
                } else {
                    1.8_f64.min(evaluation.delta_movement.y + 0.1)
                };
            } else {
                evaluation.delta_movement.y = if drag_down {
                    (-0.3_f64).max(evaluation.delta_movement.y - 0.03)
                } else {
                    0.7_f64.min(evaluation.delta_movement.y + 0.06)
                };
                evaluation.fall_distance = 0.0;
            }
            evaluation.applied_callbacks += 1;
        }
        BlockEffectKind::Honey => {
            let width = f64::from(query.bounds.max[0] - query.bounds.min[0]);
            let overlap_distance = 0.4375 + width * 0.5;
            let side = (f64::from(pos.x) + 0.5 - query.position.x).abs() + 1.0e-7
                > overlap_distance
                || (f64::from(pos.z) + 0.5 - query.position.z).abs() + 1.0e-7 > overlap_distance;
            let above_floor = query.position.y > f64::from(pos.y) + 0.9375 - 1.0e-7;
            let old_delta_y = evaluation.delta_movement.y / f64::from(0.98_f32) + 0.08;
            if !query.on_ground && side && !above_floor && old_delta_y < -0.08 {
                if old_delta_y < -0.13 {
                    let horizontal_scale = -0.05 / old_delta_y;
                    evaluation.delta_movement.x *= horizontal_scale;
                    evaluation.delta_movement.z *= horizontal_scale;
                }
                evaluation.delta_movement.y = (-0.05 - 0.08) * f64::from(0.98_f32);
                evaluation.fall_distance = 0.0;
                evaluation.applied_callbacks += 1;
            }
        }
        BlockEffectKind::Fire => {
            collector.effects.clear_freeze = true;
            collector.effects.fire_ignite = true;
            evaluation.applied_callbacks += 1;
        }
        BlockEffectKind::WaterCauldron { .. } | BlockEffectKind::PowderSnowCauldron { .. } => {
            collector.effects.extinguish = true;
            evaluation.applied_callbacks += 1;
        }
        BlockEffectKind::LavaCauldron => {
            collector.effects.clear_freeze = true;
            collector.effects.lava_ignite = true;
            evaluation.applied_callbacks += 1;
        }
        _ => {}
    }
}

#[derive(Debug, Clone, Copy)]
enum InsideBlockShapes {
    Full,
    One(Aabb),
    Cauldron { walls: [Aabb; 13], content: Aabb },
}

impl InsideBlockShapes {
    fn collided_along_vector(
        self,
        query: &EntityBlockEffectQuery<'_>,
        from: EntityVec3,
        to: EntityVec3,
    ) -> bool {
        match self {
            Self::Full => true,
            Self::One(shape) => collided_along_vector(query, from, to, shape),
            Self::Cauldron { walls, content } => walls
                .into_iter()
                .chain(std::iter::once(content))
                .any(|shape| collided_along_vector(query, from, to, shape)),
        }
    }
}

fn inside_block_shapes(
    cell: BlockEffectCell,
    pos: BlockPos,
    query: &EntityBlockEffectQuery<'_>,
) -> InsideBlockShapes {
    if cell.kind == BlockEffectKind::PowderSnow && query.fall_distance > 2.5 {
        return InsideBlockShapes::One(Aabb::local_box(
            pos,
            [0.0, 0.0, 0.0],
            [1.0, f64::from(0.9_f32), 1.0],
        ));
    }
    if cell.kind == BlockEffectKind::EndPortal {
        return InsideBlockShapes::One(Aabb::local_box(
            pos,
            [0.0, 6.0 / 16.0, 0.0],
            [1.0, 12.0 / 16.0, 1.0],
        ));
    }
    if let BlockEffectKind::Tripwire { attached } = cell.kind {
        return InsideBlockShapes::One(Aabb::local_box(
            pos,
            [0.0, if attached { 1.0 / 16.0 } else { 0.0 }, 0.0],
            [1.0, if attached { 2.5 / 16.0 } else { 0.5 }, 1.0],
        ));
    }
    if let Some(content_top) = match cell.kind {
        BlockEffectKind::WaterCauldron { level }
        | BlockEffectKind::PowderSnowCauldron { level } => Some(f64::from(6 + 3 * level) / 16.0),
        BlockEffectKind::LavaCauldron => Some(15.0 / 16.0),
        _ => None,
    } {
        let walls = cauldron_wall_boxes(pos);
        let content = Aabb::local_box(
            pos,
            [2.0 / 16.0, 4.0 / 16.0, 2.0 / 16.0],
            [14.0 / 16.0, content_top, 14.0 / 16.0],
        );
        return InsideBlockShapes::Cauldron { walls, content };
    }
    InsideBlockShapes::Full
}

fn cauldron_wall_boxes(pos: BlockPos) -> [Aabb; 13] {
    [
        Aabb::pixel_box(pos, [0.0, 0.0, 0.0], [2.0, 3.0, 4.0]),
        Aabb::pixel_box(pos, [0.0, 0.0, 12.0], [2.0, 3.0, 16.0]),
        Aabb::pixel_box(pos, [14.0, 0.0, 0.0], [16.0, 3.0, 4.0]),
        Aabb::pixel_box(pos, [14.0, 0.0, 12.0], [16.0, 3.0, 16.0]),
        Aabb::pixel_box(pos, [2.0, 0.0, 0.0], [4.0, 3.0, 2.0]),
        Aabb::pixel_box(pos, [2.0, 0.0, 14.0], [4.0, 3.0, 16.0]),
        Aabb::pixel_box(pos, [12.0, 0.0, 0.0], [14.0, 3.0, 2.0]),
        Aabb::pixel_box(pos, [12.0, 0.0, 14.0], [14.0, 3.0, 16.0]),
        Aabb::pixel_box(pos, [0.0, 3.0, 0.0], [16.0, 4.0, 16.0]),
        Aabb::pixel_box(pos, [0.0, 4.0, 0.0], [2.0, 16.0, 16.0]),
        Aabb::pixel_box(pos, [14.0, 4.0, 0.0], [16.0, 16.0, 16.0]),
        Aabb::pixel_box(pos, [2.0, 4.0, 0.0], [14.0, 16.0, 2.0]),
        Aabb::pixel_box(pos, [2.0, 4.0, 14.0], [14.0, 16.0, 16.0]),
    ]
}

fn collided_along_vector(
    query: &EntityBlockEffectQuery<'_>,
    from: EntityVec3,
    to: EntityVec3,
    shape: Aabb,
) -> bool {
    let entity = entity_bounds_at(query, from);
    let center = entity.center();
    let target_center = add(center, subtract(to, from));
    let size = entity.size();
    let inflated = shape.inflate(
        size.x * 0.5 - SHAPE_INFLATE_EPSILON,
        size.y * 0.5 - SHAPE_INFLATE_EPSILON,
        size.z * 0.5 - SHAPE_INFLATE_EPSILON,
    );
    inflated.contains(center)
        || inflated.contains(target_center)
        || inflated.clip(center, target_center).is_some()
}

#[derive(Debug, Clone, Copy)]
struct Aabb {
    min: EntityVec3,
    max: EntityVec3,
}

impl Aabb {
    fn from_arrays(min: [f64; 3], max: [f64; 3]) -> Self {
        Self {
            min: EntityVec3 {
                x: min[0],
                y: min[1],
                z: min[2],
            },
            max: EntityVec3 {
                x: max[0],
                y: max[1],
                z: max[2],
            },
        }
    }

    fn unit(pos: BlockPos) -> Self {
        Self::local_box(pos, [0.0; 3], [1.0; 3])
    }

    fn local_box(pos: BlockPos, min: [f64; 3], max: [f64; 3]) -> Self {
        Self::from_arrays(
            [
                f64::from(pos.x) + min[0],
                f64::from(pos.y) + min[1],
                f64::from(pos.z) + min[2],
            ],
            [
                f64::from(pos.x) + max[0],
                f64::from(pos.y) + max[1],
                f64::from(pos.z) + max[2],
            ],
        )
    }

    fn pixel_box(pos: BlockPos, min: [f64; 3], max: [f64; 3]) -> Self {
        Self::local_box(
            pos,
            min.map(|coordinate| coordinate / 16.0),
            max.map(|coordinate| coordinate / 16.0),
        )
    }

    fn deflate(self, amount: f64) -> Self {
        self.inflate(-amount, -amount, -amount)
    }

    fn inflate(self, x: f64, y: f64, z: f64) -> Self {
        Self::from_arrays(
            [self.min.x - x, self.min.y - y, self.min.z - z],
            [self.max.x + x, self.max.y + y, self.max.z + z],
        )
    }

    fn moved(self, movement: EntityVec3) -> Self {
        Self {
            min: add(self.min, movement),
            max: add(self.max, movement),
        }
    }

    fn center(self) -> EntityVec3 {
        scale(add(self.min, self.max), 0.5)
    }

    fn size(self) -> EntityVec3 {
        subtract(self.max, self.min)
    }

    fn contains(self, point: EntityVec3) -> bool {
        point.x >= self.min.x
            && point.x < self.max.x
            && point.y >= self.min.y
            && point.y < self.max.y
            && point.z >= self.min.z
            && point.z < self.max.z
    }

    fn intersects(self, other: Self) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
            && self.min.z < other.max.z
            && self.max.z > other.min.z
    }

    fn clip(self, from: EntityVec3, to: EntityVec3) -> Option<EntityVec3> {
        let delta = subtract(to, from);
        let mut lower: f64 = 0.0;
        let mut upper: f64 = 1.0;
        for (start, direction, min, max) in [
            (from.x, delta.x, self.min.x, self.max.x),
            (from.y, delta.y, self.min.y, self.max.y),
            (from.z, delta.z, self.min.z, self.max.z),
        ] {
            if direction == 0.0 {
                if start < min || start > max {
                    return None;
                }
                continue;
            }
            let first = (min - start) / direction;
            let second = (max - start) / direction;
            lower = lower.max(first.min(second));
            upper = upper.min(first.max(second));
            if lower > upper {
                return None;
            }
        }
        (lower > 0.0 && lower < 1.0).then(|| add(from, scale(delta, lower)))
    }
}

fn entity_bounds_at(query: &EntityBlockEffectQuery<'_>, position: EntityVec3) -> Aabb {
    Aabb::from_arrays(
        [
            position.x + f64::from(query.bounds.min[0]),
            position.y + f64::from(query.bounds.min[1]),
            position.z + f64::from(query.bounds.min[2]),
        ],
        [
            position.x + f64::from(query.bounds.max[0]),
            position.y + f64::from(query.bounds.max[1]),
            position.z + f64::from(query.bounds.max[2]),
        ],
    )
}

fn visit_between_closed(bounds: Aabb, mut visitor: impl FnMut(BlockPos) -> bool) -> bool {
    for z in floor_i32(bounds.min.z)..=floor_i32(bounds.max.z) {
        for y in floor_i32(bounds.min.y)..=floor_i32(bounds.max.y) {
            for x in floor_i32(bounds.min.x)..=floor_i32(bounds.max.x) {
                if !visitor(BlockPos { x, y, z }) {
                    return false;
                }
            }
        }
    }
    true
}

fn visit_between_corners_in_direction(
    bounds: Aabb,
    direction: EntityVec3,
    visitor: impl FnMut(BlockPos) -> bool,
) -> bool {
    visit_between_corners_in_direction_raw(
        [
            floor_i32(bounds.min.x),
            floor_i32(bounds.min.y),
            floor_i32(bounds.min.z),
        ],
        [
            floor_i32(bounds.max.x),
            floor_i32(bounds.max.y),
            floor_i32(bounds.max.z),
        ],
        direction,
        visitor,
    )
}

fn visit_between_corners_in_direction_raw(
    first: [i32; 3],
    second: [i32; 3],
    direction: EntityVec3,
    mut visitor: impl FnMut(BlockPos) -> bool,
) -> bool {
    let min: [i32; 3] = std::array::from_fn(|axis| first[axis].min(second[axis]));
    let max: [i32; 3] = std::array::from_fn(|axis| first[axis].max(second[axis]));
    let start = [
        if direction.x >= 0.0 { min[0] } else { max[0] },
        if direction.y >= 0.0 { min[1] } else { max[1] },
        if direction.z >= 0.0 { min[2] } else { max[2] },
    ];
    let axes = axis_step_order(direction);
    for first_index in 0..=(max[axes[0]] - min[axes[0]]) {
        for second_index in 0..=(max[axes[1]] - min[axes[1]]) {
            for third_index in 0..=(max[axes[2]] - min[axes[2]]) {
                let mut coordinate = start;
                for (axis, index) in [
                    (axes[0], first_index),
                    (axes[1], second_index),
                    (axes[2], third_index),
                ] {
                    let positive = [direction.x, direction.y, direction.z][axis] >= 0.0;
                    coordinate[axis] += if positive { index } else { -index };
                }
                if !visitor(BlockPos {
                    x: coordinate[0],
                    y: coordinate[1],
                    z: coordinate[2],
                }) {
                    return false;
                }
            }
        }
    }
    true
}

fn axis_step_order(movement: EntityVec3) -> [usize; 3] {
    if movement.x.abs() < movement.z.abs() {
        [1, 2, 0]
    } else {
        [1, 0, 2]
    }
}

fn coordinate(vector: EntityVec3, axis: usize) -> f64 {
    [vector.x, vector.y, vector.z][axis]
}

fn coordinate_mut(vector: &mut EntityVec3, axis: usize) -> &mut f64 {
    match axis {
        0 => &mut vector.x,
        1 => &mut vector.y,
        _ => &mut vector.z,
    }
}

fn property_bool(properties: &BTreeMap<String, String>, key: &str) -> Option<bool> {
    match properties.get(key).map(String::as_str) {
        Some("true") => Some(true),
        Some("false") => Some(false),
        _ => None,
    }
}

fn property_i32(properties: &BTreeMap<String, String>, key: &str) -> Option<i32> {
    properties.get(key)?.parse().ok()
}

fn insert_unique(positions: &mut Vec<BlockPos>, pos: BlockPos) -> bool {
    if positions.contains(&pos) {
        false
    } else {
        positions.push(pos);
        true
    }
}

fn chunk_lookup_cache_slot(pos: ChunkPos) -> usize {
    let mixed = (pos.x as u32).wrapping_mul(0x9e37_79b1) ^ (pos.z as u32).rotate_left(16);
    mixed as usize & (CHUNK_LOOKUP_CACHE_SLOTS - 1)
}

fn floor_i32(value: f64) -> i32 {
    value.floor() as i32
}

fn frac(value: f64) -> f64 {
    value - value.floor()
}

fn sign(value: f64) -> i32 {
    if value > 0.0 {
        1
    } else if value < 0.0 {
        -1
    } else {
        0
    }
}

fn length_squared(value: EntityVec3) -> f64 {
    value.x * value.x + value.y * value.y + value.z * value.z
}

fn add(left: EntityVec3, right: EntityVec3) -> EntityVec3 {
    EntityVec3 {
        x: left.x + right.x,
        y: left.y + right.y,
        z: left.z + right.z,
    }
}

fn subtract(left: EntityVec3, right: EntityVec3) -> EntityVec3 {
    EntityVec3 {
        x: left.x - right.x,
        y: left.y - right.y,
        z: left.z - right.z,
    }
}

fn scale(value: EntityVec3, amount: f64) -> EntityVec3 {
    EntityVec3 {
        x: value.x * amount,
        y: value.y * amount,
        z: value.z * amount,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind, PalettedContainerData,
        WorldStore,
    };

    #[test]
    fn vanilla_axis_step_order_prefers_yzx_only_when_x_is_smaller() {
        assert_eq!(
            axis_step_order(EntityVec3 {
                x: 0.1,
                y: 9.0,
                z: 0.2
            }),
            [1, 2, 0]
        );
        assert_eq!(
            axis_step_order(EntityVec3 {
                x: 0.2,
                y: 9.0,
                z: 0.1
            }),
            [1, 0, 2]
        );
        assert_eq!(
            axis_step_order(EntityVec3 {
                x: 0.2,
                y: 9.0,
                z: 0.2
            }),
            [1, 0, 2]
        );
    }

    #[test]
    fn between_closed_keeps_vanilla_x_then_y_then_z_order() {
        let mut positions = Vec::new();
        assert!(visit_between_closed(
            Aabb::from_arrays([0.0; 3], [1.0, 1.0, 0.0]),
            |pos| {
                positions.push(pos);
                true
            },
        ));
        assert_eq!(
            positions,
            vec![
                BlockPos { x: 0, y: 0, z: 0 },
                BlockPos { x: 1, y: 0, z: 0 },
                BlockPos { x: 0, y: 1, z: 0 },
                BlockPos { x: 1, y: 1, z: 0 },
            ]
        );
    }

    #[test]
    fn collided_along_vector_uses_entity_center_minkowski_expansion() {
        let state = crate::entities::EntityBlockEffectMovements::default();
        let query = EntityBlockEffectQuery {
            position: EntityVec3::default(),
            bounds: crate::EntityPickBoundsState {
                min: [-0.3, 0.0, -0.3],
                max: [0.3, 1.8, 0.3],
                pick_radius: 0.0,
            },
            movements: state.last_movements(),
            supporting_block: None,
            on_ground: false,
            stepping_carefully: false,
            is_living: true,
            is_fox_or_bee: false,
            has_weaving: false,
            fire_immune: false,
            freeze_immune: false,
            ticks_frozen: 0,
            delta_movement: EntityVec3::default(),
            stuck_speed_multiplier: EntityVec3::default(),
            remaining_fire_ticks: 0,
            fall_distance: 0.0,
        };
        assert!(collided_along_vector(
            &query,
            EntityVec3 {
                x: -1.0,
                y: 0.0,
                z: 0.5
            },
            EntityVec3 {
                x: 0.5,
                y: 0.0,
                z: 0.5
            },
            Aabb::unit(BlockPos { x: 0, y: 0, z: 0 }),
        ));
    }

    #[test]
    fn swept_cobweb_callback_sets_next_move_multiplier_without_probe_clones() {
        let mut world = test_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            "minecraft:cobweb",
            [],
        );
        let mut movements = crate::entities::EntityBlockEffectMovements::default();
        movements.record_movement(EntityBlockEffectMovement::direct(
            EntityVec3 {
                x: -1.0,
                y: 0.0,
                z: 0.5,
            },
            EntityVec3 {
                x: 0.5,
                y: 0.0,
                z: 0.5,
            },
        ));
        let movement_iter = movements.prepare_current(
            EntityVec3 {
                x: -1.0,
                y: 0.0,
                z: 0.5,
            },
            EntityVec3 {
                x: 0.5,
                y: 0.0,
                z: 0.5,
            },
        );
        let mut evaluator = EntityBlockEffectWorld::new(
            world.dimension,
            &world.registries,
            &world.chunks,
            world.entity_block_effect_weather(),
        );
        let result = evaluator.evaluate(test_query(
            EntityVec3 {
                x: 0.5,
                y: 0.0,
                z: 0.5,
            },
            movement_iter,
        ));
        assert_eq!(
            result.stuck_speed_multiplier,
            EntityVec3 {
                x: 0.25,
                y: 0.05,
                z: 0.25
            }
        );
        assert_eq!(result.fall_distance, 0.0);
        assert_eq!(result.applied_callbacks, 1);
        assert!(result.visited_blocks >= 1);
    }

    #[test]
    fn slime_step_on_uses_vanilla_legacy_fallback_without_support_cache() {
        let mut world = test_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            "minecraft:slime_block",
            [],
        );
        let movements = crate::entities::EntityBlockEffectMovements::default();
        let position = EntityVec3 {
            x: 0.5,
            y: 1.0,
            z: 0.5,
        };
        let mut query = test_query(position, movements.last_movements());
        query.on_ground = true;
        query.delta_movement = EntityVec3 {
            x: 1.0,
            y: 0.05,
            z: 1.0,
        };
        let mut evaluator = EntityBlockEffectWorld::new(
            world.dimension,
            &world.registries,
            &world.chunks,
            world.entity_block_effect_weather(),
        );

        let result = evaluator.evaluate(query);

        assert!((result.delta_movement.x - 0.41).abs() < 1e-12);
        assert!((result.delta_movement.z - 0.41).abs() < 1e-12);
        assert_eq!(result.applied_callbacks, 1);
    }

    #[test]
    fn powder_snow_then_fire_uses_vanilla_effect_type_order_per_step() {
        let mut world = test_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            "minecraft:powder_snow",
            [],
        );
        set_block(
            &mut world,
            BlockPos { x: 1, y: 0, z: 0 },
            "minecraft:fire",
            [],
        );
        let mut movements = crate::entities::EntityBlockEffectMovements::default();
        let from = EntityVec3 {
            x: 0.5,
            y: 0.0,
            z: 0.5,
        };
        let to = EntityVec3 {
            x: 1.5,
            y: 0.0,
            z: 0.5,
        };
        movements.record_movement(EntityBlockEffectMovement::direct(from, to));
        let movement_iter = movements.prepare_current(from, to);
        let mut evaluator = EntityBlockEffectWorld::new(
            world.dimension,
            &world.registries,
            &world.chunks,
            world.entity_block_effect_weather(),
        );
        let result = evaluator.evaluate(test_query(to, movement_iter));
        assert_eq!(result.ticks_frozen, 0);
        assert_eq!(result.remaining_fire_ticks, 160);
        assert!(result.in_powder_snow);
    }

    #[test]
    fn powder_snow_snowflakes_require_horizontal_movement() {
        let mut world = test_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            "minecraft:powder_snow",
            [],
        );
        let from = EntityVec3 {
            x: 0.4,
            y: 0.0,
            z: 0.5,
        };
        let to = EntityVec3 {
            x: 0.6,
            y: 0.0,
            z: 0.5,
        };
        let mut movements = crate::entities::EntityBlockEffectMovements::default();
        let movement_iter = movements.prepare_current(from, to);
        let mut evaluator = EntityBlockEffectWorld::new(
            world.dimension,
            &world.registries,
            &world.chunks,
            world.entity_block_effect_weather(),
        );
        let result = evaluator.evaluate(test_query(to, movement_iter));
        // One chance per stuck-applying block, anchored at the movement end
        // with the Vanilla `pos.getY() + 1` particle height.
        assert_eq!(
            result.snowflakes,
            vec![EntityVec3 {
                x: 0.6,
                y: 1.0,
                z: 0.5,
            }]
        );
        assert!((result.stuck_speed_multiplier.x - 0.9).abs() <= f64::EPSILON);

        let position = EntityVec3 {
            x: 0.5,
            y: 0.1,
            z: 0.5,
        };
        let mut movements = crate::entities::EntityBlockEffectMovements::default();
        let movement_iter = movements.prepare_current(EntityVec3 { y: 0.3, ..position }, position);
        let result = evaluator.evaluate(test_query(position, movement_iter));
        // A purely vertical move still gets stuck but never rolls snowflakes.
        assert!(result.snowflakes.is_empty());
        assert!((result.stuck_speed_multiplier.x - 0.9).abs() <= f64::EPSILON);
    }

    #[test]
    fn bubble_column_precise_callback_distinguishes_open_surface() {
        let mut world = test_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            "minecraft:bubble_column",
            [("drag", "false")],
        );
        let mut movements = crate::entities::EntityBlockEffectMovements::default();
        let position = EntityVec3 {
            x: 0.5,
            y: 0.0,
            z: 0.5,
        };
        let movement_iter = movements.prepare_current(position, position);
        let mut evaluator = EntityBlockEffectWorld::new(
            world.dimension,
            &world.registries,
            &world.chunks,
            world.entity_block_effect_weather(),
        );
        let result = evaluator.evaluate(test_query(position, movement_iter));
        assert!((result.delta_movement.y - 0.1).abs() <= f64::EPSILON);

        drop(evaluator);
        set_block(
            &mut world,
            BlockPos { x: 0, y: 1, z: 0 },
            "minecraft:torch",
            [],
        );
        let mut movements = crate::entities::EntityBlockEffectMovements::default();
        let mut evaluator = EntityBlockEffectWorld::new(
            world.dimension,
            &world.registries,
            &world.chunks,
            world.entity_block_effect_weather(),
        );
        let result = evaluator.evaluate(test_query(
            position,
            movements.prepare_current(position, position),
        ));
        assert!((result.delta_movement.y - 0.1).abs() <= f64::EPSILON);

        drop(evaluator);
        set_block(
            &mut world,
            BlockPos { x: 0, y: 1, z: 0 },
            "minecraft:stone",
            [],
        );
        let mut movements = crate::entities::EntityBlockEffectMovements::default();
        let mut evaluator = EntityBlockEffectWorld::new(
            world.dimension,
            &world.registries,
            &world.chunks,
            world.entity_block_effect_weather(),
        );
        let result = evaluator.evaluate(test_query(
            position,
            movements.prepare_current(position, position),
        ));
        assert!((result.delta_movement.y - 0.06).abs() <= f64::EPSILON);
    }

    #[test]
    fn freeze_immunity_keeps_ticks_while_still_marking_powder_snow_contact() {
        let mut world = test_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            "minecraft:powder_snow",
            [],
        );
        let mut movements = crate::entities::EntityBlockEffectMovements::default();
        let position = EntityVec3 {
            x: 0.5,
            y: 0.0,
            z: 0.5,
        };
        let movement_iter = movements.prepare_current(position, position);
        let mut query = test_query(position, movement_iter);
        query.freeze_immune = true;
        query.ticks_frozen = 17;
        let mut evaluator = EntityBlockEffectWorld::new(
            world.dimension,
            &world.registries,
            &world.chunks,
            world.entity_block_effect_weather(),
        );

        let result = evaluator.evaluate(query);

        assert_eq!(result.ticks_frozen, 17);
        assert!(result.in_powder_snow);
    }

    #[test]
    fn fluid_collision_box_uses_full_height_for_same_fluid_above() {
        let mut world = test_world();
        let lower = BlockPos { x: 0, y: 0, z: 0 };
        set_block(&mut world, lower, "minecraft:water", [("level", "0")]);
        let evaluator = EntityBlockEffectWorld::new(
            world.dimension,
            &world.registries,
            &world.chunks,
            world.entity_block_effect_weather(),
        );
        let lower_fluid = evaluator
            .cell(lower)
            .and_then(|cell| cell.fluid)
            .expect("source water fluid");
        assert!(
            (evaluator.fluid_box(lower, lower_fluid).max.y - lower_fluid.own_height()).abs()
                < 1e-12
        );

        set_block(
            &mut world,
            BlockPos { x: 0, y: 1, z: 0 },
            "minecraft:water",
            [("level", "0")],
        );
        let evaluator = EntityBlockEffectWorld::new(
            world.dimension,
            &world.registries,
            &world.chunks,
            world.entity_block_effect_weather(),
        );
        assert!((evaluator.fluid_box(lower, lower_fluid).max.y - 1.0).abs() < 1e-12);
    }

    #[test]
    fn honey_slide_uses_entity_width_and_rejects_on_ground_contact() {
        let mut world = test_world();
        set_block(
            &mut world,
            BlockPos { x: 0, y: 0, z: 0 },
            "minecraft:honey_block",
            [],
        );
        let position = EntityVec3 {
            x: 1.24,
            y: 0.5,
            z: 0.5,
        };
        let mut movements = crate::entities::EntityBlockEffectMovements::default();
        let mut query = test_query(position, movements.prepare_current(position, position));
        query.delta_movement.y = (-0.2 - 0.08) * f64::from(0.98_f32);
        let mut evaluator = EntityBlockEffectWorld::new(
            world.dimension,
            &world.registries,
            &world.chunks,
            world.entity_block_effect_weather(),
        );
        let sliding = evaluator.evaluate(query);
        assert!((sliding.delta_movement.y - (-0.05 - 0.08) * f64::from(0.98_f32)).abs() < 1e-12);

        let mut on_ground_movements = crate::entities::EntityBlockEffectMovements::default();
        let mut on_ground = test_query(
            position,
            on_ground_movements.prepare_current(position, position),
        );
        on_ground.on_ground = true;
        on_ground.delta_movement.y = (-0.2 - 0.08) * f64::from(0.98_f32);
        let grounded = evaluator.evaluate(on_ground);
        assert!((grounded.delta_movement.y - (-0.2 - 0.08) * f64::from(0.98_f32)).abs() < 1e-12);
    }

    fn test_query<'a>(
        position: EntityVec3,
        movements: EntityBlockEffectMovementIter<'a>,
    ) -> EntityBlockEffectQuery<'a> {
        EntityBlockEffectQuery {
            position,
            bounds: crate::EntityPickBoundsState {
                min: [-0.3, 0.0, -0.3],
                max: [0.3, 1.8, 0.3],
                pick_radius: 0.0,
            },
            movements,
            supporting_block: None,
            on_ground: false,
            stepping_carefully: false,
            is_living: true,
            is_fox_or_bee: false,
            has_weaving: false,
            fire_immune: false,
            freeze_immune: false,
            ticks_frozen: 0,
            delta_movement: EntityVec3::default(),
            stuck_speed_multiplier: EntityVec3::default(),
            remaining_fire_ticks: 0,
            fall_distance: 4.0,
        }
    }

    #[test]
    fn rain_clears_private_fire_only_for_exposed_columns() {
        let world = rain_world(0.8, true, None, false);
        assert_eq!(evaluate_fire_in_rain(&world, 100), 0);

        let negative_fire_immune_offset = rain_world(0.8, true, None, false);
        assert_eq!(evaluate_fire_in_rain(&negative_fire_immune_offset, -20), 0);

        let covered = rain_world(0.8, true, Some(10), false);
        assert_eq!(evaluate_fire_in_rain(&covered, 100), 100);

        let dark_sky = rain_world(0.8, true, None, true);
        assert_eq!(evaluate_fire_in_rain(&dark_sky, 100), 100);
    }

    #[test]
    fn snow_and_dry_biomes_keep_private_fire_in_rain() {
        let snow = rain_world(-0.5, true, None, false);
        assert_eq!(evaluate_fire_in_rain(&snow, 100), 100);

        let dry = rain_world(0.8, false, None, false);
        assert_eq!(evaluate_fire_in_rain(&dry, 100), 100);
    }

    fn evaluate_fire_in_rain(world: &WorldStore, remaining_fire_ticks: i32) -> i32 {
        let mut block_effect_world = EntityBlockEffectWorld::new(
            world.dimension,
            &world.registries,
            &world.chunks,
            EntityBlockEffectWeather {
                raining: true,
                sea_level: 63,
            },
        );
        let evaluation = block_effect_world.evaluate(EntityBlockEffectQuery {
            position: EntityVec3 {
                x: 0.5,
                y: 0.0,
                z: 0.5,
            },
            bounds: crate::EntityPickBoundsState {
                min: [-0.3, 0.0, -0.3],
                max: [0.3, 1.8, 0.3],
                pick_radius: 0.0,
            },
            movements: EntityBlockEffectMovementIter::Empty,
            supporting_block: None,
            on_ground: false,
            stepping_carefully: false,
            is_living: true,
            is_fox_or_bee: false,
            has_weaving: false,
            fire_immune: false,
            freeze_immune: false,
            ticks_frozen: 0,
            delta_movement: EntityVec3::default(),
            stuck_speed_multiplier: EntityVec3::default(),
            remaining_fire_ticks,
            fall_distance: 0.0,
        });
        evaluation.remaining_fire_ticks
    }

    /// A one-chunk world whose biome 0 decodes with the given climate, with an
    /// optional `MOTION_BLOCKING` first-available height for column (0, 0) and
    /// an optional empty sky-light section over the whole build height.
    fn rain_world(
        temperature: f32,
        has_precipitation: bool,
        motion_blocking_height: Option<i32>,
        empty_sky: bool,
    ) -> WorldStore {
        let mut world = test_world();
        if motion_blocking_height.is_some() || empty_sky {
            let mut heightmaps = Vec::new();
            if let Some(height) = motion_blocking_height {
                // 5 bits per entry for a 16-block build height, 12 entries per
                // long: entry 0 is column (0, 0).
                let mut data = vec![0_i64; 22];
                data[0] = i64::from(height);
                heightmaps.push(crate::HeightmapData {
                    kind_id: bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
                    data,
                });
            }
            let mut light = LightData::default();
            if empty_sky {
                // The single block section maps to light section bit 1.
                light.empty_sky_y_mask = vec![0b10];
            }
            world.insert_decoded_chunk(ChunkColumn {
                pos: ChunkPos { x: 0, z: 0 },
                state: ChunkState::Decoded,
                heightmaps,
                sections: vec![ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        crate::vanilla_air_block_state_id(),
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                }],
                block_entities: Vec::new(),
                light,
            });
        }
        world.record_registry_entries(
            "minecraft:worldgen/biome",
            0,
            vec![crate::registries::RegistryPacketEntry::with_raw_data(
                "minecraft:plains",
                crate::registries::test_biome_element_nbt(temperature, has_precipitation),
            )],
        );
        world
    }

    fn test_world() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    crate::vanilla_air_block_state_id(),
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        world
    }

    fn set_block<const N: usize>(
        world: &mut WorldStore,
        pos: BlockPos,
        name: &str,
        properties: [(&str, &str); N],
    ) {
        let properties: BTreeMap<String, String> = properties
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        let registry = crate::registries::BlockStateRegistry::vanilla_26_1();
        let state = if properties.is_empty() {
            registry.iter().find(|state| state.name == name)
        } else {
            registry.find_by_name_and_properties(name, &properties)
        };
        let block_state_id = state
            .unwrap_or_else(|| panic!("missing Vanilla block state {name} {properties:?}"))
            .id;
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: bbb_protocol::packets::BlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z
                },
                block_state_id,
            })
        );
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }
}
