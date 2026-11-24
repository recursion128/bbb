//!
//! All three ask `MultifaceBlock.canAttachTo`, which takes either the support
//! shape or the collision shape of the cell it is holding on to, and they
//! differ only in how many faces they may hold at once: six for a lichen, the
//! four horizontals for a moss carpet, and the one above for hanging moss.
//! Losing a face is not the same as dying -- a lichen drops the face and stays
//! -- so these are the first survivals here whose `updateShape` rewrites the
//! state rather than answering yes or no.

use super::{BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use bbb_protocol::block_state_types::vanilla_block_state_support_facts;
use bbb_protocol::block_types::{
    VanillaCanSurviveOverride, VanillaGetStateForPlacementOverride,
    VANILLA_BLOCK_PALE_MOSS_CARPET_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{above, below, with_property, FlowingFluid, HORIZONTAL, SET_BLOCK_UPDATE_ALL};
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, TickPriority};
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::{block_state_is_air, FluidFamily};

use super::{
    air_block_state_id, block_type_of, bool_property, can_survive_override, default_block_state_id,
    get_state_for_placement_override, multiface_can_attach_to, multiface_can_attach_to_at,
    string_property, BlockStateAt, DIRECTION_VALUES,
};

const WATERLOGGED: &str = "waterlogged";

/// `MossyCarpetBlock.BASE` (`MossyCarpetBlock.java:36`), which is
/// `BlockStateProperties.BOTTOM`, and `HangingMossBlock.TIP`
/// (`HangingMossBlock.java:27`).
const BASE: &str = "bottom";
const TIP: &str = "tip";

/// `WallSide.NONE` (`WallSide.java:6`), the value every face question here
/// answers with when it fails.
const WALL_SIDE_NONE: &str = "none";
const WALL_SIDE_LOW: &str = "low";
const WALL_SIDE_TALL: &str = "tall";

fn declares(block_state_id: i32, class: VanillaCanSurviveOverride) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override) == Some(class)
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

// ---------------------------------------------------------------------------
// `MultifaceBlock`
// ---------------------------------------------------------------------------

#[must_use]
pub fn is_multiface(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::MultifaceBlock)
}

fn face_property(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

pub fn has_face(block_state_id: i32, direction: Direction) -> bool {
    bool_property(block_state_id, face_property(direction)) == Some(true)
}

#[must_use]
pub fn with_face(block_state_id: i32, direction: Direction) -> Option<i32> {
    with_property(block_state_id, face_property(direction), "true")
}

fn has_any_face(block_state_id: i32) -> bool {
    DIRECTION_VALUES
        .iter()
        .any(|&direction| has_face(block_state_id, direction))
}

pub(super) fn has_vacant_face(block_state_id: i32) -> Option<bool> {
    for direction in DIRECTION_VALUES {
        if !bool_property(block_state_id, face_property(direction))? {
            return Some(true);
        }
    }
    Some(false)
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    old_state: i32,
    directions: [Direction; 6],
    source_water: bool,
    block_state_at: BlockStateAt,
) -> Option<i32> {
    let same_block = block_type_of(old_state)? == block_type_id;
    for direction in directions {
        if same_block && bool_property(old_state, face_property(direction))? {
            continue;
        }
        let neighbour = block_state_at(relative(pos, direction));
        match multiface_can_attach_to(neighbour, direction) {
            Some(true) => {}
            Some(false) => continue,
            None => return None,
        }
        let state = if same_block {
            old_state
        } else {
            with_property(
                default_block_state_id(block_type_id),
                WATERLOGGED,
                if source_water { "true" } else { "false" },
            )?
        };
        return with_property(state, face_property(direction), "true");
    }
    None
}

pub(super) fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    let mut has_at_least_one_face = false;
    for direction in DIRECTION_VALUES {
        if has_face(block_state_id, direction) {
            let neighbour = block_state_at(relative(pos, direction));
            if !multiface_can_attach_to(neighbour, direction)? {
                return Some(false);
            }
            has_at_least_one_face = true;
        }
    }
    Some(has_at_least_one_face)
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    let mut has_at_least_one_face = false;
    for direction in DIRECTION_VALUES {
        if has_face(block_state_id, direction) {
            let neighbour_pos = relative(pos, direction);
            if !multiface_can_attach_to_at(
                level.block_state(neighbour_pos),
                neighbour_pos,
                direction,
            ) {
                return Some(false);
            }
            has_at_least_one_face = true;
        }
    }
    Some(has_at_least_one_face)
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    re_arm_water(level, pos, block_state_id);
    if !has_any_face(block_state_id) {
        return air_block_state_id();
    }
    if !has_face(block_state_id, direction_to_neighbour) {
        return block_state_id;
    }
    let neighbour_pos = relative(pos, direction_to_neighbour);
    if multiface_can_attach_to_at(neighbour_state, neighbour_pos, direction_to_neighbour) {
        block_state_id
    } else {
        remove_face(block_state_id, direction_to_neighbour)
    }
}

