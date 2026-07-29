use std::collections::BTreeMap;

use bbb_protocol::block_state_types::{
    vanilla_block_state_static_collision_shape, vanilla_block_state_support_facts,
    VanillaBlockSupportDirection, VanillaBlockSupportType,
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
    block_name_has_empty_collision_shape, block_name_is_air, block_name_is_amethyst_cluster,
    block_name_is_anvil, block_name_is_bars_or_pane, block_name_is_bed, block_name_is_campfire,
    block_name_is_candle, block_name_is_candle_cake, block_name_is_cauldron, block_name_is_chain,
    block_name_is_chest, block_name_is_copper_golem_statue, block_name_is_copper_grate,
    block_name_is_door, block_name_is_enchantment_power_provider,
    block_name_is_enchantment_power_transmitter, block_name_is_fence, block_name_is_fence_gate,
    block_name_is_fire, block_name_is_fire_flammable, block_name_is_floor_skull,
    block_name_is_flower_pot, block_name_is_hanging_sign, block_name_is_impermeable,
    block_name_is_lantern, block_name_is_leaves, block_name_is_rod, block_name_is_shelf,
    block_name_is_shulker_box, block_name_is_slab, block_name_is_stair, block_name_is_trapdoor,
    block_name_is_wall, block_name_is_wall_skull, block_name_is_wool_carpet,
    block_name_triggers_dried_ghast_ambient_sound,
};
use crate::{
    BlockPos, BlockProbe, PistonDirection, TerrainFluidKind, TerrainMaterialClass, WorldStore,
};

pub(super) const COLLISION_EPSILON: f64 = 1.0e-7;
const VANILLA_COLLISION_CONTEXT_ABOVE_EPSILON: f64 = 1.0e-5;
const LOCAL_PLAYER_POWDER_SNOW_FALLING_COLLISION_FALL_DISTANCE: f64 = 2.5;
const LOCAL_PLAYER_POWDER_SNOW_FALLING_COLLISION_HEIGHT: f64 = 0.9;
pub(super) const LOCAL_PLAYER_HALF_WIDTH: f64 = 0.3;
const SUPPORTING_BLOCK_PROBE_DEPTH: f64 = 1.0e-6;
const MAX_COLLISION_BOXES: usize = 16;
const PX: f64 = 1.0 / 16.0;

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
    bounds: LocalPlayerBounds,
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

pub(super) fn local_player_collides(world: &WorldStore, bounds: LocalPlayerBounds) -> bool {
    local_player_collides_with_context(
        world,
        bounds,
        LocalPlayerCollisionContext::for_bounds(bounds),
    )
}

pub(super) fn local_player_collides_with_context(
    world: &WorldStore,
    bounds: LocalPlayerBounds,
    context: LocalPlayerCollisionContext,
) -> bool {
    // Some vanilla collision shapes extend outside their block cell, such as
    // piston heads and fence heights.
    let min_x = block_floor(bounds.min_x + COLLISION_EPSILON) - 1;
    let max_x = block_floor(bounds.max_x - COLLISION_EPSILON) + 1;
    let min_y = block_floor(bounds.min_y + COLLISION_EPSILON) - 1;
    let max_y = block_floor(bounds.max_y - COLLISION_EPSILON) + 1;
    let min_z = block_floor(bounds.min_z + COLLISION_EPSILON) - 1;
    let max_z = block_floor(bounds.max_z - COLLISION_EPSILON) + 1;

    for y in min_y..=max_y {
        for z in min_z..=max_z {
            for x in min_x..=max_x {
                let Some(block) = world.probe_block(BlockPos { x, y, z }) else {
                    continue;
                };
                if block_collides_with_local_player_bounds(
                    world,
                    &block,
                    BlockPos { x, y, z },
                    bounds,
                    context,
                ) {
                    return true;
                }
            }
        }
    }
    false
}

fn local_player_collides_with_suffocating_block(
    world: &WorldStore,
    bounds: LocalPlayerBounds,
    context: LocalPlayerCollisionContext,
) -> bool {
    let min_x = block_floor(bounds.min_x - COLLISION_EPSILON) - 1;
    let max_x = block_floor(bounds.max_x + COLLISION_EPSILON) + 1;
    let min_y = block_floor(bounds.min_y - COLLISION_EPSILON) - 1;
    let max_y = block_floor(bounds.max_y + COLLISION_EPSILON) + 1;
    let min_z = block_floor(bounds.min_z - COLLISION_EPSILON) - 1;
    let max_z = block_floor(bounds.max_z + COLLISION_EPSILON) + 1;
    for z in min_z..=max_z {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let pos = BlockPos { x, y, z };
                let Some(block) = world.probe_block(pos) else {
                    continue;
                };
                if !vanilla_block_state_support_facts(block.block_state_id)
                    .is_some_and(|facts| facts.suffocating())
                {
                    continue;
                }
                if block_collision_shape_with_context(world, &block, pos, context)
                    .is_some_and(|shape| bounds_intersects_block_shape(bounds, pos, shape))
                {
                    return true;
                }
            }
        }
    }
    false
}

fn find_supporting_block(
    world: &WorldStore,
    entity_position: [f64; 3],
    bounds: LocalPlayerBounds,
    context: LocalPlayerCollisionContext,
) -> Option<BlockPos> {
    if !entity_position
        .iter()
        .all(|coordinate| coordinate.is_finite())
    {
        return None;
    }
    let test_bounds = LocalPlayerBounds {
        min_y: bounds.min_y - SUPPORTING_BLOCK_PROBE_DEPTH,
        max_y: bounds.min_y,
        ..bounds
    };
    let min_x = block_floor(test_bounds.min_x - COLLISION_EPSILON) - 1;
    let max_x = block_floor(test_bounds.max_x + COLLISION_EPSILON) + 1;
    let min_y = block_floor(test_bounds.min_y - COLLISION_EPSILON) - 1;
    let max_y = block_floor(test_bounds.max_y + COLLISION_EPSILON) + 1;
    let min_z = block_floor(test_bounds.min_z - COLLISION_EPSILON) - 1;
    let max_z = block_floor(test_bounds.max_z + COLLISION_EPSILON) + 1;

    let mut main_support = None;
    let mut main_support_distance = f64::MAX;
    // Vanilla `Cursor3D` advances X fastest, then Y, then Z. Selection is
    // order-independent because exact distance ties use `BlockPos.compareTo`.
    for z in min_z..=max_z {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let pos = BlockPos { x, y, z };
                let Some(block) = world.probe_block(pos) else {
                    continue;
                };
                let Some(shape) = block_collision_shape_with_context(world, &block, pos, context)
                else {
                    continue;
                };
                if !bounds_intersects_block_shape(test_bounds, pos, shape) {
                    continue;
                }
                let dx = f64::from(x) + 0.5 - entity_position[0];
                let dy = f64::from(y) + 0.5 - entity_position[1];
                let dz = f64::from(z) + 0.5 - entity_position[2];
                let distance = dx * dx + dy * dy + dz * dz;
                let wins_tie = distance == main_support_distance
                    && main_support.is_none_or(|current: BlockPos| {
                        (current.y, current.z, current.x) < (pos.y, pos.z, pos.x)
                    });
                if distance < main_support_distance || wins_tie {
                    main_support = Some(pos);
                    main_support_distance = distance;
                }
            }
        }
    }
    main_support
}

pub(crate) fn block_collision_shape_is_full_block(block: &BlockProbe, pos: BlockPos) -> bool {
    block_collision_shape(block, pos).is_some_and(BlockCollisionShape::is_full_block)
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
    let bounds = LocalPlayerBounds::from_min_max(
        [
            position[0] - half_width,
            position[1],
            position[2] - half_width,
        ],
        [
            position[0] + half_width,
            position[1] + f64::from(entity_type.height),
            position[2] + half_width,
        ],
    );
    if empty_context_block_collision(world, bounds) {
        return None;
    }
    let border = world.world_border();
    let max_epsilon = 1.0e-5_f64;
    (bounds.min_x >= border.min_x_at(0.0)
        && bounds.max_x - max_epsilon < border.max_x_at(0.0)
        && bounds.min_z >= border.min_z_at(0.0)
        && bounds.max_z - max_epsilon < border.max_z_at(0.0))
    .then_some(position)
}

fn non_climbable_floor_height(world: &WorldStore, pos: BlockPos) -> f64 {
    let shape_max_y = |probe: Option<BlockProbe>| {
        let probe = probe?;
        let name = probe.block_name.as_deref()?;
        if matches!(
            name,
            "minecraft:ladder"
                | "minecraft:vine"
                | "minecraft:scaffolding"
                | "minecraft:weeping_vines"
                | "minecraft:weeping_vines_plant"
                | "minecraft:twisting_vines"
                | "minecraft:twisting_vines_plant"
                | "minecraft:cave_vines"
                | "minecraft:cave_vines_plant"
        ) || (block_name_is_trapdoor(name)
            && probe.block_properties.get("open").map(String::as_str) == Some("true"))
        {
            return None;
        }
        block_collision_shape_empty_context(&probe, probe.pos).and_then(|shape| {
            shape
                .boxes()
                .map(|shape_box| shape_box.max_y)
                .reduce(f64::max)
        })
    };
    if let Some(max_y) = shape_max_y(world.probe_block(pos)) {
        return max_y;
    }
    let below = BlockPos {
        y: pos.y.saturating_sub(1),
        ..pos
    };
    shape_max_y(world.probe_block(below)).map_or(f64::NEG_INFINITY, |max_y| {
        if max_y >= 1.0 {
            max_y - 1.0
        } else {
            f64::NEG_INFINITY
        }
    })
}

