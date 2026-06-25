use bbb_protocol::packets::{EntityDataEnumSerializer, EntityDataValue, EntityDataValueKind};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

use super::dimensions::{entity_data_pose, vanilla_living_entity_type};
use super::dragon::{
    EnderDragonAnimationState, ENDER_DRAGON_PHASE_DATA_ID, ENDER_DRAGON_PHASE_HOVERING_ID,
    VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
};
use super::{EntityTransform, EntityVec3};

const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
const VANILLA_ENTITY_TYPE_CREEPER_ID: i32 = 32;
/// Vanilla `EntityType.ELDER_GUARDIAN` / `EntityType.GUARDIAN`. Both share
/// `GuardianModel` and the same client `Guardian.aiStep` tail accumulator (the
/// elder is the guardian mesh scaled 2.35×, with no animation override).
const VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID: i32 = 40;
const VANILLA_ENTITY_TYPE_GUARDIAN_ID: i32 = 63;
/// Vanilla `Guardian.DATA_ID_MOVING` (`isMoving()`): the guardian's first own
/// synced accessor after `Entity` (`0..=7`), `LivingEntity` (`8..=14`) and `Mob`
/// (`15`). A `Boolean`, default `false`.
const GUARDIAN_MOVING_DATA_ID: u8 = 16;
/// Vanilla `Guardian.DATA_ID_ATTACK_TARGET`, the int synced right after `DATA_ID_MOVING` (`17`).
const GUARDIAN_ATTACK_TARGET_DATA_ID: u8 = 17;
const VANILLA_ENTITY_TYPE_GLOW_SQUID_ID: i32 = 61;
const VANILLA_ENTITY_TYPE_POLAR_BEAR_ID: i32 = 104;
const VANILLA_ENTITY_TYPE_SHEEP_ID: i32 = 111;
const VANILLA_ENTITY_TYPE_SHULKER_ID: i32 = 112;
const VANILLA_ENTITY_TYPE_SQUID_ID: i32 = 127;
const VANILLA_ENTITY_TYPE_WARDEN_ID: i32 = 142;
/// Vanilla `FlyingAnimal` implementors (`Bee`, `Parrot`): their
/// `LivingEntity.calculateEntityAnimation` measures the full 3-D travel distance
/// (`calculateEntityAnimation(this instanceof FlyingAnimal)`), so the limb-swing
/// distance includes the vertical component.
const VANILLA_ENTITY_TYPE_BEE_ID: i32 = 11;
const VANILLA_ENTITY_TYPE_PARROT_ID: i32 = 98;
/// Entities whose `updateWalkAnimation` override (`Camel`, `Creaking`, `Frog`)
/// replaces the base distance→speed mapping. `Camel`/`Frog` additionally gate on
/// pose/jump/dash animation states the client does not yet track, so their limb
/// swing is deferred rather than approximated with the base mapping.
const VANILLA_ENTITY_TYPE_CAMEL_ID: i32 = 19;
const VANILLA_ENTITY_TYPE_CREAKING_ID: i32 = 31;
const VANILLA_ENTITY_TYPE_FROG_ID: i32 = 55;
/// Vanilla `Pose.CROAKING` ordinal (`Pose.CROAKING(8, …)`), the synced `DATA_POSE` int value that
/// `Frog.onSyncedDataUpdated` reads to start/stop `croakAnimationState` (`animateWhen(pose ==
/// CROAKING, tickCount)`).
const VANILLA_POSE_CROAKING_ID: i32 = 8;
/// Vanilla `Pose.LONG_JUMPING` ordinal (`Pose.LONG_JUMPING(6, …)`), the synced `DATA_POSE` int value
/// that `Frog.onSyncedDataUpdated` reads to start/stop `jumpAnimationState` (`pose == LONG_JUMPING`
/// starts it, otherwise stops it).
const VANILLA_POSE_LONG_JUMPING_ID: i32 = 6;
const VANILLA_ENTITY_TYPE_SNIFFER_ID: i32 = 119;
/// Vanilla `Sniffer.DATA_STATE`, the synced `EntityDataSerializers.SNIFFER_STATE` accessor serialized
/// as the `Sniffer.State` ordinal VarInt. The accessor sits at id 18 (Entity 0–7, LivingEntity 8–14,
/// Mob 15, AgeableMob 16–17, then Sniffer's first own accessor); `Sniffer.onSyncedDataUpdated` reads
/// it to `resetAnimations()` and `startIfStopped` the matching one-shot `AnimationState`.
const SNIFFER_STATE_DATA_ID: u8 = 18;
/// `Sniffer.State` ordinals (the `State(id)` declaration order, which is the serialized VarInt).
/// `IDLING` and `SEARCHING` have no triggered one-shot keyframe (idle rests, search drives the
/// looping search-walk), so only the remaining five start an `AnimationState`.
const SNIFFER_STATE_IDLING_ID: i32 = 0;
const SNIFFER_STATE_FEELING_HAPPY_ID: i32 = 1;
const SNIFFER_STATE_SCENTING_ID: i32 = 2;
const SNIFFER_STATE_SNIFFING_ID: i32 = 3;
const SNIFFER_STATE_DIGGING_ID: i32 = 5;
const SNIFFER_STATE_RISING_ID: i32 = 6;
const VANILLA_ENTITY_TYPE_ARMADILLO_ID: i32 = 4;
/// Vanilla `Armadillo.ARMADILLO_STATE`, the synced `EntityDataSerializers.ARMADILLO_STATE` accessor
/// serialized as the `ArmadilloState` ordinal-id VarInt. Like `Sniffer.DATA_STATE` it is the first
/// own accessor after `AgeableMob` (Entity 0–7, LivingEntity 8–14, Mob 15, AgeableMob 16–17), id 18.
/// `Armadillo.onSyncedDataUpdated` reads it to `setupAnimationStates()`.
const ARMADILLO_STATE_DATA_ID: u8 = 18;
/// `ArmadilloState` ids (the 4th enum-ctor arg, which is the serialized VarInt; this is the declared
/// order too). Each state carries an `animationDuration` and a `shouldHideInShell(ticksInState)`.
const ARMADILLO_STATE_IDLE_ID: i32 = 0;
const ARMADILLO_STATE_ROLLING_ID: i32 = 1;
const ARMADILLO_STATE_SCARED_ID: i32 = 2;
const ARMADILLO_STATE_UNROLLING_ID: i32 = 3;
const POLAR_BEAR_STANDING_DATA_ID: u8 = 18;
const POLAR_BEAR_STAND_ANIMATION_TICKS: f32 = 6.0;
/// Vanilla `Creeper.DATA_SWELL_DIR` (synced fuse direction, default `-1`) and
/// `DATA_IS_IGNITED`; the client advances `swell` toward `maxSwell` while the
/// effective direction is positive.
const CREEPER_SWELL_DIR_DATA_ID: u8 = 16;
const CREEPER_IGNITED_DATA_ID: u8 = 18;
/// Vanilla `Creeper.DEFAULT_MAX_SWELL`. `getSwelling` divides the lerped swell
/// by `maxSwell - 2`.
const CREEPER_MAX_SWELL: i32 = 30;
const SHULKER_PEEK_DATA_ID: u8 = 17;
const SHULKER_PEEK_PER_TICK: f32 = 0.05;
const SHULKER_MAX_PEEK_AMOUNT: f32 = 1.0;
/// Vanilla `Bee.DATA_FLAGS_ID` synced metadata id: `Entity` (`0..=7`),
/// `LivingEntity` (`8..=14`), `Mob` (`15`) and `AgeableMob` (`16..=17`) precede
/// the bee's own flags byte at `18` (the same slot `PolarBear` uses for its
/// standing flag). `FLAG_ROLL` is mask `2` within that byte.
const BEE_FLAGS_DATA_ID: u8 = 18;
const BEE_FLAG_ROLL: i8 = 2;
/// Vanilla `Fox` entity type id (`EntityType.FOX`).
const VANILLA_ENTITY_TYPE_FOX_ID: i32 = 54;
const VANILLA_ENTITY_TYPE_GOAT_ID: i32 = 62;
const VANILLA_ENTITY_TYPE_HOGLIN_ID: i32 = 64;
const VANILLA_ENTITY_TYPE_IRON_GOLEM_ID: i32 = 70;
const VANILLA_ENTITY_TYPE_RAVAGER_ID: i32 = 109;
const VANILLA_ENTITY_TYPE_ZOGLIN_ID: i32 = 149;
/// Vanilla `Fox.DATA_FLAGS_ID` synced metadata id: `Entity` (`0..=7`),
/// `LivingEntity` (`8..=14`), `Mob` (`15`) and `AgeableMob` (`16..=17`) precede
/// the fox's own `DATA_TYPE_ID` (`18`, the variant int) and then the flags byte
/// (`19`). `getFlag(mask) = (byte19 & mask) != 0`.
const FOX_FLAGS_DATA_ID: u8 = 19;
/// Vanilla `Fox.FLAG_CROUCHING`: the synced crouch flag that drives both the
/// client `crouchAmount` accumulator and the `FoxModel.setCrouchingPose`.
const FOX_FLAG_CROUCHING: i8 = 4;
/// Vanilla `Fox.FLAG_INTERESTED`: the synced interest flag that drives the
/// client `interestedAngle` accumulator (the head tilt). Not a render-state
/// field itself — only its eased angle is projected.
const FOX_FLAG_INTERESTED: i8 = 8;
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
/// Vanilla `LivingEntity.getCurrentSwingDuration()` default: the melee swing ramps
/// `attackAnim` from `0` to `1` over this many client ticks (`getSwingAnimation()
/// .duration()`, `6` for the empty hand and the common items). The per-item swing
/// duration and the dig-speed / mining-fatigue modifiers are deferred, so the swing
/// always runs the default `6`-tick whack.
const ATTACK_SWING_DURATION: i32 = 6;
/// Vanilla `LivingEntity.DATA_HEALTH_ID` synced metadata id: `Entity` defines
/// ids `0..=7`, then `LivingEntity` adds the flags byte (8) and the health float
/// (9). `LivingEntity.isDeadOrDying` is `getHealth() <= 0`.
const VANILLA_ENTITY_HEALTH_DATA_ID: u8 = 9;
/// Vanilla `LivingEntity.tickDeath` removes the entity at `deathTime >= 20`, and
/// the death tip-over flip is fully clamped by then, so the client counter is
/// capped here to stay bounded when no server removal arrives.
const DEATH_ANIMATION_MAX_TICKS: i32 = 20;
/// Vanilla `Sheep.EAT_ANIMATION_TICKS`: the eat-grass animation runs for 40
/// client ticks after entity event `10`.
const SHEEP_EAT_ANIMATION_TICKS: i32 = 40;
/// Vanilla `Sheep.handleEntityEvent` triggers the eat-grass animation on event
/// id `10` (`EntityEvent.EAT_GRASS`).
const SHEEP_EAT_GRASS_EVENT_ID: i8 = 10;
/// Vanilla `Warden`: the client decrements `tendrilAnimation` from `10` toward
/// `0` each tick; `getTendrilAnimation` divides the lerped value by `10`.
const WARDEN_TENDRIL_ANIMATION_TICKS: i32 = 10;
/// Vanilla `Warden.handleEntityEvent` resets `tendrilAnimation` to `10` on event
/// id `61` (a received vibration signal).
const WARDEN_TENDRIL_EVENT_ID: i8 = 61;
/// Vanilla `Warden`: the client resets `heartAnimation` to `10` on each heartbeat
/// and decrements it toward `0` each tick; `getHeartAnimation` divides the lerped
/// value by `10`. Shares the `tendrilAnimation` range, hence the same `10` cap.
const WARDEN_HEART_ANIMATION_TICKS: i32 = 10;
/// Vanilla `Warden.CLIENT_ANGER_LEVEL` (`getClientAngerLevel()`): the warden's
/// first own synced accessor after `Entity` (`0..=7`), `LivingEntity` (`8..=14`)
/// and `Mob` (`15`); `Monster` adds none. An `Integer`, default `0`.
const WARDEN_ANGER_LEVEL_DATA_ID: u8 = 16;
/// Vanilla `AngerLevel.ANGRY.getMinimumAnger()`: the anger at which the heartbeat
/// reaches its fastest, used to normalise `getHeartBeatDelay`.
const WARDEN_ANGRY_MINIMUM_ANGER: i32 = 80;
/// Vanilla `Pose.ROARING` ordinal, the synced `DATA_POSE` int value that
/// `Warden.onSyncedDataUpdated` reads to `.start()` the `roarAnimationState` when
/// the pose CHANGES to it (the 4.2s `WARDEN_ROAR` keyframe animation).
const VANILLA_POSE_ROARING_ID: i32 = 11;
/// Vanilla `Pose.SNIFFING` ordinal, the synced `DATA_POSE` int value that
/// `Warden.onSyncedDataUpdated` reads to `.start()` the `sniffAnimationState` when
/// the pose CHANGES to it (the 4.16s `WARDEN_SNIFF` keyframe animation).
const VANILLA_POSE_SNIFFING_ID: i32 = 12;
/// Vanilla `Warden.handleEntityEvent(4)`: `roarAnimationState.stop()` then
/// `attackAnimationState.start(tickCount)` — the melee attack swing (the 0.33333s
/// `WARDEN_ATTACK` keyframe animation), which also cancels any running roar.
const WARDEN_ATTACK_EVENT_ID: i8 = 4;
/// Vanilla `Warden.handleEntityEvent(62)`: `sonicBoomAnimationState.start(tickCount)`
/// — the sonic-boom charge/blast (the 3.0s `WARDEN_SONIC_BOOM` keyframe animation).
const WARDEN_SONIC_BOOM_EVENT_ID: i8 = 62;
/// The "not in a triggered pose" sentinel for [`WardenCombatAnimationState::prev_pose`]:
/// no synced `DATA_POSE` has been observed yet, so the first ROARING/SNIFFING pose is a
/// fresh transition that starts the matching timer.
const WARDEN_POSE_UNSET: i32 = -1;
/// Vanilla `Squid.handleEntityEvent` resets `tentacleMovement` to `0` on event id
/// `19` (`EntityEvent.SQUID_RESET_MOVEMENT`, broadcast each time the server-side
/// `tentacleMovement` wraps past `2π`). Without it the client tentacles freeze at
/// `2π` after the first cycle.
const SQUID_RESET_MOVEMENT_EVENT_ID: i8 = 19;
/// Vanilla `Goat.handleEntityEvent`: event `58` sets `isLoweringHead = true` (the goat begins a ram),
/// event `59` clears it. The client `aiStep` then advances `lowerHeadTick` toward / away from the cap.
const GOAT_LOWER_HEAD_EVENT_ID: i8 = 58;
const GOAT_RAISE_HEAD_EVENT_ID: i8 = 59;
/// Vanilla `Goat.aiStep` clamps `lowerHeadTick` to `[0, 20]`: `++` while lowering, `-= 2` otherwise.
/// `getRammingXHeadRot` normalises it by this cap.
const GOAT_LOWER_HEAD_MAX_TICKS: i32 = 20;
/// Vanilla `IronGolem.handleEntityEvent`: event `4` sets `attackAnimationTick = 10` (the two-fisted
/// smash), event `11` sets `offerFlowerTick = 400` (offering a poppy to a villager), event `34` clears
/// it. Both counters decrement each client `aiStep` toward `0`.
const IRON_GOLEM_ATTACK_EVENT_ID: i8 = 4;
const IRON_GOLEM_OFFER_FLOWER_EVENT_ID: i8 = 11;
const IRON_GOLEM_STOP_OFFER_FLOWER_EVENT_ID: i8 = 34;
const IRON_GOLEM_ATTACK_TICKS: i32 = 10;
const IRON_GOLEM_OFFER_FLOWER_TICKS: i32 = 400;
/// Vanilla `Ravager.handleEntityEvent`: event `4` sets `attackTick = 10` (the bite), event `39` sets
/// `stunnedTick = 40` (a shield-block stun). The roar is not event-driven — when `stunnedTick` decays to
/// `0` the client `aiStep` sets `roarTick = 20`, so a roar always follows a stun.
const RAVAGER_ATTACK_EVENT_ID: i8 = 4;
const RAVAGER_STUN_EVENT_ID: i8 = 39;
const RAVAGER_ATTACK_TICKS: i32 = 10;
const RAVAGER_STUN_TICKS: i32 = 40;
const RAVAGER_ROAR_TICKS: i32 = 20;
/// Vanilla `Hoglin`/`Zoglin.handleEntityEvent`: event `4` sets `attackAnimationRemainingTicks = 10` (the
/// headbutt), decremented each client tick. `HoglinModel.setupAnim` drives the head-down ram from it.
const HOGLIN_ATTACK_EVENT_ID: i8 = 4;
const HOGLIN_ATTACK_TICKS: i32 = 10;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityClientAnimationState {
    #[serde(default)]
    pub age_ticks: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polar_bear_standing: Option<PolarBearStandingAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shulker_peek: Option<ShulkerPeekAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bee_roll: Option<BeeRollAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fox: Option<FoxAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frog_croak: Option<KeyframeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frog_jump: Option<KeyframeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frog_swim_idle: Option<KeyframeAnimationState>,
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
    pub hurt: Option<HurtAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack_swing: Option<AttackSwingAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death: Option<DeathAnimationState>,
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
    pub parrot_flap: Option<ParrotFlapAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guardian_tail: Option<GuardianTailAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guardian_attack: Option<GuardianAttackAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walk_animation: Option<WalkAnimationState>,
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

/// Canonical client-side melee-swing animation, mirroring vanilla `LivingEntity`'s
/// `swingTime`/`attackAnim`/`oAttackAnim`. The `ClientboundAnimate` packet (action `0`
/// main hand / `3` off hand) calls `swing()` ([`EntityClientAnimationState::trigger_swing`]),
/// which arms a [`ATTACK_SWING_DURATION`]-tick ramp; `updateSwingTime` advances it each
/// client tick and the renderer projects `getAttackAnim(partialTick)` into
/// `HumanoidModel.setupAttackAnimation`'s body twist + arm whack. `oAttackAnim` is kept for
/// the partial-tick lerp.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AttackSwingAnimationState {
    /// Vanilla `LivingEntity.swinging`: a swing is currently playing.
    pub swinging: bool,
    /// Vanilla `LivingEntity.swingTime`: the integer tick counter, `-1` for the tick
    /// just after `swing()` before the first `updateSwingTime`.
    pub swing_time: i32,
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

/// Canonical client-side bee barrel-roll animation, mirroring vanilla
/// `Bee.updateRollAmount`. While the synced `FLAG_ROLL` (mask `2` of the bee
/// flags byte, data id [`BEE_FLAGS_DATA_ID`]) is set, `rollAmount` climbs toward
/// `1` by `0.2`/tick; otherwise it falls toward `0` by `0.24`/tick.
/// `getRollAmount(partialTick)` lerps the pair, driving `BeeModel.setupAnim`'s
/// near-π `bone.xRot` flip that tips a rolling bee onto its back.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BeeRollAnimationState {
    pub rolling: bool,
    pub previous_roll_amount: f32,
    pub current_roll_amount: f32,
}

