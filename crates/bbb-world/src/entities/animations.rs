use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROAKING_ID, VANILLA_ENTITY_POSE_DIGGING_ID,
    VANILLA_ENTITY_POSE_EMERGING_ID, VANILLA_ENTITY_POSE_FALL_FLYING_ID,
    VANILLA_ENTITY_POSE_INHALING_ID, VANILLA_ENTITY_POSE_LONG_JUMPING_ID,
    VANILLA_ENTITY_POSE_ROARING_ID, VANILLA_ENTITY_POSE_SHOOTING_ID,
    VANILLA_ENTITY_POSE_SLIDING_ID, VANILLA_ENTITY_POSE_SNIFFING_ID,
    VANILLA_ENTITY_POSE_SWIMMING_ID, VANILLA_ENTITY_POSE_USING_TONGUE_ID,
};
use bbb_protocol::packets::{EntityDataEnumSerializer, EntityDataValue, EntityDataValueKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::WorldStore;

use super::dimensions::{entity_data_pose, vanilla_living_entity_type};
use super::dragon::{
    vanilla_ender_dragon_phase_is_landing_or_takeoff, DragonFlightHistorySample,
    EnderDragonAnimationState, EnderDragonClientSoundTriggers,
};
use super::renderer_catalog::vanilla_entity_renderer_uses_abstract_horse_renderer;
use super::schema::{
    vanilla_ender_dragon_phase_is_sitting,
    vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_boat_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_horse_accessors_for_entity_type,
    vanilla_entity_metadata_allay_dancing_accessor_for_entity_type,
    vanilla_entity_metadata_armadillo_state_accessor_for_entity_type,
    vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type,
    vanilla_entity_metadata_bat_flags_accessor_for_entity_type,
    vanilla_entity_metadata_bee_accessors_for_entity_type,
    vanilla_entity_metadata_camel_dash_accessor_for_entity_type,
    vanilla_entity_metadata_cat_lying_accessor_for_entity_type,
    vanilla_entity_metadata_cat_relax_state_one_accessor_for_entity_type,
    vanilla_entity_metadata_copper_golem_accessors_for_entity_type,
    vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type,
    vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type,
    vanilla_entity_metadata_creeper_accessors_for_entity_type,
    vanilla_entity_metadata_eating_sound_policy,
    vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type,
    vanilla_entity_metadata_fox_accessors_for_entity_type,
    vanilla_entity_metadata_guardian_accessors_for_entity_type,
    vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type,
    vanilla_entity_metadata_mob_accessors_for_entity_type,
    vanilla_entity_metadata_panda_accessors_for_entity_type,
    vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type,
    vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type,
    vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type,
    vanilla_entity_metadata_shulker_accessors_for_entity_type,
    vanilla_entity_metadata_sniffer_state_accessor_for_entity_type,
    vanilla_entity_metadata_type_is_camel, vanilla_entity_metadata_type_is_guardian,
    vanilla_entity_metadata_type_is_hoglin_base,
    vanilla_entity_metadata_type_is_player_rideable_jumping, vanilla_entity_metadata_type_is_slime,
    vanilla_entity_metadata_vehicle_accessors_for_entity_type,
    vanilla_entity_metadata_warden_anger_level_accessor_for_entity_type,
    vanilla_entity_metadata_wither_side_head_target_accessors_for_entity_type,
    vanilla_entity_metadata_wolf_interested_accessor_for_entity_type,
    VanillaEntityEatingSoundPolicy, VanillaVehicleMetadataAccessors,
    VANILLA_ARMADILLO_STATE_ROLLING_ID as ARMADILLO_STATE_ROLLING_ID,
    VANILLA_ARMADILLO_STATE_SCARED_ID as ARMADILLO_STATE_SCARED_ID,
    VANILLA_ARMADILLO_STATE_UNROLLING_ID as ARMADILLO_STATE_UNROLLING_ID,
    VANILLA_COPPER_GOLEM_STATE_DROPPING_ITEM_ID as COPPER_GOLEM_STATE_DROPPING_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID as COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_GETTING_ITEM_ID as COPPER_GOLEM_STATE_GETTING_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID as COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_IDLE_ID as COPPER_GOLEM_STATE_IDLE_ID,
    VANILLA_SNIFFER_STATE_DIGGING_ID as SNIFFER_STATE_DIGGING_ID,
    VANILLA_SNIFFER_STATE_FEELING_HAPPY_ID as SNIFFER_STATE_FEELING_HAPPY_ID,
    VANILLA_SNIFFER_STATE_RISING_ID as SNIFFER_STATE_RISING_ID,
    VANILLA_SNIFFER_STATE_SCENTING_ID as SNIFFER_STATE_SCENTING_ID,
    VANILLA_SNIFFER_STATE_SEARCHING_ID as SNIFFER_STATE_SEARCHING_ID,
    VANILLA_SNIFFER_STATE_SNIFFING_ID as SNIFFER_STATE_SNIFFING_ID,
};
use super::EnderDragonAnimationSource;
use super::{
    is_vanilla_abstract_nautilus_type, is_vanilla_boat_type, is_vanilla_vehicle_entity_type,
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_living_entity_accessors_for_entity_type, vanilla_entity_type_is_squid,
    EntityCameraViewBobState, EntityTransform, EntityVec3, VanillaEntityEventKind,
    VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID, VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
};
use bbb_protocol::entity_types::*;

/// Vanilla `AbstractBoat.tick` advances each active `paddlePositions[side]` by
/// `PI / 8` per client tick, otherwise resetting it to zero.
const BOAT_PADDLE_ADVANCE_PER_TICK: f32 = std::f32::consts::PI / 8.0;
const EATING_VOICE_RANDOM_SALT: u64 = 0x6561_7469_6e67_5f76;
/// Vanilla `EntityType.ARROW` / `EntityType.SPECTRAL_ARROW`: both extend
/// `AbstractArrow` and share the same client-side `shakeTime` impact wobble.
/// Vanilla `AbstractArrow.IN_GROUND`, the synced boolean after `ID_FLAGS` (`8`) and
/// `PIERCE_LEVEL` (`9`). `onSyncedDataUpdated(IN_GROUND)` starts `shakeTime = 7`
/// when the arrow is no longer on its first tick and the current shake has settled.
const ARROW_SHAKE_TICKS: i32 = 7;
/// Vanilla `ArmorStand.WOBBLE_TIME`.
const ARMOR_STAND_WOBBLE_TIME: i32 = 5;
// Vanilla `EntityType.ELDER_GUARDIAN` / `EntityType.GUARDIAN` both share
// `GuardianModel` and the same client `Guardian.aiStep` tail accumulator (the
// elder is the guardian mesh scaled 2.35×, with no animation override). Their
// generated metadata aliases follow `Entity`/`LivingEntity`/`Mob` at ids 16/17.
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
const PANDA_AMOUNT_RISE_PER_TICK: f32 = 0.15;
const PANDA_AMOUNT_FALL_PER_TICK: f32 = 0.19;
const PANDA_ROLL_COUNTER_MAX: i32 = 32;
/// Vanilla `Armadillo.ArmadilloState.SCARED.animationDuration()`; the first SCARED
/// setup tick starts `peekAnimationState` and `fastForward`s it by this many ticks.
const ARMADILLO_STATE_SCARED_ANIMATION_TICKS: i32 = 50;
const POLAR_BEAR_STAND_ANIMATION_TICKS: f32 = 6.0;
/// Vanilla `Creeper.DATA_SWELL_DIR` (synced fuse direction, default `-1`) and
/// `DATA_IS_IGNITED`; the client advances `swell` toward `maxSwell` while the
/// effective direction is positive.
/// Vanilla `Creeper.DEFAULT_MAX_SWELL`. `getSwelling` divides the lerped swell
/// by `maxSwell - 2`.
const CREEPER_MAX_SWELL: i32 = 30;
const SHULKER_PEEK_PER_TICK: f32 = 0.05;
const SHULKER_MAX_PEEK_AMOUNT: f32 = 1.0;
/// Vanilla `Cat.updateLieDownAmount` / `updateRelaxStateOneAmount` easing.
const CAT_LIE_DOWN_AMOUNT_RISE_PER_TICK: f32 = 0.15;
const CAT_LIE_DOWN_AMOUNT_FALL_PER_TICK: f32 = 0.22;
const CAT_LIE_DOWN_TAIL_AMOUNT_RISE_PER_TICK: f32 = 0.08;
const CAT_LIE_DOWN_TAIL_AMOUNT_FALL_PER_TICK: f32 = 0.13;
const CAT_RELAX_STATE_ONE_AMOUNT_RISE_PER_TICK: f32 = 0.1;
const CAT_RELAX_STATE_ONE_AMOUNT_FALL_PER_TICK: f32 = 0.13;
/// Vanilla wolf shake/drying progress increment (`Wolf.tick`: `shakeAnim += 0.05F`).
const WOLF_SHAKE_ANIM_PER_TICK: f32 = 0.05;
/// Vanilla wolf wet shade floor (`Wolf.getWetShade`: `0.75 + lerp(shake) / 2 * 0.25`).
const WOLF_WET_SHADE_BASE: f32 = 0.75;
/// Vanilla wolf wet shade shake multiplier (`lerp(shakeAnimO, shakeAnim) / 2 * 0.25`).
const WOLF_WET_SHADE_SHAKE_SCALE: f32 = 0.125;
/// Vanilla wolf drying completes once `shakeAnimO >= 2.0F`; the shade reaches white at `shakeAnim = 2`.
const WOLF_SHAKE_ANIM_DONE: f32 = 2.0;
/// Vanilla `Wolf.tick` interested-angle easing toward the synced `DATA_INTERESTED_ID` target.
const WOLF_INTERESTED_EASE: f32 = 0.4;
/// Vanilla `Wolf.getHeadRollAngle(partialTick)`: `lerp(interestedAngleO, interestedAngle) * 0.15 * π`.
const WOLF_HEAD_ROLL_SCALE: f32 = 0.15;
/// Vanilla `LivingEntity.updateSwimAmount`: the easing step per client tick.
const LIVING_SWIM_AMOUNT_PER_TICK: f32 = 0.09;
/// Vanilla `LivingEntity.updateSwimAmount`: the fully visually-swimming upper bound.
const LIVING_SWIM_AMOUNT_MAX: f32 = 1.0;
const ELYTRA_DEFAULT_X_ROT: f32 = std::f32::consts::PI / 12.0;
const ELYTRA_DEFAULT_Y_ROT: f32 = 0.0;
const ELYTRA_DEFAULT_Z_ROT: f32 = -std::f32::consts::PI / 12.0;
const ELYTRA_FALL_FLYING_X_ROT: f32 = std::f32::consts::PI / 9.0;
const ELYTRA_FALL_FLYING_Z_ROT: f32 = -std::f32::consts::PI / 2.0;
const ELYTRA_CROUCHING_X_ROT: f32 = std::f32::consts::PI * 2.0 / 9.0;
const ELYTRA_CROUCHING_Y_ROT: f32 = 0.087_266_46;
const ELYTRA_CROUCHING_Z_ROT: f32 = -std::f32::consts::PI / 4.0;
const ELYTRA_ROT_EASE: f32 = 0.3;
const PLAYER_CLOAK_TELEPORT_THRESHOLD: f64 = 10.0;
const PLAYER_CLOAK_EASE: f64 = 0.25;
const PLAYER_CLOAK_WALK_DISTANCE_SCALE: f32 = 0.6;
/// Vanilla `Rabbit.handleEntityEvent`: event `1` (`spawnSprintParticle`) also seeds the client-side
/// hop reconstruction — `jumpDuration = 15; jumpTicks = 0;` — so the `hopAnimationState` plays for
/// one 15-tick (0.75s, exactly one loop of `RabbitAnimation.HOP`) jump arc.
/// Vanilla `Rabbit.startJumping` / `handleEntityEvent(1)`: `this.jumpDuration = 15`.
const RABBIT_JUMP_DURATION: i32 = 15;
/// Vanilla `Axolotl`'s four animators are each `BinaryAnimator(10, IN_OUT_SINE)`, so every factor
/// eases between 0 and 1 over ten ticks.
const AXOLOTL_ANIMATOR_LENGTH: i32 = 10;
/// Vanilla `Pillager.IS_CHARGING_CROSSBOW`, the synced `EntityDataSerializers.BOOLEAN` accessor at id 17
/// (Entity 0–7, LivingEntity 8–14, Mob 15, `Raider.IS_CELEBRATING` 16, then `Pillager`'s first own
/// accessor). While set, the pillager is drawing its crossbow, and the client reconstructs
/// `getTicksUsingItem` by counting ticks since the flag rose (the using-item flag rises with it).
/// Vanilla `Piglin.DATA_IS_CHARGING_CROSSBOW` synched-data id (18, the regular piglin's own slot — past
/// `DATA_BABY_ID`, `DATA_IS_IMMUNE_TO_ZOMBIFICATION`). Drives the same crossbow-draw counter as the
/// pillager (the flag is a different id on a different class, so read it under a piglin type gate).
/// Vanilla `Fox.interestedAngle` per-tick ease toward the `FLAG_INTERESTED`
/// target: `interestedAngle += (target - interestedAngle) * 0.4`.
const FOX_INTERESTED_EASE: f32 = 0.4;
/// Vanilla `Fox.getHeadRollAngle` scale: `lerp(pt, interestedAngleO,
/// interestedAngle) * 0.11 * π`.
const FOX_HEAD_ROLL_SCALE: f32 = 0.11;
/// Vanilla `Fox.crouchAmount` per-tick climb while `FLAG_CROUCHING` is set
/// (`crouchAmount += 0.2`).
const FOX_CROUCH_PER_TICK: f32 = 0.2;
/// Vanilla `Fox.MAX_CROUCH_AMOUNT`: the crouch accumulator saturates here.
const FOX_MAX_CROUCH_AMOUNT: f32 = 5.0;
/// Vanilla `LivingEntity.hurtDuration`: the hurt animation (and red damage
/// overlay) runs for 10 client ticks after a hurt animation or damage event.
const HURT_ANIMATION_DURATION: i32 = 10;
/// Vanilla `LivingEntity.onKineticHit`: ignore repeated hit feedback until more
/// than ten game ticks have elapsed since the last accepted hit.
const KINETIC_HIT_FEEDBACK_RETRIGGER_TICKS: i32 = 10;
/// Vanilla `SwingAnimation.DEFAULT.duration()`: the empty hand and common-item
/// WHACK swing ramps `attackAnim` from `0` to `1` over six client ticks.
/// Per-item default durations and dig-speed / mining-fatigue modifiers are recorded on each
/// triggered swing and refreshed when runtime state changes.
pub(crate) const ATTACK_SWING_DURATION: i32 = 6;

const fn default_attack_swing_duration() -> i32 {
    ATTACK_SWING_DURATION
}
/// Vanilla `LivingEntity.tickDeath` removes the entity at `deathTime >= 20`, and
/// the death tip-over flip is fully clamped by then, so the client counter is
/// capped here to stay bounded when no server removal arrives.
const DEATH_ANIMATION_MAX_TICKS: i32 = 20;
/// Vanilla `EnderDragon.tickDeath`: `dragonDeathTime++`, with renderer fade/rays normalized over
/// 200 ticks and server removal at `>= 200`.
const ENDER_DRAGON_DEATH_ANIMATION_MAX_TICKS: i32 = 200;
/// Vanilla `Sheep.EAT_ANIMATION_TICKS`: the eat-grass animation runs for 40
/// client ticks after entity event `10`.
const SHEEP_EAT_ANIMATION_TICKS: i32 = 40;
/// Vanilla `Warden`: the client decrements `tendrilAnimation` from `10` toward
/// `0` each tick; `getTendrilAnimation` divides the lerped value by `10`.
const WARDEN_TENDRIL_ANIMATION_TICKS: i32 = 10;
/// Vanilla `Warden`: the client resets `heartAnimation` to `10` on each heartbeat
/// and decrements it toward `0` each tick; `getHeartAnimation` divides the lerped
/// value by `10`. Shares the `tendrilAnimation` range, hence the same `10` cap.
const WARDEN_HEART_ANIMATION_TICKS: i32 = 10;
/// Vanilla `Warden.CLIENT_ANGER_LEVEL` (`getClientAngerLevel()`); membership, wire index and typed
/// default come from the generated Warden accessor.
/// Vanilla `AngerLevel.ANGRY.getMinimumAnger()`: the anger at which the heartbeat
/// reaches its fastest, used to normalise `getHeartBeatDelay`.
const WARDEN_ANGRY_MINIMUM_ANGER: i32 = 80;
/// The "not in a triggered pose" sentinel for [`WardenCombatAnimationState::prev_pose`]:
/// no synced `DATA_POSE` has been observed yet, so the first ROARING/SNIFFING pose is a
/// fresh transition that starts the matching timer.
const WARDEN_POSE_UNSET: i32 = -1;
/// Vanilla `Goat.handleEntityEvent`: event `58` sets `isLoweringHead = true` (the goat begins a ram),
/// event `59` clears it. The client `aiStep` then advances `lowerHeadTick` toward / away from the cap.
/// Vanilla `Goat.aiStep` clamps `lowerHeadTick` to `[0, 20]`: `++` while lowering, `-= 2` otherwise.
/// `getRammingXHeadRot` normalises it by this cap.
const GOAT_LOWER_HEAD_MAX_TICKS: i32 = 20;
/// Vanilla `IronGolem.handleEntityEvent`: event `4` sets `attackAnimationTick = 10` (the two-fisted
/// smash), event `11` sets `offerFlowerTick = 400` (offering a poppy to a villager), event `34` clears
/// it. Both counters decrement each client `aiStep` toward `0`.
const IRON_GOLEM_ATTACK_TICKS: i32 = 10;
const IRON_GOLEM_OFFER_FLOWER_TICKS: i32 = 400;
/// Vanilla `Ravager.handleEntityEvent`: event `4` sets `attackTick = 10` (the bite), event `39` sets
/// `stunnedTick = 40` (a shield-block stun). The roar is not event-driven — when `stunnedTick` decays to
/// `0` the client `aiStep` sets `roarTick = 20`, so a roar always follows a stun.
const RAVAGER_ATTACK_TICKS: i32 = 10;
const RAVAGER_STUN_TICKS: i32 = 40;
const RAVAGER_ROAR_TICKS: i32 = 20;
const RAVAGER_STUN_PARTICLE_RANDOM_SALT: i64 = 0x5A17;
/// Client-local RNG salt for `Creaking.checkEyeBlink`. Vanilla seeds each entity's
/// `RandomSource` from process-local entropy, so the exact cursor is not present on the wire; bbb
/// keeps the same Java LCG and interval rules behind a stable per-entity seed.
const CREAKING_EYE_FLICKER_RANDOM_SALT: i64 = 0x4352_4541_4B;
/// Client-local RNG salt for `Blaze.aiStep`. Vanilla does not synchronize the
/// entity `RandomSource`; the cursor still belongs to the canonical entity and
/// is cloned into immutable publications so retained generations cannot advance.
const BLAZE_CLIENT_RANDOM_SALT: u64 = 0x626c_617a_655f_7469;
const ENDER_AMBIENT_RANDOM_SALT: u64 = 0x656e_6465_725f_7469;
const GUARDIAN_CLIENT_RANDOM_SALT: u64 = 0x6775_6172_6469_616e;
const CAMEL_VOICE_RANDOM_SALT: u64 = 0x6361_6d65_6c5f_766f;
/// Vanilla `Hoglin`/`Zoglin.handleEntityEvent`: event `4` sets `attackAnimationRemainingTicks = 10` (the
/// headbutt), decremented each client tick. `HoglinModel.setupAnim` drives the head-down ram from it.
const HOGLIN_ATTACK_TICKS: i32 = 10;

/// Vanilla `EvokerFangs.handleEntityEvent`: event `4` sets `clientSideAttackStarted = true`, after which
/// `EvokerFangs.tick` decrements `lifeTicks` (field initializer `22`) each client tick. The
/// `getAnimationProgress` ramp built from `lifeTicks` drives `EvokerFangsModel.setupAnim`.
const EVOKER_FANGS_LIFE_TICKS: i32 = 22;
const EVOKER_FANGS_CRIT_PARTICLE_TICK: i32 = 14;
const EVOKER_FANGS_CRIT_PARTICLE_COUNT: usize = 12;
const EVOKER_FANGS_PARTICLE_RANDOM_SALT: i64 = 0xF46;
/// Vanilla `Camel.DASH_COOLDOWN_TICKS`: `onSyncedDataUpdated(DASH)` seeds `dashCooldown` to 55 when
/// the synced dash flag rises, and `Camel.tick` decrements it each client tick. The renderer consumes
/// `max(dashCooldown - partialTicks, 0)` as `CamelRenderState.jumpCooldown`.
const CAMEL_DASH_COOLDOWN_TICKS: i32 = 55;
/// Vanilla `AgeableMob.AGE_LOCK_COOLDOWN_TICKS`, also assigned to
/// `ageLockParticleTimer` by `setAgeLockedData`.
const AGE_LOCK_COOLDOWN_TICKS: u8 = 40;
/// Vanilla `AbstractNautilus.DASH_COOLDOWN_TICKS`. Unlike Camel, metadata updates seed this
/// cooldown from `onSyncedDataUpdated(DASH)` even when the synchronized value is `false`.
const NAUTILUS_DASH_COOLDOWN_TICKS: i32 = 40;
const NAUTILUS_DASH_MINIMUM_DURATION_TICKS: i32 = 5;
const NAUTILUS_BUBBLE_PROBABILITY_MIN: f64 = 0.15;
const NAUTILUS_BUBBLE_PROBABILITY_MAX: f64 = 1.0;
const NAUTILUS_BUBBLE_PROBABILITY_MULTIPLIER: f64 = 2.0;
const NAUTILUS_BUBBLE_SPREAD_FACTOR: f64 = 0.8;
const NAUTILUS_CLIENT_RANDOM_SALT: u64 = 0x6e61_7574_696c_7573;
/// Client-local `Slime.random` cursor for the landing particle ring followed by
/// the two-float squish pitch roll.
const SLIME_CLIENT_RANDOM_SALT: u64 = 0x736c_696d_655f_726e;
/// Shared client-local `EnderDragon.random` cursor for flap, ambient growl,
/// sitting-attack growl, and death-particle draws in Java tick order.
const ENDER_DRAGON_CLIENT_RANDOM_SALT: u64 = 0x6472_6167_6f6e_5f72;
/// Shared client-local `LightningBolt.random` cursor. Construction consumes
/// render seed then flash count; the first tick consumes thunder and impact
/// pitch in that order.
const LIGHTNING_BOLT_CLIENT_RANDOM_SALT: u64 = 0x6c69_6768_746e_696e;
/// Client-local Warden RNG cursor for `LivingEntity.getVoicePitch()` on each
/// heartbeat. The server does not synchronize the entity RNG seed.
const WARDEN_CLIENT_RANDOM_SALT: u64 = 0x7761_7264_656e_5f72;
/// Vanilla `Camel.setupAnimationStates`: `idleAnimationTimeout = random.nextInt(40) + 80`, then
/// `idleAnimationState.start(tickCount)`. `CAMEL_IDLE` itself is the 4.0 s non-looping keyframe.
const CAMEL_IDLE_TIMEOUT_RANDOM_BOUND: i32 = 40;
const CAMEL_IDLE_TIMEOUT_BASE_TICKS: i32 = 80;
/// Vanilla `CopperGolem.setupAnimationStates`: `idleAnimationStartTick = tickCount +
/// random.nextInt(200, 240)`, then `idleAnimationState.start(tickCount)` at that tick.
const COPPER_GOLEM_IDLE_TIMEOUT_RANDOM_BOUND: i32 = 40;
const COPPER_GOLEM_IDLE_TIMEOUT_BASE_TICKS: i32 = 200;

/// Vanilla `Creaking.handleEntityEvent`: event `4` sets `attackAnimationRemainingTicks = 15` (the
/// lunge), event `66` sets `invulnerabilityAnimationRemainingTicks = 8` (the heart-bound stagger).
/// Both decrement each client tick in `aiStep`, and `setupAnimationStates` `animateWhen`s the
/// matching keyframe one-shot on `ticks > 0`.
const CREAKING_ATTACK_DURATION: i32 = 15;
const CREAKING_INVULNERABLE_DURATION: i32 = 8;
/// Vanilla `MinecartTNT.handleEntityEvent(10)` calls `primeFuse`, which sets the client-side
/// `fuse` counter to 80. `TntMinecartRenderer.extractRenderState` projects it as
/// `fuse - partialTicks + 1.0`.
const TNT_MINECART_FUSE_TICKS: i32 = 80;

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct EntityClientTickParticleTriggers {
    /// Vanilla `MinecartTNT.tick` decrements a positive fuse and then emits one
    /// smoke particle at `y + 0.5` on both client and server.
    pub(crate) minecart_tnt_smoke: bool,
    /// Vanilla `MinecartFurnace.tick` emits one large-smoke particle when the
    /// synced fuel bit is set and the entity RNG's `nextInt(4)` returns zero.
    pub(crate) minecart_furnace_large_smoke: bool,
    /// Vanilla `Blaze.aiStep` emits two large-smoke particles every client tick.
    pub(crate) blaze_large_smoke: Option<[BlazeSmokeParticleOffset; 2]>,
    /// The optional burn sound is ordered before the two smoke particles.
    pub(crate) blaze_burn_sound: Option<BlazeBurnSoundParameters>,
    /// Vanilla `Warden.tick` emits this on the same edge that resets the heart
    /// animation, after its silent gate and two-float voice-pitch draw.
    pub(crate) warden_heartbeat_pitch: Option<f32>,
    /// Ordered `EnderDragon.aiStep` sound edges. Flap always precedes the
    /// mutually-exclusive ambient or sitting-attack growl.
    pub(crate) ender_dragon_sounds: EnderDragonClientSoundTriggers,
    /// Vanilla `LightningBolt.tick` emits thunder then impact on its first
    /// client tick (`life == 2` before decrement).
    pub(crate) lightning_bolt_sounds: Option<LightningBoltSoundParameters>,
    /// Vanilla Enderman/Endermite `aiStep`: two portal particles per tick.
    pub(crate) ender_portal: Option<[EnderAmbientParticleOffset; 2]>,
    /// Vanilla moving, submerged Guardian: two bubbles behind its view vector.
    pub(crate) guardian_moving_bubbles: Option<[GuardianAmbientParticleOffset; 2]>,
    /// Vanilla dry Guardian upward flop sound after a remembered supporting face.
    pub(crate) guardian_flop_sound: bool,
    /// Vanilla dry, grounded Guardian random impulse applied after particles.
    pub(crate) guardian_dry_jump: Option<GuardianDryJumpUpdate>,
    /// Vanilla active-target LookControl result, applied after ambient bubbles.
    pub(crate) guardian_look_update: Option<GuardianLookUpdate>,
    /// Vanilla active-target beam bubbles in exact entity RNG order.
    pub(crate) guardian_attack_bubbles: Option<Vec<EntityVec3>>,
    /// Vanilla `Slime.tick` landing particles precede its squish sound.
    pub(crate) slime_landing_particles: Option<SlimeLandingParticleTrigger>,
    pub(crate) slime_squish_sound: Option<SlimeSquishSoundParameters>,
    pub(crate) ravager_stun: Option<RavagerStunParticleOffset>,
    pub(crate) evoker_fangs_crit:
        Option<[EvokerFangsCritParticleOffset; EVOKER_FANGS_CRIT_PARTICLE_COUNT]>,
    /// Vanilla `AbstractBoat.tick` tests the pre-increment paddle position for
    /// the `PI / 4` sound crossing, then advances the corresponding paddle.
    pub(crate) boat_paddle_sounds: [bool; 2],
    /// Vanilla `Camel.tick` plays the neutral dash-ready sound on the exact
    /// client tick where the retained cooldown reaches zero.
    pub(crate) camel_dash_ready_sound: bool,
    /// Vanilla `AbstractNautilus.tick` clears DASH before decrementing its cooldown.
    pub(crate) nautilus_clear_dash: bool,
    /// The ready sound precedes the optional bubble in the subclass tick.
    pub(crate) nautilus_dash_ready_sound: Option<NautilusDashReadySoundParameters>,
    pub(crate) nautilus_bubble: Option<EntityVec3>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NautilusDashReadySoundParameters {
    pub(crate) underwater: bool,
    pub(crate) pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BlazeSmokeParticleOffset {
    pub(crate) x_unit: f64,
    pub(crate) y_unit: f64,
    pub(crate) z_unit: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BlazeBurnSoundParameters {
    pub(crate) volume: f32,
    pub(crate) pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LightningBoltSoundParameters {
    pub(crate) thunder_pitch: f32,
    pub(crate) impact_pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EnderAmbientParticleOffset {
    pub(crate) x_unit: f64,
    pub(crate) y_unit: f64,
    pub(crate) z_unit: f64,
    pub(crate) velocity: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GuardianAmbientParticleOffset {
    pub(crate) x_unit: f64,
    pub(crate) y_unit: f64,
    pub(crate) z_unit: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GuardianDryJumpUpdate {
    pub(crate) x_impulse: f64,
    pub(crate) z_impulse: f64,
    pub(crate) y_rot: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GuardianLookUpdate {
    pub(crate) x_rot: f32,
    pub(crate) y_head_rot: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GuardianAttackTargetTickInput {
    pub(crate) position: EntityVec3,
    pub(crate) eye_height: f32,
    pub(crate) half_height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SlimeLandingParticleTrigger {
    pub(crate) kind: super::SlimeLandingParticleKind,
    pub(crate) offsets: Vec<EntityVec3>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SlimeSquishSoundParameters {
    pub(crate) magma_cube: bool,
    pub(crate) tiny: bool,
    pub(crate) volume: f32,
    pub(crate) pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RavagerStunParticleOffset {
    pub(crate) jitter_x: f64,
    pub(crate) jitter_z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EvokerFangsCritParticleOffset {
    pub(crate) x_unit: f64,
    pub(crate) y_offset: f64,
    pub(crate) z_unit: f64,
    pub(crate) velocity: EntityVec3,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityClientAnimationState {
    #[serde(default)]
    pub age_ticks: u32,
    /// Vanilla `ItemEntity.bobOffs`, initialized from `random.nextFloat() * TAU`
    /// in the constructor and retained for the entity lifetime. The protocol does
    /// not synchronize the RNG, so bbb derives an equivalent stable client-local
    /// phase from the entity UUID at each fresh construction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_bob_offset: Option<f32>,
    /// Vanilla `EndCrystal.time` constructor offset from `random.nextInt(100000)`.
    /// `age_ticks` remains the base `Entity.tickCount`; renderer projections add
    /// this offset only for the end-crystal `ageInTicks` override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_crystal_time_offset: Option<u32>,
    /// Vanilla `LivingEntity.recreateFromPacket` seeds both body-yaw endpoints
    /// from the packet head yaw. `Shulker.recreateFromPacket` immediately
    /// overrides both to zero. This value is consumed only before the first
    /// client tick; later projection preserves the existing movement-owned
    /// body-yaw approximation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawn_body_y_rot: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boat_damage: Option<BoatDamageAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boat_bubble: Option<BoatBubbleAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boat: Option<BoatPaddleAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrow_shake: Option<ArrowShakeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polar_bear_standing: Option<PolarBearStandingAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shulker_peek: Option<ShulkerPeekAnimationState>,
    #[serde(skip)]
    pub(crate) shulker_teleport: Option<ShulkerTeleportAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bee_roll: Option<BeeRollAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panda: Option<PandaAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fox: Option<FoxAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wolf_wet: Option<WolfWetAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wolf_head_roll: Option<WolfHeadRollAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cat_lie_down: Option<CatLieDownAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wither_heads: Option<WitherHeadRotationsState>,
    /// Vanilla `Bat.flyAnimationState`, started on the first flying client tick and stopped while
    /// resting so the renderer samples `BAT_FLYING` relative to that transition rather than the
    /// entity's absolute age.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bat_flying: Option<KeyframeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frog_croak: Option<KeyframeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frog_tongue: Option<KeyframeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frog_jump: Option<KeyframeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frog_swim_idle: Option<KeyframeAnimationState>,
    /// Vanilla `Camel.dashAnimationState`, started on the synced `DASH` boolean rising edge
    /// (`animateWhen(isDashing(), tickCount)`) and projected as the elapsed seconds the renderer feeds
    /// to the looping `CAMEL_DASH` gallop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camel_dash: Option<KeyframeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camel_idle: Option<CamelIdleAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camel_dash_cooldown: Option<CamelDashCooldownState>,
    /// Client-only `AbstractNautilus.dashCooldown`. It is canonical mutable tick state but not part
    /// of the stable probe/renderer shape; immutable publications still clone it for isolation.
    #[serde(skip)]
    pub(crate) nautilus_dash_cooldown: Option<NautilusDashCooldownState>,
    /// Vanilla `AgeableMob.ageLockParticleTimer`. A local golden-dandelion interaction starts this
    /// at 40 ticks; while nonzero it also prevents another local age-lock interaction.
    #[serde(skip)]
    pub(crate) age_lock_particle_timer_ticks: u8,
    /// Vanilla `ItemBasedSteering` client-only `boosting`/`boostTime` owner for
    /// Pig and Strider. The synchronized total remains in canonical metadata.
    #[serde(skip)]
    pub(crate) item_based_steering: Option<ItemBasedSteeringClientState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copper_golem_idle: Option<CopperGolemIdleAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copper_golem_get_item: Option<KeyframeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copper_golem_get_no_item: Option<KeyframeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copper_golem_drop_item: Option<KeyframeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copper_golem_drop_no_item: Option<KeyframeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sniffer: Option<SnifferAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armadillo: Option<ArmadilloAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ender_dragon: Option<EnderDragonAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheep_eat: Option<SheepEatAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goat_ramming: Option<GoatRammingAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iron_golem: Option<IronGolemAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ravager: Option<RavagerAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hoglin: Option<HoglinAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rabbit_hop: Option<RabbitHopAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creaking: Option<CreakingAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hurt: Option<HurtAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_rotation_lerp: Option<HeadRotationLerpState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinetic_hit_feedback: Option<KineticHitFeedbackAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armor_stand_wiggle: Option<ArmorStandWiggleAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack_swing: Option<AttackSwingAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death: Option<DeathAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub living_swim: Option<LivingSwimAmountState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elytra: Option<ElytraAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_cloak: Option<PlayerCloakAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creeper_swell: Option<CreeperSwellAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warden_tendril: Option<WardenTendrilAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warden_heart: Option<WardenHeartAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warden_combat: Option<WardenCombatAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub squid: Option<SquidAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chicken_flap: Option<ChickenFlapAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slime: Option<SlimeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evoker_fangs: Option<EvokerFangsAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allay_dance: Option<AllayDanceAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allay_holding_item: Option<AllayHoldingItemAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crossbow_charge: Option<CrossbowChargeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axolotl: Option<AxolotlAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parrot_flap: Option<ParrotFlapAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equine_tail: Option<EquineTailAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equine_pose: Option<EquinePoseAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guardian_tail: Option<GuardianTailAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guardian_spikes: Option<GuardianSpikesAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guardian_attack: Option<GuardianAttackAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breeze: Option<BreezeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walk_animation: Option<WalkAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minecart_tnt_fuse: Option<MinecartTntFuseAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minecart_spawner: Option<MinecartSpawnerClientState>,
    /// Client-local `MinecartFurnace.random` cursor used only by its fueled
    /// large-smoke branch. It is cloned into immutable publications but stays
    /// out of the stable JSON/probe shape, like other non-wire runtime cursors.
    #[serde(skip)]
    pub(crate) minecart_furnace_smoke_random_seed: Option<u64>,
    /// Client-local `Blaze.random` cursor for its ambient burn sound and smoke
    /// positions. Cloned for publication isolation, omitted from stable probes.
    #[serde(skip)]
    pub(crate) blaze_client_random_seed: Option<u64>,
    /// Shared client-local entity RNG owner for the Enderman and Endermite
    /// portal-particle family. This is one cursor per canonical entity.
    #[serde(skip)]
    pub(crate) ender_ambient_random_seed: Option<u64>,
    /// Shared client-local Guardian RNG cursor. It owns the constructor tail
    /// phase, dry spike flicker, ambient bubbles, and dry jump in Java order.
    #[serde(skip)]
    pub(crate) guardian_client_random_seed: Option<u64>,
    /// Client-local Warden RNG cursor used by heartbeat voice pitch. Cloned for
    /// generation isolation and omitted from the stable probe shape.
    #[serde(skip)]
    pub(crate) warden_client_random_seed: Option<u64>,
    /// Shared client-local Ender Dragon RNG cursor. It preserves the sound and
    /// death-particle draw sequence without exposing a fabricated wire seed.
    #[serde(skip)]
    pub(crate) ender_dragon_client_random_seed: Option<u64>,
    /// Shared client-local LightningBolt RNG cursor after constructor seed and
    /// flash-count draws.
    #[serde(skip)]
    pub(crate) lightning_bolt_client_random_seed: Option<u64>,
    /// Vanilla `LightningBolt.seed`, projected to the renderer from the same
    /// constructor RNG owner used by first-tick audio.
    #[serde(skip)]
    pub(crate) lightning_bolt_render_seed: Option<i64>,
    /// Client-local Camel voice-pitch cursor. Vanilla does not synchronize the
    /// entity RandomSource; the retained cursor preserves the two-float pitch
    /// distribution and old-generation isolation without a parallel owner.
    #[serde(skip)]
    pub(crate) camel_voice_random_seed: Option<u64>,
    /// Shared client-local `AbstractNautilus.random` cursor for ready-sound pitch and bubbles in
    /// exact subclass order. The server does not synchronize its initial seed.
    #[serde(skip)]
    pub(crate) nautilus_client_random_seed: Option<u64>,
    /// Shared client-local Slime/MagmaCube RNG cursor for landing particles and
    /// squish pitch in exact `Slime.tick` order.
    #[serde(skip)]
    pub(crate) slime_client_random_seed: Option<u64>,
    /// Client-local entity RNG cursor for `Animal.playEatingSound` overrides
    /// that evaluate `LivingEntity.getVoicePitch`. Nautilus shares its broader
    /// cursor above; other supported eat paths retain this cursor for future
    /// ambient/step/eat consolidation.
    #[serde(skip)]
    pub(crate) eating_voice_random_seed: Option<u64>,
    /// Client-owned fields consulted by repeated local mob interactions.
    /// Vanilla does not synchronize these fields: tameable sitting intent,
    /// a brown mooshroom's pending stew ingredient, and Tadpole's private age.
    #[serde(skip)]
    pub(crate) ordered_to_sit: bool,
    #[serde(skip)]
    pub(crate) mooshroom_stew_ingredient_item_id: Option<i32>,
    #[serde(skip)]
    pub(crate) tadpole_age_ticks: i32,
    /// Shared deterministic stand-in for the entity `RandomSource` calls made
    /// by local interaction methods. It is retained per incarnation and kept
    /// out of stable probe serialization like the other client RNG cursors.
    #[serde(skip)]
    pub(crate) local_interaction_random_seed: Option<u64>,
    /// Client-local `PlayerRideableJumping` input retained between the local
    /// player's release edge and the controlled vehicle's ridden tick. It is
    /// cloned into immutable publications for old-generation isolation but is
    /// omitted from the stable renderer/probe serialization shape.
    #[serde(skip)]
    pub(crate) ridden_jump: Option<RiddenJumpState>,
}

/// Vanilla `AbstractHorse.playerJumpPendingScale`, `allowStandSliding`, and
/// the local `standCounter` started by `onPlayerJump`. This belongs to the
/// canonical entity client-tick owner; it is not a parallel input cache.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct RiddenJumpState {
    pub(crate) player_jump_pending_scale: f32,
    pub(crate) allow_stand_sliding: bool,
    pub(crate) local_stand_ticks_remaining: u8,
}

/// The client-observable `BaseSpawner` state owned by `MinecartSpawner`. Vanilla constructs it
/// with delay `20` / minimum delay `200`; delegated entity event `1` resets delay to the minimum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinecartSpawnerClientState {
    pub spawn_delay: i32,
    pub min_spawn_delay: i32,
}

/// Vanilla `VehicleEntity` hurt-roll state as consumed by `AbstractBoatRenderer` and
/// `AbstractMinecartRenderer`: `hurtTime`, `hurtDir`, and `damage` are synced data values that the
/// client decrements every tick before rendering.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoatDamageAnimationState {
    pub hurt_time: i32,
    pub hurt_dir: i32,
    pub damage: f32,
}

impl BoatDamageAnimationState {
    fn from_metadata(
        accessors: VanillaVehicleMetadataAccessors,
        data_values: &[EntityDataValue],
    ) -> Self {
        Self {
            hurt_time: entity_data_int(
                data_values,
                accessors.hurt_time_data_id,
                accessors.hurt_time_default,
            ),
            hurt_dir: entity_data_int(
                data_values,
                accessors.hurt_dir_data_id,
                accessors.hurt_dir_default,
            ),
            damage: entity_data_float(data_values, accessors.damage_data_id)
                .unwrap_or(accessors.damage_default),
        }
    }

    fn advance_client_tick(&mut self) {
        if self.hurt_time > 0 {
            self.hurt_time -= 1;
        }
        if self.damage > 0.0 {
            self.damage = (self.damage - 1.0).max(0.0);
        }
    }

    fn animate_hurt(&mut self) {
        self.hurt_dir = -self.hurt_dir;
        self.hurt_time = HURT_ANIMATION_DURATION;
        self.damage *= 11.0;
    }

    fn render_state(self, partial_tick: f32) -> (f32, i32, f32) {
        (
            self.hurt_time as f32 - partial_tick,
            self.hurt_dir,
            (self.damage - partial_tick).max(0.0),
        )
    }

    fn is_settled(self) -> bool {
        self.hurt_time <= 0 && self.damage <= 0.0
    }
}

impl Default for BoatDamageAnimationState {
    fn default() -> Self {
        let accessors = vanilla_entity_metadata_vehicle_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        )
        .expect("generated oak-boat VehicleEntity metadata accessors");
        Self::from_metadata(accessors, &[])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MinecartTntFuseAnimationState {
    pub fuse_remaining_ticks: i32,
}

impl MinecartTntFuseAnimationState {
    fn advance_client_tick(&mut self) {
        if self.fuse_remaining_ticks > 0 {
            self.fuse_remaining_ticks -= 1;
        }
    }

    fn fuse_remaining_in_ticks(&self, partial_tick: f32) -> f32 {
        self.fuse_remaining_ticks as f32 - partial_tick + 1.0
    }
}

/// Vanilla `AbstractBoat` bubble-column wobble state: the client keeps a
/// multiplier and previous/current angles, increasing the multiplier while
/// synced `bubbleTime > 0` and easing it back to zero otherwise.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BoatBubbleAnimationState {
    pub multiplier: f32,
    pub previous_angle: f32,
    pub current_angle: f32,
}

impl BoatBubbleAnimationState {
    fn advance_client_tick(&mut self, age_ticks: u32, bubble_time: i32) {
        if bubble_time > 0 {
            self.multiplier += 0.05;
        } else {
            self.multiplier -= 0.1;
        }
        self.multiplier = self.multiplier.clamp(0.0, 1.0);
        self.previous_angle = self.current_angle;
        self.current_angle = 10.0 * (0.5 * age_ticks as f32).sin() * self.multiplier;
    }

    fn angle(self, partial_tick: f32) -> f32 {
        self.previous_angle + partial_tick * (self.current_angle - self.previous_angle)
    }

    fn is_settled(self, bubble_time: i32) -> bool {
        bubble_time <= 0
            && self.multiplier == 0.0
            && self.previous_angle == 0.0
            && self.current_angle == 0.0
    }
}

/// Vanilla `AbstractBoat.paddlePositions`: client-side rowing accumulators for
/// the left and right paddles. `AbstractBoatRenderer.extractRenderState` projects
/// these through `getRowingTime(side, partialTick)` before `AbstractBoatModel`
/// turns them into paddle rotations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BoatPaddleAnimationState {
    pub left: f32,
    pub right: f32,
}

impl BoatPaddleAnimationState {
    fn advance_client_tick(&mut self, left_active: bool, right_active: bool) -> [bool; 2] {
        let sounds = [
            boat_paddle_sound_crossed(self.left, left_active),
            boat_paddle_sound_crossed(self.right, right_active),
        ];
        self.left = advance_boat_paddle(self.left, left_active);
        self.right = advance_boat_paddle(self.right, right_active);
        sounds
    }

    fn rowing_times(self, partial_tick: f32, left_active: bool, right_active: bool) -> [f32; 2] {
        [
            boat_rowing_time(self.left, partial_tick, left_active),
            boat_rowing_time(self.right, partial_tick, right_active),
        ]
    }

    fn is_settled(self) -> bool {
        self.left == 0.0 && self.right == 0.0
    }
}

fn boat_paddle_sound_crossed(position: f32, active: bool) -> bool {
    active
        && position % std::f32::consts::TAU <= std::f32::consts::FRAC_PI_4
        && (position + BOAT_PADDLE_ADVANCE_PER_TICK) % std::f32::consts::TAU
            >= std::f32::consts::FRAC_PI_4
}

fn advance_boat_paddle(position: f32, active: bool) -> f32 {
    if active {
        position + BOAT_PADDLE_ADVANCE_PER_TICK
    } else {
        0.0
    }
}

fn boat_rowing_time(position: f32, partial_tick: f32, active: bool) -> f32 {
    if active {
        let start = position - BOAT_PADDLE_ADVANCE_PER_TICK;
        start + partial_tick.clamp(0.0, 1.0) * (position - start)
    } else {
        0.0
    }
}

/// Canonical client-side arrow impact wobble, mirroring vanilla
/// `AbstractArrow.shakeTime`. A metadata update of `IN_GROUND` to `true` sets it
/// to 7 once the entity has ticked at least once; each client tick decrements it,
/// and `ArrowRenderer.extractRenderState` projects `shakeTime - partialTick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrowShakeAnimationState {
    pub shake_time: i32,
}

impl ArrowShakeAnimationState {
    fn started() -> Self {
        Self {
            shake_time: ARROW_SHAKE_TICKS,
        }
    }

    fn advance_client_tick(&mut self) {
        self.shake_time = (self.shake_time - 1).max(0);
    }

    fn shake(self, partial_tick: f32) -> f32 {
        self.shake_time as f32 - partial_tick
    }

    fn is_settled(self) -> bool {
        self.shake_time <= 0
    }
}

/// Desired per-tick target for one vanilla wither side head. When the synced
/// target entity is visible, both pitch and yaw lerp toward it; when it is absent
/// or unknown, vanilla only lerps yaw back toward `yBodyRot` and leaves pitch as-is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct WitherHeadTargetRotations {
    pub(crate) x_rot: Option<f32>,
    pub(crate) y_rot: f32,
}

impl WitherHeadTargetRotations {
    pub(crate) fn fallback_to_body(y_body_rot: f32) -> Self {
        Self {
            x_rot: None,
            y_rot: y_body_rot,
        }
    }
}

/// Canonical client-side wither side-head rotations, mirroring vanilla
/// `WitherBoss.xRotHeads` / `yRotHeads`. The renderer copies these current arrays
/// directly into `WitherRenderState`; 26.1 does not partial-lerp the side heads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct WitherHeadRotationsState {
    pub x_head_rots: [f32; 2],
    pub y_head_rots: [f32; 2],
}

impl WitherHeadRotationsState {
    fn advance_client_tick(&mut self, targets: [WitherHeadTargetRotations; 2]) {
        for (index, target) in targets.into_iter().enumerate() {
            if let Some(x_rot) = target.x_rot {
                self.x_head_rots[index] = rotlerp(self.x_head_rots[index], x_rot, 40.0);
            }
            self.y_head_rots[index] = rotlerp(self.y_head_rots[index], target.y_rot, 10.0);
        }
    }
}

/// Canonical client-side creeper fuse animation, mirroring vanilla
/// `Creeper.swell`/`oldSwell`. The synced `DATA_SWELL_DIR` (forced to `1` while
/// ignited) advances `current_swell` toward `maxSwell` each client tick;
/// `getSwelling` lerps it for the renderer white swelling overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreeperSwellAnimationState {
    /// Effective fuse direction (`DATA_SWELL_DIR`, or `1` while ignited).
    pub swell_dir: i32,
    /// Vanilla `Creeper.oldSwell`.
    pub previous_swell: i32,
    /// Vanilla `Creeper.swell`.
    pub current_swell: i32,
}

/// Canonical client-side hurt animation countdown, mirroring vanilla
/// `LivingEntity.hurtTime`. A hurt animation or damage event resets it to
/// [`HURT_ANIMATION_DURATION`]; each client tick decrements it toward `0`. While
/// it is positive the renderer projects the red damage overlay
/// (`LivingEntityRenderState.hasRedOverlay`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HurtAnimationState {
    pub hurt_time: i32,
}

/// Vanilla `LivingEntity.lerpYHeadRot` plus `lerpHeadSteps`, armed by
/// `ClientboundRotateHeadPacket` and advanced once per client tick.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HeadRotationLerpState {
    pub target_y_head_rot: f32,
    pub steps_remaining: i32,
}

/// Canonical client-side kinetic hit feedback timer, mirroring vanilla
/// `LivingEntity.lastKineticHitFeedbackTime`. Entity event `2` calls
/// `onKineticHit`; the renderer reads
/// `getTicksSinceLastKineticHitFeedback(partialTicks)` to kick spear items
/// backward for the vanilla ten-tick feedback curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KineticHitFeedbackAnimationState {
    pub elapsed_ticks: i32,
}

/// Canonical armor-stand hit wobble timer. Vanilla stores the hit game time in
/// `ArmorStand.lastHit`; `ArmorStandRenderer.extractRenderState` exposes
/// `wiggle = level.getGameTime() - lastHit + partialTicks`, and
/// `setupRotations` applies it only while `< ArmorStand.WOBBLE_TIME`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmorStandWiggleAnimationState {
    pub elapsed_ticks: i32,
}

/// Canonical client-side melee-swing animation, mirroring vanilla `LivingEntity`'s
/// `swingTime`/`attackAnim`/`oAttackAnim`. The `ClientboundAnimate` packet (action `0`
/// main hand / `3` off hand) calls `swing()` ([`EntityClientAnimationState::trigger_swing`]),
/// which arms the current hand item's `getSwingAnimation().duration()` ramp (defaulting
/// to [`ATTACK_SWING_DURATION`]); `updateSwingTime` advances it each client tick and the
/// renderer projects `getAttackAnim(partialTick)` into `HumanoidModel.setupAttackAnimation`'s
/// body twist + arm swing. `oAttackAnim` is kept for the partial-tick lerp.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AttackSwingAnimationState {
    /// Vanilla `LivingEntity.swinging`: a swing is currently playing.
    pub swinging: bool,
    /// Vanilla `LivingEntity.swingTime`: the integer tick counter, `-1` for the tick
    /// just after `swing()` before the first `updateSwingTime`.
    pub swing_time: i32,
    /// Vanilla `LivingEntity.getCurrentSwingDuration()` for this swing. The value is
    /// refreshed when the active hand item or relevant mob effects change, matching
    /// the runtime duration `updateSwingTime` observes.
    #[serde(default = "default_attack_swing_duration")]
    pub duration: i32,
    /// Vanilla `LivingEntity.attackAnim`: `swingTime / duration`, the current-tick value.
    pub attack_anim: f32,
    /// Vanilla `LivingEntity.oAttackAnim`: the previous-tick `attackAnim`, the lerp start.
    pub prev_attack_anim: f32,
    /// Whether the swing is the off hand (left arm); vanilla `swingingArm`.
    pub off_hand: bool,
}

/// Canonical client-side death animation counter, mirroring vanilla
/// `LivingEntity.deathTime`. It begins when the synced health (`DATA_HEALTH_ID`)
/// reaches `<= 0` (`LivingEntity.isDeadOrDying`) and increments each client tick
/// (`LivingEntity.tickDeath`). While it is positive the renderer tips the entity
/// over (`LivingEntityRenderer.setupRotations`) and projects the red damage
/// overlay (`hasRedOverlay`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeathAnimationState {
    pub death_time: i32,
}

/// Canonical client-side `LivingEntity` swim-amount accumulator, mirroring vanilla
/// `swimAmountO` / `swimAmount`. Each client tick saves the previous value, then eases
/// toward `1.0` while `isVisuallySwimming()` and toward `0.0` otherwise by `0.09`.
/// The drowned consumes this today for `DrownedModel.setupAnim` and
/// `DrownedRenderer.setupRotations`; the state shape is generic so player/humanoid
/// swim poses can reuse it later.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LivingSwimAmountState {
    pub visually_swimming: bool,
    pub previous_swim_amount: f32,
    pub current_swim_amount: f32,
}

