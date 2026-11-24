use super::*;
use bbb_registry_codec::legacy_random::LegacyRandomSource;

/// Canonical client-side `getTicksUsingItem` counter for a crossbow draw, reconstructing the value the
/// `CROSSBOW_CHARGE` pose needs. Vanilla syncs only the binary `isChargingCrossbow()` flag (the using-item
/// flag rises with it), so the client recovers the elapsed draw ticks by counting up while the flag is set
/// (`LivingEntity` reconstructs `useItemRemaining = useDuration` when the flag rises, then decrements it
/// each tick — `getTicksUsingItem() = useDuration - useItemRemaining` is exactly this count). The counter
/// resets to `0` the moment charging stops.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct CrossbowChargeAnimationState {
    /// Ticks elapsed since the crossbow draw began (vanilla `getTicksUsingItem()`).
    #[serde(default)]
    pub ticks: f32,
}

impl CrossbowChargeAnimationState {
    /// Advances one client tick: count up while charging, reset to `0` otherwise.
    pub(super) fn advance_client_tick(&mut self, is_charging: bool) {
        self.ticks = if is_charging { self.ticks + 1.0 } else { 0.0 };
    }

    /// Whether the draw has fully stopped (counter back to `0`), so the state can be dropped.
    pub(super) fn is_settled(&self) -> bool {
        self.ticks == 0.0
    }

    /// Vanilla `LivingEntity.getTicksUsingItem(partialTicks)` = `getTicksUsingItem() + partialTicks` while
    /// using; `0` once the draw stops (the counter is `0`).
    pub(super) fn ticks_using_item(&self, partial_tick: f32) -> f32 {
        if self.ticks > 0.0 {
            self.ticks + partial_tick
        } else {
            0.0
        }
    }
}

/// Vanilla `Ease.inOutSine`: `-(cos(π·x) - 1) / 2`, the easing every axolotl `BinaryAnimator` uses.
pub(super) fn ease_in_out_sine(x: f32) -> f32 {
    -((std::f32::consts::PI * x).cos() - 1.0) / 2.0
}

/// Vanilla `net.minecraft.util.BinaryAnimator`: a tick counter that climbs toward `length` while a
/// boolean is active and falls back toward `0` while it is not, exposing an eased `0..1` factor. The
/// `animationLength`/`easing` are supplied by the caller (the axolotl uses `10` / `IN_OUT_SINE` for
/// all four of its animators), so this state stores only the live and previous tick counts.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub(super) struct BinaryAnimator {
    #[serde(default)]
    pub(super) ticks: i32,
    #[serde(default)]
    pub(super) ticks_old: i32,
}

impl BinaryAnimator {
    /// Vanilla `BinaryAnimator.tick(active)`: snapshot the old count, then climb toward `length`
    /// (active) or fall toward `0` (inactive) by one.
    pub(super) fn tick(&mut self, active: bool, length: i32) {
        self.ticks_old = self.ticks;
        if active {
            if self.ticks < length {
                self.ticks += 1;
            }
        } else if self.ticks > 0 {
            self.ticks -= 1;
        }
    }

    /// Vanilla `BinaryAnimator.getFactor(partialTicks)`: `easing(lerp(partial, ticksOld, ticks) /
    /// length)`. The axolotl's easing is `IN_OUT_SINE`.
    pub(super) fn factor(&self, partial_tick: f32, length: i32) -> f32 {
        let raw = (self.ticks_old as f32 + (self.ticks - self.ticks_old) as f32 * partial_tick)
            / length as f32;
        ease_in_out_sine(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AxolotlBabyAnimation {
    Swim,
    Walk,
    WalkUnderwater,
    IdleUnderwater,
    IdleUnderwaterOnGround,
    IdleOnGround,
    PlayDead,
}

/// Canonical client-side axolotl animation. Adults mirror `Axolotl.tickAdultAnimations`: four
/// `BinaryAnimator`s ease the factors that `AdultAxolotlModel.setupAnim` blends. Babies mirror
/// `tickBabyAnimations`: exactly one of seven `AnimationState`s remains started, preserving its
/// original start tick until the selected state changes.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct AxolotlAnimationState {
    #[serde(default)]
    pub(super) playing_dead: BinaryAnimator,
    #[serde(default)]
    pub(super) in_water: BinaryAnimator,
    #[serde(default)]
    pub(super) on_ground: BinaryAnimator,
    #[serde(default)]
    pub(super) moving: BinaryAnimator,
    #[serde(default)]
    pub(super) prev_x_rot: f32,
    #[serde(default)]
    pub(super) prev_y_rot: f32,
    #[serde(default)]
    pub(super) has_prev_rotation: bool,
    #[serde(default)]
    pub(super) baby_swim: KeyframeAnimationState,
    #[serde(default)]
    pub(super) baby_walk: KeyframeAnimationState,
    #[serde(default)]
    pub(super) baby_walk_underwater: KeyframeAnimationState,
    #[serde(default)]
    pub(super) baby_idle_underwater: KeyframeAnimationState,
    #[serde(default)]
    pub(super) baby_idle_underwater_on_ground: KeyframeAnimationState,
    #[serde(default)]
    pub(super) baby_idle_on_ground: KeyframeAnimationState,
    #[serde(default)]
    pub(super) baby_play_dead: KeyframeAnimationState,
}

impl AxolotlAnimationState {
    /// Advances one client tick of `Axolotl.tickAdultAnimations`: derive the mutually-exclusive
    /// animation state, then tick the four animators. `walk_is_moving` is the prior tick's
    /// `walkAnimation.isMoving()` (the walk animation is advanced after the per-type match, matching
    /// vanilla's `baseTick`-before-`aiStep` order); the rotation change OR mirrors `getXRot() !=
    /// xRotO || getYRot() != yRotO`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn advance_client_tick(
        &mut self,
        is_baby: bool,
        age_ticks: u32,
        is_playing_dead: bool,
        is_in_water: bool,
        on_ground: bool,
        walk_is_moving: bool,
        x_rot: f32,
        y_rot: f32,
    ) {
        let rotation_changed =
            self.has_prev_rotation && (x_rot != self.prev_x_rot || y_rot != self.prev_y_rot);
        self.prev_x_rot = x_rot;
        self.prev_y_rot = y_rot;
        self.has_prev_rotation = true;
        let is_moving = walk_is_moving || rotation_changed;

        if is_baby {
            // Vanilla `tickBabyAnimations`: `soloAnimation` starts the selected timer only if it
            // was stopped and stops all six siblings. The moving animator still advances even
            // though the baby model consumes the discrete AnimationStates instead of its factor.
            self.moving.tick(is_moving, AXOLOTL_ANIMATOR_LENGTH);
            let active = if is_playing_dead {
                AxolotlBabyAnimation::PlayDead
            } else if is_moving {
                if is_in_water && !on_ground {
                    AxolotlBabyAnimation::Swim
                } else if !is_in_water && on_ground {
                    AxolotlBabyAnimation::Walk
                } else {
                    AxolotlBabyAnimation::WalkUnderwater
                }
            } else if is_in_water && !on_ground {
                AxolotlBabyAnimation::IdleUnderwater
            } else if is_in_water && on_ground {
                AxolotlBabyAnimation::IdleUnderwaterOnGround
            } else {
                AxolotlBabyAnimation::IdleOnGround
            };
            self.baby_swim
                .animate_when(active == AxolotlBabyAnimation::Swim, age_ticks);
            self.baby_walk
                .animate_when(active == AxolotlBabyAnimation::Walk, age_ticks);
            self.baby_walk_underwater
                .animate_when(active == AxolotlBabyAnimation::WalkUnderwater, age_ticks);
            self.baby_idle_underwater
                .animate_when(active == AxolotlBabyAnimation::IdleUnderwater, age_ticks);
            self.baby_idle_underwater_on_ground.animate_when(
                active == AxolotlBabyAnimation::IdleUnderwaterOnGround,
                age_ticks,
            );
            self.baby_idle_on_ground
                .animate_when(active == AxolotlBabyAnimation::IdleOnGround, age_ticks);
            self.baby_play_dead
                .animate_when(active == AxolotlBabyAnimation::PlayDead, age_ticks);
            return;
        }

        // Vanilla `Axolotl.AxolotlAnimationState`: PLAYING_DEAD → IN_WATER → ON_GROUND → IN_AIR,
        // first match wins, so the three state animators are mutually exclusive.
        let state_playing_dead = is_playing_dead;
        let state_in_water = !is_playing_dead && is_in_water;
        let state_on_ground = !is_playing_dead && !is_in_water && on_ground;
        self.playing_dead
            .tick(state_playing_dead, AXOLOTL_ANIMATOR_LENGTH);
        self.in_water.tick(state_in_water, AXOLOTL_ANIMATOR_LENGTH);
        self.on_ground
            .tick(state_on_ground, AXOLOTL_ANIMATOR_LENGTH);
        self.moving.tick(is_moving, AXOLOTL_ANIMATOR_LENGTH);
    }

    pub(super) fn playing_dead_factor(&self, partial_tick: f32) -> f32 {
        self.playing_dead
            .factor(partial_tick, AXOLOTL_ANIMATOR_LENGTH)
    }

    pub(super) fn in_water_factor(&self, partial_tick: f32) -> f32 {
        self.in_water.factor(partial_tick, AXOLOTL_ANIMATOR_LENGTH)
    }

    pub(super) fn on_ground_factor(&self, partial_tick: f32) -> f32 {
        self.on_ground.factor(partial_tick, AXOLOTL_ANIMATOR_LENGTH)
    }

    pub(super) fn moving_factor(&self, partial_tick: f32) -> f32 {
        self.moving.factor(partial_tick, AXOLOTL_ANIMATOR_LENGTH)
    }

    pub(super) fn baby_animation_seconds(
        timer: KeyframeAnimationState,
        age_ticks: u32,
        partial_tick: f32,
    ) -> f32 {
        timer
            .elapsed_seconds(age_ticks, partial_tick)
            .unwrap_or(-1.0)
    }
}

/// Canonical client-side evoker-fangs attack animation, mirroring vanilla `EvokerFangs`
/// (`clientSideAttackStarted` / `lifeTicks`). The fang is spawned underground and hidden until the
/// server broadcasts entity event `4` (`EvokerFangs.handleEntityEvent` → `clientSideAttackStarted =
/// true`); from then on `EvokerFangs.tick` decrements `lifeTicks` (initially `22`) each client tick.
/// `getAnimationProgress` builds the `0..1` bite ramp from `lifeTicks`, which
/// `EvokerFangsModel.setupAnim` turns into the jaw snap, the rise out of the ground, and the final
/// vanish (root scale → 0). The attack sound on the same event is a `play_apply` audio side effect.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EvokerFangsAnimationState {
    /// Vanilla `EvokerFangs.clientSideAttackStarted`.
    #[serde(default)]
    pub started: bool,
    /// Vanilla `EvokerFangs.lifeTicks` (field initializer `22`).
    #[serde(default)]
    pub life_ticks: i32,
    #[serde(default)]
    pub(super) client_particle_random_seed: u64,
}

