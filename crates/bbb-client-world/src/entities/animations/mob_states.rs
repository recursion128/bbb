use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PolarBearStandingAnimationState {
    pub target_standing: bool,
    pub previous_ticks: f32,
    pub current_ticks: f32,
    pub dimensions_ticks: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShulkerPeekAnimationState {
    pub target_peek_amount: f32,
    pub previous_peek_amount: f32,
    pub current_peek_amount: f32,
}

/// Vanilla Shulker's client-only attach-position transition. Shulkers do not
/// use the generic entity interpolation handler: a block-position change snaps
/// the canonical transform and renders a six-tick quadratic offset from the
/// previous attachment block instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShulkerTeleportAnimationState {
    pub old_attach_position: Option<[i32; 3]>,
    pub interpolation_ticks: u8,
}

/// Canonical client-side bee barrel-roll animation, mirroring vanilla
/// `Bee.updateRollAmount`. While the synced `FLAG_ROLL` (mask `2` in the byte resolved through the
/// generated Bee flags accessor) is set, `rollAmount` climbs toward
/// `1` by `0.2`/tick; otherwise it falls toward `0` by `0.24`/tick.
/// `getRollAmount(partialTick)` lerps the pair, driving `BeeModel.setupAnim`'s
/// near-π `bone.xRot` flip that tips a rolling bee onto its back.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BeeRollAnimationState {
    pub rolling: bool,
    pub previous_roll_amount: f32,
    pub current_roll_amount: f32,
}

/// Canonical client-side panda sit/on-back/roll accumulators, mirroring vanilla
/// `Panda.sitAmount` / `onBackAmount` / `rollAmount` and `rollCounter`. The
/// synced `DATA_ID_FLAGS` byte selects the three targets; each amount rises by
/// `0.15`/tick while active and falls by `0.19`/tick otherwise. `rollCounter`
/// increments while rolling and clears the local rolling target after tick 32,
/// matching `Panda.handleRoll`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PandaAnimationState {
    pub sitting: bool,
    pub on_back: bool,
    pub rolling: bool,
    pub rolling_locally_cleared: bool,
    pub previous_sit_amount: f32,
    pub current_sit_amount: f32,
    pub previous_on_back_amount: f32,
    pub current_on_back_amount: f32,
    pub previous_roll_amount: f32,
    pub current_roll_amount: f32,
    pub roll_counter: i32,
}

/// Canonical client-side triggered keyframe-animation state, mirroring vanilla
/// `net.minecraft.world.entity.AnimationState`: a one-shot timer that records the
/// `age_ticks` it started at and is cleared (`None`) when stopped. The renderer
/// projects the elapsed seconds since the start and wraps/samples the matching
/// `KeyframeAnimation` definition. Reusable for the whole triggered-keyframe tier
/// (the frog's croak is the first consumer; the warden/sniffer/camel/armadillo
/// triggered poses follow the same pattern). `Default` is the stopped sentinel
/// (`start_age: None`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyframeAnimationState {
    /// Vanilla `AnimationState.startTick` recorded at the rising edge (`age_ticks`
    /// when the trigger condition first became true). `None` is vanilla's
    /// "not started" sentinel (`startTick == NOT_STARTED`).
    pub start_age: Option<u32>,
}

impl KeyframeAnimationState {
    /// Vanilla `AnimationState.animateWhen(condition, tickCount)`: starts the timer
    /// on the rising edge (`startIfStopped`, so a still-running timer keeps its
    /// original start) and stops it (clearing `start_age`) once the condition drops.
    pub(super) fn animate_when(&mut self, condition: bool, age_ticks: u32) {
        if condition {
            if self.start_age.is_none() {
                self.start_age = Some(age_ticks);
            }
        } else {
            self.start_age = None;
        }
    }

    /// Vanilla `AnimationState.getTimeInMillis(ageInTicks)` / `KeyframeAnimation`'s
    /// `getElapsedSeconds`: `((ageInTicks - startTick) * 50) / 1000` = elapsed ticks
    /// / 20, with the partial tick folded into the live age. `None` while stopped.
    pub(super) fn elapsed_seconds(self, age_ticks: u32, partial_tick: f32) -> Option<f32> {
        self.start_age
            .map(|start| ((age_ticks - start) as f32 + partial_tick) / 20.0)
    }
}

/// Vanilla `Camel.idleAnimationTimeout` plus `idleAnimationState`. The exact client RNG seed is not
/// protocol-visible, so bbb uses the same deterministic Java LCG shape used for equine tail idle
/// starts, seeded from the entity id while preserving vanilla's `nextInt(40) + 80` interval and
/// unconditional `AnimationState.start(tickCount)` restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CamelIdleAnimationState {
    pub(super) random_seed: u64,
    pub(super) timeout_ticks: i32,
    pub(super) animation: KeyframeAnimationState,
}

impl CamelIdleAnimationState {
    pub(super) fn new(entity_id: i32) -> Self {
        Self {
            random_seed: java_random_seed(i64::from(entity_id).wrapping_add(0xCAFE)),
            timeout_ticks: 0,
            animation: KeyframeAnimationState { start_age: None },
        }
    }

    pub(super) fn advance_client_tick(&mut self, age_ticks: u32) {
        if self.timeout_ticks <= 0 {
            self.timeout_ticks =
                java_random_next_int_bound(&mut self.random_seed, CAMEL_IDLE_TIMEOUT_RANDOM_BOUND)
                    + CAMEL_IDLE_TIMEOUT_BASE_TICKS;
            self.animation.start_age = Some(age_ticks);
        } else {
            self.timeout_ticks -= 1;
        }
    }

    pub(super) fn elapsed_seconds(self, age_ticks: u32, partial_tick: f32) -> Option<f32> {
        self.animation.elapsed_seconds(age_ticks, partial_tick)
    }
}

/// Vanilla `CopperGolem.idleAnimationStartTick` plus `idleAnimationState`. The exact
/// `RandomSource` seed is client-local, so bbb uses the same deterministic Java LCG shape used by
/// other client-only idle timers while preserving vanilla's `random.nextInt(200, 240)` interval, the
/// delayed first start, and the reset 10 ticks after the head-spin sound point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopperGolemIdleAnimationState {
    pub(super) random_seed: u64,
    pub(super) idle_animation_start_tick: i32,
    pub(super) animation: KeyframeAnimationState,
}

impl CopperGolemIdleAnimationState {
    pub(super) fn new(entity_id: i32) -> Self {
        Self {
            random_seed: java_random_seed(i64::from(entity_id).wrapping_add(0xC0_6E)),
            idle_animation_start_tick: 0,
            animation: KeyframeAnimationState { start_age: None },
        }
    }

    /// Advances the idle head-spin state machine (`CopperGolem.java:318-327`),
    /// returning `true` on the tick where Vanilla plays the oxidation-dependent
    /// local spin sound (`age_ticks == idle_animation_start_tick + 10`).
    pub(super) fn advance_client_tick(&mut self, age_ticks: u32, is_idle: bool) -> bool {
        if !is_idle {
            self.idle_animation_start_tick = 0;
            self.animation.start_age = None;
            return false;
        }

        let tick = i32::try_from(age_ticks).unwrap_or(i32::MAX);
        if self.idle_animation_start_tick == tick {
            self.animation.start_age = Some(age_ticks);
        } else if self.idle_animation_start_tick == 0 {
            self.idle_animation_start_tick =
                tick + java_random_next_int_bound(
                    &mut self.random_seed,
                    COPPER_GOLEM_IDLE_TIMEOUT_RANDOM_BOUND,
                ) + COPPER_GOLEM_IDLE_TIMEOUT_BASE_TICKS;
        }

        if tick == self.idle_animation_start_tick + 10 {
            self.idle_animation_start_tick = 0;
            return true;
        }
        false
    }

