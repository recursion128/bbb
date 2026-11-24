//! `LongJumpMidJump` and `LongJumpToPreferredBlock` over
//! `LongJumpToRandomPos` and `LongJumpUtil`, the two halves of `FrogAi`'s
//! LONG_JUMP package.

use crate::random::GameRandom;
use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d};

use super::super::effect::BrainWorldEffect;
use super::super::memory::{MemoryModuleType, MemoryStatus, MemoryValue};
use super::super::{Behavior, BehaviorHooks, BrainTick, BrainTickContext};
use super::{uniform_int_span_from_level, POSE_STANDING};
use crate::entity::GroundPathType;

/// `Pose.LONG_JUMPING.getId()` (`EntityPose.java`), which
/// `LongJumpMidJump.start` writes (`LongJumpMidJump.java:30`).
const POSE_LONG_JUMPING: i32 = bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_LONG_JUMPING_ID;

/// `LongJumpToRandomPos.FIND_JUMP_TRIES` (`LongJumpToRandomPos.java:31`).
const FIND_JUMP_TRIES: i32 = 20;
/// `LongJumpToRandomPos.PREPARE_JUMP_DURATION` (`:32`), the delay between
/// choosing a vector and launching it.
const PREPARE_JUMP_DURATION: i64 = 40;
/// `LongJumpToRandomPos.MIN_PATHFIND_DISTANCE_TO_VALID_JUMP` (`:33`), the
/// `maxPathLength` of the candidate's own path probe (`:166`).
const MIN_PATHFIND_DISTANCE_TO_VALID_JUMP: f32 = 8.0;
/// `LongJumpToRandomPos.TIME_OUT_DURATION` (`:34`).
const LONG_JUMP_TIME_OUT: i32 = 200;
/// `LongJumpToRandomPos.ALLOWED_ANGLES` (`:35`).
const ALLOWED_ANGLES: [i32; 4] = [65, 70, 75, 80];
/// `LongJumpMidJump.TIME_OUT_DURATION` (`LongJumpMidJump.java:14`).
const MID_JUMP_TIME_OUT: i32 = 100;

/// One `LongJumpToRandomPos.PossibleJump`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PossibleJump {
    target_pos: BlockPos,
    weight: i32,
}

/// `LongJumpMidJump`, LONG_JUMP's priority 0
#[derive(Debug, Clone, Copy)]
pub struct LongJumpMidJump {
    /// The `UniformInt` both halves of the package were built with
    /// whose `sample` the landing cooldown is
    time_between_long_jumps_min: i32,
    time_between_long_jumps_max: i32,
    /// `SoundEvents.FROG_STEP`.
    landing_sound: &'static str,
}

impl BehaviorHooks for LongJumpMidJump {
    // `LongJumpMidJump.canStillUse`.
    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        !tick.world().body().on_ground
    }

    // `LongJumpMidJump.start`.
    fn start(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) {
        tick.world_mut()
            .queue_effect(BrainWorldEffect::SetBodyDiscardFriction { discard: true });
        tick.world_mut().queue_effect(BrainWorldEffect::SetPose {
            pose_id: POSE_LONG_JUMPING,
        });
    }

    // `LongJumpMidJump.stop`.
    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        if tick.world().body().on_ground {
            // `body.setDeltaMovement(body.getDeltaMovement().multiply(0.1F,
            // 1.0, 0.1F))` (`:35`), whose `0.1F` each axis is scaled by.
            let movement = tick.world().body().delta_movement;
            let friction = f64::from(0.1_f32);
            tick.world_mut()
                .queue_effect(BrainWorldEffect::SetBodyDeltaMovement {
                    movement: Vec3d {
                        x: movement.x * friction,
                        y: movement.y,
                        z: movement.z * friction,
                    },
                });
            // `level.playSound(null, body, landingSound, NEUTRAL, 2.0F,
            // 1.0F)` (`:36`).
            tick.world_mut()
                .queue_effect(BrainWorldEffect::PlayBodySound {
                    resource_id: self.landing_sound,
                    source: SoundSource::Neutral,
                    volume: 2.0,
                    pitch: 1.0,
                });
        }
        tick.world_mut()
            .queue_effect(BrainWorldEffect::SetBodyDiscardFriction { discard: false });
        tick.world_mut().queue_effect(BrainWorldEffect::SetPose {
            pose_id: POSE_STANDING,
        });
        // `eraseMemory(LONG_JUMP_MID_JUMP)` (`:41`) before the cooldown write
        // (`:42`).
        tick.erase_memory(MemoryModuleType::LongJumpMidJump);
        let cooldown = uniform_int_span_from_level(
            tick,
            self.time_between_long_jumps_min,
            self.time_between_long_jumps_max,
        );
        tick.set_memory_value(
            MemoryModuleType::LongJumpCooldownTicks,
            MemoryValue::Int(cooldown),
        );
    }

    fn debug_string(&self) -> &'static str {
        "LongJumpMidJump"
    }
}

