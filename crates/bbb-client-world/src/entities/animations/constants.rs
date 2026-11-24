/// Vanilla `AbstractBoat.tick` advances each active `paddlePositions[side]` by
/// `PI / 8` per client tick, otherwise resetting it to zero.
pub(super) const BOAT_PADDLE_ADVANCE_PER_TICK: f32 = std::f32::consts::PI / 8.0;

/// Vanilla `EntityType.ARROW` / `EntityType.SPECTRAL_ARROW`: both extend
/// `AbstractArrow` and share the same client-side `shakeTime` impact wobble.
/// Vanilla `AbstractArrow.IN_GROUND`, the synced boolean after `ID_FLAGS` (`8`) and
/// `PIERCE_LEVEL` (`9`). `onSyncedDataUpdated(IN_GROUND)` starts `shakeTime = 7`
/// when the arrow is no longer on its first tick and the current shake has settled.
pub(super) const ARROW_SHAKE_TICKS: i32 = 7;

/// Vanilla `ArmorStand.WOBBLE_TIME`.
pub(super) const ARMOR_STAND_WOBBLE_TIME: i32 = 5;

/// Vanilla `EntityType.WITHER`; its two alternative render heads keep client-side
/// `xRotHeads` / `yRotHeads` arrays that lerp toward synced target entities.
/// Vanilla `WitherBoss.DATA_TARGET_B/C`, the two side-head target ids. `DATA_TARGET_A`
/// (16) is the center combat target; model side heads read B/C at 17/18.
/// Vanilla `FlyingAnimal` implementors (`Bee`, `Parrot`): their
/// `LivingEntity.calculateEntityAnimation` measures the full 3-D travel distance
/// (`calculateEntityAnimation(this instanceof FlyingAnimal)`), so the limb-swing
/// distance includes the vertical component.
/// Vanilla `AbstractHorse.DATA_ID_FLAGS` (id 18): the shared equine flags byte after
/// `AgeableMob`'s two accessors. `AbstractHorse.tick` eases render-state eat / stand / mouth
/// amounts toward these bits every client tick.
/// Vanilla `Panda.updateSitAmount` / `updateOnBackAnimation` / `updateRollAmount` easing.
pub(super) const PANDA_AMOUNT_RISE_PER_TICK: f32 = 0.15;

pub(super) const PANDA_AMOUNT_FALL_PER_TICK: f32 = 0.19;

pub(super) const PANDA_ROLL_COUNTER_MAX: i32 = 32;

/// Vanilla `Armadillo.ArmadilloState.SCARED.animationDuration()`; the first SCARED
/// setup tick starts `peekAnimationState` and `fastForward`s it by this many ticks.
pub(super) const ARMADILLO_STATE_SCARED_ANIMATION_TICKS: i32 = 50;

pub(super) const POLAR_BEAR_STAND_ANIMATION_TICKS: f32 = 6.0;

/// Vanilla `Creeper.DATA_SWELL_DIR` (synced fuse direction, default `-1`) and
/// `DATA_IS_IGNITED`; the client advances `swell` toward `maxSwell` while the
/// effective direction is positive.
/// Vanilla `Creeper.DEFAULT_MAX_SWELL`. `getSwelling` divides the lerped swell
/// by `maxSwell - 2`.
pub(super) const CREEPER_MAX_SWELL: i32 = 30;

pub(super) const SHULKER_PEEK_PER_TICK: f32 = 0.05;

pub(super) const SHULKER_MAX_PEEK_AMOUNT: f32 = 1.0;

/// Vanilla `Cat.updateLieDownAmount` / `updateRelaxStateOneAmount` easing.
pub(super) const CAT_LIE_DOWN_AMOUNT_RISE_PER_TICK: f32 = 0.15;

pub(super) const CAT_LIE_DOWN_AMOUNT_FALL_PER_TICK: f32 = 0.22;

pub(super) const CAT_LIE_DOWN_TAIL_AMOUNT_RISE_PER_TICK: f32 = 0.08;

pub(super) const CAT_LIE_DOWN_TAIL_AMOUNT_FALL_PER_TICK: f32 = 0.13;

pub(super) const CAT_RELAX_STATE_ONE_AMOUNT_RISE_PER_TICK: f32 = 0.1;

pub(super) const CAT_RELAX_STATE_ONE_AMOUNT_FALL_PER_TICK: f32 = 0.13;