/// Canonical client-side triggered keyframe-animation state, mirroring vanilla
/// `net.minecraft.world.entity.AnimationState`: a one-shot timer that records the
/// `age_ticks` it started at and is cleared (`None`) when stopped. The renderer
/// projects the elapsed seconds since the start and wraps/samples the matching
/// `KeyframeAnimation` definition. Reusable for the whole triggered-keyframe tier
/// (the frog's croak is the first consumer; the warden/sniffer/camel/armadillo
/// triggered poses follow the same pattern).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Canonical client-side armadillo roll animation state, mirroring vanilla
/// `Armadillo.setupAnimationStates()`. The synced `ARMADILLO_STATE` (the `ArmadilloState` enum)
/// plus the `inStateTicks` counter (vanilla `Armadillo.inStateTicks`, reset to `0` on a state
/// change and `++` each tick) drive both the `isHidingInShell` shell-ball swap and the three
/// triggered keyframe `AnimationState`s: `rollUp` on entry to ROLLING, `rollOut` on entry to
/// UNROLLING, `peek` while SCARED. We reconstruct `inStateTicks` from the `age_ticks` recorded when
/// the state last changed, and reuse [`KeyframeAnimationState`] for the rollUp/rollOut elapsed
/// timers (started at the state-entry age, vanilla's `.startIfStopped(tickCount)`). The `peek`
/// animation, which vanilla `fastForward`s on the first SCARED tick, is deferred (see
/// `docs/unsupported-features.md`).
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
}