/// `new LongJumpMidJump(TIME_BETWEEN_LONG_JUMPS, SoundEvents.FROG_STEP)`
/// whose constructor asks for a registered `LOOK_TARGET`
/// and a present `LONG_JUMP_MID_JUMP`.
pub fn long_jump_mid_jump(
    time_between_long_jumps_min: i32,
    time_between_long_jumps_max: i32,
    landing_sound: &'static str,
) -> Behavior {
    Behavior::with_duration(
        vec![
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
            (
                MemoryModuleType::LongJumpMidJump,
                MemoryStatus::ValuePresent,
            ),
        ],
        MID_JUMP_TIME_OUT,
        MID_JUMP_TIME_OUT,
        LongJumpMidJump {
            time_between_long_jumps_min,
            time_between_long_jumps_max,
            landing_sound,
        },
    )
}

/// `LongJumpToPreferredBlock` over
/// `LongJumpToRandomPos`, LONG_JUMP's
/// priority 1.
#[derive(Debug, Clone)]
pub struct LongJumpToPreferredBlock {
    time_between_long_jumps_min: i32,
    time_between_long_jumps_max: i32,
    max_long_jump_height: i32,
    max_long_jump_width: i32,
    max_jump_velocity_multiplier: f32,
    jump_sound: &'static str,
    preferred_blocks_chance: f32,
    /// `LongJumpToRandomPos.jumpCandidates` (`:40`), in the order
    /// `BlockPos.betweenClosed` walks them: x
    /// fastest, then y, then z.
    jump_candidates: Vec<PossibleJump>,
    /// `LongJumpToRandomPos.initialPosition` (`:41`).
    initial_position: Option<Vec3d>,
    /// `LongJumpToRandomPos.chosenJump` (`:42`).
    chosen_jump: Option<Vec3d>,
    /// `LongJumpToRandomPos.findJumpTries` (`:43`).
    find_jump_tries: i32,
    /// `LongJumpToRandomPos.prepareJumpStart` (`:44`).
    prepare_jump_start: i64,
    /// `LongJumpToPreferredBlock.notPrefferedJumpCandidates`
    not_preferred_jump_candidates: std::collections::VecDeque<PossibleJump>,
    /// `LongJumpToPreferredBlock.currentlyWantingPreferredOnes` (`:21`).
    currently_wanting_preferred_ones: bool,
}

impl Default for LongJumpToPreferredBlock {
    fn default() -> Self {
        Self {
            time_between_long_jumps_min: 0,
            time_between_long_jumps_max: 0,
            max_long_jump_height: 0,
            max_long_jump_width: 0,
            max_jump_velocity_multiplier: 0.0,
            jump_sound: "",
            preferred_blocks_chance: 0.0,
            jump_candidates: Vec::new(),
            initial_position: None,
            chosen_jump: None,
            find_jump_tries: 0,
            prepare_jump_start: 0,
            not_preferred_jump_candidates: std::collections::VecDeque::new(),
            currently_wanting_preferred_ones: false,
        }
    }
}

