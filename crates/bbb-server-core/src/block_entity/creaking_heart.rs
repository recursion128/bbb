//! `CreakingHeartBlockEntity` (`CreakingHeartBlockEntity.java`).
//!
//! The protector UUID round-trips and the analog half of `serverTick` runs off
//! it. The countdown half below it (`:98-133`), which spawns and releases the
//! protector, has no owner here.

use crate::random::GameRandom;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use crate::fluid::with_property;

use crate::block_behavior::creaking_heart::{AWAKE, DORMANT, STATE, UPROOTED};

use super::{codec_uuid, entry, unique_field, uuid_int_array};

/// `CreakingHeartBlockEntity.saveAdditional` / `loadAdditional` (`:346`, `:353`).
const TAG_CREAKING: &str = "creaking";

/// `CreakingHeartBlockEntity.CREAKING_ROAMING_RADIUS` (`:44`).
const CREAKING_ROAMING_RADIUS: f64 = 32.0;

/// `CreakingHeartBlockEntity.TICKS_GRACE_PERIOD` (`:57`).
const TICKS_GRACE_PERIOD: i64 = 30;

/// The release distance `serverTick` tests (`:121`), which is
/// `CREAKING_ROAMING_RADIUS` plus two.
const CREAKING_RELEASE_RADIUS: f64 = 34.0;

/// `CreakingHeartBlockEntity.updateCreakingState` (`:137-146`).
fn update_creaking_state(
    state: i32,
    protector_bound: bool,
    has_required_logs: Option<bool>,
    creaking_active: bool,
) -> i32 {
    if has_required_logs != Some(true) && !protector_bound {
        return with_property(state, STATE, UPROOTED).unwrap_or(state);
    }
    let next = if creaking_active { AWAKE } else { DORMANT };
    with_property(state, STATE, next).unwrap_or(state)
}

/// The range `computeAnalogOutputSignal` spans (`:337`).
const SIGNAL_MAX: i32 = 15;

/// `CreakingHeartBlockEntity` (`:42-68`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CreakingHeartBlockEntity {
    creaking: Option<NbtSummaryValue>,
    /// `CreakingHeartBlockEntity.ticksExisted` (`:60`).
    ticks_existed: i64,
    /// `CreakingHeartBlockEntity.ticker` (`:61`), which neither save writes.
    ticker: i32,
    /// `CreakingHeartBlockEntity.emitter` (`:62`), likewise transient.
    emitter: i32,
    /// `CreakingHeartBlockEntity.emitterTarget` (`:63`).
    emitter_target: Option<Vec3d>,
    /// `CreakingHeartBlockEntity.outputSignal` (`:64`).
    output_signal: i32,
}

/// What `CreakingHeartBlock.getAnalogOutputSignal`
/// (`CreakingHeartBlock.java`) reads off the live object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreakingHeartAnalog {
    /// Whether `creakingInfo` (`CreakingHeartBlockEntity.java`) names a
    /// protector, which is the first half of `computeAnalogOutputSignal`'s
    /// test (`:334`).
    pub protector_bound: bool,
    /// `CreakingHeartBlockEntity.getAnalogOutputSignal` (`:329-331`).
    pub output_signal: i32,
}

/// What one analog pass of `serverTick` leaves for the level to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CreakingHeartAnalogTick {
    /// `BlockEntity.setChanged` (`BlockEntity.java`).
    pub set_changed: bool,
    /// `Level.updateNeighbourForOutputSignal`
    /// (`CreakingHeartBlockEntity.java`), which `setChanged` reaches once
    /// more of its own accord (`BlockEntity.java`).
    pub notify_output_signal: bool,
}

