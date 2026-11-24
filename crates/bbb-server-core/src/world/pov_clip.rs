//! `BlockGetter.clip` (`BlockGetter.java:63-77`) over `traverseBlocks`
//! (`:103-171`), the ray `Item.getPlayerPOVHitResult` (`Item.java:349-353`)
//! hands it for an item used at nothing.

use crate::chunk::ChunkStore;
use crate::connection::ConnectionId;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::fluid::{block_state_fluid_state, FluidState};
use bbb_server_types::geometry::clip::clip_with;
use bbb_server_types::geometry::clip::ClipHit;
use bbb_server_types::geometry::{mth_cos, mth_sin, Aabb, Vec3Ops};

use super::level_blocks::stored_block_state;
use super::ServerWorld;

/// `ClipContext.Fluid` (`ClipContext.java:96-99`), the three modes this server
/// reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ClipFluid {
    /// `ClipContext.Fluid.NONE` (`:97`): the ray stops on blocks only.
    None,
    /// `ClipContext.Fluid.SOURCE_ONLY` (`:98`): a fluid contributes its
    /// shape only where it is a source, which is what a spawn egg's and a
    /// bucket's use looks through.
    SourceOnly,
    /// `ClipContext.Fluid.ANY` (`:99`): every fluid contributes its shape,
    /// which is what a boat's use looks through.
    Any,
}

/// `ClipContext.Block` (`ClipContext.java:56-58`), the three shape getters
/// this server reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ClipShape {
    /// `ClipContext.Block.OUTLINE` (`:57`): `BlockState.getShape`.
    Outline,
    /// `ClipContext.Block.VISUAL` (`:58`): `BlockBehaviour.getVisualShape`,
    /// which is the collision shape unless a block overrides it
    /// (`BlockBehaviour.java:345-347`).
    Visual,
    /// `ClipContext.Block.COLLIDER` (`:56`): `BlockState.getCollisionShape`.
    Collider,
}

/// `Item.getPlayerPOVHitResult` (`Item.java:349-353`): the eye position, the
/// view vector scaled by `blockInteractionRange`, and
/// `ClipContext.Block.OUTLINE` with the caller's fluid mode.
pub(super) fn player_pov_clip(
    world: &ServerWorld,
    connection_id: ConnectionId,
    fluid: ClipFluid,
) -> Option<ClipHit> {
    let player = world.levels.player(connection_id)?;
    let from = player.eye_position().ok()?;
    let range = player.block_interaction_range();
    let view = view_vector(player.rotation().x_rot, player.rotation().y_rot);
    let to = from.plus(view.scale(range));
    clip_player(
        &world.level_by_id(&player.dimension).chunks,
        player,
        from,
        to,
        fluid,
        ClipShape::Outline,
    )
}

/// `PlayerDetector.inLineOfSight` (`PlayerDetector.java:55-58`): the ray runs
/// from `dest` back to `origin` with `ClipContext.Block.VISUAL`, and a hit
/// inside `origin`'s own cell is the block the ray starts against.
pub(super) fn in_line_of_sight(chunks: &ChunkStore, origin: Vec3d, dest: Vec3d) -> bool {
    let hit = clip_exact(chunks, dest, origin, ClipFluid::None, ClipShape::Visual);
    match hit {
        None => false,
        Some(None) => true,
        Some(Some(hit)) => {
            hit.pos
                == BlockPos {
                    x: origin.x.floor() as i32,
                    y: origin.y.floor() as i32,
                    z: origin.z.floor() as i32,
                }
        }
    }
}

/// `BlockGetter.clip` (`BlockGetter.java:63-77`): the first cell the ray
/// enters whose block or fluid shape it clips, and inside that cell the shape
/// whose hit is nearer to the start.
pub(super) fn clip_player(
    chunks: &ChunkStore,
    player: &super::PlayerState,
    from: Vec3d,
    to: Vec3d,
    fluid: ClipFluid,
    shape: ClipShape,
) -> Option<ClipHit> {
    clip_in_context(
        chunks,
        from,
        to,
        fluid,
        shape,
        Some(player),
        player.inventory.selected_stack().item_id(),
    )
    .flatten()
}

