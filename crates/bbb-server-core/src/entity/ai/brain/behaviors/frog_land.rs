//! `TryFindLand`, `TryFindLandNearWater` and `TryLaySpawnOnFluidNearLand`,
//! the three samplers `FrogAi`'s land, swim and spawn packages register

use crate::block_behavior::weathering::within_manhattan;
use bbb_protocol::packets::{BlockPos, SoundSource};

use super::super::effect::BrainWorldEffect;
use super::super::memory::{MemoryModuleType, MemoryStatus, MemoryValue, MemoryWalkTarget};
use super::super::{Behavior, BehaviorHooks, BrainTick, BrainTickContext};

/// `TryFindLand.COOLDOWN_TICKS` (`TryFindLand.java:16`), the span
/// `nextOkStartTime` is pushed out by (`:30`, `:54`).
const TRY_FIND_LAND_COOLDOWN: i64 = 60;

/// `TryFindLandNearWater`'s own `40L` (`TryFindLandNearWater.java:27`, `:51`).
const TRY_FIND_LAND_NEAR_WATER_COOLDOWN: i64 = 40;

/// `Direction.Plane.HORIZONTAL` (`Direction.java:577`): NORTH, EAST, SOUTH,
/// WEST, as `(x, z)` offsets.
const HORIZONTAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

/// `SoundEvents.FROG_LAY_SPAWN` (`TryLaySpawnOnFluidNearLand.java:38`).
const FROG_LAY_SPAWN_SOUND: &str = "minecraft:entity.frog.lay_spawn";

/// `TryFindLand`, which IDLE registers as
/// `create(6, 1.0F)` and SWIM as `create(8, 1.5F)`
///
/// The trigger is a `OneShot` body: it runs
/// once per tick it starts on, and its `nextOkStartTime` gate still answers
/// true, so the ticks it skips are ticks it "ran" without scanning.
#[derive(Debug, Clone, Copy)]
pub struct TryFindLand {
    /// `create`'s `range` (`:18`), the manhattan reach of the scan (`:38`).
    range: i32,
    /// `create`'s `speedModifier` (`:18`), written into the `WALK_TARGET`
    /// (`:48`).
    speed_modifier: f32,
    /// `MutableLong nextOkStartTime` (`:19`), `0L` at construction.
    next_ok_start_time: i64,
}

impl TryFindLand {
    /// `TryFindLand.create`'s scan.
    ///
    /// `None` is a cell this world cannot answer, which refuses the whole
    /// trigger rather than walking the body at a position judged from a guess.
    fn find_land(&self, tick: &BrainTick<'_>, body_pos: BlockPos) -> Option<Option<BlockPos>> {
        for pos in within_manhattan(body_pos, self.range, self.range, self.range) {
            // `pos.getX() != bodyBlockPos.getX() || pos.getZ() !=
            // bodyBlockPos.getZ()` (`:39`), which every cell of the body's own
            // column fails.
            if pos.x == body_pos.x && pos.z == body_pos.z {
                continue;
            }
            let facts = tick.world().block_facts(pos)?;
            let below = tick.world().block_facts(BlockPos {
                y: pos.y - 1,
                ..pos
            })?;
            // `!state.is(Blocks.WATER) && level.getFluidState(pos).isEmpty() &&
            // state.getCollisionShape(...).isEmpty() &&
            // belowState.isFaceSturdy(level, belowPos, Direction.UP)` (`:42-45`).
            if !facts.is_water_block
                && !facts.has_fluid
                && facts.collision_shape_empty
                && below.face_sturdy_up
            {
                return Some(Some(pos));
            }
        }
        Some(None)
    }
}

impl BehaviorHooks for TryFindLand {
    // `TryFindLand.create`'s lambda.
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let timestamp = tick.world().game_time();
        let body_pos = tick.world().body().block_position;
        // `!level.getFluidState(body.blockPosition()).is(FluidTags.WATER)`
        // (`:25-27`).
        let Some(body_facts) = tick.world().block_facts(body_pos) else {
            return false;
        };
        if !body_facts.fluid_is_water {
            return false;
        }
        // `timestamp < nextOkStartTime` (`:29-32`): the skipped tick re-arms
        // the gate to `timestamp + 60`, which moves forward exactly as fast as
        // the clock, so a `OneShot` that runs every tick scans once and skips
        // for the rest of this behaviour instance's life. That is Vanilla's
        // own arithmetic; the arm still answers true.
        if timestamp < self.next_ok_start_time {
            self.next_ok_start_time = timestamp + TRY_FIND_LAND_COOLDOWN;
            return true;
        }
        let Some(found) = self.find_land(tick, body_pos) else {
            return false;
        };
        if let Some(target) = found {
            // `lookTarget.set(new BlockPosTracker(targetPos))` (`:47`),
            // carried as the memory's block-position tracker.
            tick.set_memory_value(MemoryModuleType::LookTarget, MemoryValue::BlockPos(target));
            // `walkTarget.set(new WalkTarget(new BlockPosTracker(targetPos),
            // speedModifier, 1))` (`:48`).
            tick.set_memory_value(
                MemoryModuleType::WalkTarget,
                MemoryValue::WalkTarget(MemoryWalkTarget::at(target, self.speed_modifier, 1)),
            );
        }
        self.next_ok_start_time = timestamp + TRY_FIND_LAND_COOLDOWN;
        true
    }

    fn debug_string(&self) -> &'static str {
        "TryFindLand"
    }
}