impl LivingSwimAmountState {
    fn set_visually_swimming(&mut self, visually_swimming: bool) {
        self.visually_swimming = visually_swimming;
    }

    fn advance_client_tick(&mut self) {
        self.previous_swim_amount = self.current_swim_amount;
        if self.visually_swimming {
            self.current_swim_amount = (self.current_swim_amount + LIVING_SWIM_AMOUNT_PER_TICK)
                .min(LIVING_SWIM_AMOUNT_MAX);
        } else {
            self.current_swim_amount =
                (self.current_swim_amount - LIVING_SWIM_AMOUNT_PER_TICK).max(0.0);
        }
    }

    fn swim_amount(self, partial_tick: f32) -> f32 {
        self.previous_swim_amount
            + partial_tick * (self.current_swim_amount - self.previous_swim_amount)
    }

    fn is_settled(self) -> bool {
        !self.visually_swimming
            && self.previous_swim_amount == 0.0
            && self.current_swim_amount == 0.0
    }
}

/// Canonical client-side elytra wing rotation accumulator, mirroring vanilla
/// `LivingEntity.elytraAnimationState`'s per-tick target/ease/partial-lerp behavior.
/// `HumanoidMobRenderer.extractHumanoidRenderState` samples it into
/// `HumanoidRenderState.elytraRotX/Y/Z`, and `WingsLayer` consumes those values
/// when a WINGS chest equipment layer is present.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ElytraAnimationState {
    pub rot_x: f32,
    pub rot_y: f32,
    pub rot_z: f32,
    pub rot_x_old: f32,
    pub rot_y_old: f32,
    pub rot_z_old: f32,
}

impl Default for ElytraAnimationState {
    fn default() -> Self {
        // Source rows and renderer defaults use the steady non-flying pose when no
        // ticked state exists. Start the accumulator there so native snapshots do not
        // briefly fold wings before the first client-animation tick; the tick path
        // below still follows vanilla's target/ease/partial-lerp math.
        Self {
            rot_x: ELYTRA_DEFAULT_X_ROT,
            rot_y: ELYTRA_DEFAULT_Y_ROT,
            rot_z: ELYTRA_DEFAULT_Z_ROT,
            rot_x_old: ELYTRA_DEFAULT_X_ROT,
            rot_y_old: ELYTRA_DEFAULT_Y_ROT,
            rot_z_old: ELYTRA_DEFAULT_Z_ROT,
        }
    }
}

impl ElytraAnimationState {
    fn advance_client_tick(
        &mut self,
        is_fall_flying: bool,
        is_crouching: bool,
        delta_movement: EntityVec3,
    ) {
        self.rot_x_old = self.rot_x;
        self.rot_y_old = self.rot_y;
        self.rot_z_old = self.rot_z;

        let (target_x_rot, target_y_rot, target_z_rot) = if is_fall_flying {
            let ratio = elytra_fall_flying_ratio(delta_movement);
            (
                lerp_f32(ratio, ELYTRA_DEFAULT_X_ROT, ELYTRA_FALL_FLYING_X_ROT),
                ELYTRA_DEFAULT_Y_ROT,
                lerp_f32(ratio, ELYTRA_DEFAULT_Z_ROT, ELYTRA_FALL_FLYING_Z_ROT),
            )
        } else if is_crouching {
            (
                ELYTRA_CROUCHING_X_ROT,
                ELYTRA_CROUCHING_Y_ROT,
                ELYTRA_CROUCHING_Z_ROT,
            )
        } else {
            (
                ELYTRA_DEFAULT_X_ROT,
                ELYTRA_DEFAULT_Y_ROT,
                ELYTRA_DEFAULT_Z_ROT,
            )
        };

        self.rot_x += (target_x_rot - self.rot_x) * ELYTRA_ROT_EASE;
        self.rot_y += (target_y_rot - self.rot_y) * ELYTRA_ROT_EASE;
        self.rot_z += (target_z_rot - self.rot_z) * ELYTRA_ROT_EASE;
    }

    fn rot_x(self, partial_tick: f32) -> f32 {
        lerp_f32(partial_tick, self.rot_x_old, self.rot_x)
    }

    fn rot_y(self, partial_tick: f32) -> f32 {
        lerp_f32(partial_tick, self.rot_y_old, self.rot_y)
    }

    fn rot_z(self, partial_tick: f32) -> f32 {
        lerp_f32(partial_tick, self.rot_z_old, self.rot_z)
    }
}

fn elytra_fall_flying_ratio(delta_movement: EntityVec3) -> f32 {
    if delta_movement.y >= 0.0 {
        return 1.0;
    }
    let length = (delta_movement.x * delta_movement.x
        + delta_movement.y * delta_movement.y
        + delta_movement.z * delta_movement.z)
        .sqrt();
    if length < 1.0e-5 {
        return 1.0;
    }
    let normalized_y = (delta_movement.y / length) as f32;
    1.0 - (-normalized_y).powf(1.5)
}

fn lerp_f32(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}

fn lerp_f64(delta: f32, start: f64, end: f64) -> f64 {
    start + f64::from(delta) * (end - start)
}

fn lerp_vec3(delta: f32, start: EntityVec3, end: EntityVec3) -> EntityVec3 {
    EntityVec3 {
        x: lerp_f64(delta, start.x, end.x),
        y: lerp_f64(delta, start.y, end.y),
        z: lerp_f64(delta, start.z, end.z),
    }
}

