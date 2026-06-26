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
/// Vanilla `EntityType.ARROW` / `EntityType.SPECTRAL_ARROW`: both extend
/// `AbstractArrow` and share the same client-side `shakeTime` impact wobble.
const VANILLA_ENTITY_TYPE_ARROW_ID: i32 = 6;
const VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID: i32 = 123;
/// Vanilla `AbstractArrow.IN_GROUND`, the synced boolean after `ID_FLAGS` (`8`) and
/// `PIERCE_LEVEL` (`9`). `onSyncedDataUpdated(IN_GROUND)` starts `shakeTime = 7`
/// when the arrow is no longer on its first tick and the current shake has settled.
const ABSTRACT_ARROW_IN_GROUND_DATA_ID: u8 = 10;
const ARROW_SHAKE_TICKS: i32 = 7;
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
/// Vanilla `EntityType.WITHER`; its two alternative render heads keep client-side
/// `xRotHeads` / `yRotHeads` arrays that lerp toward synced target entities.
const VANILLA_ENTITY_TYPE_WITHER_ID: i32 = 145;
/// Vanilla `WitherBoss.DATA_TARGET_B/C`, the two side-head target ids. `DATA_TARGET_A`
/// (16) is the center combat target; model side heads read B/C at 17/18.
const WITHER_TARGET_B_DATA_ID: u8 = 17;
const WITHER_TARGET_C_DATA_ID: u8 = 18;
/// Vanilla `FlyingAnimal` implementors (`Bee`, `Parrot`): their
/// `LivingEntity.calculateEntityAnimation` measures the full 3-D travel distance
/// (`calculateEntityAnimation(this instanceof FlyingAnimal)`), so the limb-swing
/// distance includes the vertical component.
const VANILLA_ENTITY_TYPE_BEE_ID: i32 = 11;
const VANILLA_ENTITY_TYPE_PARROT_ID: i32 = 98;
const VANILLA_ENTITY_TYPE_PANDA_ID: i32 = 96;
/// Vanilla `Panda.DATA_ID_FLAGS`, the panda's synced flags byte after unhappy/sneeze/eat/gene data.
const PANDA_FLAGS_DATA_ID: u8 = 23;
/// Vanilla `Panda.isRolling()` / `isSitting()` / `isOnBack()` masks within `DATA_ID_FLAGS`.
const PANDA_FLAG_ROLLING: i8 = 0x04;
const PANDA_FLAG_SITTING: i8 = 0x08;
const PANDA_FLAG_ON_BACK: i8 = 0x10;
/// Vanilla `Panda.updateSitAmount` / `updateOnBackAnimation` / `updateRollAmount` easing.
const PANDA_AMOUNT_RISE_PER_TICK: f32 = 0.15;
const PANDA_AMOUNT_FALL_PER_TICK: f32 = 0.19;
const PANDA_ROLL_COUNTER_MAX: i32 = 32;
/// Entities whose `updateWalkAnimation` override (`Camel`, `Creaking`, `Frog`)
/// replaces the base distance→speed mapping. `Camel`/`Frog` additionally gate on
/// pose/jump/dash animation states the client does not yet track, so their limb
/// swing is deferred rather than approximated with the base mapping.
const VANILLA_ENTITY_TYPE_CAMEL_ID: i32 = 19;
pub(crate) const VANILLA_ENTITY_TYPE_CREAKING_ID: i32 = 31;
const VANILLA_ENTITY_TYPE_FROG_ID: i32 = 55;
/// Vanilla `Pose.CROAKING` ordinal (`Pose.CROAKING(8, …)`), the synced `DATA_POSE` int value that
/// `Frog.onSyncedDataUpdated` reads to start/stop `croakAnimationState` (`animateWhen(pose ==
/// CROAKING, tickCount)`).
const VANILLA_POSE_CROAKING_ID: i32 = 8;
/// Vanilla `Pose.SWIMMING` ordinal, the synced `DATA_POSE` value that makes
/// `Entity.isVisuallySwimming()` true and ramps `LivingEntity.swimAmount`.
const VANILLA_POSE_SWIMMING_ID: i32 = 3;
/// Vanilla `Pose.USING_TONGUE` ordinal (`Pose.USING_TONGUE(9, …)`), the synced `DATA_POSE` int value
/// that `Frog.onSyncedDataUpdated` reads to start/stop `tongueAnimationState` (`pose == USING_TONGUE`
/// starts it, otherwise stops it).
const VANILLA_POSE_USING_TONGUE_ID: i32 = 9;
/// Vanilla `Pose.LONG_JUMPING` ordinal (`Pose.LONG_JUMPING(6, …)`), the synced `DATA_POSE` int value
/// that `Frog.onSyncedDataUpdated` reads to start/stop `jumpAnimationState` (`pose == LONG_JUMPING`
/// starts it, otherwise stops it).
const VANILLA_POSE_LONG_JUMPING_ID: i32 = 6;
/// Vanilla `Entity.DATA_SHARED_FLAGS_ID`, the base entity flags byte. Bit 7 is
/// `FLAG_FALL_FLYING`, read by `LivingEntity.isFallFlying()`.
const ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
const ENTITY_FLAG_FALL_FLYING: u8 = 1 << 7;
const VANILLA_ENTITY_TYPE_BREEZE_ID: i32 = 17;
/// Vanilla `Pose.SLIDING`/`SHOOTING`/`INHALING` ordinals (`Pose.SLIDING(15, …)`, `SHOOTING(16, …)`,
/// `INHALING(17, …)`), the synced `DATA_POSE` int values that `Breeze.onSyncedDataUpdated` reads to
/// `startIfStopped` the matching action one-shot, and `LONG_JUMPING(6)` for the jump.
const VANILLA_POSE_SLIDING_ID: i32 = 15;
const VANILLA_POSE_SHOOTING_ID: i32 = 16;
const VANILLA_POSE_INHALING_ID: i32 = 17;
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
const SNIFFER_STATE_SEARCHING_ID: i32 = 4;
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
/// Vanilla `Armadillo.ArmadilloState.SCARED.animationDuration()`; the first SCARED
/// setup tick starts `peekAnimationState` and `fastForward`s it by this many ticks.
const ARMADILLO_STATE_SCARED_ANIMATION_TICKS: i32 = 50;
/// Vanilla `Armadillo.handleEntityEvent(64)`: the client marks `peekReceivedClient`
/// so the next `setupAnimationStates()` restarts the peek animation.
const ARMADILLO_PEEK_EVENT_ID: i8 = 64;
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
/// Vanilla `Wolf` entity type id (`EntityType.WOLF`).
const VANILLA_ENTITY_TYPE_WOLF_ID: i32 = 148;
/// Vanilla `Wolf.DATA_INTERESTED_ID` (`isInterested()`), the wolf's first own accessor:
/// Entity 0–7, LivingEntity 8–14, Mob 15, AgeableMob 16–17, TamableAnimal 18–19,
/// then Wolf interested 20.
const WOLF_INTERESTED_DATA_ID: u8 = 20;
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
const VANILLA_ENTITY_TYPE_GOAT_ID: i32 = 62;
const VANILLA_ENTITY_TYPE_HOGLIN_ID: i32 = 64;
const VANILLA_ENTITY_TYPE_IRON_GOLEM_ID: i32 = 70;
const VANILLA_ENTITY_TYPE_RAVAGER_ID: i32 = 109;
const VANILLA_ENTITY_TYPE_ZOGLIN_ID: i32 = 149;
const VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID: i32 = 80;
const VANILLA_ENTITY_TYPE_SLIME_ID: i32 = 117;
const VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID: i32 = 47;
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
const VANILLA_ENTITY_TYPE_ALLAY_ID: i32 = 2;
const VANILLA_ENTITY_TYPE_PILLAGER_ID: i32 = 103;
const VANILLA_ENTITY_TYPE_PIGLIN_ID: i32 = 101;
const VANILLA_ENTITY_TYPE_PLAYER_ID: i32 = 155;
const VANILLA_ENTITY_TYPE_AXOLOTL_ID: i32 = 7;
const VANILLA_ENTITY_TYPE_RABBIT_ID: i32 = 108;
/// Vanilla `Rabbit.handleEntityEvent`: event `1` (`spawnSprintParticle`) also seeds the client-side
/// hop reconstruction — `jumpDuration = 15; jumpTicks = 0;` — so the `hopAnimationState` plays for
/// one 15-tick (0.75s, exactly one loop of `RabbitAnimation.HOP`) jump arc.
const RABBIT_JUMP_EVENT_ID: i8 = 1;
/// Vanilla `Rabbit.startJumping` / `handleEntityEvent(1)`: `this.jumpDuration = 15`.
const RABBIT_JUMP_DURATION: i32 = 15;
/// Vanilla `Axolotl.DATA_PLAYING_DEAD`, the synced `EntityDataSerializers.BOOLEAN` accessor at id 19
/// (Entity 0–7, LivingEntity 8–14, Mob 15, AgeableMob 16–17, then `Axolotl`'s first own accessor
/// `DATA_VARIANT` 18, `DATA_PLAYING_DEAD` 19, `FROM_BUCKET` 20). `Axolotl.tickAdultAnimations` reads
/// `isPlayingDead()` to drive the play-dead state animator.
const AXOLOTL_PLAYING_DEAD_DATA_ID: u8 = 19;
/// Vanilla `Axolotl`'s four animators are each `BinaryAnimator(10, IN_OUT_SINE)`, so every factor
/// eases between 0 and 1 over ten ticks.
const AXOLOTL_ANIMATOR_LENGTH: i32 = 10;
/// Vanilla `Allay.DATA_DANCING`, the synced `EntityDataSerializers.BOOLEAN` accessor at id 16 (Entity
/// 0–7, LivingEntity 8–14, Mob 15, then `Allay`'s first own accessor `DATA_DANCING`; the allay is a
/// `PathfinderMob`, not an `AgeableMob`, so there is no baby slot). `Allay.tick` advances the dance /
/// spin accumulators while `isDancing()`.
const ALLAY_DANCING_DATA_ID: u8 = 16;
/// Vanilla `Camel.DASH`, the synced `EntityDataSerializers.BOOLEAN` accessor at id 19 (Entity 0–7,
/// LivingEntity 8–14, Mob 15, AgeableMob 16–17, `AbstractHorse.DATA_ID_FLAGS` 18, then Camel's first
/// own accessor `DASH`; `LAST_POSE_CHANGE_TICK` follows at 20). `Camel.setupAnimationStates` reads
/// `isDashing()` to gate the looping dash gallop.
const CAMEL_DASH_DATA_ID: u8 = 19;
/// Vanilla `Pillager.IS_CHARGING_CROSSBOW`, the synced `EntityDataSerializers.BOOLEAN` accessor at id 17
/// (Entity 0–7, LivingEntity 8–14, Mob 15, `Raider.IS_CELEBRATING` 16, then `Pillager`'s first own
/// accessor). While set, the pillager is drawing its crossbow, and the client reconstructs
/// `getTicksUsingItem` by counting ticks since the flag rose (the using-item flag rises with it).
const PILLAGER_IS_CHARGING_CROSSBOW_DATA_ID: u8 = 17;
/// Vanilla `Piglin.DATA_IS_CHARGING_CROSSBOW` synched-data id (18, the regular piglin's own slot — past
/// `DATA_BABY_ID`, `DATA_IS_IMMUNE_TO_ZOMBIFICATION`). Drives the same crossbow-draw counter as the
/// pillager (the flag is a different id on a different class, so read it under a piglin type gate).
const PIGLIN_IS_CHARGING_CROSSBOW_DATA_ID: u8 = 18;
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
/// Vanilla `Pose.EMERGING` ordinal, the synced `DATA_POSE` int value that
/// `Warden.onSyncedDataUpdated` reads to `.start()` the `emergeAnimationState` when
/// the pose CHANGES to it (the 6.68s `WARDEN_EMERGE` spawn keyframe animation).
const VANILLA_POSE_EMERGING_ID: i32 = 13;
/// Vanilla `Pose.DIGGING` ordinal, the synced `DATA_POSE` int value that
/// `Warden.onSyncedDataUpdated` reads to `.start()` the `diggingAnimationState` when
/// the pose CHANGES to it (the 5.0s `WARDEN_DIG` despawn keyframe animation).
const VANILLA_POSE_DIGGING_ID: i32 = 14;
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