impl LongJumpToPreferredBlock {
    /// `LongJumpToRandomPos.checkExtraStartConditions`
    fn can_start(&mut self, tick: &mut BrainTick<'_>) -> bool {
        // `body.onGround() && !body.isInWater() && !body.isInLava() &&
        // !level.getBlockState(body.blockPosition()).is(Blocks.HONEY_BLOCK)`
        // (`:94`). `PathType.STICKY_HONEY` is the honey block's own state type
        let honey = tick
            .world()
            .block_path_type(tick.world().body().block_position)
            .is_some_and(|path_type| path_type == GroundPathType::StickyHoney);
        let can_start = tick.world().body().on_ground
            && !tick.world().body().in_water
            && !tick.world().body().in_lava
            && !honey;
        if !can_start {
            self.write_cooldown(tick, true);
        }
        can_start
    }

    /// `setMemory(LONG_JUMP_COOLDOWN_TICKS,
    /// timeBetweenLongJumps.sample(level.getRandom()) / 2)`
    fn write_cooldown(&self, tick: &mut BrainTick<'_>, halved: bool) {
        let sample = uniform_int_span_from_level(
            tick,
            self.time_between_long_jumps_min,
            self.time_between_long_jumps_max,
        );
        let value = if halved { sample / 2 } else { sample };
        tick.set_memory_value(
            MemoryModuleType::LongJumpCooldownTicks,
            MemoryValue::Int(value),
        );
    }

    /// `LongJumpToRandomPos.canStillUse`.
    fn still_valid(&self, tick: &mut BrainTick<'_>) -> bool {
        let valid = self.initial_position == Some(tick.world().body().position)
            && self.find_jump_tries > 0
            && !tick.world().body().in_water
            && (self.chosen_jump.is_some()
                || !self.jump_candidates.is_empty()
                || !self.not_preferred_jump_candidates.is_empty());
        if !valid && !tick.has_memory_value(MemoryModuleType::LongJumpMidJump) {
            self.write_cooldown(tick, true);
            // `eraseMemory(LOOK_TARGET)` (`:110`).
            tick.erase_memory(MemoryModuleType::LookTarget);
        }
        valid
    }

    /// `LongJumpToRandomPos.start` over
    /// `LongJumpToPreferredBlock.start`.
    fn begin(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext) {
        self.chosen_jump = None;
        self.find_jump_tries = FIND_JUMP_TRIES;
        self.initial_position = Some(tick.world().body().position);
        let mob_pos = tick.world().body().block_position;
        let width = self.max_long_jump_width;
        let height = self.max_long_jump_height;
        self.jump_candidates = between_closed(
            BlockPos {
                x: mob_pos.x - width,
                y: mob_pos.y - height,
                z: mob_pos.z - width,
            },
            BlockPos {
                x: mob_pos.x + width,
                y: mob_pos.y + height,
                z: mob_pos.z + width,
            },
        )
        .filter(|pos| *pos != mob_pos)
        .map(|pos| PossibleJump {
            target_pos: pos,
            weight: block_dist_sqr(mob_pos, pos).ceil() as i32,
        })
        .collect();
        self.not_preferred_jump_candidates.clear();
        // `body.getRandom().nextFloat() < preferredBlocksChance`
        self.currently_wanting_preferred_ones = context.next_float() < self.preferred_blocks_chance;
    }

    /// `LongJumpToRandomPos.tick`.
    fn step(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, timestamp: i64) {
        if let Some(chosen) = self.chosen_jump {
            if timestamp - self.prepare_jump_start < PREPARE_JUMP_DURATION {
                return;
            }
            // `body.setYRot(body.yBodyRot)` (`:140`), then the friction flag
            // (`:141`), the vector (`:142-144`) and the memory (`:145`), then
            // the sound (`:146`).
            let body_y_rot = tick.world().body().y_body_rot;
            tick.world_mut()
                .queue_effect(BrainWorldEffect::SetBodyYRot { y_rot: body_y_rot });
            tick.world_mut()
                .queue_effect(BrainWorldEffect::SetBodyDiscardFriction { discard: true });
            // `orgLength` is never zero: `isAcceptableLandingPosition` refuses
            // every candidate in the body's own column (`:190`) before the arc
            // is solved, so `v0` and the vector it scales are both positive.
            let original_length = chosen_length(chosen);
            let scale = (original_length + tick.world().body().jump_boost_power) / original_length;
            tick.world_mut()
                .queue_effect(BrainWorldEffect::SetBodyDeltaMovement {
                    movement: Vec3d {
                        x: chosen.x * scale,
                        y: chosen.y * scale,
                        z: chosen.z * scale,
                    },
                });
            tick.set_memory_value(MemoryModuleType::LongJumpMidJump, MemoryValue::Bool(true));
            tick.world_mut()
                .queue_effect(BrainWorldEffect::PlayBodySound {
                    resource_id: self.jump_sound,
                    source: SoundSource::Neutral,
                    volume: 1.0,
                    pitch: 1.0,
                });
        } else {
            self.find_jump_tries -= 1;
            self.pick_candidate(tick, context, timestamp);
        }
    }