fn empty_context_block_collision(world: &WorldStore, bounds: LocalPlayerBounds) -> bool {
    let min_x = block_floor(bounds.min_x + COLLISION_EPSILON) - 1;
    let max_x = block_floor(bounds.max_x - COLLISION_EPSILON) + 1;
    let min_y = block_floor(bounds.min_y + COLLISION_EPSILON) - 1;
    let max_y = block_floor(bounds.max_y - COLLISION_EPSILON) + 1;
    let min_z = block_floor(bounds.min_z + COLLISION_EPSILON) - 1;
    let max_z = block_floor(bounds.max_z - COLLISION_EPSILON) + 1;
    for y in min_y..=max_y {
        for z in min_z..=max_z {
            for x in min_x..=max_x {
                let pos = BlockPos { x, y, z };
                let Some(block) = world.probe_block(pos) else {
                    continue;
                };
                let shape = if block.block_name.as_deref() == Some("minecraft:moving_piston") {
                    moving_piston_collision_shape(
                        world,
                        &block,
                        pos,
                        LocalPlayerCollisionContext::for_bounds(bounds),
                        false,
                    )
                } else {
                    block_collision_shape_empty_context(&block, pos)
                };
                if shape.is_some_and(|shape| bounds_intersects_block_shape(bounds, pos, shape)) {
                    return true;
                }
            }
        }
    }
    false
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
        let bounds = LocalPlayerBounds::from_min_max(min, max);
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
            LocalPlayerBounds::from_min_max(min, max),
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
        Some(world_collision_box_intersects_bounds(
            WorldCollisionBox { min, max },
            LocalPlayerBounds::for_pose(pose),
        ))
    }

    pub(crate) fn entity_interpolation_target_is_collision_free(
        &self,
        input: EntityInterpolationCollisionInput,
        entity_collision_targets: &[crate::EntityPickTargetState],
    ) -> bool {
        let target_bounds = LocalPlayerBounds::from_min_max(
            [
                input.target_position.x + f64::from(input.bounds.min[0]),
                input.target_position.y + f64::from(input.bounds.min[1]),
                input.target_position.z + f64::from(input.bounds.min[2]),
            ],
            [
                input.target_position.x + f64::from(input.bounds.max[0]),
                input.target_position.y + f64::from(input.bounds.max[1]),
                input.target_position.z + f64::from(input.bounds.max[2]),
            ],
        );
        let context = LocalPlayerCollisionContext::for_remote_entity(
            target_bounds.min_y,
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
        let collides_with_entity = entity_collision_targets.iter().any(|target| {
            if target.entity_id == input.entity_id
                || !self.entity_passes_spectator_filter(target.entity_id)
                || self.debug_entities_share_root_vehicle(Some(input.entity_id), target.entity_id)
            {
                return false;
            }
            let Some(target_type) = self.entity_type_id(target.entity_id) else {
                return false;
            };
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
            self.debug_entities_can_collide(
                query,
                target.entity_id,
                target_type,
                [target.position.x, target.position.y, target.position.z],
                target_box,
            ) && world_collision_box_intersects_bounds(target_box, target_bounds)
        });
        if collides_with_entity {
            return false;
        }

        let width = (target_bounds.max_x - target_bounds.min_x)
            .abs()
            .max((target_bounds.max_z - target_bounds.min_z).abs())
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
        target_bounds.min_x >= collision_min_x
            && target_bounds.max_x <= collision_max_x
            && target_bounds.min_z >= collision_min_z
            && target_bounds.max_z <= collision_max_z
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
        let stable = scaffolding_stable_collision_shape();
        let mut builder = BlockCollisionShapeBuilder::new();
        if bottom {
            builder.push(BlockCollisionBox::SCAFFOLDING_UNSTABLE_BOTTOM);
        }
        for shape_box in stable.boxes() {
            builder.push(shape_box);
        }
        if bottom {
            let north_brace = BlockCollisionBox {
                min_x: 0.0,
                min_y: 7.0 * PX,
                min_z: 0.0,
                max_x: 1.0,
                max_y: 9.0 * PX,
                max_z: 2.0 * PX,
            };
            let mut brace = north_brace;
            for _ in 0..4 {
                builder.push(brace);
                brace = brace.rotate_y_90();
            }
        }
        Some(
            builder
                .build()
                .to_aabbs()
                .into_iter()
                .map(|shape_box| {
                    (
                        [shape_box.min_x, shape_box.min_y, shape_box.min_z],
                        [shape_box.max_x, shape_box.max_y, shape_box.max_z],
                    )
                })
                .collect(),
        )
    }

    /// Mirrors `BlockState.isFaceSturdy(level, pos, direction,
    /// SupportType.FULL)` for `SolidFaceRenderer`'s vanilla direction order:
    /// west, south, east, north, down, up.
    pub fn debug_block_sturdy_faces(&self, block: &BlockProbe) -> [bool; 6] {
        let Some(block_name) = block.block_name.as_deref() else {
            return [false; 6];
        };
        if is_soul_sand_or_mud_block_name(block_name) {
            return [true; 6];
        }
        if is_leaves_block_name(block_name)
            || block_name == "minecraft:chorus_flower"
            || is_hanging_sign_block_name(block_name)
            || is_fence_gate_block_name(block_name)
        {
            return [false; 6];
        }
        if block_name == "minecraft:snow" {
            let full = block
                .block_properties
                .get("layers")
                .and_then(|value| value.parse::<u8>().ok())
                == Some(8);
            return if full {
                [true; 6]
            } else {
                [false, false, false, false, true, false]
            };
        }
        if is_shulker_box_block_name(block_name) {
            let open = self
                .shulker_box_lids
                .iter()
                .find(|lid| lid.pos == block.pos)
                .is_some_and(|lid| lid.status != crate::ShulkerBoxAnimationStatus::Closed);
            if !open {
                return [true; 6];
            }
            let Some(support_direction) = block
                .block_properties
                .get("facing")
                .and_then(|facing| BlockDirection::parse(facing))
                .map(BlockDirection::opposite)
            else {
                return [false; 6];
            };
            return DEBUG_SOLID_FACE_DIRECTIONS.map(|direction| direction == support_direction);
        }

        block_collision_shape_empty_context(block, block.pos)
            .map(BlockCollisionShape::full_face_flags)
            .unwrap_or([false; 6])
    }

    pub(crate) fn block_has_full_sturdy_face(
        &self,
        block: &BlockProbe,
        direction: ProtocolDirection,
    ) -> bool {
        let direction = block_direction(direction);
        DEBUG_SOLID_FACE_DIRECTIONS
            .into_iter()
            .position(|candidate| candidate == direction)
            .is_some_and(|index| self.debug_block_sturdy_faces(block)[index])
    }

    /// Mirrors cached `BlockState.isFaceSturdy(direction, SupportType.FULL)`;
    /// dynamic states fall back to their live empty-context support shape.
    pub(crate) fn block_full_face_sturdy(
        &self,
        block: &BlockProbe,
        direction: ProtocolDirection,
    ) -> Option<bool> {
        self.block_full_face_sturdy_faces(block)
            .map(|faces| faces[protocol_direction_index(direction)])
    }

    /// All cached FULL support results in Vanilla `Direction.values()` order.
    pub(crate) fn block_full_face_sturdy_faces(&self, block: &BlockProbe) -> Option<[bool; 6]> {
        let facts = vanilla_block_state_support_facts(block.block_state_id)?;
        if facts.dynamic_shape() {
            let [west, south, east, north, down, up] = self.debug_block_sturdy_faces(block);
            Some([down, up, north, south, west, east])
        } else {
            Some(VANILLA_PROTOCOL_DIRECTIONS.map(|direction| {
                facts.face_sturdy(
                    vanilla_support_direction(direction),
                    VanillaBlockSupportType::Full,
                )
            }))
        }
    }

    /// Mirrors `SupportType.CENTER` for the vertical support queries used by
    /// candles, lanterns, and ceiling hanging signs. Dynamic shulker support
    /// is its one full attachment face; the remaining dynamic blocks use
    /// their live empty-context block-support shape.
    pub(crate) fn block_has_vertical_center_sturdy_face(
        &self,
        block: &BlockProbe,
        direction: ProtocolDirection,
    ) -> bool {
        if !matches!(direction, ProtocolDirection::Down | ProtocolDirection::Up) {
            return false;
        }
        if block
            .block_name
            .as_deref()
            .is_some_and(is_shulker_box_block_name)
        {
            return self.block_has_full_sturdy_face(block, direction);
        }
        block_collision_shape_empty_context(block, block.pos)
            .is_some_and(|shape| shape.has_center_face(block_direction(direction)))
    }

    /// Mirrors the vertical `SupportType.RIGID` query used by rail survival.
    pub(crate) fn block_has_vertical_rigid_sturdy_face(
        &self,
        block: &BlockProbe,
        direction: ProtocolDirection,
    ) -> bool {
        if !matches!(direction, ProtocolDirection::Down | ProtocolDirection::Up) {
            return false;
        }
        if block
            .block_name
            .as_deref()
            .is_some_and(is_shulker_box_block_name)
        {
            return self.block_has_full_sturdy_face(block, direction);
        }
        block_collision_shape_empty_context(block, block.pos)
            .is_some_and(|shape| shape.has_rigid_face(block_direction(direction)))
    }

    pub(crate) fn block_has_nonempty_collision_face(
        &self,
        block: &BlockProbe,
        direction: ProtocolDirection,
    ) -> bool {
        block_collision_shape_empty_context(block, block.pos)
            .is_some_and(|shape| shape.has_nonempty_face(block_direction(direction)))
    }

    /// Whether the empty-context collision face covers one exact local
    /// rectangle. Wall placement uses this for the vanilla post and side
    /// probes against the block above.
    pub(crate) fn block_empty_context_collision_face_covers_rectangle(
        &self,
        block: &BlockProbe,
        direction: ProtocolDirection,
        target: [f64; 4],
    ) -> bool {
        block_collision_shape_empty_context(block, block.pos)
            .is_some_and(|shape| shape.covers_face_rectangle(block_direction(direction), target))
    }

    /// Mirrors `MultifaceBlock.canAttachTo`: either the block-support shape or
    /// the empty-context collision shape may provide the requested full face.
    pub(crate) fn block_has_full_support_or_collision_face(
        &self,
        block: &BlockProbe,
        face_index: usize,
    ) -> bool {
        self.debug_block_sturdy_faces(block)
            .get(face_index)
            .copied()
            .unwrap_or(false)
            || block_collision_shape_empty_context(block, block.pos).is_some_and(|shape| {
                shape
                    .full_face_flags()
                    .get(face_index)
                    .copied()
                    .unwrap_or(false)
            })
    }

    /// Mirrors `level.getEntities(cameraEntity,
    /// cameraEntity.getBoundingBox().inflate(radius))` for
    /// `SupportBlockRenderer`'s cached nearby-entity membership.
    pub fn debug_supporting_block_entity_ids(&self, radius: f64) -> Vec<i32> {
        if !radius.is_finite() || radius < 0.0 {
            return Vec::new();
        }
        let Some(camera) = self.debug_camera_collision_query() else {
            return Vec::new();
        };
        let query = camera.bounds.inflated(radius);
        self.entity_transforms()
            .into_iter()
            .filter(|transform| camera.source_entity_id != Some(transform.id))
            .filter_map(|transform| {
                let bounds = self.probe_entity_pick_bounds(transform.id)?;
                world_collision_box_intersects_bounds(
                    WorldCollisionBox {
                        min: [
                            transform.position.x + f64::from(bounds.min[0]),
                            transform.position.y + f64::from(bounds.min[1]),
                            transform.position.z + f64::from(bounds.min[2]),
                        ],
                        max: [
                            transform.position.x + f64::from(bounds.max[0]),
                            transform.position.y + f64::from(bounds.max[1]),
                            transform.position.z + f64::from(bounds.max[2]),
                        ],
                    },
                    query,
                )
                .then_some(transform.id)
            })
            .collect()
    }

    pub fn entity_main_supporting_block_pos(&self, entity_id: i32) -> Option<BlockPos> {
        self.entities.supporting_block(entity_id)?.main
    }

    pub fn debug_entity_supporting_blocks(
        &self,
        entity_id: i32,
    ) -> Option<DebugEntitySupportingBlocks> {
        if self.local_player_id() == Some(entity_id) {
            return None;
        }
        let supporting_pos = self.entity_main_supporting_block_pos(entity_id)?;
        let entity_y = self.entities.transform(entity_id)?.position.y;
        let supporting = self.debug_supporting_block_highlight(supporting_pos);
        let legacy_pos = self.legacy_on_pos(
            Some(supporting_pos),
            crate::EntityVec3 {
                x: f64::from(supporting_pos.x),
                y: entity_y,
                z: f64::from(supporting_pos.z),
            },
        );
        let legacy_effect = (legacy_pos != supporting_pos)
            .then(|| self.debug_supporting_block_highlight(legacy_pos));
        Some(DebugEntitySupportingBlocks {
            entity_id,
            supporting,
            legacy_effect,
        })
    }

    /// Mirrors the `Entity.setOnGround` -> `checkSupportingBlock(onGround,
    /// null)` call made by vanilla's three remote-entity position packet
    /// handlers. The source-specific collision-context refinements are kept at
    /// this single world boundary for later expansion.
    pub(crate) fn update_entity_supporting_block_from_packet(
        &mut self,
        entity_id: i32,
        on_ground: bool,
    ) {
        if self.local_player_id() == Some(entity_id) {
            return;
        }
        let Some(previous) = self.entities.supporting_block(entity_id) else {
            return;
        };
        if !on_ground {
            let _ = self
                .entities
                .with_supporting_block_mut(entity_id, |support| {
                    support.clear_physical();
                });
            return;
        }
        let Some(transform) = self.entities.transform(entity_id) else {
            return;
        };
        let Some(bounds) = self.entities.pick_bounds(entity_id) else {
            return;
        };
        let world_bounds = LocalPlayerBounds::from_min_max(
            [
                transform.position.x + f64::from(bounds.min[0]),
                transform.position.y + f64::from(bounds.min[1]),
                transform.position.z + f64::from(bounds.min[2]),
            ],
            [
                transform.position.x + f64::from(bounds.max[0]),
                transform.position.y + f64::from(bounds.max[1]),
                transform.position.z + f64::from(bounds.max[2]),
            ],
        );
        let current = find_supporting_block(
            self,
            [
                transform.position.x,
                transform.position.y,
                transform.position.z,
            ],
            world_bounds,
            LocalPlayerCollisionContext::for_remote_entity(
                world_bounds.min_y,
                self.entities.is_descending(entity_id).unwrap_or(false),
                self.remote_entity_can_walk_on_powder_snow(entity_id),
                self.entity_type_id(entity_id) == Some(VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID),
            )
            .with_can_stand_on_lava(
                self.entity_type_id(entity_id) == Some(VANILLA_ENTITY_TYPE_STRIDER_ID),
            ),
        );
        let main = if current.is_some() || previous.on_ground_no_blocks {
            current
        } else {
            previous.main
        };
        let _ = self
            .entities
            .with_supporting_block_mut(entity_id, |support| {
                support.main = main;
                support.on_ground_no_blocks = current.is_none();
            });
    }

    fn remote_entity_can_walk_on_powder_snow(&self, entity_id: i32) -> bool {
        let Some(entity_type_id) = self.entity_type_id(entity_id) else {
            return false;
        };
        if vanilla_entity_type_is_powder_snow_walkable(entity_type_id) {
            return true;
        }
        vanilla_living_entity_type(entity_type_id)
            && self
                .entities
                .equipment_item(entity_id, ProtocolEquipmentSlot::Feet)
                .filter(|item| item.count > 0)
                .and_then(|item| item.item_id)
                .is_some_and(|item_id| {
                    self.items
                        .powder_snow_walkable_foot_item_ids
                        .contains(&item_id)
                })
    }

    /// Projects the local-player half of vanilla `SupportBlockRenderer`.
    ///
    /// `Entity.checkSupportingBlock` probes the one-microunit slice directly
    /// below the entity and selects the intersecting block nearest to the
    /// entity position, breaking exact ties toward the greatest `(y, z, x)`
    /// `BlockPos`. The debug renderer draws this state only while
    /// `mainSupportingBlockPos` is present.
    pub fn debug_local_player_supporting_blocks(&self) -> Option<DebugLocalSupportingBlocks> {
        let pose = self.local_player_pose()?;
        if !pose.on_ground {
            return None;
        }
        let bounds = LocalPlayerBounds::for_pose(pose);
        let supporting_pos = find_supporting_block(
            self,
            [pose.position.x, pose.position.y, pose.position.z],
            bounds,
            LocalPlayerCollisionContext::for_pose(self, pose, false),
        )?;
        let supporting = self.debug_supporting_block_highlight(supporting_pos);
        let legacy_pos = self.legacy_on_pos(
            Some(supporting_pos),
            crate::EntityVec3 {
                x: pose.position.x,
                y: pose.position.y,
                z: pose.position.z,
            },
        );
        let legacy_effect = (legacy_pos != supporting_pos)
            .then(|| self.debug_supporting_block_highlight(legacy_pos));
        Some(DebugLocalSupportingBlocks {
            supporting,
            legacy_effect,
        })
    }

    /// Mirrors `Entity.getOnPos(0.2F)`, including the main-supporting-block
    /// override and the fence/wall/gate Y exception used by entity effects.
    pub(crate) fn legacy_on_pos(
        &self,
        supporting_pos: Option<BlockPos>,
        position: crate::EntityVec3,
    ) -> BlockPos {
        let Some(supporting_pos) = supporting_pos else {
            return BlockPos {
                x: block_floor(position.x),
                y: block_floor(position.y - 0.2),
                z: block_floor(position.z),
            };
        };
        let keeps_support_y = self
            .probe_block(supporting_pos)
            .and_then(|block| block.block_name)
            .is_some_and(|name| {
                is_fence_block_name(&name)
                    || is_wall_block_name(&name)
                    || is_fence_gate_block_name(&name)
            });
        if keeps_support_y {
            supporting_pos
        } else {
            BlockPos {
                y: block_floor(position.y - 0.2),
                ..supporting_pos
            }
        }
    }

    fn debug_supporting_block_highlight(&self, pos: BlockPos) -> DebugSupportingBlockHighlight {
        let collision_boxes = self
            .probe_block(pos)
            .and_then(|block| block_collision_shape_empty_context(&block, pos))
            .map(BlockCollisionShape::to_aabbs)
            .unwrap_or_default()
            .into_iter()
            .map(|shape_box| world_collision_box(pos, shape_box))
            .collect();
        DebugSupportingBlockHighlight {
            pos,
            collision_boxes,
        }
    }

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

    fn debug_camera_block_collision_boxes_for_query(
        &self,
        camera: EntityCollisionQuery,
        radius: f64,
    ) -> Vec<WorldCollisionBox> {
        let query = camera.bounds.inflated(radius);
        let min_x = block_floor(query.min_x - COLLISION_EPSILON) - 1;
        let max_x = block_floor(query.max_x + COLLISION_EPSILON) + 1;
        let min_y = block_floor(query.min_y - COLLISION_EPSILON) - 1;
        let max_y = block_floor(query.max_y + COLLISION_EPSILON) + 1;
        let min_z = block_floor(query.min_z - COLLISION_EPSILON) - 1;
        let max_z = block_floor(query.max_z + COLLISION_EPSILON) + 1;

        let mut boxes = Vec::new();
        // Vanilla Cursor3D advances X fastest, then Y, then Z.
        for z in min_z..=max_z {
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let pos = BlockPos { x, y, z };
                    let Some(block) = self.probe_block(pos) else {
                        continue;
                    };
                    let Some(shape) =
                        block_collision_shape_with_context(self, &block, pos, camera.block_context)
                    else {
                        continue;
                    };
                    let shape_boxes = shape.to_aabbs();
                    if !shape_boxes
                        .iter()
                        .copied()
                        .any(|shape_box| bounds_intersects_block_box(query, pos, shape_box))
                    {
                        continue;
                    }
                    boxes.extend(
                        shape_boxes
                            .into_iter()
                            .map(|shape_box| world_collision_box(pos, shape_box)),
                    );
                }
            }
        }
        boxes
    }

    fn debug_camera_entity_collision_boxes_for_query(
        &self,
        camera: EntityCollisionQuery,
        radius: f64,
    ) -> Vec<WorldCollisionBox> {
        let query = camera.bounds.inflated(radius + COLLISION_EPSILON);
        self.entity_pick_targets()
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
                ) || !world_collision_box_intersects_bounds(target_box, query)
                {
                    return None;
                }
                Some(target_box)
            })
            .collect()
    }

    fn debug_entities_can_collide(
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

    fn debug_entity_can_be_collided_with(
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

    fn debug_entity_is_pushable(
        &self,
        target_id: i32,
        target_type: i32,
        target_position: [f64; 3],
    ) -> bool {
        let Some(kind) = vanilla_entity_metadata_type_pushability_kind(target_type) else {
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

    fn debug_living_entity_on_climbable(&self, entity_id: i32, position: [f64; 3]) -> bool {
        let pos = BlockPos {
            x: block_floor(position[0]),
            y: block_floor(position[1]),
            z: block_floor(position[2]),
        };
        let Some(block) = self.probe_block(pos) else {
            return false;
        };
        let Some(block_name) = block.block_name.as_deref() else {
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
        let Some(below) = self.probe_block(BlockPos {
            y: pos.y.saturating_sub(1),
            ..pos
        }) else {
            return false;
        };
        below.block_name.as_deref() == Some("minecraft:ladder")
            && below.block_properties.get("facing") == block.block_properties.get("facing")
    }

    fn debug_entities_share_root_vehicle(&self, source_id: Option<i32>, target_id: i32) -> bool {
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

    fn debug_entity_root_vehicle_id(&self, entity_id: i32) -> Option<i32> {
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

    fn debug_camera_collision_query(&self) -> Option<EntityCollisionQuery> {
        let camera = self.local_player().camera;
        if !camera.follows_player {
            let camera_id = camera.entity_id?;
            let pose = self.probe_entity_camera_pose(camera_id)?;
            let entity_type_id = self.entity_type_id(camera_id)?;
            let bounds = self.entities.pick_bounds(camera_id)?;
            let world_bounds = LocalPlayerBounds::from_min_max(
                [
                    pose.position.x + f64::from(bounds.min[0]),
                    pose.position.y + f64::from(bounds.min[1]),
                    pose.position.z + f64::from(bounds.min[2]),
                ],
                [
                    pose.position.x + f64::from(bounds.max[0]),
                    pose.position.y + f64::from(bounds.max[1]),
                    pose.position.z + f64::from(bounds.max[2]),
                ],
            );
            return Some(EntityCollisionQuery {
                source_entity_id: Some(camera_id),
                source_entity_type_id: entity_type_id,
                source_position: [pose.position.x, pose.position.y, pose.position.z],
                bounds: world_bounds,
                block_context: LocalPlayerCollisionContext::new(world_bounds.min_y, false),
            });
        }

        let pose = self.local_player_pose()?;
        Some(EntityCollisionQuery {
            source_entity_id: self.local_player_id(),
            source_entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            source_position: [pose.position.x, pose.position.y, pose.position.z],
            bounds: LocalPlayerBounds::for_pose(pose),
            block_context: LocalPlayerCollisionContext::for_pose(self, pose, false),
        })
    }

    pub(crate) fn local_player_pose_collides_with_block(
        &self,
        pos: BlockPos,
        pose: LocalPlayerPoseState,
    ) -> bool {
        let Some(block) = self.probe_block(pos) else {
            return false;
        };
        let bounds = LocalPlayerBounds::for_pose(pose);
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
        let Some(block) = self.probe_block(pos) else {
            return ParticleBlockFluidSurfaceSample::default();
        };
        let local_x = position[0] - f64::from(pos.x);
        let local_z = position[2] - f64::from(pos.z);
        let block_height = block_collision_shape(&block, pos)
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
            block_is_air: block.block_name.as_deref().map_or(true, block_name_is_air),
        }
    }

    /// Loaded block-state facts consumed by `ClientLevel.doAnimateTick` fluid
    /// animation and dripping-particle placement.
    pub fn environment_animate_block_sample(
        &self,
        pos: BlockPos,
    ) -> Option<EnvironmentAnimateBlockSample> {
        let block = self.probe_block(pos)?;
        let shape = block_collision_shape_empty_context(&block, pos);
        let block_name = block.block_name.as_deref()?;
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
        let block = self.probe_block(pos)?;
        let block_name = block.block_name.as_deref()?;
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

        let bounds = LocalPlayerBounds::from_min_max(
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
        self.clip_collision_bounds_movement(bounds, movement)
    }

    /// Vanilla `Entity.collideBoundingBox` for a locally authoritative entity:
    /// resolve generated entity colliders and the nearby world-border shape in
    /// the same axis pass as block collision. The caller supplies one narrow
    /// collision-target batch for the whole client-tick catch-up instead of
    /// rebuilding full `EntityState` payloads for every movement substep.
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
        let bounds = LocalPlayerBounds::from_min_max(
            std::array::from_fn(|axis| position[axis] + relative_min[axis]),
            std::array::from_fn(|axis| position[axis] + relative_max[axis]),
        );
        let block_context = LocalPlayerCollisionContext::for_remote_entity(
            bounds.min_y,
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
    pub(super) fn clip_local_player_collision_movement(
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
        let bounds = LocalPlayerBounds::for_pose(pose);
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
    pub(super) fn local_player_collision_bounds_are_free(
        &self,
        pose: LocalPlayerPoseState,
        bounds: LocalPlayerBounds,
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
    pub(super) fn local_player_suffocates_at(
        &self,
        pose: LocalPlayerPoseState,
        pos: BlockPos,
        descending: bool,
    ) -> bool {
        let bounds = LocalPlayerBounds::from_min_max(
            [
                f64::from(pos.x) + COLLISION_EPSILON,
                pose.position.y + COLLISION_EPSILON,
                f64::from(pos.z) + COLLISION_EPSILON,
            ],
            [
                f64::from(pos.x) + 1.0 - COLLISION_EPSILON,
                pose.position.y + pose.body_height() - COLLISION_EPSILON,
                f64::from(pos.z) + 1.0 - COLLISION_EPSILON,
            ],
        );
        local_player_collides_with_suffocating_block(
            self,
            bounds,
            LocalPlayerCollisionContext::for_pose(self, pose, descending),
        )
    }

    /// Highest world-space Y in the live local-player collision shape at one
    /// block cell. `LocalPlayer.updateAutoJump` uses this both for its ceiling
    /// probes and for stacked obstacle-height refinement.
    pub(super) fn local_player_block_collision_shape_max_y(
        &self,
        pose: LocalPlayerPoseState,
        pos: BlockPos,
        descending: bool,
    ) -> Option<f64> {
        let block = self.probe_block(pos)?;
        block_collision_shape_with_context(
            self,
            &block,
            pos,
            LocalPlayerCollisionContext::for_pose(self, pose, descending),
        )?
        .to_aabbs()
        .into_iter()
        .map(|shape_box| f64::from(pos.y) + shape_box.max_y)
        .reduce(f64::max)
    }

    /// Vanilla `CollisionGetter.getCollisions(player, bounds)` as consumed by
    /// auto-jump: collidable entities first, then block `VoxelShape.toAabbs`
    /// order. The world border is a separate collision source and is
    /// deliberately absent from this query.
    pub(super) fn local_player_auto_jump_collision_boxes(
        &self,
        pose: LocalPlayerPoseState,
        bounds: LocalPlayerBounds,
        descending: bool,
        entity_collision_targets: &[crate::EntityPickTargetState],
    ) -> Vec<WorldCollisionBox> {
        let block_context = LocalPlayerCollisionContext::for_pose(self, pose, descending);
        let query = EntityCollisionQuery {
            source_entity_id: self.local_player_id,
            source_entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            source_position: [pose.position.x, pose.position.y, pose.position.z],
            bounds: LocalPlayerBounds::for_pose(pose),
            block_context,
        };
        let mut boxes = self.entity_collision_boxes_for_targets(
            query,
            bounds.inflated(COLLISION_EPSILON),
            entity_collision_targets,
        );

        let min_x = block_floor(bounds.min_x - COLLISION_EPSILON) - 1;
        let max_x = block_floor(bounds.max_x + COLLISION_EPSILON) + 1;
        let min_y = block_floor(bounds.min_y - COLLISION_EPSILON) - 1;
        let max_y = block_floor(bounds.max_y + COLLISION_EPSILON) + 1;
        let min_z = block_floor(bounds.min_z - COLLISION_EPSILON) - 1;
        let max_z = block_floor(bounds.max_z + COLLISION_EPSILON) + 1;
        // `Cursor3D` advances X fastest, then Y, then Z.
        for z in min_z..=max_z {
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let pos = BlockPos { x, y, z };
                    let Some(block) = self.probe_block(pos) else {
                        continue;
                    };
                    let Some(shape) =
                        block_collision_shape_with_context(self, &block, pos, block_context)
                    else {
                        continue;
                    };
                    boxes.extend(shape.to_aabbs().into_iter().filter_map(|shape_box| {
                        let collision_box = world_collision_box(pos, shape_box);
                        world_collision_box_intersects_bounds(collision_box, bounds)
                            .then_some(collision_box)
                    }));
                }
            }
        }
        boxes
    }

    fn clip_locally_authoritative_collision_movement(
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
        let clipped = self.clip_collision_bounds_movement_with_context_and_boxes(
            bounds,
            movement,
            block_context,
            &base_colliders,
        );
        let x_collision = movement[0] != clipped[0];
        let y_collision = movement[1] != clipped[1];
        let z_collision = movement[2] != clipped[2];
        let on_ground_after_collision = y_collision && movement[1] < 0.0;
        if max_up_step <= 0.0
            || !(on_ground_after_collision || on_ground)
            || !(x_collision || z_collision)
        {
            return clipped;
        }

        let grounded_bounds = if on_ground_after_collision {
            bounds.moved(0.0, clipped[1], 0.0)
        } else {
            bounds
        };
        let mut step_bounds = grounded_bounds
            .swept_axis(CollisionAxis::X, movement[0])
            .swept_axis(CollisionAxis::Y, f64::from(max_up_step))
            .swept_axis(CollisionAxis::Z, movement[2]);
        if !on_ground_after_collision {
            step_bounds = step_bounds.swept_axis(CollisionAxis::Y, -f64::from(1.0e-5_f32));
        }
        let mut step_colliders = entity_colliders;
        step_colliders.extend(self.world_border_collision_boxes(query, step_bounds));
        let block_colliders = self.block_collision_boxes_for_bounds(step_bounds, block_context);
        let mut candidate_heights = step_candidate_heights(
            grounded_bounds.min_y,
            max_up_step,
            clipped[1] as f32,
            step_colliders.iter().chain(&block_colliders),
        );
        candidate_heights.sort_by(f32::total_cmp);
        candidate_heights.dedup();
        let clipped_horizontal_squared = clipped[0] * clipped[0] + clipped[2] * clipped[2];
        for candidate_height in candidate_heights {
            let stepped = self.clip_collision_bounds_movement_with_context_and_boxes(
                grounded_bounds,
                [movement[0], f64::from(candidate_height), movement[2]],
                block_context,
                &step_colliders,
            );
            if stepped[0] * stepped[0] + stepped[2] * stepped[2] > clipped_horizontal_squared {
                let distance_to_ground = bounds.min_y - grounded_bounds.min_y;
                return [stepped[0], stepped[1] - distance_to_ground, stepped[2]];
            }
        }
        clipped
    }

    fn block_collision_boxes_for_bounds(
        &self,
        bounds: LocalPlayerBounds,
        context: LocalPlayerCollisionContext,
    ) -> Vec<WorldCollisionBox> {
        let min_x = block_floor(bounds.min_x + COLLISION_EPSILON) - 1;
        let max_x = block_floor(bounds.max_x - COLLISION_EPSILON) + 1;
        let min_y = block_floor(bounds.min_y + COLLISION_EPSILON) - 1;
        let max_y = block_floor(bounds.max_y - COLLISION_EPSILON) + 1;
        let min_z = block_floor(bounds.min_z + COLLISION_EPSILON) - 1;
        let max_z = block_floor(bounds.max_z - COLLISION_EPSILON) + 1;
        let mut boxes = Vec::new();
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                for x in min_x..=max_x {
                    let pos = BlockPos { x, y, z };
                    let Some(block) = self.probe_block(pos) else {
                        continue;
                    };
                    let Some(shape) =
                        block_collision_shape_with_context(self, &block, pos, context)
                    else {
                        continue;
                    };
                    boxes.extend(shape.boxes().filter_map(|shape_box| {
                        let collision_box = world_collision_box(pos, shape_box);
                        world_collision_box_intersects_bounds(collision_box, bounds)
                            .then_some(collision_box)
                    }));
                }
            }
        }
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
        let boat_slice = WorldCollisionBox {
            min: [
                position[0] + relative_min[0],
                position[1] + relative_min[1] - 0.001,
                position[2] + relative_min[2],
            ],
            max: [
                position[0] + relative_max[0],
                position[1] + relative_min[1],
                position[2] + relative_max[2],
            ],
        };
        let x0 = boat_slice.min[0].floor() as i32 - 1;
        let x1 = boat_slice.max[0].ceil() as i32 + 1;
        let y0 = boat_slice.min[1].floor() as i32 - 1;
        let y1 = boat_slice.max[1].ceil() as i32 + 1;
        let z0 = boat_slice.min[2].floor() as i32 - 1;
        let z1 = boat_slice.max[2].ceil() as i32 + 1;
        let mut friction = 0.0_f32;
        let mut count = 0_u32;
        for x in x0..x1 {
            for z in z0..z1 {
                let edges = u8::from(x == x0 || x == x1 - 1) + u8::from(z == z0 || z == z1 - 1);
                if edges == 2 {
                    continue;
                }
                for y in y0..y1 {
                    if edges > 0 && (y == y0 || y == y1 - 1) {
                        continue;
                    }
                    let pos = BlockPos { x, y, z };
                    let Some(block) = self.probe_block(pos) else {
                        continue;
                    };
                    if block.block_name.as_deref() == Some("minecraft:lily_pad") {
                        continue;
                    }
                    let Some(shape) = block_collision_shape(&block, pos) else {
                        continue;
                    };
                    if shape.boxes().any(|shape_box| {
                        world_collision_boxes_intersect(
                            world_collision_box(pos, shape_box),
                            boat_slice,
                        )
                    }) {
                        friction +=
                            crate::terrain::vanilla_block_friction(block.block_name.as_deref());
                        count += 1;
                    }
                }
            }
        }
        friction / count as f32
    }

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

    fn entity_collision_bounds_are_free_for_source(
        &self,
        source_entity_id: Option<i32>,
        entity_type_id: i32,
        position: [f64; 3],
        relative_min: [f64; 3],
        relative_max: [f64; 3],
        on_ground: bool,
        entity_collision_targets: &[crate::EntityPickTargetState],
    ) -> bool {
        let bounds = LocalPlayerBounds::from_min_max(
            std::array::from_fn(|axis| position[axis] + relative_min[axis]),
            std::array::from_fn(|axis| position[axis] + relative_max[axis]),
        );
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

    fn entity_collision_boxes_for_targets(
        &self,
        query: EntityCollisionQuery,
        test_bounds: LocalPlayerBounds,
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
                ) && world_collision_box_intersects_bounds(target_box, test_bounds))
                .then_some(target_box)
            })
            .collect()
    }

    fn world_border_collision_boxes(
        &self,
        query: EntityCollisionQuery,
        test_bounds: LocalPlayerBounds,
    ) -> Vec<WorldCollisionBox> {
        let horizontal_size = (test_bounds.max_x - test_bounds.min_x)
            .abs()
            .max((test_bounds.max_z - test_bounds.min_z).abs())
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
        .filter(|collision_box| world_collision_box_intersects_bounds(*collision_box, test_bounds))
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
                block_hit_location[0]
                    .clamp(border.min_x_at(0.0), border.max_x_at(0.0) - max_edge_offset),
                block_hit_location[1],
                block_hit_location[2]
                    .clamp(border.min_z_at(0.0), border.max_z_at(0.0) - max_edge_offset),
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
        let block = self.probe_block(pos)?;
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
        let bounds = LocalPlayerBounds::from_min_max(
            [
                position[0] + relative_min[0] + clipped_movement[0],
                position[1] + relative_min[1] + clipped_movement[1],
                position[2] + relative_min[2] + clipped_movement[2],
            ],
            [
                position[0] + relative_max[0] + clipped_movement[0],
                position[1] + relative_max[1] + clipped_movement[1],
                position[2] + relative_max[2] + clipped_movement[2],
            ],
        );
        let entity_position = [
            position[0] + clipped_movement[0],
            position[1] + clipped_movement[1],
            position[2] + clipped_movement[2],
        ];
        let context = LocalPlayerCollisionContext::for_remote_entity(
            bounds.min_y,
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

    fn clip_collision_bounds_movement(
        &self,
        bounds: LocalPlayerBounds,
        movement: [f64; 3],
    ) -> [f64; 3] {
        self.clip_collision_bounds_movement_with_boxes(bounds, movement, &[])
    }

    fn clip_collision_bounds_movement_with_boxes(
        &self,
        bounds: LocalPlayerBounds,
        movement: [f64; 3],
        collision_boxes: &[WorldCollisionBox],
    ) -> [f64; 3] {
        let mut clipped = [0.0; 3];
        for axis in particle_collision_axis_order(movement) {
            let requested = particle_movement_component(movement, axis);
            if requested == 0.0 {
                continue;
            }
            let moved_bounds = bounds.moved(clipped[0], clipped[1], clipped[2]);
            let mut axis_clipped = self.clip_particle_axis_collision(moved_bounds, axis, requested);
            for collision_box in collision_boxes {
                axis_clipped = clipped_axis_against_world_box(
                    moved_bounds,
                    axis,
                    requested,
                    axis_clipped,
                    *collision_box,
                );
            }
            if axis_clipped.abs() < COLLISION_EPSILON {
                axis_clipped = 0.0;
            }
            set_particle_movement_component(&mut clipped, axis, axis_clipped);
        }
        clipped
    }

    fn clip_particle_axis_collision(
        &self,
        bounds: LocalPlayerBounds,
        axis: CollisionAxis,
        requested: f64,
    ) -> f64 {
        let swept = bounds.swept_axis(axis, requested);
        let min_x = block_floor(swept.min_x + COLLISION_EPSILON) - 1;
        let max_x = block_floor(swept.max_x - COLLISION_EPSILON) + 1;
        let min_y = block_floor(swept.min_y + COLLISION_EPSILON) - 1;
        let max_y = block_floor(swept.max_y - COLLISION_EPSILON) + 1;
        let min_z = block_floor(swept.min_z + COLLISION_EPSILON) - 1;
        let max_z = block_floor(swept.max_z - COLLISION_EPSILON) + 1;

        let mut clipped = requested;
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                for x in min_x..=max_x {
                    let pos = BlockPos { x, y, z };
                    let Some(block) = self.probe_block(pos) else {
                        continue;
                    };
                    let Some(shape) = block_collision_shape(&block, pos) else {
                        continue;
                    };
                    for shape_box in shape.boxes() {
                        clipped = clipped_particle_axis_against_box(
                            bounds, axis, requested, clipped, pos, shape_box,
                        );
                    }
                }
            }
        }
        if clipped.abs() < COLLISION_EPSILON {
            0.0
        } else {
            clipped
        }
    }

    fn clip_collision_bounds_movement_with_context_and_boxes(
        &self,
        bounds: LocalPlayerBounds,
        movement: [f64; 3],
        context: LocalPlayerCollisionContext,
        collision_boxes: &[WorldCollisionBox],
    ) -> [f64; 3] {
        let mut clipped = [0.0; 3];
        for axis in particle_collision_axis_order(movement) {
            let requested = particle_movement_component(movement, axis);
            if requested == 0.0 {
                continue;
            }
            let moved_bounds = bounds.moved(clipped[0], clipped[1], clipped[2]);
            let mut axis_clipped =
                self.clip_entity_axis_collision(moved_bounds, axis, requested, context);
            for collision_box in collision_boxes {
                axis_clipped = clipped_axis_against_world_box(
                    moved_bounds,
                    axis,
                    requested,
                    axis_clipped,
                    *collision_box,
                );
            }
            if axis_clipped.abs() < COLLISION_EPSILON {
                axis_clipped = 0.0;
            }
            set_particle_movement_component(&mut clipped, axis, axis_clipped);
        }
        clipped
    }

    fn clip_entity_axis_collision(
        &self,
        bounds: LocalPlayerBounds,
        axis: CollisionAxis,
        requested: f64,
        context: LocalPlayerCollisionContext,
    ) -> f64 {
        let swept = bounds.swept_axis(axis, requested);
        let min_x = block_floor(swept.min_x + COLLISION_EPSILON) - 1;
        let max_x = block_floor(swept.max_x - COLLISION_EPSILON) + 1;
        let min_y = block_floor(swept.min_y + COLLISION_EPSILON) - 1;
        let max_y = block_floor(swept.max_y - COLLISION_EPSILON) + 1;
        let min_z = block_floor(swept.min_z + COLLISION_EPSILON) - 1;
        let max_z = block_floor(swept.max_z - COLLISION_EPSILON) + 1;

        let mut clipped = requested;
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                for x in min_x..=max_x {
                    let pos = BlockPos { x, y, z };
                    let Some(block) = self.probe_block(pos) else {
                        continue;
                    };
                    let Some(shape) =
                        block_collision_shape_with_context(self, &block, pos, context)
                    else {
                        continue;
                    };
                    for shape_box in shape.boxes() {
                        clipped = clipped_particle_axis_against_box(
                            bounds, axis, requested, clipped, pos, shape_box,
                        );
                    }
                }
            }
        }
        if clipped.abs() < COLLISION_EPSILON {
            0.0
        } else {
            clipped
        }
    }
}

fn step_candidate_heights<'a>(
    grounded_min_y: f64,
    max_up_step: f32,
    step_height_to_skip: f32,
    colliders: impl Iterator<Item = &'a WorldCollisionBox>,
) -> Vec<f32> {
    let mut candidates = Vec::new();
    for collider in colliders {
        for coordinate in [collider.min[1], collider.max[1]] {
            let relative = (coordinate - grounded_min_y) as f32;
            if relative >= 0.0 && relative != step_height_to_skip && relative <= max_up_step {
                candidates.push(relative);
            }
        }
    }
    candidates
}

#[derive(Debug, Clone, Copy)]
enum BlockClipShape {
    Collision,
    Visual,
}

fn block_hit_scale(
    world: &WorldStore,
    from: [f64; 3],
    to: [f64; 3],
    context: LocalPlayerCollisionContext,
    clip_shape: BlockClipShape,
) -> Option<BlockShapeHit> {
    // `BlockGetter.traverseBlocks` extrapolates both endpoints by -1e-7 away
    // from the opposite endpoint before selecting the first cell.
    let nudged_to: [f64; 3] =
        std::array::from_fn(|axis| to[axis] + (from[axis] - to[axis]) * -1.0e-7);
    let nudged_from: [f64; 3] =
        std::array::from_fn(|axis| from[axis] + (to[axis] - from[axis]) * -1.0e-7);
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
        current[axis] = current[axis].saturating_add(step[axis]);
        t[axis] += t_delta[axis];
        if let Some(scale) = block_shape_hit_scale(world, current, from, to, context, clip_shape) {
            return Some(scale);
        }
    }
    None
}

fn block_shape_hit_scale(
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
    let block = world.probe_block(pos)?;
    let shape = match clip_shape {
        BlockClipShape::Collision => {
            block_collision_shape_with_context(world, &block, pos, context)
        }
        BlockClipShape::Visual => block_visual_shape_with_context(world, &block, pos, context),
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
            block_state_id: block.block_state_id,
        })
}

fn world_collision_box_contains(bounds: WorldCollisionBox, point: [f64; 3]) -> bool {
    (0..3).all(|axis| bounds.min[axis] <= point[axis] && point[axis] < bounds.max[axis])
}

fn world_collision_boxes_intersect(left: WorldCollisionBox, right: WorldCollisionBox) -> bool {
    (0..3).all(|axis| left.min[axis] < right.max[axis] && left.max[axis] > right.min[axis])
}