    pub(super) fn elapsed_seconds(self, age_ticks: u32, partial_tick: f32) -> Option<f32> {
        self.animation.elapsed_seconds(age_ticks, partial_tick)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CamelDashCooldownState {
    pub(super) cooldown_ticks: i32,
    pub(super) was_dashing: bool,
}

impl CamelDashCooldownState {
    pub(super) fn advance_client_tick(&mut self, is_dashing: bool) {
        if is_dashing && !self.was_dashing && self.cooldown_ticks == 0 {
            self.cooldown_ticks = CAMEL_DASH_COOLDOWN_TICKS;
        }
        self.was_dashing = is_dashing;
        if self.cooldown_ticks > 0 {
            self.cooldown_ticks -= 1;
        }
    }

    pub(super) fn start_local_dash_after_tick(&mut self) {
        // The native input owner runs after the common entity tick. Seed the
        // end-of-Camel.tick value (`55` set by executeRidersJump, then `--`).
        self.cooldown_ticks = CAMEL_DASH_COOLDOWN_TICKS - 1;
        self.was_dashing = true;
    }

    pub(super) fn stop_local_dash(&mut self) {
        self.was_dashing = false;
    }

    pub(crate) fn cooldown_ticks(self) -> i32 {
        self.cooldown_ticks
    }

    pub(super) fn should_clear_before_tick(
        self,
        is_dashing: bool,
        on_ground: bool,
        in_liquid: bool,
        is_passenger: bool,
    ) -> bool {
        is_dashing && self.cooldown_ticks < 50 && (on_ground || in_liquid || is_passenger)
    }

    pub(super) fn jump_cooldown(self, partial_tick: f32) -> f32 {
        (self.cooldown_ticks as f32 - partial_tick).max(0.0)
    }

    pub(super) fn is_settled(self) -> bool {
        self.cooldown_ticks == 0 && !self.was_dashing
    }
}

/// Vanilla `AbstractNautilus.dashCooldown`, retained on the canonical entity so a released local
/// dash, a synchronized remote dash, and immutable generations all observe one owner.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NautilusDashCooldownState {
    pub(super) cooldown_ticks: i32,
}

/// Vanilla `ItemBasedSteering` keeps the elapsed boost cursor outside
/// synchronized entity data. It is retained with the canonical entity so
/// publication A cannot observe generation B's ridden tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ItemBasedSteeringClientState {
    pub(super) boosting: bool,
    pub(super) boost_time: i32,
}

impl ItemBasedSteeringClientState {
    pub(super) fn on_synced(&mut self) {
        self.boosting = true;
        self.boost_time = 0;
    }

    pub(super) fn tick_and_factor(&mut self, boost_time_total: i32) -> f32 {
        if self.boosting {
            let previous = self.boost_time;
            self.boost_time = self.boost_time.wrapping_add(1);
            if previous > boost_time_total {
                self.boosting = false;
            }
        }
        if self.boosting {
            let phase = self.boost_time as f32 / boost_time_total as f32 * std::f32::consts::PI;
            1.0 + 1.15 * super::super::client_movement::vanilla_mth_sin(f64::from(phase))
        } else {
            1.0
        }
    }

    #[cfg(test)]
    pub(crate) fn debug_state(self) -> (bool, i32) {
        (self.boosting, self.boost_time)
    }
}

impl NautilusDashCooldownState {
    pub(super) fn seed_from_synced_update(&mut self) {
        if self.cooldown_ticks == 0 {
            self.cooldown_ticks = NAUTILUS_DASH_COOLDOWN_TICKS;
        }
    }

    pub(super) fn start_local_dash(&mut self) {
        self.cooldown_ticks = NAUTILUS_DASH_COOLDOWN_TICKS;
    }

    pub(super) fn advance_subclass_tick(&mut self, is_dashing: bool) -> bool {
        let clear_dash = is_dashing
            && self.cooldown_ticks
                < NAUTILUS_DASH_COOLDOWN_TICKS - NAUTILUS_DASH_MINIMUM_DURATION_TICKS;
        if self.cooldown_ticks > 0 {
            self.cooldown_ticks -= 1;
        }
        clear_dash
    }

    pub(crate) fn cooldown_ticks(self) -> i32 {
        self.cooldown_ticks
    }

    pub(super) fn cooldown(self, partial_tick: f32) -> f32 {
        (self.cooldown_ticks as f32 - partial_tick).max(0.0)
    }

    pub(super) fn is_settled(self) -> bool {
        self.cooldown_ticks == 0
    }
}

/// Returns the `Sniffer.State` ordinal whose one-shot `AnimationState` the renderer drives, or
/// `None` for a state with no triggered keyframe (`IDLING` rests at the bind pose, `SEARCHING` runs
/// the looping search-walk handled by the walk path, and an unknown ordinal is treated as idle).
pub(super) fn sniffer_animated_state(state_id: i32) -> Option<i32> {
    match state_id {
        SNIFFER_STATE_FEELING_HAPPY_ID
        | SNIFFER_STATE_SCENTING_ID
        | SNIFFER_STATE_SNIFFING_ID
        | SNIFFER_STATE_DIGGING_ID
        | SNIFFER_STATE_RISING_ID => Some(state_id),
        _ => None,
    }
}

/// Canonical client-side sniffer animation state, mirroring vanilla `Sniffer.onSyncedDataUpdated`:
/// the synced `DATA_STATE` (the mutually-exclusive `Sniffer.State`) drives one one-shot
/// `AnimationState` at a time. On every `DATA_STATE` change vanilla calls `resetAnimations()` (stops
/// all five) then `startIfStopped` on the new state's animation, so a state transition restarts the
/// timer from `0`. We track the current state ordinal and the [`KeyframeAnimationState`] timer that
/// the renderer samples; `IDLING`/`SEARCHING` and unknown states clear the timer (no triggered
/// keyframe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnifferAnimationState {
    /// The `Sniffer.State` ordinal whose `AnimationState` is active (`feeling_happy`/`scenting`/
    /// `sniffing`/`digging`/`rising`), or `IDLING` (the bind-pose rest) when none is running.
    pub state_id: i32,
    /// The triggered timer for the active state's keyframe animation, started at the `age_ticks` of
    /// the `DATA_STATE` transition. Stopped (`start_age == None`) for `IDLING`/`SEARCHING`.
    pub keyframe: KeyframeAnimationState,
}

impl SnifferAnimationState {
    /// Vanilla `Sniffer.onSyncedDataUpdated(DATA_STATE)`: `resetAnimations()` then `startIfStopped`
    /// on the new state's animation. A change to a different ordinal restarts the timer at the
    /// current age (vanilla `.start(tickCount)` semantics on the transition); a redundant re-set to
    /// the same state keeps the running timer.
    pub(super) fn set_state(&mut self, state_id: i32, age_ticks: u32) {
        if state_id == self.state_id {
            return;
        }
        self.state_id = state_id;
        self.keyframe.start_age = sniffer_animated_state(state_id).map(|_| age_ticks);
    }

    /// Vanilla `Sniffer.isSearching()` (`getState() == SEARCHING`): whether the synced `DATA_STATE`
    /// is `SEARCHING`, which `SnifferModel.setupAnim` uses to swap the base walk for the looping
    /// `SNIFFER_SNIFF_SEARCH` search-walk.
    pub(super) fn is_searching(self) -> bool {
        self.state_id == SNIFFER_STATE_SEARCHING_ID
    }

    /// The active state's elapsed seconds for the renderer (`-1.0` when no triggered animation is
    /// running), paired with the projected state id.
    pub(super) fn animation(self, age_ticks: u32, partial_tick: f32) -> (i32, f32) {
        match self
            .keyframe
            .elapsed_seconds(age_ticks, partial_tick)
            .and_then(|seconds| sniffer_animated_state(self.state_id).map(|id| (id, seconds)))
        {
            Some((id, seconds)) => (id, seconds),
            None => (-1, -1.0),
        }
    }
}

/// Vanilla `Armadillo.ArmadilloState.shouldHideInShell(ticksInState)`. The body is hidden in the
/// shell ball (`ArmadilloModel.setupAnim`'s `isHidingInShell` branch) for the whole steady SCARED
/// state, never while IDLE, and for an `inStateTicks` window during the ROLLING/UNROLLING
/// transitions (curl-in completes after tick 5; the unroll keeps the ball until tick 26).
pub(super) fn armadillo_should_hide_in_shell(state_id: i32, in_state_ticks: u32) -> bool {
    match state_id {
        ARMADILLO_STATE_ROLLING_ID => in_state_ticks > 5,
        ARMADILLO_STATE_SCARED_ID => true,
        ARMADILLO_STATE_UNROLLING_ID => in_state_ticks < 26,
        // IDLE and any unknown ordinal stay unrolled.
        _ => false,
    }
}

