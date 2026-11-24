//! Random block positions and generated random-tick state predicates.

#[cfg(test)]
use bbb_protocol::block_state_types::{
    vanilla_block_state_fluid_randomly_ticking, vanilla_block_state_randomly_ticking,
};
use bbb_protocol::packets::BlockPos;

use crate::random::GameRandom;

/// `GameRules.RANDOM_TICK_SPEED`'s default
/// (`GameRules.java:74`: `registerInteger("random_tick_speed",
/// GameRuleCategory.UPDATES, 3, 0)` — 3 is the default, 0 the minimum).
#[cfg(test)]
pub const DEFAULT_RANDOM_TICK_SPEED: i32 = 3;

/// The same rule's minimum (`GameRules.java:74`). Zero is legal and switches
/// random ticking off entirely (`ServerLevel.java:496`).
#[cfg(test)]
pub const MIN_RANDOM_TICK_SPEED: i32 = 0;

/// The `yMask` `tickChunk` passes to `getBlockRandomPos`
/// (`ServerLevel.java:506`), which keeps the drawn y inside the 16-block
/// section it was drawn for.
pub const RANDOM_TICK_Y_MASK: i32 = 15;

/// Blocks per section side (`SectionPos.sectionToBlockCoord`,
/// `SectionPos.java:127-129`, shifts by 4).
pub const SECTION_SIDE: i32 = 16;

/// `Level.getBlockRandomPos`: a uniform block of the 16x16 column at
/// `xo, zo`, `y_mask` bounding the height above `yo`.
pub fn random_block_pos(random: &GameRandom, xo: i32, yo: i32, zo: i32, y_mask: i32) -> BlockPos {
    let bits = random.next_u64();
    BlockPos {
        x: xo + (bits & 15) as i32,
        y: yo + ((bits >> 8) as i32 & y_mask),
        z: zo + ((bits >> 4) & 15) as i32,
    }
}

/// `BlockState.isRandomlyTicking()` for a stored global state id, with the
/// out-of-registry policy `crate::chunk` already settled on: a synthetic id
/// has no vanilla behaviour to attribute to it, so it never ticks.
#[cfg(test)]
pub fn state_randomly_ticks(state: i32) -> bool {
    vanilla_block_state_randomly_ticking(state).unwrap_or(false)
}

/// `BlockState.getFluidState().isRandomlyTicking()`; see
/// [`state_randomly_ticks`].
#[cfg(test)]
pub fn state_fluid_randomly_ticks(state: i32) -> bool {
    vanilla_block_state_fluid_randomly_ticking(state).unwrap_or(false)
}

#[cfg(test)]
mod tests;