pub(super) fn clip_exact(
    chunks: &ChunkStore,
    from: Vec3d,
    to: Vec3d,
    fluid: ClipFluid,
    shape: ClipShape,
) -> Option<Option<ClipHit>> {
    clip_in_context(chunks, from, to, fluid, shape, None, None)
}

pub(super) fn clip_entity_outline(
    chunks: &ChunkStore,
    entity: &crate::entity::Entity,
    from: Vec3d,
    to: Vec3d,
) -> Option<Option<ClipHit>> {
    let held_item = entity
        .living()
        .and_then(|living| {
            living.item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
        })
        .and_then(|stack| stack.item_id());
    clip_in_context(
        chunks,
        from,
        to,
        ClipFluid::None,
        ClipShape::Outline,
        None,
        held_item,
    )
}

fn clip_in_context(
    chunks: &ChunkStore,
    from: Vec3d,
    to: Vec3d,
    fluid: ClipFluid,
    shape: ClipShape,
    player: Option<&super::PlayerState>,
    held_item: Option<i32>,
) -> Option<Option<ClipHit>> {
    let mut hit = clip_with(from, to, |pos| {
        let state = stored_block_state(chunks, pos);
        let block = block_shape(chunks, state, pos, shape, player, held_item)?;
        let fluid = fluid_shape(chunks, pos, block_state_fluid_state(state), fluid)?;
        Some((block, fluid))
    })?;
    if let Some(result) = hit.as_mut() {
        let state = stored_block_state(chunks, result.pos);
        if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
            == Some(bbb_protocol::block_types::VANILLA_BLOCK_SCAFFOLDING_ID)
        {
            let boxes = block_shape(chunks, state, result.pos, shape, player, held_item)?;
            let block = bbb_server_types::geometry::clip::shape_clip(&boxes, from, to, result.pos);
            // BlockGetter.java:82-93; ScaffoldingBlock.java:67-69: only block-hit direction changes.
            if block.is_some_and(|block| block.location == result.location) {
                if let Some(interaction) = bbb_server_types::geometry::clip::shape_clip(
                    &full_block(),
                    from,
                    to,
                    result.pos,
                ) {
                    if from.distance_to_sqr(interaction.location)
                        < from.distance_to_sqr(result.location)
                    {
                        result.direction = interaction.direction;
                    }
                }
            }
        }
    }
    Some(hit)
}

fn block_shape(
    chunks: &ChunkStore,
    state: i32,
    pos: BlockPos,
    shape: ClipShape,
    player: Option<&super::PlayerState>,
    held_item: Option<i32>,
) -> Option<Vec<Aabb>> {
    use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
    use bbb_protocol::block_types::{
        VANILLA_BLOCK_MOVING_PISTON_ID, VANILLA_BLOCK_POWDER_SNOW_ID, VANILLA_BLOCK_SCAFFOLDING_ID,
    };
    let Some(block) = vanilla_block_state_block_type_id(state) else {
        let (spec, _) = chunks.content_registry().block_by_state_id(state)?;
        return Some(
            if spec.full_cube_collision || matches!(shape, ClipShape::Outline) {
                full_block()
            } else {
                Vec::new()
            },
        );
    };
    match shape {
        ClipShape::Collider => player.map_or_else(
            || super::movement_collision::block_collision_shape_checked(chunks, pos, None),
            |player| {
                super::movement_collision::player_block_collision_shape_checked(chunks, player, pos)
            },
        ),
        ClipShape::Outline if block == VANILLA_BLOCK_SCAFFOLDING_ID => {
            bbb_server_types::geometry::scaffolding::outline_boxes(
                state,
                held_item == Some(bbb_protocol::item_types::VANILLA_ITEM_SCAFFOLDING_ID),
            )
        }
        ClipShape::Visual if block == VANILLA_BLOCK_SCAFFOLDING_ID => player.map_or_else(
            || super::movement_collision::block_collision_shape_checked(chunks, pos, None),
            |player| {
                super::movement_collision::player_block_collision_shape_checked(chunks, player, pos)
            },
        ),
        ClipShape::Outline if block == VANILLA_BLOCK_MOVING_PISTON_ID => Some(Vec::new()),
        ClipShape::Outline if block == VANILLA_BLOCK_POWDER_SNOW_ID => Some(full_block()),
        ClipShape::Visual if block == VANILLA_BLOCK_POWDER_SNOW_ID => Some(Vec::new()),
        ClipShape::Outline | ClipShape::Visual
            if bbb_server_types::block_queries::is_shulker_box(state) =>
        {
            super::movement_collision::block_collision_shape_checked(chunks, pos, None)
        }
        ClipShape::Visual if block == VANILLA_BLOCK_MOVING_PISTON_ID => {
            super::movement_collision::block_collision_shape_checked(chunks, pos, None)
        }
        ClipShape::Outline => {
            bbb_server_types::geometry::positioned_shapes::outline_boxes(state, pos)
        }
        ClipShape::Visual => {
            bbb_server_types::geometry::positioned_shapes::visual_boxes(state, pos)
        }
    }
}