fn segment_box_entry_scale(from: [f64; 3], to: [f64; 3], bounds: WorldCollisionBox) -> Option<f64> {
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

fn particle_collision_input_is_finite(position: [f64; 3], movement: [f64; 3]) -> bool {
    position.iter().all(|value| value.is_finite()) && movement.iter().all(|value| value.is_finite())
}

fn particle_collision_bounds(
    position: [f64; 3],
    half_width: f64,
    height: f64,
) -> LocalPlayerBounds {
    let half_width = half_width.max(0.0);
    let height = height.max(0.0);
    LocalPlayerBounds {
        min_x: position[0] - half_width,
        min_y: position[1],
        min_z: position[2] - half_width,
        max_x: position[0] + half_width,
        max_y: position[1] + height,
        max_z: position[2] + half_width,
    }
}

fn particle_collision_axis_order(movement: [f64; 3]) -> [CollisionAxis; 3] {
    // Vanilla `Direction.axisStepOrder(Vec3)` always resolves Y first, then
    // resolves the larger horizontal component before the smaller one.
    if movement[0].abs() < movement[2].abs() {
        [CollisionAxis::Y, CollisionAxis::Z, CollisionAxis::X]
    } else {
        [CollisionAxis::Y, CollisionAxis::X, CollisionAxis::Z]
    }
}

fn particle_movement_component(movement: [f64; 3], axis: CollisionAxis) -> f64 {
    match axis {
        CollisionAxis::X => movement[0],
        CollisionAxis::Y => movement[1],
        CollisionAxis::Z => movement[2],
    }
}

fn set_particle_movement_component(movement: &mut [f64; 3], axis: CollisionAxis, value: f64) {
    match axis {
        CollisionAxis::X => movement[0] = value,
        CollisionAxis::Y => movement[1] = value,
        CollisionAxis::Z => movement[2] = value,
    }
}

fn clipped_particle_axis_against_box(
    bounds: LocalPlayerBounds,
    axis: CollisionAxis,
    requested: f64,
    current_clipped: f64,
    pos: BlockPos,
    shape_box: BlockCollisionBox,
) -> f64 {
    if !bounds_intersects_block_box_orthogonal(bounds, pos, shape_box, axis) {
        return current_clipped;
    }
    if requested > 0.0 {
        let shape_min = block_box_axis_min(pos, shape_box, axis);
        let distance = shape_min - bounds_axis_max(bounds, axis);
        if distance >= -COLLISION_EPSILON {
            return current_clipped.min(distance);
        }
    } else if requested < 0.0 {
        let shape_max = block_box_axis_max(pos, shape_box, axis);
        let distance = shape_max - bounds_axis_min(bounds, axis);
        if distance <= COLLISION_EPSILON {
            return current_clipped.max(distance);
        }
    }
    current_clipped
}

fn clipped_axis_against_world_box(
    bounds: LocalPlayerBounds,
    axis: CollisionAxis,
    requested: f64,
    current_clipped: f64,
    collision_box: WorldCollisionBox,
) -> f64 {
    let orthogonal_axes = match axis {
        CollisionAxis::X => [CollisionAxis::Y, CollisionAxis::Z],
        CollisionAxis::Y => [CollisionAxis::X, CollisionAxis::Z],
        CollisionAxis::Z => [CollisionAxis::X, CollisionAxis::Y],
    };
    if !orthogonal_axes.into_iter().all(|orthogonal| {
        bounds_axis_max(bounds, orthogonal)
            > world_box_axis_min(collision_box, orthogonal) + COLLISION_EPSILON
            && bounds_axis_min(bounds, orthogonal)
                < world_box_axis_max(collision_box, orthogonal) - COLLISION_EPSILON
    }) {
        return current_clipped;
    }
    if requested > 0.0 {
        let distance = world_box_axis_min(collision_box, axis) - bounds_axis_max(bounds, axis);
        if distance >= -COLLISION_EPSILON {
            return current_clipped.min(distance);
        }
    } else if requested < 0.0 {
        let distance = world_box_axis_max(collision_box, axis) - bounds_axis_min(bounds, axis);
        if distance <= COLLISION_EPSILON {
            return current_clipped.max(distance);
        }
    }
    current_clipped
}

fn block_collides_with_local_player_bounds(
    world: &WorldStore,
    block: &BlockProbe,
    pos: BlockPos,
    bounds: LocalPlayerBounds,
    context: LocalPlayerCollisionContext,
) -> bool {
    if let Some(shape) = block_collision_shape_with_context(world, block, pos, context) {
        return bounds_intersects_block_shape(bounds, pos, shape);
    }
    false
}

fn block_collision_shape_with_context(
    world: &WorldStore,
    block: &BlockProbe,
    pos: BlockPos,
    context: LocalPlayerCollisionContext,
) -> Option<BlockCollisionShape> {
    if block.block_name.as_deref() == Some("minecraft:moving_piston") {
        return moving_piston_collision_shape(world, block, pos, context, false);
    }
    if block
        .block_name
        .as_deref()
        .is_some_and(is_shulker_box_block_name)
    {
        return shulker_box_collision_shape(world, block, pos);
    }
    if context.can_stand_on_lava
        && block.fluid.is_some_and(|fluid| {
            fluid.kind == TerrainFluidKind::Lava && fluid.amount == 8 && !fluid.falling
        })
        && context.is_above(pos, 0.5)
        && !world
            .probe_block(BlockPos {
                y: pos.y.saturating_add(1),
                ..pos
            })
            .and_then(|above| above.fluid)
            .is_some_and(|fluid| fluid.kind == TerrainFluidKind::Lava)
    {
        return Some(BlockCollisionShape::single(BlockCollisionBox::BOTTOM_SLAB));
    }
    if block.block_name.as_deref() == Some("minecraft:powder_snow") {
        return powder_snow_collision_shape(pos, context);
    }
    if block.block_name.as_deref() == Some("minecraft:scaffolding") {
        return scaffolding_collision_shape(&block.block_properties, pos, context);
    }
    block_collision_shape(block, pos)
}

fn moving_piston_collision_shape(
    world: &WorldStore,
    template: &BlockProbe,
    pos: BlockPos,
    context: LocalPlayerCollisionContext,
    collision_related_only: bool,
) -> Option<BlockCollisionShape> {
    let moving = world.moving_piston(pos)?;
    let mut shape = BlockCollisionShapeBuilder::new();
    if !collision_related_only && !moving.extending && moving.is_source {
        let mut base = block_probe_for_state(world, template, moving.moved_block_state_id)?;
        base.block_properties
            .insert("extended".to_string(), "true".to_string());
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

fn moving_piston_source_head_state_id(
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

fn moving_piston_collision_related_state_id(
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

fn block_probe_for_state(
    world: &WorldStore,
    template: &BlockProbe,
    state_id: i32,
) -> Option<BlockProbe> {
    let state = world.registries.block_state(state_id)?;
    let mut probe = template.clone();
    probe.block_state_id = state_id;
    probe.block_name = Some(state.name.clone());
    probe.block_properties = state.properties.clone();
    probe.material = crate::terrain::classify_terrain_material(Some(&state.name));
    probe.fluid = None;
    Some(probe)
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
        let template = self.probe_block(pos)?;
        let block = block_probe_for_state(self, &template, block_state_id)?;
        if matches!(
            block.block_name.as_deref(),
            Some("minecraft:powder_snow" | "minecraft:scaffolding")
        ) {
            return Some(true);
        }
        let Some(shape) = block_collision_shape(&block, pos) else {
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
        let template = self.probe_block(pos)?;
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
        let Some(template) = self.probe_block(moving.pos) else {
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
        let cause_bounce = self
            .registries
            .block_state(moving.moved_block_state_id)
            .is_some_and(|state| state.name == "minecraft:slime_block");

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
            && self
                .registries
                .block_state(moving.moved_block_state_id)
                .is_some_and(|state| state.name == "minecraft:honey_block")
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
struct PistonEntityCandidate {
    entity_id: Option<i32>,
    entity_type_id: i32,
    bounds: WorldCollisionBox,
    on_ground: bool,
}

fn piston_entity_candidates(world: &WorldStore) -> Vec<PistonEntityCandidate> {
    let mut candidates = Vec::new();
    if let Some(pose) = world.local_player_pose() {
        let bounds = LocalPlayerBounds::for_pose(pose);
        candidates.push(PistonEntityCandidate {
            entity_id: None,
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            bounds: WorldCollisionBox {
                min: [bounds.min_x, bounds.min_y, bounds.min_z],
                max: [bounds.max_x, bounds.max_y, bounds.max_z],
            },
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

fn piston_entity_ignores_movement(entity_type_id: i32) -> bool {
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

fn swept_piston_box(
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

fn piston_separation(
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

fn move_piston_entity(
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

fn block_collision_shape_empty_context(
    block: &BlockProbe,
    pos: BlockPos,
) -> Option<BlockCollisionShape> {
    // Dynamic branches in `block_collision_shape` mirror the no-entity empty
    // context: powder snow and moving pistons are empty, scaffolding is stable,
    // shulker boxes lack a live block entity, and offset shapes use `pos`.
    block_collision_shape(block, pos)
}

/// Mirrors `BlockState.entityCanStandOnFace(..., Direction.UP)` for a remote
/// entity whose collision context does not add a block-specific walkability
/// exception. Powder snow therefore fails closed; scaffolding's stable top
/// shape is the same support face exposed to an entity above it.
pub(crate) fn block_collision_shape_has_full_up_face(block: &BlockProbe, pos: BlockPos) -> bool {
    block_collision_shape_empty_context(block, pos)
        .is_some_and(|shape| shape.has_full_face(BlockDirection::Up))
}

/// Mirrors the cached `BlockStateBase.propagatesSkylightDown` value from the
/// generated nearest override owner and the default empty-context shape rule.
pub(crate) fn block_state_propagates_skylight_down(
    block: &BlockProbe,
    pos: BlockPos,
) -> Option<bool> {
    let block_name = block.block_name.as_deref()?;
    let block_type = vanilla_block_type_for_resource_id(block_name)?;
    let fluid_is_empty = block.fluid.is_none();
    Some(match block_type.propagates_skylight_down_override {
        Some(
            VanillaPropagatesSkylightDownOverride::BambooStalkBlock
            | VanillaPropagatesSkylightDownOverride::HangingMossBlock
            | VanillaPropagatesSkylightDownOverride::MossyCarpetBlock
            | VanillaPropagatesSkylightDownOverride::TransparentBlock
            | VanillaPropagatesSkylightDownOverride::VineBlock,
        ) => true,
        Some(
            VanillaPropagatesSkylightDownOverride::BarrierBlock
            | VanillaPropagatesSkylightDownOverride::CrossCollisionBlock
            | VanillaPropagatesSkylightDownOverride::GlowLichenBlock
            | VanillaPropagatesSkylightDownOverride::LightBlock
            | VanillaPropagatesSkylightDownOverride::VegetationBlock
            | VanillaPropagatesSkylightDownOverride::WallBlock,
        ) => fluid_is_empty,
        Some(
            VanillaPropagatesSkylightDownOverride::LiquidBlock
            | VanillaPropagatesSkylightDownOverride::PipeBlock
            | VanillaPropagatesSkylightDownOverride::ShulkerBoxBlock
            | VanillaPropagatesSkylightDownOverride::TintedGlassBlock,
        ) => false,
        None => fluid_is_empty && !block_collision_shape_is_full_block(block, pos),
    })
}

/// Vanilla `BlockState.getVisualShape` catalog used by camera zoom.
///
/// `BlockBehaviour` defaults to the collision shape. These branches are the
/// complete 26.1 override-owner set found under `world/level/block`.
fn block_visual_shape_with_context(
    world: &WorldStore,
    block: &BlockProbe,
    pos: BlockPos,
    context: LocalPlayerCollisionContext,
) -> Option<BlockCollisionShape> {
    let name = block.block_name.as_deref()?;
    if matches!(name, "minecraft:soul_sand" | "minecraft:mud") {
        return Some(BlockCollisionShape::single(BlockCollisionBox::FULL));
    }
    if block_name_has_empty_camera_visual_shape(name) {
        return None;
    }
    if name == "minecraft:snow" {
        return snow_layer_visual_shape(&block.block_properties);
    }
    if block_name_is_fence(name) {
        return fence_visual_shape(&block.block_properties);
    }
    block_collision_shape_with_context(world, block, pos, context)
}

fn block_name_has_empty_camera_visual_shape(name: &str) -> bool {
    matches!(
        name,
        "minecraft:glass" | "minecraft:tinted_glass" | "minecraft:powder_snow"
    ) || name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_stained_glass"))
        || block_name_is_copper_grate(name)
        || block_name_is_bars_or_pane(name)
}

fn block_collision_shape(block: &BlockProbe, pos: BlockPos) -> Option<BlockCollisionShape> {
    if let Some(vanilla_shape) = vanilla_block_state_static_collision_shape(block.block_state_id) {
        let mut builder = BlockCollisionShapeBuilder::new();
        let boxes = vanilla_shape.boxes();
        debug_assert!(boxes.len() <= MAX_COLLISION_BOXES);
        for shape_box in boxes {
            builder.push(BlockCollisionBox {
                min_x: shape_box.min_x,
                min_y: shape_box.min_y,
                min_z: shape_box.min_z,
                max_x: shape_box.max_x,
                max_y: shape_box.max_y,
                max_z: shape_box.max_z,
            });
        }
        return (builder.next_index != 0).then(|| builder.build());
    }
    // Built-in non-dynamic states always return above. The name/property
    // projection below is retained only for probes that carry a recognized
    // Vanilla name but no valid built-in state id.
    if is_slab_block(block) {
        return match block.block_properties.get("type").map(String::as_str) {
            Some("bottom") => Some(BlockCollisionShape::single(BlockCollisionBox::BOTTOM_SLAB)),
            Some("top") => Some(BlockCollisionShape::single(BlockCollisionBox::TOP_SLAB)),
            Some("double") => Some(BlockCollisionShape::single(BlockCollisionBox::FULL)),
            _ => None,
        };
    }
    if is_stair_block(block) {
        return stair_collision_shape(&block.block_properties);
    }
    if let Some(block_name) = block.block_name.as_deref() {
        if block_name == "minecraft:bamboo" {
            return Some(bamboo_collision_shape(pos));
        }
        if matches!(
            block_name,
            "minecraft:moving_piston" | "minecraft:powder_snow"
        ) {
            return None;
        }
        if block_name == "minecraft:pointed_dripstone" {
            return pointed_dripstone_collision_shape(&block.block_properties, pos);
        }
        if block_name == "minecraft:scaffolding" {
            return Some(scaffolding_stable_collision_shape());
        }
        if is_shulker_box_block_name(block_name) {
            // `EmptyBlockGetter` has no block entity, so Vanilla returns a
            // full cube. Live world collision uses the tracked lid above.
            return Some(BlockCollisionShape::single(BlockCollisionBox::FULL));
        }
        if is_leaves_block_name(block_name)
            || matches!(
                block_name,
                "minecraft:mangrove_roots"
                    | "minecraft:muddy_mangrove_roots"
                    | "minecraft:chorus_flower"
            )
            || block_name_is_copper_grate(block_name)
        {
            return Some(BlockCollisionShape::single(BlockCollisionBox::FULL));
        }
        if block_name == "minecraft:snow" {
            return snow_layer_collision_shape(&block.block_properties);
        }
        if is_flat_carpet_block_name(block_name) {
            return Some(BlockCollisionShape::single(BlockCollisionBox::CARPET));
        }
        if block_name == "minecraft:pale_moss_carpet" {
            return pale_moss_carpet_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:chorus_plant" {
            return chorus_plant_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:conduit" {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::centered_column(6.0, 6.0, 5.0, 11.0),
            ));
        }
        if matches!(
            block_name,
            "minecraft:azalea" | "minecraft:flowering_azalea"
        ) {
            return Some(azalea_collision_shape());
        }
        if is_piston_base_block_name(block_name) {
            return piston_base_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:piston_head" {
            return piston_head_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:big_dripleaf" {
            return big_dripleaf_collision_shape(&block.block_properties);
        }
        if matches!(
            block_name,
            "minecraft:bamboo_sapling"
                | "minecraft:nether_portal"
                | "minecraft:big_dripleaf_stem"
                | "minecraft:light"
        ) {
            return None;
        }
        if is_flower_pot_block_name(block_name) {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::centered_column(6.0, 6.0, 0.0, 6.0),
            ));
        }
        if is_candle_cake_block_name(block_name) {
            return Some(candle_cake_collision_shape());
        }
        if is_candle_block_name(block_name) {
            return candle_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:cocoa" {
            return cocoa_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:sea_pickle" {
            return sea_pickle_collision_shape(&block.block_properties);
        }
        if is_amethyst_cluster_block_name(block_name) {
            return amethyst_cluster_collision_shape(block_name, &block.block_properties);
        }
        if is_shelf_block_name(block_name) {
            return shelf_collision_shape(&block.block_properties);
        }
        if is_copper_golem_statue_block_name(block_name) {
            return Some(copper_golem_statue_collision_shape());
        }
        if is_floor_skull_block_name(block_name) {
            return Some(BlockCollisionShape::single(floor_skull_collision_box(
                block_name,
            )));
        }
        if is_wall_skull_block_name(block_name) {
            return wall_skull_collision_shape(block_name, &block.block_properties);
        }
        if block_name == "minecraft:turtle_egg" {
            return turtle_egg_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:sniffer_egg" {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::centered_column(14.0, 12.0, 0.0, 16.0),
            ));
        }
        if block_name == "minecraft:dried_ghast" {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::centered_column(10.0, 10.0, 0.0, 10.0),
            ));
        }
        if block_name == "minecraft:decorated_pot" {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::centered_column(14.0, 14.0, 0.0, 16.0),
            ));
        }
        if block_name == "minecraft:end_portal_frame" {
            return end_portal_frame_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:daylight_detector" {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::centered_column(16.0, 16.0, 0.0, 6.0),
            ));
        }
        if matches!(block_name, "minecraft:repeater" | "minecraft:comparator") {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::centered_column(16.0, 16.0, 0.0, 2.0),
            ));
        }
        if is_sculk_sensor_block_name(block_name) || block_name == "minecraft:sculk_shrieker" {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::centered_column(16.0, 16.0, 0.0, 8.0),
            ));
        }
        if block_name == "minecraft:heavy_core" {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::centered_column(8.0, 8.0, 0.0, 8.0),
            ));
        }
        if block_name == "minecraft:dragon_egg" {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::centered_column(14.0, 14.0, 0.0, 16.0),
            ));
        }
        if is_chain_block_name(block_name) {
            return chain_collision_shape(&block.block_properties);
        }
        if is_ladder_block_name(block_name) {
            return ladder_collision_shape(&block.block_properties);
        }
        if is_lantern_block_name(block_name) {
            return lantern_collision_shape(&block.block_properties);
        }
        if is_rod_block_name(block_name) {
            return rod_collision_shape(&block.block_properties);
        }
        if is_campfire_block_name(block_name) {
            return Some(BlockCollisionShape::single(BlockCollisionBox::CAMPFIRE));
        }
        if block_name == "minecraft:cake" {
            return cake_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:lily_pad" {
            return Some(BlockCollisionShape::single(BlockCollisionBox::LILY_PAD));
        }
        if is_cactus_or_honey_block_name(block_name) {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::CENTERED_14PX_COLUMN_15PX_HIGH,
            ));
        }
        if is_farmland_or_dirt_path_block_name(block_name) {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::FULL_WIDTH_15PX_HIGH,
            ));
        }
        if is_soul_sand_or_mud_block_name(block_name) {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::FULL_WIDTH_14PX_HIGH,
            ));
        }
        if is_cauldron_block_name(block_name) {
            return Some(cauldron_collision_shape());
        }
        if block_name == "minecraft:hopper" {
            return hopper_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:composter" {
            return Some(composter_collision_shape());
        }
        if block_name == "minecraft:ender_chest" {
            return Some(BlockCollisionShape::single(BlockCollisionBox::CHEST_SINGLE));
        }
        if is_standard_chest_block_name(block_name) {
            return chest_collision_shape(&block.block_properties);
        }
        if is_bed_block_name(block_name) {
            return bed_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:enchanting_table" {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::centered_column(16.0, 16.0, 0.0, 12.0),
            ));
        }
        if block_name == "minecraft:stonecutter" {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::centered_column(16.0, 16.0, 0.0, 9.0),
            ));
        }
        if block_name == "minecraft:lectern" {
            return Some(lectern_collision_shape());
        }
        if block_name == "minecraft:grindstone" {
            return grindstone_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:bell" {
            return bell_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:brewing_stand" {
            return Some(brewing_stand_collision_shape());
        }
        if is_anvil_block_name(block_name) {
            return anvil_collision_shape(&block.block_properties);
        }
        if block_name_is_door(block_name) {
            return door_collision_shape(&block.block_properties);
        }
        if block_name_is_trapdoor(block_name) {
            return trapdoor_collision_shape(&block.block_properties);
        }
        if is_fence_gate_block_name(block_name) {
            return fence_gate_collision_shape(&block.block_properties);
        }
        if is_fence_block_name(block_name) {
            return fence_collision_shape(&block.block_properties);
        }
        if is_bars_or_pane_block_name(block_name) {
            return bars_or_pane_collision_shape(&block.block_properties);
        }
        if is_wall_block_name(block_name) {
            return wall_collision_shape(&block.block_properties);
        }
        if block_name_has_empty_collision_shape(block_name) {
            return None;
        }
    }
    match block.material {
        TerrainMaterialClass::Opaque | TerrainMaterialClass::Translucent => {
            Some(BlockCollisionShape::single(BlockCollisionBox::FULL))
        }
        TerrainMaterialClass::Invisible => {
            if matches!(block.block_name.as_deref(), Some("minecraft:barrier")) {
                Some(BlockCollisionShape::single(BlockCollisionBox::FULL))
            } else {
                None
            }
        }
        TerrainMaterialClass::Empty
        | TerrainMaterialClass::Cutout
        | TerrainMaterialClass::Fluid => None,
    }
}

fn is_slab_block(block: &BlockProbe) -> bool {
    block.block_name.as_deref().is_some_and(block_name_is_slab)
}

fn is_stair_block(block: &BlockProbe) -> bool {
    block.block_name.as_deref().is_some_and(block_name_is_stair)
}

fn is_leaves_block_name(block_name: &str) -> bool {
    block_name_is_leaves(block_name)
}

fn is_flat_carpet_block_name(block_name: &str) -> bool {
    block_name_is_wool_carpet(block_name) || block_name == "minecraft:moss_carpet"
}

fn is_piston_base_block_name(block_name: &str) -> bool {
    matches!(block_name, "minecraft:piston" | "minecraft:sticky_piston")
}

fn is_flower_pot_block_name(block_name: &str) -> bool {
    block_name_is_flower_pot(block_name)
}

fn is_candle_block_name(block_name: &str) -> bool {
    block_name_is_candle(block_name)
}

fn is_candle_cake_block_name(block_name: &str) -> bool {
    block_name_is_candle_cake(block_name)
}

fn is_shelf_block_name(block_name: &str) -> bool {
    block_name_is_shelf(block_name)
}

fn is_amethyst_cluster_block_name(block_name: &str) -> bool {
    block_name_is_amethyst_cluster(block_name)
}

fn is_copper_golem_statue_block_name(block_name: &str) -> bool {
    block_name_is_copper_golem_statue(block_name)
}

fn is_sculk_sensor_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:sculk_sensor" | "minecraft:calibrated_sculk_sensor"
    )
}

fn is_floor_skull_block_name(block_name: &str) -> bool {
    block_name_is_floor_skull(block_name)
}

fn is_wall_skull_block_name(block_name: &str) -> bool {
    block_name_is_wall_skull(block_name)
}

fn is_chain_block_name(block_name: &str) -> bool {
    block_name_is_chain(block_name)
}

fn is_ladder_block_name(block_name: &str) -> bool {
    block_name == "minecraft:ladder"
}

fn debug_climbable_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:ladder"
            | "minecraft:vine"
            | "minecraft:scaffolding"
            | "minecraft:weeping_vines"
            | "minecraft:weeping_vines_plant"
            | "minecraft:twisting_vines"
            | "minecraft:twisting_vines_plant"
            | "minecraft:cave_vines"
            | "minecraft:cave_vines_plant"
    )
}

fn debug_can_glide_through_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:vine"
            | "minecraft:weeping_vines"
            | "minecraft:weeping_vines_plant"
            | "minecraft:twisting_vines"
            | "minecraft:twisting_vines_plant"
            | "minecraft:cave_vines"
            | "minecraft:cave_vines_plant"
    )
}

fn is_lantern_block_name(block_name: &str) -> bool {
    block_name_is_lantern(block_name)
}

fn is_rod_block_name(block_name: &str) -> bool {
    block_name_is_rod(block_name)
}

fn is_campfire_block_name(block_name: &str) -> bool {
    block_name_is_campfire(block_name)
}

fn is_cactus_or_honey_block_name(block_name: &str) -> bool {
    matches!(block_name, "minecraft:cactus" | "minecraft:honey_block")
}

fn is_farmland_or_dirt_path_block_name(block_name: &str) -> bool {
    matches!(block_name, "minecraft:farmland" | "minecraft:dirt_path")
}

fn is_soul_sand_or_mud_block_name(block_name: &str) -> bool {
    matches!(block_name, "minecraft:soul_sand" | "minecraft:mud")
}

fn is_hanging_sign_block_name(block_name: &str) -> bool {
    block_name_is_hanging_sign(block_name)
}

fn is_shulker_box_block_name(block_name: &str) -> bool {
    block_name_is_shulker_box(block_name)
}

fn is_fence_gate_block_name(block_name: &str) -> bool {
    block_name_is_fence_gate(block_name)
}

fn is_fence_block_name(block_name: &str) -> bool {
    block_name_is_fence(block_name)
}

fn is_bars_or_pane_block_name(block_name: &str) -> bool {
    block_name_is_bars_or_pane(block_name)
}

fn is_wall_block_name(block_name: &str) -> bool {
    block_name_is_wall(block_name)
}

fn is_cauldron_block_name(block_name: &str) -> bool {
    block_name_is_cauldron(block_name)
}

fn is_standard_chest_block_name(block_name: &str) -> bool {
    block_name_is_chest(block_name)
}

fn is_bed_block_name(block_name: &str) -> bool {
    block_name_is_bed(block_name)
}

fn is_anvil_block_name(block_name: &str) -> bool {
    block_name_is_anvil(block_name)
}

fn stair_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let top = match properties.get("half").map(String::as_str)? {
        "bottom" => false,
        "top" => true,
        _ => return None,
    };
    let (kind, direction) = match properties.get("shape").map(String::as_str)? {
        "straight" => (StairShapeKind::Straight, facing),
        "outer_left" => (StairShapeKind::Outer, facing),
        "outer_right" => (StairShapeKind::Outer, facing.clockwise()),
        "inner_left" => (StairShapeKind::Inner, facing.counter_clockwise()),
        "inner_right" => (StairShapeKind::Inner, facing),
        _ => return None,
    };

    let mut shape = match kind {
        StairShapeKind::Straight => BlockCollisionShape::from_boxes([
            Some(BlockCollisionBox::BOTTOM_SLAB),
            Some(BlockCollisionBox::STAIR_NORTH_HALF),
            None,
            None,
            None,
        ]),
        StairShapeKind::Outer => BlockCollisionShape::from_boxes([
            Some(BlockCollisionBox::BOTTOM_SLAB),
            Some(BlockCollisionBox::STAIR_NORTH_WEST_OCTET),
            None,
            None,
            None,
        ]),
        StairShapeKind::Inner => BlockCollisionShape::from_boxes([
            Some(BlockCollisionBox::BOTTOM_SLAB),
            Some(BlockCollisionBox::STAIR_NORTH_HALF),
            Some(BlockCollisionBox::STAIR_SOUTH_EAST_OCTET),
            None,
            None,
        ]),
    };
    if top {
        shape = shape.invert_y();
    }
    Some(shape.rotate_to_direction(direction))
}

fn snow_layer_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    let layers = properties.get("layers")?.parse::<u8>().ok()?;
    if !(1..=8).contains(&layers) {
        return None;
    }
    let height = f64::from(layers - 1) / 8.0;
    Some(BlockCollisionShape::single(BlockCollisionBox::column(
        0.0, 0.0, 1.0, height, 1.0,
    )))
}

fn snow_layer_visual_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let layers = properties.get("layers")?.parse::<u8>().ok()?;
    if !(1..=8).contains(&layers) {
        return None;
    }
    Some(BlockCollisionShape::single(BlockCollisionBox::column(
        0.0,
        0.0,
        1.0,
        f64::from(layers) / 8.0,
        1.0,
    )))
}

fn piston_base_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    if !bool_property(properties, "extended")? {
        return Some(BlockCollisionShape::single(BlockCollisionBox::FULL));
    }
    let facing = BlockDirection::parse(properties.get("facing")?)?;
    Some(
        BlockCollisionShape::single(BlockCollisionBox::from_pixels(
            [0.0, 0.0, 4.0],
            [16.0, 16.0, 16.0],
        ))
        .rotate_all_from_north(facing),
    )
}

fn piston_head_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    let facing = BlockDirection::parse(properties.get("facing")?)?;
    let rod_max_z = if bool_property(properties, "short")? {
        16.0
    } else {
        20.0
    };
    Some(
        BlockCollisionShape::from_boxes([
            Some(BlockCollisionBox::from_pixels(
                [0.0, 0.0, 0.0],
                [16.0, 16.0, 4.0],
            )),
            Some(BlockCollisionBox::from_pixels(
                [6.0, 6.0, 4.0],
                [10.0, 10.0, rod_max_z],
            )),
        ])
        .rotate_all_from_north(facing),
    )
}

fn big_dripleaf_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    let height_px = match properties.get("tilt").map(String::as_str)? {
        "none" | "unstable" => 15.0,
        "partial" => 13.0,
        "full" => return None,
        _ => return None,
    };
    Some(BlockCollisionShape::single(
        BlockCollisionBox::centered_column(16.0, 16.0, 11.0, height_px),
    ))
}

fn pale_moss_carpet_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    match properties.get("bottom").map(String::as_str)? {
        "true" => Some(BlockCollisionShape::single(BlockCollisionBox::CARPET)),
        "false" => None,
        _ => None,
    }
}

fn bamboo_collision_shape(pos: BlockPos) -> BlockCollisionShape {
    let (offset_x, offset_z) = vanilla_xz_offset(pos, 0.25);
    BlockCollisionShape::single(BlockCollisionBox::centered_column(3.0, 3.0, 0.0, 16.0))
        .offset(offset_x, 0.0, offset_z)
}

fn chorus_plant_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    let mut builder = BlockCollisionShapeBuilder::new();
    builder.push(BlockCollisionBox::from_pixels(
        [3.0, 3.0, 3.0],
        [13.0, 13.0, 13.0],
    ));
    push_chorus_plant_arm_if_connected(properties, &mut builder, "north", BlockDirection::North)?;
    push_chorus_plant_arm_if_connected(properties, &mut builder, "east", BlockDirection::East)?;
    push_chorus_plant_arm_if_connected(properties, &mut builder, "south", BlockDirection::South)?;
    push_chorus_plant_arm_if_connected(properties, &mut builder, "west", BlockDirection::West)?;
    push_chorus_plant_arm_if_connected(properties, &mut builder, "up", BlockDirection::Up)?;
    push_chorus_plant_arm_if_connected(properties, &mut builder, "down", BlockDirection::Down)?;
    Some(builder.build())
}

fn push_chorus_plant_arm_if_connected(
    properties: &BTreeMap<String, String>,
    builder: &mut BlockCollisionShapeBuilder,
    key: &str,
    direction: BlockDirection,
) -> Option<()> {
    if bool_property(properties, key)? {
        builder.push(chorus_plant_arm_collision_box(direction));
    }
    Some(())
}

fn chorus_plant_arm_collision_box(direction: BlockDirection) -> BlockCollisionBox {
    let north_arm = BlockCollisionBox::from_pixels([3.0, 3.0, 0.0], [13.0, 13.0, 8.0]);
    match direction {
        BlockDirection::North => north_arm,
        BlockDirection::East => north_arm.rotate_to_direction(HorizontalDirection::East),
        BlockDirection::South => north_arm.rotate_to_direction(HorizontalDirection::South),
        BlockDirection::West => north_arm.rotate_to_direction(HorizontalDirection::West),
        BlockDirection::Up => north_arm.rotate_x_270(),
        BlockDirection::Down => north_arm.rotate_x_90(),
    }
}