/// What one `serverTick` pass reads from the level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreakingHeartTickInput {
    /// `CreakingHeartBlock.hasRequiredLogs`
    /// (`CreakingHeartBlock.java`).
    pub has_required_logs: Option<bool>,
    /// `EnvironmentAttributes.CREAKING_ACTIVE` at the heart (`:142`).
    pub creaking_active: bool,
    /// `ServerLevel.isSpawningMonsters` (`:110`).
    pub spawning_monsters: bool,
    /// `level.getNearestPlayer(pos, 32.0, false) != null` (`:111`).
    pub has_player_within_32: bool,
    /// `getCreakingProtector`'s live entity (`:180-193`).
    pub protector: Option<CreakingHeartProtectorState>,
}

/// What the resolved protector answers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreakingHeartProtectorState {
    /// `Entity.position()`, which `distanceToCreaking` (`:148-150`) measures
    /// from.
    pub position: Vec3d,
    /// `getBoundingBox()` (`:86`, `:241`).
    pub bounding_box: bbb_server_types::geometry::Aabb,
    /// `Mob.isPersistenceRequired` (`:120`).
    pub persistence_required: bool,
    /// `Creaking.playerIsStuckInYou` (`:122`).
    pub player_stuck: bool,
}

/// What one `serverTick` pass asks the level to run.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CreakingHeartTick {
    /// `BlockEntity.setChanged` (`BlockEntity.java`).
    pub set_changed: bool,
    /// `Level.updateNeighbourForOutputSignal` (`:76`).
    pub notify_output_signal: bool,
    /// `level.setBlock(pos, updatedState, 3)` (`:101-104`).
    pub new_state: Option<i32>,
    /// `emitParticles(level, 1, towardsCreaking)` (`:81-82`), in call order.
    pub emit_particles: Vec<bool>,
    /// `CREAKING_HEART_HURT` at the scaled position and volume (`:86-93`).
    pub hurt_sound: Option<(BlockPos, f32)>,
    /// `spawnProtector` (`:113`).
    pub spawn_protector: bool,
    /// `removeProtector(null)` (`:123`).
    pub remove_protector: bool,
}