/// Vanilla `EvokerFangs.handleEntityEvent`: event `4` sets `clientSideAttackStarted = true`, after which
/// `EvokerFangs.tick` decrements `lifeTicks` (field initializer `22`) each client tick. The
/// `getAnimationProgress` ramp built from `lifeTicks` drives `EvokerFangsModel.setupAnim`.
const EVOKER_FANGS_ATTACK_EVENT_ID: i8 = 4;
const EVOKER_FANGS_LIFE_TICKS: i32 = 22;

/// Vanilla `Creaking.handleEntityEvent`: event `4` sets `attackAnimationRemainingTicks = 15` (the
/// lunge), event `66` sets `invulnerabilityAnimationRemainingTicks = 8` (the heart-bound stagger).
/// Both decrement each client tick in `aiStep`, and `setupAnimationStates` `animateWhen`s the
/// matching keyframe one-shot on `ticks > 0`.
const CREAKING_ATTACK_EVENT_ID: i8 = 4;
const CREAKING_INVULNERABLE_EVENT_ID: i8 = 66;
const CREAKING_ATTACK_DURATION: i32 = 15;
const CREAKING_INVULNERABLE_DURATION: i32 = 8;
/// Vanilla `Creaking.CAN_MOVE`, the first own synced accessor after `Entity` (`0..=7`),
/// `LivingEntity` (`8..=14`) and `Mob` (`15`) — a `Boolean`, default `true`. `setupAnim` gates the
/// looping walk on it (a frozen-while-observed creaking turns to a statue).
const CREAKING_CAN_MOVE_DATA_ID: u8 = 16;
/// Vanilla `Creaking.IS_TEARING_DOWN`, the synced `Boolean` two slots after `CAN_MOVE`
/// (`CAN_MOVE` 16, `IS_ACTIVE` 17, `IS_TEARING_DOWN` 18; default `false`). `isTearingDown()` reads
/// it, and `setupAnimationStates` `animateWhen`s the death keyframe on it.
const CREAKING_IS_TEARING_DOWN_DATA_ID: u8 = 18;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityClientAnimationState {
    #[serde(default)]
    pub age_ticks: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrow_shake: Option<ArrowShakeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polar_bear_standing: Option<PolarBearStandingAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shulker_peek: Option<ShulkerPeekAnimationState>,
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
    pub wither_heads: Option<WitherHeadRotationsState>,
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
    pub crossbow_charge: Option<CrossbowChargeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axolotl: Option<AxolotlAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parrot_flap: Option<ParrotFlapAnimationState>,
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
        }
    }
}

