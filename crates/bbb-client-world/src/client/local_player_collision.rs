use std::collections::BTreeMap;

use bbb_protocol::block_state_types::{
    vanilla_block_state_name, vanilla_block_state_static_collision_shape,
    vanilla_block_state_support_facts, VanillaBlockSupportDirection, VanillaBlockSupportType,
};
use bbb_protocol::block_types::{
    vanilla_block_type_for_resource_id, VanillaPropagatesSkylightDownOverride,
};
use bbb_protocol::entity_poses::{VANILLA_ENTITY_POSE_DIGGING_ID, VANILLA_ENTITY_POSE_EMERGING_ID};
use bbb_protocol::entity_types::{
    vanilla_entity_type, vanilla_entity_type_is_powder_snow_walkable,
    VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID,
    VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    VANILLA_ENTITY_TYPE_INTERACTION_ID, VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID,
    VANILLA_ENTITY_TYPE_MARKER_ID, VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
    VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
    VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID,
};
use bbb_protocol::packets::{
    Direction as ProtocolDirection, EquipmentSlot as ProtocolEquipmentSlot,
};

use super::local_player::LocalPlayerPoseState;
use crate::entities::{
    is_vanilla_boat_type, is_vanilla_minecart_type,
    vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type,
    vanilla_entity_metadata_happy_ghast_accessors_for_entity_type,
    vanilla_entity_metadata_type_collidability_kind, vanilla_entity_metadata_type_pushability_kind,
    vanilla_living_entity_type, EntityInterpolationCollisionInput, EntitySupportingBlock,
    VanillaEntityCollidabilityKind, VanillaEntityPushabilityKind,
};
use crate::terrain::{
    block_name_is_air, block_name_is_bars_or_pane, block_name_is_copper_grate,
    block_name_is_enchantment_power_provider, block_name_is_enchantment_power_transmitter,
    block_name_is_fence, block_name_is_fence_gate, block_name_is_fire,
    block_name_is_fire_flammable, block_name_is_hanging_sign, block_name_is_impermeable,
    block_name_is_leaves, block_name_is_shulker_box, block_name_is_trapdoor, block_name_is_wall,
    block_name_resets_fall_damage, block_name_triggers_dried_ghast_ambient_sound,
};
use crate::{BlockPos, BlockProbe, BlockView, PistonDirection, TerrainFluidKind, WorldStore};

mod block_shapes;
mod entity_queries;
mod geometry;
mod movement_clip;
mod piston;
mod sturdy_and_supporting;
mod support;
pub(crate) use block_shapes::*;
pub(super) use geometry::*;
use piston::*;
use support::*;

pub(super) fn block_has_collision_shape(block: &BlockView) -> bool {
    block_collision_shape_empty_context(block, block.pos).is_some()
}