/// Canonical client-side parrot wing-flap animation, mirroring vanilla `Parrot`
/// (`flap`/`oFlap`/`flapSpeed`/`oFlapSpeed`/`flapping`). `Parrot.aiStep` runs the
/// same flap accumulator as the chicken, except the airborne build-up is gated on
/// `!onGround() && !isPassenger()` (a parrot riding a shoulder/mount holds its
/// wings), drives `flapSpeed` toward `1` while airborne (toward `0` otherwise),
/// keeps `flapping` decaying at `0.9` per tick (re-seeded to `1` whenever the
/// parrot is off the ground), and integrates `flap += flapping * 2`.
/// `ParrotRenderer.extractRenderState` lerps `flap` and `flapSpeed` separately and
/// combines them into `flapAngle = (sin(flap) + 1) * flapSpeed`, which
/// `ParrotModel.setupAnim` feeds to the wing `zRot` and the body/head/tail bob.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ParrotFlapAnimationState {
    /// Vanilla `Parrot.flap`.
    #[serde(default)]
    pub flap: f32,
    /// Vanilla `Parrot.oFlap`.
    #[serde(default)]
    pub o_flap: f32,
    /// Vanilla `Parrot.flapSpeed`.
    #[serde(default)]
    pub flap_speed: f32,
    /// Vanilla `Parrot.oFlapSpeed`.
    #[serde(default)]
    pub o_flap_speed: f32,
    /// Vanilla `Parrot.flapping` (field initializer `1.0`).
    #[serde(default)]
    pub flapping: f32,
}

/// Canonical client-side guardian tail-sway animation, mirroring vanilla
/// `Guardian` (`clientSideTailAnimation`/`clientSideTailAnimationO`/
/// `clientSideTailAnimationSpeed`). `Guardian.aiStep` advances the tail phase by
/// `clientSideTailAnimationSpeed` each client tick, ramping that speed differently
/// depending on whether the guardian is out of water (`2.0`, the frantic
/// out-of-water flop), in water and moving (toward `0.5`, a fast snap to `4.0`
/// from a near-rest speed), or in water and idle (toward `0.125`, a slow hover
/// wave). `GuardianRenderer.extractRenderState` lerps the pair into
/// `tailAnimation`, which `GuardianModel.setupAnim` feeds to the three tail
/// segments' `yRot` (`sin(swim) * π * {0.05, 0.1, 0.15}`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GuardianTailAnimationState {
    /// Vanilla `Guardian.clientSideTailAnimation` (the integrated tail phase).
    #[serde(default)]
    pub tail_animation: f32,
    /// Vanilla `Guardian.clientSideTailAnimationO` (the previous-tick phase, the
    /// lerp endpoint).
    #[serde(default)]
    pub previous_tail_animation: f32,
    /// Vanilla `Guardian.clientSideTailAnimationSpeed`.
    #[serde(default)]
    pub tail_animation_speed: f32,
    /// Vanilla `clientSideTouchedGround`, updated only by the dry branch and
    /// consumed on the following upward movement to gate the flop sound.
    #[serde(skip)]
    pub client_side_touched_ground: bool,
}

impl Default for GuardianTailAnimationState {
    fn default() -> Self {
        Self {
            tail_animation: 0.0,
            previous_tail_animation: 0.0,
            tail_animation_speed: 0.0,
            client_side_touched_ground: false,
        }
    }
}

impl GuardianTailAnimationState {
    pub(super) fn with_initial_phase(phase: f32) -> Self {
        // Vanilla's constructor assigns the same random nextFloat to both lerp
        // endpoints and leaves the speed at Java's zero field default.
        Self {
            tail_animation: phase,
            previous_tail_animation: phase,
            tail_animation_speed: 0.0,
            client_side_touched_ground: false,
        }
    }

    /// Advances one client tick of `Guardian.aiStep`'s tail-sway accumulator.
    ///
    /// Vanilla saves the lerp endpoint (`clientSideTailAnimationO`), then ramps
    /// `clientSideTailAnimationSpeed`: out of water it snaps to `2.0`; in water and
    /// moving it jumps to `4.0` from a near-rest speed (`< 0.5`) else eases toward
    /// `0.5` by `0.1`; in water and idle it eases toward `0.125` by `0.2`. Finally
    /// it integrates `clientSideTailAnimation += clientSideTailAnimationSpeed`. The
    /// Returns whether Vanilla emits the dry upward flop sound before replacing
    /// `clientSideTouchedGround` with this tick's downward/support-face result.
    pub(super) fn advance_client_tick(
        &mut self,
        in_water: bool,
        is_moving: bool,
        delta_y: f64,
        can_stand_on_below: bool,
        is_silent: bool,
    ) -> bool {
        self.previous_tail_animation = self.tail_animation;
        let flop_sound = if !in_water {
            let sound = delta_y > 0.0 && self.client_side_touched_ground && !is_silent;
            self.client_side_touched_ground = delta_y < 0.0 && can_stand_on_below;
            sound
        } else {
            false
        };
        if !in_water {
            self.tail_animation_speed = 2.0;
        } else if is_moving {
            if self.tail_animation_speed < 0.5 {
                self.tail_animation_speed = 4.0;
            } else {
                self.tail_animation_speed += (0.5 - self.tail_animation_speed) * 0.1;
            }
        } else {
            self.tail_animation_speed += (0.125 - self.tail_animation_speed) * 0.2;
        }
        self.tail_animation += self.tail_animation_speed;
        flop_sound
    }

    /// Vanilla `GuardianRenderer.extractRenderState`: `state.tailAnimation =
    /// entity.getTailAnimation(partialTicks) = Mth.lerp(partialTick,
    /// clientSideTailAnimationO, clientSideTailAnimation)`.
    pub(super) fn tail_animation(&self, partial_tick: f32) -> f32 {
        self.previous_tail_animation
            + (self.tail_animation - self.previous_tail_animation) * partial_tick
    }
}