impl ArmadilloAnimationState {
    /// Vanilla `Armadillo.switchToState` + `setupAnimationStates`: a change to a different ordinal
    /// resets `inStateTicks` (so we re-anchor `state_change_age`) and `.startIfStopped`s the new
    /// state's transition timer (rollUp into ROLLING, rollOut into UNROLLING); the other timers
    /// stop. A redundant re-set to the same state keeps the running timers.
    fn set_state(&mut self, state_id: i32, age_ticks: u32) {
        if state_id == self.state_id {
            return;
        }
        self.state_id = state_id;
        self.state_change_age = age_ticks;
        self.roll_up.start_age = (state_id == ARMADILLO_STATE_ROLLING_ID).then_some(age_ticks);
        self.roll_out.start_age = (state_id == ARMADILLO_STATE_UNROLLING_ID).then_some(age_ticks);
    }

    /// Vanilla `Armadillo.shouldHideInShell()` = `getState().shouldHideInShell(inStateTicks)`,
    /// projected for the renderer `isHidingInShell` shell-ball swap.
    fn is_hiding_in_shell(self, age_ticks: u32) -> bool {
        armadillo_should_hide_in_shell(
            self.state_id,
            age_ticks.saturating_sub(self.state_change_age),
        )
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
}

impl RavagerAnimationState {
    /// Vanilla `Ravager.aiStep`: decrement the roar, attack, and stun timers; when the stun timer hits
    /// `0` it sets `roarTick = 20` (the post-stun roar).
    fn advance_client_tick(&mut self) {
        if self.roar_tick > 0 {
            self.roar_tick -= 1;
        }
        if self.attack_tick > 0 {
            self.attack_tick -= 1;
        }
        if self.stunned_tick > 0 {
            self.stunned_tick -= 1;
            if self.stunned_tick == 0 {
                self.roar_tick = RAVAGER_ROAR_TICKS;
            }
        }
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
/// Vanilla applies ALL FOUR additively in `WardenModel.setupAnim`, so each timer is projected
/// independently; the renderer applies every active one in the vanilla order (attack, sonic_boom,
/// [deferred dig/emerge], roar, sniff). The `EMERGING`/`DIGGING` poses are deferred (their large
/// spawn/despawn tables are not transcribed yet — see `docs/unsupported-features.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WardenCombatAnimationState {
    /// The last synced `DATA_POSE` ordinal observed, so a pose CHANGE into ROARING/SNIFFING is a
    /// rising edge that restarts the matching timer (vanilla `.start(tickCount)` on the
    /// transition). [`WARDEN_POSE_UNSET`] until the first pose arrives.
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
}

impl Default for WardenCombatAnimationState {
    fn default() -> Self {
        Self {
            prev_pose: WARDEN_POSE_UNSET,
            roar: KeyframeAnimationState { start_age: None },
            sniff: KeyframeAnimationState { start_age: None },
            attack: KeyframeAnimationState { start_age: None },
            sonic_boom: KeyframeAnimationState { start_age: None },
        }
    }
}

impl WardenCombatAnimationState {
    /// Vanilla `Warden.onSyncedDataUpdated(DATA_POSE)`: the pose-change `switch` that `.start()`s the
    /// roar/sniff timer when the pose CHANGES to `Pose.ROARING`/`Pose.SNIFFING`. A redundant re-set
    /// to the same pose is not a transition, so it leaves a running timer alone (vanilla only fires
    /// on a real `onSyncedDataUpdated` change). `EMERGING`/`DIGGING` are deferred, so they only
    /// update the tracked pose.
    fn set_pose(&mut self, pose_id: i32, age_ticks: u32) {
        if pose_id == self.prev_pose {
            return;
        }
        self.prev_pose = pose_id;
        match pose_id {
            VANILLA_POSE_ROARING_ID => self.roar.start_age = Some(age_ticks),
            VANILLA_POSE_SNIFFING_ID => self.sniff.start_age = Some(age_ticks),
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
/// Only the in-water branch is modelled (a squid is a water creature; the rare
/// out-of-water/suffocating branch is deferred — see [`Self::advance_client_tick`]).
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
}

impl Default for GuardianTailAnimationState {
    fn default() -> Self {
        // Vanilla seeds `clientSideTailAnimation = random.nextFloat()` in the
        // constructor (and `clientSideTailAnimationO = clientSideTailAnimation`).
        // That per-spawn RNG is non-deterministic, so we start the phase at `0.0`:
        // only the starting phase is an approximation of a value vanilla itself
        // randomizes — the per-tick sway dynamics below are exact.
        Self {
            tail_animation: 0.0,
            previous_tail_animation: 0.0,
            tail_animation_speed: 0.0,
        }
    }
}

impl GuardianTailAnimationState {
    /// Advances one client tick of `Guardian.aiStep`'s tail-sway accumulator.
    ///
    /// Vanilla saves the lerp endpoint (`clientSideTailAnimationO`), then ramps
    /// `clientSideTailAnimationSpeed`: out of water it snaps to `2.0`; in water and
    /// moving it jumps to `4.0` from a near-rest speed (`< 0.5`) else eases toward
    /// `0.5` by `0.1`; in water and idle it eases toward `0.125` by `0.2`. Finally
    /// it integrates `clientSideTailAnimation += clientSideTailAnimationSpeed`. The
    /// out-of-water flop-sound / `clientSideTouchedGround` bookkeeping in the same
    /// branch is server-adjacent audio and is intentionally not modelled here, and
    /// the `clientSideSpikesAnimation` attack withdrawal (which needs a per-tick
    /// `random.nextFloat()` out of water) stays deferred.
    fn advance_client_tick(&mut self, in_water: bool, is_moving: bool) {
        self.previous_tail_animation = self.tail_animation;
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
    }

    /// Vanilla `GuardianRenderer.extractRenderState`: `state.tailAnimation =
    /// entity.getTailAnimation(partialTicks) = Mth.lerp(partialTick,
    /// clientSideTailAnimationO, clientSideTailAnimation)`.
    fn tail_animation(&self, partial_tick: f32) -> f32 {
        self.previous_tail_animation
            + (self.tail_animation - self.previous_tail_animation) * partial_tick
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

/// Whether the entity type is a guardian or elder guardian (the two `Guardian` subclasses carrying the
/// attack beam + the `DATA_ID_ATTACK_TARGET` int at slot `17`).
pub(crate) fn is_guardian_entity_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_GUARDIAN_ID | VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID
    )
}

/// Vanilla `Guardian.getAttackDuration()`: `80` for a guardian, `60` for an elder guardian.
pub(crate) fn guardian_attack_duration(entity_type_id: i32) -> i32 {
    if entity_type_id == VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID {
        60
    } else {
        80
    }
}

/// Vanilla `Guardian.DATA_ID_ATTACK_TARGET` (synced int at index `17`, the active target entity id, or
/// `0` for none). Read straight from the entity metadata; non-guardians (whose slot `17` is not this
/// field) are gated out by the caller.
pub(crate) fn guardian_attack_target_id(data_values: &[EntityDataValue]) -> i32 {
    entity_data_int(data_values, GUARDIAN_ATTACK_TARGET_DATA_ID, 0)
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
        feet_position: EntityVec3,
        use_y: bool,
        is_passenger: bool,
        is_alive: bool,
        is_baby: bool,
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
        let target_speed = (distance * 4.0).min(1.0);
        self.update(target_speed, 0.4, if is_baby { 3.0 } else { 1.0 });
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
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_BEE_ID | VANILLA_ENTITY_TYPE_PARROT_ID
    )
}

/// Whether an entity's `updateWalkAnimation` override is not yet modelled, so its
/// limb swing is deferred rather than driven with the base distance→speed mapping.
fn walk_animation_override_is_deferred(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_CAMEL_ID
            | VANILLA_ENTITY_TYPE_CREAKING_ID
            | VANILLA_ENTITY_TYPE_FROG_ID
    )
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
    fn advance_client_tick(&mut self, age_ticks: u32, heartbeat_delay: u32) {
        if heartbeat_delay > 0 && age_ticks % heartbeat_delay == 0 {
            self.current = WARDEN_HEART_ANIMATION_TICKS;
        }
        self.previous = self.current;
        if self.current > 0 {
            self.current -= 1;
        }
    }