    /// `LongJumpToRandomPos.pickCandidate`.
    fn pick_candidate(
        &mut self,
        tick: &mut BrainTick<'_>,
        context: &mut BrainTickContext,
        timestamp: i64,
    ) {
        while !self.jump_candidates.is_empty() || !self.not_preferred_jump_candidates.is_empty() {
            let Some(candidate) = self.jump_candidate(tick) else {
                return;
            };
            let target_pos = candidate.target_pos;
            let Some(acceptable) = self.acceptable_landing_position(tick, target_pos) else {
                return;
            };
            if !acceptable {
                continue;
            }
            let target_position = Vec3d {
                x: f64::from(target_pos.x) + 0.5,
                y: f64::from(target_pos.y) + 0.5,
                z: f64::from(target_pos.z) + 0.5,
            };
            let Some(jump_vector) = self.optimal_jump_vector(tick, context, target_position) else {
                continue;
            };
            // `body.getBrain().setMemory(LOOK_TARGET, new
            // BlockPosTracker(targetPos))` (`:164`).
            tick.set_memory_value(
                MemoryModuleType::LookTarget,
                MemoryValue::BlockPos(target_pos),
            );
            // `navigation.createPath(targetPos, 0, 8)`, and the jump is taken
            // when that path is `null` or cannot reach (`:165-171`).
            let reaches =
                tick.world()
                    .body_path_reaches(target_pos, 0, MIN_PATHFIND_DISTANCE_TO_VALID_JUMP);
            if reaches == Some(true) {
                continue;
            }
            self.chosen_jump = Some(jump_vector);
            self.prepare_jump_start = timestamp;
            return;
        }
    }

    /// `LongJumpToPreferredBlock.getJumpCandidate`
    /// over
    /// `LongJumpToRandomPos.getJumpCandidate`.
    fn jump_candidate(&mut self, tick: &mut BrainTick<'_>) -> Option<PossibleJump> {
        if !self.currently_wanting_preferred_ones {
            return self.any_jump_candidate(tick);
        }
        while !self.jump_candidates.is_empty() {
            let Some(candidate) = self.any_jump_candidate(tick) else {
                break;
            };
            // `level.getBlockState(possibleJump.targetPos().below())
            // .is(preferredBlockTag)`.
            let below = BlockPos {
                y: candidate.target_pos.y - 1,
                ..candidate.target_pos
            };
            let preferred = tick
                .world()
                .block_facts(below)
                .is_some_and(|facts| facts.prefers_frog_jump);
            if preferred {
                return Some(candidate);
            }
            self.not_preferred_jump_candidates.push_back(candidate);
        }
        // A candidate list the level cannot judge still falls back on the
        // ones already taken (`:65`).
        self.not_preferred_jump_candidates.pop_front()
    }

    /// `WeightedRandom.getRandomItem(level.getRandom(), jumpCandidates,
    /// PossibleJump::weight)` and the `remove` behind it
    fn any_jump_candidate(&mut self, tick: &mut BrainTick<'_>) -> Option<PossibleJump> {
        let total: i32 = self.jump_candidates.iter().map(|jump| jump.weight).sum();
        if total <= 0 {
            return None;
        }
        let mut selection = tick.world_mut().level_random_next_int(total);
        let mut picked = None;
        for (index, jump) in self.jump_candidates.iter().enumerate() {
            selection -= jump.weight;
            if selection < 0 {
                picked = Some(index);
                break;
            }
        }
        let index = picked?;
        Some(self.jump_candidates.swap_remove(index))
    }