/// Vanilla wolf shake/drying progress increment (`Wolf.tick`: `shakeAnim += 0.05F`).
pub(super) const WOLF_SHAKE_ANIM_PER_TICK: f32 = 0.05;

/// Vanilla wolf wet shade floor (`Wolf.getWetShade`: `0.75 + lerp(shake) / 2 * 0.25`).
pub(super) const WOLF_WET_SHADE_BASE: f32 = 0.75;

/// Vanilla wolf wet shade shake multiplier (`lerp(shakeAnimO, shakeAnim) / 2 * 0.25`).
pub(super) const WOLF_WET_SHADE_SHAKE_SCALE: f32 = 0.125;

/// Vanilla wolf drying completes once `shakeAnimO >= 2.0F`; the shade reaches white at `shakeAnim = 2`.
pub(super) const WOLF_SHAKE_ANIM_DONE: f32 = 2.0;

/// Vanilla `Wolf.tick` interested-angle easing toward the synced `DATA_INTERESTED_ID` target.
pub(super) const WOLF_INTERESTED_EASE: f32 = 0.4;

/// Vanilla `Wolf.getHeadRollAngle(partialTick)`: `lerp(interestedAngleO, interestedAngle) * 0.15 * π`.
pub(super) const WOLF_HEAD_ROLL_SCALE: f32 = 0.15;

/// Vanilla `LivingEntity.updateSwimAmount`: the easing step per client tick.
pub(super) const LIVING_SWIM_AMOUNT_PER_TICK: f32 = 0.09;

/// Vanilla `LivingEntity.updateSwimAmount`: the fully visually-swimming upper bound.
pub(super) const LIVING_SWIM_AMOUNT_MAX: f32 = 1.0;

pub(crate) use bbb_render_types::{
    ELYTRA_DEFAULT_X_ROT, ELYTRA_DEFAULT_Y_ROT, ELYTRA_DEFAULT_Z_ROT,
};

pub(super) const ELYTRA_FALL_FLYING_X_ROT: f32 = std::f32::consts::PI / 9.0;

pub(super) const ELYTRA_FALL_FLYING_Z_ROT: f32 = -std::f32::consts::PI / 2.0;

pub(super) const ELYTRA_CROUCHING_X_ROT: f32 = std::f32::consts::PI * 2.0 / 9.0;

pub(super) const ELYTRA_CROUCHING_Y_ROT: f32 = 0.087_266_46;

pub(super) const ELYTRA_CROUCHING_Z_ROT: f32 = -std::f32::consts::PI / 4.0;

pub(super) const ELYTRA_ROT_EASE: f32 = 0.3;

pub(super) const PLAYER_CLOAK_TELEPORT_THRESHOLD: f64 = 10.0;

pub(super) const PLAYER_CLOAK_EASE: f64 = 0.25;

pub(super) const PLAYER_CLOAK_WALK_DISTANCE_SCALE: f32 = 0.6;

/// Vanilla `Rabbit.handleEntityEvent`: event `1` (`spawnSprintParticle`) also seeds the client-side
/// hop reconstruction — `jumpDuration = 15; jumpTicks = 0;` — so the `hopAnimationState` plays for
/// one 15-tick (0.75s, exactly one loop of `RabbitAnimation.HOP`) jump arc.
/// Vanilla `Rabbit.startJumping` / `handleEntityEvent(1)`: `this.jumpDuration = 15`.
pub(super) const RABBIT_JUMP_DURATION: i32 = 15;

/// Vanilla `Axolotl`'s four animators are each `BinaryAnimator(10, IN_OUT_SINE)`, so every factor
/// eases between 0 and 1 over ten ticks.
pub(super) const AXOLOTL_ANIMATOR_LENGTH: i32 = 10;