fn horizontal_distance(delta: EntityVec3) -> f32 {
    (delta.x * delta.x + delta.z * delta.z).sqrt() as f32
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerCloakAnimationState {
    pub initialized: bool,
    pub entity_position_old: EntityVec3,
    pub entity_position: EntityVec3,
    pub cloak_position_old: EntityVec3,
    pub cloak_position: EntityVec3,
    pub walk_dist_old: f32,
    pub walk_dist: f32,
    pub bob_old: f32,
    pub bob: f32,
    pub fall_flying_ticks: u32,
    /// Vanilla `ClientAvatarState.deltaMovementOnPreviousTick`. Despite the
    /// name, `ClientAvatarState.tick` replaces it with the current movement once
    /// per client tick; render extraction lerps this snapshot toward the live
    /// entity movement for the partial frame.
    #[serde(default)]
    pub delta_movement_on_previous_tick: EntityVec3,
}

impl Default for PlayerCloakAnimationState {
    fn default() -> Self {
        Self {
            initialized: false,
            entity_position_old: EntityVec3::default(),
            entity_position: EntityVec3::default(),
            cloak_position_old: EntityVec3::default(),
            cloak_position: EntityVec3::default(),
            walk_dist_old: 0.0,
            walk_dist: 0.0,
            bob_old: 0.0,
            bob: 0.0,
            fall_flying_ticks: 0,
            delta_movement_on_previous_tick: EntityVec3::default(),
        }
    }
}

impl PlayerCloakAnimationState {
    /// Vanilla `ClientAvatarState.tick` / `moveCloak`, plus the local-player
    /// walk-distance and bob inputs that `AvatarRenderer.extractCapeState`
    /// consumes. bbb drives the walk pulse from the synced feet movement for all
    /// players because the local-only `LocalPlayer.addWalkedDistance` path is not
    /// split out at this render-state boundary.
    pub(super) fn advance_client_tick(
        &mut self,
        position: EntityVec3,
        delta_movement: EntityVec3,
        on_ground: bool,
        is_alive: bool,
        is_swimming: bool,
        is_fall_flying: bool,
    ) {
        if !self.initialized {
            self.initialized = true;
            self.entity_position_old = position;
            self.entity_position = position;
            self.cloak_position_old = position;
            self.cloak_position = position;
        }

        self.delta_movement_on_previous_tick = delta_movement;

        let previous_position = self.entity_position;
        self.entity_position_old = previous_position;
        self.entity_position = position;
        self.walk_dist_old = self.walk_dist;

        let travel = EntityVec3 {
            x: position.x - previous_position.x,
            y: position.y - previous_position.y,
            z: position.z - previous_position.z,
        };
        self.walk_dist += horizontal_distance(travel) * PLAYER_CLOAK_WALK_DISTANCE_SCALE;

        self.move_cloak(position);

        self.bob_old = self.bob;
        let target_bob = if on_ground && is_alive && !is_swimming {
            horizontal_distance(delta_movement).min(0.1)
        } else {
            0.0
        };
        self.bob += (target_bob - self.bob) * 0.4;

        self.fall_flying_ticks = if is_fall_flying {
            self.fall_flying_ticks.saturating_add(1)
        } else {
            0
        };
    }

    fn move_cloak(&mut self, position: EntityVec3) {
        let (x_old, x) = move_cloak_axis(self.cloak_position.x, position.x);
        let (y_old, y) = move_cloak_axis(self.cloak_position.y, position.y);
        let (z_old, z) = move_cloak_axis(self.cloak_position.z, position.z);
        self.cloak_position_old = EntityVec3 {
            x: x_old,
            y: y_old,
            z: z_old,
        };
        self.cloak_position = EntityVec3 { x, y, z };
    }

    fn cape_state(self, partial_tick: f32, y_body_rot: f32) -> (f32, f32, f32) {
        let cloak = lerp_vec3(partial_tick, self.cloak_position_old, self.cloak_position);
        let entity = lerp_vec3(partial_tick, self.entity_position_old, self.entity_position);
        let delta_x = cloak.x - entity.x;
        let delta_y = cloak.y - entity.y;
        let delta_z = cloak.z - entity.z;
        let y_body_rot_rad = y_body_rot.to_radians();
        let forward_x = f64::from(y_body_rot_rad.sin());
        let forward_z = f64::from(-y_body_rot_rad.cos());

        let mut flap = (delta_y as f32 * 10.0).clamp(-6.0, 32.0);
        let fall_flying_scale =
            ((self.fall_flying_ticks as f32 + partial_tick).powi(2) / 100.0).clamp(0.0, 1.0);
        let lean = ((delta_x * forward_x + delta_z * forward_z) as f32
            * 100.0
            * (1.0 - fall_flying_scale))
            .clamp(0.0, 150.0);
        let lean2 = ((delta_x * forward_z - delta_z * forward_x) as f32 * 100.0).clamp(-20.0, 20.0);
        let bob = lerp_f32(partial_tick, self.bob_old, self.bob);
        let walk_dist = lerp_f32(partial_tick, self.walk_dist_old, self.walk_dist);
        flap += (walk_dist * 6.0).sin() * 32.0 * bob;

        (flap, lean, lean2)
    }

    fn camera_view_bob(self, partial_tick: f32) -> EntityCameraViewBobState {
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        EntityCameraViewBobState {
            backwards_interpolated_walk_distance: -(self.walk_dist
                + (self.walk_dist - self.walk_dist_old) * partial_tick),
            bob: lerp_f32(partial_tick, self.bob_old, self.bob),
        }
    }
}

fn move_cloak_axis(current_cloak: f64, position: f64) -> (f64, f64) {
    let delta = position - current_cloak;
    if !(-PLAYER_CLOAK_TELEPORT_THRESHOLD..=PLAYER_CLOAK_TELEPORT_THRESHOLD).contains(&delta) {
        (position, position)
    } else {
        (current_cloak, current_cloak + delta * PLAYER_CLOAK_EASE)
    }
}

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
    fn animate_when(&mut self, condition: bool, age_ticks: u32) {
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
    fn elapsed_seconds(self, age_ticks: u32, partial_tick: f32) -> Option<f32> {
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
    random_seed: u64,
    timeout_ticks: i32,
    animation: KeyframeAnimationState,
}

impl CamelIdleAnimationState {
    fn new(entity_id: i32) -> Self {
        Self {
            random_seed: java_random_seed(i64::from(entity_id).wrapping_add(0xCAFE)),
            timeout_ticks: 0,
            animation: KeyframeAnimationState { start_age: None },
        }
    }

    fn advance_client_tick(&mut self, age_ticks: u32) {
        if self.timeout_ticks <= 0 {
            self.timeout_ticks =
                java_random_next_int_bound(&mut self.random_seed, CAMEL_IDLE_TIMEOUT_RANDOM_BOUND)
                    + CAMEL_IDLE_TIMEOUT_BASE_TICKS;
            self.animation.start_age = Some(age_ticks);
        } else {
            self.timeout_ticks -= 1;
        }
    }

    fn elapsed_seconds(self, age_ticks: u32, partial_tick: f32) -> Option<f32> {
        self.animation.elapsed_seconds(age_ticks, partial_tick)
    }
}

/// Vanilla `CopperGolem.idleAnimationStartTick` plus `idleAnimationState`. The exact
/// `RandomSource` seed is client-local, so bbb uses the same deterministic Java LCG shape used by
/// other client-only idle timers while preserving vanilla's `random.nextInt(200, 240)` interval, the
/// delayed first start, and the reset 10 ticks after the head-spin sound point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopperGolemIdleAnimationState {
    random_seed: u64,
    idle_animation_start_tick: i32,
    animation: KeyframeAnimationState,
}

impl CopperGolemIdleAnimationState {
    fn new(entity_id: i32) -> Self {
        Self {
            random_seed: java_random_seed(i64::from(entity_id).wrapping_add(0xC0_6E)),
            idle_animation_start_tick: 0,
            animation: KeyframeAnimationState { start_age: None },
        }
    }

    fn advance_client_tick(&mut self, age_ticks: u32, is_idle: bool) {
        if !is_idle {
            self.idle_animation_start_tick = 0;
            self.animation.start_age = None;
            return;
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
        }
    }

    fn elapsed_seconds(self, age_ticks: u32, partial_tick: f32) -> Option<f32> {
        self.animation.elapsed_seconds(age_ticks, partial_tick)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CamelDashCooldownState {
    cooldown_ticks: i32,
    was_dashing: bool,
}

impl CamelDashCooldownState {
    fn advance_client_tick(&mut self, is_dashing: bool) -> bool {
        if is_dashing && !self.was_dashing && self.cooldown_ticks == 0 {
            self.cooldown_ticks = CAMEL_DASH_COOLDOWN_TICKS;
        }
        self.was_dashing = is_dashing;
        let previous = self.cooldown_ticks;
        if self.cooldown_ticks > 0 {
            self.cooldown_ticks -= 1;
        }
        previous == 1
    }

    fn start_local_dash_after_tick(&mut self) {
        // The native input owner runs after the common entity tick. Seed the
        // end-of-Camel.tick value (`55` set by executeRidersJump, then `--`).
        self.cooldown_ticks = CAMEL_DASH_COOLDOWN_TICKS - 1;
        self.was_dashing = true;
    }

    fn stop_local_dash(&mut self) {
        self.was_dashing = false;
    }

    pub(crate) fn cooldown_ticks(self) -> i32 {
        self.cooldown_ticks
    }

    fn should_clear_before_tick(
        self,
        is_dashing: bool,
        on_ground: bool,
        in_liquid: bool,
        is_passenger: bool,
    ) -> bool {
        is_dashing && self.cooldown_ticks < 50 && (on_ground || in_liquid || is_passenger)
    }

    fn jump_cooldown(self, partial_tick: f32) -> f32 {
        (self.cooldown_ticks as f32 - partial_tick).max(0.0)
    }

    fn is_settled(self) -> bool {
        self.cooldown_ticks == 0 && !self.was_dashing
    }
}

/// Vanilla `AbstractNautilus.dashCooldown`, retained on the canonical entity so a released local
/// dash, a synchronized remote dash, and immutable generations all observe one owner.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NautilusDashCooldownState {
    cooldown_ticks: i32,
}

/// Vanilla `ItemBasedSteering` keeps the elapsed boost cursor outside
/// synchronized entity data. It is retained with the canonical entity so
/// publication A cannot observe generation B's ridden tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ItemBasedSteeringClientState {
    boosting: bool,
    boost_time: i32,
}

impl ItemBasedSteeringClientState {
    fn on_synced(&mut self) {
        self.boosting = true;
        self.boost_time = 0;
    }

    fn tick_and_factor(&mut self, boost_time_total: i32) -> f32 {
        if self.boosting {
            let previous = self.boost_time;
            self.boost_time = self.boost_time.wrapping_add(1);
            if previous > boost_time_total {
                self.boosting = false;
            }
        }
        if self.boosting {
            let phase = self.boost_time as f32 / boost_time_total as f32 * std::f32::consts::PI;
            1.0 + 1.15 * super::client_movement::vanilla_mth_sin(f64::from(phase))
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
    fn seed_from_synced_update(&mut self) {
        if self.cooldown_ticks == 0 {
            self.cooldown_ticks = NAUTILUS_DASH_COOLDOWN_TICKS;
        }
    }

    fn start_local_dash(&mut self) {
        self.cooldown_ticks = NAUTILUS_DASH_COOLDOWN_TICKS;
    }

    fn advance_subclass_tick(&mut self, is_dashing: bool) -> (bool, bool) {
        let clear_dash = is_dashing
            && self.cooldown_ticks
                < NAUTILUS_DASH_COOLDOWN_TICKS - NAUTILUS_DASH_MINIMUM_DURATION_TICKS;
        let previous = self.cooldown_ticks;
        if self.cooldown_ticks > 0 {
            self.cooldown_ticks -= 1;
        }
        (clear_dash, previous == 1)
    }

    pub(crate) fn cooldown_ticks(self) -> i32 {
        self.cooldown_ticks
    }

    fn cooldown(self, partial_tick: f32) -> f32 {
        (self.cooldown_ticks as f32 - partial_tick).max(0.0)
    }

    fn is_settled(self) -> bool {
        self.cooldown_ticks == 0
    }
}

/// Returns the `Sniffer.State` ordinal whose one-shot `AnimationState` the renderer drives, or
/// `None` for a state with no triggered keyframe (`IDLING` rests at the bind pose, `SEARCHING` runs
/// the looping search-walk handled by the walk path, and an unknown ordinal is treated as idle).
fn sniffer_animated_state(state_id: i32) -> Option<i32> {
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
    fn set_state(&mut self, state_id: i32, age_ticks: u32) {
        if state_id == self.state_id {
            return;
        }
        self.state_id = state_id;
        self.keyframe.start_age = sniffer_animated_state(state_id).map(|_| age_ticks);
    }

    /// Vanilla `Sniffer.isSearching()` (`getState() == SEARCHING`): whether the synced `DATA_STATE`
    /// is `SEARCHING`, which `SnifferModel.setupAnim` uses to swap the base walk for the looping
    /// `SNIFFER_SNIFF_SEARCH` search-walk.
    fn is_searching(self) -> bool {
        self.state_id == SNIFFER_STATE_SEARCHING_ID
    }

    /// The active state's elapsed seconds for the renderer (`-1.0` when no triggered animation is
    /// running), paired with the projected state id.
    fn animation(self, age_ticks: u32, partial_tick: f32) -> (i32, f32) {
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
fn armadillo_should_hide_in_shell(state_id: i32, in_state_ticks: u32) -> bool {
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
    fn new(state_id: i32, age_ticks: u32) -> Self {
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
    fn set_state(&mut self, state_id: i32, age_ticks: u32) {
        if state_id == self.state_id {
            return;
        }
        self.state_id = state_id;
        self.state_change_age = age_ticks;
        self.setup_animation_states(age_ticks);
    }

    fn in_state_ticks(self, age_ticks: u32) -> u32 {
        age_ticks.saturating_sub(self.state_change_age)
    }

    fn setup_animation_states(&mut self, age_ticks: u32) {
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
    fn is_hiding_in_shell(self, age_ticks: u32) -> bool {
        armadillo_should_hide_in_shell(self.state_id, self.in_state_ticks(age_ticks))
    }

    fn peek_elapsed_seconds(self, age_ticks: u32, partial_tick: f32) -> Option<f32> {
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
    fn start_shaking(&mut self) {
        self.is_shaking = true;
        self.previous_shake_anim = 0.0;
        self.current_shake_anim = 0.0;
    }

    fn cancel_shake(&mut self) {
        self.is_shaking = false;
        self.previous_shake_anim = 0.0;
        self.current_shake_anim = 0.0;
    }

    fn advance_client_tick(&mut self, in_water: bool) {
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

    fn wet_shade(self, partial_tick: f32) -> f32 {
        if !self.is_wet {
            return 1.0;
        }
        let shake_anim = self.previous_shake_anim
            + partial_tick * (self.current_shake_anim - self.previous_shake_anim);
        (WOLF_WET_SHADE_BASE + shake_anim * WOLF_WET_SHADE_SHAKE_SCALE).min(1.0)
    }

    fn shake_anim(self, partial_tick: f32) -> f32 {
        self.previous_shake_anim
            + partial_tick * (self.current_shake_anim - self.previous_shake_anim)
    }

    fn is_settled(self) -> bool {
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
    fn advance_client_tick(&mut self, interested: bool) {
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

    fn head_roll_angle(self, partial_tick: f32) -> f32 {
        lerp_f32(
            partial_tick,
            self.previous_interested_angle,
            self.current_interested_angle,
        ) * WOLF_HEAD_ROLL_SCALE
            * std::f32::consts::PI
    }

    fn is_settled(self) -> bool {
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
    fn set_flags(&mut self, is_lying: bool, relax_state_one: bool) {
        self.is_lying = is_lying;
        self.relax_state_one = relax_state_one;
    }

    fn advance_client_tick(&mut self, is_lying: bool, relax_state_one: bool) {
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

    fn lie_down_amount(self, partial_tick: f32) -> f32 {
        lerp_f32(
            partial_tick,
            self.previous_lie_down_amount,
            self.current_lie_down_amount,
        )
    }

    fn lie_down_amount_tail(self, partial_tick: f32) -> f32 {
        lerp_f32(
            partial_tick,
            self.previous_lie_down_amount_tail,
            self.current_lie_down_amount_tail,
        )
    }

    fn relax_state_one_amount(self, partial_tick: f32) -> f32 {
        lerp_f32(
            partial_tick,
            self.previous_relax_state_one_amount,
            self.current_relax_state_one_amount,
        )
    }

    fn is_settled(self) -> bool {
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
    fn advance_client_tick(&mut self) {
        if self.lowering_head {
            self.lower_head_tick += 1;
        } else {
            self.lower_head_tick -= 2;
        }
        self.lower_head_tick = self.lower_head_tick.clamp(0, GOAT_LOWER_HEAD_MAX_TICKS);
    }

    /// The ram has fully relaxed: not lowering and the counter has decayed back to `0`, so the state
    /// can be dropped (a resting goat projects no head tilt).
    fn is_settled(&self) -> bool {
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
    fn advance_client_tick(&mut self) {
        if self.attack_animation_tick > 0 {
            self.attack_animation_tick -= 1;
        }
        if self.offer_flower_tick > 0 {
            self.offer_flower_tick -= 1;
        }
    }

    /// Both timers have run out, so the golem is back to its walk/idle and the state can be dropped.
    fn is_settled(&self) -> bool {
        self.attack_animation_tick == 0 && self.offer_flower_tick == 0
    }

    /// Vanilla `IronGolemRenderer.extractRenderState`: `attackTicksRemaining = getAttackAnimationTick()
    /// > 0 ? tick - partialTicks : 0`. The partial-lerped attack timer drives the smash arm wave.
    fn attack_ticks_remaining(self, partial_tick: f32) -> f32 {
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
    client_particle_random_seed: u64,
}

impl RavagerAnimationState {
    fn new(entity_id: i32) -> Self {
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
    fn advance_client_tick(&mut self, entity_id: i32) -> Option<RavagerStunParticleOffset> {
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
    fn is_settled(&self) -> bool {
        self.attack_tick == 0 && self.stunned_tick == 0 && self.roar_tick == 0
    }

    /// Vanilla `RavagerRenderer.extractRenderState`: `stunnedTicksRemaining = getStunnedTick() > 0 ?
    /// tick - partialTicks : 0`.
    fn stunned_ticks_remaining(self, partial_tick: f32) -> f32 {
        if self.stunned_tick > 0 {
            self.stunned_tick as f32 - partial_tick
        } else {
            0.0
        }
    }

    /// Vanilla `RavagerRenderer.extractRenderState`: `attackTicksRemaining = getAttackTick() > 0 ?
    /// tick - partialTicks : 0`.
    fn attack_ticks_remaining(self, partial_tick: f32) -> f32 {
        if self.attack_tick > 0 {
            self.attack_tick as f32 - partial_tick
        } else {
            0.0
        }
    }

    /// Vanilla `RavagerRenderer.extractRenderState`: `roarAnimation = roarTick > 0 ? (20 - roarTick +
    /// partialTicks) / 20 : 0` — a `0..1` ramp as the roar timer decays from `20` to `0`.
    fn roar_animation(self, partial_tick: f32) -> f32 {
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
    fn advance_client_tick(&mut self) {
        if self.attack_animation_tick > 0 {
            self.attack_animation_tick -= 1;
        }
    }

    /// The headbutt timer has run out, so the state can be dropped (a resting hoglin holds its head at
    /// the baked rest tilt).
    fn is_settled(&self) -> bool {
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
    fn start_jump(&mut self) {
        self.jump_duration = RABBIT_JUMP_DURATION;
        self.jump_ticks = 0;
    }

    /// One client tick in vanilla order: `Rabbit.baseTick` runs `setupAnimationStates` (the hop
    /// branch — the client-local random idle is intentionally absent — `startIfStopped`/`stop` on
    /// `jumpTicks > 0`) BEFORE `Rabbit.aiStep`
    /// advances `jumpTicks` (`++` until it meets `jumpDuration`, then both reset to `0`).
    fn advance_client_tick(&mut self, age_ticks: u32) {
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
    fn is_settled(&self) -> bool {
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
    death_time: i32,
    /// Vanilla client-local `Creaking.nextFlickerTime`.
    #[serde(default)]
    next_flicker_time: i32,
    /// Vanilla client-local `Creaking.eyesGlowing`, consumed only while tearing down.
    #[serde(default)]
    eyes_glowing: bool,
    /// Deterministic local substitute for the process-random Vanilla entity RNG seed. `None` keeps
    /// older serialized snapshots compatible and is initialized from the entity id on first tick.
    #[serde(default)]
    eye_flicker_random_seed: Option<u64>,
}

impl CreakingAnimationState {
    fn new(entity_id: i32) -> Self {
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
    fn start_attack(&mut self) {
        self.attack_ticks = CREAKING_ATTACK_DURATION;
    }

    /// Vanilla `Creaking.handleEntityEvent(66)`: `invulnerabilityAnimationRemainingTicks = 8`.
    fn start_invulnerable(&mut self) {
        self.invuln_ticks = CREAKING_INVULNERABLE_DURATION;
    }

    /// One client tick in vanilla order: `Creaking.aiStep` decrements both remaining-tick counters
    /// (toward `0`) FIRST, then `Creaking.tick`'s `setupAnimationStates` `animateWhen`s the attack on
    /// `attackTicks > 0`, the invulnerable on `invulnTicks > 0`, and the death on the synced
    /// `isTearingDown()`.
    fn advance_client_tick(&mut self, entity_id: i32, age_ticks: u32, is_tearing_down: bool) {
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
    fn is_settled(&self) -> bool {
        self.attack_ticks == 0
            && self.invuln_ticks == 0
            && self.attack.start_age.is_none()
            && self.invuln.start_age.is_none()
            && self.death.start_age.is_none()
    }

    fn eyes_glowing(self) -> bool {
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
    fn set_pose(&mut self, pose_id: i32, age_ticks: u32) {
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
    fn start_attack(&mut self, age_ticks: u32) {
        self.roar.start_age = None;
        self.attack.start_age = Some(age_ticks);
    }

    /// Vanilla `Warden.handleEntityEvent(62)`: `sonicBoomAnimationState.start(tickCount)`.
    fn start_sonic_boom(&mut self, age_ticks: u32) {
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
    fn advance_client_tick(&mut self, is_dancing: bool) {
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
    fn is_dancing(&self) -> bool {
        self.dancing_animation_ticks > 0.0
    }

    /// Vanilla `Allay.isSpinning()`: `(dancingAnimationTicks % 55) < 15` — the allay spins for the
    /// first 15 ticks of each 55-tick dance cycle.
    fn is_spinning(&self) -> bool {
        self.dancing_animation_ticks % 55.0 < 15.0
    }

    /// Vanilla `Allay.getSpinningProgress(partialTick)`: `lerp(partialTick, spinningTicks0,
    /// spinningTicks) / 15`, the `0..1` spin ramp.
    fn spinning_progress(&self, partial_tick: f32) -> f32 {
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
    fn advance_client_tick(&mut self, has_item_in_hand: bool) {
        self.holding_item_animation_ticks0 = self.holding_item_animation_ticks;
        if has_item_in_hand {
            self.holding_item_animation_ticks =
                (self.holding_item_animation_ticks + 1.0).clamp(0.0, 5.0);
        } else {
            self.holding_item_animation_ticks =
                (self.holding_item_animation_ticks - 1.0).clamp(0.0, 5.0);
        }
    }

    fn is_settled(&self) -> bool {
        self.holding_item_animation_ticks == 0.0 && self.holding_item_animation_ticks0 == 0.0
    }

    /// Vanilla `Allay.getHoldingItemAnimationProgress(partialTick)`.
    fn holding_item_progress(&self, partial_tick: f32) -> f32 {
        (self.holding_item_animation_ticks0
            + (self.holding_item_animation_ticks - self.holding_item_animation_ticks0)
                * partial_tick)
            / 5.0
    }
}

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
    fn advance_client_tick(&mut self, is_charging: bool) {
        self.ticks = if is_charging { self.ticks + 1.0 } else { 0.0 };
    }

    /// Whether the draw has fully stopped (counter back to `0`), so the state can be dropped.
    fn is_settled(&self) -> bool {
        self.ticks == 0.0
    }

    /// Vanilla `LivingEntity.getTicksUsingItem(partialTicks)` = `getTicksUsingItem() + partialTicks` while
    /// using; `0` once the draw stops (the counter is `0`).
    fn ticks_using_item(&self, partial_tick: f32) -> f32 {
        if self.ticks > 0.0 {
            self.ticks + partial_tick
        } else {
            0.0
        }
    }
}

/// Vanilla `Ease.inOutSine`: `-(cos(π·x) - 1) / 2`, the easing every axolotl `BinaryAnimator` uses.
fn ease_in_out_sine(x: f32) -> f32 {
    -((std::f32::consts::PI * x).cos() - 1.0) / 2.0
}

/// Vanilla `net.minecraft.util.BinaryAnimator`: a tick counter that climbs toward `length` while a
/// boolean is active and falls back toward `0` while it is not, exposing an eased `0..1` factor. The
/// `animationLength`/`easing` are supplied by the caller (the axolotl uses `10` / `IN_OUT_SINE` for
/// all four of its animators), so this state stores only the live and previous tick counts.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
struct BinaryAnimator {
    #[serde(default)]
    ticks: i32,
    #[serde(default)]
    ticks_old: i32,
}

impl BinaryAnimator {
    /// Vanilla `BinaryAnimator.tick(active)`: snapshot the old count, then climb toward `length`
    /// (active) or fall toward `0` (inactive) by one.
    fn tick(&mut self, active: bool, length: i32) {
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
    fn factor(&self, partial_tick: f32, length: i32) -> f32 {
        let raw = (self.ticks_old as f32 + (self.ticks - self.ticks_old) as f32 * partial_tick)
            / length as f32;
        ease_in_out_sine(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AxolotlBabyAnimation {
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
    playing_dead: BinaryAnimator,
    #[serde(default)]
    in_water: BinaryAnimator,
    #[serde(default)]
    on_ground: BinaryAnimator,
    #[serde(default)]
    moving: BinaryAnimator,
    #[serde(default)]
    prev_x_rot: f32,
    #[serde(default)]
    prev_y_rot: f32,
    #[serde(default)]
    has_prev_rotation: bool,
    #[serde(default)]
    baby_swim: KeyframeAnimationState,
    #[serde(default)]
    baby_walk: KeyframeAnimationState,
    #[serde(default)]
    baby_walk_underwater: KeyframeAnimationState,
    #[serde(default)]
    baby_idle_underwater: KeyframeAnimationState,
    #[serde(default)]
    baby_idle_underwater_on_ground: KeyframeAnimationState,
    #[serde(default)]
    baby_idle_on_ground: KeyframeAnimationState,
    #[serde(default)]
    baby_play_dead: KeyframeAnimationState,
}

impl AxolotlAnimationState {
    /// Advances one client tick of `Axolotl.tickAdultAnimations`: derive the mutually-exclusive
    /// animation state, then tick the four animators. `walk_is_moving` is the prior tick's
    /// `walkAnimation.isMoving()` (the walk animation is advanced after the per-type match, matching
    /// vanilla's `baseTick`-before-`aiStep` order); the rotation change OR mirrors `getXRot() !=
    /// xRotO || getYRot() != yRotO`.
    fn advance_client_tick(
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

    fn playing_dead_factor(&self, partial_tick: f32) -> f32 {
        self.playing_dead
            .factor(partial_tick, AXOLOTL_ANIMATOR_LENGTH)
    }

    fn in_water_factor(&self, partial_tick: f32) -> f32 {
        self.in_water.factor(partial_tick, AXOLOTL_ANIMATOR_LENGTH)
    }

    fn on_ground_factor(&self, partial_tick: f32) -> f32 {
        self.on_ground.factor(partial_tick, AXOLOTL_ANIMATOR_LENGTH)
    }

    fn moving_factor(&self, partial_tick: f32) -> f32 {
        self.moving.factor(partial_tick, AXOLOTL_ANIMATOR_LENGTH)
    }

    fn baby_animation_seconds(
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
    client_particle_random_seed: u64,
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
    fn with_initial_phase(phase: f32) -> Self {
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
    fn advance_client_tick(
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
    fn tail_animation(&self, partial_tick: f32) -> f32 {
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
    fn advance_client_tick(&mut self, in_water: bool, is_moving: bool, dry_random: Option<f32>) {
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
    fn spikes_animation(&self, partial_tick: f32) -> f32 {
        self.previous_spikes_animation
            + (self.spikes_animation - self.previous_spikes_animation) * partial_tick
    }
}

/// The "not in a triggered pose" sentinel for [`BreezeAnimationState::prev_pose`]: no synced
/// `DATA_POSE` has been observed yet, so the first pose is treated as a fresh transition.
const BREEZE_POSE_UNSET: i32 = -1;

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
        }
    }
}

impl BreezeAnimationState {
    /// Vanilla `Breeze.onSyncedDataUpdated(DATA_POSE)` + `Breeze.tick`, collapsed: gate each one-shot
    /// on its pose (`animate_when`, which `startIfStopped`s on the rising edge and stops on the leave
    /// — vanilla's `resetAnimations` + per-tick `startIfStopped`), and fire `slideBack` on the falling
    /// edge of `Pose.SLIDING`.
    fn set_pose(&mut self, pose_id: i32, age_ticks: u32) {
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
    fn advance_client_tick(&mut self, attack_target_id: i32, attack_duration: i32) {
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
    fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    /// Vanilla `WalkAnimationState.update`: `speedOld = speed; speed += (target -
    /// speed) * factor; position += speed; positionScale = positionScale`.
    fn update(&mut self, target_speed: f32, factor: f32, position_scale: f32) {
        self.speed_old = self.speed;
        self.speed += (target_speed - self.speed) * factor;
        self.position += self.speed;
        self.position_scale = position_scale;
    }

    /// Vanilla `WalkAnimationState.stop`: zeroes the speed and position (it leaves
    /// `positionScale` untouched, matching vanilla).
    fn stop(&mut self) {
        self.speed_old = 0.0;
        self.speed = 0.0;
        self.position = 0.0;
    }

    /// Advances one client tick from the entity's current feet position, mirroring
    /// vanilla `LivingEntity.calculateEntityAnimation` → `updateWalkAnimation` (the
    /// base mapping `targetSpeed = min(distance * 4, 1)`, `factor = 0.4`). `use_y`
    /// is `entity instanceof FlyingAnimal` (the `Mth.length` distance includes the
    /// vertical travel only for flying animals).
    fn advance_client_tick(
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
    fn position(&self, partial_ticks: f32) -> f32 {
        (self.position - self.speed * (1.0 - partial_ticks)) * self.position_scale
    }

    /// Vanilla `WalkAnimationState.speed(partialTicks)`: `min(lerp(partialTicks,
    /// speedOld, speed), 1)`.
    fn speed(&self, partial_ticks: f32) -> f32 {
        (self.speed_old + (self.speed - self.speed_old) * partial_ticks).min(1.0)
    }

    /// Vanilla `WalkAnimationState.isMoving()`: `speed > 1.0E-5`. Read by
    /// `Frog.tick` to gate the swim-idle animation off the limb-swing speed.
    fn is_moving(&self) -> bool {
        self.speed > 1.0e-5
    }
}

/// Vanilla `FlyingAnimal` test used by `calculateEntityAnimation` to decide
/// whether the limb-swing travel distance includes the vertical component.
fn is_flying_animal(entity_type_id: i32) -> bool {
    super::schema::vanilla_entity_metadata_type_is_flying_animal(entity_type_id)
}

/// Vanilla `updateWalkAnimation`'s per-entity distance→target-speed mapping, fed to
/// `WalkAnimationState.update`. The base `LivingEntity` clamps `distance · 4` to `1.0`; `Creaking`
/// overrides it to `distance · 25` clamped to `3.0` (a faster, higher-amplitude gait that ramps the
/// limb-swing position ~3× quicker). `Frog.updateWalkAnimation` uses `distance · 25` clamped to `1.0`,
/// but forces the target to `0` while `jumpAnimationState.isStarted()`. `Camel.updateWalkAnimation`
/// uses `distance · 6` clamped to `1.0` with a slower `0.2` easing factor, and targets `0` when the
/// camel is not standing or the dash animation is active.
fn walk_update_target_speed(
    entity_type_id: i32,
    distance: f32,
    frog_jump_started: bool,
    camel_walk_active: bool,
) -> (f32, f32) {
    match entity_type_id {
        entity_type_id
            if vanilla_entity_metadata_type_is_camel(entity_type_id) && camel_walk_active =>
        {
            ((distance * 6.0).min(1.0), 0.2)
        }
        entity_type_id if vanilla_entity_metadata_type_is_camel(entity_type_id) => (0.0, 0.2),
        VANILLA_ENTITY_TYPE_CREAKING_ID => ((distance * 25.0).min(3.0), 0.4),
        VANILLA_ENTITY_TYPE_FROG_ID if frog_jump_started => (0.0, 0.4),
        VANILLA_ENTITY_TYPE_FROG_ID => ((distance * 25.0).min(1.0), 0.4),
        _ => ((distance * 4.0).min(1.0), 0.4),
    }
}

/// Vanilla `WitherBoss.getAlternativeTarget(1/2)`: side heads read
/// `DATA_TARGET_B/C`. The generated accessor pair supplies membership, wire indices and defaults;
/// target `0` means "no target".
pub(crate) fn wither_side_head_target_ids(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> [i32; 2] {
    vanilla_entity_metadata_wither_side_head_target_accessors_for_entity_type(entity_type_id)
        .map(|accessors| {
            [
                entity_data_int(
                    data_values,
                    accessors.target_b_data_id,
                    accessors.target_b_default,
                ),
                entity_data_int(
                    data_values,
                    accessors.target_c_data_id,
                    accessors.target_c_default,
                ),
            ]
        })
        .unwrap_or_default()
}

/// Vanilla `WitherBoss.aiStep` target-angle math for side head `head_index`
/// (`0` = right/head B, `1` = left/head C). The returned yaw is absolute
/// `yRotHeads[index]`, later converted in the model as `yHeadRot - bodyRot`.
pub(crate) fn wither_side_head_target_rotation(
    wither_position: EntityVec3,
    y_body_rot: f32,
    scale: f32,
    head_index: usize,
    target_position: EntityVec3,
    target_eye_height: f32,
) -> WitherHeadTargetRotations {
    let vanilla_index = head_index + 1;
    let head_angle = (y_body_rot + 180.0 * (vanilla_index as f32 - 1.0)).to_radians();
    let hx = wither_position.x + f64::from(head_angle.cos() * 1.3 * scale);
    let hy = wither_position.y + f64::from(2.2 * scale);
    let hz = wither_position.z + f64::from(head_angle.sin() * 1.3 * scale);
    let xd = target_position.x - hx;
    let yd = target_position.y + f64::from(target_eye_height) - hy;
    let zd = target_position.z - hz;
    let sd = (xd * xd + zd * zd).sqrt();
    WitherHeadTargetRotations {
        x_rot: Some(-(yd.atan2(sd).to_degrees() as f32)),
        y_rot: zd.atan2(xd).to_degrees() as f32 - 90.0,
    }
}

fn rotlerp(a: f32, b: f32, max: f32) -> f32 {
    let diff = wrap_degrees(b - a).clamp(-max, max);
    a + diff
}

fn wrap_degrees(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
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
    fn standing_animation_scale(self, partial_tick: f32) -> f32 {
        let ticks = self.previous_ticks + partial_tick * (self.current_ticks - self.previous_ticks);
        ticks / POLAR_BEAR_STAND_ANIMATION_TICKS
    }

    fn set_target(&mut self, target_standing: bool) {
        self.target_standing = target_standing;
    }

    fn advance_client_tick(&mut self) {
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
    fn advance_client_tick(&mut self) {
        self.eat_animation_tick = (self.eat_animation_tick - 1).max(0);
    }
}

impl WardenTendrilAnimationState {
    /// Vanilla `Warden.tick` client branch: `tendrilAnimationO = tendrilAnimation;
    /// if (tendrilAnimation > 0) tendrilAnimation--`.
    fn advance_client_tick(&mut self) {
        self.previous = self.current;
        if self.current > 0 {
            self.current -= 1;
        }
    }

    /// Whether the tendril pulse has fully settled at rest so the state can be
    /// dropped (both the lerp endpoints are back to zero).
    fn is_settled(&self) -> bool {
        self.previous == 0 && self.current == 0
    }

    /// Vanilla `Warden.getTendrilAnimation(partialTick)`:
    /// `lerp(partialTick, tendrilAnimationO, tendrilAnimation) / 10`.
    fn tendril_animation(self, partial_tick: f32) -> f32 {
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
    fn advance_client_tick(&mut self, age_ticks: u32, heartbeat_delay: u32) -> bool {
        let heartbeat = heartbeat_delay > 0 && age_ticks % heartbeat_delay == 0;
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
    fn heart_animation(self, partial_tick: f32) -> f32 {
        let lerped = self.previous as f32 + partial_tick * (self.current - self.previous) as f32;
        lerped / WARDEN_HEART_ANIMATION_TICKS as f32
    }
}

const JAVA_RANDOM_MULTIPLIER: u64 = 0x5DEEC_E66D;
const JAVA_RANDOM_ADDEND: u64 = 0xB;
const JAVA_RANDOM_MASK: u64 = (1_u64 << 48) - 1;

pub(super) fn java_random_seed(seed: i64) -> u64 {
    ((seed as u64) ^ JAVA_RANDOM_MULTIPLIER) & JAVA_RANDOM_MASK
}

pub(super) fn java_random_next_bits(state: &mut u64, bits: u32) -> u32 {
    *state = state
        .wrapping_mul(JAVA_RANDOM_MULTIPLIER)
        .wrapping_add(JAVA_RANDOM_ADDEND)
        & JAVA_RANDOM_MASK;
    (*state >> (48 - bits)) as u32
}

fn java_random_next_long(state: &mut u64) -> i64 {
    let high = i64::from(java_random_next_bits(state, 32) as i32);
    let low = i64::from(java_random_next_bits(state, 32) as i32);
    (high << 32).wrapping_add(low)
}

pub(super) fn java_random_next_int_bound(state: &mut u64, bound: i32) -> i32 {
    debug_assert!(bound > 0);
    if (bound & (bound - 1)) == 0 {
        return ((i64::from(bound) * i64::from(java_random_next_bits(state, 31))) >> 31) as i32;
    }

    loop {
        let bits = java_random_next_bits(state, 31) as i32;
        let value = bits % bound;
        if bits.wrapping_sub(value).wrapping_add(bound - 1) >= 0 {
            return value;
        }
    }
}

fn java_random_next_double(state: &mut u64) -> f64 {
    let high = u64::from(java_random_next_bits(state, 26));
    let low = u64::from(java_random_next_bits(state, 27));
    ((high << 27) + low) as f64 / (1_u64 << 53) as f64
}

pub(super) fn java_random_next_float(state: &mut u64) -> f32 {
    java_random_next_bits(state, 24) as f32 / ((1_u32 << 24) as f32)
}

fn lightning_bolt_spawn_random(uuid: Uuid, entity_type_id: i32) -> (u64, i64) {
    let mut random_seed = java_random_seed(spawn_random_bits(
        uuid,
        LIGHTNING_BOLT_CLIENT_RANDOM_SALT ^ entity_type_id as u64,
    ) as i64);
    let render_seed = java_random_next_long(&mut random_seed);
    java_random_next_int_bound(&mut random_seed, 3);
    (random_seed, render_seed)
}

/// Vanilla `AbstractHorse.tailCounter`, with the random idle start driven from a repo-local
/// deterministic Java LCG. The exact vanilla client RNG seed is not sent over the protocol, but the
/// `nextInt(200)` distribution and `tailCounter` lifetime mirror `AbstractHorse.aiStep` / `tick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquineTailAnimationState {
    random_seed: u64,
    tail_counter: u8,
}

impl EquineTailAnimationState {
    fn new(entity_id: i32) -> Self {
        Self {
            random_seed: java_random_seed(i64::from(entity_id)),
            tail_counter: 0,
        }
    }

    fn advance_client_tick(&mut self) {
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

    fn next_double(&mut self) -> f64 {
        java_random_next_double(&mut self.random_seed)
    }

    fn advance_eating_pitch(&mut self) {
        let _discarded_first = java_random_next_float(&mut self.random_seed);
        let _discarded_second = java_random_next_float(&mut self.random_seed);
    }

    fn animate_tail(self) -> bool {
        self.tail_counter > 0
    }
}

/// Vanilla `AbstractHorse.eatAnim` / `standAnim` / `mouthAnim` client accumulators. The targets are
/// the synced `AbstractHorse.DATA_ID_FLAGS` bits; the client eases the three floats every tick and
/// `AbstractHorseRenderer.extractRenderState` exposes their partial-tick lerps.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EquinePoseAnimationState {
    target_eating: bool,
    target_standing: bool,
    target_open_mouth: bool,
    previous_eat_animation: f32,
    current_eat_animation: f32,
    previous_stand_animation: f32,
    current_stand_animation: f32,
    previous_mouth_animation: f32,
    current_mouth_animation: f32,
}

impl EquinePoseAnimationState {
    fn from_flags(eating: bool, standing: bool, open_mouth: bool) -> Self {
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

    fn set_flags(&mut self, eating: bool, standing: bool, open_mouth: bool) {
        self.target_eating = eating;
        self.target_standing = standing;
        self.target_open_mouth = open_mouth;
    }

    fn advance_client_tick(&mut self) {
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

    fn eat_animation(self, partial_tick: f32) -> f32 {
        self.previous_eat_animation
            + partial_tick * (self.current_eat_animation - self.previous_eat_animation)
    }

    fn stand_animation(self, partial_tick: f32) -> f32 {
        self.previous_stand_animation
            + partial_tick * (self.current_stand_animation - self.previous_stand_animation)
    }

    fn mouth_animation(self, partial_tick: f32) -> f32 {
        self.previous_mouth_animation
            + partial_tick * (self.current_mouth_animation - self.previous_mouth_animation)
    }
}

/// Java `java.util.Random` LCG `setSeed(seed)` then `nextFloat()`, used to seed
/// `Squid.tentacleSpeed` from the entity id deterministically on the client.
/// Replicates the multiplier/addend/mask of `LevelEventSoundRandomState` (the
/// audio module's Java-compatible LCG) so the value matches vanilla exactly.
fn java_random_first_next_float(seed: i64) -> f32 {
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
    fn reset_movement(&mut self) {
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
    fn advance_client_tick(&mut self, delta_movement: EntityVec3, in_water: bool) {
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
    fn tentacle_angle(&self, partial_tick: f32) -> f32 {
        self.old_tentacle_angle + (self.tentacle_angle - self.old_tentacle_angle) * partial_tick
    }

    /// Vanilla `SquidRenderer.extractRenderState`: `lerp(partialTicks, xBodyRotO,
    /// xBodyRot)`.
    fn x_body_rot(&self, partial_tick: f32) -> f32 {
        self.old_x_body_rot + (self.x_body_rot - self.old_x_body_rot) * partial_tick
    }

    /// Vanilla `LivingEntityRenderer.extractRenderState`:
    /// `Mth.rotLerp(partialTicks, yBodyRotO, yBodyRot)`. Squid updates this from
    /// its movement vector during `aiStep`.
    fn y_body_rot(&self, partial_tick: f32) -> f32 {
        self.old_y_body_rot + wrap_degrees(self.y_body_rot - self.old_y_body_rot) * partial_tick
    }

    /// Vanilla `SquidRenderer.extractRenderState`: `lerp(partialTicks, zBodyRotO,
    /// zBodyRot)`.
    fn z_body_rot(&self, partial_tick: f32) -> f32 {
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
    fn advance_client_tick(&mut self, on_ground: bool) {
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
    fn flap(&self, partial_tick: f32) -> f32 {
        self.o_flap + (self.flap - self.o_flap) * partial_tick
    }

    /// Vanilla `ChickenRenderer.extractRenderState`: `lerp(partialTicks, oFlapSpeed,
    /// flapSpeed)`.
    fn flap_speed(&self, partial_tick: f32) -> f32 {
        self.o_flap_speed + (self.flap_speed - self.o_flap_speed) * partial_tick
    }
}

impl SlimeAnimationState {
    /// Advances one client tick of `Slime.tick`, including the ordered landing
    /// particle and sound random stream.
    ///
    /// Vanilla saves the lerp endpoint (`oSquish`), eases `squish` halfway toward
    /// `targetSquish`, then — after `super.tick()` — re-seeds the target on the
    /// `onGround()` landing (`-0.5`) / takeoff (`1.0`) transitions, records the
    /// ground flag, and calls the type-specific `decreaseSquish` implementation.
    fn advance_client_tick(
        &mut self,
        on_ground: bool,
        magma_cube: bool,
        size: i32,
        width: f32,
        is_silent: bool,
        random_seed: &mut u64,
        triggers: &mut EntityClientTickParticleTriggers,
    ) {
        self.o_squish = self.squish;
        self.squish += (self.target_squish - self.squish) * 0.5;
        if on_ground && !self.was_on_ground {
            let radius = width;
            let particle_count = (radius * 2.0 * 16.0).ceil() as usize;
            let mut offsets = Vec::with_capacity(particle_count);
            for _ in 0..particle_count {
                let direction = java_random_next_float(random_seed) * std::f32::consts::TAU;
                let distance = java_random_next_float(random_seed) * 0.5 + 0.5;
                offsets.push(EntityVec3 {
                    x: f64::from(
                        super::client_movement::vanilla_mth_sin(f64::from(direction))
                            * radius
                            * distance,
                    ),
                    y: 0.0,
                    z: f64::from(
                        super::client_movement::vanilla_mth_cos(f64::from(direction))
                            * radius
                            * distance,
                    ),
                });
            }
            triggers.slime_landing_particles = Some(SlimeLandingParticleTrigger {
                kind: if magma_cube {
                    super::SlimeLandingParticleKind::Flame
                } else {
                    super::SlimeLandingParticleKind::ItemSlime
                },
                offsets,
            });
            // Java evaluates both random pitch operands before `Entity.playSound`
            // applies the silent gate.
            let pitch =
                ((java_random_next_float(random_seed) - java_random_next_float(random_seed)) * 0.2
                    + 1.0)
                    / 0.8;
            if !is_silent {
                triggers.slime_squish_sound = Some(SlimeSquishSoundParameters {
                    magma_cube,
                    tiny: size <= 1,
                    volume: 0.4 * size as f32,
                    pitch,
                });
            }
            self.target_squish = -0.5;
        } else if !on_ground && self.was_on_ground {
            self.target_squish = 1.0;
        }
        self.was_on_ground = on_ground;
        self.target_squish *= if magma_cube { 0.9 } else { 0.6 };
    }

    /// Vanilla `SlimeRenderer.extractRenderState`: `lerp(partialTicks, oSquish,
    /// squish)`.
    fn squish(&self, partial_tick: f32) -> f32 {
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
    fn new(entity_id: i32) -> Self {
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
    fn advance_client_tick(
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
    fn bite_progress(&self, partial_tick: f32) -> f32 {
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
    fn advance_client_tick(&mut self, on_ground: bool, is_passenger: bool) {
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
    fn flap_angle(&self, partial_tick: f32) -> f32 {
        let flap = self.o_flap + (self.flap - self.o_flap) * partial_tick;
        let flap_speed = self.o_flap_speed + (self.flap_speed - self.o_flap_speed) * partial_tick;
        (flap.sin() + 1.0) * flap_speed
    }
}

impl CreeperSwellAnimationState {
    fn set_swell_dir(&mut self, swell_dir: i32) {
        self.swell_dir = swell_dir;
    }

    /// Vanilla `Creeper.tick`: `oldSwell = swell; swell += swellDir`, clamped to
    /// `0..=maxSwell`.
    fn advance_client_tick(&mut self) {
        self.previous_swell = self.current_swell;
        self.current_swell = (self.current_swell + self.swell_dir).clamp(0, CREEPER_MAX_SWELL);
    }

    /// Whether the fuse has fully settled at rest so the state can be dropped.
    fn is_settled(&self) -> bool {
        self.swell_dir <= 0 && self.current_swell == 0 && self.previous_swell == 0
    }

    /// Vanilla `Creeper.getSwelling`: `lerp(partialTick, oldSwell, swell) /
    /// (maxSwell - 2)`.
    fn swelling(self, partial_tick: f32) -> f32 {
        let lerped = self.previous_swell as f32
            + partial_tick * (self.current_swell - self.previous_swell) as f32;
        lerped / (CREEPER_MAX_SWELL as f32 - 2.0)
    }
}

impl ShulkerPeekAnimationState {
    fn set_target(&mut self, target_peek_amount: f32) {
        self.target_peek_amount = target_peek_amount;
    }

    fn advance_client_tick(&mut self) {
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
    fn peek_amount(self, partial_tick: f32) -> f32 {
        self.previous_peek_amount
            + partial_tick * (self.current_peek_amount - self.previous_peek_amount)
    }
}

impl BeeRollAnimationState {
    fn set_rolling(&mut self, rolling: bool) {
        self.rolling = rolling;
    }

    /// Vanilla `Bee.updateRollAmount`: `rollAmountO = rollAmount`, then a rolling
    /// bee climbs `min(1, rollAmount + 0.2)` while a non-rolling one decays
    /// `max(0, rollAmount - 0.24)`.
    fn advance_client_tick(&mut self) {
        self.previous_roll_amount = self.current_roll_amount;
        self.current_roll_amount = if self.rolling {
            (self.current_roll_amount + 0.2).min(1.0)
        } else {
            (self.current_roll_amount - 0.24).max(0.0)
        };
    }

    /// Vanilla `Bee.getRollAmount(partialTick)` =
    /// `Mth.lerp(partialTick, rollAmountO, rollAmount)`.
    fn roll_amount(self, partial_tick: f32) -> f32 {
        self.previous_roll_amount
            + partial_tick * (self.current_roll_amount - self.previous_roll_amount)
    }
}

impl PandaAnimationState {
    fn set_flags(&mut self, sitting: bool, on_back: bool, rolling: bool) {
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

    fn advance_client_tick(&mut self) {
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

    fn sit_amount(self, partial_tick: f32) -> f32 {
        lerp_panda_amount(
            partial_tick,
            self.previous_sit_amount,
            self.current_sit_amount,
        )
    }

    fn lie_on_back_amount(self, partial_tick: f32) -> f32 {
        lerp_panda_amount(
            partial_tick,
            self.previous_on_back_amount,
            self.current_on_back_amount,
        )
    }

    fn roll_amount(self, partial_tick: f32) -> f32 {
        lerp_panda_amount(
            partial_tick,
            self.previous_roll_amount,
            self.current_roll_amount,
        )
    }

    fn roll_time(self, partial_tick: f32) -> f32 {
        if self.roll_counter > 0 {
            self.roll_counter as f32 + partial_tick
        } else {
            0.0
        }
    }

    fn is_settled(self) -> bool {
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

fn ease_panda_amount(current: f32, active: bool) -> f32 {
    if active {
        (current + PANDA_AMOUNT_RISE_PER_TICK).min(1.0)
    } else {
        (current - PANDA_AMOUNT_FALL_PER_TICK).max(0.0)
    }
}

fn lerp_panda_amount(partial_tick: f32, previous: f32, current: f32) -> f32 {
    previous + partial_tick * (current - previous)
}

impl FoxAnimationState {
    fn set_flags(&mut self, interested: bool, crouching: bool) {
        self.interested = interested;
        self.crouching = crouching;
    }

    /// Vanilla `Fox.tick`: `interestedAngleO = interestedAngle`, then it eases
    /// toward `isInterested ? 1 : 0` by `* 0.4`/tick. Independently,
    /// `crouchAmountO = crouchAmount`, then a crouching fox climbs by `0.2`/tick
    /// (clamped to `MAX_CROUCH_AMOUNT`) while a non-crouching one is reset INSTANTLY
    /// to `0` (an assignment, not a decay).
    fn advance_client_tick(&mut self) {
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
    fn head_roll_angle(self, partial_tick: f32) -> f32 {
        let lerped = self.previous_interested_angle
            + partial_tick * (self.current_interested_angle - self.previous_interested_angle);
        lerped * FOX_HEAD_ROLL_SCALE * std::f32::consts::PI
    }

    /// Vanilla `Fox.getCrouchAmount(partialTick)` =
    /// `Mth.lerp(partialTick, crouchAmountO, crouchAmount)`.
    fn crouch_amount(self, partial_tick: f32) -> f32 {
        self.previous_crouch_amount
            + partial_tick * (self.current_crouch_amount - self.previous_crouch_amount)
    }
}

impl EntityClientAnimationState {
    pub(super) fn start_local_arrow_shake(&mut self) {
        self.arrow_shake = Some(ArrowShakeAnimationState::started());
    }

    pub(super) fn advance_arrow_shake_tick(&mut self) {
        if let Some(shake) = self.arrow_shake.as_mut() {
            shake.advance_client_tick();
            if shake.is_settled() {
                self.arrow_shake = None;
            }
        }
    }

    pub(crate) fn camel_dash_should_clear_before_tick(
        self,
        is_dashing: bool,
        on_ground: bool,
        in_liquid: bool,
        is_passenger: bool,
    ) -> bool {
        // A newly observed synced DASH edge has the same effective 55-tick
        // cooldown that `Camel.onSyncedDataUpdated` installs in Vanilla. The
        // state is created by this tick below, so it cannot clear on that edge.
        self.camel_dash_cooldown.is_some_and(|cooldown| {
            cooldown.should_clear_before_tick(is_dashing, on_ground, in_liquid, is_passenger)
        })
    }

    pub(crate) fn next_camel_voice_pitch(&mut self, entity_id: i32, is_baby: bool) -> f32 {
        let random_seed = self.camel_voice_random_seed.get_or_insert_with(|| {
            java_random_seed(i64::from(entity_id).wrapping_add(CAMEL_VOICE_RANDOM_SALT as i64))
        });
        let base = if is_baby { 1.5 } else { 1.0 };
        (java_random_next_float(random_seed) - java_random_next_float(random_seed)) * 0.2 + base
    }

    pub(crate) fn start_local_camel_dash_after_tick(&mut self) {
        self.camel_dash
            .get_or_insert_with(KeyframeAnimationState::default)
            .animate_when(true, self.age_ticks);
        self.camel_dash_cooldown
            .get_or_insert_with(Default::default)
            .start_local_dash_after_tick();
    }

    pub(crate) fn stop_local_camel_dash(&mut self) {
        if let Some(dash) = self.camel_dash.as_mut() {
            dash.animate_when(false, self.age_ticks);
        }
        if let Some(cooldown) = self.camel_dash_cooldown.as_mut() {
            cooldown.stop_local_dash();
        }
    }

    pub(crate) fn start_local_nautilus_dash(&mut self) {
        self.nautilus_dash_cooldown
            .get_or_insert_with(Default::default)
            .start_local_dash();
    }

    pub(crate) fn advance_item_based_steering_tick(&mut self, boost_time_total: i32) -> f32 {
        self.item_based_steering
            .as_mut()
            .map_or(1.0, |steering| steering.tick_and_factor(boost_time_total))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn advance_nautilus_subclass_tick(
        &mut self,
        entity_id: i32,
        is_dashing: bool,
        in_water: bool,
        underwater: bool,
        is_silent: bool,
        is_baby: bool,
        delta_movement: EntityVec3,
    ) -> EntityClientTickParticleTriggers {
        let mut triggers = EntityClientTickParticleTriggers::default();
        let (clear_dash, ready_sound) = self
            .nautilus_dash_cooldown
            .as_mut()
            .map_or((is_dashing, false), |cooldown| {
                cooldown.advance_subclass_tick(is_dashing)
            });
        triggers.nautilus_clear_dash = clear_dash;
        if self
            .nautilus_dash_cooldown
            .is_some_and(NautilusDashCooldownState::is_settled)
        {
            self.nautilus_dash_cooldown = None;
        }

        let random_seed = self.nautilus_client_random_seed.get_or_insert_with(|| {
            java_random_seed(i64::from(entity_id).wrapping_add(NAUTILUS_CLIENT_RANDOM_SALT as i64))
        });
        if ready_sound {
            // `makeSound(getDashReadySound())` evaluates `getVoicePitch()` before
            // `Entity.playSound` applies the silent gate.
            let base = if is_baby { 1.5 } else { 1.0 };
            let pitch = (java_random_next_float(random_seed) - java_random_next_float(random_seed))
                * 0.2
                + base;
            if !is_silent {
                triggers.nautilus_dash_ready_sound =
                    Some(NautilusDashReadySoundParameters { underwater, pitch });
            }
        }
        if in_water {
            let speed = (delta_movement.x * delta_movement.x
                + delta_movement.y * delta_movement.y
                + delta_movement.z * delta_movement.z)
                .sqrt();
            let probability = (speed * NAUTILUS_BUBBLE_PROBABILITY_MULTIPLIER).clamp(
                NAUTILUS_BUBBLE_PROBABILITY_MIN,
                NAUTILUS_BUBBLE_PROBABILITY_MAX,
            );
            if f64::from(java_random_next_float(random_seed)) < probability {
                let spread = java_random_next_double(random_seed)
                    * NAUTILUS_BUBBLE_SPREAD_FACTOR
                    * (1.0 + speed);
                triggers.nautilus_bubble = Some(EntityVec3 {
                    x: (f64::from(java_random_next_float(random_seed)) - 0.5) * spread,
                    y: (f64::from(java_random_next_float(random_seed)) - 0.5) * spread,
                    z: (f64::from(java_random_next_float(random_seed)) - 0.5) * spread,
                });
            }
        }
        triggers
    }

    pub(crate) fn advance_eating_voice_pitch_random(&mut self, entity_id: i32, nautilus: bool) {
        let (random_seed, salt) = if nautilus {
            (
                &mut self.nautilus_client_random_seed,
                NAUTILUS_CLIENT_RANDOM_SALT,
            )
        } else {
            (&mut self.eating_voice_random_seed, EATING_VOICE_RANDOM_SALT)
        };
        let random_seed = random_seed.get_or_insert_with(|| {
            java_random_seed(i64::from(entity_id).wrapping_add(salt as i64))
        });
        let _discarded_first = java_random_next_float(random_seed);
        let _discarded_second = java_random_next_float(random_seed);
    }

    pub(crate) fn next_equine_feeding_particle_units(&mut self, entity_id: i32) -> [f64; 3] {
        let equine = self
            .equine_tail
            .get_or_insert_with(|| EquineTailAnimationState::new(entity_id));
        [
            equine.next_double(),
            equine.next_double(),
            equine.next_double(),
        ]
    }

    pub(crate) fn advance_equine_eating_pitch_random(&mut self, entity_id: i32) {
        self.equine_tail
            .get_or_insert_with(|| EquineTailAnimationState::new(entity_id))
            .advance_eating_pitch();
    }

    pub(crate) fn next_local_interaction_random_float(&mut self, entity_id: i32) -> f32 {
        let random_seed = self.local_interaction_random_seed.get_or_insert_with(|| {
            java_random_seed(i64::from(entity_id).wrapping_add(0x4d4f_4249))
        });
        java_random_next_float(random_seed)
    }

    pub(crate) fn next_local_interaction_random_double(&mut self, entity_id: i32) -> f64 {
        let random_seed = self.local_interaction_random_seed.get_or_insert_with(|| {
            java_random_seed(i64::from(entity_id).wrapping_add(0x4d4f_4249))
        });
        java_random_next_double(random_seed)
    }

    pub(crate) fn needs_particle_trigger_bounds_snapshot(&self) -> bool {
        self.ravager.is_some() || self.evoker_fangs.is_some()
    }

    pub(crate) fn initialize_spawn_defaults(&mut self, entity_type_id: i32, uuid: Uuid) {
        self.item_bob_offset = (entity_type_id == VANILLA_ENTITY_TYPE_ITEM_ID)
            .then(|| spawn_random_unit_float(uuid, 0x6974_656d) * std::f32::consts::TAU);
        self.end_crystal_time_offset = (entity_type_id == VANILLA_ENTITY_TYPE_END_CRYSTAL_ID)
            .then(|| (spawn_random_bits(uuid, 0x6372_7973_7461_6c) % 100_000) as u32);
        if vanilla_entity_metadata_type_is_guardian(entity_type_id) {
            let mut random_seed = java_random_seed(spawn_random_bits(
                uuid,
                GUARDIAN_CLIENT_RANDOM_SALT ^ entity_type_id as u64,
            ) as i64);
            self.guardian_tail = Some(GuardianTailAnimationState::with_initial_phase(
                java_random_next_float(&mut random_seed),
            ));
            self.guardian_client_random_seed = Some(random_seed);
        } else {
            self.guardian_tail = None;
            self.guardian_client_random_seed = None;
        }
        self.minecart_furnace_smoke_random_seed = (entity_type_id
            == VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID)
            .then(|| java_random_seed(spawn_random_bits(uuid, 0x6675_726e_6163_655f) as i64));
        self.blaze_client_random_seed = (entity_type_id == VANILLA_ENTITY_TYPE_BLAZE_ID)
            .then(|| java_random_seed(spawn_random_bits(uuid, BLAZE_CLIENT_RANDOM_SALT) as i64));
        self.ender_ambient_random_seed = matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_ENDERMAN_ID | VANILLA_ENTITY_TYPE_ENDERMITE_ID
        )
        .then(|| {
            java_random_seed(spawn_random_bits(
                uuid,
                ENDER_AMBIENT_RANDOM_SALT ^ entity_type_id as u64,
            ) as i64)
        });
        self.warden_client_random_seed =
            (entity_type_id == VANILLA_ENTITY_TYPE_WARDEN_ID).then(|| {
                java_random_seed(spawn_random_bits(
                    uuid,
                    WARDEN_CLIENT_RANDOM_SALT ^ entity_type_id as u64,
                ) as i64)
            });
        self.ender_dragon_client_random_seed =
            (entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID).then(|| {
                java_random_seed(spawn_random_bits(
                    uuid,
                    ENDER_DRAGON_CLIENT_RANDOM_SALT ^ entity_type_id as u64,
                ) as i64)
            });
        if entity_type_id == VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID {
            let (random_seed, render_seed) = lightning_bolt_spawn_random(uuid, entity_type_id);
            self.lightning_bolt_client_random_seed = Some(random_seed);
            self.lightning_bolt_render_seed = Some(render_seed);
        } else {
            self.lightning_bolt_client_random_seed = None;
            self.lightning_bolt_render_seed = None;
        }
        self.camel_voice_random_seed =
            vanilla_entity_metadata_type_is_camel(entity_type_id).then(|| {
                java_random_seed(spawn_random_bits(
                    uuid,
                    CAMEL_VOICE_RANDOM_SALT ^ entity_type_id as u64,
                ) as i64)
            });
        self.nautilus_client_random_seed =
            is_vanilla_abstract_nautilus_type(entity_type_id).then(|| {
                java_random_seed(spawn_random_bits(
                    uuid,
                    NAUTILUS_CLIENT_RANDOM_SALT ^ entity_type_id as u64,
                ) as i64)
            });
        self.slime_client_random_seed =
            vanilla_entity_metadata_type_is_slime(entity_type_id).then(|| {
                java_random_seed(spawn_random_bits(
                    uuid,
                    SLIME_CLIENT_RANDOM_SALT ^ entity_type_id as u64,
                ) as i64)
            });
        self.eating_voice_random_seed =
            (vanilla_entity_metadata_eating_sound_policy(entity_type_id)
                == Some(VanillaEntityEatingSoundPolicy::EntityVoicePitch)
                && !is_vanilla_abstract_nautilus_type(entity_type_id))
            .then(|| {
                java_random_seed(spawn_random_bits(
                    uuid,
                    EATING_VOICE_RANDOM_SALT ^ entity_type_id as u64,
                ) as i64)
            });
        self.nautilus_dash_cooldown = None;
        self.item_based_steering =
            vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
                entity_type_id,
            )
            .map(|_| ItemBasedSteeringClientState::default());
        self.ridden_jump = vanilla_entity_metadata_type_is_player_rideable_jumping(entity_type_id)
            .then(RiddenJumpState::default);
    }

    pub(crate) fn item_bob_offset(&self) -> f32 {
        self.item_bob_offset.unwrap_or(0.0)
    }

    pub(crate) fn end_crystal_age_ticks(&self) -> u32 {
        self.end_crystal_time_offset
            .unwrap_or(0)
            .wrapping_add(self.age_ticks)
    }

    pub(crate) fn sync_events_from_metadata_update(
        &mut self,
        entity_type_id: i32,
        updated_values: &[EntityDataValue],
        data_values: &[EntityDataValue],
    ) {
        // Vanilla `AbstractArrow.onSyncedDataUpdated(IN_GROUND)`: once the entity is past
        // its first client tick, setting `IN_GROUND` true starts the seven-tick impact
        // wobble if the previous shake has already settled. This is class inheritance,
        // not the `#arrows` entity-type tag: ThrownTrident follows the same callback.
        if let Some(accessors) =
            vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(entity_type_id)
        {
            if self.age_ticks > 0
                && updated_values
                    .iter()
                    .any(|value| value.data_id == accessors.in_ground_data_id)
                && entity_data_bool(
                    data_values,
                    accessors.in_ground_data_id,
                    accessors.in_ground_default,
                )
                && self
                    .arrow_shake
                    .map_or(true, ArrowShakeAnimationState::is_settled)
            {
                self.arrow_shake = Some(ArrowShakeAnimationState::started());
            }
        }
        if let Some(accessors) =
            vanilla_entity_metadata_vehicle_accessors_for_entity_type(entity_type_id)
        {
            if updated_values.iter().any(|value| {
                value.data_id == accessors.hurt_time_data_id
                    || value.data_id == accessors.hurt_dir_data_id
                    || value.data_id == accessors.damage_data_id
            }) {
                let damage = BoatDamageAnimationState::from_metadata(accessors, data_values);
                if damage.is_settled() {
                    self.boat_damage = None;
                } else {
                    self.boat_damage = Some(damage);
                }
            }
        }
        // `AbstractNautilus.onSyncedDataUpdated(DASH)` seeds the cooldown after
        // the first entity tick regardless of the synchronized boolean value.
        if is_vanilla_abstract_nautilus_type(entity_type_id)
            && self.age_ticks > 0
            && updated_values
                .iter()
                .any(|value| value.data_id == VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID)
        {
            self.nautilus_dash_cooldown
                .get_or_insert_with(Default::default)
                .seed_from_synced_update();
        }
        if let Some((boost_time_data_id, _)) =
            vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
                entity_type_id,
            )
        {
            if updated_values
                .iter()
                .any(|value| value.data_id == boost_time_data_id)
            {
                self.item_based_steering
                    .get_or_insert_with(Default::default)
                    .on_synced();
            }
        }
    }

    pub(crate) fn sync_targets_from_metadata(
        &mut self,
        entity_type_id: i32,
        data_values: &[EntityDataValue],
    ) {
        // Vanilla `LivingEntity.isDeadOrDying()` (`getHealth() <= 0`): a living
        // entity whose synced health has reached zero begins the death animation;
        // restoring health clears it. Only living entities carry the health float,
        // so the per-type model animations below are unaffected.
        let living_accessors =
            vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id);
        if vanilla_living_entity_type(entity_type_id) {
            if let Some(health) = living_accessors
                .and_then(|accessors| entity_data_float(data_values, accessors.health_data_id))
            {
                if health <= 0.0 {
                    self.death
                        .get_or_insert(DeathAnimationState { death_time: 0 });
                } else {
                    self.death = None;
                }
            }
            // Vanilla `LivingEntity.isVisuallySwimming()` also treats the
            // FALL_FLYING pose as swimming while the shared fall-flying flag is
            // clear. This is the crawling transition used by humanoid head/root
            // animation, not merely `Entity.isSwimming()`.
            let pose = entity_data_pose(data_values);
            let visually_swimming = pose == VANILLA_ENTITY_POSE_SWIMMING_ID
                || (pose == VANILLA_ENTITY_POSE_FALL_FLYING_ID
                    && !entity_is_fall_flying(data_values));
            if let Some(swim) = self.living_swim.as_mut() {
                swim.set_visually_swimming(visually_swimming);
            } else if visually_swimming {
                self.living_swim = Some(LivingSwimAmountState {
                    visually_swimming,
                    previous_swim_amount: 0.0,
                    current_swim_amount: 0.0,
                });
            }
        }
        if entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
            if let Some(health) = living_accessors
                .and_then(|accessors| entity_data_float(data_values, accessors.health_data_id))
            {
                self.ender_dragon
                    .get_or_insert_with(EnderDragonAnimationState::default)
                    .set_dragon_dying(health <= 0.0);
            }
        }
        match entity_type_id {
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => {
                let target_standing =
                    vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type(
                        entity_type_id,
                    )
                    .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
                    .unwrap_or_default();
                if let Some(standing) = self.polar_bear_standing.as_mut() {
                    standing.set_target(target_standing);
                } else if target_standing {
                    self.polar_bear_standing = Some(PolarBearStandingAnimationState {
                        target_standing,
                        ..PolarBearStandingAnimationState::default()
                    });
                }
            }
            VANILLA_ENTITY_TYPE_CREEPER_ID => {
                let swell_dir = creeper_effective_swell_dir(entity_type_id, data_values);
                if let Some(swell) = self.creeper_swell.as_mut() {
                    swell.set_swell_dir(swell_dir);
                } else if swell_dir > 0 {
                    self.creeper_swell = Some(CreeperSwellAnimationState {
                        swell_dir,
                        previous_swell: 0,
                        current_swell: 0,
                    });
                }
            }
            VANILLA_ENTITY_TYPE_SHULKER_ID => {
                let target_peek_amount = shulker_target_peek_amount(entity_type_id, data_values);
                if let Some(peek) = self.shulker_peek.as_mut() {
                    peek.set_target(target_peek_amount);
                } else if target_peek_amount > 0.0 {
                    self.shulker_peek = Some(ShulkerPeekAnimationState {
                        target_peek_amount,
                        ..ShulkerPeekAnimationState::default()
                    });
                }
            }
            VANILLA_ENTITY_TYPE_BEE_ID => {
                let rolling = bee_is_rolling(entity_type_id, data_values);
                if let Some(roll) = self.bee_roll.as_mut() {
                    roll.set_rolling(rolling);
                } else if rolling {
                    self.bee_roll = Some(BeeRollAnimationState {
                        rolling,
                        ..BeeRollAnimationState::default()
                    });
                }
            }
            VANILLA_ENTITY_TYPE_PANDA_ID => {
                let (sitting, on_back, rolling) = panda_pose_flags(entity_type_id, data_values);
                if let Some(panda) = self.panda.as_mut() {
                    panda.set_flags(sitting, on_back, rolling);
                } else if sitting || on_back || rolling {
                    self.panda = Some(PandaAnimationState {
                        sitting,
                        on_back,
                        rolling,
                        ..PandaAnimationState::default()
                    });
                }
            }
            VANILLA_ENTITY_TYPE_FOX_ID => {
                let accessors =
                    vanilla_entity_metadata_fox_accessors_for_entity_type(entity_type_id)
                        .expect("generated Fox metadata accessors must resolve");
                let flags = entity_data_byte(
                    data_values,
                    accessors.flags_data_id,
                    accessors.flags_default,
                );
                let interested = flags & accessors.interested_mask != 0;
                let crouching = flags & accessors.crouching_mask != 0;
                if let Some(fox) = self.fox.as_mut() {
                    fox.set_flags(interested, crouching);
                } else if interested || crouching {
                    self.fox = Some(FoxAnimationState {
                        interested,
                        crouching,
                        ..FoxAnimationState::default()
                    });
                }
            }
            VANILLA_ENTITY_TYPE_CAT_ID => {
                let is_lying = cat_is_lying(entity_type_id, data_values);
                let relax_state_one = cat_relax_state_one(entity_type_id, data_values);
                if let Some(cat) = self.cat_lie_down.as_mut() {
                    cat.set_flags(is_lying, relax_state_one);
                } else if is_lying || relax_state_one {
                    self.cat_lie_down = Some(CatLieDownAnimationState {
                        is_lying,
                        relax_state_one,
                        ..CatLieDownAnimationState::default()
                    });
                }
            }
            entity_type_id
                if vanilla_entity_renderer_uses_abstract_horse_renderer(entity_type_id) =>
            {
                let (eating, standing, open_mouth) = equine_pose_flags(entity_type_id, data_values);
                if let Some(pose) = self.equine_pose.as_mut() {
                    pose.set_flags(eating, standing, open_mouth);
                } else if eating || standing || open_mouth {
                    self.equine_pose = Some(EquinePoseAnimationState::from_flags(
                        eating, standing, open_mouth,
                    ));
                }
            }
            VANILLA_ENTITY_TYPE_FROG_ID => {
                // Vanilla `Frog.onSyncedDataUpdated`: when `DATA_POSE` changes, the croak animation
                // is `animateWhen(pose == CROAKING, tickCount)` and the jump animation is started
                // when `pose == LONG_JUMPING` and stopped otherwise — both started on the rising edge
                // into their pose and stopped on leaving it. The frog's `aiStep` runs client-side for
                // remote entities, so the synced pose drives both directly.
                let pose = entity_data_pose(data_values);
                let croaking = pose == VANILLA_ENTITY_POSE_CROAKING_ID;
                if let Some(croak) = self.frog_croak.as_mut() {
                    croak.animate_when(croaking, self.age_ticks);
                } else if croaking {
                    self.frog_croak = Some(KeyframeAnimationState {
                        start_age: Some(self.age_ticks),
                    });
                }
                // Vanilla `Frog.onSyncedDataUpdated`: `tongueAnimationState` is started on the synced
                // `Pose.USING_TONGUE` and stopped otherwise — the same rising-edge pattern as the
                // croak, driving the `FROG_TONGUE` keyframe (head dip + the tongue's z-scale lash).
                let using_tongue = pose == VANILLA_ENTITY_POSE_USING_TONGUE_ID;
                if let Some(tongue) = self.frog_tongue.as_mut() {
                    tongue.animate_when(using_tongue, self.age_ticks);
                } else if using_tongue {
                    self.frog_tongue = Some(KeyframeAnimationState {
                        start_age: Some(self.age_ticks),
                    });
                }
                let jumping = pose == VANILLA_ENTITY_POSE_LONG_JUMPING_ID;
                if let Some(jump) = self.frog_jump.as_mut() {
                    jump.animate_when(jumping, self.age_ticks);
                } else if jumping {
                    self.frog_jump = Some(KeyframeAnimationState {
                        start_age: Some(self.age_ticks),
                    });
                }
            }
            VANILLA_ENTITY_TYPE_WARDEN_ID => {
                // Vanilla `Warden.onSyncedDataUpdated(DATA_POSE)`: the pose-change `switch` that
                // `.start()`s the roar/sniff/emerge/dig one-shot when the synced `DATA_POSE` CHANGES
                // to `Pose.ROARING`/`Pose.SNIFFING`/`Pose.EMERGING`/`Pose.DIGGING`. We track the
                // previous pose and restart the timer only on the transition; vanilla never
                // auto-stops on a pose leave, so the non-looping keyframe holds its final frame. The
                // warden's `aiStep` runs client-side for remote entities, so the synced pose drives
                // the pose directly. (The attack/sonic-boom one-shots are event-driven.)
                let pose_id = entity_data_pose(data_values);
                self.warden_combat
                    .get_or_insert_with(WardenCombatAnimationState::default)
                    .set_pose(pose_id, self.age_ticks);
            }
            VANILLA_ENTITY_TYPE_BREEZE_ID => {
                // Vanilla `Breeze.onSyncedDataUpdated(DATA_POSE)` + `Breeze.tick`: the synced pose
                // drives the shoot/inhale/slide/longJump one-shots (active while their pose holds) and
                // the slideBack return (on leaving SLIDING). The breeze's `tick` runs client-side for
                // remote entities, so the synced pose drives all of them directly.
                let pose_id = entity_data_pose(data_values);
                self.breeze
                    .get_or_insert_with(BreezeAnimationState::default)
                    .set_pose(pose_id, self.age_ticks);
            }
            VANILLA_ENTITY_TYPE_SNIFFER_ID => {
                // Vanilla `Sniffer.onSyncedDataUpdated(DATA_STATE)`: the synced state selects
                // the one mutually-exclusive `AnimationState` to start, and a state change
                // `resetAnimations()` + restarts it. The sniffer's `aiStep` runs client-side for
                // remote entities, so the synced state drives the pose directly. `DATA_STATE` is
                // serialized as the `SnifferState` enum (serializer 35), so it arrives as an
                // `EnumId`, not a plain `Int`.
                if let Some(state_id) = sniffer_state_id(entity_type_id, data_values) {
                    if let Some(sniffer) = self.sniffer.as_mut() {
                        sniffer.set_state(state_id, self.age_ticks);
                    } else if sniffer_animated_state(state_id).is_some() {
                        self.sniffer = Some(SnifferAnimationState {
                            state_id,
                            keyframe: KeyframeAnimationState {
                                start_age: Some(self.age_ticks),
                            },
                        });
                    }
                }
            }
            VANILLA_ENTITY_TYPE_ARMADILLO_ID => {
                // Vanilla `Armadillo.onSyncedDataUpdated(ARMADILLO_STATE)` → `setupAnimationStates`:
                // the synced `ArmadilloState` id selects the shell-ball hide window and the
                // rollUp/rollOut/peek `AnimationState`s. The armadillo's `aiStep` runs client-side for
                // remote entities, so the synced state drives the transitions directly. We track the
                // state and the `age_ticks` at the change (for `inStateTicks`); IDLE carries no hide
                // or transition timer but must still be tracked so a later roll re-anchors correctly.
                // `ARMADILLO_STATE` is serialized as the `ArmadilloState` enum (serializer 36), so it
                // arrives as an `EnumId`, not a plain `Int`.
                if let Some((data_id, default)) =
                    vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(entity_type_id)
                {
                    let state_id = entity_data_enum_id(
                        data_values,
                        data_id,
                        EntityDataEnumSerializer::ArmadilloState,
                        default,
                    );
                    if let Some(armadillo) = self.armadillo.as_mut() {
                        armadillo.set_state(state_id, self.age_ticks);
                    } else if state_id != default {
                        self.armadillo =
                            Some(ArmadilloAnimationState::new(state_id, self.age_ticks));
                    }
                }
            }
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => {
                if let Some((data_id, default)) =
                    vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type(
                        entity_type_id,
                    )
                {
                    let phase_id = entity_data_int(data_values, data_id, default);
                    // Vanilla `Mob.isNoAi()` reads the shared mob-flags bit; the client flap
                    // advance pins `flapTime = 0.5` while it is set.
                    let no_ai =
                        vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id)
                            .is_some_and(|accessors| {
                                entity_data_byte(
                                    data_values,
                                    accessors.flags_data_id,
                                    accessors.flags_default,
                                ) & accessors.no_ai_mask
                                    != 0
                            });
                    if let Some(dragon) = self.ender_dragon.as_mut() {
                        dragon.set_phase(phase_id);
                        dragon.no_ai = no_ai;
                    } else {
                        self.ender_dragon = Some(EnderDragonAnimationState {
                            phase_id,
                            no_ai,
                            ..EnderDragonAnimationState::default()
                        });
                    }
                }
            }
            _ => {}
        }
    }

    /// Projects a client entity event into client animation state. Vanilla
    /// `ArmorStand.handleEntityEvent(32)` resets the hit wobble, and
    /// `Sheep.handleEntityEvent` resets the eat-grass animation on event `10`.
    pub(crate) fn handle_entity_event(
        &mut self,
        entity_type_id: i32,
        entity_id: i32,
        event_kind: VanillaEntityEventKind,
    ) {
        match event_kind {
            VanillaEntityEventKind::LivingKineticHit => {
                // Vanilla `LivingEntity.handleEntityEvent(2)` calls `onKineticHit()`, which
                // refreshes `lastKineticHitFeedbackTime` only when more than ten game ticks
                // elapsed since the previous accepted feedback.
                if self
                    .kinetic_hit_feedback
                    .is_none_or(|state| state.elapsed_ticks > KINETIC_HIT_FEEDBACK_RETRIGGER_TICKS)
                {
                    self.kinetic_hit_feedback =
                        Some(KineticHitFeedbackAnimationState { elapsed_ticks: 0 });
                }
            }
            VanillaEntityEventKind::ArmorStandHit => {
                // Vanilla `ArmorStand.handleEntityEvent(32)`: `lastHit = level.getGameTime()`;
                // the renderer derives `wiggle = gameTime - lastHit + partialTicks`.
                self.armor_stand_wiggle = Some(ArmorStandWiggleAnimationState { elapsed_ticks: 0 });
            }
            VanillaEntityEventKind::SheepEatGrass => {
                self.sheep_eat = Some(SheepEatAnimationState {
                    eat_animation_tick: SHEEP_EAT_ANIMATION_TICKS,
                });
            }
            VanillaEntityEventKind::MinecartTntPrime => {
                self.minecart_tnt_fuse = Some(MinecartTntFuseAnimationState {
                    fuse_remaining_ticks: TNT_MINECART_FUSE_TICKS,
                });
            }
            VanillaEntityEventKind::MinecartSpawnerDelayReset => {
                // `MinecartSpawner.handleEntityEvent` delegates every id to
                // `BaseSpawner.onEventTriggered`; id 1 on the client sets
                // `spawnDelay = minSpawnDelay`. Minecart spawners start with the BaseSpawner defaults.
                let spawner = self
                    .minecart_spawner
                    .get_or_insert(MinecartSpawnerClientState {
                        spawn_delay: 20,
                        min_spawn_delay: 200,
                    });
                spawner.spawn_delay = spawner.min_spawn_delay;
            }
            VanillaEntityEventKind::WardenTendril => {
                // Vanilla `Warden.handleEntityEvent(61)`: `tendrilAnimation = 10`. Only
                // `tendrilAnimation` (`current`) is set; `tendrilAnimationO` (`previous`)
                // is untouched, so the lerp fades in from the prior frame.
                self.warden_tendril
                    .get_or_insert(WardenTendrilAnimationState {
                        previous: 0,
                        current: 0,
                    })
                    .current = WARDEN_TENDRIL_ANIMATION_TICKS;
            }
            VanillaEntityEventKind::WardenAttack => {
                // Vanilla `Warden.handleEntityEvent(4)`: `roarAnimationState.stop()` then
                // `attackAnimationState.start(tickCount)` — the melee swing also cancels any roar.
                self.warden_combat
                    .get_or_insert_with(WardenCombatAnimationState::default)
                    .start_attack(self.age_ticks);
            }
            VanillaEntityEventKind::WardenSonicBoom => {
                // Vanilla `Warden.handleEntityEvent(62)`: `sonicBoomAnimationState.start(tickCount)`.
                self.warden_combat
                    .get_or_insert_with(WardenCombatAnimationState::default)
                    .start_sonic_boom(self.age_ticks);
            }
            VanillaEntityEventKind::SquidResetMovement => {
                // Vanilla `Squid.handleEntityEvent(19)`: `tentacleMovement = 0`. Only an
                // already-ticked squid has a state; if the event arrives first the
                // animation will start fresh on the next tick anyway, so nothing to do.
                if let Some(squid) = self.squid.as_mut() {
                    squid.reset_movement();
                }
            }
            VanillaEntityEventKind::WolfStartShaking => {
                self.wolf_wet
                    .get_or_insert(WolfWetAnimationState {
                        is_wet: false,
                        is_shaking: false,
                        previous_shake_anim: 0.0,
                        current_shake_anim: 0.0,
                    })
                    .start_shaking();
            }
            VanillaEntityEventKind::WolfCancelShaking => {
                if let Some(wolf_wet) = self.wolf_wet.as_mut() {
                    wolf_wet.cancel_shake();
                    if wolf_wet.is_settled() {
                        self.wolf_wet = None;
                    }
                }
            }
            VanillaEntityEventKind::GoatLowerHead | VanillaEntityEventKind::GoatRaiseHead => {
                // Vanilla `Goat.handleEntityEvent`: event 58 → `isLoweringHead = true`, 59 → `false`. The
                // `aiStep` counter advances from there; an idle goat with no state is created on the first
                // lower so its `lowerHeadTick` can climb (a raise event before any state is a no-op rest).
                self.goat_ramming
                    .get_or_insert(GoatRammingAnimationState {
                        lowering_head: false,
                        lower_head_tick: 0,
                    })
                    .lowering_head = event_kind == VanillaEntityEventKind::GoatLowerHead;
            }
            VanillaEntityEventKind::IronGolemAttack
            | VanillaEntityEventKind::IronGolemOfferFlower
            | VanillaEntityEventKind::IronGolemStopOfferFlower => {
                // Vanilla `IronGolem.handleEntityEvent`: event 4 → `attackAnimationTick = 10`, 11 →
                // `offerFlowerTick = 400`, 34 → `offerFlowerTick = 0`. The `aiStep` decrements both.
                let golem = self.iron_golem.get_or_insert(IronGolemAnimationState {
                    attack_animation_tick: 0,
                    offer_flower_tick: 0,
                });
                match event_kind {
                    VanillaEntityEventKind::IronGolemAttack => {
                        golem.attack_animation_tick = IRON_GOLEM_ATTACK_TICKS
                    }
                    VanillaEntityEventKind::IronGolemOfferFlower => {
                        golem.offer_flower_tick = IRON_GOLEM_OFFER_FLOWER_TICKS
                    }
                    _ => golem.offer_flower_tick = 0,
                }
            }
            VanillaEntityEventKind::RavagerAttack | VanillaEntityEventKind::RavagerStun => {
                // Vanilla `Ravager.handleEntityEvent`: event 4 → `attackTick = 10`, 39 → `stunnedTick = 40`.
                // The `aiStep` decrements them and arms the post-stun roar.
                let ravager = self
                    .ravager
                    .get_or_insert_with(|| RavagerAnimationState::new(entity_id));
                if event_kind == VanillaEntityEventKind::RavagerAttack {
                    ravager.attack_tick = RAVAGER_ATTACK_TICKS;
                } else {
                    ravager.stunned_tick = RAVAGER_STUN_TICKS;
                }
            }
            VanillaEntityEventKind::HoglinAttack | VanillaEntityEventKind::ZoglinAttack => {
                // Vanilla `Hoglin`/`Zoglin.handleEntityEvent`: event 4 → `attackAnimationRemainingTicks = 10`.
                self.hoglin
                    .get_or_insert(HoglinAnimationState {
                        attack_animation_tick: 0,
                    })
                    .attack_animation_tick = HOGLIN_ATTACK_TICKS;
            }
            VanillaEntityEventKind::RabbitJump => {
                // Vanilla `Rabbit.handleEntityEvent(1)`: seed the hop reconstruction (`jumpDuration = 15;
                // jumpTicks = 0`); the next tick's `setupAnimationStates` starts the hop keyframe.
                self.rabbit_hop
                    .get_or_insert(RabbitHopAnimationState {
                        jump_ticks: 0,
                        jump_duration: 0,
                        hop: KeyframeAnimationState { start_age: None },
                    })
                    .start_jump();
            }
            VanillaEntityEventKind::ArmadilloPeek => {
                // Vanilla `Armadillo.handleEntityEvent(64)`: set `peekReceivedClient`; the next
                // `setupAnimationStates()` consumes it by stopping the old peek and immediately
                // restarting it for the current SCARED state.
                let age_ticks = self.age_ticks;
                if let Some((_, default)) =
                    vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(entity_type_id)
                {
                    self.armadillo
                        .get_or_insert_with(|| ArmadilloAnimationState::new(default, age_ticks))
                        .peek_received_client = true;
                }
            }
            VanillaEntityEventKind::CreakingAttack
            | VanillaEntityEventKind::CreakingInvulnerable => {
                // Vanilla `Creaking.handleEntityEvent`: event 4 → `attackAnimationRemainingTicks = 15`,
                // 66 → `invulnerabilityAnimationRemainingTicks = 8`. The `aiStep` countdown advances from
                // there; the next tick's `setupAnimationStates` starts the matching keyframe one-shot.
                let creaking = self
                    .creaking
                    .get_or_insert_with(|| CreakingAnimationState::new(entity_id));
                if event_kind == VanillaEntityEventKind::CreakingAttack {
                    creaking.start_attack();
                } else {
                    creaking.start_invulnerable();
                }
            }
            VanillaEntityEventKind::EvokerFangsAttack => {
                // Vanilla `EvokerFangs.handleEntityEvent`: event 4 → `clientSideAttackStarted = true`,
                // arming the `lifeTicks` countdown that `tick` runs.
                self.evoker_fangs
                    .get_or_insert_with(|| EvokerFangsAnimationState::new(entity_id))
                    .started = true;
            }
            _ => {}
        }
    }

    /// Vanilla `Sheep.eatAnimationTick`, exposed for renderer head-pose
    /// projection. Returns `0` when the sheep is not currently eating.
    pub fn sheep_eat_animation_tick(&self) -> i32 {
        self.sheep_eat.map_or(0, |state| state.eat_animation_tick)
    }

    /// Vanilla `EquineRenderState.animateTail` (`AbstractHorse.tailCounter > 0`), exposed for
    /// renderer tail yRot projection. Returns `false` for non-equine entities and idle equines.
    pub fn equine_animate_tail(&self) -> bool {
        self.equine_tail
            .is_some_and(EquineTailAnimationState::animate_tail)
    }

    /// Vanilla `EquineRenderState.eatAnimation` (`AbstractHorse.getEatAnim(partialTick)`), driven by
    /// the synced `AbstractHorse.DATA_ID_FLAGS` eating bit. Returns `0.0` for non-equines and idle
    /// equines.
    pub fn equine_eat_animation(&self, partial_tick: f32) -> f32 {
        self.equine_pose
            .map_or(0.0, |state| state.eat_animation(partial_tick))
    }

    /// Vanilla `EquineRenderState.standAnimation` (`AbstractHorse.getStandAnim(partialTick)`), driven
    /// by the synced standing bit. Returns `0.0` for non-equines and all-fours equines.
    pub fn equine_stand_animation(&self, partial_tick: f32) -> f32 {
        self.equine_pose
            .map_or(0.0, |state| state.stand_animation(partial_tick))
    }

    /// Vanilla `EquineRenderState.feedingAnimation` (`AbstractHorse.getMouthAnim(partialTick)`),
    /// driven by the synced open-mouth bit. Returns `0.0` for non-equines and closed-mouth equines.
    pub fn equine_feeding_animation(&self, partial_tick: f32) -> f32 {
        self.equine_pose
            .map_or(0.0, |state| state.mouth_animation(partial_tick))
    }

    /// Vanilla `Goat.lowerHeadTick` (the `0..=20` ram counter), exposed for the native layer to derive
    /// `getRammingXHeadRot` (which scales it by the adult/baby max head pitch). Returns `0` when the
    /// goat is not ramming. No partial-tick lerp: vanilla `getRammingXHeadRot` reads the raw int.
    pub fn goat_lower_head_tick(&self) -> i32 {
        self.goat_ramming.map_or(0, |state| state.lower_head_tick)
    }

    /// Vanilla `IronGolemRenderState.attackTicksRemaining` (the partial-lerped `attackAnimationTick`),
    /// exposed for the renderer `IronGolemModel.setupAnim` smash arm wave. `0.0` when not attacking.
    pub fn iron_golem_attack_ticks_remaining(&self, partial_tick: f32) -> f32 {
        self.iron_golem
            .map_or(0.0, |state| state.attack_ticks_remaining(partial_tick))
    }

    /// Vanilla `IronGolemRenderState.offerFlowerTick` (the raw `offerFlowerTick`), exposed for the
    /// renderer offer-flower arm hold. `0` when the golem is not offering a poppy.
    pub fn iron_golem_offer_flower_tick(&self) -> i32 {
        self.iron_golem.map_or(0, |state| state.offer_flower_tick)
    }

    /// Vanilla `RavagerRenderState.stunnedTicksRemaining` (partial-lerped `stunnedTick`), exposed for the
    /// renderer `RavagerModel.setupAnim` head-shake stun pose. `0.0` when not stunned.
    pub fn ravager_stunned_ticks_remaining(&self, partial_tick: f32) -> f32 {
        self.ravager
            .map_or(0.0, |state| state.stunned_ticks_remaining(partial_tick))
    }

    /// Vanilla `RavagerRenderState.attackTicksRemaining` (partial-lerped `attackTick`), exposed for the
    /// renderer ravager neck-lunge / mouth-open bite pose. `0.0` when not attacking.
    pub fn ravager_attack_ticks_remaining(&self, partial_tick: f32) -> f32 {
        self.ravager
            .map_or(0.0, |state| state.attack_ticks_remaining(partial_tick))
    }

    /// Vanilla `RavagerRenderState.roarAnimation` (the `0..1` roar ramp), exposed for the renderer
    /// ravager mouth-open roar pose. `0.0` when not roaring.
    pub fn ravager_roar_animation(&self, partial_tick: f32) -> f32 {
        self.ravager
            .map_or(0.0, |state| state.roar_animation(partial_tick))
    }

    /// Vanilla `HoglinRenderState.attackAnimationRemainingTicks` (the RAW `attackAnimationRemainingTicks`,
    /// not partial-lerped), exposed for the renderer `HoglinModel.setupAnim` headbutt. `0` when the
    /// hoglin / zoglin is not mid-headbutt.
    pub fn hoglin_attack_animation_tick(&self) -> i32 {
        self.hoglin.map_or(0, |state| state.attack_animation_tick)
    }

    /// The rabbit hop's elapsed seconds since its `hopAnimationState` started (vanilla
    /// `Rabbit.hopAnimationState`, the 0.75s looping `RabbitAnimation.HOP`), projected for the
    /// renderer `RabbitModel.setupAnim` `hopAnimation.apply`. Returns `-1.0` (the stopped-animation
    /// sentinel) for a resting rabbit and every other entity; a non-negative value is wrapped by the
    /// looping def length each frame in the renderer.
    pub fn rabbit_hop_seconds(&self, partial_tick: f32) -> f32 {
        self.rabbit_hop
            .and_then(|state| state.hop.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `Bat.flyAnimationState` elapsed seconds, started by `Bat.setupAnimationStates` on the
    /// first flying client tick and stopped while the bat is resting. Returns `-1.0` before that
    /// first tick, while resting, and for every non-bat entity.
    pub fn bat_flying_seconds(&self, partial_tick: f32) -> f32 {
        self.bat_flying
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The creaking attack's elapsed seconds since entity event `4` started it (vanilla
    /// `attackAnimationState`), projected for the renderer `CreakingModel.setupAnim`
    /// `attackAnimation.apply`. Returns `-1.0` (the stopped-animation sentinel) for a non-attacking
    /// creaking and every other entity; a non-negative value is wrapped by the 0.7083s looping length
    /// each frame in the renderer.
    pub fn creaking_attack_seconds(&self, partial_tick: f32) -> f32 {
        self.creaking
            .and_then(|state| state.attack.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The creaking invulnerable stagger's elapsed seconds since entity event `66` started it (vanilla
    /// `invulnerabilityAnimationState`), projected for the renderer `CreakingModel.setupAnim`
    /// `invulnerableAnimation.apply`. Returns `-1.0` (stopped) for a non-staggering creaking and every
    /// other entity; a non-negative value clamps past the 0.2917s length to its final frame.
    pub fn creaking_invulnerable_seconds(&self, partial_tick: f32) -> f32 {
        self.creaking
            .and_then(|state| state.invuln.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The creaking death collapse's elapsed seconds since `isTearingDown()` became true (vanilla
    /// `deathAnimationState`), projected for the renderer `CreakingModel.setupAnim`
    /// `deathAnimation.apply`. Returns `-1.0` (stopped) for a non-tearing-down creaking and every
    /// other entity; a non-negative value clamps past the 2.25s length to its final frame.
    pub fn creaking_death_seconds(&self, partial_tick: f32) -> f32 {
        self.creaking
            .and_then(|state| state.death.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `Creaking.hasGlowingEyes()`, the client-ticked teardown flicker bit consumed by
    /// `CreakingRenderer` instead of synced `isActive()` while `isTearingDown()` holds. The exact
    /// process-local RNG seed is not on the wire; the state preserves Vanilla's Java RNG shape,
    /// inclusive delay ranges, death-time schedule, and toggle ordering with a stable local seed.
    pub fn creaking_eyes_glowing(&self) -> bool {
        self.creaking
            .is_some_and(CreakingAnimationState::eyes_glowing)
    }

    /// Vanilla `Warden.getTendrilAnimation(partialTick)`, exposed for the renderer
    /// `WardenModel.animateTendrils` sway. Returns `0.0` when the entity is not a
    /// warden with an active tendril pulse.
    pub fn warden_tendril_animation(&self, partial_tick: f32) -> f32 {
        self.warden_tendril
            .map_or(0.0, |state| state.tendril_animation(partial_tick))
    }

    /// Vanilla `Warden.getHeartAnimation(partialTick)`, exposed for the renderer
    /// warden heart emissive overlay's alpha. Returns `0.0` when the entity is not a
    /// warden (or before its first client tick spins up the heartbeat).
    pub fn warden_heart_animation(&self, partial_tick: f32) -> f32 {
        self.warden_heart
            .map_or(0.0, |state| state.heart_animation(partial_tick))
    }

    /// The warden roar's elapsed seconds since `Pose.ROARING` started (vanilla
    /// `roarAnimationState`'s `getTimeInMillis`/`getElapsedSeconds`), projected for the renderer
    /// `WardenModel.setupAnim` `roarAnimation.apply`. Returns `-1.0` (the stopped-animation
    /// sentinel) for a non-roaring warden and every other entity, so the renderer applies no
    /// `WARDEN_ROAR` keyframe; a non-negative value is clamped past the 4.2s length to its final
    /// frame (vanilla's "hold the last frame").
    pub fn warden_roar_seconds(&self, partial_tick: f32) -> f32 {
        self.warden_combat
            .and_then(|state| state.roar.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The warden sniff's elapsed seconds since `Pose.SNIFFING` started (vanilla
    /// `sniffAnimationState`), projected for the renderer `WardenModel.setupAnim`
    /// `sniffAnimation.apply`. Returns `-1.0` (stopped) for a non-sniffing warden and every other
    /// entity; a non-negative value clamps past the 4.16s length to its final frame.
    pub fn warden_sniff_seconds(&self, partial_tick: f32) -> f32 {
        self.warden_combat
            .and_then(|state| state.sniff.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The warden attack's elapsed seconds since entity event `4` started it (vanilla
    /// `attackAnimationState`), projected for the renderer `WardenModel.setupAnim`
    /// `attackAnimation.apply`. Returns `-1.0` (stopped) for a non-attacking warden and every other
    /// entity; a non-negative value clamps past the 0.33333s length to its final frame.
    pub fn warden_attack_seconds(&self, partial_tick: f32) -> f32 {
        self.warden_combat
            .and_then(|state| state.attack.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The warden sonic boom's elapsed seconds since entity event `62` started it (vanilla
    /// `sonicBoomAnimationState`), projected for the renderer `WardenModel.setupAnim`
    /// `sonicBoomAnimation.apply`. Returns `-1.0` (stopped) for a non-booming warden and every other
    /// entity; a non-negative value clamps past the 3.0s length to its final frame.
    pub fn warden_sonic_boom_seconds(&self, partial_tick: f32) -> f32 {
        self.warden_combat
            .and_then(|state| {
                state
                    .sonic_boom
                    .elapsed_seconds(self.age_ticks, partial_tick)
            })
            .unwrap_or(-1.0)
    }

    /// The warden emerge's elapsed seconds since `Pose.EMERGING` started (vanilla
    /// `emergeAnimationState`), projected for the renderer `WardenModel.setupAnim`
    /// `emergeAnimation.apply`. Returns `-1.0` (stopped) for a non-emerging warden and every other
    /// entity; a non-negative value clamps past the 6.68s length to its final frame.
    pub fn warden_emerge_seconds(&self, partial_tick: f32) -> f32 {
        self.warden_combat
            .and_then(|state| state.emerge.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The warden dig's elapsed seconds since `Pose.DIGGING` started (vanilla
    /// `diggingAnimationState`), projected for the renderer `WardenModel.setupAnim`
    /// `diggingAnimation.apply`. Returns `-1.0` (stopped) for a non-digging warden and every other
    /// entity; a non-negative value clamps past the 5.0s length to its final frame.
    pub fn warden_dig_seconds(&self, partial_tick: f32) -> f32 {
        self.warden_combat
            .and_then(|state| state.dig.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `Creeper.getSwelling(partialTick)`, exposed for the renderer white
    /// swelling overlay. Returns `0.0` when the entity is not a priming creeper.
    pub fn creeper_swelling(&self, partial_tick: f32) -> f32 {
        self.creeper_swell
            .map_or(0.0, |state| state.swelling(partial_tick))
    }

    /// Vanilla `MinecartTntRenderState.fuseRemainingInTicks`: `MinecartTNT.getFuse() - partialTick
    /// + 1.0`, or `-1.0` while the cart is not primed. `TntMinecartRenderer` consumes this for the
    /// TNT display-block scale pulse and white overlay strobe.
    pub fn minecart_tnt_fuse_remaining_in_ticks(&self, partial_tick: f32) -> f32 {
        self.minecart_tnt_fuse
            .map_or(-1.0, |state| state.fuse_remaining_in_ticks(partial_tick))
    }

    pub fn minecart_spawner_spawn_delay(&self) -> i32 {
        self.minecart_spawner.map_or(20, |state| state.spawn_delay)
    }

    /// Vanilla `Shulker.getClientPeekAmount(partialTick)`, exposed for the renderer
    /// `ShulkerModel.setupAnim` lid open/close. Returns `0.0` (the closed/bind pose) when
    /// the entity is not a shulker or its lid is shut.
    pub fn shulker_peek_amount(&self, partial_tick: f32) -> f32 {
        self.shulker_peek
            .map_or(0.0, |state| state.peek_amount(partial_tick))
    }

    pub(crate) fn record_shulker_position_change(
        &mut self,
        old_position: EntityVec3,
        current_position: EntityVec3,
        is_passenger: bool,
    ) -> bool {
        if self.age_ticks == 0 || is_passenger {
            return false;
        }
        let old_block = entity_block_position(old_position);
        let current_block = entity_block_position(current_position);
        if old_block == current_block {
            return false;
        }
        let state = self
            .shulker_teleport
            .get_or_insert(ShulkerTeleportAnimationState {
                old_attach_position: None,
                interpolation_ticks: 0,
            });
        if state.old_attach_position == Some(current_block) {
            return false;
        }
        state.old_attach_position = Some(old_block);
        state.interpolation_ticks = 6;
        true
    }

    pub(crate) fn shulker_started_riding(&mut self) {
        self.shulker_teleport = None;
    }

    pub(crate) fn shulker_stopped_riding(&mut self, position: EntityVec3) {
        self.shulker_teleport = Some(ShulkerTeleportAnimationState {
            old_attach_position: Some(entity_block_position(position)),
            interpolation_ticks: 0,
        });
    }

    pub(crate) fn shulker_render_offset(
        &self,
        current_position: EntityVec3,
        partial_tick: f32,
        scale: f32,
    ) -> Option<EntityVec3> {
        let state = self.shulker_teleport?;
        let old = state.old_attach_position?;
        if state.interpolation_ticks == 0 {
            return None;
        }
        let current = entity_block_position(current_position);
        let mut progress = (f32::from(state.interpolation_ticks) - partial_tick) / 6.0;
        progress *= progress;
        let progress = f64::from(progress * scale);
        Some(EntityVec3 {
            x: -f64::from(current[0] - old[0]) * progress,
            y: -f64::from(current[1] - old[1]) * progress,
            z: -f64::from(current[2] - old[2]) * progress,
        })
    }

    /// Vanilla `Bee.getRollAmount(partialTick)` projected for the renderer
    /// `BeeModel.setupAnim` barrel-roll flip. Returns `0.0` when the entity is
    /// not a rolling bee.
    pub fn bee_roll_amount(&self, partial_tick: f32) -> f32 {
        self.bee_roll
            .map_or(0.0, |state| state.roll_amount(partial_tick))
    }

    /// Vanilla `Panda.getSitAmount(partialTick)`, exposed for
    /// `PandaRenderState.sitAmount` and `PandaModel.setupAnim`. Returns `0.0`
    /// for a standing panda and every non-panda entity.
    pub fn panda_sit_amount(&self, partial_tick: f32) -> f32 {
        self.panda
            .map_or(0.0, |state| state.sit_amount(partial_tick))
    }

    /// Vanilla `Panda.getLieOnBackAmount(partialTick)`, exposed for
    /// `PandaRenderState.lieOnBackAmount` and `PandaModel.setupAnim`. Returns
    /// `0.0` for a panda not lying on its back and every non-panda entity.
    pub fn panda_lie_on_back_amount(&self, partial_tick: f32) -> f32 {
        self.panda
            .map_or(0.0, |state| state.lie_on_back_amount(partial_tick))
    }

    /// Vanilla `Panda.getRollAmount(partialTick)`, exposed for the adult panda
    /// model roll pose. The vanilla renderer zeroes this for baby pandas while
    /// still using [`Self::panda_roll_time`] for the whole-model tumble.
    pub fn panda_roll_amount(&self, partial_tick: f32) -> f32 {
        self.panda
            .map_or(0.0, |state| state.roll_amount(partial_tick))
    }

    /// Vanilla `PandaRenderState.rollTime = rollCounter > 0 ? rollCounter +
    /// partialTick : 0`, used by `PandaRenderer.setupRotations` for the
    /// whole-model tumble. Returns `0.0` outside an active roll.
    pub fn panda_roll_time(&self, partial_tick: f32) -> f32 {
        self.panda
            .map_or(0.0, |state| state.roll_time(partial_tick))
    }

    /// Vanilla `LivingEntity.getSwimAmount(partialTick)`: the 0..1 eased blend toward
    /// `isVisuallySwimming()`, used today by the drowned render path for the swim body
    /// pitch and limb overrides. Returns `0.0` for a living entity that is not swimming
    /// and for every non-living entity.
    pub fn living_swim_amount(&self, partial_tick: f32) -> f32 {
        self.living_swim
            .map_or(0.0, |state| state.swim_amount(partial_tick))
    }

    /// Vanilla `HumanoidRenderState.elytraRotX` =
    /// `LivingEntity.elytraAnimationState.getRotX(partialTick)`. A source row with
    /// no ticked state preserves the renderer's steady non-flying default.
    pub fn elytra_rot_x(&self, partial_tick: f32) -> f32 {
        self.elytra
            .map_or(ELYTRA_DEFAULT_X_ROT, |state| state.rot_x(partial_tick))
    }

    /// Vanilla `HumanoidRenderState.elytraRotY` =
    /// `LivingEntity.elytraAnimationState.getRotY(partialTick)`.
    pub fn elytra_rot_y(&self, partial_tick: f32) -> f32 {
        self.elytra
            .map_or(ELYTRA_DEFAULT_Y_ROT, |state| state.rot_y(partial_tick))
    }

    /// Vanilla `HumanoidRenderState.elytraRotZ` =
    /// `LivingEntity.elytraAnimationState.getRotZ(partialTick)`.
    pub fn elytra_rot_z(&self, partial_tick: f32) -> f32 {
        self.elytra
            .map_or(ELYTRA_DEFAULT_Z_ROT, |state| state.rot_z(partial_tick))
    }

    /// Vanilla `AvatarRenderer.extractCapeState`: the player cloak lag projected
    /// into `AvatarRenderState.capeFlap`, `capeLean`, and `capeLean2`.
    pub fn player_cape_state(&self, partial_tick: f32, y_body_rot: f32) -> (f32, f32, f32) {
        self.player_cloak.map_or((0.0, 0.0, 0.0), |state| {
            state.cape_state(partial_tick, y_body_rot)
        })
    }

    /// Vanilla `ClientAvatarState` values copied into the active player
    /// camera's render state for `GameRenderer.bobView`.
    pub fn player_camera_view_bob(&self, partial_tick: f32) -> EntityCameraViewBobState {
        self.player_cloak.map_or_else(Default::default, |state| {
            state.camera_view_bob(partial_tick)
        })
    }

    /// Vanilla `AvatarRenderState.fallFlyingTimeInTicks`.
    pub fn player_fall_flying_time(&self, partial_tick: f32) -> f32 {
        self.player_cloak.map_or(partial_tick, |state| {
            state.fall_flying_ticks as f32 + partial_tick
        })
    }

    /// Vanilla `ClientAvatarState.deltaMovementOnPreviousTick`, used only for
    /// the avatar fall-flying yaw interpolation.
    pub fn player_previous_delta_movement(&self) -> EntityVec3 {
        self.player_cloak.map_or(EntityVec3::default(), |state| {
            state.delta_movement_on_previous_tick
        })
    }

    /// The frog croak's elapsed seconds since `Pose.CROAKING` started (vanilla
    /// `croakAnimationState`'s `getTimeInMillis`/`getElapsedSeconds`), projected for
    /// `FrogModel.setupAnim`. Returns `-1.0` (the stopped-animation sentinel) for a
    /// non-croaking frog and every other entity, so the renderer hides the
    /// `croaking_body` pouch and applies no `FROG_CROAK` keyframe; the renderer wraps
    /// a non-negative value by the 3.0s length before sampling.
    pub fn frog_croak_seconds(&self, partial_tick: f32) -> f32 {
        self.frog_croak
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The frog tongue's elapsed seconds since `Pose.USING_TONGUE` started (vanilla
    /// `tongueAnimationState`'s `getTimeInMillis`/`getElapsedSeconds`), projected for
    /// `FrogModel.setupAnim`. Returns `-1.0` (the stopped-animation sentinel) for a frog that is not
    /// using its tongue and every other entity, so the renderer applies no `FROG_TONGUE` keyframe;
    /// the renderer wraps a non-negative value by the 0.5s length before sampling.
    pub fn frog_tongue_seconds(&self, partial_tick: f32) -> f32 {
        self.frog_tongue
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The frog jump's elapsed seconds since `Pose.LONG_JUMPING` started (vanilla
    /// `jumpAnimationState`'s `getTimeInMillis`/`getElapsedSeconds`), projected for
    /// `FrogModel.setupAnim`. Returns `-1.0` (the stopped-animation sentinel) for a
    /// non-jumping frog and every other entity, so the renderer applies no `FROG_JUMP`
    /// keyframe; a non-negative value is sampled against the 0.5s definition.
    pub fn frog_jump_seconds(&self, partial_tick: f32) -> f32 {
        self.frog_jump
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The frog swim-idle's elapsed seconds since it started (vanilla
    /// `swimIdleAnimationState`'s `getTimeInMillis`/`getElapsedSeconds`), projected
    /// for `FrogModel.setupAnim`. Unlike the pose-driven croak/jump, the swim-idle is
    /// driven each client tick from `isInWater() && !walkAnimation.isMoving()`.
    /// Returns `-1.0` (the stopped-animation sentinel) for a frog that is out of water
    /// or moving and for every other entity, so the renderer applies no
    /// `FROG_IDLE_WATER` keyframe; a non-negative value is sampled (looping) against
    /// the 3.0s definition.
    pub fn frog_swim_idle_seconds(&self, partial_tick: f32) -> f32 {
        self.frog_swim_idle
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `Camel.dashAnimationState` elapsed seconds (the looping `CAMEL_DASH` gallop), exposed
    /// for `CamelModel.setupAnim`. Returns `-1.0` (the stopped-animation sentinel) for every non-camel
    /// entity and a camel that is not dashing.
    pub fn camel_dash_seconds(&self, partial_tick: f32) -> f32 {
        self.camel_dash
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `Camel.idleAnimationState` elapsed seconds. `Camel.setupAnimationStates` restarts it
    /// every `random.nextInt(40) + 80` client ticks and never stops it, so a ticking camel always
    /// returns a non-negative sample while non-camels use the stopped-animation sentinel.
    pub fn camel_idle_seconds(&self, partial_tick: f32) -> f32 {
        self.camel_idle
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `CopperGolem.idleAnimationState` elapsed seconds. The client starts it only on the
    /// delayed `idleAnimationStartTick` while `COPPER_GOLEM_STATE == IDLE`; non-copper golems and
    /// interaction states return `-1.0` so the renderer applies no idle keyframe.
    pub fn copper_golem_idle_seconds(&self, partial_tick: f32) -> f32 {
        self.copper_golem_idle
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `CopperGolem.interactionGetItemAnimationState` elapsed seconds, started while
    /// `COPPER_GOLEM_STATE == GETTING_ITEM` and stopped by every other state.
    pub fn copper_golem_get_item_seconds(&self, partial_tick: f32) -> f32 {
        self.copper_golem_get_item
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `CopperGolem.interactionGetNoItemAnimationState` elapsed seconds, started while
    /// `COPPER_GOLEM_STATE == GETTING_NO_ITEM` and stopped by every other state.
    pub fn copper_golem_get_no_item_seconds(&self, partial_tick: f32) -> f32 {
        self.copper_golem_get_no_item
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `CopperGolem.interactionDropItemAnimationState` elapsed seconds, started while
    /// `COPPER_GOLEM_STATE == DROPPING_ITEM` and stopped by every other state.
    pub fn copper_golem_drop_item_seconds(&self, partial_tick: f32) -> f32 {
        self.copper_golem_drop_item
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `CopperGolem.interactionDropNoItemAnimationState` elapsed seconds, started while
    /// `COPPER_GOLEM_STATE == DROPPING_NO_ITEM` and stopped by every other state.
    pub fn copper_golem_drop_no_item_seconds(&self, partial_tick: f32) -> f32 {
        self.copper_golem_drop_no_item
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `CamelRenderState.jumpCooldown` =
    /// `max(Camel.getJumpCooldown() - partialTicks, 0)`. The cooldown is seeded from the synced
    /// `DASH` rising edge and decremented each client tick, matching `Camel.tick`.
    pub fn camel_jump_cooldown(self, partial_tick: f32) -> f32 {
        self.camel_dash_cooldown
            .map(|state| state.jump_cooldown(partial_tick))
            .unwrap_or(0.0)
    }

    pub(crate) fn nautilus_jump_cooldown(self, partial_tick: f32) -> f32 {
        self.nautilus_dash_cooldown
            .map(|state| state.cooldown(partial_tick))
            .unwrap_or(0.0)
    }

    /// Vanilla `AllayRenderState.isDancing` (`Allay.isDancing()`): gates `AllayModel.setupAnim`'s dance
    /// branch (the body spin and head/body sway) against the plain head-look branch. `false` for every
    /// non-allay entity and an allay that is not dancing.
    pub fn allay_is_dancing(&self) -> bool {
        self.allay_dance.is_some_and(|state| state.is_dancing())
    }

    /// Vanilla `AllayRenderState.isSpinning` (`Allay.isSpinning()`): whether the allay is in the
    /// spinning phase of its dance (the body whirls `4π·spinningProgress`). `false` otherwise.
    pub fn allay_is_spinning(&self) -> bool {
        self.allay_dance
            .is_some_and(|state| state.is_dancing() && state.is_spinning())
    }

    /// Vanilla `AllayRenderState.spinningProgress` (`Allay.getSpinningProgress(partialTick)`): the
    /// `0..1` spin ramp blending the body spin in and out. `0.0` when the allay is not dancing.
    pub fn allay_spinning_progress(&self, partial_tick: f32) -> f32 {
        self.allay_dance
            .map_or(0.0, |state| state.spinning_progress(partial_tick))
    }

    /// Vanilla `AllayRenderState.holdingAnimationProgress`
    /// (`Allay.getHoldingItemAnimationProgress(partialTick)`): the `0..1` arm-raise
    /// blend driven by whether the synced main hand is non-empty.
    pub fn allay_holding_item_progress(&self, partial_tick: f32) -> f32 {
        self.allay_holding_item
            .map_or(0.0, |state| state.holding_item_progress(partial_tick))
    }

    /// Vanilla render-state `ticksUsingItem` (`getTicksUsingItem(partialTicks)`) reconstructed from the
    /// charge/use counter. Used by crossbow `CROSSBOW_CHARGE` poses and player spear kinetic use sway.
    /// `0.0` when the tracked charge/use bit is down.
    pub fn crossbow_charge_ticks_using_item(&self, partial_tick: f32) -> f32 {
        self.crossbow_charge
            .map_or(0.0, |state| state.ticks_using_item(partial_tick))
    }

    /// Vanilla `AxolotlRenderState.playingDeadFactor` (`Axolotl.playingDeadAnimator.getFactor`): the
    /// `0..1` eased blend into `AdultAxolotlModel.setupPlayDeadAnimation`. `0.0` for an awake axolotl
    /// and every other entity.
    pub fn axolotl_playing_dead_factor(&self, partial_tick: f32) -> f32 {
        self.axolotl
            .map_or(0.0, |state| state.playing_dead_factor(partial_tick))
    }

    /// Vanilla `AxolotlRenderState.inWaterFactor` (`Axolotl.inWaterAnimator.getFactor`): the `0..1`
    /// eased blend gating the swimming / water-hovering sub-animations. `0.0` for a grounded axolotl
    /// and every other entity.
    pub fn axolotl_in_water_factor(&self, partial_tick: f32) -> f32 {
        self.axolotl
            .map_or(0.0, |state| state.in_water_factor(partial_tick))
    }

    /// Vanilla `AxolotlRenderState.onGroundFactor` (`Axolotl.onGroundAnimator.getFactor`): the `0..1`
    /// eased blend gating the ground-crawling / lay-still sub-animations. `0.0` for a swimming axolotl
    /// and every other entity.
    pub fn axolotl_on_ground_factor(&self, partial_tick: f32) -> f32 {
        self.axolotl
            .map_or(0.0, |state| state.on_ground_factor(partial_tick))
    }

    /// Vanilla `AxolotlRenderState.movingFactor` (`Axolotl.movingAnimator.getFactor`): the `0..1`
    /// eased blend separating the moving sub-animations (swim, crawl) from the still ones (hover,
    /// lay-still) and gating the mirror-leg copy. `0.0` for a still axolotl and every other entity.
    pub fn axolotl_moving_factor(&self, partial_tick: f32) -> f32 {
        self.axolotl
            .map_or(0.0, |state| state.moving_factor(partial_tick))
    }

    /// Elapsed seconds for Vanilla's baby `swimAnimationState`, or `-1` while stopped.
    pub fn axolotl_baby_swim_seconds(&self, partial_tick: f32) -> f32 {
        self.axolotl.map_or(-1.0, |state| {
            AxolotlAnimationState::baby_animation_seconds(
                state.baby_swim,
                self.age_ticks,
                partial_tick,
            )
        })
    }

    /// Elapsed seconds for Vanilla's baby `walkAnimationState`, or `-1` while stopped.
    pub fn axolotl_baby_walk_seconds(&self, partial_tick: f32) -> f32 {
        self.axolotl.map_or(-1.0, |state| {
            AxolotlAnimationState::baby_animation_seconds(
                state.baby_walk,
                self.age_ticks,
                partial_tick,
            )
        })
    }

    /// Elapsed seconds for Vanilla's baby `walkUnderWaterAnimationState`, or `-1` while stopped.
    pub fn axolotl_baby_walk_underwater_seconds(&self, partial_tick: f32) -> f32 {
        self.axolotl.map_or(-1.0, |state| {
            AxolotlAnimationState::baby_animation_seconds(
                state.baby_walk_underwater,
                self.age_ticks,
                partial_tick,
            )
        })
    }

    /// Elapsed seconds for Vanilla's baby `idleUnderWaterAnimationState`, or `-1` while stopped.
    pub fn axolotl_baby_idle_underwater_seconds(&self, partial_tick: f32) -> f32 {
        self.axolotl.map_or(-1.0, |state| {
            AxolotlAnimationState::baby_animation_seconds(
                state.baby_idle_underwater,
                self.age_ticks,
                partial_tick,
            )
        })
    }

    /// Elapsed seconds for Vanilla's baby `idleUnderWaterOnGroundAnimationState`, or `-1` while
    /// stopped.
    pub fn axolotl_baby_idle_underwater_on_ground_seconds(&self, partial_tick: f32) -> f32 {
        self.axolotl.map_or(-1.0, |state| {
            AxolotlAnimationState::baby_animation_seconds(
                state.baby_idle_underwater_on_ground,
                self.age_ticks,
                partial_tick,
            )
        })
    }

    /// Elapsed seconds for Vanilla's baby `idleOnGroundAnimationState`, or `-1` while stopped.
    pub fn axolotl_baby_idle_on_ground_seconds(&self, partial_tick: f32) -> f32 {
        self.axolotl.map_or(-1.0, |state| {
            AxolotlAnimationState::baby_animation_seconds(
                state.baby_idle_on_ground,
                self.age_ticks,
                partial_tick,
            )
        })
    }

    /// Elapsed seconds for Vanilla's baby `playDeadAnimationState`, or `-1` while stopped.
    pub fn axolotl_baby_play_dead_seconds(&self, partial_tick: f32) -> f32 {
        self.axolotl.map_or(-1.0, |state| {
            AxolotlAnimationState::baby_animation_seconds(
                state.baby_play_dead,
                self.age_ticks,
                partial_tick,
            )
        })
    }

    /// The sniffer's active `Sniffer.State` animation (vanilla `Sniffer.onSyncedDataUpdated`'s
    /// one-shot `AnimationState`s) projected for `SnifferModel.setupAnim`: the `(state ordinal,
    /// elapsed seconds)` of the running triggered keyframe, or `(-1, -1.0)` when the sniffer is
    /// idling/searching or is any other entity. The renderer matches the id to pick the keyframe
    /// def (DIG/LONGSNIFF/STAND_UP/HAPPY/SNIFFSNIFF) and samples it at the elapsed seconds.
    pub fn sniffer_animation(&self, partial_tick: f32) -> (i32, f32) {
        self.sniffer.map_or((-1, -1.0), |state| {
            state.animation(self.age_ticks, partial_tick)
        })
    }

    /// Vanilla `SnifferRenderState.isSearching` (`Sniffer.isSearching()`): whether the sniffer's
    /// synced `DATA_STATE` is `SEARCHING`, gating `SnifferModel.setupAnim`'s swap of the base walk for
    /// the looping `SNIFFER_SNIFF_SEARCH` search-walk. `false` for every other state and entity.
    pub fn sniffer_is_searching(&self) -> bool {
        self.sniffer.is_some_and(|state| state.is_searching())
    }

    /// Vanilla `Armadillo.shouldHideInShell()` projected for the renderer `isHidingInShell` shell-ball
    /// swap: `true` for the steady SCARED state and for the `inStateTicks`-gated ROLLING/UNROLLING
    /// windows (rolling hides after tick 5, unrolling un-hides at tick 26). `false` for an IDLE
    /// armadillo and every other entity (only the armadillo is given a roll animation state).
    pub fn armadillo_is_hiding_in_shell(&self) -> bool {
        self.armadillo
            .is_some_and(|state| state.is_hiding_in_shell(self.age_ticks))
    }

    /// The armadillo's `ARMADILLO_ROLL_UP` elapsed seconds (vanilla `rollUpAnimationState`, started
    /// on entry to ROLLING), projected for `ArmadilloModel.setupAnim`. Returns `-1.0` (the
    /// stopped-animation sentinel) when no roll-up is running and for every other entity; the
    /// renderer applies no `ARMADILLO_ROLL_UP` keyframe for a negative value.
    pub fn armadillo_roll_up_seconds(&self, partial_tick: f32) -> f32 {
        self.armadillo
            .and_then(|state| state.roll_up.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The armadillo's `ARMADILLO_ROLL_OUT` elapsed seconds (vanilla `rollOutAnimationState`, started
    /// on entry to UNROLLING), projected for `ArmadilloModel.setupAnim`. Returns `-1.0` when no
    /// roll-out is running and for every other entity.
    pub fn armadillo_roll_out_seconds(&self, partial_tick: f32) -> f32 {
        self.armadillo
            .and_then(|state| state.roll_out.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The armadillo's `ARMADILLO_PEEK` elapsed seconds (vanilla `peekAnimationState`), including the
    /// first SCARED setup tick's `fastForward(50, 1.0F)` baseline and entity event `64` restart. `-1.0`
    /// when no peek is running and for every other entity.
    pub fn armadillo_peek_seconds(&self, partial_tick: f32) -> f32 {
        self.armadillo
            .and_then(|state| state.peek_elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `Fox.getHeadRollAngle(partialTick)` projected for the renderer
    /// `FoxModel.setWalkingPose` head tilt. Returns `0.0` when the entity is not an
    /// interested fox.
    pub fn fox_head_roll_angle(&self, partial_tick: f32) -> f32 {
        self.fox
            .map_or(0.0, |state| state.head_roll_angle(partial_tick))
    }

    /// Vanilla `Fox.getCrouchAmount(partialTick)` projected for the renderer
    /// `FoxModel.setCrouchingPose` body drop. Returns `0.0` when the entity is not a
    /// crouching fox.
    pub fn fox_crouch_amount(&self, partial_tick: f32) -> f32 {
        self.fox
            .map_or(0.0, |state| state.crouch_amount(partial_tick))
    }

    /// Vanilla `Cat.getLieDownAmount(partialTick)` projected for
    /// `FelineRenderState.lieDownAmount`. Returns `0.0` for upright cats and every non-cat.
    pub fn feline_lie_down_amount(&self, partial_tick: f32) -> f32 {
        self.cat_lie_down
            .map_or(0.0, |state| state.lie_down_amount(partial_tick))
    }

    /// Vanilla `Cat.getLieDownAmountTail(partialTick)` projected for the feline tail lie-down pose.
    /// Returns `0.0` for upright cats and every non-cat.
    pub fn feline_lie_down_amount_tail(&self, partial_tick: f32) -> f32 {
        self.cat_lie_down
            .map_or(0.0, |state| state.lie_down_amount_tail(partial_tick))
    }

    /// Vanilla `Cat.getRelaxStateOneAmount(partialTick)` projected for the relaxed head pitch.
    /// Returns `0.0` for non-relaxing cats and every non-cat.
    pub fn feline_relax_state_one_amount(&self, partial_tick: f32) -> f32 {
        self.cat_lie_down
            .map_or(0.0, |state| state.relax_state_one_amount(partial_tick))
    }

    /// Vanilla `WolfRenderState.wetShade` (`Wolf.getWetShade(partialTick)`): the base
    /// model tint multiplier used by `WolfRenderer.getModelTint`. Returns `1.0` when
    /// the wolf is dry, the `0.75` wet floor while submerged, and lerps back to white
    /// over the shake/drying timer. `1.0` for every non-wolf entity.
    pub fn wolf_wet_shade(&self, partial_tick: f32) -> f32 {
        self.wolf_wet
            .map_or(1.0, |state| state.wet_shade(partial_tick))
    }

    /// Vanilla `WolfRenderState.shakeAnim` (`Wolf.getShakeAnim(partialTick)`): the
    /// partial-lerped water-shake timer that `getBodyRollAngle(offset)` uses for
    /// the body / mane / tail roll. Returns `0.0` for a dry wolf and every non-wolf.
    pub fn wolf_shake_anim(&self, partial_tick: f32) -> f32 {
        self.wolf_wet
            .map_or(0.0, |state| state.shake_anim(partial_tick))
    }

    /// Vanilla `Wolf.die`: clear the client-only wet/shaking interpolation
    /// before delegating to `LivingEntity.die`.
    pub(crate) fn clear_wolf_wet_on_death(&mut self) {
        self.wolf_wet = None;
    }

    /// Vanilla `WolfRenderState.headRollAngle` (`Wolf.getHeadRollAngle(partialTick)`):
    /// the lerped interested-angle accumulator scaled by `0.15π`. Returns `0.0`
    /// when the wolf is not begging/interested and every non-wolf entity.
    pub fn wolf_head_roll_angle(&self, partial_tick: f32) -> f32 {
        self.wolf_head_roll
            .map_or(0.0, |state| state.head_roll_angle(partial_tick))
    }

    /// Vanilla `ArrowRenderState.shake` =
    /// `AbstractArrow.shakeTime - partialTick`, consumed by `ArrowModel.setupAnim`
    /// for the impact wobble. `0.0` for arrows that are not currently shaking and
    /// every non-arrow entity.
    pub fn arrow_shake(&self, partial_tick: f32) -> f32 {
        self.arrow_shake
            .map_or(0.0, |state| state.shake(partial_tick))
    }

    /// Vanilla `AbstractBoat.getRowingTime(side, partialTick)`: lerps the
    /// per-side paddle accumulator for a boat that still has an active paddle
    /// state and a controlling passenger. Returns `[left, right]`, or zeros for a
    /// boat at rest and every non-boat entity.
    pub fn boat_rowing_times(
        &self,
        partial_tick: f32,
        left_active: bool,
        right_active: bool,
    ) -> [f32; 2] {
        self.boat.map_or([0.0; 2], |state| {
            state.rowing_times(partial_tick, left_active, right_active)
        })
    }

    /// Vanilla `BoatRenderState.hurtTime/hurtDir/damageTime`: the client-decayed
    /// `VehicleEntity` damage-roll values, with `damageTime` clamped at zero after
    /// subtracting partial tick.
    pub fn boat_damage_state(&self, partial_tick: f32) -> (f32, i32, f32) {
        self.boat_damage
            .map_or((0.0, 1, 0.0), |state| state.render_state(partial_tick))
    }

    /// Vanilla `BoatRenderState.bubbleAngle` (`AbstractBoat.getBubbleAngle(partialTick)`):
    /// the partial-lerped bubble-column wobble angle in degrees.
    pub fn boat_bubble_angle(&self, partial_tick: f32) -> f32 {
        self.boat_bubble
            .map_or(0.0, |state| state.angle(partial_tick))
    }

    /// Vanilla `WitherRenderState.xHeadRots/yHeadRots`: current side-head
    /// rotations copied from `WitherBoss.xRotHeads/yRotHeads`. The arrays are
    /// `[right_head, left_head]`; every non-wither keeps the zeroed default.
    pub fn wither_head_rotations(&self) -> ([f32; 2], [f32; 2]) {
        self.wither_heads
            .map(|state| (state.x_head_rots, state.y_head_rots))
            .unwrap_or(([0.0; 2], [0.0; 2]))
    }

    /// Resets the hurt animation countdown to [`HURT_ANIMATION_DURATION`],
    /// mirroring vanilla `LivingEntity.animateHurt` / `handleDamageEvent` setting
    /// `hurtTime = hurtDuration`.
    pub(crate) fn trigger_hurt(&mut self) {
        self.hurt = Some(HurtAnimationState {
            hurt_time: HURT_ANIMATION_DURATION,
        });
    }

    /// Vanilla `LivingEntity.handleDamageEvent` starts by applying an immediate
    /// `1.5F` walk-animation speed impulse, before the ordinary next-tick
    /// low-pass update.
    pub(crate) fn set_damage_event_walk_speed(&mut self) {
        self.walk_animation
            .get_or_insert_with(WalkAnimationState::default)
            .set_speed(1.5);
    }

    /// Vanilla `AbstractBoat.animateHurt` / `AbstractMinecart.animateHurt`: flip the shared
    /// `VehicleEntity.hurtDir`, reset `hurtTime` to ten ticks, and multiply damage by eleven.
    pub(crate) fn trigger_vehicle_hurt(&mut self) {
        self.boat_damage
            .get_or_insert_with(BoatDamageAnimationState::default)
            .animate_hurt();
    }

    pub(crate) fn start_head_rotation_lerp(&mut self, target_y_head_rot: f32, steps: i32) {
        self.head_rotation_lerp = Some(HeadRotationLerpState {
            target_y_head_rot,
            steps_remaining: steps.max(1),
        });
    }

    pub(crate) fn advance_head_rotation_lerp(&mut self, y_head_rot: &mut f32) {
        let Some(lerp) = self.head_rotation_lerp.as_mut() else {
            return;
        };
        *y_head_rot +=
            wrap_degrees(lerp.target_y_head_rot - *y_head_rot) / lerp.steps_remaining as f32;
        lerp.steps_remaining -= 1;
        if lerp.steps_remaining <= 0 {
            self.head_rotation_lerp = None;
        }
    }

    /// Vanilla `LivingEntityRenderer.extractRenderState`:
    /// `hasRedOverlay = hurtTime > 0 || deathTime > 0`.
    pub fn has_red_overlay(&self) -> bool {
        self.hurt.is_some_and(|state| state.hurt_time > 0)
            || self.death.is_some_and(|state| state.death_time > 0)
    }

    /// Vanilla `Camera.extractRenderState` hurt inputs for a living camera entity:
    /// `hurtTime - cameraEntityPartialTicks` and the ten-tick duration installed by
    /// `LivingEntity.animateHurt`. `None` means no hurt animation has been armed.
    pub fn camera_hurt_time(&self, partial_tick: f32) -> Option<(f32, f32)> {
        self.hurt.map(|state| {
            (
                state.hurt_time as f32 - partial_tick.clamp(0.0, 1.0),
                HURT_ANIMATION_DURATION as f32,
            )
        })
    }

    /// Vanilla `ArmorStandRenderState.wiggle`: `gameTime - lastHit + partialTick`.
    /// Values `>= ArmorStand.WOBBLE_TIME` render as rest.
    pub fn armor_stand_wiggle(&self, partial_tick: f32) -> f32 {
        self.armor_stand_wiggle
            .map_or(ARMOR_STAND_WOBBLE_TIME as f32, |state| {
                state.elapsed_ticks as f32 + partial_tick
            })
    }

    /// Vanilla `LivingEntity.getTicksSinceLastKineticHitFeedback(partialTicks)`:
    /// returns `0.0` before any accepted feedback event, otherwise
    /// `gameTime - lastKineticHitFeedbackTime + partialTicks`.
    pub fn ticks_since_kinetic_hit_feedback(&self, partial_tick: f32) -> f32 {
        self.kinetic_hit_feedback
            .map_or(0.0, |state| state.elapsed_ticks as f32 + partial_tick)
    }

    /// Arms a melee swing, mirroring vanilla `LivingEntity.swing(hand)`: a swing only
    /// (re)starts when none is playing, the current one is past halfway, or the counter is
    /// the fresh `-1` — so a rapid re-swing during the first half is ignored (the in-flight
    /// swing keeps ramping). `off_hand` selects the left arm (`ClientboundAnimate` action `3`).
    pub(crate) fn trigger_swing(&mut self, off_hand: bool, duration: i32) {
        let duration = duration.max(1);
        let state = self.attack_swing.get_or_insert(AttackSwingAnimationState {
            swinging: false,
            swing_time: 0,
            duration,
            attack_anim: 0.0,
            prev_attack_anim: 0.0,
            off_hand,
        });
        if !state.swinging || state.swing_time >= duration / 2 || state.swing_time < 0 {
            state.swing_time = -1;
            state.swinging = true;
            state.duration = duration;
            state.off_hand = off_hand;
        }
    }

    pub(crate) fn active_swing_off_hand(&self) -> Option<bool> {
        self.attack_swing
            .as_ref()
            .filter(|state| state.swinging)
            .map(|state| state.off_hand)
    }

    pub(crate) fn refresh_swing_duration(&mut self, duration: i32) {
        if let Some(swing) = self.attack_swing.as_mut().filter(|state| state.swinging) {
            swing.duration = duration.max(1);
        }
    }

    /// Vanilla `LivingEntity.getAttackAnim(partialTick)` = `oAttackAnim + (attackAnim -
    /// oAttackAnim) * partialTick`: the lerped `0..1` melee swing progress
    /// `HumanoidModel.setupAttackAnimation` feeds the body twist + arm whack. `0.0` for an
    /// entity that is not mid-swing (and every entity that never swung).
    pub fn attack_anim(&self, partial_tick: f32) -> f32 {
        self.attack_swing.map_or(0.0, |s| {
            s.prev_attack_anim + (s.attack_anim - s.prev_attack_anim) * partial_tick
        })
    }

    /// Whether the active swing is the off (left) hand, vanilla `swingingArm == OFF_HAND`.
    /// `false` for a main-hand swing and every non-swinging entity.
    pub fn attack_arm_off_hand(&self) -> bool {
        self.attack_swing.is_some_and(|s| s.off_hand)
    }

    /// Vanilla `LivingEntity.swinging`: whether a melee swing is currently in progress. Gates the player's
    /// `CROSSBOW_HOLD` arm pose (`AvatarRenderer.getArmPose`: `!swinging && crossbow && charged`), which
    /// yields to the swing while attacking. `false` for an entity that is not mid-swing.
    pub fn is_swinging(&self) -> bool {
        self.attack_swing.is_some_and(|s| s.swinging)
    }

    /// Vanilla `LivingEntityRenderState.deathTime`: `entity.deathTime > 0 ?
    /// entity.deathTime + partialTick : 0`, projected for the renderer death
    /// tip-over (`LivingEntityRenderer.setupRotations`). Returns `0.0` for a
    /// living entity that is not dying.
    pub fn death_time(&self, partial_tick: f32) -> f32 {
        self.death.map_or(0.0, |state| {
            if state.death_time > 0 {
                state.death_time as f32 + partial_tick
            } else {
                0.0
            }
        })
    }

    /// Vanilla `EnderDragonRenderState.deathTime`: `dragonDeathTime + partialTick` once the dragon's
    /// own death counter starts ticking, used by `EnderDragonRenderer` for the dissolve body and
    /// procedural death rays.
    pub fn ender_dragon_death_time(&self, partial_tick: f32) -> f32 {
        self.ender_dragon
            .map_or(0.0, |dragon| dragon.death_time(partial_tick))
    }

    /// Vanilla `EnderDragonRenderer.extractRenderState`'s procedural-animation inputs. The flight
    /// samples are pre-interpolated at `state.partialTicks`, which vanilla freezes to `0` for a
    /// dead-or-dying dragon; `distance_to_egg` needs the world heightmap probe and is filled by the
    /// store-and-light projection layer.
    pub(crate) fn ender_dragon_animation_source(
        &self,
        partial_ticks: f32,
    ) -> Option<EnderDragonAnimationSource> {
        let dragon = self.ender_dragon?;
        let sample_partial = if dragon.dragon_dying {
            0.0
        } else {
            partial_ticks
        };
        let mut historical = [DragonFlightHistorySample::default(); 24];
        for (delay, slot) in historical.iter_mut().enumerate() {
            *slot = dragon
                .flight_history
                .get_interpolated(delay as i32, sample_partial);
        }
        Some(EnderDragonAnimationSource {
            flap_time: dragon.flap_time(partial_ticks),
            is_landing_or_taking_off: vanilla_ender_dragon_phase_is_landing_or_takeoff(
                dragon.phase_id,
            ),
            is_sitting: vanilla_ender_dragon_phase_is_sitting(dragon.phase_id),
            distance_to_egg: 0.0,
            historical,
        })
    }

    /// Vanilla `PolarBear.getStandingAnimationScale(partialTick)` projected for
    /// the renderer `PolarBearModel.setupAnim` standing pose. Returns `0.0` when
    /// the entity is not a rearing polar bear.
    pub fn polar_bear_stand_scale(&self, partial_tick: f32) -> f32 {
        self.polar_bear_standing
            .map_or(0.0, |state| state.standing_animation_scale(partial_tick))
    }

    /// Vanilla `LivingEntityRenderState.walkAnimationPos`
    /// (`WalkAnimationState.position(partialTick)`): the lerped limb-swing position
    /// that sways the model's legs/arms. Returns `0.0` for an entity that has not
    /// been ticked as a walking living entity.
    pub fn walk_animation_position(&self, partial_tick: f32) -> f32 {
        self.walk_animation
            .map_or(0.0, |walk| walk.position(partial_tick))
    }

    /// Vanilla `LivingEntityRenderState.walkAnimationSpeed`
    /// (`WalkAnimationState.speed(partialTick)`): the lerped limb-swing speed
    /// amplitude. Returns `0.0` for an entity that is not walking.
    pub fn walk_animation_speed(&self, partial_tick: f32) -> f32 {
        self.walk_animation
            .map_or(0.0, |walk| walk.speed(partial_tick))
    }

    /// Vanilla `SquidRenderState.tentacleAngle` (`Squid.tentacleAngle` lerped):
    /// the tentacle flex angle `SquidModel.setupAnim` writes to every tentacle's
    /// `xRot`. Returns `0.0` for a non-squid entity (and a frozen/unticked squid).
    pub fn squid_tentacle_angle(&self, partial_tick: f32) -> f32 {
        self.squid
            .map_or(0.0, |squid| squid.tentacle_angle(partial_tick))
    }

    /// Vanilla `SquidRenderState.xBodyRot` (`Squid.xBodyRot` lerped, degrees): the
    /// swim pitch `SquidRenderer.setupRotations` applies to the squid root.
    /// Returns `0.0` for every non-squid entity.
    pub fn squid_x_body_rot(&self, partial_tick: f32) -> f32 {
        self.squid
            .map_or(0.0, |squid| squid.x_body_rot(partial_tick))
    }

    /// Vanilla `LivingEntityRenderState.bodyRot` for squid (`Squid.yBodyRot`
    /// lerped, degrees). Returns `None` for every non-squid entity so callers can
    /// keep using the synced transform yaw.
    pub fn squid_y_body_rot(&self, partial_tick: f32) -> Option<f32> {
        self.squid.map(|squid| squid.y_body_rot(partial_tick))
    }

    /// Vanilla `SquidRenderState.zBodyRot` (`Squid.zBodyRot` lerped, degrees): the
    /// swim roll `SquidRenderer.setupRotations` applies to the squid root. Returns
    /// `0.0` for every non-squid entity.
    pub fn squid_z_body_rot(&self, partial_tick: f32) -> f32 {
        self.squid
            .map_or(0.0, |squid| squid.z_body_rot(partial_tick))
    }

    /// Vanilla `ChickenRenderState.flap` (`Chicken.flap` lerped): the wing-flap
    /// phase `ChickenModel.setupAnim` feeds to `(sin(flap) + 1) * flapSpeed`.
    /// Returns `0.0` for every non-chicken entity (and an unticked chicken).
    pub fn chicken_flap(&self, partial_tick: f32) -> f32 {
        self.chicken_flap
            .map_or(0.0, |flap| flap.flap(partial_tick))
    }

    /// Vanilla `SlimeRenderState.squish` (`Slime.squish` lerped): the squish amount
    /// `SlimeRenderer.scale` turns into the body's non-uniform stretch (`ss = squish
    /// / (size * 0.5 + 1)`, `w = 1 / (ss + 1)`, scale `[w, 1/w, w] * size`). Returns
    /// `0.0` for every non-slime/magma-cube entity (and an unticked one), so the body
    /// holds its undeformed cube shape.
    pub fn slime_squish(&self, partial_tick: f32) -> f32 {
        self.slime.map_or(0.0, |slime| slime.squish(partial_tick))
    }

    /// Vanilla `EvokerFangsRenderState.biteProgress` (`EvokerFangs.getAnimationProgress`):
    /// the `0..1` attack ramp `EvokerFangsModel.setupAnim` turns into the jaw snap, the
    /// rise out of the ground, and the final vanish. Returns `0.0` (the hidden,
    /// pre-attack fang) for every non-fang entity and a fang that has not yet started.
    pub fn evoker_fangs_bite_progress(&self, partial_tick: f32) -> f32 {
        self.evoker_fangs
            .map_or(0.0, |fangs| fangs.bite_progress(partial_tick))
    }

    /// Vanilla `ChickenRenderState.flapSpeed` (`Chicken.flapSpeed` lerped): the
    /// wing-flap amplitude `ChickenModel.setupAnim` multiplies the flap phase by.
    /// Returns `0.0` for every non-chicken entity, so the wings hold the bind pose.
    pub fn chicken_flap_speed(&self, partial_tick: f32) -> f32 {
        self.chicken_flap
            .map_or(0.0, |flap| flap.flap_speed(partial_tick))
    }

    /// Vanilla `ParrotRenderState.flapAngle` (`ParrotRenderer.extractRenderState`):
    /// the lerped flap phase and speed combined into `(sin(flap) + 1) * flapSpeed`,
    /// which `ParrotModel.setupAnim` feeds to the wing `zRot` (`±(0.0873 +
    /// flapAngle)`) and the body/head/tail bob (`flapAngle * 0.3`). Returns `0.0` for
    /// every non-parrot entity (and an unticked parrot), so the wings hold the bind
    /// pose.
    pub fn parrot_flap_angle(&self, partial_tick: f32) -> f32 {
        self.parrot_flap
            .map_or(0.0, |flap| flap.flap_angle(partial_tick))
    }

    /// Vanilla `GuardianRenderState.tailAnimation` (`Guardian.getTailAnimation`
    /// lerped): the tail-sway phase `GuardianModel.setupAnim` feeds to the three
    /// tail segments' `yRot`. Returns `0.0` for every non-guardian entity (and an
    /// unticked guardian), so the tail holds the bind pose.
    pub fn guardian_tail_animation(&self, partial_tick: f32) -> f32 {
        self.guardian_tail
            .map_or(0.0, |tail| tail.tail_animation(partial_tick))
    }

    /// Vanilla `GuardianRenderState.spikesAnimation` (`Guardian.getSpikesAnimation(partialTick)`,
    /// partial-lerped): the spike-withdrawal phase `GuardianModel.setupAnim` turns into
    /// `withdrawal = (1 - spikesAnimation) · 0.55`. Returns `1.0` (withdrawal `0`, the fully-extended
    /// rest pose) for every non-guardian entity and an unticked guardian.
    pub fn guardian_spikes_animation(&self, partial_tick: f32) -> f32 {
        self.guardian_spikes
            .map_or(1.0, |spikes| spikes.spikes_animation(partial_tick))
    }

    /// The breeze shoot's elapsed seconds since `Pose.SHOOTING` started (vanilla `Breeze.shoot`),
    /// projected for the renderer `BreezeModel.setupAnim` `shootAnimation.apply`. Returns `-1.0` (the
    /// stopped-animation sentinel) for a non-shooting breeze and every other entity; a non-negative
    /// value clamps past the 1.125s length to its final frame (the action one-shots do not loop).
    pub fn breeze_shoot_seconds(&self, partial_tick: f32) -> f32 {
        self.breeze
            .and_then(|state| state.shoot.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The breeze slide's elapsed seconds since `Pose.SLIDING` started (vanilla `Breeze.slide`).
    /// Returns `-1.0` (stopped) for a non-sliding breeze and every other entity; clamps past 0.2s.
    pub fn breeze_slide_seconds(&self, partial_tick: f32) -> f32 {
        self.breeze
            .and_then(|state| state.slide.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The breeze slideBack's elapsed seconds since it last LEFT `Pose.SLIDING` (vanilla
    /// `Breeze.slideBack`, the 0.1s return that ends at the neutral pose). Returns `-1.0` (stopped) for
    /// a breeze that has never slid and every other entity; clamps past 0.1s to its neutral final frame.
    pub fn breeze_slide_back_seconds(&self, partial_tick: f32) -> f32 {
        self.breeze
            .and_then(|state| {
                state
                    .slide_back
                    .elapsed_seconds(self.age_ticks, partial_tick)
            })
            .unwrap_or(-1.0)
    }

    /// The breeze inhale's elapsed seconds since `Pose.INHALING` started (vanilla `Breeze.inhale`).
    /// Returns `-1.0` (stopped) for a non-inhaling breeze and every other entity; clamps past 2.0s.
    pub fn breeze_inhale_seconds(&self, partial_tick: f32) -> f32 {
        self.breeze
            .and_then(|state| state.inhale.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The breeze long-jump's elapsed seconds since `Pose.LONG_JUMPING` started (vanilla
    /// `Breeze.longJump`). Returns `-1.0` (stopped) for a non-jumping breeze and every other entity;
    /// clamps past 0.5s.
    pub fn breeze_long_jump_seconds(&self, partial_tick: f32) -> f32 {
        self.breeze
            .and_then(|state| {
                state
                    .long_jump
                    .elapsed_seconds(self.age_ticks, partial_tick)
            })
            .unwrap_or(-1.0)
    }

    /// Advance the shared crossbow-draw counter (the client's `getTicksUsingItem` reconstruction that
    /// `animateCrossbowCharge` reads) for an entity whose synced charging flag is `is_charging`: count up
    /// while drawing, reset to `0` once it stops, and drop the state once it has fully settled back to rest.
    /// Shared by the pillager and the regular piglin (only their flag's metadata id differs).
    fn advance_crossbow_charge(&mut self, is_charging: bool) {
        if is_charging || self.crossbow_charge.is_some() {
            let charge = self
                .crossbow_charge
                .get_or_insert_with(CrossbowChargeAnimationState::default);
            charge.advance_client_tick(is_charging);
            if charge.is_settled() {
                self.crossbow_charge = None;
            }
        }
    }

    pub(crate) fn age_lock_particle_timer_ticks(&self) -> u8 {
        self.age_lock_particle_timer_ticks
    }

    pub(crate) fn start_age_lock_particle_timer(&mut self) {
        self.age_lock_particle_timer_ticks = AGE_LOCK_COOLDOWN_TICKS;
    }

    pub(crate) fn advance_age_lock_particle_timer_tick(&mut self) {
        self.age_lock_particle_timer_ticks = self.age_lock_particle_timer_ticks.saturating_sub(1);
    }

    pub(crate) fn advance_client_tick(
        &mut self,
        entity_type_id: i32,
        entity_id: i32,
        transform: EntityTransform,
        // Vanilla `ClientLevel.shouldTickDeath`: only this LivingEntity death
        // counter is simulation-distance gated. Other client animation/tick
        // state remains active for tracked entities.
        should_tick_death: bool,
        is_passenger: bool,
        is_baby: bool,
        is_fall_flying: bool,
        is_crouching: bool,
        // Vanilla `Bat.isResting()`, read from the synced flags byte before `Bat.tick` calls
        // `setupAnimationStates`. It selects which of the two bat AnimationStates remains started.
        bat_is_resting: bool,
        // The per-tick world fact `Guardian.aiStep` reads via `isInWater()`,
        // resolved by [`WorldStore::advance_entity_client_animations`] (which holds
        // the chunk/fluid data this context lacks). `false` for entities that do
        // not consume it. See [`entity_animation_uses_in_water`].
        in_water: bool,
        // The synced `Guardian.DATA_ID_MOVING` flag (`isMoving()`), read from the
        // entity metadata in the tick loop. `false` for entities that do not
        // consume it.
        is_moving: bool,
        // Vanilla dry Guardian `loadedAndEntityCanStandOn(blockPosition().below())`,
        // resolved by WorldStore before the mutable ECS pass.
        guardian_can_stand_on_below: bool,
        // Vanilla `Warden.getHeartBeatDelay()` over the synced anger, read from the
        // entity metadata in the tick loop ([`warden_heartbeat_delay`]). The calm
        // `40` for entities that do not consume it.
        warden_heartbeat_delay: u32,
        // The synced `Guardian.DATA_ID_ATTACK_TARGET` (`isMoving`'s sibling), read from the entity
        // metadata in the tick loop ([`guardian_attack_target_id`]). `0` (no target) for entities that
        // do not consume it.
        guardian_attack_target_id: i32,
        // Narrow live target dimensions resolved once per unique protocol id
        // after the direct-movement pass. `None` for missing/non-living targets.
        guardian_attack_target: Option<GuardianAttackTargetTickInput>,
        guardian_eye_height: Option<f32>,
        // Vanilla `WitherBoss.aiStep` side-head targets, pre-resolved by `EntityStore` because the
        // tick needs other entities' current positions and eye heights. `None` for non-withers.
        wither_head_targets: Option<[WitherHeadTargetRotations; 2]>,
        // Vanilla `Camel.updateWalkAnimation` checks `getPose() == Pose.STANDING`; read from
        // `DATA_POSE` in the tick loop. `false` for entities that do not consume it.
        camel_is_standing: bool,
        // The synced `Camel.DASH` boolean (`isDashing()`), read from the entity metadata in the tick
        // loop ([`camel_is_dashing`]). `false` for entities that do not consume it.
        camel_is_dashing: bool,
        // The synced `CopperGolem.COPPER_GOLEM_STATE == IDLE`, read from entity metadata in the tick
        // loop. It gates the golem's delayed idle head-spin keyframe timer.
        copper_golem_is_idle: bool,
        // The synced `CopperGolem.COPPER_GOLEM_STATE == GETTING_ITEM`, read from entity metadata in
        // the tick loop. It gates the no-item/get chest interaction keyframe.
        copper_golem_is_getting_item: bool,
        // The synced `CopperGolem.COPPER_GOLEM_STATE == GETTING_NO_ITEM`, read from entity metadata
        // in the tick loop. It gates the no-item/noget chest interaction keyframe.
        copper_golem_is_getting_no_item: bool,
        // The synced `CopperGolem.COPPER_GOLEM_STATE == DROPPING_ITEM`, read from entity metadata in
        // the tick loop. It gates the item/drop chest interaction keyframe.
        copper_golem_is_dropping_item: bool,
        // The synced `CopperGolem.COPPER_GOLEM_STATE == DROPPING_NO_ITEM`, read from entity metadata
        // in the tick loop. It gates the item/nodrop chest interaction keyframe.
        copper_golem_is_dropping_no_item: bool,
        // The synced `Allay.DATA_DANCING` boolean (`isDancing()`), read from the entity metadata in the
        // tick loop ([`allay_is_dancing`]). `false` for entities that do not consume it.
        allay_is_dancing: bool,
        // Vanilla `Allay.hasItemInHand()`, resolved from synced main-hand equipment before the tick.
        // `false` for entities that do not consume it.
        allay_has_item_in_hand: bool,
        // The synced `Axolotl.DATA_PLAYING_DEAD` boolean (`isPlayingDead()`), read from the entity
        // metadata in the tick loop ([`axolotl_is_playing_dead`]). `false` for entities that do not
        // consume it.
        axolotl_is_playing_dead: bool,
        // The synced `Creaking.IS_TEARING_DOWN` boolean (`isTearingDown()`), read from the entity
        // metadata in the tick loop ([`creaking_is_tearing_down`]). `false` for entities that do not
        // consume it.
        creaking_is_tearing_down: bool,
        // The synced `Pillager.IS_CHARGING_CROSSBOW` boolean (`isChargingCrossbow()`), read from the
        // entity metadata in the tick loop ([`pillager_is_charging_crossbow`]). `false` for entities that
        // do not consume it.
        pillager_is_charging_crossbow: bool,
        // The synced `Piglin.DATA_IS_CHARGING_CROSSBOW` boolean (`isChargingCrossbow()`), read from the
        // entity metadata in the tick loop ([`piglin_is_charging_crossbow`]). `false` for entities that
        // do not consume it.
        piglin_is_charging_crossbow: bool,
        // The synced `LivingEntity` `isUsingItem()` bit (`DATA_LIVING_ENTITY_FLAGS & 1`), read from the
        // entity metadata in the tick loop ([`player_is_using_item`]). Drives the player's shared
        // crossbow-draw counter (the item-agnostic `getTicksUsingItem` reconstruction). `false` for
        // entities that do not consume it.
        player_is_using_item: bool,
        // The synced `Wolf.DATA_INTERESTED_ID` boolean (`isInterested()`), read from metadata in the
        // tick loop ([`wolf_is_interested`]). It drives the wolf's client-side interested-angle ease.
        wolf_is_interested: bool,
        // The synced `Cat.IS_LYING` / `RELAX_STATE_ONE` booleans, read from metadata in the tick loop.
        // They drive the cat's client-side lie-down and relaxed-head easing. `false` for non-cats.
        cat_is_lying: bool,
        cat_relax_state_one: bool,
        // The direct locally controlled root boat advances paddles later in
        // the input/control owner, after `controlBoat` and `move`, exactly as
        // Vanilla does. Remote and uncontrolled boats advance here.
        advance_boat_paddles_here: bool,
        // Vanilla `AbstractBoat.getPaddleState(side)`: synced left/right paddle
        // booleans gated by the presence of a controlling passenger.
        boat_paddle_left: bool,
        boat_paddle_right: bool,
        // Vanilla `AbstractBoat.DATA_ID_BUBBLE_TIME`, read each client tick by
        // `tickBubbleColumn()` to ease the wobble multiplier.
        boat_bubble_time: i32,
        // Vanilla `MinecartFurnace.DATA_ID_FUEL`. The server owns fuel/push;
        // the client reads only this synced bit for its presentation smoke.
        furnace_minecart_has_fuel: bool,
        // Vanilla `Slime.getSize()`, clamped by `setSize` to `1..=127`.
        slime_size: i32,
        // Current `getDimensions(getPose()).width()`, including size, pose,
        // and the LivingEntity SCALE attribute.
        slime_width: f32,
        // Vanilla `Entity.isSilent()`, used by Blaze, Warden, Guardian,
        // Nautilus, and Slime client-local sound branches. Each gate precedes
        // that branch's sound random draws.
        is_silent: bool,
        // Vanilla `Entity.isSwimming()` for player cape bob suppression.
        is_swimming: bool,
    ) -> EntityClientTickParticleTriggers {
        self.age_ticks = self.age_ticks.saturating_add(1);
        self.advance_age_lock_particle_timer_tick();
        self.spawn_body_y_rot = None;
        let mut particle_triggers = EntityClientTickParticleTriggers::default();
        if entity_type_id == VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID && self.age_ticks == 1 {
            let random_seed = self
                .lightning_bolt_client_random_seed
                .get_or_insert_with(|| {
                    let mut random_seed = java_random_seed(
                        i64::from(entity_id).wrapping_add(LIGHTNING_BOLT_CLIENT_RANDOM_SALT as i64),
                    );
                    java_random_next_long(&mut random_seed);
                    java_random_next_int_bound(&mut random_seed, 3);
                    random_seed
                });
            particle_triggers.lightning_bolt_sounds = Some(LightningBoltSoundParameters {
                thunder_pitch: 0.8 + java_random_next_float(random_seed) * 0.2,
                impact_pitch: 0.5 + java_random_next_float(random_seed) * 0.2,
            });
        }
        // AbstractArrow owns the ordering-sensitive shake decrement in the
        // direct movement pass: an initial containment hit precedes it, while
        // `onHitBlock` installs a fresh seven-tick shake after it.
        // Vanilla `LivingEntity.baseTick`: `if (hurtTime > 0) hurtTime--`. Applies
        // to every living entity, so it runs outside the per-type match.
        if let Some(hurt) = self.hurt.as_mut() {
            hurt.hurt_time = (hurt.hurt_time - 1).max(0);
            if hurt.hurt_time == 0 {
                self.hurt = None;
            }
        }
        // Vanilla `LivingEntity.tickDeath`: `deathTime++` each tick while dying.
        // The death state is only present for a dying living entity (set from the
        // synced health), so this also runs outside the per-type match.
        if should_tick_death {
            if let Some(death) = self.death.as_mut() {
                death.death_time = (death.death_time + 1).min(DEATH_ANIMATION_MAX_TICKS);
            }
        }
        if entity_type_id == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
            if let Some(wiggle) = self.armor_stand_wiggle.as_mut() {
                wiggle.elapsed_ticks += 1;
                if wiggle.elapsed_ticks >= ARMOR_STAND_WOBBLE_TIME {
                    self.armor_stand_wiggle = None;
                }
            }
        }
        if entity_type_id == VANILLA_ENTITY_TYPE_TNT_MINECART_ID {
            if let Some(fuse) = self.minecart_tnt_fuse.as_mut() {
                particle_triggers.minecart_tnt_smoke = fuse.fuse_remaining_ticks > 0;
                fuse.advance_client_tick();
            }
        }
        if entity_type_id == VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID && furnace_minecart_has_fuel {
            let random_seed = self
                .minecart_furnace_smoke_random_seed
                .get_or_insert_with(|| {
                    java_random_seed(i64::from(entity_id).wrapping_add(0x6675_726e))
                });
            particle_triggers.minecart_furnace_large_smoke =
                java_random_next_int_bound(random_seed, 4) == 0;
        }
        if entity_type_id == VANILLA_ENTITY_TYPE_BLAZE_ID {
            let random_seed = self.blaze_client_random_seed.get_or_insert_with(|| {
                java_random_seed(i64::from(entity_id).wrapping_add(BLAZE_CLIENT_RANDOM_SALT as i64))
            });
            if java_random_next_int_bound(random_seed, 24) == 0 && !is_silent {
                particle_triggers.blaze_burn_sound = Some(BlazeBurnSoundParameters {
                    volume: 1.0 + java_random_next_float(random_seed),
                    pitch: java_random_next_float(random_seed) * 0.7 + 0.3,
                });
            }
            particle_triggers.blaze_large_smoke =
                Some(std::array::from_fn(|_| BlazeSmokeParticleOffset {
                    x_unit: java_random_next_double(random_seed) - 0.5,
                    y_unit: java_random_next_double(random_seed),
                    z_unit: java_random_next_double(random_seed) - 0.5,
                }));
        }
        if matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_ENDERMAN_ID | VANILLA_ENTITY_TYPE_ENDERMITE_ID
        ) {
            let random_seed = self.ender_ambient_random_seed.get_or_insert_with(|| {
                java_random_seed(
                    i64::from(entity_id)
                        .wrapping_add(ENDER_AMBIENT_RANDOM_SALT as i64)
                        .wrapping_add(i64::from(entity_type_id)),
                )
            });
            particle_triggers.ender_portal =
                Some(std::array::from_fn(|_| EnderAmbientParticleOffset {
                    x_unit: java_random_next_double(random_seed) - 0.5,
                    y_unit: java_random_next_double(random_seed),
                    z_unit: java_random_next_double(random_seed) - 0.5,
                    velocity: EntityVec3 {
                        x: (java_random_next_double(random_seed) - 0.5) * 2.0,
                        y: -java_random_next_double(random_seed),
                        z: (java_random_next_double(random_seed) - 0.5) * 2.0,
                    },
                }));
        }
        if vanilla_entity_metadata_type_is_guardian(entity_type_id) {
            let random_seed = self.guardian_client_random_seed.get_or_insert_with(|| {
                let mut random_seed = java_random_seed(
                    i64::from(entity_id)
                        .wrapping_add(GUARDIAN_CLIENT_RANDOM_SALT as i64)
                        .wrapping_add(i64::from(entity_type_id)),
                );
                // Guardian's constructor consumes the initial tail phase from
                // the same entity RNG before the first `aiStep` draw.
                java_random_next_float(&mut random_seed);
                random_seed
            });
            particle_triggers.guardian_flop_sound = self
                .guardian_tail
                .get_or_insert_with(GuardianTailAnimationState::default)
                .advance_client_tick(
                    in_water,
                    is_moving,
                    transform.delta_movement.y,
                    guardian_can_stand_on_below,
                    is_silent,
                );
            let dry_spike_random = (!in_water).then(|| java_random_next_float(random_seed));
            self.guardian_spikes
                .get_or_insert_with(GuardianSpikesAnimationState::default)
                .advance_client_tick(in_water, is_moving, dry_spike_random);
            if is_moving && in_water {
                particle_triggers.guardian_moving_bubbles =
                    Some(std::array::from_fn(|_| GuardianAmbientParticleOffset {
                        x_unit: java_random_next_double(random_seed) - 0.5,
                        y_unit: java_random_next_double(random_seed),
                        z_unit: java_random_next_double(random_seed) - 0.5,
                    }));
            }
            let attack_duration = guardian_attack_duration(entity_type_id);
            let attack_scale = {
                let attack = self
                    .guardian_attack
                    .get_or_insert_with(GuardianAttackAnimationState::default);
                attack.advance_client_tick(guardian_attack_target_id, attack_duration);
                attack.attack_scale(0.0, attack_duration)
            };
            if let (Some(target), Some(eye_height)) = (guardian_attack_target, guardian_eye_height)
            {
                let guardian_eye_y = transform.position.y + f64::from(eye_height);
                let look_x = target.position.x - transform.position.x;
                let look_y = target.position.y + f64::from(target.eye_height) - guardian_eye_y;
                let look_z = target.position.z - transform.position.z;
                let look_horizontal = (look_x * look_x + look_z * look_z).sqrt();
                let desired_y_head_rot = if look_x.abs() <= f64::from(1.0e-5_f32)
                    && look_z.abs() <= f64::from(1.0e-5_f32)
                {
                    transform.y_head_rot
                } else {
                    (look_z.atan2(look_x).to_degrees() as f32) - 90.0
                };
                let desired_x_rot = if look_y.abs() <= f64::from(1.0e-5_f32)
                    && look_horizontal.abs() <= f64::from(1.0e-5_f32)
                {
                    0.0
                } else {
                    -(look_y.atan2(look_horizontal).to_degrees() as f32)
                };
                particle_triggers.guardian_look_update = Some(GuardianLookUpdate {
                    // `LookControl.tick` resets xRot before applying the target.
                    x_rot: rotlerp(0.0, desired_x_rot, 90.0),
                    y_head_rot: rotlerp(transform.y_head_rot, desired_y_head_rot, 90.0),
                });

                let beam_x = target.position.x - transform.position.x;
                let beam_y = target.position.y + f64::from(target.half_height) - guardian_eye_y;
                let beam_z = target.position.z - transform.position.z;
                let beam_distance = (beam_x * beam_x + beam_y * beam_y + beam_z * beam_z).sqrt();
                let mut distance = java_random_next_double(random_seed);
                // Each loop advances by at least `1.8 - attack_scale` before
                // emitting. Reserve that exact upper bound up front so a long
                // beam does not repeatedly reallocate its particle owner. The
                // loop and RNG consumption remain byte-for-byte ordered like
                // Guardian.aiStep.
                let minimum_step = 1.8 - f64::from(attack_scale);
                let position_capacity = if distance < beam_distance {
                    ((beam_distance - distance) / minimum_step).ceil() as usize
                } else {
                    0
                };
                let mut positions = Vec::with_capacity(position_capacity);
                while distance < beam_distance {
                    distance += 1.8 - f64::from(attack_scale)
                        + java_random_next_double(random_seed) * (1.7 - f64::from(attack_scale));
                    positions.push(EntityVec3 {
                        x: transform.position.x + beam_x / beam_distance * distance,
                        y: guardian_eye_y + beam_y / beam_distance * distance,
                        z: transform.position.z + beam_z / beam_distance * distance,
                    });
                }
                if !positions.is_empty() {
                    particle_triggers.guardian_attack_bubbles = Some(positions);
                }
            }
            if !in_water && transform.on_ground.unwrap_or(false) {
                particle_triggers.guardian_dry_jump = Some(GuardianDryJumpUpdate {
                    x_impulse: f64::from((java_random_next_float(random_seed) * 2.0 - 1.0) * 0.4),
                    z_impulse: f64::from((java_random_next_float(random_seed) * 2.0 - 1.0) * 0.4),
                    y_rot: java_random_next_float(random_seed) * 360.0,
                });
            }
        }
        if entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
            if self.ender_dragon.is_some_and(|dragon| dragon.dragon_dying) {
                self.ender_dragon
                    .get_or_insert_with(EnderDragonAnimationState::default)
                    .advance_dragon_death_tick(ENDER_DRAGON_DEATH_ANIMATION_MAX_TICKS);
            } else if let Some(dragon) = self.ender_dragon.as_mut() {
                dragon.clear_dragon_death();
            }
        }
        // Vanilla `LivingEntity.updateSwimAmount`: save `swimAmountO`, then ease
        // `swimAmount` toward the current `isVisuallySwimming()` target by 0.09.
        if let Some(swim) = self.living_swim.as_mut() {
            swim.advance_client_tick();
            if swim.is_settled() {
                self.living_swim = None;
            }
        }
        // Vanilla `LivingEntity` (`oAttackAnim = attackAnim` at tick start, then
        // `updateSwingTime`): the melee swing ramps for every living entity, so it runs
        // outside the per-type match. The state is dropped once the swing has fully decayed.
        if let Some(feedback) = self.kinetic_hit_feedback.as_mut() {
            feedback.elapsed_ticks = feedback.elapsed_ticks.saturating_add(1);
        }
        if let Some(swing) = self.attack_swing.as_mut() {
            let duration = swing.duration.max(1);
            swing.prev_attack_anim = swing.attack_anim;
            if swing.swinging {
                swing.swing_time += 1;
                if swing.swing_time >= duration {
                    swing.swing_time = 0;
                    swing.swinging = false;
                }
            } else {
                swing.swing_time = 0;
            }
            swing.attack_anim = swing.swing_time as f32 / duration as f32;
            if !swing.swinging && swing.attack_anim == 0.0 && swing.prev_attack_anim == 0.0 {
                self.attack_swing = None;
            }
        }
        // Vanilla `LivingEntity.tick` always advances `elytraAnimationState` after
        // refreshing attributes. The state itself branches on `isFallFlying()`,
        // `isCrouching()`, and `getDeltaMovement()`, independent of whether an
        // elytra item is currently equipped.
        if vanilla_living_entity_type(entity_type_id) {
            self.elytra
                .get_or_insert_with(ElytraAnimationState::default)
                .advance_client_tick(is_fall_flying, is_crouching, transform.delta_movement);
        }
        match entity_type_id {
            VANILLA_ENTITY_TYPE_CREEPER_ID => {
                if let Some(swell) = self.creeper_swell.as_mut() {
                    swell.advance_client_tick();
                    if swell.is_settled() {
                        self.creeper_swell = None;
                    }
                }
            }
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => {
                if let Some(standing) = self.polar_bear_standing.as_mut() {
                    standing.advance_client_tick();
                }
            }
            VANILLA_ENTITY_TYPE_SHEEP_ID => {
                if let Some(eat) = self.sheep_eat.as_mut() {
                    eat.advance_client_tick();
                    if eat.eat_animation_tick == 0 {
                        self.sheep_eat = None;
                    }
                }
            }
            VANILLA_ENTITY_TYPE_GOAT_ID => {
                if let Some(ramming) = self.goat_ramming.as_mut() {
                    ramming.advance_client_tick();
                    if ramming.is_settled() {
                        self.goat_ramming = None;
                    }
                }
            }
            VANILLA_ENTITY_TYPE_IRON_GOLEM_ID => {
                if let Some(golem) = self.iron_golem.as_mut() {
                    golem.advance_client_tick();
                    if golem.is_settled() {
                        self.iron_golem = None;
                    }
                }
            }
            VANILLA_ENTITY_TYPE_RAVAGER_ID => {
                if let Some(ravager) = self.ravager.as_mut() {
                    if let Some(offset) = ravager.advance_client_tick(entity_id) {
                        particle_triggers.ravager_stun = Some(offset);
                    }
                    if ravager.is_settled() {
                        self.ravager = None;
                    }
                }
            }
            entity_type_id if vanilla_entity_metadata_type_is_hoglin_base(entity_type_id) => {
                if let Some(hoglin) = self.hoglin.as_mut() {
                    hoglin.advance_client_tick();
                    if hoglin.is_settled() {
                        self.hoglin = None;
                    }
                }
            }
            VANILLA_ENTITY_TYPE_RABBIT_ID => {
                if let Some(rabbit_hop) = self.rabbit_hop.as_mut() {
                    rabbit_hop.advance_client_tick(self.age_ticks);
                    if rabbit_hop.is_settled() {
                        self.rabbit_hop = None;
                    }
                }
            }
            VANILLA_ENTITY_TYPE_CREAKING_ID => {
                // The death one-shot is driven by the synced `isTearingDown()` directly, so a
                // creaking that begins tearing down spins up the state even without a prior
                // attack/invulnerable event; otherwise an idle creaking carries no state.
                if self.creaking.is_some() || creaking_is_tearing_down {
                    let creaking = self
                        .creaking
                        .get_or_insert_with(|| CreakingAnimationState::new(entity_id));
                    creaking.advance_client_tick(
                        entity_id,
                        self.age_ticks,
                        creaking_is_tearing_down,
                    );
                    if creaking.is_settled() {
                        self.creaking = None;
                    }
                }
            }
            VANILLA_ENTITY_TYPE_BAT_ID => self
                .bat_flying
                .get_or_insert_with(KeyframeAnimationState::default)
                .animate_when(!bat_is_resting, self.age_ticks),
            VANILLA_ENTITY_TYPE_SHULKER_ID => {
                if let Some(peek) = self.shulker_peek.as_mut() {
                    peek.advance_client_tick();
                }
                if let Some(teleport) = self.shulker_teleport.as_mut() {
                    if teleport.interpolation_ticks > 0 {
                        teleport.interpolation_ticks -= 1;
                        if teleport.interpolation_ticks == 0 {
                            teleport.old_attach_position = None;
                        }
                    }
                }
            }
            VANILLA_ENTITY_TYPE_BEE_ID => {
                if let Some(roll) = self.bee_roll.as_mut() {
                    roll.advance_client_tick();
                }
            }
            VANILLA_ENTITY_TYPE_PANDA_ID => {
                let settled = if let Some(panda) = self.panda.as_mut() {
                    panda.advance_client_tick();
                    panda.is_settled()
                } else {
                    false
                };
                if settled {
                    self.panda = None;
                }
            }
            VANILLA_ENTITY_TYPE_ARMADILLO_ID => {
                if let Some(armadillo) = self.armadillo.as_mut() {
                    armadillo.setup_animation_states(self.age_ticks);
                }
            }
            VANILLA_ENTITY_TYPE_FOX_ID => {
                if let Some(fox) = self.fox.as_mut() {
                    fox.advance_client_tick();
                }
            }
            VANILLA_ENTITY_TYPE_CAT_ID => {
                if self.cat_lie_down.is_some() || cat_is_lying || cat_relax_state_one {
                    let cat = self
                        .cat_lie_down
                        .get_or_insert_with(CatLieDownAnimationState::default);
                    cat.advance_client_tick(cat_is_lying, cat_relax_state_one);
                    if cat.is_settled() {
                        self.cat_lie_down = None;
                    }
                }
            }
            VANILLA_ENTITY_TYPE_WOLF_ID => {
                if self.wolf_head_roll.is_some() || wolf_is_interested {
                    let head_roll = self
                        .wolf_head_roll
                        .get_or_insert(WolfHeadRollAnimationState {
                            interested: false,
                            previous_interested_angle: 0.0,
                            current_interested_angle: 0.0,
                        });
                    head_roll.advance_client_tick(wolf_is_interested);
                    if head_roll.is_settled() {
                        self.wolf_head_roll = None;
                    }
                }
                if self.wolf_wet.is_some() || in_water {
                    let wolf_wet = self.wolf_wet.get_or_insert(WolfWetAnimationState {
                        is_wet: false,
                        is_shaking: false,
                        previous_shake_anim: 0.0,
                        current_shake_anim: 0.0,
                    });
                    wolf_wet.advance_client_tick(in_water);
                    if wolf_wet.is_settled() {
                        self.wolf_wet = None;
                    }
                }
            }
            VANILLA_ENTITY_TYPE_WARDEN_ID => {
                if let Some(tendril) = self.warden_tendril.as_mut() {
                    tendril.advance_client_tick();
                    if tendril.is_settled() {
                        self.warden_tendril = None;
                    }
                }
                // The heart beats forever (vanilla resets it every `getHeartBeatDelay()`
                // ticks), so unlike the event-driven tendril it is never dropped once a
                // warden starts ticking.
                let heartbeat = self
                    .warden_heart
                    .get_or_insert_with(WardenHeartAnimationState::default)
                    .advance_client_tick(self.age_ticks, warden_heartbeat_delay);
                if heartbeat && !is_silent {
                    let random_seed = self.warden_client_random_seed.get_or_insert_with(|| {
                        java_random_seed(
                            i64::from(entity_id).wrapping_add(WARDEN_CLIENT_RANDOM_SALT as i64),
                        )
                    });
                    particle_triggers.warden_heartbeat_pitch = Some(
                        (java_random_next_float(random_seed) - java_random_next_float(random_seed))
                            * 0.2
                            + 1.0,
                    );
                }
            }
            VANILLA_ENTITY_TYPE_WITHER_ID => {
                let targets = wither_head_targets.unwrap_or_else(|| {
                    [
                        WitherHeadTargetRotations::fallback_to_body(transform.y_rot),
                        WitherHeadTargetRotations::fallback_to_body(transform.y_rot),
                    ]
                });
                self.wither_heads
                    .get_or_insert_with(WitherHeadRotationsState::default)
                    .advance_client_tick(targets);
            }
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => {
                let random_seed = self.ender_dragon_client_random_seed.get_or_insert_with(|| {
                    java_random_seed(
                        i64::from(entity_id).wrapping_add(ENDER_DRAGON_CLIENT_RANDOM_SALT as i64),
                    )
                });
                particle_triggers.ender_dragon_sounds = self
                    .ender_dragon
                    .get_or_insert_with(EnderDragonAnimationState::default)
                    .advance_client_tick(
                        transform,
                        is_silent,
                        random_seed,
                        java_random_next_float,
                        java_random_next_int_bound,
                    );
            }
            _ if vanilla_entity_type_is_squid(entity_type_id) => self
                .squid
                .get_or_insert_with(|| {
                    SquidAnimationState::new_with_y_body_rot(entity_id, transform.y_head_rot)
                })
                .advance_client_tick(transform.delta_movement, in_water),
            _ if vanilla_entity_renderer_uses_abstract_horse_renderer(entity_type_id) => {
                self.equine_tail
                    .get_or_insert_with(|| EquineTailAnimationState::new(entity_id))
                    .advance_client_tick();
                if let Some(pose) = self.equine_pose.as_mut() {
                    pose.advance_client_tick();
                }
            }
            _ if vanilla_entity_metadata_type_is_guardian(entity_type_id) => {}
            VANILLA_ENTITY_TYPE_FROG_ID => {
                // Vanilla `Frog.tick` (client side): `swimIdleAnimationState.animateWhen(isInWater()
                // && !walkAnimation.isMoving(), tickCount)`. `isInWater()` is the per-tick world fact
                // threaded in; `walkAnimation.isMoving()` reads the limb-swing speed from the PREVIOUS
                // tick's `WalkAnimationState` — the walk accumulator below this match advances after,
                // so reading it here matches vanilla `tick` running before the limb-swing update. A
                // missing state is treated as not moving, matching the first tick before the accumulator
                // records a previous feet position.
                let walk_is_moving = self
                    .walk_animation
                    .as_ref()
                    .is_some_and(WalkAnimationState::is_moving);
                self.frog_swim_idle
                    .get_or_insert_with(|| KeyframeAnimationState { start_age: None })
                    .animate_when(in_water && !walk_is_moving, self.age_ticks);
            }
            _ if vanilla_entity_metadata_type_is_camel(entity_type_id) => {
                self.camel_idle
                    .get_or_insert_with(|| CamelIdleAnimationState::new(entity_id))
                    .advance_client_tick(self.age_ticks);
                // Vanilla `Camel.setupAnimationStates`: `dashAnimationState.animateWhen(isDashing(),
                // tickCount)` — the synced `DASH` boolean starts the looping gallop on its rising edge.
                // (Vanilla also `stop()`s it while visually sitting, but a sitting camel never has DASH
                // set, so gating purely on the flag is equivalent. The camel's sit/stand poses are
                // derived separately in the native layer from `LAST_POSE_CHANGE_TICK`.)
                self.camel_dash
                    .get_or_insert_with(|| KeyframeAnimationState { start_age: None })
                    .animate_when(camel_is_dashing, self.age_ticks);
                let cooldown = self
                    .camel_dash_cooldown
                    .get_or_insert_with(Default::default);
                particle_triggers.camel_dash_ready_sound =
                    cooldown.advance_client_tick(camel_is_dashing);
                if cooldown.is_settled() {
                    self.camel_dash_cooldown = None;
                }
            }
            VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID => {
                self.copper_golem_idle
                    .get_or_insert_with(|| CopperGolemIdleAnimationState::new(entity_id))
                    .advance_client_tick(self.age_ticks, copper_golem_is_idle);
                self.copper_golem_get_item
                    .get_or_insert_with(KeyframeAnimationState::default)
                    .animate_when(copper_golem_is_getting_item, self.age_ticks);
                self.copper_golem_get_no_item
                    .get_or_insert_with(KeyframeAnimationState::default)
                    .animate_when(copper_golem_is_getting_no_item, self.age_ticks);
                self.copper_golem_drop_item
                    .get_or_insert_with(KeyframeAnimationState::default)
                    .animate_when(copper_golem_is_dropping_item, self.age_ticks);
                self.copper_golem_drop_no_item
                    .get_or_insert_with(KeyframeAnimationState::default)
                    .animate_when(copper_golem_is_dropping_no_item, self.age_ticks);
            }
            VANILLA_ENTITY_TYPE_ALLAY_ID => {
                // Vanilla `Allay.tick`: while the synced `DATA_DANCING` flag is set, advance the dance
                // and spin counters; otherwise reset them. The counters drive `AllayModel`'s body spin.
                self.allay_dance
                    .get_or_insert_with(AllayDanceAnimationState::default)
                    .advance_client_tick(allay_is_dancing);
                let holding_item = self
                    .allay_holding_item
                    .get_or_insert_with(AllayHoldingItemAnimationState::default);
                holding_item.advance_client_tick(allay_has_item_in_hand);
                if holding_item.is_settled() {
                    self.allay_holding_item = None;
                }
            }
            VANILLA_ENTITY_TYPE_PILLAGER_ID => {
                // Vanilla `Pillager` `CROSSBOW_CHARGE`: while `isChargingCrossbow()` is set, count up the
                // draw ticks (the client's `getTicksUsingItem` reconstruction) for `animateCrossbowCharge`;
                // reset to 0 once it stops.
                self.advance_crossbow_charge(pillager_is_charging_crossbow);
            }
            VANILLA_ENTITY_TYPE_PIGLIN_ID => {
                // Vanilla `Piglin` `CROSSBOW_CHARGE`: the regular piglin draws its crossbow with the SAME
                // `animateCrossbowCharge`, so it shares the draw counter — only the synced flag's id differs.
                self.advance_crossbow_charge(piglin_is_charging_crossbow);
            }
            VANILLA_ENTITY_TYPE_PLAYER_ID => {
                // Vanilla player `CROSSBOW_CHARGE`: `animateCrossbowCharge` reads `getTicksUsingItem()`, which
                // is item-agnostic, so the same shared draw counter is advanced off the player's `isUsingItem`
                // bit. The native layer applies the pose only when the using item is an uncharged crossbow.
                self.advance_crossbow_charge(player_is_using_item);
                let is_alive = self.death.is_none();
                self.player_cloak
                    .get_or_insert_with(PlayerCloakAnimationState::default)
                    .advance_client_tick(
                        transform.position,
                        transform.delta_movement,
                        transform.on_ground.unwrap_or(false),
                        is_alive,
                        is_swimming,
                        is_fall_flying,
                    );
            }
            VANILLA_ENTITY_TYPE_AXOLOTL_ID => {
                // Vanilla `Axolotl.baseTick` selects `tickBabyAnimations` or
                // `tickAdultAnimations`: babies keep one of seven keyframe timers active; adults
                // drive the play-dead / in-water / on-ground BinaryAnimators. Both paths update the
                // moving animator.
                // `walkAnimation.isMoving()` reads the PRIOR tick's limb swing (the walk animation is
                // advanced after this match, mirroring vanilla's `baseTick`-before-`aiStep` order),
                // OR'd with a body/head rotation change.
                let walk_is_moving = self.walk_animation.is_some_and(|walk| walk.is_moving());
                self.axolotl
                    .get_or_insert_with(AxolotlAnimationState::default)
                    .advance_client_tick(
                        is_baby,
                        self.age_ticks,
                        axolotl_is_playing_dead,
                        in_water,
                        transform.on_ground.unwrap_or(false),
                        walk_is_moving,
                        transform.x_rot,
                        transform.y_rot,
                    );
            }
            VANILLA_ENTITY_TYPE_CHICKEN_ID => self
                .chicken_flap
                .get_or_insert_with(ChickenFlapAnimationState::default)
                // Vanilla `Chicken.aiStep` reads `onGround()`. A chicken with no
                // synced ground flag defaults to on-ground (wings still), the safe
                // common case.
                .advance_client_tick(transform.on_ground.unwrap_or(true)),
            _ if vanilla_entity_metadata_type_is_slime(entity_type_id) => {
                let random_seed = self.slime_client_random_seed.get_or_insert_with(|| {
                    java_random_seed(
                        i64::from(entity_id).wrapping_add(SLIME_CLIENT_RANDOM_SALT as i64),
                    )
                });
                self.slime
                    .get_or_insert_with(SlimeAnimationState::default)
                    // Vanilla's entity field initializes `onGround` to false; the
                    // first movement packet replaces this fallback.
                    .advance_client_tick(
                        transform.on_ground.unwrap_or(false),
                        entity_type_id == VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
                        slime_size,
                        slime_width,
                        is_silent,
                        random_seed,
                        &mut particle_triggers,
                    );
            }
            VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID => {
                // Vanilla `EvokerFangs.tick` counts down `lifeTicks` once the attack
                // has started (the event arm seeds the state); an un-attacked fang
                // holds `started = false` (hidden) and is left untouched.
                if let Some(fangs) = self.evoker_fangs.as_mut() {
                    if let Some(offsets) = fangs.advance_client_tick(entity_id) {
                        particle_triggers.evoker_fangs_crit = Some(offsets);
                    }
                }
            }
            VANILLA_ENTITY_TYPE_PARROT_ID => self
                .parrot_flap
                .get_or_insert_with(ParrotFlapAnimationState::default)
                // Vanilla `Parrot.calculateFlapping` reads `!onGround() &&
                // !isPassenger()`. A parrot with no synced ground flag defaults to
                // on-ground (wings still), the safe common case.
                .advance_client_tick(transform.on_ground.unwrap_or(true), is_passenger),
            _ if is_vanilla_vehicle_entity_type(entity_type_id) => {
                if let Some(damage) = self.boat_damage.as_mut() {
                    damage.advance_client_tick();
                    if damage.is_settled() {
                        self.boat_damage = None;
                    }
                }
                if is_vanilla_boat_type(entity_type_id) {
                    if self.boat_bubble.is_some() || boat_bubble_time > 0 {
                        let bubble = self
                            .boat_bubble
                            .get_or_insert_with(BoatBubbleAnimationState::default);
                        bubble.advance_client_tick(self.age_ticks, boat_bubble_time);
                        if bubble.is_settled(boat_bubble_time) {
                            self.boat_bubble = None;
                        }
                    }
                    if advance_boat_paddles_here {
                        particle_triggers.boat_paddle_sounds =
                            self.advance_boat_paddles(boat_paddle_left, boat_paddle_right);
                    }
                }
            }
            _ => {}
        }
        // Vanilla `LivingEntity.calculateEntityAnimation` runs every client tick
        // (`aiStep`, or `RemotePlayer.tick`) for every living entity, feeding the
        // per-tick travel to the limb-swing accumulator, using each modelled entity override's
        // distance mapping and gate.
        if vanilla_living_entity_type(entity_type_id) {
            // Vanilla `isAlive() = getHealth() > 0`: the client death animation
            // state is present exactly while the entity is dead/dying.
            let is_alive = self.death.is_none();
            let use_y = is_flying_animal(entity_type_id);
            let frog_jump_started = self
                .frog_jump
                .as_ref()
                .is_some_and(|jump| jump.start_age.is_some());
            let camel_walk_active = camel_is_standing && !camel_is_dashing;
            self.walk_animation
                .get_or_insert_with(WalkAnimationState::default)
                .advance_client_tick(
                    entity_type_id,
                    transform.position,
                    use_y,
                    is_passenger,
                    is_alive,
                    is_baby,
                    frog_jump_started,
                    camel_walk_active,
                );
        }
        particle_triggers
    }

    pub(crate) fn advance_boat_paddles(
        &mut self,
        left_active: bool,
        right_active: bool,
    ) -> [bool; 2] {
        if self.boat.is_none() && !left_active && !right_active {
            return [false; 2];
        }
        let boat = self
            .boat
            .get_or_insert_with(BoatPaddleAnimationState::default);
        let sounds = boat.advance_client_tick(left_active, right_active);
        if boat.is_settled() {
            self.boat = None;
        }
        sounds
    }
}

pub(crate) fn spawn_random_unit_float(uuid: Uuid, salt: u64) -> f32 {
    let bits = spawn_random_bits(uuid, salt);
    ((bits >> 40) as u32) as f32 / (1_u32 << 24) as f32
}

pub(super) fn spawn_random_bits(uuid: Uuid, salt: u64) -> u64 {
    let value = uuid.as_u128();
    let mut mixed = (value as u64) ^ ((value >> 64) as u64) ^ salt;
    // SplitMix64's finalizer gives stable, well-distributed client-local values
    // without introducing an independently mutable RNG owner.
    mixed = mixed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^ (mixed >> 31)
}

/// Whether an entity type's client-tick animation reads `isInWater()` and so needs
/// the per-entity `in_water` map [`WorldStore::advance_entity_client_animations`]
/// builds. Keeping this cheap (a tiny `match`) lets the world skip the AABB / fluid
/// probe for every other entity. Adding a consumer (the frog swim-idle, wolf wet shade,
/// dolphin, axolotl, fish, …) is just another arm here.
pub(crate) fn entity_animation_uses_in_water(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_is_guardian(entity_type_id)
        || vanilla_entity_metadata_type_is_camel(entity_type_id)
        || is_vanilla_abstract_nautilus_type(entity_type_id)
        || vanilla_entity_type_is_squid(entity_type_id)
        || matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_FROG_ID
                | VANILLA_ENTITY_TYPE_WOLF_ID
                | VANILLA_ENTITY_TYPE_AXOLOTL_ID
        )
}

/// Vanilla `Guardian.isMoving()` = the synced `DATA_ID_MOVING` boolean. The generated accessor
/// bundle supplies the inherited wire index/default and excludes non-guardians.
pub(crate) fn guardian_is_moving(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    let Some(accessors) =
        vanilla_entity_metadata_guardian_accessors_for_entity_type(entity_type_id)
    else {
        return false;
    };
    entity_data_bool(
        data_values,
        accessors.moving_data_id,
        accessors.moving_default,
    )
}

/// Vanilla `Camel.isDashing()` (`entityData.get(DASH)`): the synced boolean that gates the looping
/// dash gallop. The generated accessor supplies Camel/CamelHusk membership, wire index and default
/// while excluding the unrelated `AbstractNautilus.DASH` accessor.
pub(crate) fn camel_is_dashing(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_camel_dash_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CopperGolemAnimationFlags {
    pub(crate) idle: bool,
    pub(crate) getting_item: bool,
    pub(crate) getting_no_item: bool,
    pub(crate) dropping_item: bool,
    pub(crate) dropping_no_item: bool,
}

/// Resolves the generated `CopperGolem.COPPER_GOLEM_STATE` once, then projects the mutually
/// exclusive IDLE / GETTING_ITEM / GETTING_NO_ITEM / DROPPING_ITEM / DROPPING_NO_ITEM animation
/// gates in vanilla enum order. Non-CopperGolem entities own none of these states.
pub(crate) fn copper_golem_animation_flags(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> CopperGolemAnimationFlags {
    let Some(accessors) =
        vanilla_entity_metadata_copper_golem_accessors_for_entity_type(entity_type_id)
    else {
        return CopperGolemAnimationFlags::default();
    };
    let state_id = entity_data_enum_id(
        data_values,
        accessors.state_data_id,
        EntityDataEnumSerializer::CopperGolemState,
        accessors.state_default,
    );
    CopperGolemAnimationFlags {
        idle: state_id == COPPER_GOLEM_STATE_IDLE_ID,
        getting_item: state_id == COPPER_GOLEM_STATE_GETTING_ITEM_ID,
        getting_no_item: state_id == COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID,
        dropping_item: state_id == COPPER_GOLEM_STATE_DROPPING_ITEM_ID,
        dropping_no_item: state_id == COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID,
    }
}

/// Vanilla `Pillager.isChargingCrossbow()` (`entityData.get(IS_CHARGING_CROSSBOW)`): the synced boolean
/// that drives the `CROSSBOW_CHARGE` draw — the client counts ticks since it rose for `getTicksUsingItem`.
/// The generated accessor supplies Pillager membership, wire index and default while excluding other
/// illagers with unrelated metadata at the same slot.
pub(crate) fn pillager_is_charging_crossbow(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> bool {
    vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `Piglin.isChargingCrossbow()` (`entityData.get(DATA_IS_CHARGING_CROSSBOW)`): the synced boolean
/// that drives the regular piglin's `CROSSBOW_CHARGE` draw — the client counts ticks since it rose for
/// `getTicksUsingItem`. The generated accessor supplies regular-Piglin membership, wire index and default,
/// excluding PiglinBrute and unrelated slot-18 accessors.
pub(crate) fn piglin_is_charging_crossbow(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> bool {
    vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `LivingEntity.isUsingItem()` (the named using-item bit in
/// `DATA_LIVING_ENTITY_FLAGS`): true while the entity holds right-click on a usable item. For the
/// player this drives the shared crossbow-draw counter
/// (the client's `getTicksUsingItem` reconstruction, which is item-agnostic in vanilla); the native layer
/// then applies the `CROSSBOW_CHARGE` pose only when the using item is actually an uncharged crossbow. Read
/// straight from the entity metadata in the tick loop; the type gate in `advance_client_tick` keeps this on
/// the player.
pub(crate) fn player_is_using_item(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id).is_some_and(
        |accessors| {
            entity_data_byte(
                data_values,
                accessors.flags_data_id,
                accessors.flags_default,
            ) & accessors.is_using_mask
                != 0
        },
    )
}

/// Vanilla `Allay.isDancing()` (`entityData.get(DATA_DANCING)`): the synced boolean that drives the
/// dance/spin accumulators. The generated accessor supplies membership, wire index and default.
pub(crate) fn allay_is_dancing(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_allay_dancing_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `Axolotl.isPlayingDead()` (`entityData.get(DATA_PLAYING_DEAD)`): the synced boolean that
/// selects the `PLAYING_DEAD` animation state. The generated accessor supplies membership, wire
/// index and default.
pub(crate) fn axolotl_is_playing_dead(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> bool {
    vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `Creaking.canMove()` (`entityData.get(CAN_MOVE)`): the synced boolean (default `true`)
/// that gates the looping walk. The generated accessor supplies membership, wire index and default;
/// non-creakings fall back to `true` because the field only feeds the Creaking model.
pub(crate) fn creaking_can_move(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or(true)
}

/// Vanilla `Creaking.isTearingDown()` (`entityData.get(IS_TEARING_DOWN)`): the synced boolean
/// (default `false`) that drives the death collapse. The generated accessor supplies membership,
/// wire index and default; non-creakings fall back to `false`.
pub(crate) fn creaking_is_tearing_down(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> bool {
    vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `Sniffer.getState()` (`entityData.get(DATA_STATE)`). The generated accessor supplies
/// membership, wire index and the `IDLING` typed default; state ordinals remain generator-owned.
fn sniffer_state_id(entity_type_id: i32, data_values: &[EntityDataValue]) -> Option<i32> {
    vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(entity_type_id).map(
        |(data_id, default)| {
            entity_data_enum_id(
                data_values,
                data_id,
                EntityDataEnumSerializer::SnifferState,
                default,
            )
        },
    )
}

pub(crate) fn sniffer_can_play_digging_sound(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> bool {
    matches!(
        sniffer_state_id(entity_type_id, data_values),
        Some(SNIFFER_STATE_DIGGING_ID | SNIFFER_STATE_SEARCHING_ID)
    )
}

/// Vanilla `LivingEntity.isFallFlying()` reads the generated base Entity shared-flags accessor and
/// its named fall-flying mask.
pub(crate) fn entity_is_fall_flying(data_values: &[EntityDataValue]) -> bool {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    entity_data_byte(
        data_values,
        accessors.shared_flags_data_id,
        accessors.shared_flags_default,
    ) as u8
        & accessors.fall_flying_mask as u8
        != 0
}

/// Vanilla `Entity.isSwimming()` reads the generated base Entity shared-flags
/// accessor and its named swimming mask. This is intentionally distinct from
/// the pose-backed `isVisuallySwimming()` used by `LivingEntity.updateSwimAmount`.
pub(crate) fn entity_is_swimming(data_values: &[EntityDataValue]) -> bool {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    entity_data_byte(
        data_values,
        accessors.shared_flags_data_id,
        accessors.shared_flags_default,
    ) as u8
        & accessors.swimming_mask as u8
        != 0
}

/// Vanilla `Warden.getHeartBeatDelay()` = `40 - floor(clamp(clientAngerLevel /
/// AngerLevel.ANGRY.minimumAnger, 0, 1) · 30)`: the period (in client ticks)
/// between heartbeats, shrinking from `40` (calm) to `10` (fully angry) as the
/// synced anger rises. The generated accessor supplies membership, wire index and typed default;
/// non-wardens fall back to the calm `40` without reading another family's slot `16`.
pub(crate) fn warden_heartbeat_delay(entity_type_id: i32, data_values: &[EntityDataValue]) -> u32 {
    let anger = vanilla_entity_metadata_warden_anger_level_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_int(data_values, data_id, default))
        .unwrap_or_default();
    let ratio = (anger as f32 / WARDEN_ANGRY_MINIMUM_ANGER as f32).clamp(0.0, 1.0);
    (40 - (ratio * 30.0).floor() as i32) as u32
}

impl WorldStore {
    pub(super) fn item_entity_accepts_projectile_hurt_client(
        &self,
        entity_id: i32,
        damage_type: &str,
    ) -> bool {
        let Some(stack) = self.entities.item_stack_for_entity(entity_id) else {
            // Empty ItemStack has no DAMAGE_RESISTANT component.
            return true;
        };
        if !self.item_stack_has_effective_component(
            &stack,
            VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        ) {
            return true;
        }
        let effective = self.effective_item_stack(&stack);
        self.registry_holder_id_by_resource_id("minecraft:damage_type", damage_type)
            .is_none_or(|damage_type_id| {
                !effective
                    .component_patch
                    .damage_resistant_type_registry_ids
                    .contains(&damage_type_id)
            })
    }

    pub fn advance_entity_client_animations(&mut self, ticks: u32) {
        let _ = self.advance_entity_client_animations_with_observation(ticks);
    }

    pub fn advance_entity_client_animations_with_observation(
        &mut self,
        ticks: u32,
    ) -> crate::EntityClientTickWork {
        if ticks == 0 || self.entity_count() == 0 {
            return crate::EntityClientTickWork::default();
        }

        // The entity-only benchmark/test API leaves the frozen-step budget and
        // level owners untouched, so derive the same per-tick gate without
        // mutating `WorldTickingState`. The runtime uses the ordered client-level
        // batch below, which consumes the budget and interleaves border/time.
        // Player roots and roots with indirect Player passengers still tick
        // while `runs_normally` is false; `EntityStore` applies that exception.
        let ticking = self.ticking();
        let running_ticks = if ticking.frozen {
            (ticking.frozen_ticks_to_run.max(0) as u32).min(ticks)
        } else {
            ticks
        };
        let mut game_time = self.world_time().map_or(0, |time| time.game_time);
        let mut work = crate::EntityClientTickWork::default();
        for tick_index in 0..ticks {
            let runs_normally = tick_index < running_ticks;
            work.accumulate(self.advance_entity_client_animation_tick(runs_normally, game_time));
            if runs_normally {
                // `Minecraft.tick` runs `ClientLevel.tickEntities` before
                // `ClientLevel.tickTime`. Predict the next tick's clock only
                // after this entity tick; the entity-only benchmark API does
                // not mutate the canonical level clock.
                game_time = game_time.saturating_add(1);
            }
        }
        work
    }

    /// Advances one native catch-up batch in Vanilla `Minecraft.tick` order.
    ///
    /// Each raw client tick first advances eligible entities against the current level clock and
    /// world border. A normally-running tick then advances the border and clock before the next
    /// entity tick. Frozen player exceptions still receive every raw entity tick, while the
    /// frozen step budget is consumed exactly once for the whole batch.
    pub fn advance_client_level_entity_ticks_with_observation(
        &mut self,
        ticks: u32,
    ) -> (crate::EntityClientTickWork, u32) {
        if ticks == 0 {
            return (crate::EntityClientTickWork::default(), 0);
        }

        let running_ticks = self.consume_running_render_ticks(ticks);
        let mut work = crate::EntityClientTickWork::default();
        for tick_index in 0..ticks {
            let runs_normally = tick_index < running_ticks;
            if self.entity_count() != 0 {
                let game_time = self.world_time().map_or(0, |time| time.game_time);
                work.accumulate(
                    self.advance_entity_client_animation_tick(runs_normally, game_time),
                );
            }
            self.advance_local_player_attack_strength_ticks(1);
            if runs_normally {
                // Vanilla `ClientLevel.tick` advances these owners after
                // `tickEntities`, in this order, for every catch-up tick.
                self.advance_world_border(1);
                self.advance_client_time(1);
            }
        }
        (work, running_ticks)
    }

    fn advance_entity_client_animation_tick(
        &mut self,
        runs_normally: bool,
        game_time: i64,
    ) -> crate::EntityClientTickWork {
        let preparation_started_at = std::time::Instant::now();
        let has_client_level = self.level.is_some();
        let chunks = &self.chunks;
        let is_ticking_entity = |_: i32, entity_type_id: i32, position: EntityVec3| {
            !has_client_level
                || entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
                || chunks.iter().any(|chunk| {
                    chunk.pos
                        == crate::ChunkPos {
                            x: entity_chunk_coordinate(position.x),
                            z: entity_chunk_coordinate(position.z),
                        }
                })
        };
        let interpolation_collision_inputs = self.entities.interpolation_collision_inputs();
        let experience_orb_needs_entity_collision =
            self.entities.client_movement_has_experience_orb();
        let entity_collision_targets = if interpolation_collision_inputs.is_empty()
            && !experience_orb_needs_entity_collision
        {
            Vec::new()
        } else {
            self.entities.client_movement_entity_collision_targets()
        };
        let collision_free_interpolation_targets: std::collections::HashSet<i32> =
            interpolation_collision_inputs
                .into_iter()
                .filter(|input| {
                    self.entity_interpolation_target_is_collision_free(
                        *input,
                        &entity_collision_targets,
                    )
                })
                .map(|input| input.entity_id)
                .collect();
        let client_movement_fluid_inputs = self
            .entities
            .client_movement_fluid_inputs(&collision_free_interpolation_targets);
        // Vanilla `Guardian.aiStep` recomputes `isInWater()` each client tick. This
        // entry point normalizes batched callers to one pass per tick above, so the
        // AABBs and interpolation collision targets are sampled again after every
        // transform mutation. The immutable reads MUST precede the mutable advance.
        let in_water_inputs = self.entities.in_water_aabb_inputs();
        let mut fluid_facts_by_id = std::collections::HashMap::with_capacity(in_water_inputs.len());
        let mut guardian_can_stand_on_below_by_id = std::collections::HashMap::new();
        let mut guardian_attack_target_ids = std::collections::HashSet::new();
        for input in in_water_inputs {
            let exact_fluid_contact = (is_vanilla_abstract_nautilus_type(input.entity_type_id)
                || input.entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID)
                .then(|| {
                    crate::client::entity_fluid_contact(
                        self,
                        input.aabb_min,
                        input.aabb_max,
                        input.position.y,
                        input.eye_position,
                    )
                });
            let in_water = exact_fluid_contact.map_or_else(
                || crate::fluid::world_aabb_in_water(self, input.aabb_min, input.aabb_max),
                |contact| contact.in_water(),
            );
            let in_liquid = vanilla_entity_metadata_type_is_camel(input.entity_type_id)
                && crate::client::entity_bounds_contains_any_fluid(
                    self,
                    input.aabb_min,
                    input.aabb_max,
                );
            fluid_facts_by_id.insert(
                input.entity_id,
                super::store::EntityClientTickFluidFacts {
                    in_water,
                    in_lava: exact_fluid_contact.is_some_and(|contact| contact.in_lava()),
                    in_liquid,
                    eye_in_water: exact_fluid_contact.is_some_and(|contact| contact.eye_in_water),
                },
            );
            if let Some(pos) = input.guardian_below_pos.filter(|_| !in_water) {
                let can_stand = self.probe_block(pos).is_some_and(|block| {
                    crate::client::block_collision_shape_has_full_up_face(&block, pos)
                });
                guardian_can_stand_on_below_by_id.insert(input.entity_id, can_stand);
            }
            if input.guardian_attack_target_id != 0 {
                guardian_attack_target_ids.insert(input.guardian_attack_target_id);
            }
        }
        let local_player_pose_before_tick = self.local_player.pose;
        let local_player_can_be_pushed = local_player_pose_before_tick.is_some_and(|pose| {
            self.local_player
                .health
                .as_ref()
                .map_or(true, |health| health.health > 0.0)
                && !self.local_player_is_spectator()
                && !crate::client::local_player_is_on_climbable(self, pose)
        });
        let local_player_scoreboard_name = self.local_player_scoreboard_name();
        let has_terrain_chunks = !self.chunks.is_empty();
        let boat_world_facts = self
            .entities
            .prepare_client_tick_schedule(runs_normally, &is_ticking_entity)
            .into_iter()
            .map(|input| {
                let aabb_min = std::array::from_fn(|axis| {
                    [input.position.x, input.position.y, input.position.z][axis]
                        + input.relative_min[axis]
                });
                let aabb_max = std::array::from_fn(|axis| {
                    [input.position.x, input.position.y, input.position.z][axis]
                        + input.relative_max[axis]
                });
                let (status, water_level, land_friction) = if !has_terrain_chunks {
                    (super::EntityBoatStatus::InAir, 0.0, None)
                } else {
                    let (water_status, water_level) =
                        crate::fluid::boat_water_status(self, aabb_min, aabb_max);
                    if let Some(status) = water_status {
                        (status, water_level, None)
                    } else {
                        let friction = self.boat_ground_friction(
                            [input.position.x, input.position.y, input.position.z],
                            input.relative_min,
                            input.relative_max,
                        );
                        if friction > 0.0 {
                            (super::EntityBoatStatus::OnLand, water_level, Some(friction))
                        } else {
                            (super::EntityBoatStatus::InAir, water_level, None)
                        }
                    }
                };
                let pusher_scoreboard_name =
                    self.scoreboard_name_for_identity(input.entity_type_id, input.uuid);
                let can_push_local_player = local_player_can_be_pushed
                    && self.local_player_id != Some(input.entity_id)
                    && !self.entity_shares_local_player_root_vehicle(input.entity_id)
                    && self.scoreboard.entity_can_push_scoreboard_target(
                        &pusher_scoreboard_name,
                        local_player_scoreboard_name.as_deref(),
                    );
                super::store::EntityBoatClientTickWorldFacts {
                    entity_id: input.entity_id,
                    status,
                    water_level,
                    land_friction,
                    can_push_local_player,
                    relative_min: input.relative_min,
                    relative_max: input.relative_max,
                }
            })
            .collect::<Vec<_>>();
        let client_movement_fluid_by_id = client_movement_fluid_inputs
            .iter()
            .map(|input| {
                (
                    input.entity_id,
                    crate::client::entity_fluid_contact(
                        self,
                        input.aabb_min,
                        input.aabb_max,
                        input.position.y,
                        input.eye_position,
                    ),
                )
            })
            .collect::<std::collections::HashMap<_, _>>();
        let client_movement_world_fact_position_by_id = client_movement_fluid_inputs
            .iter()
            .map(|input| (input.entity_id, input.position))
            .collect::<std::collections::HashMap<_, _>>();
        let client_movement_collision_free_by_id = client_movement_fluid_inputs
            .iter()
            .filter(|input| input.entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID)
            .map(|input| {
                (
                    input.entity_id,
                    self.entity_interpolation_target_is_collision_free(
                        super::EntityInterpolationCollisionInput {
                            entity_id: input.entity_id,
                            entity_type_id: input.entity_type_id,
                            current_position: input.position,
                            target_position: input.position,
                            bounds: input.bounds,
                        },
                        &entity_collision_targets,
                    ),
                )
            })
            .collect::<std::collections::HashMap<_, _>>();
        let spectator_player_uuids: std::collections::HashSet<_> = self
            .player_info()
            .entries
            .iter()
            .filter_map(|(uuid, info)| info.is_spectator().then_some(*uuid))
            .collect();
        let mut abstract_arrow_collision_batch = self
            .entities
            .abstract_arrow_collision_batch(&spectator_player_uuids);
        let item_target_ids = abstract_arrow_collision_batch
            .targets
            .iter()
            .filter(|target| target.entity_type_id == VANILLA_ENTITY_TYPE_ITEM_ID)
            .map(|target| target.entity_id)
            .collect::<Vec<_>>();
        for entity_id in item_target_ids {
            abstract_arrow_collision_batch.set_item_hurt_client(
                entity_id,
                self.item_entity_accepts_projectile_hurt_client(entity_id, "minecraft:arrow"),
                self.item_entity_accepts_projectile_hurt_client(entity_id, "minecraft:trident"),
            );
        }
        let abstract_arrow_collisions_by_id =
            super::client_movement::resolve_abstract_arrow_collision_facts(
                abstract_arrow_collision_batch,
                |source| {
                    self.projectile_position_inside_block_collision(
                        source.entity_type_id,
                        [source.position.x, source.position.y, source.position.z],
                        source.on_ground,
                    )
                },
                |entity_type_id, position, movement, on_ground, collision_context_y| {
                    self.clip_projectile_block_movement_with_context_y(
                        entity_type_id,
                        [position.x, position.y, position.z],
                        [movement.x, movement.y, movement.z],
                        on_ground,
                        collision_context_y,
                    )
                },
                |owner_uuid, target_uuid| {
                    let owner_is_loaded_player = self
                        .entities
                        .protocol_id_for_uuid(owner_uuid)
                        .and_then(|id| self.entities.identity(id))
                        .is_some_and(|identity| {
                            identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
                        });
                    if !owner_is_loaded_player {
                        return true;
                    }
                    let owner_name = self
                        .scoreboard_name_for_identity(VANILLA_ENTITY_TYPE_PLAYER_ID, owner_uuid);
                    let target_name = self
                        .scoreboard_name_for_identity(VANILLA_ENTITY_TYPE_PLAYER_ID, target_uuid);
                    self.scoreboard
                        .player_can_harm_player(&owner_name, &target_name)
                },
            );
        let lava_current_scale = crate::client::local_player_lava_current_push_per_tick(self);
        let client_movement_collisions_by_id = self
            .entities
            .client_movement_collision_inputs(
                &client_movement_fluid_by_id,
                &client_movement_world_fact_position_by_id,
                &client_movement_collision_free_by_id,
                &spectator_player_uuids,
                lava_current_scale,
                game_time,
            )
            .into_iter()
            .map(|input| {
                let (requested_movement, fishing_hook_reverse_deflection) =
                    if let Some(collision_delta) = input.fishing_hook_collision_delta {
                        let clipped = self.clip_projectile_block_movement(
                            input.entity_type_id,
                            [input.position.x, input.position.y, input.position.z],
                            [collision_delta.x, collision_delta.y, collision_delta.z],
                            input.on_ground,
                        );
                        let block_clipped = EntityVec3 {
                            x: clipped.movement[0],
                            y: clipped.movement[1],
                            z: clipped.movement[2],
                        };
                        let collision_length = (collision_delta.x * collision_delta.x
                            + collision_delta.y * collision_delta.y
                            + collision_delta.z * collision_delta.z)
                            .sqrt();
                        let block_scale = if collision_length == 0.0 {
                            1.0
                        } else {
                            (block_clipped.x * block_clipped.x
                                + block_clipped.y * block_clipped.y
                                + block_clipped.z * block_clipped.z)
                                .sqrt()
                                / collision_length
                        };
                        let entity_hit = input
                            .fishing_hook_entity_hit
                            .filter(|entity_hit| entity_hit.scale < block_scale);
                        let (checked, reverse_deflection) = if let Some(entity_hit) = entity_hit {
                            if let Some((entity_id, incarnation, position)) =
                                entity_hit.reverse_deflection
                            {
                                (
                                    EntityVec3 {
                                        x: collision_delta.x * -0.5,
                                        y: collision_delta.y * -0.5,
                                        z: collision_delta.z * -0.5,
                                    },
                                    Some(super::client_movement::EntityFishingHookReverseDeflection::Entity {
                                        entity_id,
                                        incarnation,
                                        position,
                                    }),
                                )
                            } else {
                                (collision_delta, None)
                            }
                        } else if clipped.world_border_hit {
                            let reversed = EntityVec3 {
                                x: collision_delta.x * -0.5,
                                y: collision_delta.y * -0.5,
                                z: collision_delta.z * -0.5,
                            };
                            (
                                EntityVec3 {
                                    x: reversed.x * 0.2,
                                    y: reversed.y * 0.2,
                                    z: reversed.z * 0.2,
                                },
                                Some(super::client_movement::EntityFishingHookReverseDeflection::WorldBorder),
                            )
                        } else {
                            (block_clipped, None)
                        };
                        (
                            EntityVec3 {
                                x: checked.x + input.requested_movement.x - collision_delta.x,
                                y: checked.y + input.requested_movement.y - collision_delta.y,
                                z: checked.z + input.requested_movement.z - collision_delta.z,
                            },
                            reverse_deflection,
                        )
                    } else {
                        (input.requested_movement, None)
                    };
                let clipped = self.clip_entity_collision_movement(
                    [input.position.x, input.position.y, input.position.z],
                    [
                        requested_movement.x,
                        requested_movement.y,
                        requested_movement.z,
                    ],
                    input.bounds.min.map(f64::from),
                    input.bounds.max.map(f64::from),
                );
                let on_ground = requested_movement.y < 0.0 && clipped[1] != requested_movement.y;
                let supporting_block = on_ground
                    .then(|| {
                        self.entity_supporting_block_after_collision(
                            input.entity_type_id,
                            [input.position.x, input.position.y, input.position.z],
                            clipped,
                            input.bounds.min.map(f64::from),
                            input.bounds.max.map(f64::from),
                            input.previous_supporting_block,
                        )
                    })
                    .flatten();
                let new_position = [
                    input.position.x + clipped[0],
                    input.position.y + clipped[1],
                    input.position.z + clipped[2],
                ];
                let friction_pos = crate::BlockPos {
                    x: supporting_block.map_or_else(|| new_position[0].floor() as i32, |pos| pos.x),
                    y: (new_position[1] - f64::from(0.999999_f32)).floor() as i32,
                    z: supporting_block.map_or_else(|| new_position[2].floor() as i32, |pos| pos.z),
                };
                let ground_friction = crate::terrain::vanilla_block_friction(
                    self.probe_block(friction_pos)
                        .as_ref()
                        .and_then(|block| block.block_name.as_deref()),
                );
                (
                    input.entity_id,
                    super::client_movement::EntityClientMovementCollisionResult {
                        pre_move_delta: if input.fishing_hook_update.is_some() {
                            requested_movement
                        } else {
                            input.pre_move_delta
                        },
                        requested_movement,
                        clipped_movement: EntityVec3 {
                            x: clipped[0],
                            y: clipped[1],
                            z: clipped[2],
                        },
                        supporting_block,
                        ground_friction,
                        movement_skipped: input.movement_skipped,
                        experience_orb_update: input.experience_orb_update,
                        fishing_hook_update: input.fishing_hook_update,
                        fishing_hook_reverse_deflection,
                    },
                )
            })
            .collect::<std::collections::HashMap<_, _>>();
        let death_tick_player_chunk = self.local_player.pose.map(|pose| {
            (
                entity_chunk_coordinate(pose.position.x),
                entity_chunk_coordinate(pose.position.z),
            )
        });
        let death_tick_simulation_distance = self.local_player.simulation_distance;
        let minecart_improvements_enabled =
            self.is_feature_enabled("minecraft:minecart_improvements");
        let deferred_boat_paddle_entity_id =
            self.local_player_first_passenger_root_boat_vehicle_id();
        let deferred_local_camel_vehicle_id = self
            .local_player_first_passenger_root_vehicle_id()
            .filter(|entity_id| {
                self.entities
                    .entity_type_id(*entity_id)
                    .is_some_and(vanilla_entity_metadata_type_is_camel)
            });
        let deferred_local_nautilus_vehicle_id = self
            .local_player_first_passenger_root_vehicle_id()
            .filter(|entity_id| {
                self.entities
                    .entity_type_id(*entity_id)
                    .is_some_and(is_vanilla_abstract_nautilus_type)
                    && self.entity_controlling_passenger(*entity_id) == self.local_player_id
            });
        let deferred_local_item_based_steering_vehicle_id = self
            .local_player_first_passenger_root_vehicle_id()
            .filter(|entity_id| {
                self.entities
                    .entity_type_id(*entity_id)
                    .and_then(
                        vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type,
                    )
                    .is_some()
                    && self.entity_controlling_passenger(*entity_id) == self.local_player_id
            });
        let local_player_rotation = self.local_player.pose.map(|pose| (pose.x_rot, pose.y_rot));
        let local_player_is_vehicle = self
            .local_player_id
            .and_then(|entity_id| self.entities.mount(entity_id))
            .is_some_and(|mount| !mount.passengers.is_empty());
        let local_player_holds_carrot_on_a_stick = self
            .local_player_holds_item(bbb_protocol::item_types::VANILLA_ITEM_CARROT_ON_A_STICK_ID);
        let local_player_holds_warped_fungus_on_a_stick = self.local_player_holds_item(
            bbb_protocol::item_types::VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
        );
        let can_carry_interpolation_target =
            |entity_id: i32| collision_free_interpolation_targets.contains(&entity_id);
        let mut block_effect_world = crate::client::EntityBlockEffectWorld::new(
            self.dimension,
            &self.registries,
            &self.chunks,
            self.entity_block_effect_weather(),
        );
        let preparation_duration = preparation_started_at.elapsed();
        let mut work = self.entities.advance_client_animations(
            1,
            game_time,
            deferred_boat_paddle_entity_id,
            deferred_local_camel_vehicle_id,
            deferred_local_nautilus_vehicle_id,
            deferred_local_item_based_steering_vehicle_id,
            &self.items.default_item_equipment_slots,
            self.local_player_id,
            local_player_rotation,
            local_player_holds_carrot_on_a_stick,
            local_player_holds_warped_fungus_on_a_stick,
            &spectator_player_uuids,
            &fluid_facts_by_id,
            &boat_world_facts,
            self.local_player.pose.as_mut(),
            local_player_is_vehicle,
            &guardian_can_stand_on_below_by_id,
            &guardian_attack_target_ids,
            &client_movement_collisions_by_id,
            &abstract_arrow_collisions_by_id,
            minecart_improvements_enabled,
            death_tick_player_chunk,
            death_tick_simulation_distance,
            &can_carry_interpolation_target,
            &mut block_effect_world,
        );
        let local_discards = self.entities.take_local_discards();
        drop(block_effect_world);
        for entity_id in local_discards {
            self.discard_local_entity(entity_id);
        }
        work.preparation_duration += preparation_duration;
        self.counters.minecart_lerp_steps_tracked = self.entities.total_minecart_lerp_steps();
        work
    }
}

pub(super) fn entity_chunk_coordinate(position: f64) -> i32 {
    (position.floor() as i32).div_euclid(16)
}

fn entity_block_position(position: EntityVec3) -> [i32; 3] {
    [
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    ]
}

pub(crate) fn boat_paddle_states(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
    has_controlling_passenger: bool,
) -> [bool; 2] {
    let Some(accessors) =
        vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(entity_type_id)
    else {
        return [false; 2];
    };
    [
        has_controlling_passenger
            && entity_data_bool(
                data_values,
                accessors.paddle_left_data_id,
                accessors.paddle_left_default,
            ),
        has_controlling_passenger
            && entity_data_bool(
                data_values,
                accessors.paddle_right_data_id,
                accessors.paddle_right_default,
            ),
    ]
}

pub(crate) fn boat_bubble_time(entity_type_id: i32, data_values: &[EntityDataValue]) -> i32 {
    let Some(accessors) =
        vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(entity_type_id)
    else {
        return 0;
    };
    entity_data_int(
        data_values,
        accessors.bubble_time_data_id,
        accessors.bubble_time_default,
    )
}

/// Vanilla `Bat.isResting()` (`DATA_ID_FLAGS & FLAG_RESTING`), resolved through the generated Bat
/// accessor so the shared byte slot never leaks to another entity type.
pub(crate) fn bat_is_resting(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_bat_flags_accessor_for_entity_type(entity_type_id).is_some_and(
        |(data_id, default, resting_mask)| {
            entity_data_byte(data_values, data_id, default) & resting_mask != 0
        },
    )
}

pub(crate) fn entity_data_bool(
    data_values: &[EntityDataValue],
    data_id: u8,
    default: bool,
) -> bool {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Boolean(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

pub(crate) fn entity_data_long(data_values: &[EntityDataValue], data_id: u8, default: i64) -> i64 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Long(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

fn shulker_target_peek_amount(entity_type_id: i32, data_values: &[EntityDataValue]) -> f32 {
    let Some(accessors) = vanilla_entity_metadata_shulker_accessors_for_entity_type(entity_type_id)
    else {
        return 0.0;
    };
    f32::from(
        entity_data_byte(data_values, accessors.peek_data_id, accessors.peek_default).clamp(0, 100),
    ) * 0.01
}

/// Vanilla `Bee.isRolling()` = `getFlag(FLAG_ROLL)` = `(flags & 2) != 0`.
fn bee_is_rolling(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_bee_accessors_for_entity_type(entity_type_id).is_some_and(|accessors| {
        entity_data_byte(
            data_values,
            accessors.flags_data_id,
            accessors.flags_default,
        ) & accessors.roll_mask
            != 0
    })
}

fn panda_pose_flags(entity_type_id: i32, data_values: &[EntityDataValue]) -> (bool, bool, bool) {
    vanilla_entity_metadata_panda_accessors_for_entity_type(entity_type_id)
        .map(|accessors| {
            let flags = entity_data_byte(
                data_values,
                accessors.flags_data_id,
                accessors.flags_default,
            );
            (
                flags & accessors.sitting_mask != 0,
                flags & accessors.on_back_mask != 0,
                flags & accessors.rolling_mask != 0,
            )
        })
        .unwrap_or_default()
}

fn equine_pose_flags(entity_type_id: i32, data_values: &[EntityDataValue]) -> (bool, bool, bool) {
    vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id)
        .map(|accessors| {
            let flags = entity_data_byte(
                data_values,
                accessors.flags_data_id,
                accessors.flags_default,
            );
            (
                flags & accessors.eating_mask != 0,
                flags & accessors.standing_mask != 0,
                flags & accessors.open_mouth_mask != 0,
            )
        })
        .unwrap_or_default()
}

/// Vanilla `Wolf.isInterested()` = `entityData.get(DATA_INTERESTED_ID)`. The generated accessor
/// supplies Wolf membership, wire index and default before the client-side head-roll ease reads it.
pub(crate) fn wolf_is_interested(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_wolf_interested_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `Cat.isLying()` = `entityData.get(IS_LYING)`. The generated accessor supplies Cat
/// membership, wire index and default for animation and sleeping-player projection consumers.
pub(crate) fn cat_is_lying(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_cat_lying_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `Cat.isRelaxStateOne()` = `entityData.get(RELAX_STATE_ONE)`. The generated accessor
/// supplies Cat membership, wire index and default for the relax amount ease.
pub(crate) fn cat_relax_state_one(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_cat_relax_state_one_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `Creeper.tick`: the fuse advances by `getSwellDir()`, but an ignited
/// creeper forces the direction to `1`. Mirrors that effective direction from
/// the synced `DATA_SWELL_DIR` (default `-1`) and `DATA_IS_IGNITED`.
fn creeper_effective_swell_dir(entity_type_id: i32, data_values: &[EntityDataValue]) -> i32 {
    let Some(accessors) = vanilla_entity_metadata_creeper_accessors_for_entity_type(entity_type_id)
    else {
        return 0;
    };
    if entity_data_bool(
        data_values,
        accessors.ignited_data_id,
        accessors.ignited_default,
    ) {
        1
    } else {
        entity_data_int(
            data_values,
            accessors.swell_dir_data_id,
            accessors.swell_dir_default,
        )
    }
}

fn entity_data_byte(data_values: &[EntityDataValue], data_id: u8, default: i8) -> i8 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Byte(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

fn entity_data_float(data_values: &[EntityDataValue], data_id: u8) -> Option<f32> {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Float(value) => Some(*value),
            _ => None,
        })
}

pub(crate) fn entity_data_int(data_values: &[EntityDataValue], data_id: u8, fallback: i32) -> i32 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Int(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(fallback)
}

/// Reads an `EntityDataSerializers` enum accessor (e.g. `ARMADILLO_STATE`, serializer 36), which is
/// wire-encoded as the enum's id VarInt and decoded into an [`EntityDataValueKind::EnumId`]. Returns
/// the `id` of the matching serializer, or `fallback` when the slot is absent or carries a different
/// serializer.
fn entity_data_enum_id(
    data_values: &[EntityDataValue],
    data_id: u8,
    serializer: EntityDataEnumSerializer,
    fallback: i32,
) -> i32 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::EnumId {
                serializer: value_serializer,
                id,
            } if *value_serializer == serializer => Some(*id),
            _ => None,
        })
        .unwrap_or(fallback)
}