/// Canonical client-side guardian spike-withdrawal accumulator, mirroring vanilla
/// `Guardian.clientSideSpikesAnimation`/`clientSideSpikesAnimationO` (both `0.0` at spawn). Each
/// client tick saves the previous value (the lerp endpoint), then IN WATER eases `spikes_animation`
/// toward `0` while `isMoving()` (the spikes retract as it swims) by `0.25` or toward `1` while idle
/// (the spikes fully extend) by `0.06`. Vanilla OUT OF WATER instead sets it to a fresh
/// `random.nextFloat()` every tick; that flicker is not seeded on the client, so it is intentionally
/// held at the last value rather than faked — a guardian out of water is a flopping/dying edge case.
/// `getSpikesAnimation` lerps the pair, which `GuardianModel.setupAnim` turns into
/// `withdrawal = (1 - spikesAnimation) · 0.55`, the per-spike inset subtracted from each spike offset.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct GuardianSpikesAnimationState {
    /// Vanilla `Guardian.clientSideSpikesAnimation` (the eased withdrawal phase, `0..=1`).
    #[serde(default)]
    pub spikes_animation: f32,
    /// Vanilla `Guardian.clientSideSpikesAnimationO` (the previous-tick phase, the lerp endpoint).
    #[serde(default)]
    pub previous_spikes_animation: f32,
}

impl GuardianSpikesAnimationState {
    /// Advances one client tick of `Guardian.aiStep`'s spike-withdrawal accumulator: save the lerp
    /// endpoint, then ease toward `0` (moving) / `1` (idle) in water. Out of
    /// water Vanilla uses the entity RNG value supplied by the common owner.
    pub(super) fn advance_client_tick(
        &mut self,
        in_water: bool,
        is_moving: bool,
        dry_random: Option<f32>,
    ) {
        self.previous_spikes_animation = self.spikes_animation;
        if !in_water {
            self.spikes_animation = dry_random.expect("dry Guardian tick requires one RNG float");
        } else if is_moving {
            self.spikes_animation += (0.0 - self.spikes_animation) * 0.25;
        } else {
            self.spikes_animation += (1.0 - self.spikes_animation) * 0.06;
        }
    }

    /// Vanilla `GuardianRenderer.extractRenderState`: `state.spikesAnimation =
    /// entity.getSpikesAnimation(partialTicks) = Mth.lerp(partialTick, clientSideSpikesAnimationO,
    /// clientSideSpikesAnimation)`.
    pub(super) fn spikes_animation(&self, partial_tick: f32) -> f32 {
        self.previous_spikes_animation
            + (self.spikes_animation - self.previous_spikes_animation) * partial_tick
    }
}

/// The "not in a triggered pose" sentinel for [`BreezeAnimationState::prev_pose`]: no synced
/// `DATA_POSE` has been observed yet, so the first pose is treated as a fresh transition.
pub(super) const BREEZE_POSE_UNSET: i32 = -1;

/// Canonical client-side breeze action animations, mirroring vanilla `Breeze`'s pose-driven
/// `shoot`/`slide`/`slideBack`/`inhale`/`longJump` `AnimationState`s (the looping `idle` stays
/// renderer-side, free-running off `ageInTicks`). `Breeze.onSyncedDataUpdated(DATA_POSE)` runs
/// `resetAnimations()` then `startIfStopped`s the new pose's one-shot, and `Breeze.tick` starts
/// `longJump` while `Pose.LONG_JUMPING` and, when LEAVING `Pose.SLIDING`, starts `slideBack` and stops
/// `slide`. Collapsed: each pose-gated one-shot runs exactly while its pose holds (`animate_when(pose
/// == X)`, the reset handling the stop-on-leave), and the falling edge of `Pose.SLIDING` fires the
/// brief `slideBack` return (a one-shot that ends at the neutral pose, so it holds harmlessly).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BreezeAnimationState {
    /// The last synced `DATA_POSE` ordinal observed, so the falling edge of `Pose.SLIDING` can fire
    /// `slideBack`.
    pub prev_pose: i32,
    /// Vanilla `Breeze.shoot` (the 1.125s `SHOOT`), active while `Pose.SHOOTING`.
    pub shoot: KeyframeAnimationState,
    /// Vanilla `Breeze.slide` (the 0.2s `SLIDE`), active while `Pose.SLIDING`.
    pub slide: KeyframeAnimationState,
    /// Vanilla `Breeze.slideBack` (the 0.1s `SLIDE_BACK` return), fired on leaving `Pose.SLIDING`.
    pub slide_back: KeyframeAnimationState,
    /// Vanilla `Breeze.inhale` (the 2.0s `INHALE`), active while `Pose.INHALING`.
    pub inhale: KeyframeAnimationState,
    /// Vanilla `Breeze.longJump` (the 0.5s `JUMP`), active while `Pose.LONG_JUMPING`.
    pub long_jump: KeyframeAnimationState,
    /// Vanilla `Breeze.soundTick` countdown (`1..=80`) for local whirl sound (`Breeze.java:144-147`).
    #[serde(skip)]
    pub sound_tick: u8,
    /// Vanilla `Mob.ambientSoundTime` for local `idle_ground`/`idle_air` (`Breeze.java:183-187`).
    #[serde(skip)]
    pub ambient_sound_tick: i16,
    /// Vanilla `Breeze.jumpTrailStartedTick` (`1..=5`, `Breeze.java:157-167`).
    #[serde(skip)]
    pub jump_trail_started_tick: u8,
}

impl Default for BreezeAnimationState {
    fn default() -> Self {
        Self {
            prev_pose: BREEZE_POSE_UNSET,
            shoot: KeyframeAnimationState::default(),
            slide: KeyframeAnimationState::default(),
            slide_back: KeyframeAnimationState::default(),
            inhale: KeyframeAnimationState::default(),
            long_jump: KeyframeAnimationState::default(),
            sound_tick: 0,
            ambient_sound_tick: -80,
            jump_trail_started_tick: 0,
        }
    }
}

impl BreezeAnimationState {
    /// Vanilla `Breeze.onSyncedDataUpdated(DATA_POSE)` + `Breeze.tick`, collapsed: gate each one-shot
    /// on its pose (`animate_when`, which `startIfStopped`s on the rising edge and stops on the leave
    /// — vanilla's `resetAnimations` + per-tick `startIfStopped`), and fire `slideBack` on the falling
    /// edge of `Pose.SLIDING`.
    pub(super) fn set_pose(&mut self, pose_id: i32, age_ticks: u32) {
        if self.prev_pose == VANILLA_ENTITY_POSE_SLIDING_ID
            && pose_id != VANILLA_ENTITY_POSE_SLIDING_ID
        {
            self.slide_back.start_age = Some(age_ticks);
        }
        self.prev_pose = pose_id;
        self.shoot
            .animate_when(pose_id == VANILLA_ENTITY_POSE_SHOOTING_ID, age_ticks);
        self.inhale
            .animate_when(pose_id == VANILLA_ENTITY_POSE_INHALING_ID, age_ticks);
        self.slide
            .animate_when(pose_id == VANILLA_ENTITY_POSE_SLIDING_ID, age_ticks);
        self.long_jump
            .animate_when(pose_id == VANILLA_ENTITY_POSE_LONG_JUMPING_ID, age_ticks);
    }
}

/// Canonical client-side guardian attack-beam counter, mirroring vanilla `Guardian.clientSideAttackTime`.
/// Each client tick `Guardian.aiStep` increments it (capped at `getAttackDuration()`) while the synced
/// `DATA_ID_ATTACK_TARGET` names a live target, and `onSyncedDataUpdated(DATA_ID_ATTACK_TARGET)` resets it
/// to `0` when the target changes. `GuardianRenderer.extractRenderState` reads it for the beam's
/// `attackTime` / `attackScale`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GuardianAttackAnimationState {
    /// Vanilla `Guardian.clientSideAttackTime` (the ramp-up counter, `0..=attackDuration`).
    #[serde(default)]
    pub client_side_attack_time: i32,
    /// The last-observed synced `DATA_ID_ATTACK_TARGET`, so a target change resets the counter exactly
    /// as vanilla `onSyncedDataUpdated` does (`0` is "no target").
    #[serde(default)]
    pub previous_attack_target_id: i32,
}

impl GuardianAttackAnimationState {
    /// Advances one client tick of `Guardian.aiStep`'s attack-time counter. `attack_target_id` is the
    /// synced `DATA_ID_ATTACK_TARGET` (`0` = none) and `attack_duration` is `getAttackDuration()`
    /// (`80` guardian / `60` elder).
    pub(super) fn advance_client_tick(&mut self, attack_target_id: i32, attack_duration: i32) {
        // Vanilla `onSyncedDataUpdated(DATA_ID_ATTACK_TARGET)` resets the counter when the target
        // changes (we reset on an observed value change, the client-visible signal).
        if attack_target_id != self.previous_attack_target_id {
            self.client_side_attack_time = 0;
            self.previous_attack_target_id = attack_target_id;
        }
        // Vanilla `aiStep`: `if hasActiveAttackTarget() && clientSideAttackTime < getAttackDuration()`.
        if attack_target_id != 0 && self.client_side_attack_time < attack_duration {
            self.client_side_attack_time += 1;
        }
    }