fn remove_face(block_state_id: i32, direction: Direction) -> i32 {
    let Some(without) = with_property(block_state_id, face_property(direction), "false") else {
        return block_state_id;
    };
    if has_any_face(without) {
        without
    } else {
        air_block_state_id()
    }
}

// ---------------------------------------------------------------------------
// `MossyCarpetBlock`
// ---------------------------------------------------------------------------

#[must_use]
pub fn is_mossy_carpet(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::MossyCarpetBlock)
}

fn is_base(block_state_id: i32) -> bool {
    bool_property(block_state_id, BASE) == Some(true)
}

fn wall_side(block_state_id: i32, direction: Direction) -> Option<String> {
    string_property(block_state_id, face_property(direction))
}

pub(super) fn mossy_carpet_can_survive(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    let base = bool_property(block_state_id, BASE)?;
    let below_state = block_state_at(below(pos));
    block_type_of(below_state)?;
    if base {
        Some(!block_state_is_air(below_state))
    } else if is_mossy_carpet(below_state) {
        bool_property(below_state, BASE)
    } else {
        Some(false)
    }
}

fn has_faces(block_state_id: i32) -> bool {
    is_base(block_state_id)
        || HORIZONTAL.iter().any(|&direction| {
            wall_side(block_state_id, direction).as_deref() != Some(WALL_SIDE_NONE)
        })
}

fn updated_state_at<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
    create_sides: bool,
) -> Option<i32> {
    let create_sides = create_sides || is_base(block_state_id);
    let mut state = block_state_id;
    for direction in HORIZONTAL {
        let neighbour_pos = relative(pos, direction);
        let supported = Some(multiface_can_attach_to_at(
            level.block_state(neighbour_pos),
            neighbour_pos,
            direction,
        ))?;
        let mut side = if supported {
            if create_sides {
                WALL_SIDE_LOW.to_owned()
            } else {
                wall_side(state, direction).unwrap_or_else(|| WALL_SIDE_NONE.to_owned())
            }
        } else {
            WALL_SIDE_NONE.to_owned()
        };
        if side == WALL_SIDE_LOW {
            let above_state = level.block_state(above(pos));
            if is_mossy_carpet(above_state)
                && wall_side(above_state, direction).as_deref() != Some(WALL_SIDE_NONE)
                && !is_base(above_state)
            {
                side = WALL_SIDE_TALL.to_owned();
            }
            if !is_base(block_state_id) {
                let below_state = level.block_state(below(pos));
                if is_mossy_carpet(below_state)
                    && wall_side(below_state, direction).as_deref() == Some(WALL_SIDE_NONE)
                {
                    side = WALL_SIDE_NONE.to_owned();
                }
            }
        }
        state = with_property(state, face_property(direction), &side)?;
    }
    Some(state)
}

pub(super) fn mossy_carpet_state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::MossyCarpetBlock)
    {
        return None;
    }
    let mut state = default_block_state_id(block_type_id);
    bool_property(state, BASE)?;
    let mut above_state = None;
    for direction in HORIZONTAL {
        let neighbour = block_state_at(relative(pos, direction));
        block_type_of(neighbour)?;
        let mut side = if multiface_can_attach_to(neighbour, direction)? {
            WALL_SIDE_LOW
        } else {
            WALL_SIDE_NONE
        };
        if side == WALL_SIDE_LOW {
            let above_state = match above_state {
                Some(above_state) => above_state,
                None => {
                    let above = block_state_at(above(pos));
                    block_type_of(above)?;
                    above_state = Some(above);
                    above
                }
            };
            if is_mossy_carpet(above_state)
                && wall_side(above_state, direction)?.as_str() != WALL_SIDE_NONE
                && !bool_property(above_state, BASE)?
            {
                side = WALL_SIDE_TALL;
            }
        }
        state = with_property(state, face_property(direction), side)?;
    }
    Some(state)
}

