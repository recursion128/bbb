//!
//! Thirty-five blocks. A coral holds on to what it grew from -- the plants and
//! fans to a sturdy face below, the wall fans to the wall behind `FACING` --
//! and a living one starts a sixty-to-hundred-tick countdown the moment
//! `scanForWater` stops finding water beside it. The living subclasses run
//! their own body and then their parent's, so a waterlogged one re-arms its
//! water twice on one shape update, which is transcribed rather than folded.
//! The countdown converts each of the twenty living registrations to its
//! constructor's dead partner.
//!
//! # What is deliberately not here
//!
//! * `getStateForPlacement` for all four, which belongs to the placement seam.

use super::{BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::{
    self, vanilla_block_type, VanillaCanSurviveOverride, VanillaTickOverride,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{below, with_property, FlowingFluid};
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::FluidFamily;

use super::{
    block_type_of, bool_property, can_survive_override, direction_property, face_sturdy,
    face_sturdy_at, tick_override, BlockStateAt, DIRECTION_VALUES, SET_BLOCK_UPDATE_CLIENTS,
};

const WATERLOGGED: &str = "waterlogged";
pub(crate) const WATER_TAG: &str = "minecraft:water";

/// `HorizontalDirectionalBlock.FACING` as `BaseCoralWallFanBlock` re-exports
/// it (`BaseCoralWallFanBlock.java:24`).
const FACING: &str = "facing";

/// `tryScheduleDieTick`'s `60 + random.nextInt(40)`
/// (`BaseCoralPlantTypeBlock.java:39`).
pub const DIE_TICK_MIN_DELAY: i32 = 60;

/// The spread on top of it.
pub const DIE_TICK_DELAY_SPREAD: i32 = 40;

/// Whether the block's registration declares an `onPlace`, which is exactly
/// what separates the three living subclasses from the two dead bases.
fn living(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .is_some_and(|entry| entry.on_place_override.is_some())
}

fn declares_can_survive(block_state_id: i32, class: VanillaCanSurviveOverride) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override) == Some(class)
}

#[must_use]
pub fn is_dead_plant_type(block_state_id: i32) -> bool {
    declares_can_survive(
        block_state_id,
        VanillaCanSurviveOverride::BaseCoralPlantTypeBlock,
    ) && !living(block_state_id)
}

#[must_use]
pub fn is_living_plant_type(block_state_id: i32) -> bool {
    declares_can_survive(
        block_state_id,
        VanillaCanSurviveOverride::BaseCoralPlantTypeBlock,
    ) && living(block_state_id)
}

#[must_use]
pub fn is_dead_wall_fan(block_state_id: i32) -> bool {
    declares_can_survive(
        block_state_id,
        VanillaCanSurviveOverride::BaseCoralWallFanBlock,
    ) && !living(block_state_id)
}

#[must_use]
pub fn is_living_wall_fan(block_state_id: i32) -> bool {
    declares_can_survive(
        block_state_id,
        VanillaCanSurviveOverride::BaseCoralWallFanBlock,
    ) && living(block_state_id)
}

/// `CoralBlock`, which declares no `canSurvive` at all and is named by its
/// `tick` instead.
#[must_use]
pub fn is_coral_block(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(tick_override) == Some(VanillaTickOverride::CoralBlock)
}

#[derive(Clone, Copy)]
enum DeadStateKind {
    Solid,
    Waterlogged,
    WallFan,
}

