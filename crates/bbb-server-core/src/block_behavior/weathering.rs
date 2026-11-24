//! `ChangeOverTimeBlock` (`net/minecraft/world/level/block/ChangeOverTimeBlock.java`)
//! and its one 26.1 chain, `WeatheringCopper` (`WeatheringCopper.java`).
//!
//!
//! # Two draws, in order
//!
//!
//! # The scan
//!
//!
//! Inside the shell the scan skips the position itself, ignores anything that
//! is not a `ChangeOverTimeBlock` (`:36`), and refuses to consider a neighbour
//! from a different chain (`:38`). In 26.1 there is only one chain --
//! `WeatheringCopper` is the sole `ChangeOverTimeBlock` and its age is always a
//! `WeatheringCopper.WeatherState` -- and the block-catalog generator fails
//! rather than emit an age column if that stops being true, so the class
//! comparison is a comparison every neighbour here passes.
//!
//! # What is deliberately not here
//!

use super::LevelCtx;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_property_values,
    vanilla_block_state_id_by_name_and_properties, vanilla_block_state_identity,
};
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaRandomTickOverride, VANILLA_BLOCK_TYPES,
};
use bbb_protocol::item_types::VANILLA_AXE_WAX_OFF;
use bbb_protocol::packets::BlockPos;

use crate::fluid::SET_BLOCK_UPDATE_ALL;

use super::chest;
use super::door::{self, DoorHalf};
use super::{block_type_of, random_tick_override, BlockBehaviorOwner};

/// `ChangeOverTimeBlock.SCAN_DISTANCE` (`ChangeOverTimeBlock.java:10`).
pub const SCAN_DISTANCE: i32 = 4;

/// `changeOverTime`'s `eachBlockOncePerDayChance`
/// (`ChangeOverTimeBlock.java:17-18`).
///
/// The local is dead in the bytecode -- the comparison on `:18` spells the
/// literal again -- but the name is the only statement anywhere of what the
/// number means, so it is kept.
pub const EACH_BLOCK_ONCE_PER_DAY_CHANCE: f32 = 0.056_888_89;

/// `WeatheringCopper.getChanceModifier` (`WeatheringCopper.java:95-97`) for a
/// block still at `WeatherState.UNAFFECTED`: the first step is the slow one.
pub const UNAFFECTED_CHANCE_MODIFIER: f32 = 0.75;

/// The same method for every later age.
pub const WEATHERED_CHANCE_MODIFIER: f32 = 1.0;

/// `WeatheringCopper.WeatherState.UNAFFECTED.ordinal()`
/// (`WeatheringCopper.java:99-103`).
const UNAFFECTED_AGE: i32 = 0;

/// The behaviour owner a stored state belongs to, or `None` when it is not a
/// `ChangeOverTimeBlock` this slice carries.
///
#[must_use]
pub(super) fn weathering_owner(block_state_id: i32) -> Option<BlockBehaviorOwner> {
    age(block_state_id)?;
    match random_tick_override(block_type_of(block_state_id)?)? {
        VanillaRandomTickOverride::WeatheringCopperBarsBlock => {
            Some(BlockBehaviorOwner::WeatheringCopperBars)
        }
        VanillaRandomTickOverride::WeatheringCopperChainBlock => {
            Some(BlockBehaviorOwner::WeatheringCopperChain)
        }
        VanillaRandomTickOverride::WeatheringCopperSlabBlock => {
            Some(BlockBehaviorOwner::WeatheringCopperSlab)
        }
        VanillaRandomTickOverride::WeatheringCopperStairBlock => {
            Some(BlockBehaviorOwner::WeatheringCopperStair)
        }
        VanillaRandomTickOverride::WeatheringCopperGolemStatueBlock => {
            Some(BlockBehaviorOwner::WeatheringCopperGolemStatue)
        }
        VanillaRandomTickOverride::WeatheringCopperGrateBlock => {
            Some(BlockBehaviorOwner::WeatheringCopperGrate)
        }
        VanillaRandomTickOverride::WeatheringCopperDoorBlock => {
            Some(BlockBehaviorOwner::WeatheringCopperDoor)
        }
        VanillaRandomTickOverride::WeatheringCopperTrapDoorBlock => {
            Some(BlockBehaviorOwner::WeatheringCopperTrapDoor)
        }
        VanillaRandomTickOverride::WeatheringLanternBlock => {
            Some(BlockBehaviorOwner::WeatheringCopperLantern)
        }
        // The earlier CopperChest row also owns waxed chests that have no age.
        _ => Some(BlockBehaviorOwner::Weathering),
    }
}

