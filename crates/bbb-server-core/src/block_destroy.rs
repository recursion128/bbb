//! `ServerPlayerGameMode`'s destroy-progress machinery: the arithmetic
//! `BlockBehaviour.getDestroyProgress` (`BlockBehaviour.java:355-363`) does and
//! the per-session state the three `handleBlockBreakAction` branches
//! (`ServerPlayerGameMode.java:150-251`) and `tick` (`:107-130`) keep between
//! packets.
//!
//! The world-touching half -- the packets, the block write and the tick call --
//! lives beside the rest of the session handling in `crate::world`; only what
//! can be answered from a block state, a held stack and a session's own fields
//! lives here.
//!
//! ## The two calls the START branch does not make
//!
//! `EnchantmentHelper.onHitBlock` (`ServerPlayerGameMode.java:187-196`) would
//! break the held item's `EquipmentSlot.MAINHAND` copy through an enchantment
//! effect; this server models no enchantment effects and no item damage, so the
//! call has nothing to reach.
//!
//! `blockState.attack(..)` (`:197`) is a divergence. 26.1 has three overrides of
//! `BlockBehaviour.attack` (`BlockBehaviour.java:368-369` is the empty default):
//! `RedStoneOreBlock` (`RedStoneOreBlock.java:39-42`), `DragonEggBlock`
//! (`DragonEggBlock.java:47-49`) and `NoteBlock` (`NoteBlock.java:136-141`).
//! None of the three blocks has an owner in [`crate::block_behavior`], so
//! punching one of them here lights no redstone ore, teleports no dragon egg and
//! plays no note. It only reaches a restored save: the flat fixture generates
//! none of them.

use bbb_protocol::{
    block_state_types::{
        vanilla_block_state_block_type_id, vanilla_block_state_destroy_speed,
        vanilla_block_state_requires_correct_tool_for_drops,
    },
    packets::{BlockPos, ItemStackSummary},
};

use crate::inventory::{item_destroy_speed, item_is_correct_tool_for_drops};
use crate::registry_tags::RegistryTagSnapshot;
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::{Result, ServerWorldError};

/// The stage `ServerLevel.destroyBlockProgress` is given to clear a break
/// (`ServerPlayerGameMode.java:123`, `:224`, `:243`, `:247`) and the value
/// `lastSentState` starts at (`:48`).
pub const NO_DESTROY_STAGE: i32 = -1;

/// The acceptance threshold `STOP_DESTROY_BLOCK` compares against
/// (`ServerPlayerGameMode.java:222`).
pub const STOP_DESTROY_PROGRESS_THRESHOLD: f32 = 0.7;

/// `ServerLevel.destroyBlockProgress`'s radius (`ServerLevel.java:1015`), which
/// is a squared distance from the block position to the player position.
pub const DESTROY_PROGRESS_RANGE_SQR: f64 = 1024.0;

/// `ServerPlayerGameMode`'s destroy-progress fields
/// (`ServerPlayerGameMode.java:41-48`), one set per session.
///
/// `gameTicks` counts this session's own `tick()` calls (`:108`) rather than the
/// world's, which is why it lives here and not beside the world tick counter: a
/// session that joined late starts from zero, and every comparison the machinery
/// makes is a difference between two of its own readings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestroyProgress {
    is_destroying_block: bool,
    destroy_progress_start: i32,
    destroy_pos: BlockPos,
    game_ticks: i32,
    has_delayed_destroy: bool,
    delayed_destroy_pos: BlockPos,
    delayed_tick_start: i32,
    last_sent_state: i32,
}

impl Default for DestroyProgress {
    /// The field initialisers at `ServerPlayerGameMode.java:41-48`:
    /// `BlockPos.ZERO` for both positions and `-1` for `lastSentState`, which is
    /// a value no `(int)(progress * 10.0F)` can take and therefore forces the
    /// first stage of a break to be broadcast.
    fn default() -> Self {
        Self {
            is_destroying_block: false,
            destroy_progress_start: 0,
            destroy_pos: BlockPos { x: 0, y: 0, z: 0 },
            game_ticks: 0,
            has_delayed_destroy: false,
            delayed_destroy_pos: BlockPos { x: 0, y: 0, z: 0 },
            delayed_tick_start: 0,
            last_sent_state: NO_DESTROY_STAGE,
        }
    }
}