/// Canonical client-side armadillo roll/peek animation state, mirroring vanilla
/// `Armadillo.setupAnimationStates()`. The synced `ARMADILLO_STATE` (the `ArmadilloState` enum)
/// plus the `inStateTicks` counter (vanilla `Armadillo.inStateTicks`, reset to `0` on a state
/// change and `++` each tick) drive both the `isHidingInShell` shell-ball swap and the three
/// triggered keyframe `AnimationState`s: `rollUp` on entry to ROLLING, `rollOut` on entry to
/// UNROLLING, `peek` while SCARED. We reconstruct `inStateTicks` from the `age_ticks` recorded when
/// the state last changed, and reuse [`KeyframeAnimationState`] for the rollUp/rollOut elapsed
/// timers (started at the state-entry age, vanilla's `.startIfStopped(tickCount)`). The `peek`
/// timer stores a signed start tick because vanilla `fastForward(50, 1.0F)` subtracts 50 ticks from
/// `AnimationState.startTick`, which can make the first client tick's start negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmadilloAnimationState {
    /// The current `ArmadilloState` ordinal id (the synced `ARMADILLO_STATE`).
    pub state_id: i32,
    /// The `age_ticks` recorded when the state last changed; `in_state_ticks = age_ticks - this`
    /// reconstructs vanilla `Armadillo.inStateTicks`.
    pub state_change_age: u32,
    /// The `ARMADILLO_ROLL_UP` one-shot, started on the rising edge into ROLLING.
    pub roll_up: KeyframeAnimationState,
    /// The `ARMADILLO_ROLL_OUT` one-shot, started on the rising edge into UNROLLING.
    pub roll_out: KeyframeAnimationState,
    /// The `ARMADILLO_PEEK` one-shot, active while SCARED. Signed for vanilla fast-forward.
    #[serde(default)]
    pub peek_start_age: Option<i32>,
    /// Vanilla `peekReceivedClient`, set by entity event `64` and consumed by the next SCARED setup.
    #[serde(default)]
    pub peek_received_client: bool,
}

impl ArmadilloAnimationState {
    pub(super) fn new(state_id: i32, age_ticks: u32) -> Self {
        let mut state = Self {
            state_id,
            state_change_age: age_ticks,
            roll_up: KeyframeAnimationState { start_age: None },
            roll_out: KeyframeAnimationState { start_age: None },
            peek_start_age: None,
            peek_received_client: false,
        };
        state.setup_animation_states(age_ticks);
        state
    }

    /// Vanilla `Armadillo.switchToState` + `setupAnimationStates`: a change to a different ordinal
    /// resets `inStateTicks` (so we re-anchor `state_change_age`) and `.startIfStopped`s the new
    /// state's transition timer (rollUp into ROLLING, rollOut into UNROLLING, peek while SCARED);
    /// the other timers stop. A redundant re-set to the same state keeps the running timers.
    pub(super) fn set_state(&mut self, state_id: i32, age_ticks: u32) {
        if state_id == self.state_id {
            return;
        }
        self.state_id = state_id;
        self.state_change_age = age_ticks;
        self.setup_animation_states(age_ticks);
    }

    pub(super) fn in_state_ticks(self, age_ticks: u32) -> u32 {
        age_ticks.saturating_sub(self.state_change_age)
    }

    pub(super) fn setup_animation_states(&mut self, age_ticks: u32) {
        match self.state_id {
            ARMADILLO_STATE_ROLLING_ID => {
                self.roll_out.start_age = None;
                if self.roll_up.start_age.is_none() {
                    self.roll_up.start_age = Some(age_ticks);
                }
                self.peek_start_age = None;
            }
            ARMADILLO_STATE_SCARED_ID => {
                self.roll_out.start_age = None;
                self.roll_up.start_age = None;
                if self.peek_received_client {
                    self.peek_start_age = None;
                    self.peek_received_client = false;
                }
                if self.in_state_ticks(age_ticks) == 0 {
                    self.peek_start_age =
                        Some(age_ticks as i32 - ARMADILLO_STATE_SCARED_ANIMATION_TICKS);
                } else if self.peek_start_age.is_none() {
                    self.peek_start_age = Some(age_ticks as i32);
                }
            }
            ARMADILLO_STATE_UNROLLING_ID => {
                if self.roll_out.start_age.is_none() {
                    self.roll_out.start_age = Some(age_ticks);
                }
                self.roll_up.start_age = None;
                self.peek_start_age = None;
            }
            _ => {
                self.roll_out.start_age = None;
                self.roll_up.start_age = None;
                self.peek_start_age = None;
            }
        }
    }

    /// Vanilla `Armadillo.shouldHideInShell()` = `getState().shouldHideInShell(inStateTicks)`,
    /// projected for the renderer `isHidingInShell` shell-ball swap.
    pub(super) fn is_hiding_in_shell(self, age_ticks: u32) -> bool {
        armadillo_should_hide_in_shell(self.state_id, self.in_state_ticks(age_ticks))
    }

    pub(super) fn peek_elapsed_seconds(self, age_ticks: u32, partial_tick: f32) -> Option<f32> {
        self.peek_start_age
            .map(|start| (age_ticks as f32 - start as f32 + partial_tick) / 20.0)
    }
}

/// Canonical client-side fox accumulators, mirroring vanilla `Fox.tick`'s two
/// eased fields. While the synced `FLAG_INTERESTED` (mask `8`) is set,
/// `interestedAngle` eases toward `1` by `* 0.4`/tick (and toward `0` otherwise);
/// `getHeadRollAngle(pt) = lerp(pt, interestedAngleO, interestedAngle) * 0.11 * π`
/// drives the head tilt. While `FLAG_CROUCHING` (mask `4`) is set, `crouchAmount`
/// climbs by `0.2`/tick (clamped to `5.0`); otherwise it is reset INSTANTLY to
/// `0` (vanilla's non-crouching branch is an assignment, not a decay).
/// `getCrouchAmount(pt) = lerp(pt, crouchAmountO, crouchAmount)` drives
/// `FoxModel.setCrouchingPose`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FoxAnimationState {
    pub interested: bool,
    pub previous_interested_angle: f32,
    pub current_interested_angle: f32,
    pub crouching: bool,
    pub previous_crouch_amount: f32,
    pub current_crouch_amount: f32,
}

/// Canonical client-side wolf wet/drying shade state, mirroring the `Wolf.tick` fields that
/// `Wolf.getWetShade(partialTick)` reads. While `isInWaterOrRain()` is true, vanilla marks the wolf
/// wet and `getWetShade` returns the `0.75` floor. The server broadcasts event `8` after leaving
/// water when grounded/path-free and event `56` when water/rain interrupts a shake; the client only
/// applies those events, then increments `shakeAnim` by `0.05` until the wet shade has lerped back to
/// white. The same `shakeAnim` is also projected for `WolfRenderState.getBodyRollAngle`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WolfWetAnimationState {
    pub is_wet: bool,
    pub is_shaking: bool,
    pub previous_shake_anim: f32,
    pub current_shake_anim: f32,
}

impl WolfWetAnimationState {
    pub(super) fn start_shaking(&mut self) {
        self.is_shaking = true;
        self.previous_shake_anim = 0.0;
        self.current_shake_anim = 0.0;
    }

    pub(super) fn cancel_shake(&mut self) {
        self.is_shaking = false;
        self.previous_shake_anim = 0.0;
        self.current_shake_anim = 0.0;
    }

    pub(super) fn advance_client_tick(&mut self, in_water: bool) {
        if in_water {
            self.is_wet = true;
            return;
        }

        if (self.is_wet || self.is_shaking) && self.is_shaking {
            self.previous_shake_anim = self.current_shake_anim;
            self.current_shake_anim += WOLF_SHAKE_ANIM_PER_TICK;
            if self.previous_shake_anim >= WOLF_SHAKE_ANIM_DONE {
                self.is_wet = false;
                self.is_shaking = false;
                self.previous_shake_anim = 0.0;
                self.current_shake_anim = 0.0;
            }
        }
    }