    /// Vanilla `Warden.getHeartAnimation(partialTick)`:
    /// `lerp(partialTick, heartAnimationO, heartAnimation) / 10`.
    fn heart_animation(self, partial_tick: f32) -> f32 {
        let lerped = self.previous as f32 + partial_tick * (self.current - self.previous) as f32;
        lerped / WARDEN_HEART_ANIMATION_TICKS as f32
    }
}

/// Java `java.util.Random` LCG `setSeed(seed)` then `nextFloat()`, used to seed
/// `Squid.tentacleSpeed` from the entity id deterministically on the client.
/// Replicates the multiplier/addend/mask of `LevelEventSoundRandomState` (the
/// audio module's Java-compatible LCG) so the value matches vanilla exactly.
fn java_random_first_next_float(seed: i64) -> f32 {
    const MULTIPLIER: u64 = 0x5DEEC_E66D;
    const ADDEND: u64 = 0xB;
    const MASK: u64 = (1_u64 << 48) - 1;
    // `setSeed(seed)`.
    let mut state = ((seed as u64) ^ MULTIPLIER) & MASK;
    // `next(24)`.
    state = state.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND) & MASK;
    let bits = (state >> (48 - 24)) as u32;
    // `nextFloat() = next(24) / (1 << 24)`.
    bits as f32 / (1_u32 << 24) as f32
}

impl SquidAnimationState {
    /// Vanilla `Squid` constructor: `random.setSeed(getId()); tentacleSpeed = 1 /
    /// (random.nextFloat() + 1) * 0.2`. Seeding by the entity id makes the speed
    /// deterministic on the client (the `aiStep` re-randomization is server-only).
    pub(crate) fn new(entity_id: i32) -> Self {
        let tentacle_speed = 1.0 / (java_random_first_next_float(i64::from(entity_id)) + 1.0) * 0.2;
        Self {
            tentacle_speed,
            tentacle_movement: 0.0,
            old_tentacle_movement: 0.0,
            tentacle_angle: 0.0,
            old_tentacle_angle: 0.0,
            x_body_rot: 0.0,
            old_x_body_rot: 0.0,
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

    /// Advances one client tick of `Squid.aiStep`, in-water branch.
    ///
    /// Vanilla saves the lerp endpoints (`xBodyRotO`/`zBodyRotO`/
    /// `oldTentacleMovement`/`oldTentacleAngle`), advances `tentacleMovement` by
    /// `tentacleSpeed`, and on the client clamps it at `2π` (the server instead
    /// resets to `0` and broadcasts event `19`). The in-water branch then derives
    /// `tentacleAngle`/`rotateSpeed` from the half-cycle position and turns the
    /// body roll/pitch from the synced velocity.
    ///
    /// DEFERRED: the out-of-water/suffocating branch (`!isInWater()`:
    /// `tentacleAngle = abs(sin(tentacleMovement)) * π * 0.25` and `xBodyRot`
    /// easing toward `-90°`) is not modelled — a squid is a water creature, so the
    /// in-water branch is the always-relevant case. The vanilla movement-derived
    /// `yBodyRot` refinement is also deferred (that is the entity body yaw, which
    /// bbb projects separately and must not be perturbed here).
    fn advance_client_tick(&mut self, delta_movement: EntityVec3) {
        use std::f32::consts::{PI, TAU};

        self.old_x_body_rot = self.x_body_rot;
        self.old_z_body_rot = self.z_body_rot;
        self.old_tentacle_movement = self.tentacle_movement;
        self.old_tentacle_angle = self.tentacle_angle;
        self.tentacle_movement += self.tentacle_speed;
        if self.tentacle_movement > TAU {
            // Client clamp (server resets to 0 and broadcasts event 19).
            self.tentacle_movement = TAU;
        }

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

        let horizontal = (delta_movement.x * delta_movement.x + delta_movement.z * delta_movement.z)
            .sqrt() as f32;
        self.z_body_rot += PI * self.rotate_speed * 1.5;
        self.x_body_rot +=
            (-(horizontal.atan2(delta_movement.y as f32)) * (180.0 / PI) - self.x_body_rot) * 0.1;
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
    pub(crate) fn sync_targets_from_metadata(
        &mut self,
        entity_type_id: i32,
        data_values: &[EntityDataValue],
    ) {
        // Vanilla `LivingEntity.isDeadOrDying()` (`getHealth() <= 0`): a living
        // entity whose synced health has reached zero begins the death animation;
        // restoring health clears it. Only living entities carry the health float,
        // so the per-type model animations below are unaffected.
        if vanilla_living_entity_type(entity_type_id) {
            if let Some(health) = entity_data_float(data_values, VANILLA_ENTITY_HEALTH_DATA_ID) {
                if health <= 0.0 {
                    self.death
                        .get_or_insert(DeathAnimationState { death_time: 0 });
                } else {
                    self.death = None;
                }
            }
        }
        match entity_type_id {
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => {
                let target_standing =
                    entity_data_bool(data_values, POLAR_BEAR_STANDING_DATA_ID, false);
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
                let swell_dir = creeper_effective_swell_dir(data_values);
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
                let target_peek_amount = shulker_target_peek_amount(data_values);
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
                let rolling = bee_is_rolling(data_values);
                if let Some(roll) = self.bee_roll.as_mut() {
                    roll.set_rolling(rolling);
                } else if rolling {
                    self.bee_roll = Some(BeeRollAnimationState {
                        rolling,
                        ..BeeRollAnimationState::default()
                    });
                }
            }
            VANILLA_ENTITY_TYPE_FOX_ID => {
                let interested = fox_is_interested(data_values);
                let crouching = fox_is_crouching(data_values);
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
            VANILLA_ENTITY_TYPE_FROG_ID => {
                // Vanilla `Frog.onSyncedDataUpdated`: when `DATA_POSE` changes, the croak animation
                // is `animateWhen(pose == CROAKING, tickCount)` and the jump animation is started
                // when `pose == LONG_JUMPING` and stopped otherwise — both started on the rising edge
                // into their pose and stopped on leaving it. The frog's `aiStep` runs client-side for
                // remote entities, so the synced pose drives both directly.
                let pose = entity_data_pose(data_values);
                let croaking = pose == VANILLA_POSE_CROAKING_ID;
                if let Some(croak) = self.frog_croak.as_mut() {
                    croak.animate_when(croaking, self.age_ticks);
                } else if croaking {
                    self.frog_croak = Some(KeyframeAnimationState {
                        start_age: Some(self.age_ticks),
                    });
                }
                let jumping = pose == VANILLA_POSE_LONG_JUMPING_ID;
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
                // `.start()`s the roar/sniff one-shot when the synced `DATA_POSE` CHANGES to
                // `Pose.ROARING`/`Pose.SNIFFING`. We track the previous pose and restart the timer
                // only on the transition; vanilla never auto-stops on a pose leave, so the
                // non-looping keyframe holds its final frame. The warden's `aiStep` runs client-side
                // for remote entities, so the synced pose drives the pose directly. (`EMERGING`/
                // `DIGGING` are deferred; the attack/sonic-boom one-shots are event-driven.)
                let pose_id = entity_data_pose(data_values);
                self.warden_combat
                    .get_or_insert_with(WardenCombatAnimationState::default)
                    .set_pose(pose_id, self.age_ticks);
            }
            VANILLA_ENTITY_TYPE_SNIFFER_ID => {
                // Vanilla `Sniffer.onSyncedDataUpdated(DATA_STATE)`: the synced state (id 18) selects
                // the one mutually-exclusive `AnimationState` to start, and a state change
                // `resetAnimations()` + restarts it. The sniffer's `aiStep` runs client-side for
                // remote entities, so the synced state drives the pose directly. `DATA_STATE` is
                // serialized as the `SnifferState` enum (serializer 35), so it arrives as an
                // `EnumId`, not a plain `Int`.
                let state_id = entity_data_enum_id(
                    data_values,
                    SNIFFER_STATE_DATA_ID,
                    EntityDataEnumSerializer::SnifferState,
                    SNIFFER_STATE_IDLING_ID,
                );
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
            VANILLA_ENTITY_TYPE_ARMADILLO_ID => {
                // Vanilla `Armadillo.onSyncedDataUpdated(ARMADILLO_STATE)` → `setupAnimationStates`:
                // the synced `ArmadilloState` id (id 18) selects the shell-ball hide window and the
                // rollUp/rollOut/peek `AnimationState`s. The armadillo's `aiStep` runs client-side for
                // remote entities, so the synced state drives the transitions directly. We track the
                // state and the `age_ticks` at the change (for `inStateTicks`); IDLE carries no hide
                // or transition timer but must still be tracked so a later roll re-anchors correctly.
                // `ARMADILLO_STATE` is serialized as the `ArmadilloState` enum (serializer 36), so it
                // arrives as an `EnumId`, not a plain `Int`.
                let state_id = entity_data_enum_id(
                    data_values,
                    ARMADILLO_STATE_DATA_ID,
                    EntityDataEnumSerializer::ArmadilloState,
                    ARMADILLO_STATE_IDLE_ID,
                );
                if let Some(armadillo) = self.armadillo.as_mut() {
                    armadillo.set_state(state_id, self.age_ticks);
                } else if state_id != ARMADILLO_STATE_IDLE_ID {
                    self.armadillo = Some(ArmadilloAnimationState {
                        state_id,
                        state_change_age: self.age_ticks,
                        roll_up: KeyframeAnimationState {
                            start_age: (state_id == ARMADILLO_STATE_ROLLING_ID)
                                .then_some(self.age_ticks),
                        },
                        roll_out: KeyframeAnimationState {
                            start_age: (state_id == ARMADILLO_STATE_UNROLLING_ID)
                                .then_some(self.age_ticks),
                        },
                    });
                }
            }
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => {
                let phase_id = entity_data_int(
                    data_values,
                    ENDER_DRAGON_PHASE_DATA_ID,
                    ENDER_DRAGON_PHASE_HOVERING_ID,
                );
                if let Some(dragon) = self.ender_dragon.as_mut() {
                    dragon.set_phase(phase_id);
                } else {
                    self.ender_dragon = Some(EnderDragonAnimationState {
                        phase_id,
                        ..EnderDragonAnimationState::default()
                    });
                }
            }
            _ => {}
        }
    }

    /// Projects a client entity event into client animation state. Vanilla
    /// `Sheep.handleEntityEvent` resets the eat-grass animation on event `10`.
    pub(crate) fn handle_entity_event(&mut self, entity_type_id: i32, event_id: i8) {
        if entity_type_id == VANILLA_ENTITY_TYPE_SHEEP_ID && event_id == SHEEP_EAT_GRASS_EVENT_ID {
            self.sheep_eat = Some(SheepEatAnimationState {
                eat_animation_tick: SHEEP_EAT_ANIMATION_TICKS,
            });
        } else if entity_type_id == VANILLA_ENTITY_TYPE_WARDEN_ID
            && event_id == WARDEN_TENDRIL_EVENT_ID
        {
            // Vanilla `Warden.handleEntityEvent(61)`: `tendrilAnimation = 10`. Only
            // `tendrilAnimation` (`current`) is set; `tendrilAnimationO` (`previous`)
            // is untouched, so the lerp fades in from the prior frame.
            self.warden_tendril
                .get_or_insert(WardenTendrilAnimationState {
                    previous: 0,
                    current: 0,
                })
                .current = WARDEN_TENDRIL_ANIMATION_TICKS;
        } else if entity_type_id == VANILLA_ENTITY_TYPE_WARDEN_ID
            && event_id == WARDEN_ATTACK_EVENT_ID
        {
            // Vanilla `Warden.handleEntityEvent(4)`: `roarAnimationState.stop()` then
            // `attackAnimationState.start(tickCount)` — the melee swing also cancels any roar.
            self.warden_combat
                .get_or_insert_with(WardenCombatAnimationState::default)
                .start_attack(self.age_ticks);
        } else if entity_type_id == VANILLA_ENTITY_TYPE_WARDEN_ID
            && event_id == WARDEN_SONIC_BOOM_EVENT_ID
        {
            // Vanilla `Warden.handleEntityEvent(62)`: `sonicBoomAnimationState.start(tickCount)`.
            self.warden_combat
                .get_or_insert_with(WardenCombatAnimationState::default)
                .start_sonic_boom(self.age_ticks);
        } else if matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_SQUID_ID | VANILLA_ENTITY_TYPE_GLOW_SQUID_ID
        ) && event_id == SQUID_RESET_MOVEMENT_EVENT_ID
        {
            // Vanilla `Squid.handleEntityEvent(19)`: `tentacleMovement = 0`. Only an
            // already-ticked squid has a state; if the event arrives first the
            // animation will start fresh on the next tick anyway, so nothing to do.
            if let Some(squid) = self.squid.as_mut() {
                squid.reset_movement();
            }
        } else if entity_type_id == VANILLA_ENTITY_TYPE_GOAT_ID
            && matches!(
                event_id,
                GOAT_LOWER_HEAD_EVENT_ID | GOAT_RAISE_HEAD_EVENT_ID
            )
        {
            // Vanilla `Goat.handleEntityEvent`: event 58 → `isLoweringHead = true`, 59 → `false`. The
            // `aiStep` counter advances from there; an idle goat with no state is created on the first
            // lower so its `lowerHeadTick` can climb (a raise event before any state is a no-op rest).
            self.goat_ramming
                .get_or_insert(GoatRammingAnimationState {
                    lowering_head: false,
                    lower_head_tick: 0,
                })
                .lowering_head = event_id == GOAT_LOWER_HEAD_EVENT_ID;
        } else if entity_type_id == VANILLA_ENTITY_TYPE_IRON_GOLEM_ID
            && matches!(
                event_id,
                IRON_GOLEM_ATTACK_EVENT_ID
                    | IRON_GOLEM_OFFER_FLOWER_EVENT_ID
                    | IRON_GOLEM_STOP_OFFER_FLOWER_EVENT_ID
            )
        {
            // Vanilla `IronGolem.handleEntityEvent`: event 4 → `attackAnimationTick = 10`, 11 →
            // `offerFlowerTick = 400`, 34 → `offerFlowerTick = 0`. The `aiStep` decrements both.
            let golem = self.iron_golem.get_or_insert(IronGolemAnimationState {
                attack_animation_tick: 0,
                offer_flower_tick: 0,
            });
            match event_id {
                IRON_GOLEM_ATTACK_EVENT_ID => golem.attack_animation_tick = IRON_GOLEM_ATTACK_TICKS,
                IRON_GOLEM_OFFER_FLOWER_EVENT_ID => {
                    golem.offer_flower_tick = IRON_GOLEM_OFFER_FLOWER_TICKS
                }
                _ => golem.offer_flower_tick = 0,
            }
        } else if entity_type_id == VANILLA_ENTITY_TYPE_RAVAGER_ID
            && matches!(event_id, RAVAGER_ATTACK_EVENT_ID | RAVAGER_STUN_EVENT_ID)
        {
            // Vanilla `Ravager.handleEntityEvent`: event 4 → `attackTick = 10`, 39 → `stunnedTick = 40`.
            // The `aiStep` decrements them and arms the post-stun roar.
            let ravager = self.ravager.get_or_insert(RavagerAnimationState {
                attack_tick: 0,
                stunned_tick: 0,
                roar_tick: 0,
            });
            if event_id == RAVAGER_ATTACK_EVENT_ID {
                ravager.attack_tick = RAVAGER_ATTACK_TICKS;
            } else {
                ravager.stunned_tick = RAVAGER_STUN_TICKS;
            }
        } else if matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_HOGLIN_ID | VANILLA_ENTITY_TYPE_ZOGLIN_ID
        ) && event_id == HOGLIN_ATTACK_EVENT_ID
        {
            // Vanilla `Hoglin`/`Zoglin.handleEntityEvent`: event 4 → `attackAnimationRemainingTicks = 10`.
            self.hoglin
                .get_or_insert(HoglinAnimationState {
                    attack_animation_tick: 0,
                })
                .attack_animation_tick = HOGLIN_ATTACK_TICKS;
        }
    }

    /// Vanilla `Sheep.eatAnimationTick`, exposed for renderer head-pose
    /// projection. Returns `0` when the sheep is not currently eating.
    pub fn sheep_eat_animation_tick(&self) -> i32 {
        self.sheep_eat.map_or(0, |state| state.eat_animation_tick)
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

    /// Vanilla `Creeper.getSwelling(partialTick)`, exposed for the renderer white
    /// swelling overlay. Returns `0.0` when the entity is not a priming creeper.
    pub fn creeper_swelling(&self, partial_tick: f32) -> f32 {
        self.creeper_swell
            .map_or(0.0, |state| state.swelling(partial_tick))
    }

    /// Vanilla `Shulker.getClientPeekAmount(partialTick)`, exposed for the renderer
    /// `ShulkerModel.setupAnim` lid open/close. Returns `0.0` (the closed/bind pose) when
    /// the entity is not a shulker or its lid is shut.
    pub fn shulker_peek_amount(&self, partial_tick: f32) -> f32 {
        self.shulker_peek
            .map_or(0.0, |state| state.peek_amount(partial_tick))
    }

    /// Vanilla `Bee.getRollAmount(partialTick)` projected for the renderer
    /// `BeeModel.setupAnim` barrel-roll flip. Returns `0.0` when the entity is
    /// not a rolling bee.
    pub fn bee_roll_amount(&self, partial_tick: f32) -> f32 {
        self.bee_roll
            .map_or(0.0, |state| state.roll_amount(partial_tick))
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

    /// The armadillo's `ARMADILLO_PEEK` elapsed seconds (vanilla `peekAnimationState`). Deferred: the
    /// `fastForward(50, 1.0)` baseline vanilla applies on the first SCARED tick is not cleanly
    /// derivable from the synced state + `inStateTicks`, so the peek is never driven (`-1.0`), and
    /// the renderer applies no `ARMADILLO_PEEK` keyframe. See `docs/unsupported-features.md`.
    pub fn armadillo_peek_seconds(&self, _partial_tick: f32) -> f32 {
        -1.0
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

    /// Resets the hurt animation countdown to [`HURT_ANIMATION_DURATION`],
    /// mirroring vanilla `LivingEntity.animateHurt` / `handleDamageEvent` setting
    /// `hurtTime = hurtDuration`.
    pub(crate) fn trigger_hurt(&mut self) {
        self.hurt = Some(HurtAnimationState {
            hurt_time: HURT_ANIMATION_DURATION,
        });
    }

    /// Vanilla `LivingEntityRenderer.extractRenderState`:
    /// `hasRedOverlay = hurtTime > 0 || deathTime > 0`.
    pub fn has_red_overlay(&self) -> bool {
        self.hurt.is_some_and(|state| state.hurt_time > 0)
            || self.death.is_some_and(|state| state.death_time > 0)
    }

    /// Arms a melee swing, mirroring vanilla `LivingEntity.swing(hand)`: a swing only
    /// (re)starts when none is playing, the current one is past halfway, or the counter is
    /// the fresh `-1` — so a rapid re-swing during the first half is ignored (the in-flight
    /// swing keeps ramping). `off_hand` selects the left arm (`ClientboundAnimate` action `3`).
    pub(crate) fn trigger_swing(&mut self, off_hand: bool) {
        let state = self.attack_swing.get_or_insert(AttackSwingAnimationState {
            swinging: false,
            swing_time: 0,
            attack_anim: 0.0,
            prev_attack_anim: 0.0,
            off_hand,
        });
        if !state.swinging || state.swing_time >= ATTACK_SWING_DURATION / 2 || state.swing_time < 0
        {
            state.swing_time = -1;
            state.swinging = true;
            state.off_hand = off_hand;
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

    pub(crate) fn advance_client_tick(
        &mut self,
        entity_type_id: i32,
        entity_id: i32,
        transform: EntityTransform,
        is_passenger: bool,
        is_baby: bool,
        // The per-tick world fact `Guardian.aiStep` reads via `isInWater()`,
        // resolved by [`WorldStore::advance_entity_client_animations`] (which holds
        // the chunk/fluid data this context lacks). `false` for entities that do
        // not consume it. See [`entity_animation_uses_in_water`].
        in_water: bool,
        // The synced `Guardian.DATA_ID_MOVING` flag (`isMoving()`), read from the
        // entity metadata in the tick loop. `false` for entities that do not
        // consume it.
        is_moving: bool,
        // Vanilla `Warden.getHeartBeatDelay()` over the synced anger, read from the
        // entity metadata in the tick loop ([`warden_heartbeat_delay`]). The calm
        // `40` for entities that do not consume it.
        warden_heartbeat_delay: u32,
        // The synced `Guardian.DATA_ID_ATTACK_TARGET` (`isMoving`'s sibling), read from the entity
        // metadata in the tick loop ([`guardian_attack_target_id`]). `0` (no target) for entities that
        // do not consume it.
        guardian_attack_target_id: i32,
    ) {
        self.age_ticks = self.age_ticks.saturating_add(1);
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
        if let Some(death) = self.death.as_mut() {
            death.death_time = (death.death_time + 1).min(DEATH_ANIMATION_MAX_TICKS);
        }
        // Vanilla `LivingEntity` (`oAttackAnim = attackAnim` at tick start, then
        // `updateSwingTime`): the melee swing ramps for every living entity, so it runs
        // outside the per-type match. The state is dropped once the swing has fully decayed.
        if let Some(swing) = self.attack_swing.as_mut() {
            swing.prev_attack_anim = swing.attack_anim;
            if swing.swinging {
                swing.swing_time += 1;
                if swing.swing_time >= ATTACK_SWING_DURATION {
                    swing.swing_time = 0;
                    swing.swinging = false;
                }
            } else {
                swing.swing_time = 0;
            }
            swing.attack_anim = swing.swing_time as f32 / ATTACK_SWING_DURATION as f32;
            if !swing.swinging && swing.attack_anim == 0.0 && swing.prev_attack_anim == 0.0 {
                self.attack_swing = None;
            }
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
                    ravager.advance_client_tick();
                    if ravager.is_settled() {
                        self.ravager = None;
                    }
                }
            }
            VANILLA_ENTITY_TYPE_HOGLIN_ID | VANILLA_ENTITY_TYPE_ZOGLIN_ID => {
                if let Some(hoglin) = self.hoglin.as_mut() {
                    hoglin.advance_client_tick();
                    if hoglin.is_settled() {
                        self.hoglin = None;
                    }
                }
            }
            VANILLA_ENTITY_TYPE_SHULKER_ID => {
                if let Some(peek) = self.shulker_peek.as_mut() {
                    peek.advance_client_tick();
                }
            }
            VANILLA_ENTITY_TYPE_BEE_ID => {
                if let Some(roll) = self.bee_roll.as_mut() {
                    roll.advance_client_tick();
                }
            }
            VANILLA_ENTITY_TYPE_FOX_ID => {
                if let Some(fox) = self.fox.as_mut() {
                    fox.advance_client_tick();
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
                self.warden_heart
                    .get_or_insert_with(WardenHeartAnimationState::default)
                    .advance_client_tick(self.age_ticks, warden_heartbeat_delay);
            }
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => self
                .ender_dragon
                .get_or_insert_with(EnderDragonAnimationState::default)
                .advance_client_tick(transform),
            VANILLA_ENTITY_TYPE_SQUID_ID | VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => self
                .squid
                .get_or_insert_with(|| SquidAnimationState::new(entity_id))
                .advance_client_tick(transform.delta_movement),
            VANILLA_ENTITY_TYPE_GUARDIAN_ID | VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID => {
                // Vanilla `Guardian.aiStep` reads `isInWater()` (the world fact
                // threaded in) and the synced `isMoving()` flag (`DATA_ID_MOVING`).
                self.guardian_tail
                    .get_or_insert_with(GuardianTailAnimationState::default)
                    .advance_client_tick(in_water, is_moving);
                // The same `aiStep` ramps `clientSideAttackTime` while a target is locked.
                self.guardian_attack
                    .get_or_insert_with(GuardianAttackAnimationState::default)
                    .advance_client_tick(
                        guardian_attack_target_id,
                        guardian_attack_duration(entity_type_id),
                    );
            }
            VANILLA_ENTITY_TYPE_FROG_ID => {
                // Vanilla `Frog.tick` (client side): `swimIdleAnimationState.animateWhen(isInWater()
                // && !walkAnimation.isMoving(), tickCount)`. `isInWater()` is the per-tick world fact
                // threaded in; `walkAnimation.isMoving()` reads the limb-swing speed from the PREVIOUS
                // tick's `WalkAnimationState` — the walk accumulator below this match advances after,
                // so reading it here matches vanilla `tick` running before the limb-swing update. The
                // frog's `updateWalkAnimation` override is deferred (no `walk_animation` state), so a
                // missing state is treated as not moving (idle), which is the common in-water case.
                let walk_is_moving = self
                    .walk_animation
                    .as_ref()
                    .is_some_and(WalkAnimationState::is_moving);
                self.frog_swim_idle
                    .get_or_insert_with(|| KeyframeAnimationState { start_age: None })
                    .animate_when(in_water && !walk_is_moving, self.age_ticks);
            }
            VANILLA_ENTITY_TYPE_CHICKEN_ID => self
                .chicken_flap
                .get_or_insert_with(ChickenFlapAnimationState::default)
                // Vanilla `Chicken.aiStep` reads `onGround()`. A chicken with no
                // synced ground flag defaults to on-ground (wings still), the safe
                // common case.
                .advance_client_tick(transform.on_ground.unwrap_or(true)),
            VANILLA_ENTITY_TYPE_PARROT_ID => self
                .parrot_flap
                .get_or_insert_with(ParrotFlapAnimationState::default)
                // Vanilla `Parrot.calculateFlapping` reads `!onGround() &&
                // !isPassenger()`. A parrot with no synced ground flag defaults to
                // on-ground (wings still), the safe common case.
                .advance_client_tick(transform.on_ground.unwrap_or(true), is_passenger),
            _ => {}
        }
        // Vanilla `LivingEntity.calculateEntityAnimation` runs every client tick
        // (`aiStep`, or `RemotePlayer.tick`) for every living entity, feeding the
        // per-tick travel to the limb-swing accumulator. Entities whose
        // `updateWalkAnimation` override is not yet modelled are left deferred.
        if vanilla_living_entity_type(entity_type_id)
            && !walk_animation_override_is_deferred(entity_type_id)
        {
            // Vanilla `isAlive() = getHealth() > 0`: the client death animation
            // state is present exactly while the entity is dead/dying.
            let is_alive = self.death.is_none();
            let use_y = is_flying_animal(entity_type_id);
            self.walk_animation
                .get_or_insert_with(WalkAnimationState::default)
                .advance_client_tick(transform.position, use_y, is_passenger, is_alive, is_baby);
        }
    }
}

/// Whether an entity type's client-tick animation reads `isInWater()` and so needs
/// the per-entity `in_water` map [`WorldStore::advance_entity_client_animations`]
/// builds. Keeping this cheap (a tiny `match`) lets the world skip the AABB / fluid
/// probe for every other entity. Adding a consumer (the frog swim-idle, dolphin,
/// axolotl, fish, …) is just another arm here.
pub(crate) fn entity_animation_uses_in_water(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_GUARDIAN_ID
            | VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID
            | VANILLA_ENTITY_TYPE_FROG_ID
    )
}

/// Vanilla `Guardian.isMoving()` = the synced `DATA_ID_MOVING` boolean (default
/// `false`). Read straight from the entity metadata in the tick loop.
pub(crate) fn guardian_is_moving(data_values: &[EntityDataValue]) -> bool {
    entity_data_bool(data_values, GUARDIAN_MOVING_DATA_ID, false)
}

/// Vanilla `Warden.getHeartBeatDelay()` = `40 - floor(clamp(clientAngerLevel /
/// AngerLevel.ANGRY.minimumAnger, 0, 1) · 30)`: the period (in client ticks)
/// between heartbeats, shrinking from `40` (calm) to `10` (fully angry) as the
/// synced anger rises. Read straight from the entity metadata in the tick loop;
/// non-wardens (whose synced slot `16` is not an int) fall back to the calm `40`.
pub(crate) fn warden_heartbeat_delay(data_values: &[EntityDataValue]) -> u32 {
    let anger = entity_data_int(data_values, WARDEN_ANGER_LEVEL_DATA_ID, 0);
    let ratio = (anger as f32 / WARDEN_ANGRY_MINIMUM_ANGER as f32).clamp(0.0, 1.0);
    (40 - (ratio * 30.0).floor() as i32) as u32
}

impl WorldStore {
    pub fn advance_entity_client_animations(&mut self, ticks: u32) {
        // Vanilla `Guardian.aiStep` recomputes `isInWater()` each client tick, but
        // within one `advance` batch the entity transforms and the chunk fluid state
        // are static (only the animation accumulators mutate), so the per-entity
        // `in_water` result is identical across all `ticks` iterations. Compute it
        // once here — where the chunk/fluid data lives — and thread it into the tick
        // loop. The immutable reads (build the map) MUST precede the mutable advance.
        let in_water_inputs = self.entities.in_water_aabb_inputs();
        let in_water_by_id: std::collections::HashMap<i32, bool> = in_water_inputs
            .into_iter()
            .map(|(id, aabb_min, aabb_max)| {
                (
                    id,
                    crate::fluid::world_aabb_in_water(self, aabb_min, aabb_max),
                )
            })
            .collect();
        self.entities
            .advance_client_animations(ticks, &in_water_by_id);
    }
}

fn entity_data_bool(data_values: &[EntityDataValue], data_id: u8, default: bool) -> bool {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Boolean(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

fn shulker_target_peek_amount(data_values: &[EntityDataValue]) -> f32 {
    f32::from(entity_data_byte(data_values, SHULKER_PEEK_DATA_ID, 0).clamp(0, 100)) * 0.01
}

/// Vanilla `Bee.isRolling()` = `getFlag(FLAG_ROLL)` = `(flags & 2) != 0`.
fn bee_is_rolling(data_values: &[EntityDataValue]) -> bool {
    entity_data_byte(data_values, BEE_FLAGS_DATA_ID, 0) & BEE_FLAG_ROLL != 0
}

/// Vanilla `Fox.isInterested()` = `getFlag(FLAG_INTERESTED)` = `(flags & 8) != 0`.
fn fox_is_interested(data_values: &[EntityDataValue]) -> bool {
    entity_data_byte(data_values, FOX_FLAGS_DATA_ID, 0) & FOX_FLAG_INTERESTED != 0
}

/// Vanilla `Fox.isCrouching()` = `getFlag(FLAG_CROUCHING)` = `(flags & 4) != 0`.
fn fox_is_crouching(data_values: &[EntityDataValue]) -> bool {
    entity_data_byte(data_values, FOX_FLAGS_DATA_ID, 0) & FOX_FLAG_CROUCHING != 0
}

/// Vanilla `Creeper.tick`: the fuse advances by `getSwellDir()`, but an ignited
/// creeper forces the direction to `1`. Mirrors that effective direction from
/// the synced `DATA_SWELL_DIR` (default `-1`) and `DATA_IS_IGNITED`.
fn creeper_effective_swell_dir(data_values: &[EntityDataValue]) -> i32 {
    if entity_data_bool(data_values, CREEPER_IGNITED_DATA_ID, false) {
        1
    } else {
        entity_data_int(data_values, CREEPER_SWELL_DIR_DATA_ID, -1)
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

fn entity_data_int(data_values: &[EntityDataValue], data_id: u8, fallback: i32) -> i32 {
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