fn dead_partner(block_type_id: i32) -> Option<(i32, DeadStateKind)> {
    use block_types::*;

    Some(match block_type_id {
        VANILLA_BLOCK_TUBE_CORAL_BLOCK_ID => {
            (VANILLA_BLOCK_DEAD_TUBE_CORAL_BLOCK_ID, DeadStateKind::Solid)
        }
        VANILLA_BLOCK_BRAIN_CORAL_BLOCK_ID => (
            VANILLA_BLOCK_DEAD_BRAIN_CORAL_BLOCK_ID,
            DeadStateKind::Solid,
        ),
        VANILLA_BLOCK_BUBBLE_CORAL_BLOCK_ID => (
            VANILLA_BLOCK_DEAD_BUBBLE_CORAL_BLOCK_ID,
            DeadStateKind::Solid,
        ),
        VANILLA_BLOCK_FIRE_CORAL_BLOCK_ID => {
            (VANILLA_BLOCK_DEAD_FIRE_CORAL_BLOCK_ID, DeadStateKind::Solid)
        }
        VANILLA_BLOCK_HORN_CORAL_BLOCK_ID => {
            (VANILLA_BLOCK_DEAD_HORN_CORAL_BLOCK_ID, DeadStateKind::Solid)
        }
        VANILLA_BLOCK_TUBE_CORAL_ID => {
            (VANILLA_BLOCK_DEAD_TUBE_CORAL_ID, DeadStateKind::Waterlogged)
        }
        VANILLA_BLOCK_BRAIN_CORAL_ID => (
            VANILLA_BLOCK_DEAD_BRAIN_CORAL_ID,
            DeadStateKind::Waterlogged,
        ),
        VANILLA_BLOCK_BUBBLE_CORAL_ID => (
            VANILLA_BLOCK_DEAD_BUBBLE_CORAL_ID,
            DeadStateKind::Waterlogged,
        ),
        VANILLA_BLOCK_FIRE_CORAL_ID => {
            (VANILLA_BLOCK_DEAD_FIRE_CORAL_ID, DeadStateKind::Waterlogged)
        }
        VANILLA_BLOCK_HORN_CORAL_ID => {
            (VANILLA_BLOCK_DEAD_HORN_CORAL_ID, DeadStateKind::Waterlogged)
        }
        VANILLA_BLOCK_TUBE_CORAL_FAN_ID => (
            VANILLA_BLOCK_DEAD_TUBE_CORAL_FAN_ID,
            DeadStateKind::Waterlogged,
        ),
        VANILLA_BLOCK_BRAIN_CORAL_FAN_ID => (
            VANILLA_BLOCK_DEAD_BRAIN_CORAL_FAN_ID,
            DeadStateKind::Waterlogged,
        ),
        VANILLA_BLOCK_BUBBLE_CORAL_FAN_ID => (
            VANILLA_BLOCK_DEAD_BUBBLE_CORAL_FAN_ID,
            DeadStateKind::Waterlogged,
        ),
        VANILLA_BLOCK_FIRE_CORAL_FAN_ID => (
            VANILLA_BLOCK_DEAD_FIRE_CORAL_FAN_ID,
            DeadStateKind::Waterlogged,
        ),
        VANILLA_BLOCK_HORN_CORAL_FAN_ID => (
            VANILLA_BLOCK_DEAD_HORN_CORAL_FAN_ID,
            DeadStateKind::Waterlogged,
        ),
        VANILLA_BLOCK_TUBE_CORAL_WALL_FAN_ID => (
            VANILLA_BLOCK_DEAD_TUBE_CORAL_WALL_FAN_ID,
            DeadStateKind::WallFan,
        ),
        VANILLA_BLOCK_BRAIN_CORAL_WALL_FAN_ID => (
            VANILLA_BLOCK_DEAD_BRAIN_CORAL_WALL_FAN_ID,
            DeadStateKind::WallFan,
        ),
        VANILLA_BLOCK_BUBBLE_CORAL_WALL_FAN_ID => (
            VANILLA_BLOCK_DEAD_BUBBLE_CORAL_WALL_FAN_ID,
            DeadStateKind::WallFan,
        ),
        VANILLA_BLOCK_FIRE_CORAL_WALL_FAN_ID => (
            VANILLA_BLOCK_DEAD_FIRE_CORAL_WALL_FAN_ID,
            DeadStateKind::WallFan,
        ),
        VANILLA_BLOCK_HORN_CORAL_WALL_FAN_ID => (
            VANILLA_BLOCK_DEAD_HORN_CORAL_WALL_FAN_ID,
            DeadStateKind::WallFan,
        ),
        _ => return None,
    })
}