/// What one `ServerPlayerGameMode.tick` (`ServerPlayerGameMode.java:107-130`)
/// leaves for the world to do.
///
/// The call is split from the block read because both of vanilla's arms read the
/// live block state and one of them writes it, while the session state and the
/// world are two different borrows on this side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestroyTickStep {
    /// Neither arm ran (`:109` and `:120` both false).
    Idle,
    /// The block is still there. `stage` is `Some` when
    /// `incrementDestroyProgress` (`:132-142`) found a stage different from
    /// `lastSentState` and therefore broadcast it (`:136-139`), and `destroy` is
    /// the delayed arm's `destroyProgress >= 1.0F` (`:115-118`).
    Progress {
        pos: BlockPos,
        stage: Option<i32>,
        destroy: bool,
    },
    /// The block being destroyed turned to air under the session, which clears
    /// the broadcast stage (`:122-125`).
    Cleared { pos: BlockPos },
}

/// What `handleBlockBreakAction`'s STOP branch decided (`:216-238`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopOutcome {
    /// `:222-226`: the stage is cleared and the block destroyed.
    Destroy,
    /// `:229-234`: the break is handed to the delayed arm of `tick`.
    Delayed,
    /// The `!hasDelayedDestroy` guard (`:229`) refused, so nothing moved.
    Kept,
}

/// The positions ABORT clears (`ServerPlayerGameMode.java:239-249`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbortOutcome {
    /// The position the session thought it was destroying, when the packet named
    /// a different one (`:241-245`).
    pub mismatched: Option<BlockPos>,
    /// The position the packet named (`:247`), cleared either way.
    pub pos: BlockPos,
}

impl DestroyProgress {
    pub fn destroy_pos(&self) -> BlockPos {
        self.destroy_pos
    }

    /// The position this tick's arm will read, before the world reads it.
    ///
    /// It answers the same two fields `tick` branches on (`:109`, `:120`) and
    /// nothing between the two calls writes either, so the position it names is
    /// the position the arm goes on to use.
    pub fn tick_target(&self) -> Option<BlockPos> {
        if self.has_delayed_destroy {
            return Some(self.delayed_destroy_pos);
        }
        self.is_destroying_block.then_some(self.destroy_pos)
    }

    /// `ServerPlayerGameMode.tick` (`ServerPlayerGameMode.java:107-130`).
    ///
    /// `block_state_id` is what the world found at [`Self::tick_target`], with
    /// `None` for a position outside the world -- which is the void air
    /// `Level.getBlockState` answers there and therefore the `isAir()` branch
    /// (`:111`, `:122`).
    ///
    /// `gameTicks` advances first (`:108`) whichever arm runs, including the arm
    /// that runs nothing, because every `ticksSpentDestroying` is measured
    /// against it.
    pub fn tick(
        &mut self,
        block_state_id: Option<i32>,
        item: &ItemStackSummary,
        facts: PlayerDestroySpeedFacts,
        tags: &RegistryTagSnapshot,
    ) -> Result<DestroyTickStep> {
        self.game_ticks += 1;
        let standing = block_state_id.filter(|id| !block_state_is_air(*id));
        if self.has_delayed_destroy {
            let pos = self.delayed_destroy_pos;
            let Some(block_state_id) = standing else {
                // `:110-112`: the delayed arm drops the break without clearing a
                // stage, because the `STOP_DESTROY_BLOCK` that armed it
                // (`:229-234`) broadcast none of its own.
                self.has_delayed_destroy = false;
                return Ok(DestroyTickStep::Idle);
            };
            let start = self.delayed_tick_start;
            let (progress, stage) =
                self.increment_destroy_progress(block_state_id, start, item, facts, tags)?;
            let destroy = progress >= 1.0;
            if destroy {
                self.has_delayed_destroy = false;
            }
            return Ok(DestroyTickStep::Progress {
                pos,
                stage,
                destroy,
            });
        }
        if !self.is_destroying_block {
            return Ok(DestroyTickStep::Idle);
        }
        let pos = self.destroy_pos;
        let Some(block_state_id) = standing else {
            // `:122-125`.
            self.last_sent_state = NO_DESTROY_STAGE;
            self.is_destroying_block = false;
            return Ok(DestroyTickStep::Cleared { pos });
        };
        let start = self.destroy_progress_start;
        let (_, stage) =
            self.increment_destroy_progress(block_state_id, start, item, facts, tags)?;
        Ok(DestroyTickStep::Progress {
            pos,
            stage,
            destroy: false,
        })
    }