/// Vanilla `Pillager.IS_CHARGING_CROSSBOW`, the synced `EntityDataSerializers.BOOLEAN` accessor at id 17
/// (Entity 0–7, LivingEntity 8–14, Mob 15, `Raider.IS_CELEBRATING` 16, then `Pillager`'s first own
/// accessor). While set, the pillager is drawing its crossbow, and the client reconstructs
/// `getTicksUsingItem` by counting ticks since the flag rose (the using-item flag rises with it).
/// Vanilla `Piglin.DATA_IS_CHARGING_CROSSBOW` synched-data id (18, the regular piglin's own slot — past
/// `DATA_BABY_ID`, `DATA_IS_IMMUNE_TO_ZOMBIFICATION`). Drives the same crossbow-draw counter as the
/// pillager (the flag is a different id on a different class, so read it under a piglin type gate).
/// Vanilla `Fox.interestedAngle` per-tick ease toward the `FLAG_INTERESTED`
/// target: `interestedAngle += (target - interestedAngle) * 0.4`.
pub(super) const FOX_INTERESTED_EASE: f32 = 0.4;

/// Vanilla `Fox.getHeadRollAngle` scale: `lerp(pt, interestedAngleO,
/// interestedAngle) * 0.11 * π`.
pub(super) const FOX_HEAD_ROLL_SCALE: f32 = 0.11;

/// Vanilla `Fox.crouchAmount` per-tick climb while `FLAG_CROUCHING` is set
/// (`crouchAmount += 0.2`).
pub(super) const FOX_CROUCH_PER_TICK: f32 = 0.2;

/// Vanilla `Fox.MAX_CROUCH_AMOUNT`: the crouch accumulator saturates here.
pub(super) const FOX_MAX_CROUCH_AMOUNT: f32 = 5.0;

/// Vanilla `LivingEntity.hurtDuration`: the hurt animation (and red damage
/// overlay) runs for 10 client ticks after a hurt animation or damage event.
pub(super) const HURT_ANIMATION_DURATION: i32 = 10;

/// Vanilla `LivingEntity.onKineticHit`: ignore repeated hit feedback until more
/// than ten game ticks have elapsed since the last accepted hit.
pub(super) const KINETIC_HIT_FEEDBACK_RETRIGGER_TICKS: i32 = 10;

/// Vanilla `SwingAnimation.DEFAULT.duration()`: the empty hand and common-item
/// WHACK swing ramps `attackAnim` from `0` to `1` over six client ticks.
/// Per-item default durations and dig-speed / mining-fatigue modifiers are recorded on each
/// triggered swing and refreshed when runtime state changes.
pub(crate) const ATTACK_SWING_DURATION: i32 = 6;

pub(super) const fn default_attack_swing_duration() -> i32 {
    ATTACK_SWING_DURATION
}

/// Vanilla `LivingEntity.tickDeath` removes the entity at `deathTime >= 20`, and
/// the death tip-over flip is fully clamped by then, so the client counter is
/// capped here to stay bounded when no server removal arrives.
pub(super) const DEATH_ANIMATION_MAX_TICKS: i32 = 20;

/// Vanilla `EnderDragon.tickDeath`: `dragonDeathTime++`, with renderer fade/rays normalized over
/// 200 ticks and server removal at `>= 200`.
pub(super) const ENDER_DRAGON_DEATH_ANIMATION_MAX_TICKS: i32 = 200;

/// Vanilla `Sheep.EAT_ANIMATION_TICKS`: the eat-grass animation runs for 40
/// client ticks after entity event `10`.
pub(super) const SHEEP_EAT_ANIMATION_TICKS: i32 = 40;

/// Vanilla `Warden`: the client decrements `tendrilAnimation` from `10` toward
/// `0` each tick; `getTendrilAnimation` divides the lerped value by `10`.
pub(super) const WARDEN_TENDRIL_ANIMATION_TICKS: i32 = 10;

/// Vanilla `Warden`: the client resets `heartAnimation` to `10` on each heartbeat
/// and decrements it toward `0` each tick; `getHeartAnimation` divides the lerped
/// value by `10`. Shares the `tendrilAnimation` range, hence the same `10` cap.
pub(super) const WARDEN_HEART_ANIMATION_TICKS: i32 = 10;

/// Vanilla `Warden.CLIENT_ANGER_LEVEL` (`getClientAngerLevel()`); membership, wire index and typed
/// default come from the generated Warden accessor.
/// Vanilla `AngerLevel.ANGRY.getMinimumAnger()`: the anger at which the heartbeat
/// reaches its fastest, used to normalise `getHeartBeatDelay`.
pub(super) const WARDEN_ANGRY_MINIMUM_ANGER: i32 = 80;

/// The "not in a triggered pose" sentinel for [`WardenCombatAnimationState::prev_pose`]:
/// no synced `DATA_POSE` has been observed yet, so the first ROARING/SNIFFING pose is a
/// fresh transition that starts the matching timer.
pub(super) const WARDEN_POSE_UNSET: i32 = -1;