fn dead_state(block_state_id: i32) -> Option<i32> {
    let (dead_block_type_id, kind) = dead_partner(block_type_of(block_state_id)?)?;
    let dead = super::default_block_state_id(dead_block_type_id);
    match kind {
        DeadStateKind::Solid => Some(dead),
        DeadStateKind::Waterlogged => {
            bool_property(block_state_id, WATERLOGGED)?;
            with_property(dead, WATERLOGGED, "false")
        }
        DeadStateKind::WallFan => {
            bool_property(block_state_id, WATERLOGGED)?;
            let facing = direction_property(block_state_id, FACING)?;
            with_property(dead, WATERLOGGED, "false").and_then(|state| {
                with_property(state, FACING, super::direction_serialized_name(facing))
            })
        }
    }
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if scan_for_water(pos, block_state_id, |at| {
        level.fluid_identity_in_tag(level.fluid_state(at).identity(), WATER_TAG)
    }) {
        return;
    }
    let Some(dead) = dead_state(block_state_id) else {
        return;
    };
    level.set_block(
        pos,
        dead,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

fn scan_for_water(
    pos: BlockPos,
    block_state_id: i32,
    water_at: impl FnMut(BlockPos) -> bool,
) -> bool {
    bool_property(block_state_id, WATERLOGGED) == Some(true) || water_beside(pos, water_at)
}

fn water_beside(pos: BlockPos, mut water_at: impl FnMut(BlockPos) -> bool) -> bool {
    for direction in DIRECTION_VALUES {
        if water_at(relative(pos, direction)) {
            return true;
        }
    }
    false
}

fn try_schedule_die_tick(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    if !scan_for_water(pos, block_state_id, |at| {
        level.fluid_identity_in_tag(level.fluid_state(at).identity(), WATER_TAG)
    }) {
        schedule_die_tick(level, pos, block_state_id);
    }
}

fn schedule_die_tick(level: &mut BlockShapeCtx<'_, '_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(block) = BlockTickType::of_block_state(block_state_id) else {
        return;
    };
    let delay = DIE_TICK_MIN_DELAY + level.level_random().next_int(DIE_TICK_DELAY_SPREAD);
    level.schedule_block_tick(pos, block, delay, TickPriority::Normal);
}

fn re_arm_water(level: &mut BlockShapeCtx<'_, '_, '_>, pos: BlockPos, block_state_id: i32) {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
}

pub(super) fn plant_type_can_survive(pos: BlockPos, block_state_at: BlockStateAt) -> Option<bool> {
    face_sturdy(block_state_at(below(pos)), Direction::Up, SupportType::Full)
}

pub(super) fn wall_fan_can_survive(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    let facing = direction_property(block_state_id, FACING)?;
    face_sturdy(
        block_state_at(relative(pos, opposite(facing))),
        facing,
        SupportType::Full,
    )
}

pub(super) fn plant_type_can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
) -> Option<bool> {
    let support_pos = below(pos);
    Some(face_sturdy_at(
        level.block_state(support_pos),
        support_pos,
        Direction::Up,
        SupportType::Full,
    ))
}

pub(super) fn wall_fan_can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    let facing = direction_property(block_state_id, FACING)?;
    let support_pos = relative(pos, opposite(facing));
    Some(face_sturdy_at(
        level.block_state(support_pos),
        support_pos,
        facing,
        SupportType::Full,
    ))
}

pub(super) fn plant_type_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    re_arm_water(level, pos, block_state_id);
    if direction_to_neighbour == Direction::Down
        && plant_type_can_survive_live(level, pos) == Some(false)
    {
        super::air_block_state_id()
    } else {
        block_state_id
    }
}

pub(super) fn living_plant_type_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if direction_to_neighbour == Direction::Down
        && plant_type_can_survive_live(level, pos) == Some(false)
    {
        return super::air_block_state_id();
    }
    re_arm_water(level, pos, block_state_id);
    try_schedule_die_tick(level, pos, block_state_id);
    plant_type_update_shape(level, block_state_id, pos, direction_to_neighbour)
}

pub(super) fn wall_fan_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    re_arm_water(level, pos, block_state_id);
    if wall_direction(block_state_id) == Some(direction_to_neighbour)
        && wall_fan_can_survive_live(level, block_state_id, pos) == Some(false)
    {
        super::air_block_state_id()
    } else {
        block_state_id
    }
}

pub(super) fn living_wall_fan_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if wall_direction(block_state_id) == Some(direction_to_neighbour)
        && wall_fan_can_survive_live(level, block_state_id, pos) == Some(false)
    {
        return super::air_block_state_id();
    }
    re_arm_water(level, pos, block_state_id);
    try_schedule_die_tick(level, pos, block_state_id);
    wall_fan_update_shape(level, block_state_id, pos, direction_to_neighbour)
}

pub(super) fn coral_block_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if !water_beside(pos, |at| {
        level.fluid_identity_in_tag(level.fluid_state(at).identity(), WATER_TAG)
    }) {
        schedule_die_tick(level, pos, block_state_id);
    }
    block_state_id
}

pub(super) fn on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    try_schedule_die_tick(&mut BlockShapeCtx::Live(level), pos, block_state_id);
}

fn wall_direction(block_state_id: i32) -> Option<Direction> {
    direction_property(block_state_id, FACING).map(opposite)
}