    /// `ServerPlayerGameMode.incrementDestroyProgress`
    /// (`ServerPlayerGameMode.java:132-142`), returning both the progress the
    /// caller compares against `1.0F` and the stage to broadcast -- `None` when
    /// it equals the one already sent (`:136`).
    pub fn increment_destroy_progress(
        &mut self,
        block_state_id: i32,
        destroy_start_tick: i32,
        item: &ItemStackSummary,
        facts: PlayerDestroySpeedFacts,
        tags: &RegistryTagSnapshot,
    ) -> Result<(f32, Option<i32>)> {
        let ticks_spent_destroying = self.game_ticks - destroy_start_tick;
        let destroy_progress = block_destroy_progress(item, block_state_id, facts, tags)?
            * ticks_spent_destroying.saturating_add(1) as f32;
        let stage = destroy_stage(destroy_progress);
        let broadcast = (stage != self.last_sent_state).then(|| {
            self.last_sent_state = stage;
            stage
        });
        Ok((destroy_progress, broadcast))
    }

    /// `destroyProgressStart = this.gameTicks` (`:183`), which vanilla writes
    /// before it knows whether the break will be insta-mined.
    pub fn begin_progress_window(&mut self) {
        self.destroy_progress_start = self.game_ticks;
    }

    /// The else arm of the START branch (`:204-214`): the switch to
    /// `isDestroyingBlock` and the first broadcast stage.
    ///
    /// The `Option` is the position whose stage has to be put back on the client
    /// because a second break started while the first was live (`:204-207`).
    pub fn start(&mut self, pos: BlockPos, progress: f32) -> (Option<BlockPos>, i32) {
        let interrupted = self.is_destroying_block.then_some(self.destroy_pos);
        self.is_destroying_block = true;
        self.destroy_pos = pos;
        let stage = destroy_stage(progress);
        self.last_sent_state = stage;
        (interrupted, stage)
    }

    /// `handleBlockBreakAction`'s STOP branch (`:217-235`) for the position it
    /// is already destroying, once the caller has established that the block is
    /// not air (`:220`).
    pub fn stop(
        &mut self,
        block_state_id: i32,
        item: &ItemStackSummary,
        facts: PlayerDestroySpeedFacts,
        tags: &RegistryTagSnapshot,
    ) -> Result<StopOutcome> {
        let ticks_spent_destroying = self.game_ticks - self.destroy_progress_start;
        let destroy_progress = block_destroy_progress(item, block_state_id, facts, tags)?
            * ticks_spent_destroying.saturating_add(1) as f32;
        if destroy_progress >= STOP_DESTROY_PROGRESS_THRESHOLD {
            self.is_destroying_block = false;
            return Ok(StopOutcome::Destroy);
        }
        if !self.has_delayed_destroy {
            self.is_destroying_block = false;
            self.has_delayed_destroy = true;
            self.delayed_destroy_pos = self.destroy_pos;
            self.delayed_tick_start = self.destroy_progress_start;
            return Ok(StopOutcome::Delayed);
        }
        Ok(StopOutcome::Kept)
    }

    /// `handleBlockBreakAction`'s ABORT branch (`:239-249`).
    ///
    /// Both `destroyBlockProgress` calls happen once the positions disagree
    /// (`:243` then `:247`), so a mismatched abort clears two positions rather
    /// than one. `hasDelayedDestroy` is deliberately untouched: vanilla's abort
    /// never looks at it, so a break already handed to the delayed arm keeps
    /// running.
    pub fn abort(&mut self, pos: BlockPos) -> AbortOutcome {
        self.is_destroying_block = false;
        AbortOutcome {
            mismatched: (self.destroy_pos != pos).then_some(self.destroy_pos),
            pos,
        }
    }
}

/// `(int)(destroyProgress * 10.0F)` (`ServerPlayerGameMode.java:135`, `:211`).
///
/// Java's `(int)` narrowing of a float truncates toward zero and saturates at
/// the integer bounds, which is what `as` does here. No input can be NaN: every
/// factor of `destroyProgress` is finite, and the two the client controls -- the
/// held stack's mining speed and the tick count -- are both bounded.
pub fn destroy_stage(destroy_progress: f32) -> i32 {
    (destroy_progress * 10.0) as i32
}