    pub(super) fn wet_shade(self, partial_tick: f32) -> f32 {
        if !self.is_wet {
            return 1.0;
        }
        let shake_anim = self.previous_shake_anim
            + partial_tick * (self.current_shake_anim - self.previous_shake_anim);
        (WOLF_WET_SHADE_BASE + shake_anim * WOLF_WET_SHADE_SHAKE_SCALE).min(1.0)
    }

    pub(super) fn shake_anim(self, partial_tick: f32) -> f32 {
        self.previous_shake_anim
            + partial_tick * (self.current_shake_anim - self.previous_shake_anim)
    }

    pub(super) fn is_settled(self) -> bool {
        !self.is_wet
            && !self.is_shaking
            && self.previous_shake_anim == 0.0
            && self.current_shake_anim == 0.0
    }
}

/// Canonical client-side wolf begging/head-roll accumulator, mirroring vanilla
/// `Wolf.interestedAngleO` / `interestedAngle`. The synced `DATA_INTERESTED_ID` boolean
/// selects the target; each client tick eases toward `1` or `0` by `0.4`, and
/// `Wolf.getHeadRollAngle(partialTick)` scales the lerped angle by `0.15π`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WolfHeadRollAnimationState {
    pub interested: bool,
    pub previous_interested_angle: f32,
    pub current_interested_angle: f32,
}

impl WolfHeadRollAnimationState {
    pub(super) fn advance_client_tick(&mut self, interested: bool) {
        self.interested = interested;
        self.previous_interested_angle = self.current_interested_angle;
        let target = if interested { 1.0 } else { 0.0 };
        self.current_interested_angle +=
            (target - self.current_interested_angle) * WOLF_INTERESTED_EASE;
        if self.is_settled() {
            self.previous_interested_angle = 0.0;
            self.current_interested_angle = 0.0;
        }
    }

    pub(super) fn head_roll_angle(self, partial_tick: f32) -> f32 {
        lerp_f32(
            partial_tick,
            self.previous_interested_angle,
            self.current_interested_angle,
        ) * WOLF_HEAD_ROLL_SCALE
            * std::f32::consts::PI
    }

    pub(super) fn is_settled(self) -> bool {
        !self.interested
            && self.previous_interested_angle.abs() <= f32::EPSILON
            && self.current_interested_angle.abs() <= f32::EPSILON
    }
}

/// Canonical client-side cat lie-down / relax accumulators, mirroring vanilla `Cat.lieDownAmount`,
/// `lieDownAmountTail`, and `relaxStateOneAmount` plus their `*O` previous-frame fields. The synced
/// `Cat.IS_LYING` and `Cat.RELAX_STATE_ONE` booleans select the targets; each client tick saves the
/// previous values, then rises or falls by the vanilla fixed per-tick step.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CatLieDownAnimationState {
    pub is_lying: bool,
    pub relax_state_one: bool,
    pub previous_lie_down_amount: f32,
    pub current_lie_down_amount: f32,
    pub previous_lie_down_amount_tail: f32,
    pub current_lie_down_amount_tail: f32,
    pub previous_relax_state_one_amount: f32,
    pub current_relax_state_one_amount: f32,
}

impl CatLieDownAnimationState {
    pub(super) fn set_flags(&mut self, is_lying: bool, relax_state_one: bool) {
        self.is_lying = is_lying;
        self.relax_state_one = relax_state_one;
    }

    pub(super) fn advance_client_tick(&mut self, is_lying: bool, relax_state_one: bool) {
        self.set_flags(is_lying, relax_state_one);
        self.previous_lie_down_amount = self.current_lie_down_amount;
        self.previous_lie_down_amount_tail = self.current_lie_down_amount_tail;
        self.previous_relax_state_one_amount = self.current_relax_state_one_amount;

        if is_lying {
            self.current_lie_down_amount =
                (self.current_lie_down_amount + CAT_LIE_DOWN_AMOUNT_RISE_PER_TICK).min(1.0);
            self.current_lie_down_amount_tail = (self.current_lie_down_amount_tail
                + CAT_LIE_DOWN_TAIL_AMOUNT_RISE_PER_TICK)
                .min(1.0);
        } else {
            self.current_lie_down_amount =
                (self.current_lie_down_amount - CAT_LIE_DOWN_AMOUNT_FALL_PER_TICK).max(0.0);
            self.current_lie_down_amount_tail = (self.current_lie_down_amount_tail
                - CAT_LIE_DOWN_TAIL_AMOUNT_FALL_PER_TICK)
                .max(0.0);
        }

        if relax_state_one {
            self.current_relax_state_one_amount = (self.current_relax_state_one_amount
                + CAT_RELAX_STATE_ONE_AMOUNT_RISE_PER_TICK)
                .min(1.0);
        } else {
            self.current_relax_state_one_amount = (self.current_relax_state_one_amount
                - CAT_RELAX_STATE_ONE_AMOUNT_FALL_PER_TICK)
                .max(0.0);
        }
    }

    pub(super) fn lie_down_amount(self, partial_tick: f32) -> f32 {
        lerp_f32(
            partial_tick,
            self.previous_lie_down_amount,
            self.current_lie_down_amount,
        )
    }

    pub(super) fn lie_down_amount_tail(self, partial_tick: f32) -> f32 {
        lerp_f32(
            partial_tick,
            self.previous_lie_down_amount_tail,
            self.current_lie_down_amount_tail,
        )
    }

    pub(super) fn relax_state_one_amount(self, partial_tick: f32) -> f32 {
        lerp_f32(
            partial_tick,
            self.previous_relax_state_one_amount,
            self.current_relax_state_one_amount,
        )
    }

    pub(super) fn is_settled(self) -> bool {
        !self.is_lying
            && !self.relax_state_one
            && self.previous_lie_down_amount <= f32::EPSILON
            && self.current_lie_down_amount <= f32::EPSILON
            && self.previous_lie_down_amount_tail <= f32::EPSILON
            && self.current_lie_down_amount_tail <= f32::EPSILON
            && self.previous_relax_state_one_amount <= f32::EPSILON
            && self.current_relax_state_one_amount <= f32::EPSILON
    }
}

/// Canonical client-side sheep eat-grass animation countdown, mirroring vanilla
/// `Sheep.eatAnimationTick`. Entity event `10` resets it to
/// [`SHEEP_EAT_ANIMATION_TICKS`]; each client tick decrements it toward `0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheepEatAnimationState {
    pub eat_animation_tick: i32,
}

/// Canonical client-side goat head-lowering ram, mirroring vanilla `Goat.lowerHeadTick` /
/// `isLoweringHead`. Entity event `58` sets `lowering_head`, event `59` clears it; each client
/// `aiStep` advances `lower_head_tick` (`++` while lowering, `-= 2` otherwise), clamped to
/// `[0, GOAT_LOWER_HEAD_MAX_TICKS]`. `Goat.getRammingXHeadRot` normalises it by the cap and scales
/// by the adult/baby max head pitch, driving the renderer `GoatModel.setupAnim` head tilt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoatRammingAnimationState {
    pub lowering_head: bool,
    pub lower_head_tick: i32,
}

impl GoatRammingAnimationState {
    /// Vanilla `Goat.aiStep`: `lowerHeadTick++` while lowering, else `lowerHeadTick -= 2`, clamped.
    pub(super) fn advance_client_tick(&mut self) {
        if self.lowering_head {
            self.lower_head_tick += 1;
        } else {
            self.lower_head_tick -= 2;
        }
        self.lower_head_tick = self.lower_head_tick.clamp(0, GOAT_LOWER_HEAD_MAX_TICKS);
    }

    /// The ram has fully relaxed: not lowering and the counter has decayed back to `0`, so the state
    /// can be dropped (a resting goat projects no head tilt).
    pub(super) fn is_settled(&self) -> bool {
        !self.lowering_head && self.lower_head_tick == 0
    }
}

