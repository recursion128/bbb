//! `DoorBlock` (`net/minecraft/world/level/block/DoorBlock.java`).
//!
//!
//! A door is two blocks that have to agree, plus a handle:
//!
//!
//! # What is not here
//!

use super::{BlockShapeCtx, LevelCtx};
use crate::block_getter::BlockGetter;

use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::{
    VanillaCanSurviveOverride, VANILLA_BLOCK_ACACIA_DOOR_ID, VANILLA_BLOCK_BAMBOO_DOOR_ID,
    VANILLA_BLOCK_BIRCH_DOOR_ID, VANILLA_BLOCK_CHERRY_DOOR_ID, VANILLA_BLOCK_COPPER_DOOR_ID,
    VANILLA_BLOCK_CRIMSON_DOOR_ID, VANILLA_BLOCK_DARK_OAK_DOOR_ID,
    VANILLA_BLOCK_EXPOSED_COPPER_DOOR_ID, VANILLA_BLOCK_IRON_DOOR_ID, VANILLA_BLOCK_JUNGLE_DOOR_ID,
    VANILLA_BLOCK_MANGROVE_DOOR_ID, VANILLA_BLOCK_OAK_DOOR_ID,
    VANILLA_BLOCK_OXIDIZED_COPPER_DOOR_ID, VANILLA_BLOCK_PALE_OAK_DOOR_ID,
    VANILLA_BLOCK_SPRUCE_DOOR_ID, VANILLA_BLOCK_WARPED_DOOR_ID, VANILLA_BLOCK_WAXED_COPPER_DOOR_ID,
    VANILLA_BLOCK_WAXED_EXPOSED_COPPER_DOOR_ID, VANILLA_BLOCK_WAXED_OXIDIZED_COPPER_DOOR_ID,
    VANILLA_BLOCK_WAXED_WEATHERED_COPPER_DOOR_ID, VANILLA_BLOCK_WEATHERED_COPPER_DOOR_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{above, below, with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use bbb_server_types::directions::{clock_wise, counter_clock_wise, relative};

use crate::redstone;

use super::{
    block_type_of, bool_property, can_survive_override, collision_shape_full_block,
    default_block_state_id, face_sturdy, face_sturdy_at, state_is_block, BlockInteractionPlayer,
    BlockStateAt, SET_BLOCK_UPDATE_CLIENTS, SET_BLOCK_UPDATE_IMMEDIATE,
};

/// `DoorBlock.HALF` (`DoorBlock.java:48`), i.e.
/// `BlockStateProperties.DOUBLE_BLOCK_HALF`.
const HALF: &str = "half";
const OPEN: &str = "open";
const POWERED: &str = "powered";

/// `DoorBlock.FACING` (`DoorBlock.java:47`) and `DoorBlock.HINGE` (`:49`),
/// i.e. `HorizontalDirectionalBlock.FACING` and
/// `BlockStateProperties.DOOR_HINGE` (`BlockStateProperties.java:132`).
const FACING: &str = "facing";
const HINGE: &str = "hinge";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorHalf {
    Upper,
    Lower,
}

impl DoorHalf {
    /// The stored property value, i.e. `DoubleBlockHalf.getSerializedName`
    /// (`DoubleBlockHalf.java:26-28`).
    const fn property_value(self) -> &'static str {
        match self {
            Self::Upper => "upper",
            Self::Lower => "lower",
        }
    }
}

/// Whether this slice owns a stored state's survival.
///
#[must_use]
pub fn is_door(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override)
        == Some(VanillaCanSurviveOverride::DoorBlock)
}

/// `state.getValue(DoorBlock.HALF)` for a state already known to be a door.
///
/// The `is_door` gate is not decoration: `half` is also a `StairBlock` and a
/// `TrapDoorBlock` property, whose values are `top` and `bottom`, so reading
/// the property without the class test would answer for the wrong blocks.
#[must_use]
pub fn half(block_state_id: i32) -> Option<DoorHalf> {
    if !is_door(block_state_id) {
        return None;
    }
    match super::support::str_property(block_state_id, HALF)? {
        "upper" => Some(DoorHalf::Upper),
        "lower" => Some(DoorHalf::Lower),
        _ => None,
    }
}

///
/// The cell below is read before the branch, exactly as vanilla does
/// (`:242-243`), so both halves cost the same one read.
///
/// `None` is "this slice does not answer", which for the lower half also
/// covers `face_sturdy`'s dynamic-shape gap.
#[must_use]
pub fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    let half = half(block_state_id)?;
    let below_state = block_state_at(below(pos));
    match half {
        DoorHalf::Lower => face_sturdy(below_state, Direction::Up, SupportType::Full),
        // `belowState.is(this)`: the same *block*, not merely another door, so
        // an oak upper half over an iron lower half does not survive.
        DoorHalf::Upper => Some(state_is_block(below_state, block_type_of(block_state_id)?)),
    }
}