/// `ChangeOverTimeBlock.getAge().ordinal()` for a stored state, or `None` when
/// the block is not a `ChangeOverTimeBlock`.
///
/// The age is a constructor argument rather than a block-state property, which
/// is why it comes off the block catalog and not off the state.
#[must_use]
pub fn age(block_state_id: i32) -> Option<i32> {
    vanilla_block_type(block_type_of(block_state_id)?)?.change_over_time_age
}

#[must_use]
fn next_block_type(block_state_id: i32) -> Option<i32> {
    vanilla_block_type(block_type_of(block_state_id)?)?.change_over_time_next_block_type_id
}

///
/// Not read by the dispatch -- [`crate::random_ticks`] answers that gate from
/// the generated per-state fact -- but pinned against that fact by this
/// module's tests, so the two spellings cannot drift apart.
#[must_use]
pub fn is_randomly_ticking(block_state_id: i32) -> bool {
    next_block_type(block_state_id).is_some()
}

#[must_use]
fn chance_modifier(own_age: i32) -> f32 {
    if own_age == UNAFFECTED_AGE {
        UNAFFECTED_CHANCE_MODIFIER
    } else {
        WEATHERED_CHANCE_MODIFIER
    }
}

#[must_use]
pub fn next_state(block_state_id: i32) -> Option<i32> {
    with_properties_of(next_block_type(block_state_id)?, block_state_id)
}

///
/// Vanilla's `result.hasProperty(property)` is identity on the shared static
/// `Property` instance, so a matching *name* is not enough -- `half` is both
/// `BlockStateProperties.DOUBLE_BLOCK_HALF` and `BlockStateProperties.HALF`,
/// over different values. Two blocks are taken to share a property here when
/// they spell the same name over the same ordered value list, which is what
/// distinguishes those two.
#[must_use]
pub(crate) fn with_properties_of(target_block_type_id: i32, source_state_id: i32) -> Option<i32> {
    let target_name = vanilla_block_type(target_block_type_id)?.resource_id;
    let source = vanilla_block_state_identity(source_state_id)?;
    let default_state_id = vanilla_block_default_state_id(target_block_type_id)?;
    let mut properties = vanilla_block_state_identity(default_state_id)?
        .properties
        .clone();
    for (name, value) in &source.properties {
        if vanilla_block_property_values(target_name, name)
            != vanilla_block_property_values(&source.name, name)
        {
            continue;
        }
        properties.insert(name.clone(), value.clone());
    }
    vanilla_block_state_id_by_name_and_properties(target_name, &properties)
}

///
/// `None` is `Optional.empty()`, which reaches the caller as "write nothing".
/// It has three sources here and they are not the same thing: a neighbour
/// younger than this block (`:40-42`, and the scan stops there without
/// drawing), a failed second draw (`:55`), and a block already at the end of
/// its chain (`getNext` answering empty).
fn get_next_state(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) -> Option<i32> {
    let own_age = age(block_state_id)?;
    let mut same_age_count = 0;
    let mut older_count = 0;
    for scan_pos in within_manhattan(pos, SCAN_DISTANCE, SCAN_DISTANCE, SCAN_DISTANCE) {
        if dist_manhattan(scan_pos, pos) > SCAN_DISTANCE {
            break;
        }
        if scan_pos == pos {
            continue;
        }
        // `level.getBlockState(blockPos).getBlock() instanceof
        // ChangeOverTimeBlock<?>` plus the `getAge().getClass()` test on `:38`
        // folded into one question, because in 26.1 every `ChangeOverTimeBlock`
        // ages over the same enum; see the module header.
        let Some(found_age) = age(level.block_state(scan_pos)) else {
            continue;
        };
        if found_age < own_age {
            return None;
        }
        if found_age > own_age {
            older_count += 1;
        } else {
            same_age_count += 1;
        }
    }
    let chance = (older_count + 1) as f32 / (older_count + same_age_count + 1) as f32;
    let actual_chance = chance * chance * chance_modifier(own_age);
    if level.level_random().next_float() < actual_chance {
        next_state(block_state_id)
    } else {
        None
    }
}

pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if level.level_random().next_float() < EACH_BLOCK_ONCE_PER_DAY_CHANCE {
        if let Some(weathered_state) = get_next_state(level, pos, block_state_id) {
            level.set_block(
                pos,
                weathered_state,
                SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
    }
}

///
/// Guard refusal consumes no random draw; `age` excludes waxed registrations.
pub(super) fn chest_random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if age(block_state_id).is_none()
        || chest::chest_type_is_right(block_state_id)
        || !level.has_container(pos)
        || level.chest_open_count(pos) != 0
    {
        return;
    }
    random_tick(level, pos, block_state_id);
}