impl CreakingHeartBlockEntity {
    /// `CreakingHeartBlockEntity.saveAdditional` (`CreakingHeartBlockEntity.java`).
    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        match &self.creaking {
            Some(creaking) => vec![entry(TAG_CREAKING, creaking.clone())],
            None => Vec::new(),
        }
    }

    /// `CreakingHeartBlockEntity.loadAdditional` (`CreakingHeartBlockEntity.java`).
    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.creaking = unique_field(record, TAG_CREAKING)
            .filter(|value| uuid_codec_value(value))
            .cloned();
        // Either arm of the `ifPresentOrElse` dates the object from now:
        // `setCreakingInfo` (`:162-166`) zeroes `ticksExisted` and
        // `clearCreakingInfo` (`:152-155`) leaves a fresh object's zero.
        self.ticks_existed = 0;
    }

    /// The half of `CreakingHeartBlockEntity.getCreakingProtector` (`:168-197`)
    /// this object holds: the UUID a level resolves to a live `Creaking`, or
    /// `None` when `creakingInfo` names nobody.
    #[must_use]
    pub fn protector_uuid(&self) -> Option<Uuid> {
        codec_uuid(self.creaking.as_ref()?)
    }

    /// What `CreakingHeartBlock.getAnalogOutputSignal`
    /// (`CreakingHeartBlock.java`) reads.
    #[must_use]
    pub const fn analog(&self) -> CreakingHeartAnalog {
        CreakingHeartAnalog {
            protector_bound: self.creaking.is_some(),
            output_signal: self.output_signal,
        }
    }

    /// `CreakingHeartBlockEntity.serverTick`'s analog half (`:70-77`), with
    /// `getCreakingProtector`'s `getEntity(uuid)` (`:182-193`) already
    /// answered: `protector` is the live creaking's position, and `None` is a
    /// stored UUID that names no live one.
    pub fn server_tick_analog(
        &mut self,
        pos: BlockPos,
        protector: Option<Vec3d>,
    ) -> CreakingHeartAnalogTick {
        self.ticks_existed += 1;
        let mut outcome = CreakingHeartAnalogTick::default();
        // `getCreakingProtector` drops a UUID that has named nobody for the
        // whole grace period (`:188-190`).
        if self.creaking.is_some()
            && protector.is_none()
            && self.ticks_existed >= TICKS_GRACE_PERIOD
        {
            self.creaking = None;
            outcome.set_changed = true;
            outcome.notify_output_signal = true;
        }
        let computed = self.compute_analog_output_signal(pos, protector);
        if self.output_signal != computed {
            self.output_signal = computed;
            outcome.notify_output_signal = true;
        }
        outcome
    }

    /// `CreakingHeartBlockEntity.serverTick` (`:70-133`) as one pass.
    ///
    /// The level answers `getCreakingProtector`, `hasRequiredLogs`,
    /// `CREAKING_ACTIVE`, `isSpawningMonsters` and the 32-block player search;
    /// everything the object itself owns -- the ticker, the emitter countdown,
    /// the state the pass wants and the spawn/release decisions -- happens
    /// here, so the two halves stay in the order Vanilla runs them.
    pub fn server_tick(
        &mut self,
        pos: BlockPos,
        state: i32,
        input: CreakingHeartTickInput,
        random: &GameRandom,
    ) -> CreakingHeartTick {
        self.ticks_existed += 1;
        let mut out = CreakingHeartTick::default();
        // `:74-77`: `getCreakingProtector` drops a UUID that has named nobody
        // for the whole grace period (`:188-190`).
        if self.creaking.is_some()
            && input.protector.is_none()
            && self.ticks_existed >= TICKS_GRACE_PERIOD
        {
            self.creaking = None;
            out.set_changed = true;
            out.notify_output_signal = true;
        }
        let computed = self.compute_analog_output_signal(pos, input.protector.map(|p| p.position));
        if self.output_signal != computed {
            self.output_signal = computed;
            out.notify_output_signal = true;
        }

        // `:79-96`.
        if self.emitter > 0 {
            if self.emitter > 50 {
                out.emit_particles.push(true);
                out.emit_particles.push(false);
            }
            if self.emitter % 10 == 0 {
                if let Some(protector) = input.protector {
                    self.emitter_target = Some(protector.bounding_box.center());
                }
                if let Some(target) = self.emitter_target {
                    let heart = Vec3d {
                        x: f64::from(pos.x) + 0.5,
                        y: f64::from(pos.y) + 0.5,
                        z: f64::from(pos.z) + 0.5,
                    };
                    let progress = 0.2_f64 + 0.8 * f64::from(100 - self.emitter) / 100.0;
                    let location = Vec3d {
                        x: target.x + (heart.x - target.x) * progress,
                        y: target.y + (heart.y - target.y) * progress,
                        z: target.z + (heart.z - target.z) * progress,
                    };
                    out.hurt_sound = Some((
                        BlockPos {
                            x: location.x.floor() as i32,
                            y: location.y.floor() as i32,
                            z: location.z.floor() as i32,
                        },
                        self.emitter as f32 / 2.0 / 100.0 + 0.5,
                    ));
                }
            }
            self.emitter -= 1;
        }

        // `:98-133`.
        let ticker_was = self.ticker;
        self.ticker -= 1;
        if ticker_was < 0 {
            self.ticker = 20 + random.next_int(5);
            let updated = update_creaking_state(
                state,
                self.creaking.is_some(),
                input.has_required_logs,
                input.creaking_active,
            );
            if updated != state {
                out.new_state = Some(updated);
                if property(updated, STATE).as_deref() == Some(UPROOTED) {
                    return out;
                }
            }
            let state_name = property(updated, STATE);
            if self.creaking.is_none() {
                if state_name.as_deref() == Some(AWAKE)
                    && input.spawning_monsters
                    && input.has_player_within_32
                {
                    out.spawn_protector = true;
                }
            } else if let Some(protector) = input.protector {
                if (!input.creaking_active && !protector.persistence_required)
                    || distance_to_creaking(pos, protector.position) > CREAKING_RELEASE_RADIUS
                    || protector.player_stuck
                {
                    out.remove_protector = true;
                }
            }
        }
        out
    }

    /// Whether the next `server_tick` will enter the `ticker-- < 0` check
    /// (`CreakingHeartBlockEntity.java:98`).
    #[must_use]
    pub fn ticker_due(&self) -> bool {
        self.ticker < 0
    }

    /// `CreakingHeartBlockEntity.setCreakingInfo(UUID)` (`:162-166`).
    pub fn bind_protector(&mut self, uuid: Uuid) {
        self.creaking = Some(NbtSummaryValue::IntArray(uuid_int_array(uuid)));
        self.ticks_existed = 0;
    }

    /// `CreakingHeartBlockEntity.clearCreakingInfo` (`:152-155`).
    pub fn clear_protector(&mut self) {
        self.creaking = None;
        self.ticks_existed = 0;
    }

    /// `CreakingHeartBlockEntity.creakingHurt`'s emitter window (`:225-243`):
    /// only a hit that arrives with the countdown already run down starts the
    /// trail and moves its target, and the caller then owns the particles and
    /// the resin, which need the level.
    pub fn creaking_hurt_start(&mut self, target: Vec3d) -> bool {
        if self.emitter > 0 {
            return false;
        }
        self.emitter = 100;
        self.emitter_target = Some(target);
        true
    }

    /// `CreakingHeartBlockEntity.creakingHurt`'s countdown start (`:240-241`),
    /// whose entry is the protector's own damage path.
    #[cfg(test)]
    pub fn set_emitter_for_test(&mut self, emitter: i32, target: Vec3d) {
        self.emitter = emitter;
        self.emitter_target = Some(target);
    }

    /// `CreakingHeartBlockEntity.computeAnalogOutputSignal` (`:333-341`).
    fn compute_analog_output_signal(&self, pos: BlockPos, protector: Option<Vec3d>) -> i32 {
        let Some(protector) = protector.filter(|_| self.creaking.is_some()) else {
            return 0;
        };
        let scaled = distance_to_creaking(pos, protector).clamp(0.0, CREAKING_ROAMING_RADIUS)
            / CREAKING_ROAMING_RADIUS;
        SIGNAL_MAX - (scaled * f64::from(SIGNAL_MAX)).floor() as i32
    }

    /// `CreakingHeartBlockEntity.setCreakingInfo(UUID)` (`:162-166`), which in
    /// Vanilla only `loadAdditional` and the unowned spawn reach.
    #[cfg(test)]
    pub fn set_protector_for_test(&mut self, uuid: Uuid) {
        self.bind_protector(uuid);
    }
}