/// Vanilla `Goat.handleEntityEvent`: event `58` sets `isLoweringHead = true` (the goat begins a ram),
/// event `59` clears it. The client `aiStep` then advances `lowerHeadTick` toward / away from the cap.
/// Vanilla `Goat.aiStep` clamps `lowerHeadTick` to `[0, 20]`: `++` while lowering, `-= 2` otherwise.
/// `getRammingXHeadRot` normalises it by this cap.
pub(super) const GOAT_LOWER_HEAD_MAX_TICKS: i32 = 20;

/// Vanilla `IronGolem.handleEntityEvent`: event `4` sets `attackAnimationTick = 10` (the two-fisted
/// smash), event `11` sets `offerFlowerTick = 400` (offering a poppy to a villager), event `34` clears
/// it. Both counters decrement each client `aiStep` toward `0`.
pub(super) const IRON_GOLEM_ATTACK_TICKS: i32 = 10;

pub(super) const IRON_GOLEM_OFFER_FLOWER_TICKS: i32 = 400;

/// Vanilla `Ravager.handleEntityEvent`: event `4` sets `attackTick = 10` (the bite), event `39` sets
/// `stunnedTick = 40` (a shield-block stun). The roar is not event-driven — when `stunnedTick` decays to
/// `0` the client `aiStep` sets `roarTick = 20`, so a roar always follows a stun.
pub(super) const RAVAGER_ATTACK_TICKS: i32 = 10;

pub(super) const RAVAGER_STUN_TICKS: i32 = 40;

pub(super) const RAVAGER_ROAR_TICKS: i32 = 20;

pub(super) const RAVAGER_STUN_PARTICLE_RANDOM_SALT: i64 = 0x5A17;

/// Client-local RNG salt for `Creaking.checkEyeBlink`. Vanilla seeds each entity's
/// `RandomSource` from process-local entropy, so the exact cursor is not present on the wire; bbb
/// keeps the same Java LCG and interval rules behind a stable per-entity seed.
pub(super) const CREAKING_EYE_FLICKER_RANDOM_SALT: i64 = 0x0043_5245_414B;

/// Client-local RNG salt for `Blaze.aiStep`. Vanilla does not synchronize the
/// entity `RandomSource`; the cursor still belongs to the canonical entity and
/// is cloned into immutable publications so retained generations cannot advance.
pub(super) const BLAZE_CLIENT_RANDOM_SALT: u64 = 0x626c_617a_655f_7469;

pub(super) const ENDER_AMBIENT_RANDOM_SALT: u64 = 0x656e_6465_725f_7469;

pub(super) const GUARDIAN_CLIENT_RANDOM_SALT: u64 = 0x6775_6172_6469_616e;

pub(super) const ENTITY_CLIENT_RANDOM_SALT: u64 = 0x4d4f_4249;

/// Vanilla `Hoglin`/`Zoglin.handleEntityEvent`: event `4` sets `attackAnimationRemainingTicks = 10` (the
/// headbutt), decremented each client tick. `HoglinModel.setupAnim` drives the head-down ram from it.
pub(super) const HOGLIN_ATTACK_TICKS: i32 = 10;

/// Vanilla `EvokerFangs.handleEntityEvent`: event `4` sets `clientSideAttackStarted = true`, after which
/// `EvokerFangs.tick` decrements `lifeTicks` (field initializer `22`) each client tick. The
/// `getAnimationProgress` ramp built from `lifeTicks` drives `EvokerFangsModel.setupAnim`.
pub(super) const EVOKER_FANGS_LIFE_TICKS: i32 = 22;

pub(super) const EVOKER_FANGS_CRIT_PARTICLE_TICK: i32 = 14;

pub(super) const EVOKER_FANGS_CRIT_PARTICLE_COUNT: usize = 12;

pub(super) const EVOKER_FANGS_PARTICLE_RANDOM_SALT: i64 = 0xF46;

/// Vanilla `Camel.DASH_COOLDOWN_TICKS`: `onSyncedDataUpdated(DASH)` seeds `dashCooldown` to 55 when
/// the synced dash flag rises, and `Camel.tick` decrements it each client tick. The renderer consumes
/// `max(dashCooldown - partialTicks, 0)` as `CamelRenderState.jumpCooldown`.
pub(super) const CAMEL_DASH_COOLDOWN_TICKS: i32 = 55;