/// Canonical client-side iron golem attack / offer-flower timers, mirroring vanilla
/// `IronGolem.attackAnimationTick` / `offerFlowerTick`. Entity event `4` sets the attack timer to
/// [`IRON_GOLEM_ATTACK_TICKS`] (the two-fisted smash), event `11` sets the offer timer to
/// [`IRON_GOLEM_OFFER_FLOWER_TICKS`] (holding out a poppy) and event `34` clears it; each client
/// `aiStep` decrements both toward `0`. `IronGolemModel.setupAnim` raises the arms for whichever is
/// active (attack taking priority), else the limbs fall back to the walk swing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IronGolemAnimationState {
    pub attack_animation_tick: i32,
    pub offer_flower_tick: i32,
}

impl IronGolemAnimationState {
    /// Vanilla `IronGolem.aiStep`: `if attackAnimationTick > 0 { attackAnimationTick-- }` and the same
    /// for `offerFlowerTick`.
    pub(super) fn advance_client_tick(&mut self) {
        if self.attack_animation_tick > 0 {
            self.attack_animation_tick -= 1;
        }
        if self.offer_flower_tick > 0 {
            self.offer_flower_tick -= 1;
        }
    }

    /// Both timers have run out, so the golem is back to its walk/idle and the state can be dropped.
    pub(super) fn is_settled(&self) -> bool {
        self.attack_animation_tick == 0 && self.offer_flower_tick == 0
    }

    /// Vanilla `IronGolemRenderer.extractRenderState`: `attackTicksRemaining = getAttackAnimationTick()
    /// > 0 ? tick - partialTicks : 0`. The partial-lerped attack timer drives the smash arm wave.
    pub(super) fn attack_ticks_remaining(self, partial_tick: f32) -> f32 {
        if self.attack_animation_tick > 0 {
            self.attack_animation_tick as f32 - partial_tick
        } else {
            0.0
        }
    }
}

/// Canonical client-side ravager attack / stun / roar timers, mirroring vanilla `Ravager.attackTick` /
/// `stunnedTick` / `roarTick`. Entity event `4` sets the attack timer to [`RAVAGER_ATTACK_TICKS`] (the
/// bite), event `39` sets the stun timer to [`RAVAGER_STUN_TICKS`]; each client `aiStep` decrements all
/// three, and when the stun timer reaches `0` it arms the roar timer ([`RAVAGER_ROAR_TICKS`]) — so a
/// roar always follows a stun. `RavagerModel.setupAnim` drives the neck lunge, head shake, and mouth
/// open from the three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RavagerAnimationState {
    pub attack_tick: i32,
    pub stunned_tick: i32,
    pub roar_tick: i32,
    #[serde(default)]
    pub(super) client_particle_random_seed: u64,
}

impl RavagerAnimationState {
    pub(super) fn new(entity_id: i32) -> Self {
        Self {
            attack_tick: 0,
            stunned_tick: 0,
            roar_tick: 0,
            client_particle_random_seed: java_random_seed(
                i64::from(entity_id).wrapping_add(RAVAGER_STUN_PARTICLE_RANDOM_SALT),
            ),
        }
    }

    /// Vanilla `Ravager.aiStep`: decrement the roar, attack, and stun timers; when the stun timer hits
    /// `0` it sets `roarTick = 20` (the post-stun roar).
    pub(super) fn advance_client_tick(
        &mut self,
        entity_id: i32,
    ) -> Option<RavagerStunParticleOffset> {
        if self.client_particle_random_seed == 0 {
            self.client_particle_random_seed = java_random_seed(
                i64::from(entity_id).wrapping_add(RAVAGER_STUN_PARTICLE_RANDOM_SALT),
            );
        }
        if self.roar_tick > 0 {
            self.roar_tick -= 1;
        }
        if self.attack_tick > 0 {
            self.attack_tick -= 1;
        }
        let mut particle = None;
        if self.stunned_tick > 0 {
            self.stunned_tick -= 1;
            if java_random_next_int_bound(&mut self.client_particle_random_seed, 6) == 0 {
                particle = Some(RavagerStunParticleOffset {
                    jitter_x: java_random_next_double(&mut self.client_particle_random_seed) * 0.6
                        - 0.3,
                    jitter_z: java_random_next_double(&mut self.client_particle_random_seed) * 0.6
                        - 0.3,
                });
            }
            if self.stunned_tick == 0 {
                self.roar_tick = RAVAGER_ROAR_TICKS;
            }
        }
        particle
    }

    /// All three timers have run out, so the ravager is back to its idle/walk and the state can be
    /// dropped.
    pub(super) fn is_settled(&self) -> bool {
        self.attack_tick == 0 && self.stunned_tick == 0 && self.roar_tick == 0
    }

    /// Vanilla `RavagerRenderer.extractRenderState`: `stunnedTicksRemaining = getStunnedTick() > 0 ?
    /// tick - partialTicks : 0`.
    pub(super) fn stunned_ticks_remaining(self, partial_tick: f32) -> f32 {
        if self.stunned_tick > 0 {
            self.stunned_tick as f32 - partial_tick
        } else {
            0.0
        }
    }

    /// Vanilla `RavagerRenderer.extractRenderState`: `attackTicksRemaining = getAttackTick() > 0 ?
    /// tick - partialTicks : 0`.
    pub(super) fn attack_ticks_remaining(self, partial_tick: f32) -> f32 {
        if self.attack_tick > 0 {
            self.attack_tick as f32 - partial_tick
        } else {
            0.0
        }
    }

    /// Vanilla `RavagerRenderer.extractRenderState`: `roarAnimation = roarTick > 0 ? (20 - roarTick +
    /// partialTicks) / 20 : 0` — a `0..1` ramp as the roar timer decays from `20` to `0`.
    pub(super) fn roar_animation(self, partial_tick: f32) -> f32 {
        if self.roar_tick > 0 {
            (RAVAGER_ROAR_TICKS as f32 - self.roar_tick as f32 + partial_tick)
                / RAVAGER_ROAR_TICKS as f32
        } else {
            0.0
        }
    }
}

/// Canonical client-side hoglin / zoglin headbutt timer, mirroring vanilla
/// `Hoglin.attackAnimationRemainingTicks`. Entity event `4` sets it to [`HOGLIN_ATTACK_TICKS`]; each
/// client tick decrements it toward `0`. `HoglinModel.setupAnim`'s `animateHeadbutt` raises the head
/// from its rest down-tilt as the timer ramps through its midpoint. Projected as the RAW int (vanilla
/// `AbstractHoglinRenderer.extractRenderState` does not partial-lerp it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoglinAnimationState {
    pub attack_animation_tick: i32,
}

impl HoglinAnimationState {
    /// Vanilla `Hoglin.aiStep`: `if attackAnimationRemainingTicks > 0 { attackAnimationRemainingTicks-- }`.
    pub(super) fn advance_client_tick(&mut self) {
        if self.attack_animation_tick > 0 {
            self.attack_animation_tick -= 1;
        }
    }

    /// The headbutt timer has run out, so the state can be dropped (a resting hoglin holds its head at
    /// the baked rest tilt).
    pub(super) fn is_settled(&self) -> bool {
        self.attack_animation_tick == 0
    }
}

/// Canonical client-side rabbit hop timer, mirroring vanilla `Rabbit`'s `jumpTicks`/`jumpDuration`
/// counter and its `hopAnimationState`. Entity event `1` seeds `jumpDuration = 15; jumpTicks = 0`;
/// each client tick `Rabbit.setupAnimationStates` `startIfStopped`/`stop`s the hop on `jumpTicks > 0`
/// and `Rabbit.aiStep` advances `jumpTicks` toward `jumpDuration`, wrapping it back to `0` (and
/// clearing `jumpDuration`) when they meet — so the hop runs for exactly one 15-tick window per jump,
/// matching the 0.75s looping `RabbitAnimation.HOP`. The random idle-head-tilt animation
/// (`shouldPlayIdleAnimation` gated on a `random.nextInt(40) + 180` timeout) is NOT reconstructable
/// from the wire and is intentionally not synthesized, so the hop branch always governs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RabbitHopAnimationState {
    /// Vanilla `Rabbit.jumpTicks`.
    pub jump_ticks: i32,
    /// Vanilla `Rabbit.jumpDuration`.
    pub jump_duration: i32,
    /// Vanilla `Rabbit.hopAnimationState` (the 0.75s looping `WARDEN`-style `HOP` keyframe).
    pub hop: KeyframeAnimationState,
}