pub(super) fn can_survive_live<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
) -> Option<bool> {
    let door_half = half(block_state_id)?;
    let support_pos = below(pos);
    let support = level.block_state(support_pos);
    match door_half {
        DoorHalf::Lower => Some(face_sturdy_at(
            support,
            support_pos,
            Direction::Up,
            SupportType::Full,
        )),
        DoorHalf::Upper => block_type_of(block_state_id)
            .map(|block_type_id| state_is_block(support, block_type_id)),
    }
}

#[derive(Clone, Copy)]
enum DoorSoundFamily {
    Iron,
    Copper,
    CherryWood,
    BambooWood,
    NetherWood,
    Wooden,
}

fn door_sound(block_state_id: i32, opening: bool) -> Option<&'static str> {
    let family = match block_type_of(block_state_id)? {
        VANILLA_BLOCK_IRON_DOOR_ID => DoorSoundFamily::Iron,
        VANILLA_BLOCK_COPPER_DOOR_ID
        | VANILLA_BLOCK_EXPOSED_COPPER_DOOR_ID
        | VANILLA_BLOCK_WEATHERED_COPPER_DOOR_ID
        | VANILLA_BLOCK_OXIDIZED_COPPER_DOOR_ID
        | VANILLA_BLOCK_WAXED_COPPER_DOOR_ID
        | VANILLA_BLOCK_WAXED_EXPOSED_COPPER_DOOR_ID
        | VANILLA_BLOCK_WAXED_WEATHERED_COPPER_DOOR_ID
        | VANILLA_BLOCK_WAXED_OXIDIZED_COPPER_DOOR_ID => DoorSoundFamily::Copper,
        VANILLA_BLOCK_CHERRY_DOOR_ID => DoorSoundFamily::CherryWood,
        VANILLA_BLOCK_BAMBOO_DOOR_ID => DoorSoundFamily::BambooWood,
        VANILLA_BLOCK_CRIMSON_DOOR_ID | VANILLA_BLOCK_WARPED_DOOR_ID => DoorSoundFamily::NetherWood,
        VANILLA_BLOCK_OAK_DOOR_ID
        | VANILLA_BLOCK_SPRUCE_DOOR_ID
        | VANILLA_BLOCK_BIRCH_DOOR_ID
        | VANILLA_BLOCK_JUNGLE_DOOR_ID
        | VANILLA_BLOCK_ACACIA_DOOR_ID
        | VANILLA_BLOCK_DARK_OAK_DOOR_ID
        | VANILLA_BLOCK_PALE_OAK_DOOR_ID
        | VANILLA_BLOCK_MANGROVE_DOOR_ID => DoorSoundFamily::Wooden,
        _ => return None,
    };
    Some(match (family, opening) {
        (DoorSoundFamily::Iron, true) => "minecraft:block.iron_door.open",
        (DoorSoundFamily::Iron, false) => "minecraft:block.iron_door.close",
        (DoorSoundFamily::Copper, true) => "minecraft:block.copper_door.open",
        (DoorSoundFamily::Copper, false) => "minecraft:block.copper_door.close",
        (DoorSoundFamily::CherryWood, true) => "minecraft:block.cherry_wood_door.open",
        (DoorSoundFamily::CherryWood, false) => "minecraft:block.cherry_wood_door.close",
        (DoorSoundFamily::BambooWood, true) => "minecraft:block.bamboo_wood_door.open",
        (DoorSoundFamily::BambooWood, false) => "minecraft:block.bamboo_wood_door.close",
        (DoorSoundFamily::NetherWood, true) => "minecraft:block.nether_wood_door.open",
        (DoorSoundFamily::NetherWood, false) => "minecraft:block.nether_wood_door.close",
        (DoorSoundFamily::Wooden, true) => "minecraft:block.wooden_door.open",
        (DoorSoundFamily::Wooden, false) => "minecraft:block.wooden_door.close",
    })
}

pub fn can_open_by_hand(block_state_id: i32) -> Option<bool> {
    block_type_of(block_state_id).map(|id| id != VANILLA_BLOCK_IRON_DOOR_ID)
}

pub(super) fn can_open_by_wind_charge(block_state_id: i32) -> Option<bool> {
    can_open_by_hand(block_state_id)
}