fn azalea_collision_shape() -> BlockCollisionShape {
    BlockCollisionShape::from_boxes([
        Some(BlockCollisionBox::centered_column(16.0, 16.0, 8.0, 16.0)),
        Some(BlockCollisionBox::centered_column(4.0, 4.0, 0.0, 8.0)),
    ])
}

fn scaffolding_collision_shape(
    properties: &BTreeMap<String, String>,
    pos: BlockPos,
    context: LocalPlayerCollisionContext,
) -> Option<BlockCollisionShape> {
    if context.is_above(pos, 1.0) && !context.descending {
        return Some(scaffolding_stable_collision_shape());
    }

    let distance = properties.get("distance")?.parse::<i32>().ok()?;
    if distance != 0 && bool_property(properties, "bottom")? && context.is_above(pos, 0.0) {
        return Some(BlockCollisionShape::single(
            BlockCollisionBox::SCAFFOLDING_UNSTABLE_BOTTOM,
        ));
    }

    None
}

fn shulker_box_collision_shape(
    world: &WorldStore,
    block: &BlockProbe,
    pos: BlockPos,
) -> Option<BlockCollisionShape> {
    let facing = BlockDirection::parse(block.block_properties.get("facing")?)?;
    let extension = 0.5 * f64::from(world.shulker_box_progress_at(pos, 1.0)).clamp(0.0, 1.0);
    let mut shape_box = BlockCollisionBox::FULL;
    match facing {
        BlockDirection::North => shape_box.min_z -= extension,
        BlockDirection::East => shape_box.max_x += extension,
        BlockDirection::South => shape_box.max_z += extension,
        BlockDirection::West => shape_box.min_x -= extension,
        BlockDirection::Up => shape_box.max_y += extension,
        BlockDirection::Down => shape_box.min_y -= extension,
    }
    Some(BlockCollisionShape::single(shape_box))
}

fn scaffolding_stable_collision_shape() -> BlockCollisionShape {
    BlockCollisionShape::from_boxes([
        Some(BlockCollisionBox::SCAFFOLDING_TOP),
        Some(BlockCollisionBox::SCAFFOLDING_NORTH_WEST_POST),
        Some(BlockCollisionBox::SCAFFOLDING_NORTH_EAST_POST),
        Some(BlockCollisionBox::SCAFFOLDING_SOUTH_EAST_POST),
        Some(BlockCollisionBox::SCAFFOLDING_SOUTH_WEST_POST),
    ])
}

fn powder_snow_collision_shape(
    pos: BlockPos,
    context: LocalPlayerCollisionContext,
) -> Option<BlockCollisionShape> {
    if context.fall_distance > LOCAL_PLAYER_POWDER_SNOW_FALLING_COLLISION_FALL_DISTANCE {
        return Some(BlockCollisionShape::single(
            BlockCollisionBox::POWDER_SNOW_FALLING,
        ));
    }
    if context.is_falling_block {
        return Some(BlockCollisionShape::single(BlockCollisionBox::FULL));
    }
    if context.can_walk_on_powder_snow && context.is_above(pos, 1.0) && !context.descending {
        return Some(BlockCollisionShape::single(BlockCollisionBox::FULL));
    }
    None
}

fn pointed_dripstone_collision_shape(
    properties: &BTreeMap<String, String>,
    pos: BlockPos,
) -> Option<BlockCollisionShape> {
    let shape_box = match properties.get("thickness").map(String::as_str)? {
        "tip_merge" => BlockCollisionBox::centered_column(6.0, 6.0, 0.0, 16.0),
        "tip" => match properties.get("vertical_direction").map(String::as_str)? {
            "down" => BlockCollisionBox::centered_column(6.0, 6.0, 5.0, 16.0),
            "up" => BlockCollisionBox::centered_column(6.0, 6.0, 0.0, 11.0),
            _ => return None,
        },
        "frustum" => BlockCollisionBox::centered_column(8.0, 8.0, 0.0, 16.0),
        "middle" => BlockCollisionBox::centered_column(10.0, 10.0, 0.0, 16.0),
        "base" => BlockCollisionBox::centered_column(12.0, 12.0, 0.0, 16.0),
        _ => return None,
    };
    let (offset_x, offset_z) = pointed_dripstone_xz_offset(pos);
    Some(BlockCollisionShape::single(shape_box).offset(offset_x, 0.0, offset_z))
}

fn pointed_dripstone_xz_offset(pos: BlockPos) -> (f64, f64) {
    vanilla_xz_offset(pos, 2.0 * PX)
}

fn vanilla_xz_offset(pos: BlockPos, max_horizontal_offset: f64) -> (f64, f64) {
    let seed = vanilla_block_seed(pos.x, 0, pos.z);
    let x = (((seed & 15) as f64 / 15.0) - 0.5) * 0.5;
    let z = ((((seed >> 8) & 15) as f64 / 15.0) - 0.5) * 0.5;
    (
        x.clamp(-max_horizontal_offset, max_horizontal_offset),
        z.clamp(-max_horizontal_offset, max_horizontal_offset),
    )
}

fn vanilla_block_seed(x: i32, y: i32, z: i32) -> i64 {
    let mut seed = i64::from(x).wrapping_mul(3_129_871)
        ^ i64::from(z).wrapping_mul(116_129_781)
        ^ i64::from(y);
    seed = seed
        .wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11));
    seed >> 16
}

fn chain_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let axis = ShapeAxis::parse(properties.get("axis")?)?;
    Some(BlockCollisionShape::single(
        BlockCollisionBox::centered_axis(axis, 3.0),
    ))
}

fn ladder_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    Some(
        BlockCollisionShape::single(BlockCollisionBox::NORTH_VERTICAL_THIN)
            .rotate_to_direction(facing),
    )
}

fn lantern_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let y_offset = if bool_property(properties, "hanging")? {
        PX
    } else {
        0.0
    };
    Some(BlockCollisionShape::from_boxes([
        Some(BlockCollisionBox::cuboid(
            5.0 * PX,
            y_offset,
            5.0 * PX,
            11.0 * PX,
            y_offset + 7.0 * PX,
            11.0 * PX,
        )),
        Some(BlockCollisionBox::cuboid(
            6.0 * PX,
            y_offset + 7.0 * PX,
            6.0 * PX,
            10.0 * PX,
            y_offset + 9.0 * PX,
            10.0 * PX,
        )),
        None,
        None,
        None,
    ]))
}

fn rod_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let axis = ShapeAxis::parse_direction(properties.get("facing")?)?;
    Some(BlockCollisionShape::single(
        BlockCollisionBox::centered_axis(axis, 4.0),
    ))
}

fn cake_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let bites = properties.get("bites")?.parse::<u8>().ok()?;
    if bites > 6 {
        return None;
    }
    Some(BlockCollisionShape::single(BlockCollisionBox::from_pixels(
        [1.0 + f64::from(bites) * 2.0, 0.0, 1.0],
        [15.0, 8.0, 15.0],
    )))
}

fn candle_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let candles = properties.get("candles")?.parse::<u8>().ok()?;
    let shape_box = match candles {
        1 => BlockCollisionBox::centered_column(2.0, 2.0, 0.0, 6.0),
        2 => BlockCollisionBox::from_pixels([5.0, 0.0, 6.0], [11.0, 6.0, 9.0]),
        3 => BlockCollisionBox::from_pixels([5.0, 0.0, 6.0], [10.0, 6.0, 11.0]),
        4 => BlockCollisionBox::from_pixels([5.0, 0.0, 5.0], [11.0, 6.0, 10.0]),
        _ => return None,
    };
    Some(BlockCollisionShape::single(shape_box))
}

fn candle_cake_collision_shape() -> BlockCollisionShape {
    BlockCollisionShape::from_boxes([
        Some(BlockCollisionBox::centered_column(2.0, 2.0, 8.0, 14.0)),
        Some(BlockCollisionBox::centered_column(14.0, 14.0, 0.0, 8.0)),
    ])
}

fn cocoa_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let age = match properties.get("age").map(String::as_str)? {
        "0" => 0.0,
        "1" => 1.0,
        "2" => 2.0,
        _ => return None,
    };

    Some(BlockCollisionShape::single(
        BlockCollisionBox {
            min_x: (6.0 - age) * PX,
            min_y: (7.0 - 2.0 * age) * PX,
            min_z: PX,
            max_x: (10.0 + age) * PX,
            max_y: 12.0 * PX,
            max_z: (5.0 + 2.0 * age) * PX,
        }
        .rotate_to_direction(facing),
    ))
}

fn sea_pickle_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    let shape_box = match properties.get("pickles")?.parse::<u8>().ok()? {
        1 => BlockCollisionBox::centered_column(4.0, 4.0, 0.0, 6.0),
        2 => BlockCollisionBox::centered_column(10.0, 10.0, 0.0, 6.0),
        3 => BlockCollisionBox::centered_column(12.0, 12.0, 0.0, 6.0),
        4 => BlockCollisionBox::centered_column(12.0, 12.0, 0.0, 7.0),
        _ => return None,
    };
    Some(BlockCollisionShape::single(shape_box))
}

fn amethyst_cluster_collision_shape(
    block_name: &str,
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    let facing = BlockDirection::parse(properties.get("facing")?)?;
    let (height_px, width_px) = match block_name {
        "minecraft:amethyst_cluster" => (7.0, 10.0),
        "minecraft:large_amethyst_bud" => (5.0, 10.0),
        "minecraft:medium_amethyst_bud" => (4.0, 10.0),
        "minecraft:small_amethyst_bud" => (3.0, 8.0),
        _ => return None,
    };
    Some(
        BlockCollisionShape::single(BlockCollisionBox::from_pixels(
            [
                (16.0 - width_px) * 0.5,
                (16.0 - width_px) * 0.5,
                16.0 - height_px,
            ],
            [(16.0 + width_px) * 0.5, (16.0 + width_px) * 0.5, 16.0],
        ))
        .rotate_all_from_north(facing),
    )
}

fn shelf_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    Some(
        BlockCollisionShape::from_boxes([
            Some(BlockCollisionBox::from_pixels(
                [0.0, 12.0, 11.0],
                [16.0, 16.0, 13.0],
            )),
            Some(BlockCollisionBox::from_pixels(
                [0.0, 0.0, 13.0],
                [16.0, 16.0, 16.0],
            )),
            Some(BlockCollisionBox::from_pixels(
                [0.0, 0.0, 11.0],
                [16.0, 4.0, 13.0],
            )),
        ])
        .rotate_to_direction(facing),
    )
}

fn copper_golem_statue_collision_shape() -> BlockCollisionShape {
    BlockCollisionShape::single(BlockCollisionBox::centered_column(10.0, 10.0, 0.0, 14.0))
}

fn floor_skull_collision_box(block_name: &str) -> BlockCollisionBox {
    if block_name == "minecraft:piglin_head" {
        BlockCollisionBox::centered_column(10.0, 10.0, 0.0, 8.0)
    } else {
        BlockCollisionBox::centered_column(8.0, 8.0, 0.0, 8.0)
    }
}

fn wall_skull_collision_shape(
    block_name: &str,
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let shape_box = if block_name == "minecraft:piglin_wall_head" {
        BlockCollisionBox::from_pixels([3.0, 4.0, 8.0], [13.0, 12.0, 16.0])
    } else {
        BlockCollisionBox::from_pixels([4.0, 4.0, 8.0], [12.0, 12.0, 16.0])
    };
    Some(BlockCollisionShape::single(shape_box).rotate_to_direction(facing))
}

fn turtle_egg_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    let eggs = properties.get("eggs")?.parse::<u8>().ok()?;
    if eggs == 1 {
        return Some(BlockCollisionShape::single(BlockCollisionBox::from_pixels(
            [3.0, 0.0, 3.0],
            [12.0, 7.0, 12.0],
        )));
    }
    if eggs <= 4 {
        return Some(BlockCollisionShape::single(
            BlockCollisionBox::centered_column(14.0, 14.0, 0.0, 7.0),
        ));
    }
    None
}

fn door_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let open = bool_property(properties, "open")?;
    let direction = if open {
        match properties.get("hinge").map(String::as_str)? {
            "left" => facing.clockwise(),
            "right" => facing.counter_clockwise(),
            _ => return None,
        }
    } else {
        facing
    };
    Some(
        BlockCollisionShape::single(BlockCollisionBox::NORTH_VERTICAL_THIN)
            .rotate_to_direction(direction),
    )
}

fn trapdoor_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    if bool_property(properties, "open")? {
        let facing = HorizontalDirection::parse(properties.get("facing")?)?;
        return Some(
            BlockCollisionShape::single(BlockCollisionBox::NORTH_VERTICAL_THIN)
                .rotate_to_direction(facing),
        );
    }
    match properties.get("half").map(String::as_str)? {
        "bottom" => Some(BlockCollisionShape::single(
            BlockCollisionBox::BOTTOM_TRAPDOOR,
        )),
        "top" => Some(BlockCollisionShape::single(BlockCollisionBox::TOP_TRAPDOOR)),
        _ => None,
    }
}

fn fence_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    cross_collision_shape(
        properties,
        BlockCollisionBox::FENCE_POST,
        BlockCollisionBox::FENCE_NORTH_ARM,
        BlockCollisionBox::FENCE_EAST_ARM,
        BlockCollisionBox::FENCE_SOUTH_ARM,
        BlockCollisionBox::FENCE_WEST_ARM,
    )
}

fn fence_visual_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let mut shape = fence_collision_shape(properties)?;
    for shape_box in shape.boxes.iter_mut().flatten() {
        shape_box.max_y = 1.0;
    }
    Some(shape)
}

fn fence_gate_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    if bool_property(properties, "open")? {
        return None;
    }
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let shape_box = match facing.axis() {
        HorizontalAxis::X => BlockCollisionBox::FENCE_GATE_X_AXIS,
        HorizontalAxis::Z => BlockCollisionBox::FENCE_GATE_Z_AXIS,
    };
    Some(BlockCollisionShape::single(shape_box))
}

fn bars_or_pane_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    cross_collision_shape(
        properties,
        BlockCollisionBox::PANE_POST,
        BlockCollisionBox::PANE_NORTH_ARM,
        BlockCollisionBox::PANE_EAST_ARM,
        BlockCollisionBox::PANE_SOUTH_ARM,
        BlockCollisionBox::PANE_WEST_ARM,
    )
}

fn wall_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let mut builder = BlockCollisionShapeBuilder::new();
    if bool_property(properties, "up")? {
        builder.push(BlockCollisionBox::WALL_POST);
    }
    if wall_side_has_collision(properties, "north")? {
        builder.push(BlockCollisionBox::WALL_NORTH_SIDE);
    }
    if wall_side_has_collision(properties, "east")? {
        builder.push(BlockCollisionBox::WALL_EAST_SIDE);
    }
    if wall_side_has_collision(properties, "south")? {
        builder.push(BlockCollisionBox::WALL_SOUTH_SIDE);
    }
    if wall_side_has_collision(properties, "west")? {
        builder.push(BlockCollisionBox::WALL_WEST_SIDE);
    }
    Some(builder.build())
}

fn cauldron_collision_shape() -> BlockCollisionShape {
    let mut builder = BlockCollisionShapeBuilder::new();
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 0.0, 0.0],
        [2.0, 3.0, 4.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 0.0, 12.0],
        [2.0, 3.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [14.0, 0.0, 0.0],
        [16.0, 3.0, 4.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [14.0, 0.0, 12.0],
        [16.0, 3.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [2.0, 0.0, 0.0],
        [4.0, 3.0, 2.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [2.0, 0.0, 14.0],
        [4.0, 3.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [12.0, 0.0, 0.0],
        [14.0, 3.0, 2.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [12.0, 0.0, 14.0],
        [14.0, 3.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 3.0, 0.0],
        [16.0, 4.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 4.0, 0.0],
        [2.0, 16.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [14.0, 4.0, 0.0],
        [16.0, 16.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [2.0, 4.0, 0.0],
        [14.0, 16.0, 2.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [2.0, 4.0, 14.0],
        [14.0, 16.0, 16.0],
    ));
    builder.build()
}

fn hopper_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let spout = match properties.get("facing").map(String::as_str)? {
        "down" => BlockCollisionBox::from_pixels([6.0, 0.0, 6.0], [10.0, 6.0, 10.0]),
        "north" => BlockCollisionBox::from_pixels([6.0, 4.0, 0.0], [10.0, 8.0, 8.0]),
        "east" => BlockCollisionBox::from_pixels([8.0, 4.0, 6.0], [16.0, 8.0, 10.0]),
        "south" => BlockCollisionBox::from_pixels([6.0, 4.0, 8.0], [10.0, 8.0, 16.0]),
        "west" => BlockCollisionBox::from_pixels([0.0, 4.0, 6.0], [8.0, 8.0, 10.0]),
        _ => return None,
    };

    let mut builder = BlockCollisionShapeBuilder::new();
    push_spoutless_hopper_boxes(&mut builder);
    builder.push(spout);
    Some(builder.build())
}

fn push_spoutless_hopper_boxes(builder: &mut BlockCollisionShapeBuilder) {
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 10.0, 0.0],
        [16.0, 11.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 11.0, 0.0],
        [16.0, 16.0, 2.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 11.0, 14.0],
        [16.0, 16.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 11.0, 2.0],
        [2.0, 16.0, 14.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [14.0, 11.0, 2.0],
        [16.0, 16.0, 14.0],
    ));
    builder.push(BlockCollisionBox::centered_column(8.0, 8.0, 4.0, 10.0));
}

fn composter_collision_shape() -> BlockCollisionShape {
    let hole_y = 2.0;
    BlockCollisionShape::from_boxes([
        Some(BlockCollisionBox::from_pixels(
            [0.0, 0.0, 0.0],
            [16.0, hole_y, 16.0],
        )),
        Some(BlockCollisionBox::from_pixels(
            [0.0, hole_y, 0.0],
            [16.0, 16.0, 2.0],
        )),
        Some(BlockCollisionBox::from_pixels(
            [0.0, hole_y, 14.0],
            [16.0, 16.0, 16.0],
        )),
        Some(BlockCollisionBox::from_pixels(
            [0.0, hole_y, 2.0],
            [2.0, 16.0, 14.0],
        )),
        Some(BlockCollisionBox::from_pixels(
            [14.0, hole_y, 2.0],
            [16.0, 16.0, 14.0],
        )),
    ])
}

fn chest_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let shape_box = match properties.get("type").map(String::as_str)? {
        "single" => BlockCollisionBox::CHEST_SINGLE,
        "left" => {
            let connected = HorizontalDirection::parse(properties.get("facing")?)?.clockwise();
            chest_connected_collision_box(connected)
        }
        "right" => {
            let connected =
                HorizontalDirection::parse(properties.get("facing")?)?.counter_clockwise();
            chest_connected_collision_box(connected)
        }
        _ => return None,
    };
    Some(BlockCollisionShape::single(shape_box))
}

fn chest_connected_collision_box(connected: HorizontalDirection) -> BlockCollisionBox {
    match connected {
        HorizontalDirection::North => BlockCollisionBox::CHEST_CONNECTED_NORTH,
        HorizontalDirection::East => BlockCollisionBox::CHEST_CONNECTED_EAST,
        HorizontalDirection::South => BlockCollisionBox::CHEST_CONNECTED_SOUTH,
        HorizontalDirection::West => BlockCollisionBox::CHEST_CONNECTED_WEST,
    }
}

fn bed_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let connected = match properties.get("part").map(String::as_str)? {
        "head" => facing.opposite(),
        "foot" => facing,
        _ => return None,
    };
    Some(bed_collision_shape_for_direction(connected.opposite()))
}

fn bed_collision_shape_for_direction(direction: HorizontalDirection) -> BlockCollisionShape {
    let (left_leg, right_leg) = match direction {
        HorizontalDirection::North => (
            BlockCollisionBox::from_pixels([0.0, 0.0, 0.0], [3.0, 3.0, 3.0]),
            BlockCollisionBox::from_pixels([13.0, 0.0, 0.0], [16.0, 3.0, 3.0]),
        ),
        HorizontalDirection::East => (
            BlockCollisionBox::from_pixels([13.0, 0.0, 0.0], [16.0, 3.0, 3.0]),
            BlockCollisionBox::from_pixels([13.0, 0.0, 13.0], [16.0, 3.0, 16.0]),
        ),
        HorizontalDirection::South => (
            BlockCollisionBox::from_pixels([13.0, 0.0, 13.0], [16.0, 3.0, 16.0]),
            BlockCollisionBox::from_pixels([0.0, 0.0, 13.0], [3.0, 3.0, 16.0]),
        ),
        HorizontalDirection::West => (
            BlockCollisionBox::from_pixels([0.0, 0.0, 13.0], [3.0, 3.0, 16.0]),
            BlockCollisionBox::from_pixels([0.0, 0.0, 0.0], [3.0, 3.0, 3.0]),
        ),
    };
    BlockCollisionShape::from_boxes([
        Some(BlockCollisionBox::BED_PLATFORM),
        Some(left_leg),
        Some(right_leg),
    ])
}

fn anvil_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let boxes = match facing.axis() {
        HorizontalAxis::X => [
            Some(BlockCollisionBox::centered_column(12.0, 12.0, 0.0, 4.0)),
            Some(BlockCollisionBox::centered_column(10.0, 8.0, 4.0, 5.0)),
            Some(BlockCollisionBox::centered_column(8.0, 4.0, 5.0, 10.0)),
            Some(BlockCollisionBox::centered_column(16.0, 10.0, 10.0, 16.0)),
        ],
        HorizontalAxis::Z => [
            Some(BlockCollisionBox::centered_column(12.0, 12.0, 0.0, 4.0)),
            Some(BlockCollisionBox::centered_column(8.0, 10.0, 4.0, 5.0)),
            Some(BlockCollisionBox::centered_column(4.0, 8.0, 5.0, 10.0)),
            Some(BlockCollisionBox::centered_column(10.0, 16.0, 10.0, 16.0)),
        ],
    };
    Some(BlockCollisionShape::from_boxes(boxes))
}

fn end_portal_frame_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    let mut builder = BlockCollisionShapeBuilder::new();
    builder.push(BlockCollisionBox::centered_column(16.0, 16.0, 0.0, 13.0));
    if bool_property(properties, "eye")? {
        builder.push(BlockCollisionBox::centered_column(8.0, 8.0, 13.0, 16.0));
    }
    Some(builder.build())
}

fn lectern_collision_shape() -> BlockCollisionShape {
    BlockCollisionShape::from_boxes([
        Some(BlockCollisionBox::centered_column(16.0, 16.0, 0.0, 2.0)),
        Some(BlockCollisionBox::centered_column(8.0, 8.0, 2.0, 14.0)),
    ])
}

fn grindstone_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let shape = match properties.get("face").map(String::as_str)? {
        "wall" => grindstone_north_wall_collision_shape(),
        "floor" => grindstone_north_wall_collision_shape().map_boxes(rotate_grindstone_to_floor),
        "ceiling" => {
            grindstone_north_wall_collision_shape().map_boxes(rotate_grindstone_to_ceiling)
        }
        _ => return None,
    };
    Some(shape.rotate_to_direction(facing))
}

fn grindstone_north_wall_collision_shape() -> BlockCollisionShape {
    BlockCollisionShape::from_boxes([
        Some(BlockCollisionBox::from_pixels(
            [4.0, 2.0, 0.0],
            [12.0, 14.0, 12.0],
        )),
        Some(BlockCollisionBox::from_pixels(
            [2.0, 6.0, 7.0],
            [4.0, 10.0, 16.0],
        )),
        Some(BlockCollisionBox::from_pixels(
            [2.0, 5.0, 3.0],
            [4.0, 11.0, 9.0],
        )),
        Some(BlockCollisionBox::from_pixels(
            [12.0, 6.0, 7.0],
            [14.0, 10.0, 16.0],
        )),
        Some(BlockCollisionBox::from_pixels(
            [12.0, 5.0, 3.0],
            [14.0, 11.0, 9.0],
        )),
    ])
}

fn rotate_grindstone_to_floor(shape_box: BlockCollisionBox) -> BlockCollisionBox {
    BlockCollisionBox {
        min_x: shape_box.min_x,
        min_y: 1.0 - shape_box.max_z,
        min_z: shape_box.min_y,
        max_x: shape_box.max_x,
        max_y: 1.0 - shape_box.min_z,
        max_z: shape_box.max_y,
    }
}

fn rotate_grindstone_to_ceiling(shape_box: BlockCollisionBox) -> BlockCollisionBox {
    BlockCollisionBox {
        min_x: 1.0 - shape_box.max_x,
        min_y: shape_box.min_z,
        min_z: shape_box.min_y,
        max_x: 1.0 - shape_box.min_x,
        max_y: shape_box.max_z,
        max_z: shape_box.max_y,
    }
}

fn bell_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    match properties.get("attachment").map(String::as_str)? {
        "floor" => Some(BlockCollisionShape::single(bell_floor_collision_box(
            facing,
        ))),
        "ceiling" => Some(bell_ceiling_collision_shape()),
        "single_wall" => Some(bell_single_wall_collision_shape(facing)),
        "double_wall" => Some(bell_double_wall_collision_shape(facing)),
        _ => None,
    }
}

fn bell_floor_collision_box(facing: HorizontalDirection) -> BlockCollisionBox {
    match facing.axis() {
        HorizontalAxis::X => BlockCollisionBox::BELL_FLOOR_X_AXIS,
        HorizontalAxis::Z => BlockCollisionBox::BELL_FLOOR_Z_AXIS,
    }
}

fn bell_ceiling_collision_shape() -> BlockCollisionShape {
    let mut builder = bell_body_collision_shape_builder();
    builder.push(BlockCollisionBox::BELL_CEILING_SUPPORT);
    builder.build()
}

fn bell_single_wall_collision_shape(facing: HorizontalDirection) -> BlockCollisionShape {
    let mut builder = bell_body_collision_shape_builder();
    builder.push(BlockCollisionBox::BELL_SINGLE_WALL_NORTH_SUPPORT.rotate_to_direction(facing));
    builder.build()
}

fn bell_double_wall_collision_shape(facing: HorizontalDirection) -> BlockCollisionShape {
    let mut builder = bell_body_collision_shape_builder();
    let support = match facing.axis() {
        HorizontalAxis::X => BlockCollisionBox::BELL_DOUBLE_WALL_X_AXIS_SUPPORT,
        HorizontalAxis::Z => BlockCollisionBox::BELL_DOUBLE_WALL_Z_AXIS_SUPPORT,
    };
    builder.push(support);
    builder.build()
}

fn bell_body_collision_shape_builder() -> BlockCollisionShapeBuilder {
    let mut builder = BlockCollisionShapeBuilder::new();
    builder.push(BlockCollisionBox::BELL_BODY);
    builder.push(BlockCollisionBox::BELL_BASE);
    builder
}

fn brewing_stand_collision_shape() -> BlockCollisionShape {
    BlockCollisionShape::from_boxes([
        Some(BlockCollisionBox::BREWING_STAND_ROD),
        Some(BlockCollisionBox::BREWING_STAND_BASE),
    ])
}

fn cross_collision_shape(
    properties: &BTreeMap<String, String>,
    post: BlockCollisionBox,
    north_arm: BlockCollisionBox,
    east_arm: BlockCollisionBox,
    south_arm: BlockCollisionBox,
    west_arm: BlockCollisionBox,
) -> Option<BlockCollisionShape> {
    let mut builder = BlockCollisionShapeBuilder::new();
    builder.push(post);
    if bool_property(properties, "north")? {
        builder.push(north_arm);
    }
    if bool_property(properties, "east")? {
        builder.push(east_arm);
    }
    if bool_property(properties, "south")? {
        builder.push(south_arm);
    }
    if bool_property(properties, "west")? {
        builder.push(west_arm);
    }
    Some(builder.build())
}