impl RabbitHopAnimationState {
    /// Vanilla `Rabbit.handleEntityEvent(1)`: `jumpDuration = 15; jumpTicks = 0`. The hop itself is
    /// started by the next tick's `setupAnimationStates` once `jumpTicks` climbs past `0`.
    pub(super) fn start_jump(&mut self) {
        self.jump_duration = RABBIT_JUMP_DURATION;
        self.jump_ticks = 0;
    }

    /// One client tick in vanilla order: `Rabbit.baseTick` runs `setupAnimationStates` (the hop
    /// branch — the client-local random idle is intentionally absent — `startIfStopped`/`stop` on
    /// `jumpTicks > 0`) BEFORE `Rabbit.aiStep`
    /// advances `jumpTicks` (`++` until it meets `jumpDuration`, then both reset to `0`).
    pub(super) fn advance_client_tick(&mut self, age_ticks: u32) {
        self.hop.animate_when(self.jump_ticks > 0, age_ticks);
        if self.jump_ticks != self.jump_duration {
            self.jump_ticks += 1;
        } else if self.jump_duration != 0 {
            self.jump_ticks = 0;
            self.jump_duration = 0;
        }
    }

    /// The jump window has fully wound down and the hop has stopped, so the state can be dropped (a
    /// resting rabbit holds the bind pose plus its look).
    pub(super) fn is_settled(&self) -> bool {
        self.jump_duration == 0 && self.jump_ticks == 0 && self.hop.start_age.is_none()
    }
}

/// Canonical client-side creaking combat/death state, mirroring vanilla `Creaking`'s three
/// `AnimationState`s, their two event-seeded counters, and the tearing-down eye flicker owned by
/// `checkEyeBlink`. Entity event `4` seeds `attackAnimationRemainingTicks = 15`, event `66` seeds
/// `invulnerabilityAnimationRemainingTicks = 8`; `aiStep` decrements both before
/// `setupAnimationStates`. The death one-shot follows synced `isTearingDown()` directly. While that
/// flag holds, the client-local `deathTime`, `nextFlickerTime`, and `eyesGlowing` fields reproduce
/// the exact toggle order and inclusive random-delay ranges consumed by `CreakingRenderer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CreakingAnimationState {
    /// Vanilla `Creaking.attackAnimationRemainingTicks`.
    pub attack_ticks: i32,
    /// Vanilla `Creaking.invulnerabilityAnimationRemainingTicks`.
    pub invuln_ticks: i32,
    /// Vanilla `Creaking.attackAnimationState` (the 0.7083s looping `CREAKING_ATTACK` lunge).
    pub attack: KeyframeAnimationState,
    /// Vanilla `Creaking.invulnerabilityAnimationState` (the 0.2917s `CREAKING_INVULNERABLE` stagger).
    pub invuln: KeyframeAnimationState,
    /// Vanilla `Creaking.deathAnimationState` (the 2.25s `CREAKING_DEATH` collapse).
    pub death: KeyframeAnimationState,
    /// Vanilla `LivingEntity.deathTime` as advanced by `Creaking.tickDeath` during the heart-bound
    /// tearing-down path. Kept separately because the shared death-tip timer is capped at 20 ticks,
    /// while Creaking flicker continues through its 45-tick teardown.
    #[serde(default)]
    pub(super) death_time: i32,
    /// Vanilla client-local `Creaking.nextFlickerTime`.
    #[serde(default)]
    pub(super) next_flicker_time: i32,
    /// Vanilla client-local `Creaking.eyesGlowing`, consumed only while tearing down.
    #[serde(default)]
    pub(super) eyes_glowing: bool,
    /// Deterministic local substitute for the process-random Vanilla entity RNG seed. `None` keeps
    /// older serialized snapshots compatible and is initialized from the entity id on first tick.
    #[serde(default)]
    pub(super) eye_flicker_random_seed: Option<u64>,
}

impl CreakingAnimationState {
    pub(super) fn new(entity_id: i32) -> Self {
        Self {
            eye_flicker_random_seed: Some(java_random_seed(
                i64::from(entity_id).wrapping_add(CREAKING_EYE_FLICKER_RANDOM_SALT),
            )),
            ..Self::default()
        }
    }

    /// Vanilla `Creaking.handleEntityEvent(4)`: `attackAnimationRemainingTicks = 15`. The one-shot
    /// itself starts on the next tick's `setupAnimationStates` once the (post-decrement) counter is
    /// still positive.
    pub(super) fn start_attack(&mut self) {
        self.attack_ticks = CREAKING_ATTACK_DURATION;
    }

    /// Vanilla `Creaking.handleEntityEvent(66)`: `invulnerabilityAnimationRemainingTicks = 8`.
    pub(super) fn start_invulnerable(&mut self) {
        self.invuln_ticks = CREAKING_INVULNERABLE_DURATION;
    }

    /// One client tick in vanilla order: `Creaking.aiStep` decrements both remaining-tick counters
    /// (toward `0`) FIRST, then `Creaking.tick`'s `setupAnimationStates` `animateWhen`s the attack on
    /// `attackTicks > 0`, the invulnerable on `invulnTicks > 0`, and the death on the synced
    /// `isTearingDown()`.
    pub(super) fn advance_client_tick(
        &mut self,
        entity_id: i32,
        age_ticks: u32,
        is_tearing_down: bool,
    ) {
        if self.attack_ticks > 0 {
            self.attack_ticks -= 1;
        }
        if self.invuln_ticks > 0 {
            self.invuln_ticks -= 1;
        }
        self.attack.animate_when(self.attack_ticks > 0, age_ticks);
        self.invuln.animate_when(self.invuln_ticks > 0, age_ticks);
        self.death.animate_when(is_tearing_down, age_ticks);

        if is_tearing_down {
            // Vanilla `Creaking.tick`: `super.tick()` runs the specialized `tickDeath` first, then
            // `setupAnimationStates`, then `checkEyeBlink`. Thus the first teardown tick observes
            // `deathTime == 1`, schedules from the false-eye `0..=0` interval, and toggles on.
            self.death_time = self.death_time.saturating_add(1);
            if self.death_time > self.next_flicker_time {
                let min_delay = if self.eyes_glowing {
                    2
                } else {
                    self.death_time / 4
                };
                let max_delay = if self.eyes_glowing {
                    8
                } else {
                    self.death_time / 2
                };
                let random_seed = self.eye_flicker_random_seed.get_or_insert_with(|| {
                    java_random_seed(
                        i64::from(entity_id).wrapping_add(CREAKING_EYE_FLICKER_RANDOM_SALT),
                    )
                });
                let delay =
                    min_delay + java_random_next_int_bound(random_seed, max_delay - min_delay + 1);
                self.next_flicker_time = self.death_time.saturating_add(delay);
                self.eyes_glowing = !self.eyes_glowing;
            }
        } else {
            // `IS_TEARING_DOWN` is monotonic for a real Vanilla teardown. Resetting the otherwise
            // invisible local state lets malformed/replayed clear→set sequences start fresh and
            // lets an event-only Creaking state settle.
            self.death_time = 0;
            self.next_flicker_time = 0;
            self.eyes_glowing = false;
        }
    }

    /// Both counters have wound down and all three one-shots have stopped, so the state can be dropped
    /// (a resting creaking holds the bind pose plus its look, gated only by the directly-projected
    /// `canMove`). A tearing-down creaking keeps its death timer alive, so it never settles while the
    /// synced `IS_TEARING_DOWN` holds.
    pub(super) fn is_settled(&self) -> bool {
        self.attack_ticks == 0
            && self.invuln_ticks == 0
            && self.attack.start_age.is_none()
            && self.invuln.start_age.is_none()
            && self.death.start_age.is_none()
    }

    pub(super) fn eyes_glowing(self) -> bool {
        self.eyes_glowing
    }
}