    /// Vanilla `Guardian.getAttackAnimationScale(pt) = (clientSideAttackTime + pt) / getAttackDuration()`.
    pub(crate) fn attack_scale(&self, partial_tick: f32, attack_duration: i32) -> f32 {
        (self.client_side_attack_time as f32 + partial_tick) / attack_duration as f32
    }

    /// Vanilla `GuardianRenderer.extractRenderState`: `attackTime = getClientSideAttackTime() + pt`.
    pub(crate) fn attack_time(&self, partial_tick: f32) -> f32 {
        self.client_side_attack_time as f32 + partial_tick
    }
}

/// Vanilla `Guardian.getAttackDuration()`: `80` for a guardian, `60` for an elder guardian.
pub(crate) fn guardian_attack_duration(entity_type_id: i32) -> i32 {
    if entity_type_id == VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID {
        60
    } else {
        80
    }
}

/// Vanilla `Guardian.DATA_ID_ATTACK_TARGET` (the active target entity id, or `0` for none). The
/// generated accessor bundle supplies the inherited wire index/default and excludes non-guardians.
pub(crate) fn guardian_attack_target_id(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> i32 {
    let Some(accessors) =
        vanilla_entity_metadata_guardian_accessors_for_entity_type(entity_type_id)
    else {
        return 0;
    };
    entity_data_int(
        data_values,
        accessors.attack_target_data_id,
        accessors.attack_target_default,
    )
}

/// Canonical client-side limb-swing accumulator, mirroring vanilla
/// `WalkAnimationState` (`net.minecraft.world.entity.WalkAnimationState`). Each
/// client tick `LivingEntity.calculateEntityAnimation` measures the entity's
/// per-tick travel and feeds it to `update`, which low-passes the speed and
/// integrates the swing position; the living-entity models read the lerped
/// `position`/`speed` to sway legs and arms. Tracked for every living entity
/// whose `updateWalkAnimation` is the base mapping.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WalkAnimationState {
    /// Vanilla `WalkAnimationState.speedOld`.
    pub speed_old: f32,
    /// Vanilla `WalkAnimationState.speed`.
    pub speed: f32,
    /// Vanilla `WalkAnimationState.position`.
    pub position: f32,
    /// Vanilla `WalkAnimationState.positionScale` (`1.0`, or `3.0` for a baby).
    pub position_scale: f32,
    /// The entity feet position recorded at the previous client tick, used to
    /// measure the per-tick travel distance (vanilla `Entity.xo/yo/zo`). `None`
    /// until the first tick, when vanilla's `xo == getX()` makes the travel zero.
    #[serde(default)]
    pub previous_feet_position: Option<EntityVec3>,
}

impl Default for WalkAnimationState {
    fn default() -> Self {
        // Vanilla `WalkAnimationState` initialises `positionScale = 1.0F`; every
        // other field defaults to `0`.
        Self {
            speed_old: 0.0,
            speed: 0.0,
            position: 0.0,
            position_scale: 1.0,
            previous_feet_position: None,
        }
    }
}

impl WalkAnimationState {
    /// Vanilla `WalkAnimationState.setSpeed`: replace only the current sample.
    /// `speedOld`, position, scale, and the previous feet position are retained.
    pub(super) fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    /// Vanilla `WalkAnimationState.update`: `speedOld = speed; speed += (target -
    /// speed) * factor; position += speed; positionScale = positionScale`.
    pub(super) fn update(&mut self, target_speed: f32, factor: f32, position_scale: f32) {
        self.speed_old = self.speed;
        self.speed += (target_speed - self.speed) * factor;
        self.position += self.speed;
        self.position_scale = position_scale;
    }

    /// Vanilla `WalkAnimationState.stop`: zeroes the speed and position (it leaves
    /// `positionScale` untouched, matching vanilla).
    pub(super) fn stop(&mut self) {
        self.speed_old = 0.0;
        self.speed = 0.0;
        self.position = 0.0;
    }

    /// Advances one client tick from the entity's current feet position, mirroring
    /// vanilla `LivingEntity.calculateEntityAnimation` → `updateWalkAnimation` (the
    /// base mapping `targetSpeed = min(distance * 4, 1)`, `factor = 0.4`). `use_y`
    /// is `entity instanceof FlyingAnimal` (the `Mth.length` distance includes the
    /// vertical travel only for flying animals).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn advance_client_tick(
        &mut self,
        entity_type_id: i32,
        feet_position: EntityVec3,
        use_y: bool,
        is_passenger: bool,
        is_alive: bool,
        is_baby: bool,
        frog_jump_started: bool,
        camel_walk_active: bool,
    ) {
        let distance = match self.previous_feet_position {
            Some(previous) => {
                let dx = feet_position.x - previous.x;
                let dy = if use_y {
                    feet_position.y - previous.y
                } else {
                    0.0
                };
                let dz = feet_position.z - previous.z;
                (dx * dx + dy * dy + dz * dz).sqrt() as f32
            }
            None => 0.0,
        };
        self.previous_feet_position = Some(feet_position);
        // Vanilla `LivingEntity.calculateEntityAnimation`: only an alive, non-riding
        // entity animates; otherwise the swing is stopped.
        if is_passenger || !is_alive {
            self.stop();
            return;
        }
        let (target_speed, factor) = walk_update_target_speed(
            entity_type_id,
            distance,
            frog_jump_started,
            camel_walk_active,
        );
        self.update(target_speed, factor, if is_baby { 3.0 } else { 1.0 });
    }

    /// Vanilla `WalkAnimationState.position(partialTicks)`: `(position - speed * (1
    /// - partialTicks)) * positionScale`.
    pub(super) fn position(&self, partial_ticks: f32) -> f32 {
        (self.position - self.speed * (1.0 - partial_ticks)) * self.position_scale
    }

    /// Vanilla `WalkAnimationState.speed(partialTicks)`: `min(lerp(partialTicks,
    /// speedOld, speed), 1)`.
    pub(super) fn speed(&self, partial_ticks: f32) -> f32 {
        (self.speed_old + (self.speed - self.speed_old) * partial_ticks).min(1.0)
    }

    /// Vanilla `WalkAnimationState.isMoving()`: `speed > 1.0E-5`. Read by
    /// `Frog.tick` to gate the swim-idle animation off the limb-swing speed.
    pub(super) fn is_moving(&self) -> bool {
        self.speed > 1.0e-5
    }
}

impl Default for PolarBearStandingAnimationState {
    fn default() -> Self {
        Self {
            target_standing: false,
            previous_ticks: 0.0,
            current_ticks: 0.0,
            dimensions_ticks: 0.0,
        }
    }
}

impl Default for ShulkerPeekAnimationState {
    fn default() -> Self {
        Self {
            target_peek_amount: 0.0,
            previous_peek_amount: 0.0,
            current_peek_amount: 0.0,
        }
    }
}

impl Default for BeeRollAnimationState {
    fn default() -> Self {
        Self {
            rolling: false,
            previous_roll_amount: 0.0,
            current_roll_amount: 0.0,
        }
    }
}

impl Default for FoxAnimationState {
    fn default() -> Self {
        Self {
            interested: false,
            previous_interested_angle: 0.0,
            current_interested_angle: 0.0,
            crouching: false,
            previous_crouch_amount: 0.0,
            current_crouch_amount: 0.0,
        }
    }
}

impl PolarBearStandingAnimationState {
    pub(crate) fn dimensions_height_scale(self) -> f32 {
        1.0 + self.dimensions_ticks / POLAR_BEAR_STAND_ANIMATION_TICKS
    }

    /// Vanilla `PolarBear.getStandingAnimationScale`: `lerp(partialTick,
    /// clientSideStandAnimationO, clientSideStandAnimation) / 6.0`.
    pub(super) fn standing_animation_scale(self, partial_tick: f32) -> f32 {
        let ticks = self.previous_ticks + partial_tick * (self.current_ticks - self.previous_ticks);
        ticks / POLAR_BEAR_STAND_ANIMATION_TICKS
    }

    pub(super) fn set_target(&mut self, target_standing: bool) {
        self.target_standing = target_standing;
    }