/// `BlockBehaviour.getDestroyProgress` (`BlockBehaviour.java:355-363`), the one
/// implementation 26.1 has: no block overrides it, and no block overrides the
/// `BlockStateBase.getDestroySpeed` (`:639-641`) it reads.
///
/// The `-1.0F` early return (`:356-359`) is why an unbreakable block never
/// reaches the division, and the `30`/`100` divisor (`:361`) is an `int` vanilla
/// widens to `float` for the second division.
pub fn block_destroy_progress(
    item: &ItemStackSummary,
    block_state_id: i32,
    facts: PlayerDestroySpeedFacts,
    tags: &RegistryTagSnapshot,
) -> Result<f32> {
    let destroy_speed = vanilla_block_state_destroy_speed(block_state_id)
        .or_else(|| {
            tags.content_registry()
                .block_by_state_id(block_state_id)
                .map(|(block, _)| block.destroy_speed)
        })
        .ok_or(ServerWorldError::MissingCatalogFact(
            "block state destroy speed",
        ))?;
    if destroy_speed == -1.0 {
        return Ok(0.0);
    }
    let modifier = if has_correct_tool_for_drops(item, block_state_id, tags)? {
        30.0
    } else {
        100.0
    };
    Ok(player_destroy_speed(item, block_state_id, facts, tags)? / destroy_speed / modifier)
}

/// `Player.hasCorrectToolForDrops` (`Player.java:618-620`): a block that does not
/// require the correct tool is correctly tooled whatever the hand holds.
pub fn has_correct_tool_for_drops(
    item: &ItemStackSummary,
    block_state_id: i32,
    tags: &RegistryTagSnapshot,
) -> Result<bool> {
    let requires_correct = vanilla_block_state_requires_correct_tool_for_drops(block_state_id)
        .or_else(|| {
            tags.content_registry()
                .block_by_state_id(block_state_id)
                .map(|_| false)
        })
        .ok_or(ServerWorldError::MissingCatalogFact(
            "block state correct-tool requirement",
        ))?;
    if !requires_correct {
        return Ok(true);
    }
    item_is_correct_tool_for_drops(
        item,
        block_type_id_with_registry(block_state_id, tags.content_registry())?,
        tags,
    )
}

/// The session reads `Player.getDestroySpeed` (`Player.java:586-616`) makes
/// beside the held stack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerDestroySpeedFacts {
    pub mining_efficiency: f64,
    pub block_break_speed: f64,
    pub submerged_mining_speed: f64,
    /// `MobEffectUtil.getDigSpeedAmplification` when `hasDigSpeed`
    /// (`MobEffectUtil.java:25-40`).
    pub dig_speed_amplification: Option<i32>,
    pub mining_fatigue_amplifier: Option<i32>,
    pub eye_in_water: bool,
    pub on_ground: bool,
}

/// `Player.getDestroySpeed` (`Player.java:586-616`).
pub fn player_destroy_speed(
    item: &ItemStackSummary,
    block_state_id: i32,
    facts: PlayerDestroySpeedFacts,
    tags: &RegistryTagSnapshot,
) -> Result<f32> {
    let mut speed = item_destroy_speed(
        item,
        block_type_id_with_registry(block_state_id, tags.content_registry())?,
        tags,
    )?;
    if speed > 1.0 {
        speed += facts.mining_efficiency as f32;
    }
    if let Some(amplification) = facts.dig_speed_amplification {
        speed *= 1.0 + (amplification + 1) as f32 * 0.2;
    }
    if let Some(amplifier) = facts.mining_fatigue_amplifier {
        speed *= match amplifier {
            0 => 0.3,
            1 => 0.09,
            2 => 0.0027,
            _ => 8.1E-4,
        };
    }
    speed *= facts.block_break_speed as f32;
    if facts.eye_in_water {
        speed *= facts.submerged_mining_speed as f32;
    }
    if !facts.on_ground {
        speed /= 5.0;
    }
    Ok(speed)
}

/// The block a state belongs to, for the `state.is(rule.blocks)` searches
/// `Tool.getMiningSpeed` (`Tool.java:41`) and `Tool.isCorrectForDrops`
/// (`Tool.java:51`) make: a `HolderSet<Block>` holds blocks, so a rule is
/// answered per block and never per state.
pub fn block_type_id_with_registry(
    block_state_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<i32> {
    vanilla_block_state_block_type_id(block_state_id)
        .or_else(|| {
            content_registry
                .block_by_state_id(block_state_id)
                .map(|(block, _)| block.block_id)
        })
        .ok_or(ServerWorldError::MissingCatalogFact(
            "block state block registry id",
        ))
}

#[cfg(test)]
mod tests;