impl PlayerCloakAnimationState {
    /// Vanilla `ClientAvatarState.tick` / `moveCloak`, plus the local-player
    /// walk-distance and bob inputs that `AvatarRenderer.extractCapeState`
    /// consumes. bbb drives the walk pulse from the synced feet movement for all
    /// players because the local-only `LocalPlayer.addWalkedDistance` path is not
    /// split out at this render-state boundary.
    fn advance_client_tick(
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
/// wet and `getWetShade` returns the `0.75` floor (re-entering water also cancels any drying shake, as
/// vanilla event `56` does). After leaving water, the server broadcasts the shake event when
/// grounded/path-free; bbb reconstructs the common world-side visual timer from the same `isInWater()`
/// and `onGround()` facts, incrementing `shakeAnim` by `0.05` until the wet shade has lerped back to
/// white. The same `shakeAnim` is also projected for `WolfRenderState.getBodyRollAngle`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WolfWetAnimationState {
    pub is_wet: bool,
    pub is_shaking: bool,
    pub previous_shake_anim: f32,
    pub current_shake_anim: f32,
}

impl WolfWetAnimationState {
    fn advance_client_tick(&mut self, in_water: bool, on_ground: bool) {
        if in_water {
            self.is_wet = true;
            self.is_shaking = false;
            self.previous_shake_anim = 0.0;
            self.current_shake_anim = 0.0;
            return;
        }

        if self.is_wet && !self.is_shaking && on_ground {
            self.is_shaking = true;
            self.previous_shake_anim = 0.0;
            self.current_shake_anim = 0.0;
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

/// Canonical client-side rabbit hop timer, mirroring vanilla `Rabbit`'s `jumpTicks`/`jumpDuration`
/// counter and its `hopAnimationState`. Entity event `1` seeds `jumpDuration = 15; jumpTicks = 0`;
/// each client tick `Rabbit.setupAnimationStates` `startIfStopped`/`stop`s the hop on `jumpTicks > 0`
/// and `Rabbit.aiStep` advances `jumpTicks` toward `jumpDuration`, wrapping it back to `0` (and
/// clearing `jumpDuration`) when they meet — so the hop runs for exactly one 15-tick window per jump,
/// matching the 0.75s looping `RabbitAnimation.HOP`. The random idle-head-tilt animation
/// (`shouldPlayIdleAnimation` gated on a `random.nextInt(40) + 180` timeout) is NOT reconstructable
/// and stays deferred, so the hop branch always governs.
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
    /// branch — idle deferred — `startIfStopped`/`stop` on `jumpTicks > 0`) BEFORE `Rabbit.aiStep`
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

/// Canonical client-side creaking combat/death keyframe state, mirroring vanilla `Creaking`'s
/// `attackAnimationState`/`invulnerabilityAnimationState`/`deathAnimationState` and the two
/// remaining-tick counters that drive them. Entity event `4` seeds `attackAnimationRemainingTicks =
/// 15`, event `66` seeds `invulnerabilityAnimationRemainingTicks = 8`; `aiStep` decrements both each
/// client tick, then `Creaking.tick`'s `setupAnimationStates` `animateWhen`s each one-shot on its
/// `ticks > 0`. The death one-shot has no counter — it `animateWhen`s on the synced `isTearingDown()`
/// (`IS_TEARING_DOWN`) directly, so the per-tick advance is fed that flag. (Vanilla decrements in
/// `aiStep` BEFORE `setupAnimationStates`, unlike `Rabbit`, whose `setupAnimationStates` runs first.)
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
}

impl CreakingAnimationState {
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
    fn advance_client_tick(&mut self, age_ticks: u32, is_tearing_down: bool) {
        if self.attack_ticks > 0 {
            self.attack_ticks -= 1;
        }
        if self.invuln_ticks > 0 {
            self.invuln_ticks -= 1;
        }
        self.attack.animate_when(self.attack_ticks > 0, age_ticks);
        self.invuln.animate_when(self.invuln_ticks > 0, age_ticks);
        self.death.animate_when(is_tearing_down, age_ticks);
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
            VANILLA_POSE_ROARING_ID => self.roar.start_age = Some(age_ticks),
            VANILLA_POSE_SNIFFING_ID => self.sniff.start_age = Some(age_ticks),
            VANILLA_POSE_EMERGING_ID => self.emerge.start_age = Some(age_ticks),
            VANILLA_POSE_DIGGING_ID => self.dig.start_age = Some(age_ticks),
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
/// client tick, re-seeds the target to `-0.5` on landing / `1.0` on takeoff (the
/// `onGround()` transitions), then decays the target by `0.6` (`decreaseSquish`).
/// `SlimeRenderer.extractRenderState` lerps `squish` by the partial tick, and
/// `SlimeRenderer.scale` turns it into the non-uniform body stretch. The vanilla
/// landing squish particles/sound are server/audio effects and are not modelled.
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

/// Canonical client-side axolotl animation, mirroring vanilla `Axolotl.tickAdultAnimations`. Four
/// `BinaryAnimator`s ease the play-dead / in-water / on-ground / moving factors that
/// `AdultAxolotlModel.setupAnim` blends into its swimming, water-hovering, ground-crawling,
/// lay-still, and play-dead sub-animations. The mutually-exclusive `playingDead → inWater →
/// onGround → inAir` state machine feeds the first three; `moving` is fed `walkAnimation.isMoving()`
/// OR a body/head rotation change (tracked here, since the synced rotation is the only source).
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
}

impl AxolotlAnimationState {
    /// Advances one client tick of `Axolotl.tickAdultAnimations`: derive the mutually-exclusive
    /// animation state, then tick the four animators. `walk_is_moving` is the prior tick's
    /// `walkAnimation.isMoving()` (the walk animation is advanced after the per-type match, matching
    /// vanilla's `baseTick`-before-`aiStep` order); the rotation change OR mirrors `getXRot() !=
    /// xRotO || getYRot() != yRotO`.
    fn advance_client_tick(
        &mut self,
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
}

/// Canonical client-side evoker-fangs attack animation, mirroring vanilla `EvokerFangs`
/// (`clientSideAttackStarted` / `lifeTicks`). The fang is spawned underground and hidden until the
/// server broadcasts entity event `4` (`EvokerFangs.handleEntityEvent` → `clientSideAttackStarted =
/// true`); from then on `EvokerFangs.tick` decrements `lifeTicks` (initially `22`) each client tick.
/// `getAnimationProgress` builds the `0..1` bite ramp from `lifeTicks`, which
/// `EvokerFangsModel.setupAnim` turns into the jaw snap, the rise out of the ground, and the final
/// vanish (root scale → 0). The attack sound on the same event is audio, not pose, and is not modelled.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EvokerFangsAnimationState {
    /// Vanilla `EvokerFangs.clientSideAttackStarted`.
    #[serde(default)]
    pub started: bool,
    /// Vanilla `EvokerFangs.lifeTicks` (field initializer `22`).
    #[serde(default)]
    pub life_ticks: i32,
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

/// Canonical client-side guardian spike-withdrawal accumulator, mirroring vanilla
/// `Guardian.clientSideSpikesAnimation`/`clientSideSpikesAnimationO` (both `0.0` at spawn). Each
/// client tick saves the previous value (the lerp endpoint), then IN WATER eases `spikes_animation`
/// toward `0` while `isMoving()` (the spikes retract as it swims) by `0.25` or toward `1` while idle
/// (the spikes fully extend) by `0.06`. Vanilla OUT OF WATER instead sets it to a fresh
/// `random.nextFloat()` every tick; that flicker is not seeded on the client, so it is deferred (held
/// at the last value) rather than faked — a guardian out of water is a flopping/dying edge case.
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
    /// endpoint, then ease toward `0` (moving) / `1` (idle) in water. Out of water vanilla randomizes
    /// it each tick (`random.nextFloat()`) — not reconstructable, so the value is held steady.
    fn advance_client_tick(&mut self, in_water: bool, is_moving: bool) {
        self.previous_spikes_animation = self.spikes_animation;
        if !in_water {
            // Deferred: vanilla `clientSideSpikesAnimation = random.nextFloat()` flickers the spikes
            // with an unseeded client RNG. Hold the last value rather than fake the randomness.
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
        if self.prev_pose == VANILLA_POSE_SLIDING_ID && pose_id != VANILLA_POSE_SLIDING_ID {
            self.slide_back.start_age = Some(age_ticks);
        }
        self.prev_pose = pose_id;
        self.shoot
            .animate_when(pose_id == VANILLA_POSE_SHOOTING_ID, age_ticks);
        self.inhale
            .animate_when(pose_id == VANILLA_POSE_INHALING_ID, age_ticks);
        self.slide
            .animate_when(pose_id == VANILLA_POSE_SLIDING_ID, age_ticks);
        self.long_jump
            .animate_when(pose_id == VANILLA_POSE_LONG_JUMPING_ID, age_ticks);
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
        entity_type_id: i32,
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
        let target_speed = walk_update_target_speed(entity_type_id, distance);
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
/// `Camel`/`Frog` additionally gate on pose/jump/dash animation states the client
/// does not fully track; the `Creaking` override is pure (`min(distance·25, 3)`,
/// driven here), so it is no longer deferred.
fn walk_animation_override_is_deferred(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_CAMEL_ID | VANILLA_ENTITY_TYPE_FROG_ID
    )
}

/// Vanilla `updateWalkAnimation`'s per-entity distance→target-speed mapping, fed to
/// `WalkAnimationState.update`. The base `LivingEntity` clamps `distance · 4` to `1.0`; `Creaking`
/// overrides it to `distance · 25` clamped to `3.0` (a faster, higher-amplitude gait that ramps the
/// limb-swing position ~3× quicker). The `Camel`/`Frog` overrides additionally gate on pose/jump
/// animation states the client does not track, so they stay deferred (and never reach here).
fn walk_update_target_speed(entity_type_id: i32, distance: f32) -> f32 {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_CREAKING_ID => (distance * 25.0).min(3.0),
        _ => (distance * 4.0).min(1.0),
    }
}

/// Vanilla `WitherBoss.getAlternativeTarget(1/2)`: side heads read
/// `DATA_TARGET_B/C`. Target `0` means "no target".
pub(crate) fn wither_side_head_target_ids(data_values: &[EntityDataValue]) -> [i32; 2] {
    [
        entity_data_int(data_values, WITHER_TARGET_B_DATA_ID, 0),
        entity_data_int(data_values, WITHER_TARGET_C_DATA_ID, 0),
    ]
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
    /// Advances one client tick of `Slime.tick`'s squish accumulator.
    ///
    /// Vanilla saves the lerp endpoint (`oSquish`), eases `squish` halfway toward
    /// `targetSquish`, then — after `super.tick()` — re-seeds the target on the
    /// `onGround()` landing (`-0.5`) / takeoff (`1.0`) transitions, records the
    /// ground flag, and decays the target by `0.6` (`decreaseSquish`). The landing
    /// squish particles and sound in the same method are effects, not pose state.
    fn advance_client_tick(&mut self, on_ground: bool) {
        self.o_squish = self.squish;
        self.squish += (self.target_squish - self.squish) * 0.5;
        if on_ground && !self.was_on_ground {
            self.target_squish = -0.5;
        } else if !on_ground && self.was_on_ground {
            self.target_squish = 1.0;
        }
        self.was_on_ground = on_ground;
        self.target_squish *= 0.6;
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
        }
    }
}

impl EvokerFangsAnimationState {
    /// Advances one client tick of `EvokerFangs.tick`. Vanilla decrements `lifeTicks`
    /// every tick once the attack has started; the count is clamped at `2` (where the
    /// progress ramp has already saturated at `1.0` — the vanished state) to bound it
    /// while the entity waits to be removed.
    fn advance_client_tick(&mut self) {
        if self.started && self.life_ticks > 2 {
            self.life_ticks -= 1;
        }
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
    pub(crate) fn sync_events_from_metadata_update(
        &mut self,
        entity_type_id: i32,
        updated_values: &[EntityDataValue],
        data_values: &[EntityDataValue],
    ) {
        // Vanilla `AbstractArrow.onSyncedDataUpdated(IN_GROUND)`: once the entity is past
        // its first client tick, setting `IN_GROUND` true starts the seven-tick impact
        // wobble if the previous shake has already settled.
        if is_vanilla_arrow_type(entity_type_id)
            && self.age_ticks > 0
            && updated_values
                .iter()
                .any(|value| value.data_id == ABSTRACT_ARROW_IN_GROUND_DATA_ID)
            && entity_data_bool(data_values, ABSTRACT_ARROW_IN_GROUND_DATA_ID, false)
            && self
                .arrow_shake
                .map_or(true, ArrowShakeAnimationState::is_settled)
        {
            self.arrow_shake = Some(ArrowShakeAnimationState::started());
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
        if vanilla_living_entity_type(entity_type_id) {
            if let Some(health) = entity_data_float(data_values, VANILLA_ENTITY_HEALTH_DATA_ID) {
                if health <= 0.0 {
                    self.death
                        .get_or_insert(DeathAnimationState { death_time: 0 });
                } else {
                    self.death = None;
                }
            }
            let visually_swimming = entity_data_pose(data_values) == VANILLA_POSE_SWIMMING_ID;
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
            VANILLA_ENTITY_TYPE_PANDA_ID => {
                let (sitting, on_back, rolling) = panda_pose_flags(data_values);
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
                // Vanilla `Frog.onSyncedDataUpdated`: `tongueAnimationState` is started on the synced
                // `Pose.USING_TONGUE` and stopped otherwise — the same rising-edge pattern as the
                // croak, driving the `FROG_TONGUE` keyframe (head dip + the tongue's z-scale lash).
                let using_tongue = pose == VANILLA_POSE_USING_TONGUE_ID;
                if let Some(tongue) = self.frog_tongue.as_mut() {
                    tongue.animate_when(using_tongue, self.age_ticks);
                } else if using_tongue {
                    self.frog_tongue = Some(KeyframeAnimationState {
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
                    self.armadillo = Some(ArmadilloAnimationState::new(state_id, self.age_ticks));
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
        } else if entity_type_id == VANILLA_ENTITY_TYPE_RABBIT_ID
            && event_id == RABBIT_JUMP_EVENT_ID
        {
            // Vanilla `Rabbit.handleEntityEvent(1)`: seed the hop reconstruction (`jumpDuration = 15;
            // jumpTicks = 0`); the next tick's `setupAnimationStates` starts the hop keyframe.
            self.rabbit_hop
                .get_or_insert(RabbitHopAnimationState {
                    jump_ticks: 0,
                    jump_duration: 0,
                    hop: KeyframeAnimationState { start_age: None },
                })
                .start_jump();
        } else if entity_type_id == VANILLA_ENTITY_TYPE_ARMADILLO_ID
            && event_id == ARMADILLO_PEEK_EVENT_ID
        {
            // Vanilla `Armadillo.handleEntityEvent(64)`: set `peekReceivedClient`; the next
            // `setupAnimationStates()` consumes it by stopping the old peek and immediately
            // restarting it for the current SCARED state.
            let age_ticks = self.age_ticks;
            self.armadillo
                .get_or_insert_with(|| {
                    ArmadilloAnimationState::new(ARMADILLO_STATE_IDLE_ID, age_ticks)
                })
                .peek_received_client = true;
        } else if entity_type_id == VANILLA_ENTITY_TYPE_CREAKING_ID
            && matches!(
                event_id,
                CREAKING_ATTACK_EVENT_ID | CREAKING_INVULNERABLE_EVENT_ID
            )
        {
            // Vanilla `Creaking.handleEntityEvent`: event 4 → `attackAnimationRemainingTicks = 15`,
            // 66 → `invulnerabilityAnimationRemainingTicks = 8`. The `aiStep` countdown advances from
            // there; the next tick's `setupAnimationStates` starts the matching keyframe one-shot.
            let creaking = self
                .creaking
                .get_or_insert_with(CreakingAnimationState::default);
            if event_id == CREAKING_ATTACK_EVENT_ID {
                creaking.start_attack();
            } else {
                creaking.start_invulnerable();
            }
        } else if entity_type_id == VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID
            && event_id == EVOKER_FANGS_ATTACK_EVENT_ID
        {
            // Vanilla `EvokerFangs.handleEntityEvent`: event 4 → `clientSideAttackStarted = true`,
            // arming the `lifeTicks` countdown that `tick` runs.
            self.evoker_fangs
                .get_or_insert_with(EvokerFangsAnimationState::default)
                .started = true;
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

    /// Vanilla `IllagerRenderState.ticksUsingItem` (`getTicksUsingItem(partialTicks)`) for the pillager's
    /// `CROSSBOW_CHARGE` draw, reconstructed from the charge counter. `0.0` for a pillager that is not
    /// charging and every other entity.
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

    pub(crate) fn advance_client_tick(
        &mut self,
        entity_type_id: i32,
        entity_id: i32,
        transform: EntityTransform,
        is_passenger: bool,
        is_baby: bool,
        is_fall_flying: bool,
        is_crouching: bool,
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
        // Vanilla `WitherBoss.aiStep` side-head targets, pre-resolved by `EntityStore` because the
        // tick needs other entities' current positions and eye heights. `None` for non-withers.
        wither_head_targets: Option<[WitherHeadTargetRotations; 2]>,
        // The synced `Camel.DASH` boolean (`isDashing()`), read from the entity metadata in the tick
        // loop ([`camel_is_dashing`]). `false` for entities that do not consume it.
        camel_is_dashing: bool,
        // The synced `Allay.DATA_DANCING` boolean (`isDancing()`), read from the entity metadata in the
        // tick loop ([`allay_is_dancing`]). `false` for entities that do not consume it.
        allay_is_dancing: bool,
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
        // Vanilla `Entity.isSwimming()` for player cape bob suppression.
        is_swimming: bool,
    ) {
        self.age_ticks = self.age_ticks.saturating_add(1);
        // Vanilla `AbstractArrow.tick`: `if (shakeTime > 0) --shakeTime`.
        // The state is only created for arrow/spectral-arrow metadata updates, so
        // it can advance outside the per-type match without touching other entities.
        if let Some(shake) = self.arrow_shake.as_mut() {
            shake.advance_client_tick();
            if shake.is_settled() {
                self.arrow_shake = None;
            }
        }
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
                        .get_or_insert_with(CreakingAnimationState::default);
                    creaking.advance_client_tick(self.age_ticks, creaking_is_tearing_down);
                    if creaking.is_settled() {
                        self.creaking = None;
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
                    wolf_wet.advance_client_tick(in_water, transform.on_ground.unwrap_or(false));
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
                self.warden_heart
                    .get_or_insert_with(WardenHeartAnimationState::default)
                    .advance_client_tick(self.age_ticks, warden_heartbeat_delay);
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
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => self
                .ender_dragon
                .get_or_insert_with(EnderDragonAnimationState::default)
                .advance_client_tick(transform),
            VANILLA_ENTITY_TYPE_SQUID_ID | VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => self
                .squid
                .get_or_insert_with(|| {
                    SquidAnimationState::new_with_y_body_rot(entity_id, transform.y_head_rot)
                })
                .advance_client_tick(transform.delta_movement, in_water),
            VANILLA_ENTITY_TYPE_GUARDIAN_ID | VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID => {
                // Vanilla `Guardian.aiStep` reads `isInWater()` (the world fact
                // threaded in) and the synced `isMoving()` flag (`DATA_ID_MOVING`).
                self.guardian_tail
                    .get_or_insert_with(GuardianTailAnimationState::default)
                    .advance_client_tick(in_water, is_moving);
                // The same `aiStep` eases the spike-withdrawal accumulator (retract while swimming,
                // extend while idle); out of water it would randomize, which is deferred.
                self.guardian_spikes
                    .get_or_insert_with(GuardianSpikesAnimationState::default)
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
            VANILLA_ENTITY_TYPE_CAMEL_ID => {
                // Vanilla `Camel.setupAnimationStates`: `dashAnimationState.animateWhen(isDashing(),
                // tickCount)` — the synced `DASH` boolean starts the looping gallop on its rising edge.
                // (Vanilla also `stop()`s it while visually sitting, but a sitting camel never has DASH
                // set, so gating purely on the flag is equivalent. The camel's sit/stand poses are
                // derived separately in the native layer from `LAST_POSE_CHANGE_TICK`.)
                self.camel_dash
                    .get_or_insert_with(|| KeyframeAnimationState { start_age: None })
                    .animate_when(camel_is_dashing, self.age_ticks);
            }
            VANILLA_ENTITY_TYPE_ALLAY_ID => {
                // Vanilla `Allay.tick`: while the synced `DATA_DANCING` flag is set, advance the dance
                // and spin counters; otherwise reset them. The counters drive `AllayModel`'s body spin.
                self.allay_dance
                    .get_or_insert_with(AllayDanceAnimationState::default)
                    .advance_client_tick(allay_is_dancing);
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
                // Vanilla `Axolotl.tickAdultAnimations`: the play-dead / in-water / on-ground state
                // machine drives three `BinaryAnimator`s plus the moving animator.
                // `walkAnimation.isMoving()` reads the PRIOR tick's limb swing (the walk animation is
                // advanced after this match, mirroring vanilla's `baseTick`-before-`aiStep` order),
                // OR'd with a body/head rotation change.
                let walk_is_moving = self.walk_animation.is_some_and(|walk| walk.is_moving());
                self.axolotl
                    .get_or_insert_with(AxolotlAnimationState::default)
                    .advance_client_tick(
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
            VANILLA_ENTITY_TYPE_SLIME_ID | VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID => self
                .slime
                .get_or_insert_with(SlimeAnimationState::default)
                // Vanilla `Slime.tick` reads `onGround()` for the squish target. A
                // slime with no synced ground flag defaults to on-ground (resting
                // squish), the safe common case.
                .advance_client_tick(transform.on_ground.unwrap_or(true)),
            VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID => {
                // Vanilla `EvokerFangs.tick` counts down `lifeTicks` once the attack
                // has started (the event arm seeds the state); an un-attacked fang
                // holds `started = false` (hidden) and is left untouched.
                if let Some(fangs) = self.evoker_fangs.as_mut() {
                    fangs.advance_client_tick();
                }
            }
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
                .advance_client_tick(
                    entity_type_id,
                    transform.position,
                    use_y,
                    is_passenger,
                    is_alive,
                    is_baby,
                );
        }
    }
}

/// Whether an entity type's client-tick animation reads `isInWater()` and so needs
/// the per-entity `in_water` map [`WorldStore::advance_entity_client_animations`]
/// builds. Keeping this cheap (a tiny `match`) lets the world skip the AABB / fluid
/// probe for every other entity. Adding a consumer (the frog swim-idle, wolf wet shade,
/// dolphin, axolotl, fish, …) is just another arm here.
pub(crate) fn entity_animation_uses_in_water(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_GUARDIAN_ID
            | VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID
            | VANILLA_ENTITY_TYPE_FROG_ID
            | VANILLA_ENTITY_TYPE_WOLF_ID
            | VANILLA_ENTITY_TYPE_AXOLOTL_ID
            | VANILLA_ENTITY_TYPE_SQUID_ID
            | VANILLA_ENTITY_TYPE_GLOW_SQUID_ID
    )
}

fn is_vanilla_arrow_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_ARROW_ID | VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID
    )
}

/// Vanilla `Guardian.isMoving()` = the synced `DATA_ID_MOVING` boolean (default
/// `false`). Read straight from the entity metadata in the tick loop.
pub(crate) fn guardian_is_moving(data_values: &[EntityDataValue]) -> bool {
    entity_data_bool(data_values, GUARDIAN_MOVING_DATA_ID, false)
}

/// Vanilla `Camel.isDashing()` (`entityData.get(DASH)`): the synced boolean that gates the looping
/// dash gallop. Read straight from the entity metadata in the tick loop; non-camels (whose synced slot
/// `19` is not this boolean) fall back to `false`.
pub(crate) fn camel_is_dashing(data_values: &[EntityDataValue]) -> bool {
    entity_data_bool(data_values, CAMEL_DASH_DATA_ID, false)
}

/// Vanilla `Pillager.isChargingCrossbow()` (`entityData.get(IS_CHARGING_CROSSBOW)`): the synced boolean
/// that drives the `CROSSBOW_CHARGE` draw — the client counts ticks since it rose for `getTicksUsingItem`.
/// Read straight from the entity metadata in the tick loop; non-pillagers (whose synced slot `17` is not
/// this boolean — e.g. the spellcaster illagers' `DATA_SPELL_CASTING_ID` byte) fall back to `false`.
pub(crate) fn pillager_is_charging_crossbow(data_values: &[EntityDataValue]) -> bool {
    entity_data_bool(data_values, PILLAGER_IS_CHARGING_CROSSBOW_DATA_ID, false)
}

/// Vanilla `Piglin.isChargingCrossbow()` (`entityData.get(DATA_IS_CHARGING_CROSSBOW)`): the synced boolean
/// (id 18) that drives the regular piglin's `CROSSBOW_CHARGE` draw — the client counts ticks since it rose
/// for `getTicksUsingItem`. Read straight from the entity metadata in the tick loop; the type gate in
/// `advance_client_tick` keeps non-piglins (whose slot 18 holds something else) off this counter.
pub(crate) fn piglin_is_charging_crossbow(data_values: &[EntityDataValue]) -> bool {
    entity_data_bool(data_values, PIGLIN_IS_CHARGING_CROSSBOW_DATA_ID, false)
}

/// Vanilla `LivingEntity.isUsingItem()` (`DATA_LIVING_ENTITY_FLAGS & 1`, synced byte id `8`): true while the
/// entity holds right-click on a usable item. For the player this drives the shared crossbow-draw counter
/// (the client's `getTicksUsingItem` reconstruction, which is item-agnostic in vanilla); the native layer
/// then applies the `CROSSBOW_CHARGE` pose only when the using item is actually an uncharged crossbow. Read
/// straight from the entity metadata in the tick loop; the type gate in `advance_client_tick` keeps this on
/// the player.
pub(crate) fn player_is_using_item(data_values: &[EntityDataValue]) -> bool {
    const LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
    const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;
    entity_data_byte(data_values, LIVING_ENTITY_FLAGS_DATA_ID, 0) & LIVING_ENTITY_FLAG_IS_USING != 0
}

/// Vanilla `Allay.isDancing()` (`entityData.get(DATA_DANCING)`): the synced boolean that drives the
/// dance/spin accumulators. Read straight from the entity metadata in the tick loop; non-allays (whose
/// synced slot `16` is not this boolean) fall back to `false`.
pub(crate) fn allay_is_dancing(data_values: &[EntityDataValue]) -> bool {
    entity_data_bool(data_values, ALLAY_DANCING_DATA_ID, false)
}

/// Vanilla `Axolotl.isPlayingDead()` (`entityData.get(DATA_PLAYING_DEAD)`): the synced boolean that
/// selects the `PLAYING_DEAD` animation state. Read straight from the entity metadata in the tick
/// loop; non-axolotls (whose synced slot `19` is not this boolean) fall back to `false`.
pub(crate) fn axolotl_is_playing_dead(data_values: &[EntityDataValue]) -> bool {
    entity_data_bool(data_values, AXOLOTL_PLAYING_DEAD_DATA_ID, false)
}

/// Vanilla `Creaking.canMove()` (`entityData.get(CAN_MOVE)`): the synced boolean (default `true`)
/// that gates the looping walk. Read straight from the entity metadata; a creaking frozen while a
/// player observes it turns to a statue. Non-creakings fall back to `true` (the field only feeds the
/// creaking model).
pub(crate) fn creaking_can_move(data_values: &[EntityDataValue]) -> bool {
    entity_data_bool(data_values, CREAKING_CAN_MOVE_DATA_ID, true)
}

/// Vanilla `Creaking.isTearingDown()` (`entityData.get(IS_TEARING_DOWN)`): the synced boolean
/// (default `false`) that drives the death collapse. Read straight from the entity metadata in the
/// tick loop; non-creakings fall back to `false`.
pub(crate) fn creaking_is_tearing_down(data_values: &[EntityDataValue]) -> bool {
    entity_data_bool(data_values, CREAKING_IS_TEARING_DOWN_DATA_ID, false)
}

/// Vanilla `LivingEntity.isFallFlying()` = `Entity.getSharedFlag(7)`, where
/// `Entity.DATA_SHARED_FLAGS_ID` is metadata id `0`.
pub(crate) fn entity_is_fall_flying(data_values: &[EntityDataValue]) -> bool {
    entity_data_byte(data_values, ENTITY_SHARED_FLAGS_DATA_ID, 0) as u8 & ENTITY_FLAG_FALL_FLYING
        != 0
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

fn panda_pose_flags(data_values: &[EntityDataValue]) -> (bool, bool, bool) {
    let flags = entity_data_byte(data_values, PANDA_FLAGS_DATA_ID, 0);
    (
        flags & PANDA_FLAG_SITTING != 0,
        flags & PANDA_FLAG_ON_BACK != 0,
        flags & PANDA_FLAG_ROLLING != 0,
    )
}

/// Vanilla `Fox.isInterested()` = `getFlag(FLAG_INTERESTED)` = `(flags & 8) != 0`.
fn fox_is_interested(data_values: &[EntityDataValue]) -> bool {
    entity_data_byte(data_values, FOX_FLAGS_DATA_ID, 0) & FOX_FLAG_INTERESTED != 0
}

/// Vanilla `Wolf.isInterested()` = `entityData.get(DATA_INTERESTED_ID)`.
pub(crate) fn wolf_is_interested(data_values: &[EntityDataValue]) -> bool {
    entity_data_bool(data_values, WOLF_INTERESTED_DATA_ID, false)
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