    pub(super) fn advance_client_tick(&mut self) {
        if self.current_ticks != self.previous_ticks {
            self.dimensions_ticks = self.current_ticks;
        }

        self.previous_ticks = self.current_ticks;
        self.current_ticks = if self.target_standing {
            (self.current_ticks + 1.0).min(POLAR_BEAR_STAND_ANIMATION_TICKS)
        } else {
            (self.current_ticks - 1.0).max(0.0)
        };
    }
}

impl SheepEatAnimationState {
    /// Vanilla `Sheep.aiStep` on the client: `eatAnimationTick = max(0,
    /// eatAnimationTick - 1)`.
    pub(super) fn advance_client_tick(&mut self) {
        self.eat_animation_tick = (self.eat_animation_tick - 1).max(0);
    }
}

impl WardenTendrilAnimationState {
    /// Vanilla `Warden.tick` client branch: `tendrilAnimationO = tendrilAnimation;
    /// if (tendrilAnimation > 0) tendrilAnimation--`.
    pub(super) fn advance_client_tick(&mut self) {
        self.previous = self.current;
        if self.current > 0 {
            self.current -= 1;
        }
    }

    /// Whether the tendril pulse has fully settled at rest so the state can be
    /// dropped (both the lerp endpoints are back to zero).
    pub(super) fn is_settled(&self) -> bool {
        self.previous == 0 && self.current == 0
    }

    /// Vanilla `Warden.getTendrilAnimation(partialTick)`:
    /// `lerp(partialTick, tendrilAnimationO, tendrilAnimation) / 10`.
    pub(super) fn tendril_animation(self, partial_tick: f32) -> f32 {
        let lerped = self.previous as f32 + partial_tick * (self.current - self.previous) as f32;
        lerped / WARDEN_TENDRIL_ANIMATION_TICKS as f32
    }
}

impl WardenHeartAnimationState {
    /// Vanilla `Warden.tick` client branch: a heartbeat (`tickCount % getHeartBeatDelay()
    /// == 0`) resets `heartAnimation` to `10`, then `heartAnimationO = heartAnimation;
    /// if (heartAnimation > 0) heartAnimation--`. `age_ticks` is the post-increment
    /// `tickCount` (the caller bumps it before the per-type tick) and `heartbeat_delay`
    /// is [`warden_heartbeat_delay`] over the synced anger.
    pub(super) fn advance_client_tick(&mut self, age_ticks: u32, heartbeat_delay: u32) -> bool {
        let heartbeat = heartbeat_delay > 0 && age_ticks.is_multiple_of(heartbeat_delay);
        if heartbeat {
            self.current = WARDEN_HEART_ANIMATION_TICKS;
        }
        self.previous = self.current;
        if self.current > 0 {
            self.current -= 1;
        }
        heartbeat
    }

    /// Vanilla `Warden.getHeartAnimation(partialTick)`:
    /// `lerp(partialTick, heartAnimationO, heartAnimation) / 10`.
    pub(super) fn heart_animation(self, partial_tick: f32) -> f32 {
        let lerped = self.previous as f32 + partial_tick * (self.current - self.previous) as f32;
        lerped / WARDEN_HEART_ANIMATION_TICKS as f32
    }
}

/// Runs one `java.util.Random` draw against a raw 48-bit state cursor.
fn java_random_draw<T>(state: &mut u64, draw: impl FnOnce(&mut LegacyRandomSource) -> T) -> T {
    let mut random = LegacyRandomSource::from_state(*state);
    let value = draw(&mut random);
    *state = random.state();
    value
}

pub(in crate::entities) const fn java_random_seed(seed: i64) -> u64 {
    LegacyRandomSource::with_seed(seed).state()
}

pub(in crate::entities) fn java_random_next_bits(state: &mut u64, bits: u32) -> u32 {
    java_random_draw(state, |random| random.next_bits(bits))
}

pub(super) fn java_random_next_long(state: &mut u64) -> i64 {
    java_random_draw(state, LegacyRandomSource::next_long)
}

pub(in crate::entities) fn java_random_next_int_bound(state: &mut u64, bound: i32) -> i32 {
    java_random_draw(state, |random| random.next_int(bound))
}

pub(super) fn java_random_next_double(state: &mut u64) -> f64 {
    java_random_draw(state, LegacyRandomSource::next_double)
}

pub(in crate::entities) fn java_random_next_float(state: &mut u64) -> f32 {
    java_random_draw(state, LegacyRandomSource::next_float)
}

pub(super) fn lightning_bolt_spawn_random(uuid: Uuid, entity_type_id: i32) -> (u64, i64) {
    let mut random_seed = java_random_seed(spawn_random_bits(
        uuid,
        LIGHTNING_BOLT_CLIENT_RANDOM_SALT ^ entity_type_id as u64,
    ) as i64);
    let render_seed = java_random_next_long(&mut random_seed);
    (random_seed, render_seed)
}

/// Vanilla `AbstractHorse.tailCounter`, with the random idle start driven from a repo-local
/// deterministic Java LCG. The exact vanilla client RNG seed is not sent over the protocol, but the
/// `nextInt(200)` distribution and `tailCounter` lifetime mirror `AbstractHorse.aiStep` / `tick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquineTailAnimationState {
    pub(super) random_seed: u64,
    pub(super) tail_counter: u8,
}

impl EquineTailAnimationState {
    pub(super) fn new(entity_id: i32) -> Self {
        Self {
            random_seed: java_random_seed(i64::from(entity_id)),
            tail_counter: 0,
        }
    }

    pub(super) fn advance_client_tick(&mut self) {
        if java_random_next_int_bound(&mut self.random_seed, 200) == 0 {
            self.tail_counter = 1;
        }
        if self.tail_counter > 0 {
            self.tail_counter += 1;
            if self.tail_counter > 8 {
                self.tail_counter = 0;
            }
        }
    }

    pub(super) fn next_double(&mut self) -> f64 {
        java_random_next_double(&mut self.random_seed)
    }

    pub(super) fn animate_tail(self) -> bool {
        self.tail_counter > 0
    }
}

/// Vanilla `AbstractHorse.eatAnim` / `standAnim` / `mouthAnim` client accumulators. The targets are
/// the synced `AbstractHorse.DATA_ID_FLAGS` bits; the client eases the three floats every tick and
/// `AbstractHorseRenderer.extractRenderState` exposes their partial-tick lerps.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EquinePoseAnimationState {
    pub(super) target_eating: bool,
    pub(super) target_standing: bool,
    pub(super) target_open_mouth: bool,
    pub(super) previous_eat_animation: f32,
    pub(super) current_eat_animation: f32,
    pub(super) previous_stand_animation: f32,
    pub(super) current_stand_animation: f32,
    pub(super) previous_mouth_animation: f32,
    pub(super) current_mouth_animation: f32,
}

impl EquinePoseAnimationState {
    pub(super) fn from_flags(eating: bool, standing: bool, open_mouth: bool) -> Self {
        Self {
            target_eating: eating,
            target_standing: standing,
            target_open_mouth: open_mouth,
            previous_eat_animation: 0.0,
            current_eat_animation: 0.0,
            previous_stand_animation: 0.0,
            current_stand_animation: 0.0,
            previous_mouth_animation: 0.0,
            current_mouth_animation: 0.0,
        }
    }

    pub(super) fn set_flags(&mut self, eating: bool, standing: bool, open_mouth: bool) {
        self.target_eating = eating;
        self.target_standing = standing;
        self.target_open_mouth = open_mouth;
    }

    pub(super) fn advance_client_tick(&mut self) {
        self.previous_eat_animation = self.current_eat_animation;
        if self.target_eating {
            self.current_eat_animation += (1.0 - self.current_eat_animation) * 0.4 + 0.05;
            self.current_eat_animation = self.current_eat_animation.min(1.0);
        } else {
            self.current_eat_animation += (0.0 - self.current_eat_animation) * 0.4 - 0.05;
            self.current_eat_animation = self.current_eat_animation.max(0.0);
        }

        self.previous_stand_animation = self.current_stand_animation;
        if self.target_standing {
            self.current_eat_animation = 0.0;
            self.previous_eat_animation = self.current_eat_animation;
            self.current_stand_animation += (1.0 - self.current_stand_animation) * 0.4 + 0.05;
            self.current_stand_animation = self.current_stand_animation.min(1.0);
        } else {
            self.current_stand_animation += (0.8
                * self.current_stand_animation
                * self.current_stand_animation
                * self.current_stand_animation
                - self.current_stand_animation)
                * 0.6
                - 0.05;
            self.current_stand_animation = self.current_stand_animation.max(0.0);
        }

        self.previous_mouth_animation = self.current_mouth_animation;
        if self.target_open_mouth {
            self.current_mouth_animation += (1.0 - self.current_mouth_animation) * 0.7 + 0.05;
            self.current_mouth_animation = self.current_mouth_animation.min(1.0);
        } else {
            self.current_mouth_animation += (0.0 - self.current_mouth_animation) * 0.7 - 0.05;
            self.current_mouth_animation = self.current_mouth_animation.max(0.0);
        }
    }