pub(super) fn use_without_item(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    player: BlockInteractionPlayer,
) -> bool {
    if can_open_by_hand(block_state_id) != Some(true) {
        return false;
    }
    let Some(open) = bool_property(block_state_id, OPEN) else {
        return false;
    };
    set_open(level, pos, block_state_id, player.source_entity_id(), !open)
}

pub(crate) fn set_open(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    source_entity_id: Option<i32>,
    should_open: bool,
) -> bool {
    if !is_door(block_state_id) || bool_property(block_state_id, OPEN) == Some(should_open) {
        return false;
    }
    let opening = should_open;
    let Some(updated) = with_property(block_state_id, OPEN, if opening { "true" } else { "false" })
    else {
        return false;
    };
    let Some(sound) = door_sound(block_state_id, opening) else {
        return false;
    };
    level.set_block(
        pos,
        updated,
        SET_BLOCK_UPDATE_CLIENTS | SET_BLOCK_UPDATE_IMMEDIATE,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    let pitch = level.level_random().next_float() * 0.1 + 0.9;
    // DoorBlock.java:247-248: the acting player predicted its own sound.
    level.play_block_sound_except(pos, sound, 1.0, pitch, source_entity_id);
    level.game_event_from_entity(
        if opening {
            VanillaGameEvent::BLOCK_OPEN
        } else {
            VanillaGameEvent::BLOCK_CLOSE
        },
        pos,
        source_entity_id,
        None,
    );
    true
}

pub(super) fn neighbor_changed(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    changed_block_type_id: i32,
) {
    let signal = level.has_neighbor_signal(pos) || {
        let Some(other_half) = half(block_state_id).map(|half| match half {
            DoorHalf::Lower => relative(pos, Direction::Up),
            DoorHalf::Upper => relative(pos, Direction::Down),
        }) else {
            return;
        };
        level.has_neighbor_signal(other_half)
    };

    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    if changed_block_type_id == block_type_id {
        return;
    }
    let Some(powered) = bool_property(block_state_id, POWERED) else {
        return;
    };
    if signal == powered {
        return;
    }
    let Some(open) = bool_property(block_state_id, OPEN) else {
        return;
    };
    let value = if signal { "true" } else { "false" };
    let Some(updated) = with_property(block_state_id, POWERED, value)
        .and_then(|state| with_property(state, OPEN, value))
    else {
        return;
    };

    if signal != open {
        let Some(sound) = door_sound(block_state_id, signal) else {
            return;
        };
        level.play_block_sound(pos, sound);
        level.block_game_event(
            if signal {
                VanillaGameEvent::BLOCK_OPEN
            } else {
                VanillaGameEvent::BLOCK_CLOSE
            },
            pos,
            None,
        );
    }

    level.set_block(
        pos,
        updated,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

///
/// The guard on `:99` is transcribed as vanilla writes it rather than folded
/// into "the neighbour is the other half", because the folded form hides that
/// `directionToNeighbour == Direction.DOWN` is tested again inside the branch.
///
/// `neighbourState.setValue(HALF, half)` (`:105`) keeps *the neighbour's*
/// facing, hinge, open and powered values and only rewrites `HALF`, which is
/// how a door repairs itself when one half is replaced by a different door
/// state.
pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    let Some(this_half) = half(block_state_id) else {
        // Unreachable through the dispatch, which routes only doors here.
        return block_state_id;
    };
    let axis_is_y = matches!(direction_to_neighbour, Direction::Down | Direction::Up);
    if !axis_is_y || (this_half == DoorHalf::Lower) != (direction_to_neighbour == Direction::Up) {
        // A `None` survival answer keeps the door standing, which is the
        // fail-closed direction: a door vanilla would remove stays, rather
        // than one vanilla would keep being destroyed.
        if this_half == DoorHalf::Lower
            && direction_to_neighbour == Direction::Down
            && can_survive_live(level, block_state_id, pos) == Some(false)
        {
            super::air_block_state_id()
        } else {
            block_state_id
        }
    } else if half(neighbour_state).is_some_and(|neighbour_half| neighbour_half != this_half) {
        with_property(neighbour_state, HALF, this_half.property_value())
            .unwrap_or_else(super::air_block_state_id)
    } else {
        super::air_block_state_id()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HingeSide {
    Left,
    Right,
}

impl HingeSide {
    const fn property_value(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

/// `Direction.getStepX` and `Direction.getStepZ` (`Direction.java:255-265`)
/// off the `normal` each constant carries (`Direction.java:33-38`).
const fn step_x(direction: Direction) -> i32 {
    match direction {
        Direction::West => -1,
        Direction::East => 1,
        _ => 0,
    }
}

const fn step_z(direction: Direction) -> i32 {
    match direction {
        Direction::North => -1,
        Direction::South => 1,
        _ => 0,
    }
}

///
/// The two neighbour columns are read first and only their disagreement is
/// consulted: `solidBlockBalance` leans negative for full blocks on the left
/// and positive for full blocks on the right, and a door already standing on
/// one side outranks it. The click position is the last resort, reached only
/// when both columns say the same thing (`:181-190`).
///
/// `None` is a refusal, not a side: a neighbour whose collision shape reads
/// the level cannot be weighed here, and guessing would put the hinge — and
/// so the whole door's swing — on the wrong side.
fn hinge(
    pos: BlockPos,
    place_direction: Direction,
    click_x: f64,
    click_z: f64,
    block_state_at: BlockStateAt<'_>,
) -> Option<HingeSide> {
    let above_pos = above(pos);
    let left_direction = counter_clock_wise(place_direction);
    let left_pos = relative(pos, left_direction);
    let left_state = block_state_at(left_pos);
    let left_above_state = block_state_at(relative(above_pos, left_direction));
    let right_direction = clock_wise(place_direction);
    let right_pos = relative(pos, right_direction);
    let right_state = block_state_at(right_pos);
    let right_above_state = block_state_at(relative(above_pos, right_direction));
    let solid_block_balance = -i32::from(collision_shape_full_block(left_state)?)
        - i32::from(collision_shape_full_block(left_above_state)?)
        + i32::from(collision_shape_full_block(right_state)?)
        + i32::from(collision_shape_full_block(right_above_state)?);
    let door_left = half(left_state) == Some(DoorHalf::Lower);
    let door_right = half(right_state) == Some(DoorHalf::Lower);
    if (!door_left || door_right) && solid_block_balance <= 0 {
        if (!door_right || door_left) && solid_block_balance >= 0 {
            let step_x = step_x(place_direction);
            let step_z = step_z(place_direction);
            // Transcribed with vanilla's negated comparisons (`:188`), which
            // answer LEFT rather than RIGHT for a click coordinate that is
            // not a number.
            Some(
                if (step_x >= 0 || click_z.partial_cmp(&0.5) != Some(std::cmp::Ordering::Less))
                    && (step_x <= 0
                        || click_z.partial_cmp(&0.5) != Some(std::cmp::Ordering::Greater))
                    && (step_z >= 0
                        || click_x.partial_cmp(&0.5) != Some(std::cmp::Ordering::Greater))
                    && (step_z <= 0 || click_x.partial_cmp(&0.5) != Some(std::cmp::Ordering::Less))
                {
                    HingeSide::Left
                } else {
                    HingeSide::Right
                },
            )
        } else {
            Some(HingeSide::Left)
        }
    } else {
        Some(HingeSide::Right)
    }
}

///
/// A door faces the way the player faces, not the opposite: `FACING` is
/// `getHorizontalDirection` unchanged (`:145`).
#[must_use]
pub fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    horizontal_direction: Direction,
    click_x: f64,
    click_z: f64,
    block_state_at: BlockStateAt<'_>,
    sources: redstone::SignalSources<'_>,
) -> Option<i32> {
    // `:143`.
    let powered = redstone::has_neighbor_signal(pos, &mut sources.emitting(&mut *block_state_at))
        || redstone::has_neighbor_signal(above(pos), &mut sources.emitting(&mut *block_state_at));
    let hinge = hinge(
        pos,
        horizontal_direction,
        click_x,
        click_z,
        &mut *block_state_at,
    )?;
    let powered_value = if powered { "true" } else { "false" };
    let state = with_property(
        default_block_state_id(block_type_id),
        FACING,
        super::direction_serialized_name(horizontal_direction),
    )?;
    let state = with_property(state, HINGE, hinge.property_value())?;
    let state = with_property(state, POWERED, powered_value)?;
    let state = with_property(state, OPEN, powered_value)?;
    with_property(state, HALF, DoorHalf::Lower.property_value())
}

#[must_use]
pub fn upper_state_after_placement(block_state_id: i32) -> Option<i32> {
    upper_half(block_state_id)
}

pub(super) fn set_placed_by(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if let Some(upper) = upper_half(block_state_id) {
        level.set_block(
            above(pos),
            upper,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

fn upper_half(block_state_id: i32) -> Option<i32> {
    if half(block_state_id) != Some(DoorHalf::Lower) {
        return None;
    }
    with_property(block_state_id, HALF, DoorHalf::Upper.property_value())
}