/// Canonical client-side warden tendril animation, mirroring vanilla
/// `Warden.tendrilAnimation`/`tendrilAnimationO`. Entity event `61` resets
/// `current` to [`WARDEN_TENDRIL_ANIMATION_TICKS`]; each client tick saves the
/// previous value and decrements `current` toward `0`. `getTendrilAnimation`
/// lerps the pair and divides by `10`, driving the renderer
/// `WardenModel.animateTendrils` antenna sway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WardenTendrilAnimationState {
    /// Vanilla `Warden.tendrilAnimationO`.
    pub previous: i32,
    /// Vanilla `Warden.tendrilAnimation`.
    pub current: i32,
}

/// Canonical client-side warden heart animation, mirroring vanilla
/// `Warden.heartAnimation`/`heartAnimationO`. Unlike the event-driven tendril, the
/// heart is a free-running heartbeat: `Warden.tick` resets `current` to
/// [`WARDEN_HEART_ANIMATION_TICKS`] whenever `tickCount % getHeartBeatDelay() == 0`
/// (the delay shrinking from `40` toward `10` as the synced anger rises), then each
/// tick saves the previous value and decrements `current` toward `0`.
/// `getHeartAnimation` lerps the pair and divides by `10`, driving the renderer
/// warden heart emissive overlay's alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WardenHeartAnimationState {
    /// Vanilla `Warden.heartAnimationO`.
    pub previous: i32,
    /// Vanilla `Warden.heartAnimation`.
    pub current: i32,
}

/// Canonical client-side warden combat/threat keyframe animations, mirroring vanilla
/// `Warden`'s `roarAnimationState`/`sniffAnimationState`/`attackAnimationState`/
/// `sonicBoomAnimationState`. These are the four triggered one-shots the client drives:
///
/// - **roar** / **sniff** are pose-driven: `Warden.onSyncedDataUpdated(DATA_POSE)` `.start()`s
///   the matching timer when the synced `DATA_POSE` CHANGES to `Pose.ROARING`/`Pose.SNIFFING`.
///   We track [`Self::prev_pose`] and restart a timer only on the transition into its pose;
///   vanilla never auto-stops these on a pose leave, so the non-looping keyframe just holds its
///   final/neutral frame (the renderer clamp reproduces that).
/// - **attack** / **sonic_boom** are event-driven: `Warden.handleEntityEvent(4)` stops the roar
///   and starts the attack, and `handleEntityEvent(62)` starts the sonic boom.
///
/// Vanilla applies ALL SIX additively in `WardenModel.setupAnim`, so each timer is projected
/// independently; the renderer applies every active one in the vanilla order (attack, sonic_boom,
/// dig, emerge, roar, sniff). The `EMERGING`/`DIGGING` poses drive the spawn/despawn one-shots the
/// same way the roar/sniff poses do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WardenCombatAnimationState {
    /// The last synced `DATA_POSE` ordinal observed, so a pose CHANGE into
    /// ROARING/SNIFFING/EMERGING/DIGGING is a rising edge that restarts the matching timer (vanilla
    /// `.start(tickCount)` on the transition). [`WARDEN_POSE_UNSET`] until the first pose arrives.
    pub prev_pose: i32,
    /// Vanilla `Warden.roarAnimationState` (the 4.2s `WARDEN_ROAR`), started when the pose changes
    /// to `Pose.ROARING` and stopped by the attack event.
    pub roar: KeyframeAnimationState,
    /// Vanilla `Warden.sniffAnimationState` (the 4.16s `WARDEN_SNIFF`), started when the pose
    /// changes to `Pose.SNIFFING`.
    pub sniff: KeyframeAnimationState,
    /// Vanilla `Warden.attackAnimationState` (the 0.33333s `WARDEN_ATTACK`), started by event `4`.
    pub attack: KeyframeAnimationState,
    /// Vanilla `Warden.sonicBoomAnimationState` (the 3.0s `WARDEN_SONIC_BOOM`), started by event `62`.
    pub sonic_boom: KeyframeAnimationState,
    /// Vanilla `Warden.emergeAnimationState` (the 6.68s `WARDEN_EMERGE` spawn rise), started when
    /// the pose changes to `Pose.EMERGING`.
    pub emerge: KeyframeAnimationState,
    /// Vanilla `Warden.diggingAnimationState` (the 5.0s `WARDEN_DIG` despawn burrow), started when
    /// the pose changes to `Pose.DIGGING`.
    pub dig: KeyframeAnimationState,
}

impl Default for WardenCombatAnimationState {
    fn default() -> Self {
        Self {
            prev_pose: WARDEN_POSE_UNSET,
            roar: KeyframeAnimationState { start_age: None },
            sniff: KeyframeAnimationState { start_age: None },
            attack: KeyframeAnimationState { start_age: None },
            sonic_boom: KeyframeAnimationState { start_age: None },
            emerge: KeyframeAnimationState { start_age: None },
            dig: KeyframeAnimationState { start_age: None },
        }
    }
}

impl WardenCombatAnimationState {
    /// Vanilla `Warden.onSyncedDataUpdated(DATA_POSE)`: the pose-change `switch` that `.start()`s the
    /// matching timer when the pose CHANGES to `Pose.ROARING`/`Pose.SNIFFING`/`Pose.EMERGING`/
    /// `Pose.DIGGING`. A redundant re-set to the same pose is not a transition, so it leaves a
    /// running timer alone (vanilla only fires on a real `onSyncedDataUpdated` change).
    pub(super) fn set_pose(&mut self, pose_id: i32, age_ticks: u32) {
        if pose_id == self.prev_pose {
            return;
        }
        self.prev_pose = pose_id;
        match pose_id {
            VANILLA_ENTITY_POSE_ROARING_ID => self.roar.start_age = Some(age_ticks),
            VANILLA_ENTITY_POSE_SNIFFING_ID => self.sniff.start_age = Some(age_ticks),
            VANILLA_ENTITY_POSE_EMERGING_ID => self.emerge.start_age = Some(age_ticks),
            VANILLA_ENTITY_POSE_DIGGING_ID => self.dig.start_age = Some(age_ticks),
            _ => {}
        }
    }

    /// Vanilla `Warden.handleEntityEvent(4)`: `roarAnimationState.stop()` then
    /// `attackAnimationState.start(tickCount)`.
    pub(super) fn start_attack(&mut self, age_ticks: u32) {
        self.roar.start_age = None;
        self.attack.start_age = Some(age_ticks);
    }

    /// Vanilla `Warden.handleEntityEvent(62)`: `sonicBoomAnimationState.start(tickCount)`.
    pub(super) fn start_sonic_boom(&mut self, age_ticks: u32) {
        self.sonic_boom.start_age = Some(age_ticks);
    }
}

/// Canonical client-side squid tentacle/body animation, mirroring vanilla
/// `Squid` (`xBodyRot`/`zBodyRot`/`tentacleMovement`/`tentacleAngle` and their
/// `O`/`old` lerp endpoints). `Squid.aiStep` advances `tentacleMovement` by the
/// id-seeded `tentacleSpeed` each client tick and derives the tentacle flex angle
/// and body pitch/roll; `SquidRenderer.extractRenderState` lerps the pairs.
///
/// Both `isInWater()` branches are modelled. The movement-derived `yBodyRot`
/// refinement is kept as render-state projection so bbb does not mutate the
/// canonical synced transform.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SquidAnimationState {
    /// Vanilla `Squid.tentacleSpeed`, seeded once from the entity id in the
    /// constructor: `random.setSeed(getId()); 1 / (random.nextFloat() + 1) * 0.2`.
    #[serde(default)]
    pub tentacle_speed: f32,
    /// Vanilla `Squid.tentacleMovement`.
    #[serde(default)]
    pub tentacle_movement: f32,
    /// Vanilla `Squid.oldTentacleMovement`.
    #[serde(default)]
    pub old_tentacle_movement: f32,
    /// Vanilla `Squid.tentacleAngle`.
    #[serde(default)]
    pub tentacle_angle: f32,
    /// Vanilla `Squid.oldTentacleAngle`.
    #[serde(default)]
    pub old_tentacle_angle: f32,
    /// Vanilla `Squid.xBodyRot` (swim pitch, degrees).
    #[serde(default)]
    pub x_body_rot: f32,
    /// Vanilla `Squid.xBodyRotO`.
    #[serde(default)]
    pub old_x_body_rot: f32,
    /// Vanilla `Squid.yBodyRot` (movement-derived body yaw, degrees).
    #[serde(default)]
    pub y_body_rot: f32,
    /// Vanilla `Squid.yBodyRotO`.
    #[serde(default)]
    pub old_y_body_rot: f32,
    /// Vanilla `Squid.zBodyRot` (swim roll, degrees).
    #[serde(default)]
    pub z_body_rot: f32,
    /// Vanilla `Squid.zBodyRotO`.
    #[serde(default)]
    pub old_z_body_rot: f32,
    /// Vanilla `Squid.rotateSpeed`.
    #[serde(default)]
    pub rotate_speed: f32,
}