    /// `LongJumpToRandomPos.isAcceptableLandingPosition`
    /// over `FrogAi::isAcceptableLandingSpot`
    fn acceptable_landing_position(
        &self,
        tick: &BrainTick<'_>,
        target_pos: BlockPos,
    ) -> Option<bool> {
        let mob_pos = tick.world().body().block_position;
        if mob_pos.x == target_pos.x && mob_pos.z == target_pos.z {
            return Some(false);
        }
        let below = BlockPos {
            y: target_pos.y - 1,
            ..target_pos
        };
        let above = BlockPos {
            y: target_pos.y + 1,
            ..target_pos
        };
        let facts = tick.world().block_facts(target_pos)?;
        let below_facts = tick.world().block_facts(below)?;
        let above_facts = tick.world().block_facts(above)?;
        // `level.getFluidState(targetPos).isEmpty() &&
        // level.getFluidState(below).isEmpty() &&
        // level.getFluidState(targetPos.above()).isEmpty()`.
        if facts.has_fluid || below_facts.has_fluid || above_facts.has_fluid {
            return Some(false);
        }
        // `!bs.is(FROG_PREFER_JUMP_TO) && !bsBelow.is(FROG_PREFER_JUMP_TO)`
        // (`:212`), whose negation answers true.
        if facts.prefers_frog_jump || below_facts.prefers_frog_jump {
            return Some(true);
        }
        let path_type = tick.world().block_path_type(target_pos)?;
        let path_type_below = tick.world().block_path_type(below)?;
        // `pathType != TRAPDOOR && (!bs.isAir() || pathTypeBelow != TRAPDOOR)`
        // (`:216-218`): the false arm answers true, the true arm asks
        // `defaultAcceptableLandingSpot` (`:63`).
        if path_type == GroundPathType::Trapdoor
            || (facts.is_air && path_type_below == GroundPathType::Trapdoor)
        {
            return Some(true);
        }
        if !below_facts.is_solid_render {
            return Some(false);
        }
        Some(tick.world().body_pathfinding_malus(path_type) == 0.0)
    }

    /// `LongJumpToRandomPos.calculateOptimalJumpVector`
    /// the shuffled angles are tried in
    /// order and the first solvable one wins.
    fn optimal_jump_vector(
        &mut self,
        tick: &BrainTick<'_>,
        context: &mut BrainTickContext,
        target_position: Vec3d,
    ) -> Option<Vec3d> {
        let mut angles = ALLOWED_ANGLES;
        shuffle(&mut angles, &context.random);
        let max_jump_velocity =
            tick.world().body().jump_strength * f64::from(self.max_jump_velocity_multiplier);
        for angle in angles {
            if let Some(velocity) =
                calculate_jump_vector_for_angle(tick, target_position, max_jump_velocity, angle)
            {
                return Some(velocity);
            }
        }
        None
    }
}

impl BehaviorHooks for LongJumpToPreferredBlock {
    // `LongJumpToRandomPos.checkExtraStartConditions`
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        self.can_start(tick)
    }

    // `LongJumpToRandomPos.canStillUse`,
    // whose invalid arm writes the aborted jump's cooldown and erases the look
    // target before the behaviour is stopped.
    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        self.still_valid(tick)
    }

    // `LongJumpToRandomPos.start` over `LongJumpToPreferredBlock.start`
    fn start(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, _timestamp: i64) {
        self.begin(tick, context);
    }

    // `LongJumpToRandomPos.tick`.
    fn tick(&mut self, tick: &mut BrainTick<'_>, context: &mut BrainTickContext, timestamp: i64) {
        self.step(tick, context, timestamp);
    }

    fn debug_string(&self) -> &'static str {
        "LongJumpToPreferredBlock"
    }
}