/// `CreakingHeartBlockEntity.distanceToCreaking` (`:148-150`) over
/// `Entity.distanceToSqr(Vec3)` (`Entity.java`) and
/// `Vec3.atBottomCenterOf` (`Vec3.java`).
/// `CreakingHeartBlockEntity.preRemoveSideEffects` (`:306-309`) and
/// `removeProtector(null)` (`:311-321`): the level resolves the binding and
/// tears the protector down. An unbound heart answers `None`.
#[must_use]
pub fn removal_response(heart: &CreakingHeartBlockEntity) -> super::BlockEntityRemovalDrops {
    match heart.protector_uuid() {
        Some(uuid) => super::BlockEntityRemovalDrops::CreakingHeartProtector { uuid },
        None => super::BlockEntityRemovalDrops::None,
    }
}

/// `StateHolder.getValue` over a string property.
fn property(state: i32, name: &str) -> Option<String> {
    bbb_protocol::block_state_types::vanilla_block_state_property(state, name).map(str::to_owned)
}

fn distance_to_creaking(pos: BlockPos, protector: Vec3d) -> f64 {
    let dx = protector.x - (f64::from(pos.x) + 0.5);
    let dy = protector.y - f64::from(pos.y);
    let dz = protector.z - (f64::from(pos.z) + 0.5);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// `UUIDUtil.CODEC` (`UUIDUtil.java`): an int-stream of length 4.
fn uuid_codec_value(value: &NbtSummaryValue) -> bool {
    codec_uuid(value).is_some()
}

#[cfg(test)]
mod tests {

    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_CREAKING_HEART_ID;
    use bbb_protocol::packets::{BlockPos, Vec3d};
    use bbb_protocol::NbtSummaryValue;
    use uuid::Uuid;

    use super::{
        entry, CreakingHeartAnalog, CreakingHeartAnalogTick, CreakingHeartBlockEntity,
        TAG_CREAKING, TICKS_GRACE_PERIOD,
    };
    use crate::block_entity::{new_block_entity, unique_field};
    use crate::random::GameRandom;

    fn pos() -> BlockPos {
        BlockPos { x: 2, y: 3, z: 4 }
    }

    fn heart_state() -> i32 {
        vanilla_block_default_state_id(VANILLA_BLOCK_CREAKING_HEART_ID)
            .expect("minecraft:creaking_heart has a default state")
    }

    fn creaking_uuid() -> NbtSummaryValue {
        NbtSummaryValue::IntArray(vec![10, 20, 30, 40])
    }

    fn protector() -> Uuid {
        Uuid::from_u64_pair(0x0123_4567_89ab_cdef, 0xfedc_ba98_7654_3210)
    }

    fn bound_heart() -> CreakingHeartBlockEntity {
        let mut heart = CreakingHeartBlockEntity::default();
        heart.set_protector_for_test(protector());
        heart
    }

    fn state_with(value: &str) -> i32 {
        crate::fluid::with_property(heart_state(), super::STATE, value)
            .expect("minecraft:creaking_heart carries its state property")
    }

    /// A protector standing on the heart's own block, so the release distance
    /// test measures the state under test rather than the fixture.
    fn protector_state() -> super::CreakingHeartProtectorState {
        super::CreakingHeartProtectorState {
            position: Vec3d {
                x: 2.5,
                y: 5.0,
                z: 4.5,
            },
            bounding_box: bbb_server_types::geometry::Aabb {
                min_x: 2.0,
                min_y: 5.0,
                min_z: 4.0,
                max_x: 3.0,
                max_y: 8.0,
                max_z: 5.0,
            },
            persistence_required: false,
            player_stuck: false,
        }
    }

    fn input(has_required_logs: bool, creaking_active: bool) -> super::CreakingHeartTickInput {
        super::CreakingHeartTickInput {
            has_required_logs: Some(has_required_logs),
            creaking_active,
            spawning_monsters: true,
            has_player_within_32: true,
            protector: None,
        }
    }

    /// `updateCreakingState` (`:137-146`): missing logs uproot an unbound
    /// heart, and the attribute picks awake or dormant otherwise.
    #[test]
    fn the_state_follows_the_logs_and_the_attribute() {
        let random = GameRandom::with_seed(0);
        let uprooted = state_with("uprooted");
        let dormant = state_with("dormant");
        // Each pass starts with the ticker already run out (`:98`).
        let pass = |heart: &mut CreakingHeartBlockEntity,
                    state: i32,
                    input: super::CreakingHeartTickInput| {
            heart.ticker = -1;
            heart.server_tick(pos(), state, input, &random).new_state
        };
        assert_eq!(
            pass(
                &mut CreakingHeartBlockEntity::default(),
                dormant,
                input(false, true)
            ),
            Some(uprooted),
            "missing logs uproot an unbound heart"
        );
        assert_eq!(
            pass(
                &mut CreakingHeartBlockEntity::default(),
                uprooted,
                input(false, true)
            ),
            None,
            "a heart already uprooted needs no write"
        );
        assert_eq!(
            pass(
                &mut CreakingHeartBlockEntity::default(),
                uprooted,
                input(true, false)
            ),
            Some(dormant),
            "the attribute picks dormant outside its window"
        );
        assert_eq!(
            pass(
                &mut CreakingHeartBlockEntity::default(),
                dormant,
                input(true, true)
            ),
            Some(state_with("awake"))
        );
        // A bound heart keeps its state through missing logs (`:138`).
        assert_eq!(
            pass(&mut bound_heart(), dormant, input(false, false)),
            None,
            "a bound heart stays where it is"
        );
    }

    /// `:108-116`: only an awake heart with monsters spawning and a player in
    /// range spawns, and only while unbound.
    #[test]
    fn only_an_awake_unbound_heart_with_a_nearby_player_spawns() {
        let heart = CreakingHeartBlockEntity {
            ticker: -1,
            ..CreakingHeartBlockEntity::default()
        };
        let random = GameRandom::with_seed(0);
        let awake = state_with("awake");
        let spawns = |input: super::CreakingHeartTickInput| {
            let mut heart = heart.clone();
            heart.ticker = -1;
            heart
                .server_tick(pos(), awake, input, &random)
                .spawn_protector
        };
        assert!(spawns(input(true, true)));
        let mut no_monsters = input(true, true);
        no_monsters.spawning_monsters = false;
        assert!(!spawns(no_monsters));
        let mut no_player = input(true, true);
        no_player.has_player_within_32 = false;
        assert!(!spawns(no_player));
        let mut bound = heart.clone();
        bound.set_protector_for_test(protector());
        assert!(
            !bound
                .server_tick(pos(), awake, input(true, true), &random)
                .spawn_protector
        );
    }

    /// `:117-125`: the three release conditions.
    #[test]
    fn a_bound_heart_releases_on_the_attribute_distance_or_a_stuck_player() {
        let awake = state_with("awake");
        let random = GameRandom::with_seed(0);
        let release = |input: super::CreakingHeartTickInput| {
            let mut heart = bound_heart();
            heart.ticker = -1;
            heart
                .server_tick(pos(), awake, input, &random)
                .remove_protector
        };
        let mut active = input(true, true);
        active.protector = Some(protector_state());
        assert!(!release(active));
        let mut inactive = input(true, false);
        inactive.protector = Some(protector_state());
        assert!(release(inactive), "the attribute clearing releases");
        let mut persistent = input(true, false);
        persistent.protector = Some(super::CreakingHeartProtectorState {
            persistence_required: true,
            ..protector_state()
        });
        assert!(!release(persistent), "a persistent protector stays");
        let mut far = input(true, true);
        far.protector = Some(super::CreakingHeartProtectorState {
            position: Vec3d {
                x: 40.5,
                y: 64.0,
                z: 40.5,
            },
            bounding_box: bbb_server_types::geometry::Aabb {
                min_x: 40.0,
                min_y: 64.0,
                min_z: 40.0,
                max_x: 41.0,
                max_y: 67.0,
                max_z: 41.0,
            },
            ..protector_state()
        });
        assert!(release(far), "34 blocks is the release distance");
        let mut stuck = input(true, true);
        stuck.protector = Some(super::CreakingHeartProtectorState {
            player_stuck: true,
            ..protector_state()
        });
        assert!(release(stuck));
    }

    /// `:79-96`: the emitter runs down with two particle bursts above fifty and
    /// a hurt sound every ten ticks.
    #[test]
    fn the_emitter_counts_down_with_particles_and_a_hurt_sound() {
        let mut heart = bound_heart();
        heart.ticker = 100;
        heart.set_emitter_for_test(
            100,
            Vec3d {
                x: 1.5,
                y: 65.5,
                z: 1.5,
            },
        );
        let random = GameRandom::with_seed(0);
        let mut input = input(true, true);
        input.protector = Some(protector_state());
        let first = heart.server_tick(pos(), state_with("awake"), input, &random);
        assert_eq!(first.emit_particles, vec![true, false]);
        assert_eq!(
            first.hurt_sound,
            Some((BlockPos { x: 2, y: 5, z: 4 }, 1.0)),
            "a hundred is a tenth tick too"
        );
        assert_eq!(heart.emitter, 99);
        heart.emitter = 50;
        let tenth = heart.server_tick(pos(), state_with("awake"), input, &random);
        assert!(tenth.emit_particles.is_empty(), "fifty is not above fifty");
        let (sound_pos, volume) = tenth.hurt_sound.expect("fifty is a tenth tick");
        // `0.2 + 0.8 * (100 - 50) / 100` of the way from the protector back to
        // the heart centre: y runs 5.5 -> 3.5 by 0.6, which floors to 4.
        assert_eq!(volume, 50.0 / 2.0 / 100.0 + 0.5);
        assert_eq!(sound_pos, BlockPos { x: 2, y: 4, z: 4 });
    }

    /// `CreakingHeartBlockEntity.preRemoveSideEffects` (`:306-309`).
    #[test]
    fn only_a_bound_heart_answers_on_removal() {
        let heart = CreakingHeartBlockEntity::default();
        assert_eq!(
            super::removal_response(&heart),
            super::super::BlockEntityRemovalDrops::None
        );
        let mut bound = CreakingHeartBlockEntity::default();
        bound.set_protector_for_test(protector());
        assert_eq!(
            super::removal_response(&bound),
            super::super::BlockEntityRemovalDrops::CreakingHeartProtector { uuid: protector() }
        );
    }

    /// `CreakingHeartBlockEntity.computeAnalogOutputSignal` (`:333-341`) over
    /// `distanceToCreaking` (`:148-150`): full strength on the heart's own
    /// bottom centre, and nothing at the roaming radius or past it.
    #[test]
    fn a_bound_protector_scales_the_analog_output_by_its_distance() {
        let heart_pos = BlockPos { x: 8, y: 64, z: -3 };
        let bottom_centre = Vec3d {
            x: 8.5,
            y: 64.0,
            z: -2.5,
        };
        for (offset, expected) in [
            (0.0, 15),
            (1.0, 15),
            (2.2, 14),
            (16.0, 8),
            (31.999, 1),
            (32.0, 0),
            (48.0, 0),
        ] {
            let mut heart = bound_heart();
            let outcome = heart.server_tick_analog(
                heart_pos,
                Some(Vec3d {
                    x: bottom_centre.x + offset,
                    ..bottom_centre
                }),
            );
            assert_eq!(
                heart.analog(),
                CreakingHeartAnalog {
                    protector_bound: true,
                    output_signal: expected,
                },
                "{offset} blocks away"
            );
            assert_eq!(outcome.notify_output_signal, expected != 0);
            assert!(!outcome.set_changed);
        }
    }

    /// The distance is a real one: a protector offset on all three axes reads
    /// the same as one the same euclidean distance along a single axis.
    #[test]
    fn the_distance_the_signal_scales_by_is_euclidean() {
        let heart_pos = BlockPos { x: 0, y: 0, z: 0 };
        let mut diagonal = bound_heart();
        diagonal.server_tick_analog(
            heart_pos,
            Some(Vec3d {
                x: 0.5 + 6.0,
                y: 3.0,
                z: 0.5 + 2.0,
            }),
        );
        let mut axial = bound_heart();
        axial.server_tick_analog(
            heart_pos,
            Some(Vec3d {
                x: 0.5 + 7.0,
                y: 0.0,
                z: 0.5,
            }),
        );
        assert_eq!(diagonal.analog().output_signal, 12);
        assert_eq!(axial.analog().output_signal, 12);
    }

    /// `getCreakingProtector` (`:168-197`) answers an unresolved UUID with
    /// nobody, and only drops the binding once the grace period (`:57`) has
    /// run out; `clearCreakingInfo` (`:152-155`) is what marks the record.
    #[test]
    fn an_unresolved_protector_reads_zero_until_the_grace_period_drops_it() {
        let heart_pos = BlockPos { x: 1, y: 2, z: 3 };
        let mut heart = bound_heart();
        for tick in 1..TICKS_GRACE_PERIOD {
            assert_eq!(
                heart.server_tick_analog(heart_pos, None),
                CreakingHeartAnalogTick::default(),
                "tick {tick}"
            );
            assert!(heart.analog().protector_bound);
            assert_eq!(heart.analog().output_signal, 0);
        }
        assert_eq!(
            heart.server_tick_analog(heart_pos, None),
            CreakingHeartAnalogTick {
                set_changed: true,
                notify_output_signal: true,
            }
        );
        assert_eq!(
            heart.analog(),
            CreakingHeartAnalog {
                protector_bound: false,
                output_signal: 0,
            }
        );
        assert_eq!(heart.protector_uuid(), None);
    }

    /// A heart that never had a protector is not a heart whose protector left:
    /// `computeAnalogOutputSignal`'s first test (`:334`) is the binding, and
    /// the block seam refuses over it rather than reading its zero.
    #[test]
    fn an_unbound_heart_stays_unbound_and_reads_zero() {
        let heart_pos = BlockPos { x: 4, y: 5, z: 6 };
        let mut heart = CreakingHeartBlockEntity::default();
        let near = Vec3d {
            x: 4.5,
            y: 5.0,
            z: 6.5,
        };
        for _ in 0..TICKS_GRACE_PERIOD + 1 {
            assert_eq!(
                heart.server_tick_analog(heart_pos, Some(near)),
                CreakingHeartAnalogTick::default()
            );
        }
        assert_eq!(
            heart.analog(),
            CreakingHeartAnalog {
                protector_bound: false,
                output_signal: 0,
            }
        );
    }

    /// `loadAdditional` (`:344-347`) dates the object from the load, so a save
    /// restored beside a creaking that is still coming back does not lose its
    /// binding on the first tick.
    #[test]
    fn a_loaded_binding_starts_its_grace_period_again() {
        let heart_pos = pos();
        let state = heart_state();
        let mut record = new_block_entity(heart_pos, state)
            .expect("a creaking heart builds")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        record.push(entry(TAG_CREAKING, creaking_uuid()));
        let mut spent = CreakingHeartBlockEntity::default();
        spent.set_protector_for_test(protector());
        for _ in 0..TICKS_GRACE_PERIOD {
            spent.server_tick_analog(heart_pos, None);
        }
        assert!(!spent.analog().protector_bound);

        let mut loaded = crate::block_entity::load_test_fixture(heart_pos, state, &record)
            .expect("fixture codec succeeds")
            .expect("a heart loads");
        let heart = loaded
            .creaking_heart_mut()
            .expect("the record names a creaking heart");
        for _ in 1..TICKS_GRACE_PERIOD {
            heart.server_tick_analog(heart_pos, None);
        }
        assert!(heart.analog().protector_bound);
    }

    #[test]
    fn a_creaking_heart_round_trips_its_protector_uuid() {
        let state = heart_state();
        let mut record = new_block_entity(pos(), state)
            .expect("a creaking heart builds")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        record.push(entry(TAG_CREAKING, creaking_uuid()));
        let loaded = crate::block_entity::load_test_fixture(pos(), state, &record)
            .expect("fixture codec succeeds")
            .expect("a heart loads");
        let saved = loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        let again = crate::block_entity::load_test_fixture(pos(), state, &saved)
            .expect("fixture codec succeeds")
            .expect("the round-trip still names a creaking heart")
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        assert_eq!(unique_field(&saved, TAG_CREAKING), Some(&creaking_uuid()));
        assert_eq!(unique_field(&again, TAG_CREAKING), Some(&creaking_uuid()));
        assert_eq!(
            unique_field(
                &loaded
                    .update_tag_with_resolver(crate::block_entity::test_item_resolver())
                    .expect("fixture codec succeeds"),
                TAG_CREAKING
            ),
            Some(&creaking_uuid())
        );
    }
}