fn bool_property(properties: &BTreeMap<String, String>, key: &str) -> Option<bool> {
    match properties.get(key).map(String::as_str)? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn wall_side_has_collision(properties: &BTreeMap<String, String>, key: &str) -> Option<bool> {
    match properties.get(key).map(String::as_str)? {
        "low" | "tall" => Some(true),
        "none" => Some(false),
        _ => None,
    }
}

fn bounds_intersects_block_shape(
    bounds: LocalPlayerBounds,
    pos: BlockPos,
    shape: BlockCollisionShape,
) -> bool {
    shape
        .boxes()
        .any(|shape_box| bounds_intersects_block_box(bounds, pos, shape_box))
}

fn world_collision_box_intersects_bounds(
    collision_box: WorldCollisionBox,
    bounds: LocalPlayerBounds,
) -> bool {
    collision_box.max[0] > bounds.min_x
        && collision_box.min[0] < bounds.max_x
        && collision_box.max[1] > bounds.min_y
        && collision_box.min[1] < bounds.max_y
        && collision_box.max[2] > bounds.min_z
        && collision_box.min[2] < bounds.max_z
}

/// Vanilla `AABB.intersects(Vec3, Vec3)`: strict overlap with the endpoint
/// bounds, not a ray/segment clip.
pub(super) fn world_collision_box_intersects_segment_bounds(
    collision_box: WorldCollisionBox,
    from: [f64; 3],
    to: [f64; 3],
) -> bool {
    (0..3).all(|axis| {
        collision_box.min[axis] < from[axis].max(to[axis])
            && collision_box.max[axis] > from[axis].min(to[axis])
    })
}

fn world_collision_box(pos: BlockPos, shape: BlockCollisionBox) -> WorldCollisionBox {
    WorldCollisionBox {
        min: [
            f64::from(pos.x) + shape.min_x,
            f64::from(pos.y) + shape.min_y,
            f64::from(pos.z) + shape.min_z,
        ],
        max: [
            f64::from(pos.x) + shape.max_x,
            f64::from(pos.y) + shape.max_y,
            f64::from(pos.z) + shape.max_z,
        ],
    }
}

fn bounds_intersects_block_box(
    bounds: LocalPlayerBounds,
    pos: BlockPos,
    shape: BlockCollisionBox,
) -> bool {
    let block_x = f64::from(pos.x);
    let min_x = block_x + shape.min_x;
    let max_x = block_x + shape.max_x;
    let min_y = f64::from(pos.y) + shape.min_y;
    let max_y = f64::from(pos.y) + shape.max_y;
    let block_z = f64::from(pos.z);
    let min_z = block_z + shape.min_z;
    let max_z = block_z + shape.max_z;

    bounds.max_x > min_x + COLLISION_EPSILON
        && bounds.min_x < max_x - COLLISION_EPSILON
        && bounds.max_y > min_y + COLLISION_EPSILON
        && bounds.min_y < max_y - COLLISION_EPSILON
        && bounds.max_z > min_z + COLLISION_EPSILON
        && bounds.min_z < max_z - COLLISION_EPSILON
}

fn bounds_intersects_block_box_orthogonal(
    bounds: LocalPlayerBounds,
    pos: BlockPos,
    shape: BlockCollisionBox,
    axis: CollisionAxis,
) -> bool {
    match axis {
        CollisionAxis::X => {
            bounds_axis_overlaps_block_box(bounds, pos, shape, CollisionAxis::Y)
                && bounds_axis_overlaps_block_box(bounds, pos, shape, CollisionAxis::Z)
        }
        CollisionAxis::Y => {
            bounds_axis_overlaps_block_box(bounds, pos, shape, CollisionAxis::X)
                && bounds_axis_overlaps_block_box(bounds, pos, shape, CollisionAxis::Z)
        }
        CollisionAxis::Z => {
            bounds_axis_overlaps_block_box(bounds, pos, shape, CollisionAxis::X)
                && bounds_axis_overlaps_block_box(bounds, pos, shape, CollisionAxis::Y)
        }
    }
}

fn bounds_axis_overlaps_block_box(
    bounds: LocalPlayerBounds,
    pos: BlockPos,
    shape: BlockCollisionBox,
    axis: CollisionAxis,
) -> bool {
    bounds_axis_max(bounds, axis) > block_box_axis_min(pos, shape, axis) + COLLISION_EPSILON
        && bounds_axis_min(bounds, axis) < block_box_axis_max(pos, shape, axis) - COLLISION_EPSILON
}

fn bounds_axis_min(bounds: LocalPlayerBounds, axis: CollisionAxis) -> f64 {
    match axis {
        CollisionAxis::X => bounds.min_x,
        CollisionAxis::Y => bounds.min_y,
        CollisionAxis::Z => bounds.min_z,
    }
}

fn bounds_axis_max(bounds: LocalPlayerBounds, axis: CollisionAxis) -> f64 {
    match axis {
        CollisionAxis::X => bounds.max_x,
        CollisionAxis::Y => bounds.max_y,
        CollisionAxis::Z => bounds.max_z,
    }
}

fn world_box_axis_min(bounds: WorldCollisionBox, axis: CollisionAxis) -> f64 {
    match axis {
        CollisionAxis::X => bounds.min[0],
        CollisionAxis::Y => bounds.min[1],
        CollisionAxis::Z => bounds.min[2],
    }
}

fn world_box_axis_max(bounds: WorldCollisionBox, axis: CollisionAxis) -> f64 {
    match axis {
        CollisionAxis::X => bounds.max[0],
        CollisionAxis::Y => bounds.max[1],
        CollisionAxis::Z => bounds.max[2],
    }
}

fn block_box_axis_min(pos: BlockPos, shape: BlockCollisionBox, axis: CollisionAxis) -> f64 {
    match axis {
        CollisionAxis::X => f64::from(pos.x) + shape.min_x,
        CollisionAxis::Y => f64::from(pos.y) + shape.min_y,
        CollisionAxis::Z => f64::from(pos.z) + shape.min_z,
    }
}

fn block_box_axis_max(pos: BlockPos, shape: BlockCollisionBox, axis: CollisionAxis) -> f64 {
    match axis {
        CollisionAxis::X => f64::from(pos.x) + shape.max_x,
        CollisionAxis::Y => f64::from(pos.y) + shape.max_y,
        CollisionAxis::Z => f64::from(pos.z) + shape.max_z,
    }
}

fn block_floor(value: f64) -> i32 {
    value.floor() as i32
}

#[derive(Debug, Clone, Copy)]
pub(super) enum CollisionAxis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HorizontalDirection {
    North,
    East,
    South,
    West,
}

impl HorizontalDirection {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "north" => Some(Self::North),
            "east" => Some(Self::East),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            _ => None,
        }
    }

    fn clockwise(self) -> Self {
        match self {
            Self::North => Self::East,
            Self::East => Self::South,
            Self::South => Self::West,
            Self::West => Self::North,
        }
    }

    fn counter_clockwise(self) -> Self {
        match self {
            Self::North => Self::West,
            Self::East => Self::North,
            Self::South => Self::East,
            Self::West => Self::South,
        }
    }

    fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::West => Self::East,
        }
    }

    fn quarter_turns_from_north(self) -> usize {
        match self {
            Self::North => 0,
            Self::East => 1,
            Self::South => 2,
            Self::West => 3,
        }
    }

    fn axis(self) -> HorizontalAxis {
        match self {
            Self::North | Self::South => HorizontalAxis::Z,
            Self::East | Self::West => HorizontalAxis::X,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HorizontalAxis {
    X,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShapeAxis {
    X,
    Y,
    Z,
}

impl ShapeAxis {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "x" => Some(Self::X),
            "y" => Some(Self::Y),
            "z" => Some(Self::Z),
            _ => None,
        }
    }

    fn parse_direction(value: &str) -> Option<Self> {
        match value {
            "east" | "west" => Some(Self::X),
            "up" | "down" => Some(Self::Y),
            "north" | "south" => Some(Self::Z),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockDirection {
    North,
    East,
    South,
    West,
    Up,
    Down,
}

impl BlockDirection {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "north" => Some(Self::North),
            "east" => Some(Self::East),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            _ => None,
        }
    }

    fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

fn block_direction(direction: ProtocolDirection) -> BlockDirection {
    match direction {
        ProtocolDirection::Down => BlockDirection::Down,
        ProtocolDirection::Up => BlockDirection::Up,
        ProtocolDirection::North => BlockDirection::North,
        ProtocolDirection::South => BlockDirection::South,
        ProtocolDirection::West => BlockDirection::West,
        ProtocolDirection::East => BlockDirection::East,
    }
}

fn vanilla_support_direction(direction: ProtocolDirection) -> VanillaBlockSupportDirection {
    match direction {
        ProtocolDirection::Down => VanillaBlockSupportDirection::Down,
        ProtocolDirection::Up => VanillaBlockSupportDirection::Up,
        ProtocolDirection::North => VanillaBlockSupportDirection::North,
        ProtocolDirection::South => VanillaBlockSupportDirection::South,
        ProtocolDirection::West => VanillaBlockSupportDirection::West,
        ProtocolDirection::East => VanillaBlockSupportDirection::East,
    }
}

fn protocol_direction_index(direction: ProtocolDirection) -> usize {
    match direction {
        ProtocolDirection::Down => 0,
        ProtocolDirection::Up => 1,
        ProtocolDirection::North => 2,
        ProtocolDirection::South => 3,
        ProtocolDirection::West => 4,
        ProtocolDirection::East => 5,
    }
}

const VANILLA_PROTOCOL_DIRECTIONS: [ProtocolDirection; 6] = [
    ProtocolDirection::Down,
    ProtocolDirection::Up,
    ProtocolDirection::North,
    ProtocolDirection::South,
    ProtocolDirection::West,
    ProtocolDirection::East,
];

const DEBUG_SOLID_FACE_DIRECTIONS: [BlockDirection; 6] = [
    BlockDirection::West,
    BlockDirection::South,
    BlockDirection::East,
    BlockDirection::North,
    BlockDirection::Down,
    BlockDirection::Up,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StairShapeKind {
    Straight,
    Outer,
    Inner,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LocalPlayerCollisionContext {
    entity_bottom: f64,
    descending: bool,
    fall_distance: f64,
    can_walk_on_powder_snow: bool,
    can_stand_on_lava: bool,
    is_falling_block: bool,
}

impl LocalPlayerCollisionContext {
    pub(super) fn new(entity_bottom: f64, descending: bool) -> Self {
        Self {
            entity_bottom,
            descending,
            fall_distance: 0.0,
            can_walk_on_powder_snow: false,
            can_stand_on_lava: false,
            is_falling_block: false,
        }
    }

    pub(super) fn for_pose(
        world: &WorldStore,
        pose: LocalPlayerPoseState,
        descending: bool,
    ) -> Self {
        Self {
            entity_bottom: pose.position.y,
            descending,
            fall_distance: pose.fall_distance,
            can_walk_on_powder_snow: world.local_player_can_walk_on_powder_snow(),
            can_stand_on_lava: false,
            is_falling_block: false,
        }
    }

    fn for_remote_entity(
        entity_bottom: f64,
        descending: bool,
        can_walk_on_powder_snow: bool,
        is_falling_block: bool,
    ) -> Self {
        Self {
            entity_bottom,
            descending,
            fall_distance: 0.0,
            can_walk_on_powder_snow,
            can_stand_on_lava: false,
            is_falling_block,
        }
    }

    fn with_can_stand_on_lava(mut self, can_stand_on_lava: bool) -> Self {
        self.can_stand_on_lava = can_stand_on_lava;
        self
    }

    fn for_bounds(bounds: LocalPlayerBounds) -> Self {
        Self::new(bounds.min_y, false)
    }

    fn is_above(self, pos: BlockPos, shape_max_y: f64) -> bool {
        self.entity_bottom
            > f64::from(pos.y) + shape_max_y - VANILLA_COLLISION_CONTEXT_ABOVE_EPSILON
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LocalPlayerBounds {
    min_x: f64,
    min_y: f64,
    min_z: f64,
    max_x: f64,
    max_y: f64,
    max_z: f64,
}

impl LocalPlayerBounds {
    pub(super) fn for_pose(pose: LocalPlayerPoseState) -> Self {
        Self::at_height(pose.position, pose.body_height())
    }

    fn at_height(position: bbb_protocol::packets::Vec3d, height: f64) -> Self {
        Self {
            min_x: position.x - LOCAL_PLAYER_HALF_WIDTH,
            min_y: position.y,
            min_z: position.z - LOCAL_PLAYER_HALF_WIDTH,
            max_x: position.x + LOCAL_PLAYER_HALF_WIDTH,
            max_y: position.y + height,
            max_z: position.z + LOCAL_PLAYER_HALF_WIDTH,
        }
    }

    pub(super) fn from_min_max(min: [f64; 3], max: [f64; 3]) -> Self {
        Self {
            min_x: min[0],
            min_y: min[1],
            min_z: min[2],
            max_x: max[0],
            max_y: max[1],
            max_z: max[2],
        }
    }

    fn inflated(self, amount: f64) -> Self {
        Self {
            min_x: self.min_x - amount,
            min_y: self.min_y - amount,
            min_z: self.min_z - amount,
            max_x: self.max_x + amount,
            max_y: self.max_y + amount,
            max_z: self.max_z + amount,
        }
    }

    pub(super) fn moved(self, x: f64, y: f64, z: f64) -> Self {
        Self {
            min_x: self.min_x + x,
            min_y: self.min_y + y,
            min_z: self.min_z + z,
            max_x: self.max_x + x,
            max_y: self.max_y + y,
            max_z: self.max_z + z,
        }
    }

    pub(super) fn deflated(self, amount: f64) -> Self {
        Self {
            min_x: self.min_x + amount,
            min_y: self.min_y + amount,
            min_z: self.min_z + amount,
            max_x: self.max_x - amount,
            max_y: self.max_y - amount,
            max_z: self.max_z - amount,
        }
    }

    pub(super) fn min_x(self) -> f64 {
        self.min_x
    }

    pub(super) fn min_y(self) -> f64 {
        self.min_y
    }

    pub(super) fn min_z(self) -> f64 {
        self.min_z
    }

    pub(super) fn max_x(self) -> f64 {
        self.max_x
    }

    pub(super) fn max_y(self) -> f64 {
        self.max_y
    }

    pub(super) fn max_z(self) -> f64 {
        self.max_z
    }

    pub(super) fn edge_support_probe(self, min_height: f64) -> Self {
        Self {
            min_x: self.min_x + COLLISION_EPSILON,
            min_y: self.min_y - min_height - COLLISION_EPSILON,
            min_z: self.min_z + COLLISION_EPSILON,
            max_x: self.max_x - COLLISION_EPSILON,
            max_y: self.min_y,
            max_z: self.max_z - COLLISION_EPSILON,
        }
    }

    fn moved_axis(self, axis: CollisionAxis, amount: f64) -> Self {
        match axis {
            CollisionAxis::X => self.moved(amount, 0.0, 0.0),
            CollisionAxis::Y => self.moved(0.0, amount, 0.0),
            CollisionAxis::Z => self.moved(0.0, 0.0, amount),
        }
    }

    pub(super) fn swept_axis(self, axis: CollisionAxis, amount: f64) -> Self {
        let moved = self.moved_axis(axis, amount);
        match axis {
            CollisionAxis::X => Self {
                min_x: self.min_x.min(moved.min_x),
                max_x: self.max_x.max(moved.max_x),
                ..self
            },
            CollisionAxis::Y => Self {
                min_y: self.min_y.min(moved.min_y),
                max_y: self.max_y.max(moved.max_y),
                ..self
            },
            CollisionAxis::Z => Self {
                min_z: self.min_z.min(moved.min_z),
                max_z: self.max_z.max(moved.max_z),
                ..self
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct BlockCollisionShape {
    boxes: [Option<BlockCollisionBox>; MAX_COLLISION_BOXES],
}

impl BlockCollisionShape {
    fn single(shape_box: BlockCollisionBox) -> Self {
        let mut boxes = [None; MAX_COLLISION_BOXES];
        boxes[0] = Some(shape_box);
        Self { boxes }
    }

    fn from_boxes<const N: usize>(boxes: [Option<BlockCollisionBox>; N]) -> Self {
        let mut out = [None; MAX_COLLISION_BOXES];
        let mut next_index = 0;
        for shape_box in boxes.into_iter().flatten() {
            if next_index >= MAX_COLLISION_BOXES {
                break;
            }
            out[next_index] = Some(shape_box);
            next_index += 1;
        }
        Self { boxes: out }
    }

    fn boxes(self) -> impl Iterator<Item = BlockCollisionBox> {
        self.boxes.into_iter().flatten()
    }

    /// Mirrors `VoxelShape.toAabbs()`: build the coordinate-compressed union,
    /// then apply `BitSetDiscreteVoxelShape.forAllBoxes(..., true)`'s z-strip,
    /// x-neighbor, and y-neighbor merge order.
    fn to_aabbs(self) -> Vec<BlockCollisionBox> {
        let boxes = self.boxes().collect::<Vec<_>>();
        if boxes.len() <= 1 {
            return boxes;
        }
        CompressedCollisionShape::from_boxes(&boxes).merged_boxes()
    }

    fn is_full_block(self) -> bool {
        let mut boxes = self.boxes();
        matches!(boxes.next(), Some(BlockCollisionBox::FULL)) && boxes.next().is_none()
    }

    fn full_face_flags(self) -> [bool; 6] {
        DEBUG_SOLID_FACE_DIRECTIONS.map(|direction| self.has_full_face(direction))
    }

    /// Mirrors `Block.isFaceFull(shape, direction)`: sample the boundary
    /// slice selected by `VoxelShape.getFaceShape`, then require its union to
    /// cover the complete unit square.
    fn has_full_face(self, direction: BlockDirection) -> bool {
        self.covers_face_rectangle(direction, [0.0, 1.0, 0.0, 1.0])
    }

    fn has_center_face(self, direction: BlockDirection) -> bool {
        self.covers_face_rectangle(direction, [7.0 * PX, 9.0 * PX, 7.0 * PX, 9.0 * PX])
    }

    fn has_rigid_face(self, direction: BlockDirection) -> bool {
        [
            [0.0, 2.0 * PX, 0.0, 1.0],
            [14.0 * PX, 1.0, 0.0, 1.0],
            [2.0 * PX, 14.0 * PX, 0.0, 2.0 * PX],
            [2.0 * PX, 14.0 * PX, 14.0 * PX, 1.0],
        ]
        .into_iter()
        .all(|target| self.covers_face_rectangle(direction, target))
    }

    fn has_nonempty_face(self, direction: BlockDirection) -> bool {
        self.boxes()
            .any(|shape_box| block_collision_box_reaches_face(shape_box, direction))
    }

    fn covers_face_rectangle(self, direction: BlockDirection, target: [f64; 4]) -> bool {
        let [target_u0, target_u1, target_v0, target_v1] = target;
        if target_u0 >= target_u1 || target_v0 >= target_v1 {
            return false;
        }
        let face_boxes = self
            .boxes()
            .filter(|shape_box| block_collision_box_reaches_face(*shape_box, direction))
            .collect::<Vec<_>>();
        if face_boxes.is_empty() {
            return false;
        }

        let projection = |shape_box: BlockCollisionBox| match direction {
            BlockDirection::West | BlockDirection::East => [
                shape_box.min_y,
                shape_box.max_y,
                shape_box.min_z,
                shape_box.max_z,
            ],
            BlockDirection::South | BlockDirection::North => [
                shape_box.min_x,
                shape_box.max_x,
                shape_box.min_y,
                shape_box.max_y,
            ],
            BlockDirection::Down | BlockDirection::Up => [
                shape_box.min_x,
                shape_box.max_x,
                shape_box.min_z,
                shape_box.max_z,
            ],
        };
        let mut u_coords = vec![target_u0, target_u1];
        let mut v_coords = vec![target_v0, target_v1];
        for shape_box in &face_boxes {
            let [u0, u1, v0, v1] = projection(*shape_box);
            u_coords.extend([
                u0.clamp(target_u0, target_u1),
                u1.clamp(target_u0, target_u1),
            ]);
            v_coords.extend([
                v0.clamp(target_v0, target_v1),
                v1.clamp(target_v0, target_v1),
            ]);
        }
        u_coords.sort_by(f64::total_cmp);
        u_coords.dedup_by(|left, right| *left == *right);
        v_coords.sort_by(f64::total_cmp);
        v_coords.dedup_by(|left, right| *left == *right);

        u_coords.windows(2).all(|u| {
            v_coords.windows(2).all(|v| {
                let midpoint_u = (u[0] + u[1]) * 0.5;
                let midpoint_v = (v[0] + v[1]) * 0.5;
                face_boxes.iter().any(|shape_box| {
                    let [u0, u1, v0, v1] = projection(*shape_box);
                    midpoint_u >= u0 && midpoint_u < u1 && midpoint_v >= v0 && midpoint_v < v1
                })
            })
        })
    }

    fn max_y_at(self, local_x: f64, local_z: f64) -> f64 {
        self.boxes()
            .filter(|shape_box| shape_box.contains_xz(local_x, local_z))
            .map(|shape_box| shape_box.max_y)
            .fold(0.0, f64::max)
    }

    fn environment_animate_bounds(self) -> Option<EnvironmentAnimateCollisionBounds> {
        let mut boxes = self.boxes();
        let first = boxes.next()?;
        Some(boxes.fold(
            EnvironmentAnimateCollisionBounds {
                min_x: first.min_x,
                min_y: first.min_y,
                min_z: first.min_z,
                max_x: first.max_x,
                max_y: first.max_y,
                max_z: first.max_z,
            },
            |bounds, shape_box| EnvironmentAnimateCollisionBounds {
                min_x: bounds.min_x.min(shape_box.min_x),
                min_y: bounds.min_y.min(shape_box.min_y),
                min_z: bounds.min_z.min(shape_box.min_z),
                max_x: bounds.max_x.max(shape_box.max_x),
                max_y: bounds.max_y.max(shape_box.max_y),
                max_z: bounds.max_z.max(shape_box.max_z),
            },
        ))
    }

    fn invert_y(self) -> Self {
        Self {
            boxes: self
                .boxes
                .map(|shape_box| shape_box.map(BlockCollisionBox::invert_y)),
        }
    }

    fn offset(self, x: f64, y: f64, z: f64) -> Self {
        self.map_boxes(|shape_box| shape_box.offset(x, y, z))
    }

    fn map_boxes(self, mut f: impl FnMut(BlockCollisionBox) -> BlockCollisionBox) -> Self {
        Self {
            boxes: self.boxes.map(|shape_box| shape_box.map(&mut f)),
        }
    }

    fn rotate_to_direction(self, direction: HorizontalDirection) -> Self {
        let mut rotated = self;
        for _ in 0..direction.quarter_turns_from_north() {
            rotated = Self {
                boxes: rotated
                    .boxes
                    .map(|shape_box| shape_box.map(BlockCollisionBox::rotate_y_90)),
            };
        }
        rotated
    }

    fn rotate_all_from_north(self, direction: BlockDirection) -> Self {
        match direction {
            BlockDirection::North => self,
            BlockDirection::East => self.rotate_to_direction(HorizontalDirection::East),
            BlockDirection::South => self.rotate_to_direction(HorizontalDirection::South),
            BlockDirection::West => self.rotate_to_direction(HorizontalDirection::West),
            BlockDirection::Up => self.map_boxes(BlockCollisionBox::rotate_x_270),
            BlockDirection::Down => self.map_boxes(BlockCollisionBox::rotate_x_90),
        }
    }
}

fn block_collision_box_reaches_face(
    shape_box: BlockCollisionBox,
    direction: BlockDirection,
) -> bool {
    const NEGATIVE_FACE_SAMPLE: f64 = 1.0e-7;
    const POSITIVE_FACE_SAMPLE: f64 = 0.999_999_9;

    let (min, max, sample) = match direction {
        BlockDirection::West => (shape_box.min_x, shape_box.max_x, NEGATIVE_FACE_SAMPLE),
        BlockDirection::South => (shape_box.min_z, shape_box.max_z, POSITIVE_FACE_SAMPLE),
        BlockDirection::East => (shape_box.min_x, shape_box.max_x, POSITIVE_FACE_SAMPLE),
        BlockDirection::North => (shape_box.min_z, shape_box.max_z, NEGATIVE_FACE_SAMPLE),
        BlockDirection::Down => (shape_box.min_y, shape_box.max_y, NEGATIVE_FACE_SAMPLE),
        BlockDirection::Up => (shape_box.min_y, shape_box.max_y, POSITIVE_FACE_SAMPLE),
    };
    min <= sample && sample < max
}

struct CompressedCollisionShape {
    x_coords: Vec<f64>,
    y_coords: Vec<f64>,
    z_coords: Vec<f64>,
    full: Vec<bool>,
    x_size: usize,
    y_size: usize,
    z_size: usize,
}

impl CompressedCollisionShape {
    fn from_boxes(boxes: &[BlockCollisionBox]) -> Self {
        let x_coords =
            collision_shape_axis_coords(boxes, |shape_box| [shape_box.min_x, shape_box.max_x]);
        let y_coords =
            collision_shape_axis_coords(boxes, |shape_box| [shape_box.min_y, shape_box.max_y]);
        let z_coords =
            collision_shape_axis_coords(boxes, |shape_box| [shape_box.min_z, shape_box.max_z]);
        let x_size = x_coords.len().saturating_sub(1);
        let y_size = y_coords.len().saturating_sub(1);
        let z_size = z_coords.len().saturating_sub(1);
        let mut full = vec![false; x_size.saturating_mul(y_size).saturating_mul(z_size)];

        for shape_box in boxes {
            let x0 = collision_shape_coord_index(&x_coords, shape_box.min_x);
            let x1 = collision_shape_coord_index(&x_coords, shape_box.max_x);
            let y0 = collision_shape_coord_index(&y_coords, shape_box.min_y);
            let y1 = collision_shape_coord_index(&y_coords, shape_box.max_y);
            let z0 = collision_shape_coord_index(&z_coords, shape_box.min_z);
            let z1 = collision_shape_coord_index(&z_coords, shape_box.max_z);
            for y in y0..y1 {
                for x in x0..x1 {
                    for z in z0..z1 {
                        full[Self::index(x_size, z_size, x, y, z)] = true;
                    }
                }
            }
        }

        Self {
            x_coords,
            y_coords,
            z_coords,
            full,
            x_size,
            y_size,
            z_size,
        }
    }

    fn merged_boxes(mut self) -> Vec<BlockCollisionBox> {
        let mut boxes = Vec::new();
        for y in 0..self.y_size {
            for x in 0..self.x_size {
                let mut start_z = None;
                for z in 0..=self.z_size {
                    if z < self.z_size && self.is_full(x, y, z) {
                        start_z.get_or_insert(z);
                    } else if let Some(z0) = start_z.take() {
                        let mut x1 = x + 1;
                        let mut y1 = y + 1;
                        self.clear_z_strip(z0, z, x, y);
                        while x1 < self.x_size && self.is_z_strip_full(z0, z, x1, y) {
                            self.clear_z_strip(z0, z, x1, y);
                            x1 += 1;
                        }
                        while y1 < self.y_size && self.is_xz_rectangle_full(x, x1, z0, z, y1) {
                            for clear_x in x..x1 {
                                self.clear_z_strip(z0, z, clear_x, y1);
                            }
                            y1 += 1;
                        }
                        boxes.push(BlockCollisionBox {
                            min_x: self.x_coords[x],
                            min_y: self.y_coords[y],
                            min_z: self.z_coords[z0],
                            max_x: self.x_coords[x1],
                            max_y: self.y_coords[y1],
                            max_z: self.z_coords[z],
                        });
                    }
                }
            }
        }
        boxes
    }

    fn index(x_size: usize, z_size: usize, x: usize, y: usize, z: usize) -> usize {
        (y * x_size + x) * z_size + z
    }

    fn is_full(&self, x: usize, y: usize, z: usize) -> bool {
        self.full[Self::index(self.x_size, self.z_size, x, y, z)]
    }

    fn is_z_strip_full(&self, z0: usize, z1: usize, x: usize, y: usize) -> bool {
        (z0..z1).all(|z| self.is_full(x, y, z))
    }

    fn is_xz_rectangle_full(&self, x0: usize, x1: usize, z0: usize, z1: usize, y: usize) -> bool {
        (x0..x1).all(|x| self.is_z_strip_full(z0, z1, x, y))
    }

    fn clear_z_strip(&mut self, z0: usize, z1: usize, x: usize, y: usize) {
        for z in z0..z1 {
            let index = Self::index(self.x_size, self.z_size, x, y, z);
            self.full[index] = false;
        }
    }
}

fn collision_shape_axis_coords(
    boxes: &[BlockCollisionBox],
    axis: impl Fn(BlockCollisionBox) -> [f64; 2],
) -> Vec<f64> {
    let mut coords = boxes.iter().copied().flat_map(axis).collect::<Vec<_>>();
    coords.sort_by(f64::total_cmp);
    coords.dedup_by(|left, right| *left == *right);
    coords
}

fn collision_shape_coord_index(coords: &[f64], target: f64) -> usize {
    coords
        .iter()
        .position(|coordinate| *coordinate == target)
        .expect("collision shape coordinate was inserted from the same box")
}

struct BlockCollisionShapeBuilder {
    boxes: [Option<BlockCollisionBox>; MAX_COLLISION_BOXES],
    next_index: usize,
}

impl BlockCollisionShapeBuilder {
    fn new() -> Self {
        Self {
            boxes: [None; MAX_COLLISION_BOXES],
            next_index: 0,
        }
    }

    fn push(&mut self, shape_box: BlockCollisionBox) {
        if self.next_index < MAX_COLLISION_BOXES {
            self.boxes[self.next_index] = Some(shape_box);
            self.next_index += 1;
        }
    }

    fn extend(&mut self, shape: BlockCollisionShape) {
        for shape_box in shape.boxes() {
            self.push(shape_box);
        }
    }

    fn build(self) -> BlockCollisionShape {
        BlockCollisionShape { boxes: self.boxes }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BlockCollisionBox {
    min_x: f64,
    min_y: f64,
    min_z: f64,
    max_x: f64,
    max_y: f64,
    max_z: f64,
}

impl BlockCollisionBox {
    const FULL: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    const SCAFFOLDING_TOP: Self = Self {
        min_x: 0.0,
        min_y: 14.0 * PX,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    const SCAFFOLDING_UNSTABLE_BOTTOM: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 2.0 * PX,
        max_z: 1.0,
    };
    const POWDER_SNOW_FALLING: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: LOCAL_PLAYER_POWDER_SNOW_FALLING_COLLISION_HEIGHT,
        max_z: 1.0,
    };
    const SCAFFOLDING_NORTH_WEST_POST: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 2.0 * PX,
        max_y: 1.0,
        max_z: 2.0 * PX,
    };
    const SCAFFOLDING_NORTH_EAST_POST: Self = Self {
        min_x: 14.0 * PX,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 2.0 * PX,
    };
    const SCAFFOLDING_SOUTH_EAST_POST: Self = Self {
        min_x: 14.0 * PX,
        min_y: 0.0,
        min_z: 14.0 * PX,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    const SCAFFOLDING_SOUTH_WEST_POST: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 14.0 * PX,
        max_x: 2.0 * PX,
        max_y: 1.0,
        max_z: 1.0,
    };
    const BOTTOM_SLAB: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 0.5,
        max_z: 1.0,
    };
    const TOP_SLAB: Self = Self {
        min_x: 0.0,
        min_y: 0.5,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    const CARPET: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: PX,
        max_z: 1.0,
    };
    const CAMPFIRE: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 7.0 * PX,
        max_z: 1.0,
    };
    const FULL_WIDTH_15PX_HIGH: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 15.0 * PX,
        max_z: 1.0,
    };
    const FULL_WIDTH_14PX_HIGH: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 14.0 * PX,
        max_z: 1.0,
    };
    const CENTERED_14PX_COLUMN_15PX_HIGH: Self = Self {
        min_x: PX,
        min_y: 0.0,
        min_z: PX,
        max_x: 15.0 * PX,
        max_y: 15.0 * PX,
        max_z: 15.0 * PX,
    };
    const LILY_PAD: Self = Self {
        min_x: PX,
        min_y: 0.0,
        min_z: PX,
        max_x: 15.0 * PX,
        max_y: 1.5 * PX,
        max_z: 15.0 * PX,
    };
    const CHEST_SINGLE: Self = Self {
        min_x: PX,
        min_y: 0.0,
        min_z: PX,
        max_x: 15.0 * PX,
        max_y: 14.0 * PX,
        max_z: 15.0 * PX,
    };
    const CHEST_CONNECTED_NORTH: Self = Self {
        min_x: PX,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 15.0 * PX,
        max_y: 14.0 * PX,
        max_z: 15.0 * PX,
    };
    const CHEST_CONNECTED_EAST: Self = Self {
        min_x: PX,
        min_y: 0.0,
        min_z: PX,
        max_x: 1.0,
        max_y: 14.0 * PX,
        max_z: 15.0 * PX,
    };
    const CHEST_CONNECTED_SOUTH: Self = Self {
        min_x: PX,
        min_y: 0.0,
        min_z: PX,
        max_x: 15.0 * PX,
        max_y: 14.0 * PX,
        max_z: 1.0,
    };
    const CHEST_CONNECTED_WEST: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: PX,
        max_x: 15.0 * PX,
        max_y: 14.0 * PX,
        max_z: 15.0 * PX,
    };
    const BED_PLATFORM: Self = Self {
        min_x: 0.0,
        min_y: 3.0 * PX,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 9.0 * PX,
        max_z: 1.0,
    };
    const BELL_BODY: Self = Self {
        min_x: 5.0 * PX,
        min_y: 6.0 * PX,
        min_z: 5.0 * PX,
        max_x: 11.0 * PX,
        max_y: 13.0 * PX,
        max_z: 11.0 * PX,
    };
    const BELL_BASE: Self = Self {
        min_x: 4.0 * PX,
        min_y: 4.0 * PX,
        min_z: 4.0 * PX,
        max_x: 12.0 * PX,
        max_y: 6.0 * PX,
        max_z: 12.0 * PX,
    };
    const BELL_FLOOR_X_AXIS: Self = Self {
        min_x: 4.0 * PX,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 12.0 * PX,
        max_y: 1.0,
        max_z: 1.0,
    };
    const BELL_FLOOR_Z_AXIS: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 4.0 * PX,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 12.0 * PX,
    };
    const BELL_CEILING_SUPPORT: Self = Self {
        min_x: 7.0 * PX,
        min_y: 13.0 * PX,
        min_z: 7.0 * PX,
        max_x: 9.0 * PX,
        max_y: 1.0,
        max_z: 9.0 * PX,
    };
    const BELL_SINGLE_WALL_NORTH_SUPPORT: Self = Self {
        min_x: 7.0 * PX,
        min_y: 13.0 * PX,
        min_z: 0.0,
        max_x: 9.0 * PX,
        max_y: 15.0 * PX,
        max_z: 13.0 * PX,
    };
    const BELL_DOUBLE_WALL_X_AXIS_SUPPORT: Self = Self {
        min_x: 0.0,
        min_y: 13.0 * PX,
        min_z: 7.0 * PX,
        max_x: 1.0,
        max_y: 15.0 * PX,
        max_z: 9.0 * PX,
    };
    const BELL_DOUBLE_WALL_Z_AXIS_SUPPORT: Self = Self {
        min_x: 7.0 * PX,
        min_y: 13.0 * PX,
        min_z: 0.0,
        max_x: 9.0 * PX,
        max_y: 15.0 * PX,
        max_z: 1.0,
    };
    const BREWING_STAND_ROD: Self = Self {
        min_x: 7.0 * PX,
        min_y: 2.0 * PX,
        min_z: 7.0 * PX,
        max_x: 9.0 * PX,
        max_y: 14.0 * PX,
        max_z: 9.0 * PX,
    };
    const BREWING_STAND_BASE: Self = Self {
        min_x: PX,
        min_y: 0.0,
        min_z: PX,
        max_x: 15.0 * PX,
        max_y: 2.0 * PX,
        max_z: 15.0 * PX,
    };
    const BOTTOM_TRAPDOOR: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 3.0 * PX,
        max_z: 1.0,
    };
    const TOP_TRAPDOOR: Self = Self {
        min_x: 0.0,
        min_y: 13.0 * PX,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    const NORTH_VERTICAL_THIN: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 13.0 * PX,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    const STAIR_NORTH_HALF: Self = Self {
        min_x: 0.0,
        min_y: 0.5,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 0.5,
    };
    const STAIR_NORTH_WEST_OCTET: Self = Self {
        min_x: 0.0,
        min_y: 0.5,
        min_z: 0.0,
        max_x: 0.5,
        max_y: 1.0,
        max_z: 0.5,
    };
    const STAIR_SOUTH_EAST_OCTET: Self = Self {
        min_x: 0.5,
        min_y: 0.5,
        min_z: 0.5,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    const FENCE_POST: Self = Self {
        min_x: 6.0 * PX,
        min_y: 0.0,
        min_z: 6.0 * PX,
        max_x: 10.0 * PX,
        max_y: 1.5,
        max_z: 10.0 * PX,
    };
    const FENCE_NORTH_ARM: Self = Self {
        min_x: 6.0 * PX,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 10.0 * PX,
        max_y: 1.5,
        max_z: 8.0 * PX,
    };
    const FENCE_EAST_ARM: Self = Self {
        min_x: 8.0 * PX,
        min_y: 0.0,
        min_z: 6.0 * PX,
        max_x: 1.0,
        max_y: 1.5,
        max_z: 10.0 * PX,
    };
    const FENCE_SOUTH_ARM: Self = Self {
        min_x: 6.0 * PX,
        min_y: 0.0,
        min_z: 8.0 * PX,
        max_x: 10.0 * PX,
        max_y: 1.5,
        max_z: 1.0,
    };
    const FENCE_WEST_ARM: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 6.0 * PX,
        max_x: 8.0 * PX,
        max_y: 1.5,
        max_z: 10.0 * PX,
    };
    const FENCE_GATE_Z_AXIS: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 6.0 * PX,
        max_x: 1.0,
        max_y: 1.5,
        max_z: 10.0 * PX,
    };
    const FENCE_GATE_X_AXIS: Self = Self {
        min_x: 6.0 * PX,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 10.0 * PX,
        max_y: 1.5,
        max_z: 1.0,
    };
    const PANE_POST: Self = Self {
        min_x: 7.0 * PX,
        min_y: 0.0,
        min_z: 7.0 * PX,
        max_x: 9.0 * PX,
        max_y: 1.0,
        max_z: 9.0 * PX,
    };
    const PANE_NORTH_ARM: Self = Self {
        min_x: 7.0 * PX,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 9.0 * PX,
        max_y: 1.0,
        max_z: 8.0 * PX,
    };
    const PANE_EAST_ARM: Self = Self {
        min_x: 8.0 * PX,
        min_y: 0.0,
        min_z: 7.0 * PX,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 9.0 * PX,
    };
    const PANE_SOUTH_ARM: Self = Self {
        min_x: 7.0 * PX,
        min_y: 0.0,
        min_z: 8.0 * PX,
        max_x: 9.0 * PX,
        max_y: 1.0,
        max_z: 1.0,
    };
    const PANE_WEST_ARM: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 7.0 * PX,
        max_x: 8.0 * PX,
        max_y: 1.0,
        max_z: 9.0 * PX,
    };
    const WALL_POST: Self = Self {
        min_x: 4.0 * PX,
        min_y: 0.0,
        min_z: 4.0 * PX,
        max_x: 12.0 * PX,
        max_y: 1.5,
        max_z: 12.0 * PX,
    };
    const WALL_NORTH_SIDE: Self = Self {
        min_x: 5.0 * PX,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 11.0 * PX,
        max_y: 1.5,
        max_z: 11.0 * PX,
    };
    const WALL_EAST_SIDE: Self = Self {
        min_x: 5.0 * PX,
        min_y: 0.0,
        min_z: 5.0 * PX,
        max_x: 1.0,
        max_y: 1.5,
        max_z: 11.0 * PX,
    };
    const WALL_SOUTH_SIDE: Self = Self {
        min_x: 5.0 * PX,
        min_y: 0.0,
        min_z: 5.0 * PX,
        max_x: 11.0 * PX,
        max_y: 1.5,
        max_z: 1.0,
    };
    const WALL_WEST_SIDE: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 5.0 * PX,
        max_x: 11.0 * PX,
        max_y: 1.5,
        max_z: 11.0 * PX,
    };

    fn cuboid(min_x: f64, min_y: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> Self {
        Self {
            min_x,
            min_y,
            min_z,
            max_x,
            max_y,
            max_z,
        }
    }

    fn from_pixels(min: [f64; 3], max: [f64; 3]) -> Self {
        Self::cuboid(
            min[0] * PX,
            min[1] * PX,
            min[2] * PX,
            max[0] * PX,
            max[1] * PX,
            max[2] * PX,
        )
    }

    fn column(min_x: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> Self {
        Self {
            min_x,
            min_y: 0.0,
            min_z,
            max_x,
            max_y,
            max_z,
        }
    }

    fn centered_column(width_x_px: f64, width_z_px: f64, min_y_px: f64, max_y_px: f64) -> Self {
        Self::from_pixels(
            [
                (16.0 - width_x_px) * 0.5,
                min_y_px,
                (16.0 - width_z_px) * 0.5,
            ],
            [
                (16.0 + width_x_px) * 0.5,
                max_y_px,
                (16.0 + width_z_px) * 0.5,
            ],
        )
    }

    fn centered_axis(axis: ShapeAxis, width_px: f64) -> Self {
        let min = ((16.0 - width_px) / 2.0) * PX;
        let max = ((16.0 + width_px) / 2.0) * PX;
        match axis {
            ShapeAxis::X => Self::cuboid(0.0, min, min, 1.0, max, max),
            ShapeAxis::Y => Self::cuboid(min, 0.0, min, max, 1.0, max),
            ShapeAxis::Z => Self::cuboid(min, min, 0.0, max, max, 1.0),
        }
    }

    fn invert_y(self) -> Self {
        Self {
            min_x: self.min_x,
            min_y: 1.0 - self.max_y,
            min_z: self.min_z,
            max_x: self.max_x,
            max_y: 1.0 - self.min_y,
            max_z: self.max_z,
        }
    }

    fn offset(self, x: f64, y: f64, z: f64) -> Self {
        Self {
            min_x: self.min_x + x,
            min_y: self.min_y + y,
            min_z: self.min_z + z,
            max_x: self.max_x + x,
            max_y: self.max_y + y,
            max_z: self.max_z + z,
        }
    }

    fn rotate_y_90(self) -> Self {
        Self {
            min_x: 1.0 - self.max_z,
            min_y: self.min_y,
            min_z: self.min_x,
            max_x: 1.0 - self.min_z,
            max_y: self.max_y,
            max_z: self.max_x,
        }
    }

    fn rotate_x_90(self) -> Self {
        Self {
            min_x: self.min_x,
            min_y: self.min_z,
            min_z: 1.0 - self.max_y,
            max_x: self.max_x,
            max_y: self.max_z,
            max_z: 1.0 - self.min_y,
        }
    }

    fn rotate_x_270(self) -> Self {
        Self {
            min_x: self.min_x,
            min_y: 1.0 - self.max_z,
            min_z: self.min_y,
            max_x: self.max_x,
            max_y: 1.0 - self.min_z,
            max_z: self.max_y,
        }
    }

    fn rotate_to_direction(self, direction: HorizontalDirection) -> Self {
        let mut rotated = self;
        for _ in 0..direction.quarter_turns_from_north() {
            rotated = rotated.rotate_y_90();
        }
        rotated
    }

    fn contains_xz(self, local_x: f64, local_z: f64) -> bool {
        local_x >= self.min_x
            && local_x < self.max_x
            && local_z >= self.min_z
            && local_z < self.max_z
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::vanilla_entity_metadata_entity_accessors;
    use crate::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_BAT_ID,
        VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_SHULKER_ID,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    };
    use bbb_protocol::packets::SetBorderSize as ProtocolSetBorderSize;

    fn entity_shift_key_down_data() -> bbb_protocol::packets::EntityDataValue {
        let accessors = vanilla_entity_metadata_entity_accessors()
            .expect("generated base Entity metadata accessors must resolve");
        assert_eq!(accessors.shared_flags_default, 0);
        bbb_protocol::packets::EntityDataValue {
            data_id: accessors.shared_flags_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: bbb_protocol::packets::EntityDataValueKind::Byte(accessors.shift_key_down_mask),
        }
    }

    fn entity_pose_data(pose_id: i32) -> bbb_protocol::packets::EntityDataValue {
        let accessors = vanilla_entity_metadata_entity_accessors()
            .expect("generated base Entity metadata accessors must resolve");
        bbb_protocol::packets::EntityDataValue {
            data_id: accessors.pose_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
            value: bbb_protocol::packets::EntityDataValueKind::Pose(pose_id),
        }
    }

    #[test]
    fn leaves_collision_membership_rejects_suffix_lookalikes() {
        assert!(is_leaves_block_name("minecraft:oak_leaves"));
        assert!(is_leaves_block_name("minecraft:flowering_azalea_leaves"));
        assert!(!is_leaves_block_name("minecraft:custom_leaves"));
        assert!(!is_leaves_block_name("example:oak_leaves"));
    }

    #[test]
    fn vanilla_custom_collision_shape_blocks_do_not_reach_material_fallback() {
        let mut world = empty_world();
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let context = LocalPlayerCollisionContext::new(2.0, false);
        let mut unresolved = Vec::new();

        for block_name in crate::terrain::vanilla_block_collision_shape_override_names() {
            let block_type = vanilla_block_type_for_resource_id(block_name)
                .unwrap_or_else(|| panic!("generated vanilla block type exists for {block_name}"));
            let state_id =
                bbb_protocol::block_state_types::vanilla_block_default_state_id(block_type.id)
                    .unwrap_or_else(|| {
                        panic!("generated vanilla default state exists for {block_name}")
                    });
            set_block(&mut world, pos, state_id);
            let probe = world.probe_block(pos).expect("catalog block probe");

            let mut opaque = probe.clone();
            opaque.material = TerrainMaterialClass::Opaque;
            let opaque_boxes = block_collision_shape_with_context(&world, &opaque, pos, context)
                .map(|shape| shape.boxes().collect::<Vec<_>>());

            let mut empty = probe;
            empty.material = TerrainMaterialClass::Empty;
            let empty_boxes = block_collision_shape_with_context(&world, &empty, pos, context)
                .map(|shape| shape.boxes().collect::<Vec<_>>());

            if opaque_boxes != empty_boxes {
                unresolved.push(*block_name);
            }
        }

        assert!(
            unresolved.is_empty(),
            "vanilla custom collision-shape blocks reached the material fallback: {unresolved:?}"
        );
    }

    #[test]
    fn every_static_vanilla_state_projects_the_generated_collision_aabbs_exactly() {
        let world = empty_world();
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let template = world.probe_block(pos).expect("air template probe");

        for state in crate::registries::BlockStateRegistry::vanilla_26_1().iter() {
            let Some(expected) = vanilla_block_state_static_collision_shape(state.id) else {
                continue;
            };
            let probe =
                block_probe_for_state(&world, &template, state.id).expect("vanilla state probe");
            let actual = block_collision_shape(&probe, pos)
                .map(|shape| {
                    shape
                        .boxes()
                        .map(|shape_box| {
                            [
                                shape_box.min_x,
                                shape_box.min_y,
                                shape_box.min_z,
                                shape_box.max_x,
                                shape_box.max_y,
                                shape_box.max_z,
                            ]
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let expected = expected
                .boxes()
                .map(|shape_box| {
                    [
                        shape_box.min_x,
                        shape_box.min_y,
                        shape_box.min_z,
                        shape_box.max_x,
                        shape_box.max_y,
                        shape_box.max_z,
                    ]
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "{} ({})", state.name, state.id);
        }
    }

    #[test]
    fn every_dynamic_vanilla_state_uses_a_live_resolver_branch() {
        let world = empty_world();
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let template = world.probe_block(pos).expect("air template probe");
        let context = LocalPlayerCollisionContext::new(2.0, false);
        let mut dynamic_state_count = 0;

        for state in crate::registries::BlockStateRegistry::vanilla_26_1().iter() {
            if !vanilla_block_state_support_facts(state.id)
                .unwrap()
                .dynamic_shape()
            {
                continue;
            }
            dynamic_state_count += 1;
            let probe =
                block_probe_for_state(&world, &template, state.id).expect("dynamic state probe");
            let mut opaque = probe.clone();
            opaque.material = TerrainMaterialClass::Opaque;
            let opaque_boxes = block_collision_shape_with_context(&world, &opaque, pos, context)
                .map(|shape| shape.boxes().collect::<Vec<_>>());
            let mut empty = probe;
            empty.material = TerrainMaterialClass::Empty;
            let empty_boxes = block_collision_shape_with_context(&world, &empty, pos, context)
                .map(|shape| shape.boxes().collect::<Vec<_>>());
            assert_eq!(
                opaque_boxes, empty_boxes,
                "dynamic vanilla state reached material fallback: {} ({})",
                state.name, state.id
            );
        }

        assert_eq!(dynamic_state_count, 179);
    }

    #[test]
    fn diode_and_light_collision_shapes_match_vanilla() {
        let mut world = empty_world();
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        for block_name in ["minecraft:repeater", "minecraft:comparator"] {
            let state_id = bbb_protocol::block_state_types::vanilla_block_default_state_id(
                vanilla_block_type_for_resource_id(block_name).unwrap().id,
            )
            .unwrap();
            set_block(&mut world, pos, state_id);
            let probe = world.probe_block(pos).expect("diode probe");
            let shape_box = block_collision_shape(&probe, pos)
                .expect("diode collision shape")
                .boxes()
                .next()
                .expect("diode collision box");
            assert_eq!(
                shape_box,
                BlockCollisionBox::centered_column(16.0, 16.0, 0.0, 2.0),
                "{block_name}"
            );
        }

        set_block(
            &mut world,
            pos,
            vanilla_block_state_id(
                "minecraft:light",
                [("level", "15"), ("waterlogged", "false")],
            ),
        );
        let mut light = world.probe_block(pos).expect("light probe");
        light.material = TerrainMaterialClass::Opaque;
        assert!(
            block_collision_shape(&light, pos).is_none(),
            "LightBlock's entity collision uses its empty-context outline shape"
        );
    }

    #[test]
    fn shulker_box_collision_shape_tracks_current_lid_progress_in_all_directions() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let cases = [
            ("north", [0.0, 0.0, -0.25], [1.0, 1.0, 1.0]),
            ("east", [0.0, 0.0, 0.0], [1.25, 1.0, 1.0]),
            ("south", [0.0, 0.0, 0.0], [1.0, 1.0, 1.25]),
            ("west", [-0.25, 0.0, 0.0], [1.0, 1.0, 1.0]),
            ("up", [0.0, 0.0, 0.0], [1.0, 1.25, 1.0]),
            ("down", [0.0, -0.25, 0.0], [1.0, 1.0, 1.0]),
        ];

        for (facing, expected_min, expected_max) in cases {
            let mut world = empty_world();
            set_block(
                &mut world,
                pos,
                vanilla_block_state_id("minecraft:shulker_box", [("facing", facing)]),
            );
            let closed = world.probe_block(pos).expect("closed shulker probe");
            assert_eq!(
                shulker_box_collision_shape(&world, &closed, pos)
                    .unwrap()
                    .boxes()
                    .next(),
                Some(BlockCollisionBox::FULL)
            );

            world.apply_block_event(bbb_protocol::packets::BlockEvent {
                pos: bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 },
                b0: 1,
                b1: 1,
                block_id: 0,
            });
            world.advance_shulker_box_lid_ticks(5);
            let opening = world.probe_block(pos).expect("opening shulker probe");
            let shape_box = shulker_box_collision_shape(&world, &opening, pos)
                .unwrap()
                .boxes()
                .next()
                .unwrap();
            for (actual, expected) in [
                (shape_box.min_x, expected_min[0]),
                (shape_box.min_y, expected_min[1]),
                (shape_box.min_z, expected_min[2]),
                (shape_box.max_x, expected_max[0]),
                (shape_box.max_y, expected_max[1]),
                (shape_box.max_z, expected_max[2]),
            ] {
                assert_f64_near(actual, expected);
            }
        }
    }

    #[test]
    fn cocoa_shape_matches_vanilla_age_and_facing() {
        let mut properties = BTreeMap::new();
        properties.insert("age".to_owned(), "2".to_owned());
        properties.insert("facing".to_owned(), "north".to_owned());

        let shape = cocoa_collision_shape(&properties).expect("north cocoa shape");
        let mut boxes = shape.boxes();
        let shape_box = boxes.next().expect("shape box");
        assert!(boxes.next().is_none());

        assert_f64_near(shape_box.min_x, 4.0 * PX);
        assert_f64_near(shape_box.min_y, 3.0 * PX);
        assert_f64_near(shape_box.min_z, PX);
        assert_f64_near(shape_box.max_x, 12.0 * PX);
        assert_f64_near(shape_box.max_y, 12.0 * PX);
        assert_f64_near(shape_box.max_z, 9.0 * PX);

        properties.insert("facing".to_owned(), "south".to_owned());
        let shape = cocoa_collision_shape(&properties).expect("south cocoa shape");
        let mut boxes = shape.boxes();
        let shape_box = boxes.next().expect("shape box");
        assert!(boxes.next().is_none());

        assert_f64_near(shape_box.min_x, 4.0 * PX);
        assert_f64_near(shape_box.min_y, 3.0 * PX);
        assert_f64_near(shape_box.min_z, 7.0 * PX);
        assert_f64_near(shape_box.max_x, 12.0 * PX);
        assert_f64_near(shape_box.max_y, 12.0 * PX);
        assert_f64_near(shape_box.max_z, 15.0 * PX);
    }

    #[test]
    fn shelf_shape_matches_vanilla_backboard_and_lips() {
        let mut properties = BTreeMap::new();
        properties.insert("facing".to_owned(), "north".to_owned());

        let shape = shelf_collision_shape(&properties).expect("north shelf shape");
        let boxes = shape.boxes().collect::<Vec<_>>();
        assert_eq!(boxes.len(), 3);

        assert_f64_near(boxes[0].min_x, 0.0);
        assert_f64_near(boxes[0].min_y, 12.0 * PX);
        assert_f64_near(boxes[0].min_z, 11.0 * PX);
        assert_f64_near(boxes[0].max_x, 1.0);
        assert_f64_near(boxes[0].max_y, 1.0);
        assert_f64_near(boxes[0].max_z, 13.0 * PX);

        assert_f64_near(boxes[1].min_x, 0.0);
        assert_f64_near(boxes[1].min_y, 0.0);
        assert_f64_near(boxes[1].min_z, 13.0 * PX);
        assert_f64_near(boxes[1].max_x, 1.0);
        assert_f64_near(boxes[1].max_y, 1.0);
        assert_f64_near(boxes[1].max_z, 1.0);

        assert_f64_near(boxes[2].min_x, 0.0);
        assert_f64_near(boxes[2].min_y, 0.0);
        assert_f64_near(boxes[2].min_z, 11.0 * PX);
        assert_f64_near(boxes[2].max_x, 1.0);
        assert_f64_near(boxes[2].max_y, 4.0 * PX);
        assert_f64_near(boxes[2].max_z, 13.0 * PX);

        properties.insert("facing".to_owned(), "south".to_owned());
        let shape = shelf_collision_shape(&properties).expect("south shelf shape");
        let boxes = shape.boxes().collect::<Vec<_>>();
        assert_eq!(boxes.len(), 3);

        assert_f64_near(boxes[0].min_z, 3.0 * PX);
        assert_f64_near(boxes[0].max_z, 5.0 * PX);
        assert_f64_near(boxes[1].min_z, 0.0);
        assert_f64_near(boxes[1].max_z, 3.0 * PX);
        assert_f64_near(boxes[2].min_z, 3.0 * PX);
        assert_f64_near(boxes[2].max_z, 5.0 * PX);
    }

    #[test]
    fn amethyst_cluster_shape_matches_vanilla_size_and_facing() {
        for block_name in [
            "minecraft:amethyst_cluster",
            "minecraft:large_amethyst_bud",
            "minecraft:medium_amethyst_bud",
            "minecraft:small_amethyst_bud",
        ] {
            assert!(is_amethyst_cluster_block_name(block_name), "{block_name}");
        }
        assert!(!is_amethyst_cluster_block_name(
            "minecraft:custom_amethyst_bud"
        ));

        let mut properties = BTreeMap::new();
        properties.insert("facing".to_owned(), "north".to_owned());

        let shape = amethyst_cluster_collision_shape("minecraft:amethyst_cluster", &properties)
            .expect("north amethyst cluster shape");
        let mut boxes = shape.boxes();
        let shape_box = boxes.next().expect("shape box");
        assert!(boxes.next().is_none());

        assert_f64_near(shape_box.min_x, 3.0 * PX);
        assert_f64_near(shape_box.min_y, 3.0 * PX);
        assert_f64_near(shape_box.min_z, 9.0 * PX);
        assert_f64_near(shape_box.max_x, 13.0 * PX);
        assert_f64_near(shape_box.max_y, 13.0 * PX);
        assert_f64_near(shape_box.max_z, 1.0);

        properties.insert("facing".to_owned(), "up".to_owned());
        let shape = amethyst_cluster_collision_shape("minecraft:amethyst_cluster", &properties)
            .expect("up amethyst cluster shape");
        let mut boxes = shape.boxes();
        let shape_box = boxes.next().expect("shape box");
        assert!(boxes.next().is_none());

        assert_f64_near(shape_box.min_x, 3.0 * PX);
        assert_f64_near(shape_box.min_y, 0.0);
        assert_f64_near(shape_box.min_z, 3.0 * PX);
        assert_f64_near(shape_box.max_x, 13.0 * PX);
        assert_f64_near(shape_box.max_y, 7.0 * PX);
        assert_f64_near(shape_box.max_z, 13.0 * PX);

        let shape = amethyst_cluster_collision_shape("minecraft:small_amethyst_bud", &properties)
            .expect("up small amethyst bud shape");
        let mut boxes = shape.boxes();
        let shape_box = boxes.next().expect("shape box");
        assert!(boxes.next().is_none());

        assert_f64_near(shape_box.min_x, 4.0 * PX);
        assert_f64_near(shape_box.min_y, 0.0);
        assert_f64_near(shape_box.min_z, 4.0 * PX);
        assert_f64_near(shape_box.max_x, 12.0 * PX);
        assert_f64_near(shape_box.max_y, 3.0 * PX);
        assert_f64_near(shape_box.max_z, 12.0 * PX);
    }

    #[test]
    fn copper_golem_statue_shape_matches_vanilla_column() {
        for block_name in [
            "minecraft:copper_golem_statue",
            "minecraft:exposed_copper_golem_statue",
            "minecraft:waxed_oxidized_copper_golem_statue",
        ] {
            assert!(
                is_copper_golem_statue_block_name(block_name),
                "{block_name}"
            );
        }
        assert!(!is_copper_golem_statue_block_name("minecraft:copper_golem"));

        let shape = copper_golem_statue_collision_shape();
        let mut boxes = shape.boxes();
        let shape_box = boxes.next().expect("shape box");
        assert!(boxes.next().is_none());

        assert_f64_near(shape_box.min_x, 3.0 * PX);
        assert_f64_near(shape_box.min_y, 0.0);
        assert_f64_near(shape_box.min_z, 3.0 * PX);
        assert_f64_near(shape_box.max_x, 13.0 * PX);
        assert_f64_near(shape_box.max_y, 14.0 * PX);
        assert_f64_near(shape_box.max_z, 13.0 * PX);
    }

    #[test]
    fn pointed_dripstone_xz_offset_matches_vanilla_seed() {
        let (origin_x, origin_z) = pointed_dripstone_xz_offset(BlockPos { x: 0, y: 0, z: 0 });
        assert_f64_near(origin_x, -0.125);
        assert_f64_near(origin_z, -0.125);

        let (offset_x, offset_z) = pointed_dripstone_xz_offset(BlockPos { x: 1, y: 99, z: 0 });
        assert_f64_near(offset_x, 7.0 / 60.0);
        assert_f64_near(offset_z, -1.0 / 60.0);
    }

    #[test]
    fn pointed_dripstone_tip_up_shape_uses_direction_height_and_offset() {
        let mut properties = BTreeMap::new();
        properties.insert("thickness".to_owned(), "tip".to_owned());
        properties.insert("vertical_direction".to_owned(), "up".to_owned());

        let shape = pointed_dripstone_collision_shape(&properties, BlockPos { x: 0, y: 7, z: 0 })
            .expect("pointed dripstone shape");
        let mut boxes = shape.boxes();
        let shape_box = boxes.next().expect("shape box");
        assert!(boxes.next().is_none());

        assert_f64_near(shape_box.min_x, 3.0 * PX);
        assert_f64_near(shape_box.min_y, 0.0);
        assert_f64_near(shape_box.min_z, 3.0 * PX);
        assert_f64_near(shape_box.max_x, 9.0 * PX);
        assert_f64_near(shape_box.max_y, 11.0 * PX);
        assert_f64_near(shape_box.max_z, 9.0 * PX);
    }

    #[test]
    fn particle_y_collision_clips_downward_motion_to_shape_top() {
        let bounds = particle_collision_bounds([0.5, 1.05, 0.5], 0.1, 0.2);
        let clipped = clipped_particle_axis_against_box(
            bounds,
            CollisionAxis::Y,
            -0.2,
            -0.2,
            BlockPos { x: 0, y: 0, z: 0 },
            BlockCollisionBox::FULL,
        );

        assert_f64_near(clipped, -0.05);
    }

    #[test]
    fn entity_collision_clip_accepts_catalog_relative_bounds() {
        let mut world = empty_world();
        let grass = vanilla_block_state_id("minecraft:grass_block", [("snowy", "false")]);
        set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, grass);

        let clipped = world.clip_entity_collision_movement(
            [0.5, 1.1, 0.5],
            [0.0, -0.2, 0.0],
            [-0.49, 0.0, -0.49],
            [0.49, 0.98, 0.49],
        );

        assert_f64_near(clipped[0], 0.0);
        assert_f64_near(clipped[1], -0.1);
        assert_f64_near(clipped[2], 0.0);
    }

    #[test]
    fn boat_ground_friction_samples_the_thin_shape_slice_below_the_hull() {
        let mut world = empty_world();
        let stone = vanilla_block_state_id("minecraft:stone", []);
        set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, stone);

        let friction =
            world.boat_ground_friction([0.5, 1.0, 0.5], [-0.49, 0.0, -0.49], [0.49, 0.5625, 0.49]);

        assert!((friction - 0.6).abs() < f32::EPSILON);
        assert!(world
            .boat_ground_friction([2.5, 1.0, 2.5], [-0.49, 0.0, -0.49], [0.49, 0.5625, 0.49],)
            .is_nan());
    }

    #[test]
    fn projectile_block_clip_stops_at_the_first_collision_shape_surface() {
        let mut world = empty_world();
        let stone = vanilla_block_state_id("minecraft:stone", []);
        set_block(&mut world, BlockPos { x: 1, y: 1, z: 0 }, stone);

        let clipped = world.clip_projectile_block_movement(
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            [0.5, 1.5, 0.5],
            [2.0, 0.0, 0.0],
            false,
        );

        assert_f64_near(clipped.movement[0], 0.5);
        assert_f64_near(clipped.movement[1], 0.0);
        assert_f64_near(clipped.movement[2], 0.0);
        assert!(!clipped.world_border_hit);
        assert_eq!(clipped.block_state_id, Some(stone));
    }

    #[test]
    fn camera_visual_block_clip_stops_at_the_nearest_visual_surface() {
        let mut world = empty_world();
        world.set_local_player_pose(LocalPlayerPoseState::default());
        let stone = vanilla_block_state_id("minecraft:stone", []);
        set_block(&mut world, BlockPos { x: 1, y: 1, z: 0 }, stone);

        let hit = world
            .camera_visual_block_hit([0.5, 1.5, 0.5], [2.5, 1.5, 0.5])
            .expect("stone visual hit");

        assert_f64_near(hit[0], 1.0);
        assert_f64_near(hit[1], 1.5);
        assert_f64_near(hit[2], 0.5);
    }

    #[test]
    fn camera_visual_shape_overrides_match_vanilla_catalog() {
        let world = empty_world();
        let context = LocalPlayerCollisionContext::for_bounds(LocalPlayerBounds::from_min_max(
            [0.2, 1.0, 0.2],
            [0.8, 2.8, 0.8],
        ));
        let shape_for = |name: &str, properties: BTreeMap<String, String>| {
            let block = BlockProbe {
                pos: BlockPos { x: 0, y: 0, z: 0 },
                chunk: ChunkPos { x: 0, z: 0 },
                local_x: 0,
                local_y: 0,
                local_z: 0,
                section_y: 0,
                section_index: 0,
                block_state_id: 0,
                block_name: Some(name.to_owned()),
                block_properties: properties,
                material: TerrainMaterialClass::Opaque,
                fluid: None,
                block_palette_kind: PaletteKind::SingleValue,
                block_palette_index: Some(0),
                biome_id: Some(0),
                biome_palette_kind: PaletteKind::SingleValue,
            };
            block_visual_shape_with_context(&world, &block, block.pos, context)
        };

        for empty in [
            "minecraft:glass",
            "minecraft:red_stained_glass",
            "minecraft:tinted_glass",
            "minecraft:copper_grate",
            "minecraft:iron_bars",
            "minecraft:blue_stained_glass_pane",
            "minecraft:powder_snow",
        ] {
            assert!(shape_for(empty, BTreeMap::new()).is_none(), "{empty}");
        }

        for full in ["minecraft:soul_sand", "minecraft:mud"] {
            assert!(
                shape_for(full, BTreeMap::new()).is_some_and(BlockCollisionShape::is_full_block),
                "{full}"
            );
        }

        let snow = shape_for("minecraft:snow", string_props([("layers", "4")]))
            .expect("four-layer visual snow");
        assert_f64_near(snow.boxes().next().expect("snow box").max_y, 0.5);

        let fence = shape_for(
            "minecraft:oak_fence",
            string_props([
                ("east", "false"),
                ("north", "true"),
                ("south", "false"),
                ("waterlogged", "false"),
                ("west", "false"),
            ]),
        )
        .expect("fence visual shape");
        assert!(fence.boxes().all(|shape_box| shape_box.max_y == 1.0));
    }

    #[test]
    fn projectile_block_clip_uses_vanilla_inside_shape_test_point() {
        let mut world = empty_world();
        let stone = vanilla_block_state_id("minecraft:stone", []);
        set_block(&mut world, BlockPos { x: 0, y: 1, z: 0 }, stone);

        let clipped = world.clip_projectile_block_movement(
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            [0.5, 1.5, 0.5],
            [2.0, 0.0, 0.0],
            false,
        );

        assert_f64_near(clipped.movement[0], 0.002);
        assert_f64_near(clipped.movement[1], 0.0);
        assert_f64_near(clipped.movement[2], 0.0);
        assert!(!clipped.world_border_hit);
        assert_eq!(clipped.block_state_id, Some(stone));
        assert_eq!(
            world.projectile_position_inside_block_collision(
                VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
                [0.5, 1.5, 0.5],
                false,
            ),
            Some(stone),
        );
        assert_eq!(
            world.projectile_position_inside_block_collision(
                VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
                [1.5, 1.5, 0.5],
                false,
            ),
            None,
        );
    }

    #[test]
    fn projectile_block_clip_includes_the_world_border() {
        let mut world = empty_world();
        world.apply_set_border_size(ProtocolSetBorderSize { size: 4.0 });

        let clipped = world.clip_projectile_block_movement(
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            [0.0, 1.5, 0.0],
            [10.0, 0.0, 0.0],
            false,
        );

        assert_f64_near(clipped.movement[0], 2.0 - f64::from(1.0e-5_f32));
        assert_f64_near(clipped.movement[1], 0.0);
        assert_f64_near(clipped.movement[2], 0.0);
        assert!(clipped.world_border_hit);
        assert_eq!(clipped.block_state_id, None);
    }

    #[test]
    fn particle_y_collision_ignores_boxes_outside_horizontal_overlap() {
        let bounds = particle_collision_bounds([1.5, 1.05, 0.5], 0.1, 0.2);
        let clipped = clipped_particle_axis_against_box(
            bounds,
            CollisionAxis::Y,
            -0.2,
            -0.2,
            BlockPos { x: 0, y: 0, z: 0 },
            BlockCollisionBox::FULL,
        );

        assert_f64_near(clipped, -0.2);
    }

    #[test]
    fn particle_y_collision_clips_upward_motion_to_shape_bottom() {
        let bounds = particle_collision_bounds([0.5, -0.25, 0.5], 0.1, 0.2);
        let clipped = clipped_particle_axis_against_box(
            bounds,
            CollisionAxis::Y,
            0.2,
            0.2,
            BlockPos { x: 0, y: 0, z: 0 },
            BlockCollisionBox::FULL,
        );

        assert_f64_near(clipped, 0.05);
    }

    #[test]
    fn particle_x_collision_clips_horizontal_motion_to_shape_side() {
        let bounds = particle_collision_bounds([-0.15, 0.5, 0.5], 0.1, 0.2);
        let clipped = clipped_particle_axis_against_box(
            bounds,
            CollisionAxis::X,
            0.2,
            0.2,
            BlockPos { x: 0, y: 0, z: 0 },
            BlockCollisionBox::FULL,
        );

        assert_f64_near(clipped, 0.05);
    }

    #[test]
    fn particle_collision_axis_order_matches_vanilla_horizontal_priority() {
        assert_eq!(
            particle_collision_axis_order([0.3, -0.1, 0.2]).map(axis_name),
            ["y", "x", "z"]
        );
        assert_eq!(
            particle_collision_axis_order([0.2, -0.1, 0.3]).map(axis_name),
            ["y", "z", "x"]
        );
    }

    #[test]
    fn particle_block_fluid_surface_height_uses_block_collision_shape() {
        let mut world = empty_world();
        let air_sample = world.particle_block_fluid_surface_sample([0.5, 0.25, 0.5]);
        assert!(air_sample.block_is_air);

        let bottom_slab_id = vanilla_block_state_id(
            "minecraft:oak_slab",
            [("type", "bottom"), ("waterlogged", "false")],
        );
        set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, bottom_slab_id);

        let sample = world.particle_block_fluid_surface_sample([0.5, 0.25, 0.5]);
        assert_f64_near(sample.block_collision_height, 0.5);
        assert_f64_near(sample.fluid_height, 0.0);
        assert_eq!(sample.fluid_kind, None);
        assert!(!sample.block_is_air);
        assert_f64_near(
            world.particle_block_fluid_surface_height([0.5, 0.25, 0.5]),
            0.5,
        );
    }

    #[test]
    fn particle_block_fluid_surface_height_uses_fluid_height() {
        let mut world = empty_world();
        let water_id = vanilla_block_state_id("minecraft:water", [("level", "0")]);
        set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, water_id);

        let sample = world.particle_block_fluid_surface_sample([0.5, 0.25, 0.5]);
        assert_f64_near(sample.block_collision_height, 0.0);
        assert_f64_near(sample.fluid_height, 8.0 / 9.0);
        assert_eq!(sample.fluid_kind, Some(TerrainFluidKind::Water));
        assert!(!sample.block_is_air);
        assert_f64_near(
            world.particle_block_fluid_surface_height([0.5, 0.25, 0.5]),
            8.0 / 9.0,
        );
    }

    #[test]
    fn environment_animate_block_sample_reports_fluid_shape_and_drip_facts() {
        let mut world = empty_world();
        let pos = BlockPos { x: 0, y: 0, z: 0 };

        assert_eq!(
            world.environment_animate_block_sample(pos),
            Some(EnvironmentAnimateBlockSample {
                fluid: None,
                block_is_air: true,
                can_occlude: false,
                solid_render: false,
                full_face_sturdy: [false; 6],
                collision_full_face: [false; 6],
                fire_flammable: false,
                triggers_dried_ghast_ambient_sound: false,
                enchantment_power_provider: false,
                enchantment_power_transmitter: true,
                impermeable: false,
                collision_bounds: None,
            })
        );

        let stone_id = vanilla_block_state_id("minecraft:stone", []);
        set_block(&mut world, pos, stone_id);
        assert_eq!(
            world.environment_animate_block_sample(pos),
            Some(EnvironmentAnimateBlockSample {
                fluid: None,
                block_is_air: false,
                can_occlude: true,
                solid_render: true,
                full_face_sturdy: [true; 6],
                collision_full_face: [true; 6],
                fire_flammable: false,
                triggers_dried_ghast_ambient_sound: false,
                enchantment_power_provider: false,
                enchantment_power_transmitter: false,
                impermeable: false,
                collision_bounds: Some(EnvironmentAnimateCollisionBounds {
                    min_x: 0.0,
                    min_y: 0.0,
                    min_z: 0.0,
                    max_x: 1.0,
                    max_y: 1.0,
                    max_z: 1.0,
                }),
            })
        );

        let top_slab_id = vanilla_block_state_id(
            "minecraft:oak_slab",
            [("type", "top"), ("waterlogged", "false")],
        );
        set_block(&mut world, pos, top_slab_id);
        let top_slab = world
            .environment_animate_block_sample(pos)
            .expect("loaded top slab");
        assert!(!top_slab.solid_render);
        assert!(top_slab.can_occlude);
        assert_eq!(
            top_slab.full_face_sturdy,
            [false, true, false, false, false, false]
        );
        assert_eq!(
            top_slab.collision_full_face,
            [false, true, false, false, false, false]
        );
        assert_eq!(
            top_slab.collision_bounds,
            Some(EnvironmentAnimateCollisionBounds {
                min_x: 0.0,
                min_y: 0.5,
                min_z: 0.0,
                max_x: 1.0,
                max_y: 1.0,
                max_z: 1.0,
            })
        );

        let glass_id = vanilla_block_state_id("minecraft:glass", []);
        set_block(&mut world, pos, glass_id);
        let glass = world
            .environment_animate_block_sample(pos)
            .expect("loaded glass");
        assert!(glass.impermeable);
        assert!(!glass.can_occlude);
        assert!(!glass.solid_render);

        let oak_planks_id = vanilla_block_state_id("minecraft:oak_planks", []);
        set_block(&mut world, pos, oak_planks_id);
        assert!(
            world
                .environment_animate_block_sample(pos)
                .expect("loaded oak planks")
                .fire_flammable
        );

        let soul_sand_id = vanilla_block_state_id("minecraft:soul_sand", []);
        set_block(&mut world, pos, soul_sand_id);
        assert!(
            world
                .environment_animate_block_sample(pos)
                .expect("loaded soul sand")
                .triggers_dried_ghast_ambient_sound
        );

        let bookshelf_id = vanilla_block_state_id("minecraft:bookshelf", []);
        set_block(&mut world, pos, bookshelf_id);
        let bookshelf = world
            .environment_animate_block_sample(pos)
            .expect("loaded bookshelf");
        assert!(bookshelf.enchantment_power_provider);
        assert!(!bookshelf.enchantment_power_transmitter);

        set_block(&mut world, pos, 0);
        let air = world
            .environment_animate_block_sample(pos)
            .expect("loaded air");
        assert!(!air.enchantment_power_provider);
        assert!(air.enchantment_power_transmitter);

        let water_id = vanilla_block_state_id("minecraft:water", [("level", "0")]);
        set_block(&mut world, pos, water_id);
        assert_eq!(
            world
                .environment_animate_block_sample(pos)
                .expect("loaded water")
                .fluid,
            Some(crate::TerrainFluidState::new(
                TerrainFluidKind::Water,
                8,
                false,
            ))
        );
        assert_eq!(
            world.environment_animate_block_sample(BlockPos { x: 32, ..pos }),
            None
        );
    }

    #[test]
    fn falling_block_free_query_matches_air_fire_liquid_and_replaceable_states() {
        let mut world = empty_world();
        let pos = BlockPos { x: 0, y: 0, z: 0 };

        for block_state_id in [
            vanilla_block_state_id("minecraft:air", []),
            vanilla_block_state_id(
                "minecraft:fire",
                [
                    ("age", "0"),
                    ("east", "false"),
                    ("north", "false"),
                    ("south", "false"),
                    ("up", "false"),
                    ("west", "false"),
                ],
            ),
            vanilla_block_state_id("minecraft:soul_fire", []),
            vanilla_block_state_id("minecraft:water", [("level", "0")]),
            vanilla_block_state_id("minecraft:short_grass", []),
        ] {
            set_block(&mut world, pos, block_state_id);
            assert_eq!(world.falling_block_is_free(pos), Some(true));
        }

        set_block(
            &mut world,
            pos,
            vanilla_block_state_id("minecraft:stone", []),
        );
        assert_eq!(world.falling_block_is_free(pos), Some(false));
        assert_eq!(
            world.falling_block_is_free(BlockPos { y: -1, ..pos }),
            Some(true)
        );
        assert_eq!(world.falling_block_is_free(BlockPos { x: 32, ..pos }), None);
    }

    #[test]
    fn particle_block_fluid_surface_sample_reports_lava_kind_and_height() {
        let mut world = empty_world();
        let lava_id = vanilla_block_state_id("minecraft:lava", [("level", "0")]);
        set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, lava_id);

        let sample = world.particle_block_fluid_surface_sample([0.5, 0.25, 0.5]);

        assert_f64_near(sample.block_collision_height, 0.0);
        assert_f64_near(sample.fluid_height, 8.0 / 9.0);
        assert_eq!(sample.fluid_kind, Some(TerrainFluidKind::Lava));
        assert!(!sample.block_is_air);
        assert_f64_near(sample.max_surface_height(), 8.0 / 9.0);
    }

    #[test]
    fn particle_block_fluid_surface_height_is_empty_for_unloaded_or_nonfinite_position() {
        let world = empty_world();

        assert_eq!(
            world.particle_block_fluid_surface_sample([32.5, 0.25, 0.5]),
            ParticleBlockFluidSurfaceSample::default()
        );
        assert_f64_near(
            world.particle_block_fluid_surface_height([32.5, 0.25, 0.5]),
            0.0,
        );
        assert_eq!(
            world.particle_block_fluid_surface_sample([f64::NAN, 0.25, 0.5]),
            ParticleBlockFluidSurfaceSample::default()
        );
        assert_f64_near(
            world.particle_block_fluid_surface_height([f64::NAN, 0.25, 0.5]),
            0.0,
        );
    }

    #[test]
    fn debug_camera_block_collision_boxes_use_inflated_player_bounds_and_shape_components() {
        let mut world = empty_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: 0.5,
                y: 0.0,
                z: 0.5,
            },
            ..LocalPlayerPoseState::default()
        });
        let slab = vanilla_block_state_id(
            "minecraft:oak_slab",
            [("type", "bottom"), ("waterlogged", "false")],
        );
        let stone = vanilla_block_state_id("minecraft:stone", []);
        set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, slab);
        set_block(&mut world, BlockPos { x: 1, y: 0, z: 0 }, stone);
        set_block(&mut world, BlockPos { x: 3, y: 0, z: 0 }, stone);

        let boxes = world.debug_camera_block_collision_boxes(1.0);
        assert_eq!(
            boxes,
            [
                WorldCollisionBox {
                    min: [0.0, 0.0, 0.0],
                    max: [1.0, 0.5, 1.0],
                },
                WorldCollisionBox {
                    min: [1.0, 0.0, 0.0],
                    max: [2.0, 1.0, 1.0],
                },
            ]
        );
        assert!(world
            .debug_camera_block_collision_boxes(f64::NAN)
            .is_empty());
    }

    #[test]
    fn debug_camera_block_collision_boxes_require_a_camera_entity_pose() {
        let world = empty_world();
        assert!(world.debug_camera_block_collision_boxes(6.0).is_empty());
    }

    #[test]
    fn debug_camera_block_collision_boxes_follow_vanilla_cursor_x_y_z_order() {
        let mut world = empty_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: 0.5,
                y: 1.0,
                z: 0.5,
            },
            ..LocalPlayerPoseState::default()
        });
        let stone = vanilla_block_state_id("minecraft:stone", []);
        set_block(&mut world, BlockPos { x: 0, y: 1, z: 0 }, stone);
        set_block(&mut world, BlockPos { x: 0, y: 0, z: 1 }, stone);

        assert_eq!(
            world.debug_camera_block_collision_boxes(2.0),
            [
                WorldCollisionBox {
                    min: [0.0, 1.0, 0.0],
                    max: [1.0, 2.0, 1.0],
                },
                WorldCollisionBox {
                    min: [0.0, 0.0, 1.0],
                    max: [1.0, 1.0, 2.0],
                },
            ]
        );
    }

    #[test]
    fn debug_supporting_block_membership_uses_camera_bounds_inflated_by_radius() {
        let mut world = empty_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: 0.5,
                y: 1.0,
                z: 0.5,
            },
            ..LocalPlayerPoseState::default()
        });
        world.local_player_id = Some(99);
        add_collision_test_entity(
            &mut world,
            99,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [0.5, 1.0, 0.5],
        );
        add_collision_test_entity(
            &mut world,
            10,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            [17.2, 1.0, 0.5],
        );
        add_collision_test_entity(
            &mut world,
            11,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            [17.3, 1.0, 0.5],
        );

        assert_eq!(world.debug_supporting_block_entity_ids(16.0), [10]);
        assert!(world.debug_supporting_block_entity_ids(f64::NAN).is_empty());
        assert!(world.debug_supporting_block_entity_ids(-1.0).is_empty());
    }

    #[test]
    fn debug_local_supporting_block_uses_vanilla_distance_and_block_pos_tie_break() {
        let mut world = empty_world();
        let stone = vanilla_block_state_id("minecraft:stone", []);
        set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, stone);
        set_block(&mut world, BlockPos { x: 1, y: 0, z: 0 }, stone);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: 1.0,
                y: 1.0,
                z: 0.5,
            },
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let debug = world
            .debug_local_player_supporting_blocks()
            .expect("grounded player should have a support block");
        assert_eq!(debug.supporting.pos, BlockPos { x: 1, y: 0, z: 0 });
        assert_eq!(
            debug.supporting.collision_boxes,
            [WorldCollisionBox {
                min: [1.0, 0.0, 0.0],
                max: [2.0, 1.0, 1.0],
            }]
        );
        assert_eq!(debug.legacy_effect, None);

        let mut airborne = world.local_player_pose().unwrap();
        airborne.on_ground = false;
        world.set_local_player_pose(airborne);
        assert_eq!(world.debug_local_player_supporting_blocks(), None);
    }

    #[test]
    fn debug_local_supporting_block_projects_legacy_effect_and_fence_exception() {
        let mut piston_world = empty_world();
        let piston_head = vanilla_block_state_id(
            "minecraft:piston_head",
            [("facing", "down"), ("short", "false"), ("type", "normal")],
        );
        set_block(
            &mut piston_world,
            BlockPos { x: 0, y: 0, z: 0 },
            piston_head,
        );
        piston_world.set_local_player_pose(LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: 0.5,
                y: 1.25,
                z: 0.5,
            },
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let piston_debug = piston_world
            .debug_local_player_supporting_blocks()
            .expect("the upward-extending rod should support the player");
        assert_eq!(piston_debug.supporting.pos, BlockPos { x: 0, y: 0, z: 0 });
        assert!(!piston_debug.supporting.collision_boxes.is_empty());
        assert_eq!(
            piston_debug
                .legacy_effect
                .expect("legacy on-pos should use the floored entity position")
                .pos,
            BlockPos { x: 0, y: 1, z: 0 }
        );

        let mut fence_world = empty_world();
        let fence = vanilla_block_state_id(
            "minecraft:oak_fence",
            [
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("waterlogged", "false"),
                ("west", "false"),
            ],
        );
        set_block(&mut fence_world, BlockPos { x: 0, y: 0, z: 0 }, fence);
        fence_world.set_local_player_pose(LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: 0.5,
                y: 1.5,
                z: 0.5,
            },
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });
        let fence_debug = fence_world
            .debug_local_player_supporting_blocks()
            .expect("fence should support the player at height 1.5");
        assert_eq!(fence_debug.supporting.pos, BlockPos { x: 0, y: 0, z: 0 });
        assert_eq!(fence_debug.legacy_effect, None);
    }

    #[test]
    fn debug_supporting_highlight_uses_vanilla_empty_collision_context() {
        let mut world = empty_world();
        let powder_snow = vanilla_block_state_id("minecraft:powder_snow", []);
        let scaffolding = vanilla_block_state_id(
            "minecraft:scaffolding",
            [
                ("bottom", "false"),
                ("distance", "0"),
                ("waterlogged", "false"),
            ],
        );
        let powder_pos = BlockPos { x: 0, y: 0, z: 0 };
        let scaffolding_pos = BlockPos { x: 1, y: 0, z: 0 };
        set_block(&mut world, powder_pos, powder_snow);
        set_block(&mut world, scaffolding_pos, scaffolding);

        assert!(block_collision_shape_empty_context(
            &world.probe_block(powder_pos).unwrap(),
            powder_pos,
        )
        .is_none());
        assert!(block_collision_shape_empty_context(
            &world.probe_block(scaffolding_pos).unwrap(),
            scaffolding_pos,
        )
        .is_some());
    }

    #[test]
    fn remote_guardian_stand_probe_requires_a_loaded_full_up_face() {
        let mut world = empty_world();
        let stone_pos = BlockPos { x: 0, y: 0, z: 0 };
        let top_slab_pos = BlockPos { x: 1, y: 0, z: 0 };
        let bottom_slab_pos = BlockPos { x: 2, y: 0, z: 0 };
        set_block(
            &mut world,
            stone_pos,
            vanilla_block_state_id("minecraft:stone", []),
        );
        set_block(
            &mut world,
            top_slab_pos,
            vanilla_block_state_id(
                "minecraft:stone_slab",
                [("type", "top"), ("waterlogged", "false")],
            ),
        );
        set_block(
            &mut world,
            bottom_slab_pos,
            vanilla_block_state_id(
                "minecraft:stone_slab",
                [("type", "bottom"), ("waterlogged", "false")],
            ),
        );

        assert!(block_collision_shape_has_full_up_face(
            &world.probe_block(stone_pos).unwrap(),
            stone_pos
        ));
        assert!(block_collision_shape_has_full_up_face(
            &world.probe_block(top_slab_pos).unwrap(),
            top_slab_pos
        ));
        assert!(!block_collision_shape_has_full_up_face(
            &world.probe_block(bottom_slab_pos).unwrap(),
            bottom_slab_pos
        ));
        assert!(world.probe_block(BlockPos { x: 16, y: 0, z: 0 }).is_none());
    }

    #[test]
    fn remote_supporting_block_state_matches_packet_set_on_ground_lifecycle() {
        let mut world = empty_world();
        let entity_id = 41;
        add_collision_test_entity(
            &mut world,
            entity_id,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            [0.5, 1.0, 0.5],
        );
        let stone = vanilla_block_state_id("minecraft:stone", []);
        let air = vanilla_block_state_id("minecraft:air", []);
        let support_pos = BlockPos { x: 0, y: 0, z: 0 };
        set_block(&mut world, support_pos, stone);

        assert!(
            world.apply_entity_position_sync(bbb_protocol::packets::EntityPositionSync {
                id: entity_id,
                position: bbb_protocol::packets::Vec3d {
                    x: 0.5,
                    y: 1.0,
                    z: 0.5,
                },
                delta_movement: bbb_protocol::packets::Vec3d::default(),
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: true,
            })
        );
        let supported = world.probe_entity(entity_id).unwrap();
        assert_eq!(supported.main_supporting_block_pos, Some(support_pos));
        assert!(!supported.on_ground_no_blocks);
        assert_eq!(
            world.entity_main_supporting_block_pos(entity_id),
            Some(support_pos)
        );
        assert_eq!(
            world.debug_entity_supporting_blocks(entity_id),
            Some(DebugEntitySupportingBlocks {
                entity_id,
                supporting: DebugSupportingBlockHighlight {
                    pos: support_pos,
                    collision_boxes: vec![WorldCollisionBox {
                        min: [0.0, 0.0, 0.0],
                        max: [1.0, 1.0, 1.0],
                    }],
                },
                legacy_effect: None,
            })
        );

        let cloned = world.clone();
        assert_eq!(
            cloned
                .probe_entity(entity_id)
                .unwrap()
                .main_supporting_block_pos,
            Some(support_pos)
        );
        let serialized = serde_json::to_vec(&world).unwrap();
        let decoded: WorldStore = serde_json::from_slice(&serialized).unwrap();
        assert_eq!(
            decoded
                .probe_entity(entity_id)
                .unwrap()
                .main_supporting_block_pos,
            Some(support_pos)
        );

        set_block(&mut world, support_pos, air);
        assert!(world.apply_entity_move(bbb_protocol::packets::EntityMove {
            id: entity_id,
            has_position: false,
            delta_x: 0,
            delta_y: 0,
            delta_z: 0,
            y_rot: None,
            x_rot: None,
            on_ground: true,
        }));
        let first_missing = world.probe_entity(entity_id).unwrap();
        assert_eq!(first_missing.main_supporting_block_pos, Some(support_pos));
        assert!(first_missing.on_ground_no_blocks);

        assert!(
            world.apply_teleport_entity(bbb_protocol::packets::TeleportEntity {
                id: entity_id,
                position: bbb_protocol::packets::Vec3d {
                    x: 0.5,
                    y: 1.0,
                    z: 0.5,
                },
                delta_movement: bbb_protocol::packets::Vec3d::default(),
                y_rot: 0.0,
                x_rot: 0.0,
                relatives_mask: 0,
                on_ground: true,
            })
        );
        let second_missing = world.probe_entity(entity_id).unwrap();
        assert_eq!(second_missing.main_supporting_block_pos, None);
        assert!(second_missing.on_ground_no_blocks);
        assert_eq!(world.debug_entity_supporting_blocks(entity_id), None);

        assert!(world.apply_entity_move(bbb_protocol::packets::EntityMove {
            id: entity_id,
            has_position: false,
            delta_x: 0,
            delta_y: 0,
            delta_z: 0,
            y_rot: None,
            x_rot: None,
            on_ground: false,
        }));
        let airborne = world.probe_entity(entity_id).unwrap();
        assert_eq!(airborne.main_supporting_block_pos, None);
        assert!(!airborne.on_ground_no_blocks);
    }

    #[test]
    fn remote_powder_snow_walkability_matches_generated_entity_type_tag() {
        let mut world = empty_world();
        for entry in bbb_protocol::entity_types::VANILLA_ENTITY_TYPES {
            let entity_id = 1_000 + entry.id;
            add_collision_test_entity(&mut world, entity_id, entry.id, [0.0; 3]);
            assert_eq!(
                world.remote_entity_can_walk_on_powder_snow(entity_id),
                entry.powder_snow_walkable,
                "entity type {}",
                entry.resource_id
            );
        }
    }

    #[test]
    fn remote_supporting_block_context_uses_type_boots_descending_and_falling_block() {
        const BOOTS_ITEM_ID: i32 = 43;

        let mut world = empty_world();
        world.set_powder_snow_walkable_foot_item_ids(std::collections::BTreeSet::from([
            BOOTS_ITEM_ID,
        ]));
        let powder_snow = vanilla_block_state_id("minecraft:powder_snow", []);
        let tagged_entity_type_id = bbb_protocol::entity_types::VANILLA_ENTITY_TYPES
            .iter()
            .find(|entry| entry.powder_snow_walkable)
            .map(|entry| entry.id)
            .expect("generated powder-snow-walkable entity type");
        let cases = [
            (51, tagged_entity_type_id, 0, true),
            (
                52,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
                2,
                true,
            ),
            (
                53,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
                4,
                false,
            ),
            (54, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, 6, true),
            (
                55,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
                8,
                false,
            ),
        ];
        for (entity_id, entity_type_id, x, _) in cases {
            set_block(&mut world, BlockPos { x, y: 0, z: 0 }, powder_snow);
            add_collision_test_entity(
                &mut world,
                entity_id,
                entity_type_id,
                [f64::from(x) + 0.5, 1.0, 0.5],
            );
        }
        for entity_id in [52, 53] {
            assert!(
                world.apply_set_equipment(bbb_protocol::packets::SetEquipment {
                    entity_id,
                    slots: vec![bbb_protocol::packets::EquipmentSlotUpdate {
                        slot: ProtocolEquipmentSlot::Feet,
                        item: bbb_protocol::packets::ItemStackSummary {
                            item_id: Some(BOOTS_ITEM_ID),
                            count: 1,
                            component_patch: Default::default(),
                        },
                    }],
                })
            );
        }
        assert!(
            world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
                id: 53,
                values: vec![entity_shift_key_down_data()],
            })
        );

        for (entity_id, _, x, expected_support) in cases {
            assert!(
                world.apply_entity_position_sync(bbb_protocol::packets::EntityPositionSync {
                    id: entity_id,
                    position: bbb_protocol::packets::Vec3d {
                        x: f64::from(x) + 0.5,
                        y: 1.0,
                        z: 0.5,
                    },
                    delta_movement: bbb_protocol::packets::Vec3d::default(),
                    y_rot: 0.0,
                    x_rot: 0.0,
                    on_ground: true,
                })
            );
            assert_eq!(
                world.entity_main_supporting_block_pos(entity_id),
                expected_support.then_some(BlockPos { x, y: 0, z: 0 }),
                "entity {entity_id} powder-snow context"
            );
        }
    }

    #[test]
    fn remote_packet_fall_delta_does_not_create_powder_snow_falling_shape() {
        let mut world = empty_world();
        let entity_id = 56;
        let powder_snow = vanilla_block_state_id("minecraft:powder_snow", []);
        set_block(&mut world, BlockPos { x: 10, y: 0, z: 0 }, powder_snow);
        add_collision_test_entity(
            &mut world,
            entity_id,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            [10.5, 5.0, 0.5],
        );

        // Vanilla remote packet handlers call moveOrInterpolateTo/setOnGround;
        // neither packet displacement nor synced velocity runs checkFallDamage
        // on a server-authoritative client entity, so fallDistance stays zero.
        assert!(world.apply_entity_move(bbb_protocol::packets::EntityMove {
            id: entity_id,
            has_position: true,
            delta_x: 0,
            delta_y: -16_384,
            delta_z: 0,
            y_rot: None,
            x_rot: None,
            on_ground: true,
        }));
        world.advance_entity_client_animations(3);
        assert_eq!(
            world.probe_entity(entity_id).unwrap().position.y,
            1.0,
            "the packet moves the entity downward by four blocks"
        );
        assert_eq!(world.entity_main_supporting_block_pos(entity_id), None);

        let velocity_entity_id = 57;
        set_block(&mut world, BlockPos { x: 12, y: 0, z: 0 }, powder_snow);
        add_collision_test_entity(
            &mut world,
            velocity_entity_id,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            [12.5, 1.0, 0.5],
        );
        assert!(
            world.apply_entity_position_sync(bbb_protocol::packets::EntityPositionSync {
                id: velocity_entity_id,
                position: bbb_protocol::packets::Vec3d {
                    x: 12.5,
                    y: 1.0,
                    z: 0.5,
                },
                delta_movement: bbb_protocol::packets::Vec3d {
                    x: 0.0,
                    y: -4.0,
                    z: 0.0,
                },
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: true,
            })
        );
        assert_eq!(
            world
                .probe_entity(velocity_entity_id)
                .unwrap()
                .delta_movement
                .y,
            0.0,
            "ClientPacketListener position sync does not apply packet movement"
        );
        assert_eq!(
            world.entity_main_supporting_block_pos(velocity_entity_id),
            None
        );
    }

    #[test]
    fn collision_debug_collidability_matches_generated_class_dispatch() {
        let mut world = empty_world();
        world.set_local_player_pose(LocalPlayerPoseState::default());
        let camera = world
            .debug_camera_collision_query()
            .expect("local-player collision query");
        let target_box = WorldCollisionBox {
            min: [0.0; 3],
            max: [1.0; 3],
        };
        for entry in bbb_protocol::entity_types::VANILLA_ENTITY_TYPES {
            let entity_id = 1_000 + entry.id;
            add_collision_test_entity(&mut world, entity_id, entry.id, [0.5, 0.0, 0.5]);
            let expected = matches!(
                vanilla_entity_metadata_type_collidability_kind(entry.id),
                Some(
                    VanillaEntityCollidabilityKind::Always | VanillaEntityCollidabilityKind::Alive
                )
            );
            assert_eq!(
                world.debug_entity_can_be_collided_with(camera, entity_id, entry.id, target_box,),
                expected,
                "entity type {}",
                entry.resource_id
            );
        }
        assert!(!world.debug_entity_can_be_collided_with(camera, i32::MAX, -1, target_box,));
    }

    #[test]
    fn debug_camera_collision_boxes_prepend_collidable_entities_and_exclude_same_vehicle() {
        let mut world = empty_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: 0.5,
                y: 0.0,
                z: 0.5,
            },
            ..LocalPlayerPoseState::default()
        });
        world.local_player_id = Some(99);
        add_collision_test_entity(
            &mut world,
            99,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            [0.5, 0.0, 0.5],
        );
        add_collision_test_entity(
            &mut world,
            10,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            [2.5, 0.0, 0.5],
        );
        add_collision_test_entity(
            &mut world,
            11,
            VANILLA_ENTITY_TYPE_SHULKER_ID,
            [4.5, 0.0, 0.5],
        );
        add_collision_test_entity(
            &mut world,
            12,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            [5.5, 0.0, 0.5],
        );

        let expected = world
            .entity_pick_targets()
            .into_iter()
            .filter(|target| matches!(target.entity_id, 10 | 11))
            .map(world_collision_box_from_pick_target)
            .collect::<Vec<_>>();
        assert_eq!(world.debug_camera_collision_boxes(6.0), expected);

        assert!(
            world.apply_set_passengers(bbb_protocol::packets::SetPassengers {
                vehicle_id: 10,
                passenger_ids: vec![99],
            })
        );
        let boxes = world.debug_camera_collision_boxes(6.0);
        assert_eq!(boxes, expected[1..]);
    }

    #[test]
    fn happy_ghast_collision_requires_adult_alive_and_player_above_or_stays_still() {
        let mut world = empty_world();
        let accessors = vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        )
        .expect("generated HappyGhast collision accessors");
        let (baby_data_id, baby_default) =
            crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
            )
            .expect("generated HappyGhast baby accessor");
        assert!(!baby_default);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: 0.5,
                y: 0.0,
                z: 0.5,
            },
            ..LocalPlayerPoseState::default()
        });
        add_collision_test_entity(
            &mut world,
            20,
            VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
            [2.5, 0.0, 0.5],
        );
        assert!(world.debug_camera_collision_boxes(6.0).is_empty());

        world.set_local_player_pose(LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: 0.5,
                y: 4.0,
                z: 0.5,
            },
            ..LocalPlayerPoseState::default()
        });
        assert_eq!(world.debug_camera_collision_boxes(6.0).len(), 1);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: 0.5,
                y: 0.0,
                z: 0.5,
            },
            ..LocalPlayerPoseState::default()
        });

        assert!(
            world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
                id: 20,
                values: vec![bbb_protocol::packets::EntityDataValue {
                    data_id: accessors.is_leash_holder_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    value: bbb_protocol::packets::EntityDataValueKind::Boolean(true),
                }],
            })
        );
        assert!(world.debug_camera_collision_boxes(6.0).is_empty());

        assert!(
            world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
                id: 20,
                values: vec![bbb_protocol::packets::EntityDataValue {
                    data_id: accessors.stays_still_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    value: bbb_protocol::packets::EntityDataValueKind::Boolean(true),
                }],
            })
        );
        assert_eq!(world.debug_camera_collision_boxes(6.0).len(), 1);

        assert!(
            world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
                id: 20,
                values: vec![bbb_protocol::packets::EntityDataValue {
                    data_id: baby_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    value: bbb_protocol::packets::EntityDataValueKind::Boolean(true),
                }],
            })
        );
        assert!(world.debug_camera_collision_boxes(6.0).is_empty());
    }

    #[test]
    fn collision_debug_pushability_matches_generated_class_dispatch() {
        let mut world = empty_world();
        for entry in bbb_protocol::entity_types::VANILLA_ENTITY_TYPES {
            let entity_id = 1_000 + entry.id;
            add_collision_test_entity(&mut world, entity_id, entry.id, [0.5, 0.0, 0.5]);
            let expected = vanilla_entity_metadata_type_pushability_kind(entry.id)
                .is_some_and(|kind| kind != VanillaEntityPushabilityKind::Never);
            assert_eq!(
                world.debug_entity_is_pushable(entity_id, entry.id, [0.5, 0.0, 0.5]),
                expected,
                "entity type {}",
                entry.resource_id
            );
        }
        assert!(!world.debug_entity_is_pushable(i32::MAX, -1, [0.5, 0.0, 0.5]));
    }

    #[test]
    fn spectated_minecart_collision_expands_to_pushable_entities() {
        let mut world = empty_world();
        add_collision_test_entity(
            &mut world,
            30,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.5, 0.0, 0.5],
        );
        add_collision_test_entity(
            &mut world,
            31,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            [2.5, 0.0, 0.5],
        );
        add_collision_test_entity(
            &mut world,
            32,
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            [3.5, 0.0, 0.5],
        );
        add_collision_test_entity(&mut world, 33, VANILLA_ENTITY_TYPE_BAT_ID, [4.5, 0.0, 0.5]);
        add_collision_test_entity(
            &mut world,
            34,
            VANILLA_ENTITY_TYPE_PARROT_ID,
            [5.5, 0.0, 0.5],
        );
        add_collision_test_entity(
            &mut world,
            35,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID,
            [6.5, 0.0, 0.5],
        );
        world.local_player.camera = crate::CameraState {
            entity_id: Some(30),
            follows_player: false,
            entity_known: true,
        };

        let expected_ids = [31, 34, 35];
        let expected = world
            .entity_pick_targets()
            .into_iter()
            .filter(|target| expected_ids.contains(&target.entity_id))
            .map(world_collision_box_from_pick_target)
            .collect::<Vec<_>>();
        assert_eq!(world.debug_camera_collision_boxes(6.0), expected);

        assert!(
            world.apply_set_passengers(bbb_protocol::packets::SetPassengers {
                vehicle_id: 35,
                passenger_ids: vec![31],
            })
        );
        let horse_box = world
            .entity_pick_targets()
            .into_iter()
            .find(|target| target.entity_id == 35)
            .map(world_collision_box_from_pick_target)
            .expect("horse bounds");
        assert!(!world.debug_camera_collision_boxes(6.0).contains(&horse_box));
    }

    #[test]
    fn turtle_collision_uses_living_pushability_and_never_direct_collidability() {
        let turtle_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID;
        assert_eq!(
            vanilla_entity_metadata_type_pushability_kind(turtle_type),
            Some(VanillaEntityPushabilityKind::Living)
        );
        assert_eq!(
            vanilla_entity_metadata_type_collidability_kind(turtle_type),
            Some(VanillaEntityCollidabilityKind::Never)
        );

        let mut world = empty_world();
        add_collision_test_entity(&mut world, 39, turtle_type, [0.5, 0.0, 0.5]);
        assert!(world.debug_entity_is_pushable(39, turtle_type, [0.5, 0.0, 0.5]));

        let ladder = vanilla_block_state_id(
            "minecraft:ladder",
            [("facing", "north"), ("waterlogged", "false")],
        );
        set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, ladder);
        assert!(!world.debug_entity_is_pushable(39, turtle_type, [0.5, 0.0, 0.5]));
    }

    #[test]
    fn living_pushability_rejects_entities_on_climbable_blocks() {
        let mut world = empty_world();
        add_collision_test_entity(
            &mut world,
            40,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            [0.5, 0.0, 0.5],
        );
        assert!(world.debug_entity_is_pushable(
            40,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            [0.5, 0.0, 0.5],
        ));
        let ladder = vanilla_block_state_id(
            "minecraft:ladder",
            [("facing", "north"), ("waterlogged", "false")],
        );
        set_block(&mut world, BlockPos { x: 0, y: 0, z: 0 }, ladder);
        assert!(!world.debug_entity_is_pushable(
            40,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            [0.5, 0.0, 0.5],
        ));
    }

    #[test]
    fn generated_pushability_kinds_preserve_warden_and_creaking_instance_gates() {
        let mut world = empty_world();
        let (creaking_can_move_data_id, creaking_can_move_default) =
            vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_CREAKING_ID,
            )
            .expect("generated Creaking can-move accessor");
        assert!(creaking_can_move_default);
        add_collision_test_entity(
            &mut world,
            41,
            VANILLA_ENTITY_TYPE_WARDEN_ID,
            [0.5, 0.0, 0.5],
        );
        add_collision_test_entity(
            &mut world,
            42,
            VANILLA_ENTITY_TYPE_CREAKING_ID,
            [1.5, 0.0, 0.5],
        );
        assert!(world.debug_entity_is_pushable(41, VANILLA_ENTITY_TYPE_WARDEN_ID, [0.5, 0.0, 0.5],));
        assert!(world.debug_entity_is_pushable(
            42,
            VANILLA_ENTITY_TYPE_CREAKING_ID,
            [1.5, 0.0, 0.5],
        ));

        assert!(
            world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
                id: 41,
                values: vec![entity_pose_data(VANILLA_ENTITY_POSE_DIGGING_ID)],
            })
        );
        assert!(!world.debug_entity_is_pushable(
            41,
            VANILLA_ENTITY_TYPE_WARDEN_ID,
            [0.5, 0.0, 0.5],
        ));

        assert!(
            world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
                id: 42,
                values: vec![bbb_protocol::packets::EntityDataValue {
                    data_id: creaking_can_move_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    value: bbb_protocol::packets::EntityDataValueKind::Boolean(false),
                }],
            })
        );
        assert!(!world.debug_entity_is_pushable(
            42,
            VANILLA_ENTITY_TYPE_CREAKING_ID,
            [1.5, 0.0, 0.5],
        ));
    }

    #[test]
    fn collision_shape_to_aabbs_coalesces_overlapping_union_cells_like_vanilla() {
        let shape = BlockCollisionShape::from_boxes([
            Some(BlockCollisionBox {
                min_x: 0.0,
                min_y: 0.0,
                min_z: 0.0,
                max_x: 0.75,
                max_y: 1.0,
                max_z: 1.0,
            }),
            Some(BlockCollisionBox {
                min_x: 0.25,
                min_y: 0.0,
                min_z: 0.0,
                max_x: 1.0,
                max_y: 1.0,
                max_z: 1.0,
            }),
        ]);
        assert_eq!(shape.to_aabbs(), [BlockCollisionBox::FULL]);
    }

    #[test]
    fn collision_shape_to_aabbs_preserves_non_rectangular_union_as_merged_boxes() {
        let shape = BlockCollisionShape::from_boxes([
            Some(BlockCollisionBox {
                min_x: 0.0,
                min_y: 0.0,
                min_z: 0.0,
                max_x: 0.5,
                max_y: 1.0,
                max_z: 1.0,
            }),
            Some(BlockCollisionBox {
                min_x: 0.5,
                min_y: 0.0,
                min_z: 0.5,
                max_x: 1.0,
                max_y: 1.0,
                max_z: 1.0,
            }),
        ]);
        assert_eq!(
            shape.to_aabbs(),
            [
                BlockCollisionBox {
                    min_x: 0.0,
                    min_y: 0.0,
                    min_z: 0.0,
                    max_x: 0.5,
                    max_y: 1.0,
                    max_z: 1.0,
                },
                BlockCollisionBox {
                    min_x: 0.5,
                    min_y: 0.0,
                    min_z: 0.5,
                    max_x: 1.0,
                    max_y: 1.0,
                    max_z: 1.0,
                },
            ]
        );
    }

    #[test]
    fn debug_solid_faces_use_full_support_shapes_and_vanilla_overrides() {
        let mut world = empty_world();
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let cases = [
            (vanilla_block_state_id("minecraft:stone", []), [true; 6]),
            (
                vanilla_block_state_id(
                    "minecraft:oak_slab",
                    [("type", "bottom"), ("waterlogged", "false")],
                ),
                [false, false, false, false, true, false],
            ),
            (
                vanilla_block_state_id("minecraft:snow", [("layers", "1")]),
                [false, false, false, false, true, false],
            ),
            (
                vanilla_block_state_id("minecraft:snow", [("layers", "8")]),
                [true; 6],
            ),
            (vanilla_block_state_id("minecraft:soul_sand", []), [true; 6]),
            (
                vanilla_block_state_id(
                    "minecraft:oak_leaves",
                    [
                        ("distance", "7"),
                        ("persistent", "false"),
                        ("waterlogged", "false"),
                    ],
                ),
                [false; 6],
            ),
            (
                vanilla_block_state_id(
                    "minecraft:oak_fence_gate",
                    [
                        ("facing", "north"),
                        ("in_wall", "false"),
                        ("open", "false"),
                        ("powered", "false"),
                    ],
                ),
                [false; 6],
            ),
            (
                vanilla_block_state_id(
                    "minecraft:scaffolding",
                    [
                        ("bottom", "false"),
                        ("distance", "0"),
                        ("waterlogged", "false"),
                    ],
                ),
                [false, false, false, false, false, true],
            ),
        ];

        for (block_state_id, expected) in cases {
            set_block(&mut world, pos, block_state_id);
            let probe = world.probe_block(pos).expect("test block probe");
            assert_eq!(world.debug_block_sturdy_faces(&probe), expected);
            for (direction, expected) in [
                ProtocolDirection::Down,
                ProtocolDirection::Up,
                ProtocolDirection::North,
                ProtocolDirection::South,
                ProtocolDirection::West,
                ProtocolDirection::East,
            ]
            .into_iter()
            .zip([
                expected[4],
                expected[5],
                expected[3],
                expected[1],
                expected[0],
                expected[2],
            ]) {
                assert_eq!(
                    world.block_full_face_sturdy(&probe, direction),
                    Some(expected),
                    "{block_state_id} {direction:?}"
                );
            }
        }
    }

    #[test]
    fn debug_solid_faces_track_open_shulker_support_direction() {
        let mut world = empty_world();
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let shulker = vanilla_block_state_id("minecraft:shulker_box", [("facing", "east")]);
        set_block(&mut world, pos, shulker);
        let probe = world.probe_block(pos).expect("closed shulker probe");
        assert_eq!(world.debug_block_sturdy_faces(&probe), [true; 6]);

        world.apply_block_event(bbb_protocol::packets::BlockEvent {
            pos: bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 },
            b0: 1,
            b1: 1,
            block_id: 0,
        });
        let probe = world.probe_block(pos).expect("opening shulker probe");
        assert_eq!(
            world.debug_block_sturdy_faces(&probe),
            [true, false, false, false, false, false]
        );
        assert_eq!(
            world.block_full_face_sturdy(&probe, ProtocolDirection::West),
            Some(true)
        );
        assert_eq!(
            world.block_full_face_sturdy(&probe, ProtocolDirection::East),
            Some(false)
        );
    }

    #[test]
    fn debug_scaffolding_outline_boxes_reuse_vanilla_voxel_union_merge() {
        let mut world = empty_world();
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let stable = vanilla_block_state_id(
            "minecraft:scaffolding",
            [
                ("bottom", "false"),
                ("distance", "0"),
                ("waterlogged", "false"),
            ],
        );
        set_block(&mut world, pos, stable);
        let probe = world.probe_block(pos).expect("stable scaffolding probe");
        let boxes = world
            .debug_scaffolding_outline_shape_boxes(&probe)
            .expect("stable scaffolding outline boxes");
        assert_eq!(boxes.len(), 7);
        assert_eq!(boxes[0], ([0.0, 0.0, 0.0], [0.125, 1.0, 0.125]));
        assert_eq!(boxes[4], ([0.0, 0.875, 0.125], [1.0, 1.0, 0.875]));

        let unstable = vanilla_block_state_id(
            "minecraft:scaffolding",
            [
                ("bottom", "true"),
                ("distance", "1"),
                ("waterlogged", "false"),
            ],
        );
        set_block(&mut world, pos, unstable);
        let probe = world.probe_block(pos).expect("bottom scaffolding probe");
        let boxes = world
            .debug_scaffolding_outline_shape_boxes(&probe)
            .expect("bottom scaffolding outline boxes");
        assert_eq!(boxes.len(), 12);
        assert_eq!(boxes[0], ([0.0, 0.0, 0.0], [1.0, 0.125, 1.0]));
        assert_eq!(boxes[5], ([0.0, 0.4375, 0.125], [0.125, 0.5625, 0.875]));
    }

    #[test]
    fn connected_fence_to_aabbs_removes_post_arm_overlap() {
        let properties = BTreeMap::from([
            ("north".to_owned(), "true".to_owned()),
            ("east".to_owned(), "true".to_owned()),
            ("south".to_owned(), "false".to_owned()),
            ("west".to_owned(), "false".to_owned()),
        ]);
        let boxes = fence_collision_shape(&properties)
            .expect("connected fence shape")
            .to_aabbs();
        assert_eq!(
            boxes,
            [
                BlockCollisionBox {
                    min_x: 6.0 * PX,
                    min_y: 0.0,
                    min_z: 0.0,
                    max_x: 10.0 * PX,
                    max_y: 1.5,
                    max_z: 10.0 * PX,
                },
                BlockCollisionBox {
                    min_x: 10.0 * PX,
                    min_y: 0.0,
                    min_z: 6.0 * PX,
                    max_x: 1.0,
                    max_y: 1.5,
                    max_z: 10.0 * PX,
                },
            ]
        );
    }

    fn axis_name(axis: CollisionAxis) -> &'static str {
        match axis {
            CollisionAxis::X => "x",
            CollisionAxis::Y => "y",
            CollisionAxis::Z => "z",
        }
    }

    fn add_collision_test_entity(
        world: &mut WorldStore,
        id: i32,
        entity_type_id: i32,
        position: [f64; 3],
    ) {
        world.apply_add_entity(bbb_protocol::packets::AddEntity {
            id,
            uuid: uuid::Uuid::from_u128(id as u128 + 1),
            entity_type_id,
            position: bbb_protocol::packets::Vec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            delta_movement: bbb_protocol::packets::Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
    }

    fn world_collision_box_from_pick_target(
        target: crate::EntityPickTargetState,
    ) -> WorldCollisionBox {
        WorldCollisionBox {
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
        }
    }

    fn empty_world() -> WorldStore {
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
                block_states: single_value_container(PaletteDomain::BlockStates, 4096, 0),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        world
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

    fn set_block(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: bbb_protocol::packets::BlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_state_id,
            })
        );
    }

    fn vanilla_block_state_id<const N: usize>(name: &str, props: [(&str, &str); N]) -> i32 {
        crate::registries::BlockStateRegistry::vanilla_26_1()
            .find_by_name_and_properties(name, &string_props(props))
            .unwrap_or_else(|| panic!("vanilla 26.1 block state exists for {name}"))
            .id
    }

    fn string_props<const N: usize>(entries: [(&str, &str); N]) -> BTreeMap<String, String> {
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    fn assert_f64_near(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1.0e-12,
            "actual {actual} expected {expected}",
        );
    }
}
