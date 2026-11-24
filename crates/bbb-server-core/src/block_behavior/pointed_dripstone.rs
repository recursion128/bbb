//! Server-side `PointedDripstoneBlock` state transitions.

use super::{BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use crate::fluid::{with_property, FlowingFluid, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_state_types::vanilla_block_state_static_collision_shape;
use bbb_protocol::block_types::{
    VANILLA_BLOCK_CLAY_ID, VANILLA_BLOCK_DRIPSTONE_BLOCK_ID, VANILLA_BLOCK_MUD_ID,
    VANILLA_BLOCK_POINTED_DRIPSTONE_ID, VANILLA_BLOCK_WATER_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_protocol::registry_data_types::VanillaFluidIdentity;
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air, FluidFamily};

use super::cauldron::{can_receive_drip, DripFluid};
use super::{
    block_type_of, bool_property, default_block_state_id, direction_property, face_sturdy_at,
    offset, state_is_block, string_property, BlockStateAt, FallingBlockFallRequest,
    FallingBlockHurt, SupportType,
};

const TIP_DIRECTION: &str = "vertical_direction";
const THICKNESS: &str = "thickness";
const WATERLOGGED: &str = "waterlogged";
const MAX_FLUID_SEARCH: i32 = 11;
const MAX_GROWTH_LENGTH: i32 = 7;
const MAX_STALAGMITE_SEARCH: i32 = 10;
// `PointedDripstoneBlock.java:77`.
const CENTRAL_MIN: f64 = 0.375;
const CENTRAL_MAX: f64 = 0.625;
const WATER_TRANSFER_PROBABILITY: f32 = 0.175_781_25;
const LAVA_TRANSFER_PROBABILITY: f32 = 0.058_593_75;
const GROWTH_PROBABILITY: f32 = 0.011_377_778;
// `LevelEvent.DRIPSTONE_DRIP` (`LevelEvent.java:50`).
const DRIPSTONE_DRIP_LEVEL_EVENT: i32 = 1504;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Thickness {
    TipMerge,
    Tip,
    Frustum,
    Middle,
    Base,
}

impl Thickness {
    fn of_state(block_state_id: i32) -> Option<Self> {
        match string_property(block_state_id, THICKNESS)?.as_str() {
            "tip_merge" => Some(Self::TipMerge),
            "tip" => Some(Self::Tip),
            "frustum" => Some(Self::Frustum),
            "middle" => Some(Self::Middle),
            "base" => Some(Self::Base),
            _ => None,
        }
    }

    const fn property(self) -> &'static str {
        match self {
            Self::TipMerge => "tip_merge",
            Self::Tip => "tip",
            Self::Frustum => "frustum",
            Self::Middle => "middle",
            Self::Base => "base",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FluidInfo {
    pos: BlockPos,
    identity: VanillaFluidIdentity,
    source_state: i32,
}

fn is_pointed(block_state_id: i32) -> bool {
    state_is_block(block_state_id, VANILLA_BLOCK_POINTED_DRIPSTONE_ID)
}

fn is_pointed_with_direction(block_state_id: i32, direction: Direction) -> bool {
    is_pointed(block_state_id)
        && direction_property(block_state_id, TIP_DIRECTION) == Some(direction)
}

fn is_pointed_with_direction_exact(block_state_id: i32, direction: Direction) -> Option<bool> {
    if block_type_of(block_state_id)? != VANILLA_BLOCK_POINTED_DRIPSTONE_ID {
        return Some(false);
    }
    Some(direction_property(block_state_id, TIP_DIRECTION)? == direction)
}

fn is_tip(block_state_id: i32, include_merged: bool) -> bool {
    match Thickness::of_state(block_state_id).filter(|_| is_pointed(block_state_id)) {
        Some(Thickness::Tip) => true,
        Some(Thickness::TipMerge) => include_merged,
        _ => false,
    }
}

fn is_unmerged_tip_with_direction(block_state_id: i32, direction: Direction) -> bool {
    is_tip(block_state_id, false)
        && direction_property(block_state_id, TIP_DIRECTION) == Some(direction)
}

fn is_stalactite(block_state_id: i32) -> bool {
    is_pointed_with_direction(block_state_id, Direction::Down)
}

fn is_stalagmite(block_state_id: i32) -> bool {
    is_pointed_with_direction(block_state_id, Direction::Up)
}

fn is_valid_placement(pos: BlockPos, tip_direction: Direction, at: BlockStateAt<'_>) -> bool {
    let behind_pos = relative(pos, opposite(tip_direction));
    let behind = at(behind_pos);
    face_sturdy_at(behind, behind_pos, tip_direction, SupportType::Full)
        || is_pointed_with_direction(behind, tip_direction)
}

fn is_valid_placement_exact(
    pos: BlockPos,
    tip_direction: Direction,
    at: BlockStateAt<'_>,
) -> Option<bool> {
    let behind_pos = relative(pos, opposite(tip_direction));
    let behind = at(behind_pos);
    if face_sturdy_at(behind, behind_pos, tip_direction, SupportType::Full) {
        Some(true)
    } else {
        is_pointed_with_direction_exact(behind, tip_direction)
    }
}

fn is_valid_placement_at<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    tip_direction: Direction,
) -> Option<bool> {
    is_valid_placement_exact(pos, tip_direction, &mut |at| level.block_state(at))
}

fn placement_tip_direction(
    pos: BlockPos,
    default_tip_direction: Direction,
    at: BlockStateAt<'_>,
) -> Option<Direction> {
    if is_valid_placement_exact(pos, default_tip_direction, &mut *at)? {
        return Some(default_tip_direction);
    }
    let opposite_direction = opposite(default_tip_direction);
    is_valid_placement_exact(pos, opposite_direction, &mut *at)?.then_some(opposite_direction)
}

fn placement_thickness(
    pos: BlockPos,
    tip_direction: Direction,
    merge_opposing_tips: bool,
    at: BlockStateAt<'_>,
) -> Option<Thickness> {
    let base_direction = opposite(tip_direction);
    let in_front = at(relative(pos, tip_direction));
    if is_pointed_with_direction_exact(in_front, base_direction)? {
        return if !merge_opposing_tips && Thickness::of_state(in_front)? != Thickness::TipMerge {
            Some(Thickness::Tip)
        } else {
            Some(Thickness::TipMerge)
        };
    }
    if !is_pointed_with_direction_exact(in_front, tip_direction)? {
        return Some(Thickness::Tip);
    }
    if matches!(
        Thickness::of_state(in_front)?,
        Thickness::Tip | Thickness::TipMerge
    ) {
        return Some(Thickness::Frustum);
    }
    let behind = at(relative(pos, base_direction));
    Some(if is_pointed_with_direction_exact(behind, tip_direction)? {
        Thickness::Middle
    } else {
        Thickness::Base
    })
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    x_rot: f32,
    secondary_use_active: bool,
    at: BlockStateAt<'_>,
) -> Option<i32> {
    if block_type_id != VANILLA_BLOCK_POINTED_DRIPSTONE_ID || !x_rot.is_finite() {
        return None;
    }
    let default_tip_direction = if x_rot < 0.0 {
        Direction::Down
    } else {
        Direction::Up
    };
    let tip_direction = placement_tip_direction(pos, default_tip_direction, &mut *at)?;
    let thickness = placement_thickness(pos, tip_direction, !secondary_use_active, &mut *at)?;
    let replaced = at(pos);
    block_type_of(replaced)?;
    let fluid = block_state_fluid_state(replaced);
    let waterlogged = fluid.is_source() && fluid.is_same_family(FluidFamily::Water);
    let state = with_property(
        default_block_state_id(block_type_id),
        TIP_DIRECTION,
        super::direction_serialized_name(tip_direction),
    )?;
    let state = with_property(state, THICKNESS, thickness.property())?;
    with_property(
        state,
        WATERLOGGED,
        if waterlogged { "true" } else { "false" },
    )
}

pub(super) fn can_survive(block_state_id: i32, pos: BlockPos, at: BlockStateAt<'_>) -> bool {
    direction_property(block_state_id, TIP_DIRECTION)
        .is_some_and(|direction| is_valid_placement(pos, direction, at))
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    let tip_direction = direction_property(block_state_id, TIP_DIRECTION)?;
    let support_pos = relative(pos, opposite(tip_direction));
    let support = level.block_state(support_pos);
    if face_sturdy_at(support, support_pos, tip_direction, SupportType::Full) {
        return Some(true);
    }
    is_pointed_with_direction_exact(support, tip_direction)
}

fn calculate_thickness<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
    tip_direction: Direction,
    merge_opposing_tips: bool,
) -> Thickness {
    let base_direction = opposite(tip_direction);
    let in_front = level.block_state(relative(pos, tip_direction));
    if is_pointed_with_direction(in_front, base_direction) {
        if !merge_opposing_tips && Thickness::of_state(in_front) != Some(Thickness::TipMerge) {
            Thickness::Tip
        } else {
            Thickness::TipMerge
        }
    } else if !is_pointed_with_direction(in_front, tip_direction) {
        Thickness::Tip
    } else if !matches!(
        Thickness::of_state(in_front),
        Some(Thickness::Tip | Thickness::TipMerge)
    ) {
        let behind = level.block_state(relative(pos, base_direction));
        if is_pointed_with_direction(behind, tip_direction) {
            Thickness::Middle
        } else {
            Thickness::Base
        }
    } else {
        Thickness::Frustum
    }
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    if !matches!(direction_to_neighbour, Direction::Up | Direction::Down) {
        return block_state_id;
    }
    let Some(tip_direction) = direction_property(block_state_id, TIP_DIRECTION) else {
        return block_state_id;
    };
    let Some(tick_type) = BlockTickType::of_block_state(block_state_id) else {
        return block_state_id;
    };
    if tip_direction == Direction::Down && level.has_scheduled_tick(pos, tick_type) {
        return block_state_id;
    }
    let survives = is_valid_placement_at(level, pos, tip_direction);
    if direction_to_neighbour == opposite(tip_direction) && survives == Some(false) {
        level.schedule_block_tick(
            pos,
            tick_type,
            if tip_direction == Direction::Down {
                2
            } else {
                1
            },
            TickPriority::Normal,
        );
        return block_state_id;
    }
    let merge = Thickness::of_state(block_state_id) == Some(Thickness::TipMerge);
    with_property(
        block_state_id,
        THICKNESS,
        calculate_thickness(level, pos, tip_direction, merge).property(),
    )
    .unwrap_or(block_state_id)
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if is_stalagmite(block_state_id)
        && is_valid_placement_at(level, pos, Direction::Up) == Some(false)
    {
        level.destroy_block(pos, true);
    } else {
        spawn_falling_stalactite(level, pos, block_state_id);
    }
}

fn spawn_falling_stalactite(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let mut fall_pos = pos;
    let mut fall_state = block_state_id;
    while is_stalactite(fall_state) {
        // `:299-302`: only the first tip, merged included, hurts on landing.
        let hurt_entities = is_tip(fall_state, true).then(|| {
            let size = (1 + pos.y - fall_pos.y).max(6);
            FallingBlockHurt {
                damage_per_distance: 1.0 * size as f32,
                damage_max: 40,
            }
        });
        let is_tip_segment = hurt_entities.is_some();
        if level.request_falling_block_fall(FallingBlockFallRequest {
            pos: fall_pos,
            block_state_id: fall_state,
            disable_drop: false,
            hurt_entities,
        }) != Some(true)
        {
            return;
        }
        if is_tip_segment {
            break;
        }
        fall_pos = relative(fall_pos, Direction::Down);
        fall_state = level.block_state(fall_pos);
    }
}

fn relative_steps(pos: BlockPos, direction: Direction, steps: i32) -> BlockPos {
    let mut answer = pos;
    for _ in 0..steps {
        answer = relative(answer, direction);
    }
    answer
}

fn is_stalactite_start(level: &LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) -> bool {
    is_stalactite(block_state_id) && !is_pointed(level.block_state(relative(pos, Direction::Up)))
}

fn find_root(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    max_search: i32,
) -> Option<BlockPos> {
    let tip_direction = direction_property(block_state_id, TIP_DIRECTION)?;
    let search_direction = opposite(tip_direction);
    for step in 1..max_search {
        let test_pos = relative_steps(pos, search_direction, step);
        let state = level.block_state(test_pos);
        if !is_pointed(state) {
            return Some(test_pos);
        }
        if test_pos.y < level.get_min_y()
            || test_pos.y > level.get_max_y()
            || !is_pointed_with_direction(state, tip_direction)
        {
            return None;
        }
    }
    None
}

/// `PointedDripstoneBlock.findStalactiteTipAboveCauldron`
/// (`PointedDripstoneBlock.java:529-532`).
pub(super) fn find_stalactite_tip_above_cauldron(
    level: &LevelCtx<'_, '_>,
    cauldron_pos: BlockPos,
) -> Option<BlockPos> {
    for step in 1..MAX_FLUID_SEARCH {
        let pos = offset(cauldron_pos, 0, step, 0);
        let state = level.block_state(pos);
        if is_unmerged_tip_with_direction(state, Direction::Down)
            && bool_property(state, WATERLOGGED) == Some(false)
        {
            return Some(pos);
        }
        if pos.y < level.get_min_y()
            || pos.y > level.get_max_y()
            || !can_drip_through(level, pos, state)
        {
            return None;
        }
    }
    None
}

/// `PointedDripstoneBlock.getCauldronFillFluidType`
/// (`PointedDripstoneBlock.java:534-539`).
pub(super) fn cauldron_fill_fluid_type(
    level: &LevelCtx<'_, '_>,
    stalactite_tip: BlockPos,
) -> Option<DripFluid> {
    let tip_state = level.block_state(stalactite_tip);
    match fluid_above_stalactite(level, stalactite_tip, tip_state)?.identity {
        VanillaFluidIdentity::Water => Some(DripFluid::Water),
        VanillaFluidIdentity::Lava => Some(DripFluid::Lava),
        _ => None,
    }
}

/// `PointedDripstoneBlock.canDripThrough` (`PointedDripstoneBlock.java:601-616`).
fn can_drip_through(level: &LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) -> bool {
    if block_state_is_air(block_state_id) {
        return true;
    }
    match super::support::solid_render(block_state_id) {
        Some(true) | None => return false,
        Some(false) => {}
    }
    if !level.fluid_state(pos).is_empty() {
        return false;
    }
    let Some(shape) = vanilla_block_state_static_collision_shape(block_state_id) else {
        return false;
    };
    !shape.boxes().any(|aabb| {
        aabb.max_x > CENTRAL_MIN
            && aabb.min_x < CENTRAL_MAX
            && aabb.max_y > 0.0
            && aabb.min_y < 1.0
            && aabb.max_z > CENTRAL_MIN
            && aabb.min_z < CENTRAL_MAX
    })
}

fn fluid_above_stalactite(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) -> Option<FluidInfo> {
    if !is_stalactite(block_state_id) {
        return None;
    }
    let root = find_root(level, pos, block_state_id, MAX_FLUID_SEARCH)?;
    let source_pos = relative(root, Direction::Up);
    let source_state = level.block_state(source_pos);
    let identity = if state_is_block(source_state, VANILLA_BLOCK_MUD_ID)
        && !level.water_evaporates(source_pos)
    {
        VanillaFluidIdentity::Water
    } else {
        level.fluid_state(source_pos).identity()
    };
    Some(FluidInfo {
        pos: source_pos,
        identity,
        source_state,
    })
}

fn find_tip(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    max_search: i32,
) -> Option<BlockPos> {
    if is_tip(block_state_id, false) {
        return Some(pos);
    }
    let direction = direction_property(block_state_id, TIP_DIRECTION)?;
    for step in 1..max_search {
        let test_pos = relative_steps(pos, direction, step);
        let state = level.block_state(test_pos);
        if is_tip(state, false) {
            return Some(test_pos);
        }
        if test_pos.y < level.get_min_y()
            || test_pos.y > level.get_max_y()
            || !is_pointed_with_direction(state, direction)
        {
            return None;
        }
    }
    None
}

fn drip_fluid(identity: VanillaFluidIdentity) -> Option<DripFluid> {
    match identity {
        VanillaFluidIdentity::Water => Some(DripFluid::Water),
        VanillaFluidIdentity::Lava => Some(DripFluid::Lava),
        _ => None,
    }
}

fn find_fillable_cauldron(
    level: &LevelCtx<'_, '_>,
    tip_pos: BlockPos,
    fluid: DripFluid,
) -> Option<BlockPos> {
    for step in 1..MAX_FLUID_SEARCH {
        let test_pos = relative_steps(tip_pos, Direction::Down, step);
        let state = level.block_state(test_pos);
        if can_receive_drip(state, fluid) {
            return Some(test_pos);
        }
        if test_pos.y < level.get_min_y()
            || test_pos.y > level.get_max_y()
            || !can_drip_through(level, test_pos, state)
        {
            return None;
        }
    }
    None
}

fn maybe_transfer_fluid(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    random_value: f32,
) {
    if random_value > WATER_TRANSFER_PROBABILITY || !is_stalactite_start(level, pos, block_state_id)
    {
        return;
    }
    let Some(fluid_info) = fluid_above_stalactite(level, pos, block_state_id) else {
        return;
    };
    let probability = match fluid_info.identity {
        VanillaFluidIdentity::Water => WATER_TRANSFER_PROBABILITY,
        VanillaFluidIdentity::Lava => LAVA_TRANSFER_PROBABILITY,
        _ => return,
    };
    if random_value >= probability {
        return;
    }
    let Some(tip_pos) = find_tip(level, pos, block_state_id, MAX_FLUID_SEARCH) else {
        return;
    };
    if state_is_block(fluid_info.source_state, VANILLA_BLOCK_MUD_ID)
        && fluid_info.identity == VanillaFluidIdentity::Water
    {
        let clay = default_block_state_id(VANILLA_BLOCK_CLAY_ID);
        level.set_block(
            fluid_info.pos,
            clay,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        // PointedDripstoneBlock.java:216.
        bbb_server_worldgen::WorldGenLevel::push_entities_up(
            level,
            fluid_info.source_state,
            clay,
            fluid_info.pos,
        );
        level.block_game_event(VanillaGameEvent::BLOCK_CHANGE, fluid_info.pos, Some(clay));
        level.level_event(DRIPSTONE_DRIP_LEVEL_EVENT, tip_pos, 0);
        return;
    }
    let Some(fluid) = drip_fluid(fluid_info.identity) else {
        return;
    };
    let Some(cauldron_pos) = find_fillable_cauldron(level, tip_pos, fluid) else {
        return;
    };
    let cauldron_state = level.block_state(cauldron_pos);
    let Some(tick_type) = BlockTickType::of_block_state(cauldron_state) else {
        return;
    };
    level.level_event(DRIPSTONE_DRIP_LEVEL_EVENT, tip_pos, 0);
    level.schedule_block_tick(
        cauldron_pos,
        tick_type,
        50 + tip_pos.y - cauldron_pos.y,
        TickPriority::Normal,
    );
}

fn can_drip(block_state_id: i32) -> bool {
    is_stalactite(block_state_id)
        && Thickness::of_state(block_state_id) == Some(Thickness::Tip)
        && bool_property(block_state_id, WATERLOGGED) == Some(false)
}

fn can_tip_grow(level: &LevelCtx<'_, '_>, tip_pos: BlockPos, tip_state: i32) -> bool {
    let Some(direction) = direction_property(tip_state, TIP_DIRECTION) else {
        return false;
    };
    let grow_pos = relative(tip_pos, direction);
    let state = level.block_state(grow_pos);
    level.fluid_state(grow_pos).is_empty()
        && (block_state_is_air(state) || is_unmerged_tip_with_direction(state, opposite(direction)))
}

fn create_dripstone(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    direction: Direction,
    thickness: Thickness,
) {
    let direction_value = match direction {
        Direction::Up => "up",
        Direction::Down => "down",
        _ => return,
    };
    let mut state = default_block_state_id(VANILLA_BLOCK_POINTED_DRIPSTONE_ID);
    let Some(next) = with_property(state, TIP_DIRECTION, direction_value) else {
        return;
    };
    state = next;
    let Some(next) = with_property(state, THICKNESS, thickness.property()) else {
        return;
    };
    state = next;
    let waterlogged = level.fluid_state(pos).identity() == VanillaFluidIdentity::Water;
    let Some(state) = with_property(
        state,
        WATERLOGGED,
        if waterlogged { "true" } else { "false" },
    ) else {
        return;
    };
    level.set_block(
        pos,
        state,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

fn create_merged_tips(level: &mut LevelCtx<'_, '_>, tip_pos: BlockPos, tip_state: i32) {
    let (stalactite_pos, stalagmite_pos) =
        if direction_property(tip_state, TIP_DIRECTION) == Some(Direction::Up) {
            (relative(tip_pos, Direction::Up), tip_pos)
        } else {
            (tip_pos, relative(tip_pos, Direction::Down))
        };
    create_dripstone(level, stalactite_pos, Direction::Down, Thickness::TipMerge);
    create_dripstone(level, stalagmite_pos, Direction::Up, Thickness::TipMerge);
}

fn grow(level: &mut LevelCtx<'_, '_>, from: BlockPos, direction: Direction) {
    let target = relative(from, direction);
    let state = level.block_state(target);
    if is_unmerged_tip_with_direction(state, opposite(direction)) {
        create_merged_tips(level, target, state);
    } else if block_state_is_air(state) || block_type_of(state) == Some(VANILLA_BLOCK_WATER_ID) {
        create_dripstone(level, target, direction, Thickness::Tip);
    }
}

fn grow_stalagmite_below(level: &mut LevelCtx<'_, '_>, tip_pos: BlockPos) {
    for step in 1..=MAX_STALAGMITE_SEARCH {
        let pos = relative_steps(tip_pos, Direction::Down, step);
        let state = level.block_state(pos);
        if !level.fluid_state(pos).is_empty() {
            return;
        }
        if is_unmerged_tip_with_direction(state, Direction::Up) && can_tip_grow(level, pos, state) {
            grow(level, pos, Direction::Up);
            return;
        }
        let Some(valid) = is_valid_placement_at(level, pos, Direction::Up) else {
            return;
        };
        let below = relative(pos, Direction::Down);
        if valid && level.fluid_state(below).family() != Some(FluidFamily::Water) {
            grow(level, below, Direction::Up);
            return;
        }
        if !can_drip_through(level, pos, state) {
            return;
        }
    }
}

fn maybe_grow(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let root_pos = relative(pos, Direction::Up);
    let above_root_pos = relative(root_pos, Direction::Up);
    let above_root_fluid = level.fluid_state(above_root_pos);
    if !state_is_block(
        level.block_state(root_pos),
        VANILLA_BLOCK_DRIPSTONE_BLOCK_ID,
    ) || above_root_fluid.identity() != VanillaFluidIdentity::Water
        || !above_root_fluid.is_source()
    {
        return;
    }
    let Some(tip_pos) = find_tip(level, pos, block_state_id, MAX_GROWTH_LENGTH) else {
        return;
    };
    let tip_state = level.block_state(tip_pos);
    if !can_drip(tip_state) || !can_tip_grow(level, tip_pos, tip_state) {
        return;
    }
    if level.level_random().next_boolean() {
        grow(level, tip_pos, Direction::Down);
    } else {
        grow_stalagmite_below(level, tip_pos);
    }
}

pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let transfer_draw = level.level_random().next_float();
    maybe_transfer_fluid(level, pos, block_state_id, transfer_draw);
    let growth_draw = level.level_random().next_float();
    if growth_draw < GROWTH_PROBABILITY && is_stalactite_start(level, pos, block_state_id) {
        maybe_grow(level, pos, block_state_id);
    }
}
