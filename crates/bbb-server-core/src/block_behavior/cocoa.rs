//! `CocoaBlock` (`net/minecraft/world/level/block/CocoaBlock.java`).
//!
//! A pod hung on the side of a jungle log: `FACING` says which way it looks,
//! `AGE` counts 0..2, and a one-in-five draw ages it (`:52-59`). It is the one
//! member of this family whose support is horizontal, so both `canSurvive`
//! (`:62-65`) and `updateShape` (`:91-104`) read the cell `FACING` points at
//! rather than the one below.
//!
//! # `level.getRandom()` is the argument
//!
//!
//! # What is not here
//!
//! * The bonemeal triple (`:107-119`), which belongs with the bonemeal slice
//!   exactly as `CropBlock`'s does.
//! * `getShape` (`:68-70`) and `isPathfindable` (`:127-129`), collision
//!   surfaces.

use super::BlockTags;
use super::{block_tag_matches, BlockGetter, BlockShapeCtx, LevelCtx};
use bbb_protocol::block_types::{VanillaGetStateForPlacementOverride, VanillaRandomTickOverride};
use bbb_protocol::packets::{BlockPos, Direction};

use bbb_server_types::directions::relative;
use bbb_server_types::fluid::air_block_state_id;

use super::{
    block_type_of, default_block_state_id, direction_property, direction_serialized_name,
    get_state_for_placement_override, int_property, random_tick_override, with_property,
    BlockStateAt, SET_BLOCK_UPDATE_CLIENTS,
};

/// The property `CocoaBlock.AGE` names (`CocoaBlock.java:31`,
/// `BlockStateProperties.AGE_2` = `IntegerProperty.create("age", 0, 2)`,
/// `BlockStateProperties.java:92`).
const AGE: &str = "age";

/// The property `HorizontalDirectionalBlock.FACING` names
/// (`HorizontalDirectionalBlock.java:11`,
/// `BlockStateProperties.HORIZONTAL_FACING` = `EnumProperty.create("facing",
/// …)`, `BlockStateProperties.java:57`).
const FACING: &str = "facing";
const SUPPORTS_COCOA_TAG: &str = "minecraft:supports_cocoa";

/// `CocoaBlock.MAX_AGE` (`CocoaBlock.java:30`).
///
/// It is also `isRandomlyTicking` (`:47-49`), so the scheduler never brings a
/// ripe pod here.
pub const MAX_AGE: i32 = 2;

/// The bound of the growth draw (`CocoaBlock.java:53`).
pub const GROWTH_DRAW_BOUND: i32 = 5;

/// Whether a stored state's behaviour belongs to `CocoaBlock`.
///
/// `CocoaBlock` has no subclass in 26.1, so the generated
/// `random_tick_override` class tag names the class outright.
#[must_use]
pub fn is_cocoa(block_state_id: i32) -> bool {
    block_type_of(block_state_id).is_some_and(|block_type_id| {
        random_tick_override(block_type_id) == Some(VanillaRandomTickOverride::CocoaBlock)
    })
}

///
/// The draw comes *before* the age test here, the other way round from
/// [`super::nether_wart`], so a ripe pod that somehow reaches this body still
/// consumes one number.
pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    if level.level_random().next_int(GROWTH_DRAW_BOUND) != 0 {
        return;
    }
    let Some(age) = int_property(block_state_id, AGE) else {
        return;
    };
    if age >= MAX_AGE {
        return;
    }
    // `state.setValue(AGE, age + 1)` (`:56`) keeps `FACING`, so this is
    // `with_property` on the standing state rather than a fresh state built
    // from one property.
    if let Some(grown) = with_property(block_state_id, AGE, &(age + 1).to_string()) {
        level.set_block(
            pos,
            grown,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

///
/// `None` keeps an unreadable facing or support state fail closed.
#[must_use]
pub fn can_survive(
    block_state_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<bool> {
    let facing = direction_property(block_state_id, FACING)?;
    let support = block_state_at(relative(pos, facing));
    block_type_of(support)?;
    Some(tags.block_in_tag(support, "minecraft:supports_cocoa"))
}

fn can_survive_at<L: BlockGetter + ?Sized>(
    level: &L,
    block_state_id: i32,
    pos: BlockPos,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    let facing = direction_property(block_state_id, FACING)?;
    let support = level.block_state(relative(pos, facing));
    block_type_of(support)?;
    Some(block_tag_matches(support, SUPPORTS_COCOA_TAG, membership))
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    directions: [Direction; 6],
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::CocoaBlock)
    {
        return None;
    }
    let mut state = default_block_state_id(block_type_id);
    for direction in directions {
        if matches!(direction, Direction::Down | Direction::Up) {
            continue;
        }
        state = with_property(state, FACING, direction_serialized_name(direction))?;
        match can_survive(state, pos, tags, block_state_at) {
            Some(true) => return Some(state),
            Some(false) => {}
            None => return None,
        }
    }
    None
}

///
/// The only body in this slice that gates the survival test on the direction
/// the update came from: a neighbour change on any other side leaves the pod
/// alone even when the log it hangs on is already gone, and it takes an update
/// from `FACING` to notice.
pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if direction_property(block_state_id, FACING) != Some(direction_to_neighbour) {
        return block_state_id;
    }
    // Missing live membership keeps the stored state.
    if can_survive_at(level, block_state_id, pos, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    })
    .unwrap_or(true)
    {
        block_state_id
    } else {
        air_block_state_id()
    }
}