    pub(super) fn eat_animation(self, partial_tick: f32) -> f32 {
        self.previous_eat_animation
            + partial_tick * (self.current_eat_animation - self.previous_eat_animation)
    }

    pub(super) fn stand_animation(self, partial_tick: f32) -> f32 {
        self.previous_stand_animation
            + partial_tick * (self.current_stand_animation - self.previous_stand_animation)
    }

    pub(super) fn mouth_animation(self, partial_tick: f32) -> f32 {
        self.previous_mouth_animation
            + partial_tick * (self.current_mouth_animation - self.previous_mouth_animation)
    }
}

/// Java `java.util.Random` LCG `setSeed(seed)` then `nextFloat()`, used to seed
/// `Squid.tentacleSpeed` from the entity id deterministically on the client.
/// Replicates the multiplier/addend/mask of `LevelEventSoundRandomState` (the
/// audio module's Java-compatible LCG) so the value matches vanilla exactly.
pub(super) fn java_random_first_next_float(seed: i64) -> f32 {
    let mut state = java_random_seed(seed);
    let bits = java_random_next_bits(&mut state, 24);
    // `nextFloat() = next(24) / (1 << 24)`.
    bits as f32 / (1_u32 << 24) as f32
}

impl SquidAnimationState {
    /// Vanilla `Squid` constructor: `random.setSeed(getId()); tentacleSpeed = 1 /
    /// (random.nextFloat() + 1) * 0.2`. Seeding by the entity id makes the speed
    /// deterministic on the client (the `aiStep` re-randomization is server-only).
    #[cfg(test)]
    pub(crate) fn new(entity_id: i32) -> Self {
        Self::new_with_y_body_rot(entity_id, 0.0)
    }

    /// Vanilla `LivingEntity.recreateFromPacket` seeds `yBodyRot` and
    /// `yBodyRotO` from the add-entity packet's head yaw before the first squid
    /// `aiStep` refines it from movement.
    pub(crate) fn new_with_y_body_rot(entity_id: i32, y_body_rot: f32) -> Self {
        let tentacle_speed = 1.0 / (java_random_first_next_float(i64::from(entity_id)) + 1.0) * 0.2;
        Self {
            tentacle_speed,
            tentacle_movement: 0.0,
            old_tentacle_movement: 0.0,
            tentacle_angle: 0.0,
            old_tentacle_angle: 0.0,
            x_body_rot: 0.0,
            old_x_body_rot: 0.0,
            y_body_rot,
            old_y_body_rot: y_body_rot,
            z_body_rot: 0.0,
            old_z_body_rot: 0.0,
            rotate_speed: 0.0,
        }
    }

    /// Vanilla `Squid.handleEntityEvent(19)`: resets `tentacleMovement = 0`, the
    /// per-cycle reset broadcast by the server when its counter wraps past `2π`.
    pub(super) fn reset_movement(&mut self) {
        self.tentacle_movement = 0.0;
    }

    /// Advances one client tick of `Squid.aiStep`.
    ///
    /// Vanilla saves the lerp endpoints (`xBodyRotO`/`zBodyRotO`/
    /// `oldTentacleMovement`/`oldTentacleAngle`), advances `tentacleMovement` by
    /// `tentacleSpeed`, and on the client clamps it at `2π` (the server instead
    /// resets to `0` and broadcasts event `19`). The in-water branch then derives
    /// `tentacleAngle`/`rotateSpeed` from the half-cycle position and turns the
    /// body yaw/roll/pitch from the synced velocity. Out of water it switches the
    /// tentacle angle to `abs(sin(tentacleMovement)) * π * 0.25` and eases the
    /// body pitch toward `-90°`; the server-side gravity/levitation branch is not
    /// mirrored because the client renderer reads only these local animation
    /// fields.
    pub(super) fn advance_client_tick(&mut self, delta_movement: EntityVec3, in_water: bool) {
        use std::f32::consts::{PI, TAU};

        self.old_x_body_rot = self.x_body_rot;
        self.old_y_body_rot = self.y_body_rot;
        self.old_z_body_rot = self.z_body_rot;
        self.old_tentacle_movement = self.tentacle_movement;
        self.old_tentacle_angle = self.tentacle_angle;
        self.tentacle_movement += self.tentacle_speed;
        if self.tentacle_movement > TAU {
            // Client clamp (server resets to 0 and broadcasts event 19).
            self.tentacle_movement = TAU;
        }

        if in_water {
            if self.tentacle_movement < PI {
                let scale = self.tentacle_movement / PI;
                self.tentacle_angle = (scale * scale * PI).sin() * PI * 0.25;
                if scale > 0.75 {
                    self.rotate_speed = 1.0;
                } else {
                    self.rotate_speed *= 0.8;
                }
            } else {
                self.tentacle_angle = 0.0;
                self.rotate_speed *= 0.99;
            }

            let horizontal = (delta_movement.x * delta_movement.x
                + delta_movement.z * delta_movement.z)
                .sqrt() as f32;
            self.y_body_rot += (-(delta_movement.x as f32).atan2(delta_movement.z as f32)
                * (180.0 / PI)
                - self.y_body_rot)
                * 0.1;
            self.z_body_rot += PI * self.rotate_speed * 1.5;
            self.x_body_rot += (-(horizontal.atan2(delta_movement.y as f32)) * (180.0 / PI)
                - self.x_body_rot)
                * 0.1;
        } else {
            self.tentacle_angle = self.tentacle_movement.sin().abs() * PI * 0.25;
            self.x_body_rot += (-90.0 - self.x_body_rot) * 0.02;
        }
    }

    /// Vanilla `SquidRenderer.extractRenderState`: `lerp(partialTicks,
    /// oldTentacleAngle, tentacleAngle)`.
    pub(super) fn tentacle_angle(&self, partial_tick: f32) -> f32 {
        self.old_tentacle_angle + (self.tentacle_angle - self.old_tentacle_angle) * partial_tick
    }

    /// Vanilla `SquidRenderer.extractRenderState`: `lerp(partialTicks, xBodyRotO,
    /// xBodyRot)`.
    pub(super) fn x_body_rot(&self, partial_tick: f32) -> f32 {
        self.old_x_body_rot + (self.x_body_rot - self.old_x_body_rot) * partial_tick
    }

    /// Vanilla `LivingEntityRenderer.extractRenderState`:
    /// `Mth.rotLerp(partialTicks, yBodyRotO, yBodyRot)`. Squid updates this from
    /// its movement vector during `aiStep`.
    pub(super) fn y_body_rot(&self, partial_tick: f32) -> f32 {
        self.old_y_body_rot + wrap_degrees(self.y_body_rot - self.old_y_body_rot) * partial_tick
    }

    /// Vanilla `SquidRenderer.extractRenderState`: `lerp(partialTicks, zBodyRotO,
    /// zBodyRot)`.
    pub(super) fn z_body_rot(&self, partial_tick: f32) -> f32 {
        self.old_z_body_rot + (self.z_body_rot - self.old_z_body_rot) * partial_tick
    }
}

impl Default for ChickenFlapAnimationState {
    fn default() -> Self {
        // Vanilla `Chicken` field initializers: `flapping = 1.0F`; every other flap
        // field defaults to `0`.
        Self {
            flap: 0.0,
            o_flap: 0.0,
            flap_speed: 0.0,
            o_flap_speed: 0.0,
            flapping: 1.0,
        }
    }
}

impl ChickenFlapAnimationState {
    /// Advances one client tick of `Chicken.aiStep`'s wing-flap accumulator.
    ///
    /// Vanilla saves the lerp endpoints (`oFlap`/`oFlapSpeed`), drives `flapSpeed`
    /// toward `1` while airborne (toward `0` on the ground) clamped to `0..=1`,
    /// re-seeds `flapping` to `1` whenever the chicken is airborne and decays it by
    /// `0.9`, then integrates `flap += flapping * 2`. The `deltaMovement` tweak in
    /// the same method is server physics and is intentionally not modelled here.
    pub(super) fn advance_client_tick(&mut self, on_ground: bool) {
        self.o_flap = self.flap;
        self.o_flap_speed = self.flap_speed;
        self.flap_speed += if on_ground { -1.0 } else { 4.0 } * 0.3;
        self.flap_speed = self.flap_speed.clamp(0.0, 1.0);
        if !on_ground && self.flapping < 1.0 {
            self.flapping = 1.0;
        }
        self.flapping *= 0.9;
        self.flap += self.flapping * 2.0;
    }