/// Vanilla `AgeableMob.AGE_LOCK_COOLDOWN_TICKS`, also assigned to
/// `ageLockParticleTimer` by `setAgeLockedData`.
pub(super) const AGE_LOCK_COOLDOWN_TICKS: u8 = 40;

/// Vanilla `AbstractNautilus.DASH_COOLDOWN_TICKS`. Unlike Camel, metadata updates seed this
/// cooldown from `onSyncedDataUpdated(DASH)` even when the synchronized value is `false`.
pub(super) const NAUTILUS_DASH_COOLDOWN_TICKS: i32 = 40;

pub(super) const NAUTILUS_DASH_MINIMUM_DURATION_TICKS: i32 = 5;

pub(super) const NAUTILUS_BUBBLE_PROBABILITY_MIN: f64 = 0.15;

pub(super) const NAUTILUS_BUBBLE_PROBABILITY_MAX: f64 = 1.0;

pub(super) const NAUTILUS_BUBBLE_PROBABILITY_MULTIPLIER: f64 = 2.0;

pub(super) const NAUTILUS_BUBBLE_SPREAD_FACTOR: f64 = 0.8;

pub(super) const NAUTILUS_CLIENT_RANDOM_SALT: u64 = 0x6e61_7574_696c_7573;

/// Client-local `Slime.random` cursor for the landing particle ring.
pub(super) const SLIME_CLIENT_RANDOM_SALT: u64 = 0x736c_696d_655f_726e;

/// Shared client-local `EnderDragon.random` cursor for flap, ambient growl,
/// sitting-attack growl, and death-particle draws in Java tick order.
pub(super) const ENDER_DRAGON_CLIENT_RANDOM_SALT: u64 = 0x6472_6167_6f6e_5f72;

/// Shared client-local `LightningBolt.random` cursor. Construction consumes
/// render seed then flash count; the first tick consumes thunder and impact
/// pitch in that order.
pub(super) const LIGHTNING_BOLT_CLIENT_RANDOM_SALT: u64 = 0x6c69_6768_746e_696e;

/// Client-local Warden RNG cursor for `LivingEntity.getVoicePitch()` on each
/// heartbeat. The server does not synchronize the entity RNG seed.
pub(super) const WARDEN_CLIENT_RANDOM_SALT: u64 = 0x7761_7264_656e_5f72;

/// Vanilla `Camel.setupAnimationStates`: `idleAnimationTimeout = random.nextInt(40) + 80`, then
/// `idleAnimationState.start(tickCount)`. `CAMEL_IDLE` itself is the 4.0 s non-looping keyframe.
pub(super) const CAMEL_IDLE_TIMEOUT_RANDOM_BOUND: i32 = 40;

pub(super) const CAMEL_IDLE_TIMEOUT_BASE_TICKS: i32 = 80;

/// Vanilla `CopperGolem.setupAnimationStates`: `idleAnimationStartTick = tickCount +
/// random.nextInt(200, 240)`, then `idleAnimationState.start(tickCount)` at that tick.
pub(super) const COPPER_GOLEM_IDLE_TIMEOUT_RANDOM_BOUND: i32 = 40;

pub(super) const COPPER_GOLEM_IDLE_TIMEOUT_BASE_TICKS: i32 = 200;

/// Vanilla `Creaking.handleEntityEvent`: event `4` sets `attackAnimationRemainingTicks = 15` (the
/// lunge), event `66` sets `invulnerabilityAnimationRemainingTicks = 8` (the heart-bound stagger).
/// Both decrement each client tick in `aiStep`, and `setupAnimationStates` `animateWhen`s the
/// matching keyframe one-shot on `ticks > 0`.
pub(super) const CREAKING_ATTACK_DURATION: i32 = 15;

pub(super) const CREAKING_INVULNERABLE_DURATION: i32 = 8;

/// Vanilla `MinecartTNT.handleEntityEvent(10)` calls `primeFuse`, which sets the client-side
/// `fuse` counter to 80. `TntMinecartRenderer.extractRenderState` projects it as
/// `fuse - partialTicks + 1.0`.
pub(super) const TNT_MINECART_FUSE_TICKS: i32 = 80;