/// `ClipContext.Fluid.getFluidShape` (`ClipContext.java:52-54`) over
/// `FlowingFluid.getShape` (`FlowingFluid.java:484-488`): a source fluid is a
/// box `0..1` in x and z whose height is `FluidState.getHeight`
/// (`FlowingFluid.java:471-473`), one full block when the same fluid sits
/// above. `FlowingFluid.getShape`'s `getAmount() == 9` shortcut is not
/// transcribed: `FlowingFluid.Source.getAmount` is 8 for both water and lava
/// (`WaterFluid.java:151-153`), so it never takes that branch.
fn fluid_shape(
    chunks: &ChunkStore,
    pos: BlockPos,
    state: FluidState,
    fluid: ClipFluid,
) -> Option<Option<&Aabb>> {
    match fluid {
        ClipFluid::None => Some(None),
        ClipFluid::SourceOnly => source_fluid_shape(chunks, pos, state),
        ClipFluid::Any => chunks.fluid_shape_cache().shape_with_registry(
            state,
            pos,
            chunks.content_registry(),
            |pos| Some(stored_block_state(chunks, pos)),
        ),
    }
}

pub(super) fn source_fluid_shape(
    chunks: &ChunkStore,
    pos: BlockPos,
    state: FluidState,
) -> Option<Option<&Aabb>> {
    if !state.is_source() {
        return Some(None);
    }
    chunks
        .fluid_shape_cache()
        .shape_with_registry(state, pos, chunks.content_registry(), |pos| {
            Some(stored_block_state(chunks, pos))
        })
}

/// `Entity.calculateViewVector` (`Entity.java:1872-1879`) over the `Mth`
/// sine table.
pub(in crate::world) fn view_vector(x_rot: f32, y_rot: f32) -> Vec3d {
    let to_radians = std::f32::consts::PI / 180.0;
    let real_x_rot = x_rot * to_radians;
    let real_y_rot = -y_rot * to_radians;
    let y_cos = mth_cos(f64::from(real_y_rot));
    let y_sin = mth_sin(f64::from(real_y_rot));
    let x_cos = mth_cos(f64::from(real_x_rot));
    let x_sin = mth_sin(f64::from(real_x_rot));
    Vec3d {
        x: f64::from(y_sin * x_cos),
        y: f64::from(-x_sin),
        z: f64::from(y_cos * x_cos),
    }
}

fn full_block() -> Vec<Aabb> {
    vec![Aabb {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    }]
}

#[cfg(test)]
mod contextual_tests;

/// ClipContext.java:59-81; ClipContext.java:96-100: player fall-resetting blocks and all tagged water.
pub(super) fn fall_resetting_clip(
    chunks: &ChunkStore,
    from: Vec3d,
    to: Vec3d,
    nether_portal_delay_zero: bool,
) -> Option<bool> {
    use crate::fall_clip::{fall_resetting_clip_with, FallResettingContext};
    let tags = chunks.registry_tags();
    fall_resetting_clip_with(
        from,
        to,
        FallResettingContext::Player {
            nether_portal_delay_zero,
        },
        chunks.fluid_shape_cache(),
        |pos| Some(stored_block_state(chunks, pos)),
        |registry, tag, resource| tags.is_some_and(|tags| tags.contains(registry, tag, resource)),
    )
}