pub(super) use bbb_facts::collision::COLLISION_EPSILON;
const VANILLA_COLLISION_CONTEXT_ABOVE_EPSILON: f64 = 1.0e-5;
const LOCAL_PLAYER_POWDER_SNOW_FALLING_COLLISION_FALL_DISTANCE: f64 = 2.5;
const LOCAL_PLAYER_POWDER_SNOW_FALLING_COLLISION_HEIGHT: f64 = 0.9;
const SUPPORTING_BLOCK_PROBE_DEPTH: f64 = 1.0e-6;
const MAX_COLLISION_BOXES: usize = 16;
const PX: f64 = 1.0 / 16.0;
const FALL_DAMAGE_RESETTING_BLOCK_TAG: &str = "minecraft:fall_damage_resetting";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleBlockFluidSurfaceSample {
    pub block_collision_height: f64,
    pub fluid_height: f64,
    pub fluid_kind: Option<TerrainFluidKind>,
    pub block_is_air: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvironmentAnimateBlockSample {
    pub fluid: Option<crate::TerrainFluidState>,
    pub block_is_air: bool,
    pub can_occlude: bool,
    pub solid_render: bool,
    /// `BlockState.isFaceSturdy(..., SupportType.FULL)` in Vanilla
    /// `Direction.values()` order: down, up, north, south, west, east.
    pub full_face_sturdy: [bool; 6],
    /// `Block.isFaceFull(collisionShape, direction)` in Vanilla
    /// `Direction.values()` order: down, up, north, south, west, east.
    pub collision_full_face: [bool; 6],
    pub fire_flammable: bool,
    pub triggers_dried_ghast_ambient_sound: bool,
    pub enchantment_power_provider: bool,
    pub enchantment_power_transmitter: bool,
    pub impermeable: bool,
    pub collision_bounds: Option<EnvironmentAnimateCollisionBounds>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvironmentAnimateCollisionBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProjectileBlockMovementClip {
    pub(crate) movement: [f64; 3],
    pub(crate) world_border_hit: bool,
    /// Global block-state id at the hit cell. `None` identifies a miss.
    pub(crate) block_state_id: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BlockShapeHit {
    scale: f64,
    pos: BlockPos,
    block_state_id: i32,
}

/// One world-space AABB from `CollisionGetter.getCollisions` near the active
/// camera entity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldCollisionBox {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

/// One block highlighted by vanilla's `SupportBlockRenderer`: the block cell
/// itself is outlined separately from every component of its empty-context
/// collision shape.
#[derive(Debug, Clone, PartialEq)]
pub struct DebugSupportingBlockHighlight {
    pub pos: BlockPos,
    pub collision_boxes: Vec<WorldCollisionBox>,
}

/// The current local player's main supporting block and, when it differs, the
/// legacy effect block returned by `Entity.getOnPosLegacy()`.
#[derive(Debug, Clone, PartialEq)]
pub struct DebugLocalSupportingBlocks {
    pub supporting: DebugSupportingBlockHighlight,
    pub legacy_effect: Option<DebugSupportingBlockHighlight>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugEntitySupportingBlocks {
    pub entity_id: i32,
    pub supporting: DebugSupportingBlockHighlight,
    pub legacy_effect: Option<DebugSupportingBlockHighlight>,
}

#[derive(Debug, Clone, Copy)]
struct EntityCollisionQuery {
    source_entity_id: Option<i32>,
    source_entity_type_id: i32,
    source_position: [f64; 3],
    bounds: WorldCollisionBox,
    block_context: LocalPlayerCollisionContext,
}

impl Default for ParticleBlockFluidSurfaceSample {
    fn default() -> Self {
        Self {
            block_collision_height: 0.0,
            fluid_height: 0.0,
            fluid_kind: None,
            block_is_air: true,
        }
    }
}

impl ParticleBlockFluidSurfaceSample {
    pub fn max_surface_height(self) -> f64 {
        self.block_collision_height.max(self.fluid_height)
    }
}

pub(super) fn local_player_collides(world: &WorldStore, bounds: WorldCollisionBox) -> bool {
    local_player_collides_with_context(
        world,
        bounds,
        LocalPlayerCollisionContext::for_bounds(bounds),
    )
}

pub(super) fn local_player_collides_with_context(
    world: &WorldStore,
    bounds: WorldCollisionBox,
    context: LocalPlayerCollisionContext,
) -> bool {
    // Some vanilla collision shapes extend outside their block cell, such as
    // piston heads and fence heights.
    let min_x = block_floor(bounds.min[0] + COLLISION_EPSILON).saturating_sub(1);
    let max_x = block_floor(bounds.max[0] - COLLISION_EPSILON).saturating_add(1);
    let min_y = block_floor(bounds.min[1] + COLLISION_EPSILON).saturating_sub(1);
    let max_y = block_floor(bounds.max[1] - COLLISION_EPSILON).saturating_add(1);
    let min_z = block_floor(bounds.min[2] + COLLISION_EPSILON).saturating_sub(1);
    let max_z = block_floor(bounds.max[2] - COLLISION_EPSILON).saturating_add(1);

    let mut collides = false;
    world.visit_loaded_chunk_block_positions([min_x, min_y, min_z], [max_x, max_y, max_z], |pos| {
        if collides {
            return;
        }
        let Some(block) = world.block_view(pos) else {
            return;
        };
        if block_collides_with_local_player_bounds(world, &block, pos, bounds, context) {
            collides = true;
        }
    });
    collides
}

fn local_player_collides_with_suffocating_block(
    world: &WorldStore,
    bounds: WorldCollisionBox,
    context: LocalPlayerCollisionContext,
) -> bool {
    let min_x = block_floor(bounds.min[0] - COLLISION_EPSILON).saturating_sub(1);
    let max_x = block_floor(bounds.max[0] + COLLISION_EPSILON).saturating_add(1);
    let min_y = block_floor(bounds.min[1] - COLLISION_EPSILON).saturating_sub(1);
    let max_y = block_floor(bounds.max[1] + COLLISION_EPSILON).saturating_add(1);
    let min_z = block_floor(bounds.min[2] - COLLISION_EPSILON).saturating_sub(1);
    let max_z = block_floor(bounds.max[2] + COLLISION_EPSILON).saturating_add(1);
    let mut suffocating = false;
    world.visit_loaded_chunk_block_positions([min_x, min_y, min_z], [max_x, max_y, max_z], |pos| {
        if suffocating {
            return;
        }
        let Some(block) = world.block_view(pos) else {
            return;
        };
        if !vanilla_block_state_support_facts(block.block_state_id)
            .is_some_and(|facts| facts.suffocating())
        {
            return;
        }
        if block_collision_shape_with_context(world, &block, pos, context)
            .is_some_and(|shape| bounds_intersects_block_shape(bounds, pos, shape))
        {
            suffocating = true;
        }
    });
    suffocating
}

/// Vanilla nether-portal `entityInside` occupancy for the local player: the
/// pose AABB (deflated `1.0E-7` like `Entity.checkInsideBlocks`) intersects a
/// `minecraft:nether_portal` block, which is the only CONFUSION-transition
/// portal.
pub(super) fn local_player_inside_nether_portal(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
) -> bool {
    const INSIDE_EPSILON: f64 = 1.0e-7;
    let bounds = world.local_player_bounds(pose);
    let min_x = block_floor(bounds.min[0] + INSIDE_EPSILON);
    let max_x = block_floor(bounds.max[0] - INSIDE_EPSILON);
    let min_y = block_floor(bounds.min[1] + INSIDE_EPSILON);
    let max_y = block_floor(bounds.max[1] - INSIDE_EPSILON);
    let min_z = block_floor(bounds.min[2] + INSIDE_EPSILON);
    let max_z = block_floor(bounds.max[2] - INSIDE_EPSILON);
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                let inside = world
                    .block_view(BlockPos { x, y, z })
                    .and_then(|probe| probe.block_name)
                    .is_some_and(|name| name == "minecraft:nether_portal");
                if inside {
                    return true;
                }
            }
        }
    }
    false
}

/// Geometry half of Vanilla `DismountHelper.findSafeDismountLocation`.
///
/// The caller owns the EntityType-specific dangerous-block predicates. This
/// helper resolves the non-climbable floor, fixed EntityType bounds, empty
/// collision-context block shapes, and exact world-border containment.
pub(crate) fn entity_type_safe_dismount_geometry(
    world: &WorldStore,
    entity_type_id: i32,
    block_pos: BlockPos,
) -> Option<[f64; 3]> {
    let floor_height = non_climbable_floor_height(world, block_pos);
    if !floor_height.is_finite() || floor_height >= 1.0 {
        return None;
    }
    let position = [
        f64::from(block_pos.x) + 0.5,
        f64::from(block_pos.y) + floor_height,
        f64::from(block_pos.z) + 0.5,
    ];
    let entity_type = vanilla_entity_type(entity_type_id)?;
    let half_width = f64::from(entity_type.width) * 0.5;
    let bounds = WorldCollisionBox {
        min: [
            position[0] - half_width,
            position[1],
            position[2] - half_width,
        ],
        max: [
            position[0] + half_width,
            position[1] + f64::from(entity_type.height),
            position[2] + half_width,
        ],
    };
    if empty_context_block_collision(world, bounds) {
        return None;
    }
    let border = world.world_border();
    let max_epsilon = 1.0e-5_f64;
    (bounds.min[0] >= border.min_x_at(0.0)
        && bounds.max[0] - max_epsilon < border.max_x_at(0.0)
        && bounds.min[2] >= border.min_z_at(0.0)
        && bounds.max[2] - max_epsilon < border.max_z_at(0.0))
    .then_some(position)
}

impl WorldStore {
    /// Mirrors the block- and world-border halves of
    /// `HangingEntity.hasLevelCollision` for one prospective non-living
    /// entity. The entity position, rather than its AABB minimum, seeds
    /// `EntityCollisionContext.entityBottom`, matching Vanilla's constructor.
    pub(crate) fn local_non_living_entity_block_and_border_bounds_are_free(
        &self,
        entity_type_id: i32,
        position: [f64; 3],
        min: [f64; 3],
        max: [f64; 3],
    ) -> Option<bool> {
        if !(0..3).all(|axis| {
            position[axis].is_finite()
                && min[axis].is_finite()
                && max[axis].is_finite()
                && min[axis] < max[axis]
        }) {
            return None;
        }
        let bounds = WorldCollisionBox { min, max };
        let block_context = LocalPlayerCollisionContext::new(position[1], false);
        if local_player_collides_with_context(self, bounds, block_context) {
            return Some(false);
        }
        let query = EntityCollisionQuery {
            source_entity_id: None,
            source_entity_type_id: entity_type_id,
            source_position: position,
            bounds,
            block_context,
        };
        Some(self.world_border_collision_boxes(query, bounds).is_empty())
    }

    /// Mirrors the block-shape half of
    /// `CollisionGetter.noCollision(null, bounds)`.
    ///
    /// A null source selects `CollisionContext.empty()`, so powder snow is
    /// empty while scaffolding uses its stable shape. Unloaded collision
    /// chunks are skipped just like `BlockCollisions`.
    pub(crate) fn local_empty_context_block_bounds_are_free(
        &self,
        min: [f64; 3],
        max: [f64; 3],
    ) -> Option<bool> {
        if !(0..3)
            .all(|axis| min[axis].is_finite() && max[axis].is_finite() && min[axis] < max[axis])
        {
            return None;
        }
        Some(!empty_context_block_collision(
            self,
            WorldCollisionBox { min, max },
        ))
    }

    /// Tests the separately owned local-player AABB against one entity query.
    /// `ClientLevel` stores `LocalPlayer` among level entities; bbb keeps its
    /// authoritative predicted pose outside `EntityStore`.
    pub(crate) fn local_player_bounds_intersect(
        &self,
        min: [f64; 3],
        max: [f64; 3],
    ) -> Option<bool> {
        if !(0..3)
            .all(|axis| min[axis].is_finite() && max[axis].is_finite() && min[axis] < max[axis])
        {
            return None;
        }
        if self.local_player_is_spectator() {
            return Some(false);
        }
        let Some(pose) = self.local_player_pose() else {
            return Some(false);
        };
        Some(world_collision_boxes_intersect(
            WorldCollisionBox { min, max },
            self.local_player_bounds(pose),
        ))
    }

    pub(crate) fn collect_collision_free_interpolation_targets(
        &self,
        inputs: &[EntityInterpolationCollisionInput],
        entity_collision_targets: &[crate::EntityPickTargetState],
    ) -> rustc_hash::FxHashSet<i32> {
        if inputs.is_empty() {
            return rustc_hash::FxHashSet::default();
        }
        let target_index = EntityCollisionTargetIndex::new(entity_collision_targets);
        inputs
            .iter()
            .filter(|input| {
                self.entity_interpolation_target_is_collision_free_indexed(**input, &target_index)
            })
            .map(|input| input.entity_id)
            .collect()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn entity_interpolation_target_is_collision_free(
        &self,
        input: EntityInterpolationCollisionInput,
        entity_collision_targets: &[crate::EntityPickTargetState],
    ) -> bool {
        self.entity_interpolation_target_is_collision_free_indexed(
            input,
            &EntityCollisionTargetIndex::linear(entity_collision_targets),
        )
    }

    fn entity_interpolation_target_is_collision_free_indexed(
        &self,
        input: EntityInterpolationCollisionInput,
        target_index: &EntityCollisionTargetIndex<'_>,
    ) -> bool {
        let target_bounds = WorldCollisionBox {
            min: [
                input.target_position.x + f64::from(input.bounds.min[0]),
                input.target_position.y + f64::from(input.bounds.min[1]),
                input.target_position.z + f64::from(input.bounds.min[2]),
            ],
            max: [
                input.target_position.x + f64::from(input.bounds.max[0]),
                input.target_position.y + f64::from(input.bounds.max[1]),
                input.target_position.z + f64::from(input.bounds.max[2]),
            ],
        };
        let context = LocalPlayerCollisionContext::for_remote_entity(
            target_bounds.min[1],
            self.entities
                .is_descending(input.entity_id)
                .unwrap_or(false),
            self.remote_entity_can_walk_on_powder_snow(input.entity_id),
            input.entity_type_id == VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        )
        .with_can_stand_on_lava(input.entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID);
        if local_player_collides_with_context(self, target_bounds, context) {
            return false;
        }

        let query = EntityCollisionQuery {
            source_entity_id: Some(input.entity_id),
            source_entity_type_id: input.entity_type_id,
            source_position: [
                input.target_position.x,
                input.target_position.y,
                input.target_position.z,
            ],
            bounds: target_bounds,
            block_context: context,
        };
        let collides_with_entity =
            target_index.any_in_aabb(target_bounds.min, target_bounds.max, |target| {
                if target.entity_id == input.entity_id
                    || !self.entity_passes_spectator_filter(target.entity_id)
                    || self
                        .debug_entities_share_root_vehicle(Some(input.entity_id), target.entity_id)
                {
                    return false;
                }
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
                if !world_collision_boxes_intersect(target_box, target_bounds) {
                    return false;
                }
                let Some(target_type) = self.entity_type_id(target.entity_id) else {
                    return false;
                };
                self.debug_entities_can_collide(
                    query,
                    target.entity_id,
                    target_type,
                    [target.position.x, target.position.y, target.position.z],
                    target_box,
                )
            });
        if collides_with_entity {
            return false;
        }

        let width = (target_bounds.max[0] - target_bounds.min[0])
            .abs()
            .max((target_bounds.max[2] - target_bounds.min[2]).abs())
            .max(1.0);
        let border = self.world_border();
        let source_is_close = border
            .distance_to_border(input.current_position.x, input.current_position.z)
            < width * 2.0
            && input.current_position.x >= border.min_x_at(0.0) - width
            && input.current_position.x < border.max_x_at(0.0) + width
            && input.current_position.z >= border.min_z_at(0.0) - width
            && input.current_position.z < border.max_z_at(0.0) + width;
        if !source_is_close {
            return true;
        }
        let collision_min_x = border.min_x_at(0.0).floor();
        let collision_min_z = border.min_z_at(0.0).floor();
        let collision_max_x = border.max_x_at(0.0).ceil();
        let collision_max_z = border.max_z_at(0.0).ceil();
        target_bounds.min[0] >= collision_min_x
            && target_bounds.max[0] <= collision_max_x
            && target_bounds.min[2] >= collision_min_z
            && target_bounds.max[2] <= collision_max_z
    }

    /// Exact empty-hand `ScaffoldingBlock.getShape(...).toAabbs()` projection,
    /// derived from the same shape constants and voxel-union merge used by
    /// collision queries. `None` means the probe is not valid scaffolding.
    pub fn debug_scaffolding_outline_shape_boxes(
        &self,
        block: &BlockProbe,
    ) -> Option<Vec<([f64; 3], [f64; 3])>> {
        if block.block_name.as_deref() != Some("minecraft:scaffolding") {
            return None;
        }
        let bottom = bool_property(&block.block_properties, "bottom")?;
        Some(
            scaffolding_outline_shape(bottom)
                .to_aabbs()
                .into_iter()
                .map(|shape_box| (shape_box.min, shape_box.max))
                .collect(),
        )
    }

    /// Context-dependent selection shapes for the local player's block queries.
    pub fn local_player_contextual_outline_shape_boxes(
        &self,
        block: &BlockView,
    ) -> Option<Vec<([f64; 3], [f64; 3])>> {
        let shape = match block.block_name {
            Some("minecraft:scaffolding") => {
                // ScaffoldingBlock.getShape / EntityCollisionContext.isHoldingItem use the main hand.
                let holding_scaffolding = self
                    .local_item_in_hand(bbb_protocol::packets::InteractionHand::MainHand)
                    .is_some_and(|stack| {
                        stack.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_SCAFFOLDING_ID)
                    });
                if holding_scaffolding {
                    BlockCollisionShape::single(WorldCollisionBox::FULL)
                } else {
                    scaffolding_outline_shape(bool_property(block.block_properties, "bottom")?)
                }
            }
            Some(name) if is_shulker_box_block_name(name) => {
                // ShulkerBoxBlock.getShape uses the same current-progress box as collision.
                shulker_box_collision_shape(self, block, block.pos)?
            }
            _ => return None,
        };
        Some(
            shape
                .to_aabbs()
                .into_iter()
                .map(|shape_box| (shape_box.min, shape_box.max))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests;

const COLLISION_INDEX_MIN_TARGETS: usize = 8;
const COLLISION_INDEX_MAX_TARGET_SECTIONS_PER_AXIS: i32 = 4;
const COLLISION_INDEX_MAX_QUERY_SECTIONS: i64 = 64;

fn collision_section_coord(value: f64) -> Option<i32> {
    if !value.is_finite() {
        return None;
    }
    let floored = value.floor();
    if floored < f64::from(i32::MIN) || floored > f64::from(i32::MAX) {
        return None;
    }
    Some((floored as i32) >> 4)
}

struct EntityCollisionTargetIndex<'a> {
    targets: &'a [crate::EntityPickTargetState],
    sections: Option<rustc_hash::FxHashMap<(i32, i32, i32), Vec<usize>>>,
    oversized: Vec<usize>,
}

impl<'a> EntityCollisionTargetIndex<'a> {
    fn linear(targets: &'a [crate::EntityPickTargetState]) -> Self {
        Self {
            targets,
            sections: None,
            oversized: Vec::new(),
        }
    }

    fn new(targets: &'a [crate::EntityPickTargetState]) -> Self {
        if targets.len() <= COLLISION_INDEX_MIN_TARGETS {
            return Self::linear(targets);
        }
        let mut sections: rustc_hash::FxHashMap<(i32, i32, i32), Vec<usize>> =
            rustc_hash::FxHashMap::default();
        let mut oversized = Vec::new();
        for (idx, target) in targets.iter().enumerate() {
            let min_x = target.position.x + f64::from(target.bounds.min[0]);
            let min_y = target.position.y + f64::from(target.bounds.min[1]);
            let min_z = target.position.z + f64::from(target.bounds.min[2]);
            let max_x = target.position.x + f64::from(target.bounds.max[0]);
            let max_y = target.position.y + f64::from(target.bounds.max[1]);
            let max_z = target.position.z + f64::from(target.bounds.max[2]);
            let (
                Some(min_sx),
                Some(min_sy),
                Some(min_sz),
                Some(max_sx),
                Some(max_sy),
                Some(max_sz),
            ) = (
                collision_section_coord(min_x),
                collision_section_coord(min_y),
                collision_section_coord(min_z),
                collision_section_coord(max_x),
                collision_section_coord(max_y),
                collision_section_coord(max_z),
            )
            else {
                oversized.push(idx);
                continue;
            };
            if max_sx.saturating_sub(min_sx) >= COLLISION_INDEX_MAX_TARGET_SECTIONS_PER_AXIS
                || max_sy.saturating_sub(min_sy) >= COLLISION_INDEX_MAX_TARGET_SECTIONS_PER_AXIS
                || max_sz.saturating_sub(min_sz) >= COLLISION_INDEX_MAX_TARGET_SECTIONS_PER_AXIS
            {
                oversized.push(idx);
                continue;
            }
            for sy in min_sy..=max_sy {
                for sz in min_sz..=max_sz {
                    for sx in min_sx..=max_sx {
                        sections.entry((sx, sy, sz)).or_default().push(idx);
                    }
                }
            }
        }
        Self {
            targets,
            sections: Some(sections),
            oversized,
        }
    }

    fn any_in_aabb(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
        mut predicate: impl FnMut(&crate::EntityPickTargetState) -> bool,
    ) -> bool {
        let Some(sections) = self.sections.as_ref() else {
            return self.targets.iter().any(predicate);
        };
        let (Some(min_sx), Some(min_sy), Some(min_sz), Some(max_sx), Some(max_sy), Some(max_sz)) = (
            collision_section_coord(query_min[0]),
            collision_section_coord(query_min[1]),
            collision_section_coord(query_min[2]),
            collision_section_coord(query_max[0]),
            collision_section_coord(query_max[1]),
            collision_section_coord(query_max[2]),
        ) else {
            return self.targets.iter().any(predicate);
        };
        let span_x = i64::from(max_sx) - i64::from(min_sx) + 1;
        let span_y = i64::from(max_sy) - i64::from(min_sy) + 1;
        let span_z = i64::from(max_sz) - i64::from(min_sz) + 1;
        let Some(total_cells) = span_x
            .checked_mul(span_y)
            .and_then(|v| v.checked_mul(span_z))
        else {
            return self.targets.iter().any(predicate);
        };
        if total_cells > COLLISION_INDEX_MAX_QUERY_SECTIONS
            || total_cells as usize > self.targets.len()
        {
            return self.targets.iter().any(predicate);
        }
        for &idx in &self.oversized {
            if predicate(&self.targets[idx]) {
                return true;
            }
        }
        for sy in min_sy..=max_sy {
            for sz in min_sz..=max_sz {
                for sx in min_sx..=max_sx {
                    if let Some(bucket) = sections.get(&(sx, sy, sz)) {
                        for &idx in bucket {
                            if predicate(&self.targets[idx]) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }
}