pub(super) fn mossy_carpet_set_placed_by(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    if let Some(topper) = mossy_carpet_topper(level, pos, || !level.level_random().next_boolean()) {
        level.set_block(
            above(pos),
            topper,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

pub(super) fn mossy_carpet_is_valid_bonemeal_target(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
) -> bool {
    is_base(state) && mossy_carpet_topper(level, pos, || true).is_some()
}

pub(super) fn mossy_carpet_perform_bonemeal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    _state: i32,
) {
    if let Some(topper) = mossy_carpet_topper(level, pos, || true) {
        level.set_block(
            above(pos),
            topper,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

/// `MossyCarpetBlock.createTopperWithSideChance` (`MossyCarpetBlock.java:189-208`).
fn mossy_carpet_topper(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    mut side_survival: impl FnMut() -> bool,
) -> Option<i32> {
    let above_pos = above(pos);
    let above_previous = level.block_state(above_pos);
    let is_carpet_above = is_mossy_carpet(above_previous);
    if (is_carpet_above && is_base(above_previous))
        || (!is_carpet_above
            && vanilla_block_state_support_facts(above_previous).map(|facts| facts.replaceable())
                != Some(true))
    {
        return None;
    }
    let no_base = with_property(
        default_block_state_id(VANILLA_BLOCK_PALE_MOSS_CARPET_ID),
        BASE,
        "false",
    )?;
    let mut topper = updated_state_at(level, no_base, above_pos, true)?;
    for direction in HORIZONTAL {
        if wall_side(topper, direction).as_deref() != Some(WALL_SIDE_NONE) && !side_survival() {
            topper = with_property(topper, face_property(direction), WALL_SIDE_NONE)?;
        }
    }
    (has_faces(topper) && topper != above_previous).then_some(topper)
}

pub(super) fn mossy_carpet_update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    match mossy_carpet_can_survive(block_state_id, pos, &mut |at| level.block_state(at)) {
        Some(true) => {}
        Some(false) => return air_block_state_id(),
        None => return block_state_id,
    }
    let Some(updated) = updated_state_at(level, block_state_id, pos, false) else {
        return block_state_id;
    };
    if has_faces(updated) {
        updated
    } else {
        air_block_state_id()
    }
}

// ---------------------------------------------------------------------------
// `HangingMossBlock`
// ---------------------------------------------------------------------------

/// The delay `HangingMossBlock.updateShape` arms its own tick with
/// (`HangingMossBlock.java:82`).
const HANGING_MOSS_TICK_DELAY: i32 = 1;

#[must_use]
pub fn is_hanging_moss(block_state_id: i32) -> bool {
    declares(block_state_id, VanillaCanSurviveOverride::HangingMossBlock)
}

fn can_stay_at_position(pos: BlockPos, block_state_at: BlockStateAt) -> Option<bool> {
    let above_state = block_state_at(above(pos));
    if is_hanging_moss(above_state) {
        return Some(true);
    }
    multiface_can_attach_to(above_state, Direction::Up)
}

fn can_stay_at_position_at<L: BlockGetter + ?Sized>(level: &L, pos: BlockPos) -> Option<bool> {
    let support_pos = above(pos);
    let support = level.block_state(support_pos);
    if is_hanging_moss(support) {
        return Some(true);
    }
    Some(multiface_can_attach_to_at(
        support,
        support_pos,
        Direction::Up,
    ))
}

pub(super) fn hanging_moss_can_survive(
    pos: BlockPos,
    block_state_at: BlockStateAt,
) -> Option<bool> {
    can_stay_at_position(pos, block_state_at)
}

pub(super) fn hanging_moss_can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    pos: BlockPos,
) -> Option<bool> {
    let support_pos = above(pos);
    let support = level.block_state(support_pos);
    if is_hanging_moss(support) {
        return Some(true);
    }
    Some(multiface_can_attach_to_at(
        support,
        support_pos,
        Direction::Up,
    ))
}

pub(super) fn hanging_moss_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if can_stay_at_position_at(level, pos) == Some(false) {
        if let Some(block) = BlockTickType::of_block_state(block_state_id) {
            level.schedule_block_tick(pos, block, HANGING_MOSS_TICK_DELAY, TickPriority::Normal);
        }
    }
    let tip = !is_hanging_moss(level.block_state(below(pos)));
    with_property(block_state_id, TIP, if tip { "true" } else { "false" }).unwrap_or(block_state_id)
}

pub(super) fn hanging_moss_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    if can_stay_at_position_at(level, pos) == Some(false) {
        level.destroy_block(pos, true);
    }
}