/// Canonical client-side chicken wing-flap animation, mirroring vanilla
/// `Chicken` (`flap`/`oFlap`/`flapSpeed`/`oFlapSpeed`/`flapping`). `Chicken.aiStep`
/// drives `flapSpeed` toward `1` while airborne (toward `0` on the ground), keeps
/// `flapping` decaying at `0.9` per tick (re-seeded to `1` whenever the chicken
/// leaves the ground), and integrates `flap += flapping * 2`.
/// `ChickenRenderer.extractRenderState` lerps `flap`/`flapSpeed` by the partial
/// tick, and `ChickenModel.setupAnim` turns them into the wing `zRot`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChickenFlapAnimationState {
    /// Vanilla `Chicken.flap`.
    #[serde(default)]
    pub flap: f32,
    /// Vanilla `Chicken.oFlap`.
    #[serde(default)]
    pub o_flap: f32,
    /// Vanilla `Chicken.flapSpeed`.
    #[serde(default)]
    pub flap_speed: f32,
    /// Vanilla `Chicken.oFlapSpeed`.
    #[serde(default)]
    pub o_flap_speed: f32,
    /// Vanilla `Chicken.flapping` (field initializer `1.0`).
    #[serde(default)]
    pub flapping: f32,
}

/// Canonical client-side slime/magma-cube squish accumulator, mirroring vanilla
/// `Slime` (`squish`/`oSquish`/`targetSquish`/`wasOnGround`). `Slime.tick` saves the
/// lerp endpoint (`oSquish`), eases `squish` halfway toward `targetSquish` each
/// client tick, emits the landing audiovisual branch before re-seeding the target
/// to `-0.5` on landing / `1.0` on takeoff (the `onGround()` transitions), then
/// decays the target by `0.6` (`Slime`) or `0.9` (`MagmaCube`).
/// `SlimeRenderer.extractRenderState` lerps `squish` by the partial tick, and
/// `SlimeRenderer.scale` turns it into the non-uniform body stretch.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct SlimeAnimationState {
    /// Vanilla `Slime.squish`.
    #[serde(default)]
    pub squish: f32,
    /// Vanilla `Slime.oSquish`.
    #[serde(default)]
    pub o_squish: f32,
    /// Vanilla `Slime.targetSquish`.
    #[serde(default)]
    pub target_squish: f32,
    /// Vanilla `Slime.wasOnGround` (field initializer `false`).
    #[serde(default)]
    pub was_on_ground: bool,
}

/// Canonical client-side allay dance/spin accumulators, mirroring vanilla `Allay`
/// (`dancingAnimationTicks` / `spinningAnimationTicks` / `spinningAnimationTicks0`). While the synced
/// `DATA_DANCING` boolean is set, `Allay.tick` increments the dance counter, drives the spin counter
/// up while `isSpinning()` (the first 15 ticks of each 55-tick dance cycle) and down otherwise
/// (clamped `0..=15`), and saves the previous spin counter for the partial-tick lerp. `AllayModel`
/// turns them into the body spin (`root.yRot = 4π·spinningProgress`) and the head/body sway. The
/// counters reset to `0` the moment dancing stops.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct AllayDanceAnimationState {
    /// Vanilla `Allay.dancingAnimationTicks`.
    #[serde(default)]
    pub dancing_animation_ticks: f32,
    /// Vanilla `Allay.spinningAnimationTicks`.
    #[serde(default)]
    pub spinning_animation_ticks: f32,
    /// Vanilla `Allay.spinningAnimationTicks0` (the previous-tick spin counter, the lerp endpoint).
    #[serde(default)]
    pub spinning_animation_ticks0: f32,
}

impl AllayDanceAnimationState {
    /// Advances one client tick of `Allay.tick`'s dance/spin accumulators.
    pub(super) fn advance_client_tick(&mut self, is_dancing: bool) {
        if is_dancing {
            self.dancing_animation_ticks += 1.0;
            self.spinning_animation_ticks0 = self.spinning_animation_ticks;
            if self.is_spinning() {
                self.spinning_animation_ticks += 1.0;
            } else {
                self.spinning_animation_ticks -= 1.0;
            }
            self.spinning_animation_ticks = self.spinning_animation_ticks.clamp(0.0, 15.0);
        } else {
            self.dancing_animation_ticks = 0.0;
            self.spinning_animation_ticks = 0.0;
            self.spinning_animation_ticks0 = 0.0;
        }
    }

    /// Vanilla `Allay.isDancing()` reconstructed from the accumulator: the dance counter only advances
    /// while the synced `DATA_DANCING` flag is set, so a non-zero counter means the allay is dancing.
    pub(super) fn is_dancing(&self) -> bool {
        self.dancing_animation_ticks > 0.0
    }

    /// Vanilla `Allay.isSpinning()`: `(dancingAnimationTicks % 55) < 15` — the allay spins for the
    /// first 15 ticks of each 55-tick dance cycle.
    pub(super) fn is_spinning(&self) -> bool {
        self.dancing_animation_ticks % 55.0 < 15.0
    }

    /// Vanilla `Allay.getSpinningProgress(partialTick)`: `lerp(partialTick, spinningTicks0,
    /// spinningTicks) / 15`, the `0..1` spin ramp.
    pub(super) fn spinning_progress(&self, partial_tick: f32) -> f32 {
        (self.spinning_animation_ticks0
            + (self.spinning_animation_ticks - self.spinning_animation_ticks0) * partial_tick)
            / 15.0
    }
}

/// Canonical client-side allay held-item accumulator, mirroring vanilla `Allay`
/// (`holdingItemAnimationTicks` / `holdingItemAnimationTicks0`). The client raises
/// the counter toward `5` while `hasItemInHand()` is true and lowers it toward `0`
/// otherwise; `AllayModel.setupAnim` consumes the lerped `0..1` progress for both
/// arm rotations.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct AllayHoldingItemAnimationState {
    /// Vanilla `Allay.holdingItemAnimationTicks`.
    #[serde(default)]
    pub holding_item_animation_ticks: f32,
    /// Vanilla `Allay.holdingItemAnimationTicks0` (the previous-tick counter, the lerp endpoint).
    #[serde(default)]
    pub holding_item_animation_ticks0: f32,
}

impl AllayHoldingItemAnimationState {
    /// Advances one client tick of `Allay.tick`'s held-item accumulator.
    pub(super) fn advance_client_tick(&mut self, has_item_in_hand: bool) {
        self.holding_item_animation_ticks0 = self.holding_item_animation_ticks;
        if has_item_in_hand {
            self.holding_item_animation_ticks =
                (self.holding_item_animation_ticks + 1.0).clamp(0.0, 5.0);
        } else {
            self.holding_item_animation_ticks =
                (self.holding_item_animation_ticks - 1.0).clamp(0.0, 5.0);
        }
    }

    pub(super) fn is_settled(&self) -> bool {
        self.holding_item_animation_ticks == 0.0 && self.holding_item_animation_ticks0 == 0.0
    }

    /// Vanilla `Allay.getHoldingItemAnimationProgress(partialTick)`.
    pub(super) fn holding_item_progress(&self, partial_tick: f32) -> f32 {
        (self.holding_item_animation_ticks0
            + (self.holding_item_animation_ticks - self.holding_item_animation_ticks0)
                * partial_tick)
            / 5.0
    }
}