/// `TryFindLand.create(range, speedModifier)`:
/// `BehaviorBuilder.create`'s group asks for an absent `ATTACK_TARGET`, an
/// absent `WALK_TARGET` and a registered `LOOK_TARGET`.
pub fn try_find_land(range: i32, speed_modifier: f32) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::AttackTarget, MemoryStatus::ValueAbsent),
            (MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent),
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
        ],
        TryFindLand {
            range,
            speed_modifier,
            next_ok_start_time: 0,
        },
    )
}

/// `TryFindLandNearWater`, which LAY_SPAWN
/// registers as `create(8, 1.0F)`.
#[derive(Debug, Clone, Copy)]
pub struct TryFindLandNearWater {
    range: i32,
    speed_modifier: f32,
    /// `MutableLong nextOkStartTime` (`:16`), `0L` at construction.
    next_ok_start_time: i64,
}

impl TryFindLandNearWater {
    /// `TryFindLandNearWater.create`'s scan (`:35-49`).
    ///
    /// `None` is a cell this world cannot answer, which refuses the trigger.
    fn find_land_near_water(
        &self,
        tick: &BrainTick<'_>,
        body_pos: BlockPos,
    ) -> Option<Option<BlockPos>> {
        for pos in within_manhattan(body_pos, self.range, self.range, self.range) {
            if pos.x == body_pos.x && pos.z == body_pos.z {
                continue;
            }
            let facts = tick.world().block_facts(pos)?;
            let below = tick.world().block_facts(BlockPos {
                y: pos.y - 1,
                ..pos
            })?;
            // `state.getCollisionShape(...).isEmpty() &&
            // !belowState.getCollisionShape(...).isEmpty()` (`:38-39`).
            if !facts.collision_shape_empty || below.collision_shape_empty {
                continue;
            }
            for (dx, dz) in HORIZONTAL_OFFSETS {
                let side = BlockPos {
                    x: pos.x + dx,
                    z: pos.z + dz,
                    ..pos
                };
                let side_facts = tick.world().block_facts(side)?;
                let side_below = tick.world().block_facts(BlockPos {
                    y: side.y - 1,
                    ..side
                })?;
                // `level.getBlockState(testPos).isAir() &&
                // level.getBlockState(testPos.move(Direction.DOWN)).
                // is(Blocks.WATER)` (`:42`).
                if side_facts.is_air && side_below.is_water_block {
                    return Some(Some(pos));
                }
            }
        }
        Some(None)
    }
}

impl BehaviorHooks for TryFindLandNearWater {
    // `TryFindLandNearWater.create`'s lambda.
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let timestamp = tick.world().game_time();
        let body_pos = tick.world().body().block_position;
        // `level.getFluidState(body.blockPosition()).is(FluidTags.WATER)`
        // (`:22-24`), the exact negation of the land search's first term.
        let Some(body_facts) = tick.world().block_facts(body_pos) else {
            return false;
        };
        if body_facts.fluid_is_water {
            return false;
        }
        // The same self-re-arming gate over this package's own forty ticks
        // (`:26-29`).
        if timestamp < self.next_ok_start_time {
            self.next_ok_start_time = timestamp + TRY_FIND_LAND_NEAR_WATER_COOLDOWN;
            return true;
        }
        let Some(found) = self.find_land_near_water(tick, body_pos) else {
            return false;
        };
        if let Some(target) = found {
            // `lookTarget.set(new BlockPosTracker(pos))` (`:43`).
            tick.set_memory_value(MemoryModuleType::LookTarget, MemoryValue::BlockPos(target));
            // `walkTarget.set(new WalkTarget(new BlockPosTracker(pos),
            // speedModifier, 0))` (`:44`).
            tick.set_memory_value(
                MemoryModuleType::WalkTarget,
                MemoryValue::WalkTarget(MemoryWalkTarget::at(target, self.speed_modifier, 0)),
            );
        }
        self.next_ok_start_time = timestamp + TRY_FIND_LAND_NEAR_WATER_COOLDOWN;
        true
    }

    fn debug_string(&self) -> &'static str {
        "TryFindLandNearWater"
    }
}