/// `new LongJumpToPreferredBlock<>(TIME_BETWEEN_LONG_JUMPS, 2, 4, 3.5714288F,
/// body -> SoundEvents.FROG_LONG_JUMP, BlockTags.FROG_PREFER_JUMP_TO, 0.5F,
/// FrogAi::isAcceptableLandingSpot)`, whose
/// constructor asks for a registered `LOOK_TARGET`, an absent
/// `LONG_JUMP_COOLDOWN_TICKS` and an absent `LONG_JUMP_MID_JUMP`
pub fn long_jump_to_preferred_block(
    time_between_long_jumps_min: i32,
    time_between_long_jumps_max: i32,
    max_long_jump_height: i32,
    max_long_jump_width: i32,
    max_jump_velocity_multiplier: f32,
    jump_sound: &'static str,
    preferred_blocks_chance: f32,
) -> Behavior {
    Behavior::with_duration(
        vec![
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
            (
                MemoryModuleType::LongJumpCooldownTicks,
                MemoryStatus::ValueAbsent,
            ),
            (MemoryModuleType::LongJumpMidJump, MemoryStatus::ValueAbsent),
        ],
        LONG_JUMP_TIME_OUT,
        LONG_JUMP_TIME_OUT,
        LongJumpToPreferredBlock {
            time_between_long_jumps_min,
            time_between_long_jumps_max,
            max_long_jump_height,
            max_long_jump_width,
            max_jump_velocity_multiplier,
            jump_sound,
            preferred_blocks_chance,
            ..LongJumpToPreferredBlock::default()
        },
    )
}

/// `BlockPos.betweenClosed(minX, minY, minZ, maxX, maxY, maxZ)`
/// every cell of the box, x fastest, then y, then
/// z.
fn between_closed(min: BlockPos, max: BlockPos) -> impl Iterator<Item = BlockPos> {
    let width = max.x - min.x + 1;
    let height = max.y - min.y + 1;
    let depth = max.z - min.z + 1;
    let end = width * height * depth;
    (0..end).map(move |index| {
        let x = index % width;
        let slice = index / width;
        let y = slice % height;
        let z = slice / height;
        BlockPos {
            x: min.x + x,
            y: min.y + y,
            z: min.z + z,
        }
    })
}

/// `Vec3i.distSqr(Vec3i)` (`Vec3i.java`), which the candidate weights ceil.
fn block_dist_sqr(from: BlockPos, to: BlockPos) -> f64 {
    let dx = f64::from(from.x - to.x);
    let dy = f64::from(from.y - to.y);
    let dz = f64::from(from.z - to.z);
    dx * dx + dy * dy + dz * dz
}

/// `Vec3.length()` (`Vec3.java`).
fn chosen_length(vector: Vec3d) -> f64 {
    (vector.x * vector.x + vector.y * vector.y + vector.z * vector.z).sqrt()
}

/// `Collections.shuffle(List)` over `Collections.shuffle(List, Random)`'s
/// Fisher-Yates walk from the end. Both live in the JDK, outside the
/// Vanilla tree, so neither carries a source anchor.
fn shuffle(angles: &mut [i32; 4], random: &GameRandom) {
    for index in (1..angles.len()).rev() {
        let swap = random.next_int(index as i32 + 1) as usize;
        angles.swap(index, swap);
    }
}

fn calculate_jump_vector_for_angle(
    tick: &BrainTick<'_>,
    target_pos: Vec3d,
    max_jump_velocity: f64,
    angle: i32,
) -> Option<Vec3d> {
    let body = tick.world().body();
    let body_box = body.bounding_box;
    let width = body_box.max_x - body_box.min_x;
    let height = body_box.max_y - body_box.min_y;
    let mut transition_clear = |from, to| {
        bbb_server_types::geometry::long_jump_transition_clear(from, to, width, height, |area| {
            tick.world().body_can_fit_at(area)
        })
    };
    bbb_server_types::geometry::long_jump_velocity(
        body.position,
        target_pos,
        max_jump_velocity,
        angle,
        body.gravity,
        Some(&mut transition_clear),
    )
}

#[cfg(test)]
mod tests;