///
/// The upper half is still *randomly ticking* -- `isRandomlyTicking`
/// (`:42-44`) does not look at `HALF` -- so it is sampled and then does
/// nothing, drawing no numbers at all.
pub(super) fn door_random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if door::half(block_state_id) == Some(DoorHalf::Lower) {
        random_tick(level, pos, block_state_id);
    }
}

#[must_use]
pub fn previous_state(block_state_id: i32) -> Option<i32> {
    let block_type_id = block_type_of(block_state_id)?;
    let previous = VANILLA_BLOCK_TYPES
        .iter()
        .find(|block| block.change_over_time_next_block_type_id == Some(block_type_id))?;
    with_properties_of(previous.id, block_state_id)
}

#[must_use]
pub fn first_state(block_state_id: i32) -> i32 {
    let mut state = block_state_id;
    while let Some(previous) = previous_state(state) {
        state = previous;
    }
    state
}

#[must_use]
pub fn is_waxed(block_state_id: i32) -> bool {
    block_type_of(block_state_id).is_some_and(|block_type_id| {
        VANILLA_AXE_WAX_OFF
            .iter()
            .any(|transform| transform.source_block_type_id == block_type_id)
    })
}

///
/// Vanilla sums the three absolute differences as `float` and truncates back
/// to `int`; the sum here is at most twelve, so the widening is exact and the
/// arithmetic stays integral.
#[must_use]
pub fn dist_manhattan(a: BlockPos, b: BlockPos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs()
}

///
/// Transcribed field for field from the `AbstractIterator` vanilla returns,
/// including the mirrored second visit of every position whose `z` offset is
/// non-zero, because the order it produces is what makes the `break` in
/// [`get_next_state`] correct: positions come out in non-decreasing Manhattan
/// distance from the origin, starting with the origin itself.
///
/// The reach arguments bound each axis independently and `maxDepth` is their
/// sum, so an iterator run to exhaustion visits the whole box's Manhattan
/// diamond, not just the shells a single reach allows.
#[must_use]
pub fn within_manhattan(
    origin: BlockPos,
    reach_x: i32,
    reach_y: i32,
    reach_z: i32,
) -> WithinManhattan {
    WithinManhattan {
        origin,
        reach_x,
        reach_y,
        reach_z,
        max_depth: reach_x + reach_y + reach_z,
        cursor: origin,
        current_depth: 0,
        max_x: 0,
        max_y: 0,
        x: 0,
        y: 0,
        z_mirror: false,
        finished: false,
    }
}

/// The iterator [`within_manhattan`] builds.
pub struct WithinManhattan {
    origin: BlockPos,
    reach_x: i32,
    reach_y: i32,
    reach_z: i32,
    max_depth: i32,
    /// `MutableBlockPos cursor`, whose `z` the mirror step rewrites in place.
    cursor: BlockPos,
    current_depth: i32,
    max_x: i32,
    max_y: i32,
    x: i32,
    y: i32,
    z_mirror: bool,
    /// `AbstractIterator.endOfData()` is terminal; a Rust `Iterator` is not
    /// required to be, so the state is kept explicitly.
    finished: bool,
}

impl Iterator for WithinManhattan {
    type Item = BlockPos;

    fn next(&mut self) -> Option<BlockPos> {
        if self.finished {
            return None;
        }
        if self.z_mirror {
            self.z_mirror = false;
            self.cursor.z = self.origin.z - (self.cursor.z - self.origin.z);
            return Some(self.cursor);
        }
        loop {
            if self.y > self.max_y {
                self.x += 1;
                if self.x > self.max_x {
                    self.current_depth += 1;
                    if self.current_depth > self.max_depth {
                        self.finished = true;
                        return None;
                    }
                    self.max_x = self.reach_x.min(self.current_depth);
                    self.x = -self.max_x;
                }
                self.max_y = self.reach_y.min(self.current_depth - self.x.abs());
                self.y = -self.max_y;
            }
            let offset_x = self.x;
            let offset_y = self.y;
            let offset_z = self.current_depth - offset_x.abs() - offset_y.abs();
            // The `this.y++` step of vanilla's `for`, which runs after the body
            // even on the iteration that found a position.
            self.y += 1;
            if offset_z <= self.reach_z {
                self.z_mirror = offset_z != 0;
                self.cursor = BlockPos {
                    x: self.origin.x + offset_x,
                    y: self.origin.y + offset_y,
                    z: self.origin.z + offset_z,
                };
                return Some(self.cursor);
            }
        }
    }
}