/// `TryFindLandNearWater.create(range, speedModifier)`
/// whose group asks for an absent
/// `ATTACK_TARGET`, an absent `WALK_TARGET` and a registered `LOOK_TARGET`
pub fn try_find_land_near_water(range: i32, speed_modifier: f32) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::AttackTarget, MemoryStatus::ValueAbsent),
            (MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent),
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
        ],
        TryFindLandNearWater {
            range,
            speed_modifier,
            next_ok_start_time: 0,
        },
    )
}

/// `TryLaySpawnOnFluidNearLand`,
/// which LAY_SPAWN registers as `create(Blocks.FROGSPAWN)`
#[derive(Debug, Clone, Copy)]
pub struct TryLaySpawnOnFluidNearLand {
    /// `spawnBlock.defaultBlockState()` (`:35`), which a level takes as a
    /// global block-state id.
    spawn_block_state_id: i32,
}

impl BehaviorHooks for TryLaySpawnOnFluidNearLand {
    // `TryLaySpawnOnFluidNearLand.create`'s lambda (`:22-48`).
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        // `!body.isInWater() && body.onGround()` (`:23`).
        if tick.world().body().in_water || !tick.world().body().on_ground {
            return false;
        }
        let below_pos = BlockPos {
            y: tick.world().body().block_position.y - 1,
            ..tick.world().body().block_position
        };
        // Every arm answers true once the body is in water and standing: a
        // spawn laid at `:40` and the fall-through at `:45`.
        for (dx, dz) in HORIZONTAL_OFFSETS {
            let relative = BlockPos {
                x: below_pos.x + dx,
                z: below_pos.z + dz,
                ..below_pos
            };
            let Some(facts) = tick.world().block_facts(relative) else {
                return false;
            };
            // `state.getCollisionShape(...).getFaceShape(Direction.UP)
            // .isEmpty() && (fluidState(relativePos).is(SUPPORTS_FROGSPAWN) ||
            // state(relativePos).is(BlockTags.SUPPORTS_FROGSPAWN))`.
            if !facts.collision_face_up_empty
                || !(facts.fluid_supports_frogspawn || facts.supports_frogspawn)
            {
                continue;
            }
            let spawn_pos = BlockPos {
                y: relative.y + 1,
                ..relative
            };
            let Some(spawn_facts) = tick.world().block_facts(spawn_pos) else {
                return false;
            };
            // `level.getBlockState(spawnPos).isAir()` (`:34`).
            if !spawn_facts.is_air {
                continue;
            }
            // `level.setBlock(spawnPos, newState, 3)` and the
            // `GameEvent.BLOCK_PLACE` behind it (`:36-37`), then
            // `level.playSound(null, body, FROG_LAY_SPAWN, BLOCKS, 1.0F,
            // 1.0F)` (`:38`), in that order.
            tick.world_mut().queue_effect(BrainWorldEffect::PlaceBlock {
                pos: spawn_pos,
                block_state_id: self.spawn_block_state_id,
            });
            tick.world_mut()
                .queue_effect(BrainWorldEffect::PlayBodySound {
                    resource_id: FROG_LAY_SPAWN_SOUND,
                    source: SoundSource::Blocks,
                    volume: 1.0,
                    pitch: 1.0,
                });
            // The synchronous block callback must still see the live pregnancy memory.
            let entity_id = tick.world().body().entity_id;
            tick.world_mut()
                .queue_effect(BrainWorldEffect::SetMobBrainMemory {
                    entity_id,
                    memory_type: MemoryModuleType::IsPregnant,
                    value: None,
                });
            return true;
        }
        true
    }

    fn debug_string(&self) -> &'static str {
        "TryLaySpawnOnFluidNearLand"
    }
}

/// `TryLaySpawnOnFluidNearLand.create(spawnBlock)`
/// whose group asks for an absent
/// `ATTACK_TARGET`, a present `WALK_TARGET` and a present `IS_PREGNANT`
pub fn try_lay_spawn_on_fluid_near_land(spawn_block_state_id: i32) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::AttackTarget, MemoryStatus::ValueAbsent),
            (MemoryModuleType::WalkTarget, MemoryStatus::ValuePresent),
            (MemoryModuleType::IsPregnant, MemoryStatus::ValuePresent),
        ],
        TryLaySpawnOnFluidNearLand {
            spawn_block_state_id,
        },
    )
}

#[cfg(test)]
mod tests;