    /// Vanilla `ChickenRenderer.extractRenderState`: `lerp(partialTicks, oFlap,
    /// flap)`.
    pub(super) fn flap(&self, partial_tick: f32) -> f32 {
        self.o_flap + (self.flap - self.o_flap) * partial_tick
    }

    /// Vanilla `ChickenRenderer.extractRenderState`: `lerp(partialTicks, oFlapSpeed,
    /// flapSpeed)`.
    pub(super) fn flap_speed(&self, partial_tick: f32) -> f32 {
        self.o_flap_speed + (self.flap_speed - self.o_flap_speed) * partial_tick
    }
}

impl SlimeAnimationState {
    /// Advances one client tick of `Slime.tick`, including landing particles.
    ///
    /// Vanilla saves the lerp endpoint (`oSquish`), eases `squish` halfway toward
    /// `targetSquish`, then — after `super.tick()` — re-seeds the target on the
    /// `onGround()` landing (`-0.5`) / takeoff (`1.0`) transitions, records the
    /// ground flag, and calls the type-specific `decreaseSquish` implementation.
    pub(super) fn advance_client_tick(
        &mut self,
        on_ground: bool,
        magma_cube: bool,
        width: f32,
        random_seed: &mut u64,
        triggers: &mut EntityClientTickParticleTriggers,
    ) {
        self.o_squish = self.squish;
        self.squish += (self.target_squish - self.squish) * 0.5;
        if on_ground && !self.was_on_ground {
            const MAX_SLIME_LANDING_PARTICLES: usize = 4096;
            let radius = if width.is_finite() && width > 0.0 {
                width
            } else {
                0.0
            };
            let particle_count =
                ((radius * 2.0 * 16.0).ceil() as usize).min(MAX_SLIME_LANDING_PARTICLES);
            let mut offsets = Vec::with_capacity(particle_count);
            for _ in 0..particle_count {
                let direction = java_random_next_float(random_seed) * std::f32::consts::TAU;
                let distance = java_random_next_float(random_seed) * 0.5 + 0.5;
                offsets.push(EntityVec3 {
                    x: f64::from(
                        super::super::client_movement::vanilla_mth_sin(f64::from(direction))
                            * radius
                            * distance,
                    ),
                    y: 0.0,
                    z: f64::from(
                        super::super::client_movement::vanilla_mth_cos(f64::from(direction))
                            * radius
                            * distance,
                    ),
                });
            }
            triggers.slime_landing_particles = Some(SlimeLandingParticleTrigger {
                kind: if magma_cube {
                    super::super::SlimeLandingParticleKind::Flame
                } else {
                    super::super::SlimeLandingParticleKind::ItemSlime
                },
                offsets,
            });
            self.target_squish = -0.5;
        } else if !on_ground && self.was_on_ground {
            self.target_squish = 1.0;
        }
        self.was_on_ground = on_ground;
        self.target_squish *= if magma_cube { 0.9 } else { 0.6 };
    }

    /// Vanilla `SlimeRenderer.extractRenderState`: `lerp(partialTicks, oSquish,
    /// squish)`.
    pub(super) fn squish(&self, partial_tick: f32) -> f32 {
        self.o_squish + (self.squish - self.o_squish) * partial_tick
    }
}

impl Default for EvokerFangsAnimationState {
    fn default() -> Self {
        // Vanilla `EvokerFangs` field initializers: `lifeTicks = 22`,
        // `clientSideAttackStarted = false`.
        Self {
            started: false,
            life_ticks: EVOKER_FANGS_LIFE_TICKS,
            client_particle_random_seed: 0,
        }
    }
}

impl EvokerFangsAnimationState {
    pub(super) fn new(entity_id: i32) -> Self {
        Self {
            client_particle_random_seed: java_random_seed(
                i64::from(entity_id).wrapping_add(EVOKER_FANGS_PARTICLE_RANDOM_SALT),
            ),
            ..Self::default()
        }
    }

    /// Advances one client tick of `EvokerFangs.tick`. Vanilla decrements `lifeTicks`
    /// every tick once the attack has started; the count is clamped at `2` (where the
    /// progress ramp has already saturated at `1.0` — the vanished state) to bound it
    /// while the entity waits to be removed.
    pub(super) fn advance_client_tick(
        &mut self,
        entity_id: i32,
    ) -> Option<[EvokerFangsCritParticleOffset; EVOKER_FANGS_CRIT_PARTICLE_COUNT]> {
        if self.client_particle_random_seed == 0 {
            self.client_particle_random_seed = java_random_seed(
                i64::from(entity_id).wrapping_add(EVOKER_FANGS_PARTICLE_RANDOM_SALT),
            );
        }
        if self.started && self.life_ticks > 2 {
            self.life_ticks -= 1;
            if self.life_ticks == EVOKER_FANGS_CRIT_PARTICLE_TICK {
                return Some(std::array::from_fn(|_| EvokerFangsCritParticleOffset {
                    x_unit: java_random_next_double(&mut self.client_particle_random_seed) * 2.0
                        - 1.0,
                    y_offset: 1.05 + java_random_next_double(&mut self.client_particle_random_seed),
                    z_unit: java_random_next_double(&mut self.client_particle_random_seed) * 2.0
                        - 1.0,
                    velocity: EntityVec3 {
                        x: (java_random_next_double(&mut self.client_particle_random_seed) * 2.0
                            - 1.0)
                            * 0.3,
                        y: 0.3
                            + java_random_next_double(&mut self.client_particle_random_seed) * 0.3,
                        z: (java_random_next_double(&mut self.client_particle_random_seed) * 2.0
                            - 1.0)
                            * 0.3,
                    },
                }));
            }
        }
        None
    }

    /// Vanilla `EvokerFangs.getAnimationProgress(partialTick)`: `0` until the attack
    /// starts, then the `lifeTicks`-driven `0..1` ramp `1 - (lifeTicks - 2 -
    /// partialTick) / 20`, saturating at `1.0` once `lifeTicks <= 2`.
    pub(super) fn bite_progress(&self, partial_tick: f32) -> f32 {
        if !self.started {
            return 0.0;
        }
        let remaining_life = self.life_ticks - 2;
        if remaining_life <= 0 {
            1.0
        } else {
            1.0 - (remaining_life as f32 - partial_tick) / 20.0
        }
    }
}

impl Default for ParrotFlapAnimationState {
    fn default() -> Self {
        // Vanilla `Parrot` field initializers: `flapping = 1.0F` (and `nextFlap =
        // 1.0F`, which is server-only re-seeding noise); every other flap field
        // defaults to `0`.
        Self {
            flap: 0.0,
            o_flap: 0.0,
            flap_speed: 0.0,
            o_flap_speed: 0.0,
            flapping: 1.0,
        }
    }
}

impl ParrotFlapAnimationState {
    /// Advances one client tick of `Parrot.calculateFlapping`'s wing-flap
    /// accumulator.
    ///
    /// Vanilla saves the lerp endpoints (`oFlap`/`oFlapSpeed`), drives `flapSpeed`
    /// toward `1` while `!onGround() && !isPassenger()` (toward `0` otherwise)
    /// clamped to `0..=1`, re-seeds `flapping` to `1` whenever the parrot is
    /// airborne, decays it by `0.9`, then integrates `flap += flapping * 2`. This is
    /// identical to the chicken flap except for the added `!isPassenger()` term: a
    /// parrot riding a shoulder or mount lets its wings settle. Note the airborne
    /// re-seed of `flapping` uses only `!onGround()` (not the passenger test), the
    /// same as vanilla.
    pub(super) fn advance_client_tick(&mut self, on_ground: bool, is_passenger: bool) {
        self.o_flap = self.flap;
        self.o_flap_speed = self.flap_speed;
        self.flap_speed += if !on_ground && !is_passenger {
            4.0
        } else {
            -1.0
        } * 0.3;
        self.flap_speed = self.flap_speed.clamp(0.0, 1.0);
        if !on_ground && self.flapping < 1.0 {
            self.flapping = 1.0;
        }
        self.flapping *= 0.9;
        self.flap += self.flapping * 2.0;
    }

    /// Vanilla `ParrotRenderer.extractRenderState`: lerps `flap` and `flapSpeed`
    /// separately by the partial tick, then `flapAngle = (sin(flap) + 1) *
    /// flapSpeed`.
    pub(super) fn flap_angle(&self, partial_tick: f32) -> f32 {
        let flap = self.o_flap + (self.flap - self.o_flap) * partial_tick;
        let flap_speed = self.o_flap_speed + (self.flap_speed - self.o_flap_speed) * partial_tick;
        (flap.sin() + 1.0) * flap_speed
    }
}

impl CreeperSwellAnimationState {
    pub(super) fn set_swell_dir(&mut self, swell_dir: i32) {
        self.swell_dir = swell_dir;
    }

    /// Vanilla `Creeper.tick`: `oldSwell = swell; swell += swellDir`, clamped to
    /// `0..=maxSwell`.
    pub(super) fn advance_client_tick(&mut self) {
        self.previous_swell = self.current_swell;
        self.current_swell = (self.current_swell + self.swell_dir).clamp(0, CREEPER_MAX_SWELL);
    }

    /// Whether the fuse has fully settled at rest so the state can be dropped.
    pub(super) fn is_settled(&self) -> bool {
        self.swell_dir <= 0 && self.current_swell == 0 && self.previous_swell == 0
    }

    /// Vanilla `Creeper.getSwelling`: `lerp(partialTick, oldSwell, swell) /
    /// (maxSwell - 2)`.
    pub(super) fn swelling(self, partial_tick: f32) -> f32 {
        let lerped = self.previous_swell as f32
            + partial_tick * (self.current_swell - self.previous_swell) as f32;
        lerped / (CREEPER_MAX_SWELL as f32 - 2.0)
    }
}

impl ShulkerPeekAnimationState {
    pub(super) fn set_target(&mut self, target_peek_amount: f32) {
        self.target_peek_amount = target_peek_amount;
    }

    pub(super) fn advance_client_tick(&mut self) {
        self.previous_peek_amount = self.current_peek_amount;
        if self.current_peek_amount == self.target_peek_amount {
            return;
        }

        self.current_peek_amount = if self.current_peek_amount > self.target_peek_amount {
            (self.current_peek_amount - SHULKER_PEEK_PER_TICK)
                .clamp(self.target_peek_amount, SHULKER_MAX_PEEK_AMOUNT)
        } else {
            (self.current_peek_amount + SHULKER_PEEK_PER_TICK).clamp(0.0, self.target_peek_amount)
        };
    }

    /// Vanilla `Shulker.getClientPeekAmount(partialTick)` =
    /// `Mth.lerp(partialTick, currentPeekAmountO, currentPeekAmount)`.
    pub(super) fn peek_amount(self, partial_tick: f32) -> f32 {
        self.previous_peek_amount
            + partial_tick * (self.current_peek_amount - self.previous_peek_amount)
    }
}

impl BeeRollAnimationState {
    pub(super) fn set_rolling(&mut self, rolling: bool) {
        self.rolling = rolling;
    }

    /// Vanilla `Bee.updateRollAmount`: `rollAmountO = rollAmount`, then a rolling
    /// bee climbs `min(1, rollAmount + 0.2)` while a non-rolling one decays
    /// `max(0, rollAmount - 0.24)`.
    pub(super) fn advance_client_tick(&mut self) {
        self.previous_roll_amount = self.current_roll_amount;
        self.current_roll_amount = if self.rolling {
            (self.current_roll_amount + 0.2).min(1.0)
        } else {
            (self.current_roll_amount - 0.24).max(0.0)
        };
    }

    /// Vanilla `Bee.getRollAmount(partialTick)` =
    /// `Mth.lerp(partialTick, rollAmountO, rollAmount)`.
    pub(super) fn roll_amount(self, partial_tick: f32) -> f32 {
        self.previous_roll_amount
            + partial_tick * (self.current_roll_amount - self.previous_roll_amount)
    }
}

impl PandaAnimationState {
    pub(super) fn set_flags(&mut self, sitting: bool, on_back: bool, rolling: bool) {
        self.sitting = sitting;
        self.on_back = on_back;
        if rolling {
            if !self.rolling_locally_cleared {
                self.rolling = true;
            }
        } else {
            self.rolling = false;
            self.rolling_locally_cleared = false;
        }
    }

    pub(super) fn advance_client_tick(&mut self) {
        self.previous_sit_amount = self.current_sit_amount;
        self.current_sit_amount = ease_panda_amount(self.current_sit_amount, self.sitting);

        self.previous_on_back_amount = self.current_on_back_amount;
        self.current_on_back_amount = ease_panda_amount(self.current_on_back_amount, self.on_back);

        self.previous_roll_amount = self.current_roll_amount;
        if self.rolling {
            self.roll_counter += 1;
            if self.roll_counter > PANDA_ROLL_COUNTER_MAX {
                self.rolling = false;
                self.rolling_locally_cleared = true;
            }
        } else {
            self.roll_counter = 0;
        }
        self.current_roll_amount = ease_panda_amount(self.current_roll_amount, self.rolling);
    }

    pub(super) fn sit_amount(self, partial_tick: f32) -> f32 {
        lerp_panda_amount(
            partial_tick,
            self.previous_sit_amount,
            self.current_sit_amount,
        )
    }

    pub(super) fn lie_on_back_amount(self, partial_tick: f32) -> f32 {
        lerp_panda_amount(
            partial_tick,
            self.previous_on_back_amount,
            self.current_on_back_amount,
        )
    }

    pub(super) fn roll_amount(self, partial_tick: f32) -> f32 {
        lerp_panda_amount(
            partial_tick,
            self.previous_roll_amount,
            self.current_roll_amount,
        )
    }

    pub(super) fn roll_time(self, partial_tick: f32) -> f32 {
        if self.roll_counter > 0 {
            self.roll_counter as f32 + partial_tick
        } else {
            0.0
        }
    }

    pub(super) fn is_settled(self) -> bool {
        !self.sitting
            && !self.on_back
            && !self.rolling
            && !self.rolling_locally_cleared
            && self.previous_sit_amount == 0.0
            && self.current_sit_amount == 0.0
            && self.previous_on_back_amount == 0.0
            && self.current_on_back_amount == 0.0
            && self.previous_roll_amount == 0.0
            && self.current_roll_amount == 0.0
            && self.roll_counter == 0
    }
}

pub(super) fn ease_panda_amount(current: f32, active: bool) -> f32 {
    if active {
        (current + PANDA_AMOUNT_RISE_PER_TICK).min(1.0)
    } else {
        (current - PANDA_AMOUNT_FALL_PER_TICK).max(0.0)
    }
}

pub(super) fn lerp_panda_amount(partial_tick: f32, previous: f32, current: f32) -> f32 {
    previous + partial_tick * (current - previous)
}

impl FoxAnimationState {
    pub(super) fn set_flags(&mut self, interested: bool, crouching: bool) {
        self.interested = interested;
        self.crouching = crouching;
    }

    /// Vanilla `Fox.tick`: `interestedAngleO = interestedAngle`, then it eases
    /// toward `isInterested ? 1 : 0` by `* 0.4`/tick. Independently,
    /// `crouchAmountO = crouchAmount`, then a crouching fox climbs by `0.2`/tick
    /// (clamped to `MAX_CROUCH_AMOUNT`) while a non-crouching one is reset INSTANTLY
    /// to `0` (an assignment, not a decay).
    pub(super) fn advance_client_tick(&mut self) {
        self.previous_interested_angle = self.current_interested_angle;
        let target = if self.interested { 1.0 } else { 0.0 };
        self.current_interested_angle +=
            (target - self.current_interested_angle) * FOX_INTERESTED_EASE;

        self.previous_crouch_amount = self.current_crouch_amount;
        if self.crouching {
            self.current_crouch_amount =
                (self.current_crouch_amount + FOX_CROUCH_PER_TICK).min(FOX_MAX_CROUCH_AMOUNT);
        } else {
            self.current_crouch_amount = 0.0;
        }
    }

    /// Vanilla `Fox.getHeadRollAngle(partialTick)` =
    /// `Mth.lerp(partialTick, interestedAngleO, interestedAngle) * 0.11 * π`.
    pub(super) fn head_roll_angle(self, partial_tick: f32) -> f32 {
        let lerped = self.previous_interested_angle
            + partial_tick * (self.current_interested_angle - self.previous_interested_angle);
        lerped * FOX_HEAD_ROLL_SCALE * std::f32::consts::PI
    }

    /// Vanilla `Fox.getCrouchAmount(partialTick)` =
    /// `Mth.lerp(partialTick, crouchAmountO, crouchAmount)`.
    pub(super) fn crouch_amount(self, partial_tick: f32) -> f32 {
        self.previous_crouch_amount
            + partial_tick * (self.current_crouch_amount - self.previous_crouch_amount)
    }
}
