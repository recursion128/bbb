use std::collections::BTreeMap;

use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, AttributeSnapshot as ProtocolAttributeSnapshot,
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
    EquipmentSlotUpdate as ProtocolEquipmentSlotUpdate,
    ItemStackSummary as ProtocolItemStackSummary, MinecartStep as ProtocolMinecartStep,
    RemoveEntities as ProtocolRemoveEntities, TakeItemEntity as ProtocolTakeItemEntity,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{BlockPos, TerrainLight, WorldStore};

mod animations;
mod components;
mod dimensions;
mod dragon;
mod metadata;
mod movement;
mod passengers;
mod projectiles;
pub(crate) mod state;
mod status;
mod store;
mod updates;

pub use animations::{EntityClientAnimationState, PolarBearStandingAnimationState};
pub(crate) use components::{
    EntityAttributes, EntityClientAnimations, EntityDamage, EntityEquipment,
    EntityHurtingProjectile, EntityIdentity, EntityLeash, EntityMetadata, EntityMinecartLerp,
    EntityMobEffects, EntityMount, EntityTransform, EntityTransientEvents,
};
use dimensions::vanilla_client_position_for_entity_data;
pub use dimensions::EntityPickBoundsState;
pub use dragon::{DragonFlightHistorySample, DragonFlightHistoryState, EnderDragonAnimationState};
use movement::entity_vec3;
use projectiles::initial_hurting_projectile_state;
use status::{EntityDamageEventState, MobEffectState};
pub(crate) use store::EntityStore;

// IDs are the vanilla 26.1 EntityType registry order from EntityType.java.
pub(crate) const VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID: i32 = 0;
pub(crate) const VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID: i32 = 1;
pub(crate) const VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID: i32 = 8;
pub(crate) const VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID: i32 = 9;
pub(crate) const VANILLA_ENTITY_TYPE_BIRCH_BOAT_ID: i32 = 12;
pub(crate) const VANILLA_ENTITY_TYPE_BIRCH_CHEST_BOAT_ID: i32 = 13;
pub(crate) const VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID: i32 = 18;
pub(crate) const VANILLA_ENTITY_TYPE_CAMEL_ID: i32 = 19;
pub(crate) const VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID: i32 = 20;
pub(crate) const VANILLA_ENTITY_TYPE_CHERRY_BOAT_ID: i32 = 23;
pub(crate) const VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID: i32 = 24;
pub(crate) const VANILLA_ENTITY_TYPE_CHEST_MINECART_ID: i32 = 25;
pub(crate) const VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID: i32 = 29;
pub(crate) const VANILLA_ENTITY_TYPE_DARK_OAK_BOAT_ID: i32 = 33;
pub(crate) const VANILLA_ENTITY_TYPE_DARK_OAK_CHEST_BOAT_ID: i32 = 34;
pub(crate) const VANILLA_ENTITY_TYPE_DONKEY_ID: i32 = 36;
pub(crate) const VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID: i32 = 37;
pub(crate) const VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID: i32 = 49;
pub(crate) const VANILLA_ENTITY_TYPE_FIREBALL_ID: i32 = 52;
pub(crate) const VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID: i32 = 56;
pub(crate) const VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID: i32 = 65;
pub(crate) const VANILLA_ENTITY_TYPE_HORSE_ID: i32 = 66;
pub(crate) const VANILLA_ENTITY_TYPE_ITEM_ID: i32 = 71;
pub(crate) const VANILLA_ENTITY_TYPE_JUNGLE_BOAT_ID: i32 = 74;
pub(crate) const VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID: i32 = 75;
pub(crate) const VANILLA_ENTITY_TYPE_LLAMA_ID: i32 = 78;
pub(crate) const VANILLA_ENTITY_TYPE_MANGROVE_BOAT_ID: i32 = 81;
pub(crate) const VANILLA_ENTITY_TYPE_MANGROVE_CHEST_BOAT_ID: i32 = 82;
pub(crate) const VANILLA_ENTITY_TYPE_MINECART_ID: i32 = 85;
pub(crate) const VANILLA_ENTITY_TYPE_MULE_ID: i32 = 87;
pub(crate) const VANILLA_ENTITY_TYPE_NAUTILUS_ID: i32 = 88;
pub(crate) const VANILLA_ENTITY_TYPE_OAK_BOAT_ID: i32 = 89;
pub(crate) const VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID: i32 = 90;
pub(crate) const VANILLA_ENTITY_TYPE_PALE_OAK_BOAT_ID: i32 = 94;
pub(crate) const VANILLA_ENTITY_TYPE_PALE_OAK_CHEST_BOAT_ID: i32 = 95;
pub(crate) const VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID: i32 = 116;
pub(crate) const VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID: i32 = 118;
pub(crate) const VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID: i32 = 122;
pub(crate) const VANILLA_ENTITY_TYPE_SPRUCE_BOAT_ID: i32 = 125;
pub(crate) const VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID: i32 = 126;
pub(crate) const VANILLA_ENTITY_TYPE_TNT_MINECART_ID: i32 = 133;
pub(crate) const VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID: i32 = 134;
pub(crate) const VANILLA_ENTITY_TYPE_WIND_CHARGE_ID: i32 = 143;
pub(crate) const VANILLA_ENTITY_TYPE_WITHER_SKULL_ID: i32 = 147;
pub(crate) const VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID: i32 = 151;
pub(crate) const VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID: i32 = 152;
pub(crate) const VANILLA_ENTITY_TYPE_PLAYER_ID: i32 = 155;

/// Vanilla `LivingEntityRenderer.isUpsideDownName`: the custom/profile names that
/// flip a living entity upside down (the Dinnerbone/Grumm easter egg).
pub(crate) const VANILLA_UPSIDE_DOWN_NAMES: [&str; 2] = ["Dinnerbone", "Grumm"];
pub(crate) const VANILLA_ENTITY_SILENT_DATA_ID: u8 = 4;
pub(crate) const VANILLA_ENTITY_NO_GRAVITY_DATA_ID: u8 = 5;
pub(crate) const VANILLA_ENTITY_TICKS_FROZEN_DATA_ID: u8 = 7;
pub(crate) const VANILLA_ITEM_ENTITY_STACK_DATA_ID: u8 = 8;

pub(crate) fn is_vanilla_boat_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID
            | VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID
            | VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID
            | VANILLA_ENTITY_TYPE_BIRCH_BOAT_ID
            | VANILLA_ENTITY_TYPE_BIRCH_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_CHERRY_BOAT_ID
            | VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_DARK_OAK_BOAT_ID
            | VANILLA_ENTITY_TYPE_DARK_OAK_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_JUNGLE_BOAT_ID
            | VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_MANGROVE_BOAT_ID
            | VANILLA_ENTITY_TYPE_MANGROVE_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_OAK_BOAT_ID
            | VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_PALE_OAK_BOAT_ID
            | VANILLA_ENTITY_TYPE_PALE_OAK_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_SPRUCE_BOAT_ID
            | VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID
    )
}

pub(crate) fn is_vanilla_abstract_horse_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_CAMEL_ID
            | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
            | VANILLA_ENTITY_TYPE_DONKEY_ID
            | VANILLA_ENTITY_TYPE_HORSE_ID
            | VANILLA_ENTITY_TYPE_LLAMA_ID
            | VANILLA_ENTITY_TYPE_MULE_ID
            | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
            | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
    )
}

pub(crate) fn is_vanilla_abstract_nautilus_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
    )
}

pub(crate) fn is_vanilla_can_equip_saddle_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_CAMEL_ID
            | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
            | VANILLA_ENTITY_TYPE_DONKEY_ID
            | VANILLA_ENTITY_TYPE_HORSE_ID
            | VANILLA_ENTITY_TYPE_MULE_ID
            | VANILLA_ENTITY_TYPE_NAUTILUS_ID
            | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
    )
}

pub(crate) fn is_vanilla_can_wear_horse_armor_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_HORSE_ID | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
    )
}

pub(crate) fn is_vanilla_horse_slot_always_active_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_HORSE_ID
            | VANILLA_ENTITY_TYPE_LLAMA_ID
            | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
            | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
    )
}

pub(crate) fn is_vanilla_llama_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
    )
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityVec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemEntityStackState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub stack: ProtocolItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityState {
    pub id: i32,
    pub uuid: Uuid,
    pub entity_type_id: i32,
    pub data: i32,
    pub position: EntityVec3,
    pub position_base: EntityVec3,
    pub delta_movement: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub y_head_rot: f32,
    pub on_ground: Option<bool>,
    pub data_values: Vec<ProtocolEntityDataValue>,
    pub equipment: Vec<ProtocolEquipmentSlotUpdate>,
    pub attributes: Vec<ProtocolAttributeSnapshot>,
    pub vehicle_id: Option<i32>,
    pub passengers: Vec<i32>,
    pub leash_holder_id: Option<i32>,
    pub last_animation_action: Option<u8>,
    pub last_event_id: Option<i8>,
    pub last_hurt_yaw: Option<f32>,
    #[serde(default)]
    pub mob_effects: BTreeMap<i32, MobEffectState>,
    #[serde(default)]
    pub client_animations: EntityClientAnimationState,
    #[serde(default)]
    pub last_damage: Option<EntityDamageEventState>,
    #[serde(default)]
    pub minecart_lerp_steps: Vec<ProtocolMinecartStep>,
    #[serde(default)]
    pub hurting_projectile: Option<HurtingProjectileState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HurtingProjectileState {
    pub acceleration_power: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProjectilePowerUpdateState {
    pub entity_id: i32,
    pub acceleration_power: f64,
    pub applied: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehicleMoveReport {
    pub vehicle_id: i32,
    pub position: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
    pub snapped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityTransformState {
    pub id: i32,
    pub uuid: Uuid,
    pub entity_type_id: i32,
    pub data: i32,
    pub position: EntityVec3,
    pub position_base: EntityVec3,
    pub delta_movement: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub y_head_rot: f32,
    pub on_ground: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityStatusProbeState {
    pub id: i32,
    pub entity_type_id: i32,
    pub last_animation_action: Option<u8>,
    pub last_event_id: Option<i8>,
    pub last_hurt_yaw: Option<f32>,
    pub mob_effects: BTreeMap<i32, MobEffectState>,
    pub last_damage: Option<EntityDamageEventState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityCameraPoseState {
    pub id: i32,
    pub position: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub eye_height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityPickTargetState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub bounds: EntityPickBoundsState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityModelSourceState {
    pub entity_id: i32,
    pub entity_type_id: i32,
    pub position: EntityVec3,
    pub y_rot: f32,
    /// Canonical entity head pitch (`Entity.getXRot`), applied as the model head
    /// `xRot` look.
    #[serde(default)]
    pub x_rot: f32,
    /// Canonical entity head yaw (`Entity.yHeadRot`). The renderer projection
    /// derives the net head-look yaw `Mth.wrapDegrees(y_head_rot - y_rot)`.
    #[serde(default)]
    pub y_head_rot: f32,
    #[serde(default)]
    pub age_ticks: u32,
    /// Vanilla `LivingEntityRenderState.isFullyFrozen` (`Entity.isFullyFrozen`,
    /// `ticksFrozen >= 140`): a living entity frozen solid in powder snow, whose
    /// body the renderer shakes (`LivingEntityRenderer.isShaking`). `false` for
    /// non-living entities and any entity that is not frozen solid.
    #[serde(default)]
    pub is_fully_frozen: bool,
    /// Vanilla `Mob.isAggressive()` (`DATA_MOB_FLAGS_ID & 4`): whether the mob is in its
    /// aggressive AI state, which deepens the held-out `animateZombieArms` arm drop
    /// (`-π / 1.5` aggressive vs `-π / 2.25` calm). Projected only for the zombie-model
    /// family ([`vanilla_zombie_model_family`](crate::entities::dimensions)); `false` for
    /// every other entity (which has no mob-flags byte or does not use those arms).
    #[serde(default)]
    pub is_aggressive: bool,
    /// Vanilla `EndermanRenderState.carriedBlock` non-empty (`Enderman.getCarriedBlock`
    /// present, the synced `DATA_CARRY_STATE`): the enderman is holding a block, which
    /// `EndermanModel.setupAnim` poses both arms forward to carry (`xRot = -0.5`, `zRot =
    /// ±0.05`). Projected only for the enderman ([`vanilla_is_enderman`](crate::entities::dimensions));
    /// `false` for every other entity.
    #[serde(default)]
    pub enderman_carrying: bool,
    /// Vanilla `EndermanRenderState.isCreepy` (`Enderman.isCreepy`, the synced
    /// `DATA_CREEPY`): the enderman is in its aggressive staring state, which
    /// `EndermanModel.setupAnim` shows by dropping the head `y -= 5` and raising the hat
    /// `y += 5` (the open-mouth screech pose). Projected only for the enderman; `false`
    /// for every other entity.
    #[serde(default)]
    pub enderman_creepy: bool,
    /// Vanilla `BatRenderState.isResting` (`Bat.isResting`, the synced `DATA_ID_FLAGS & 1`):
    /// the bat is hanging at rest, which `BatModel.setupAnim` shows by swapping to the
    /// `BatAnimation.BAT_RESTING` upside-down pose (and applying a head look). Projected only
    /// for the bat ([`vanilla_is_bat`](crate::entities::dimensions)); `false` for every other
    /// entity.
    #[serde(default)]
    pub bat_resting: bool,
    /// Vanilla `BeeRenderState.hasStinger` (`!Bee.hasStung()`, the synced `DATA_FLAGS_ID & 4`):
    /// whether the bee still carries its stinger, which `BeeModel.setupAnim` toggles as the
    /// stinger cube's `visible` flag. Projected only for the bee
    /// ([`vanilla_is_bee`](crate::entities::dimensions)) and defaults to `true` (a bee that has
    /// not stung still has its stinger; every other entity is unaffected).
    #[serde(default = "entity_model_source_default_true")]
    pub bee_has_stinger: bool,
    /// Vanilla `BeeRenderState.rollAmount` (`Bee.getRollAmount(partialTick)`, the lerped client
    /// accumulator driven by the synced `DATA_FLAGS_ID & 2` roll flag): a rolling bee tips onto its
    /// back, which `BeeModel.setupAnim` applies as a near-π `bone.xRot` flip. Projected only for the
    /// bee and `0.0` (upright) for every other entity.
    #[serde(default)]
    pub bee_roll_amount: f32,
    /// Vanilla frog croak timing (`FrogRenderState.croakAnimationState`, the triggered
    /// `AnimationState` started/stopped by the synced `Pose.CROAKING`): the elapsed seconds since the
    /// croak started, which `FrogModel.setupAnim` uses to show the `croaking_body` pouch and sample
    /// the `FrogAnimation.FROG_CROAK` animation. Projected only for the frog and `-1.0` (the
    /// stopped-animation sentinel, so the pouch stays hidden) for a non-croaking frog and every other
    /// entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub frog_croak_seconds: f32,
    /// Vanilla frog jump timing (`FrogRenderState.jumpAnimationState`, the triggered `AnimationState`
    /// started/stopped by the synced `Pose.LONG_JUMPING`): the elapsed seconds since the long-jump
    /// started, which `FrogModel.setupAnim` uses to sample the `FrogAnimation.FROG_JUMP` animation.
    /// Projected only for the frog and `-1.0` (the stopped-animation sentinel, so no keyframe is
    /// applied) for a non-jumping frog and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub frog_jump_seconds: f32,
    /// Vanilla frog swim-idle timing (`FrogRenderState.swimIdleAnimationState`, the triggered
    /// `AnimationState` `Frog.tick` drives each client tick via `animateWhen(isInWater() &&
    /// !walkAnimation.isMoving(), tickCount)`): the elapsed seconds since the in-water idle started,
    /// which `FrogModel.setupAnim` uses to sample the looping `FrogAnimation.FROG_IDLE_WATER`
    /// animation. Projected only for the frog and `-1.0` (the stopped-animation sentinel, so no
    /// keyframe is applied) for a frog that is dry or moving and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub frog_swim_idle_seconds: f32,
    /// Vanilla sniffer animation selector (`Sniffer.onSyncedDataUpdated`'s one-shot `AnimationState`s
    /// driven by the synced `DATA_STATE`): the active `Sniffer.State` ordinal whose triggered
    /// keyframe is playing (`FEELING_HAPPY`/`SCENTING`/`SNIFFING`/`DIGGING`/`RISING`), which
    /// `SnifferModel.setupAnim` matches to pick and apply the keyframe def. `-1` (no triggered
    /// animation) for an idling/searching sniffer and every other entity.
    #[serde(default = "entity_model_source_default_neg_one_i32")]
    pub sniffer_animation_id: i32,
    /// Vanilla sniffer animation timing: the elapsed seconds since the active `Sniffer.State`
    /// animation started (paired with [`Self::sniffer_animation_id`]), sampled by
    /// `SnifferModel.setupAnim`. `-1.0` (the stopped-animation sentinel) when no triggered animation
    /// is running.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub sniffer_animation_seconds: f32,
    /// Vanilla `Armadillo.shouldHideInShell()` (`ArmadilloRenderState.isHidingInShell`, the synced
    /// `ARMADILLO_STATE` gated on the client `inStateTicks`): `true` for the steady SCARED ball and
    /// for the ROLLING/UNROLLING transition windows (rolling hides after tick 5, unrolling un-hides at
    /// tick 26), which `ArmadilloModel.setupAnim` renders as the shell ball. `false` (unrolled) for an
    /// IDLE armadillo and every other entity.
    #[serde(default)]
    pub armadillo_is_hiding_in_shell: bool,
    /// Vanilla armadillo roll-up timing (`Armadillo.rollUpAnimationState`, started on entry to
    /// ROLLING): the elapsed seconds since the curl-in began, which `ArmadilloModel.setupAnim` samples
    /// from `ARMADILLO_ROLL_UP` onto the body/legs/head. `-1.0` (the stopped-animation sentinel) when
    /// no roll-up is running and for every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub armadillo_roll_up_seconds: f32,
    /// Vanilla armadillo roll-out timing (`Armadillo.rollOutAnimationState`, started on entry to
    /// UNROLLING): the elapsed seconds since the un-curl began, sampled from `ARMADILLO_ROLL_OUT`.
    /// `-1.0` when no roll-out is running and for every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub armadillo_roll_out_seconds: f32,
    /// Vanilla armadillo peek timing (`Armadillo.peekAnimationState`). Deferred: always `-1.0` (the
    /// peek's `fastForward` baseline is not cleanly derivable; see `docs/unsupported-features.md`),
    /// so the renderer applies no `ARMADILLO_PEEK` keyframe.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub armadillo_peek_seconds: f32,
    /// Vanilla `FoxRenderState.headRollAngle` (`Fox.getHeadRollAngle(partialTick)`, the lerped client
    /// `interestedAngle` accumulator driven by the synced `DATA_FLAGS_ID & 8` interest flag, scaled by
    /// `0.11 · π`): an interested fox tilts its head, which `FoxModel.setWalkingPose` applies as
    /// `head.zRot`. Projected only for the fox and `0.0` (level) for every other entity.
    #[serde(default)]
    pub fox_head_roll_angle: f32,
    /// Vanilla `FoxRenderState.crouchAmount` (`Fox.getCrouchAmount(partialTick)`, the lerped client
    /// `crouchAmount` accumulator driven by the synced `DATA_FLAGS_ID & 4` crouch flag, climbing
    /// `0.2`/tick to `5.0` and reset instantly to `0` when not crouching): a stalking fox lowers its
    /// body, which `FoxModel.setCrouchingPose` applies as `head.y += crouchAmount · ageScale` (and the
    /// subclass `body.y` drop). Projected only for the fox and `0.0` for every other entity.
    #[serde(default)]
    pub fox_crouch_amount: f32,
    /// Vanilla `FoxRenderState.isCrouching` (`Fox.isCrouching()`, the synced `DATA_FLAGS_ID & 4`): a
    /// stalking fox, whose `FoxModel.setupAnim` runs `setCrouchingPose` (overriding the sleeping/sitting
    /// branches). Projected only for the fox; `false` for every other entity.
    #[serde(default)]
    pub fox_is_crouching: bool,
    /// Vanilla `FoxRenderState.isSleeping` (`Fox.isSleeping()`, the synced `DATA_FLAGS_ID & 32`): a
    /// sleeping fox, whose `FoxModel.setupAnim` runs `setSleepingPose` (hiding all four legs) and
    /// overrides the head pose. Projected only for the fox; `false` for every other entity.
    #[serde(default)]
    pub fox_is_sleeping: bool,
    /// Vanilla `FoxRenderState.isSitting` (`Fox.isSitting()`, the synced `DATA_FLAGS_ID & 1`): a
    /// perched fox, whose `FoxModel.setupAnim` runs `setSittingPose`. Projected only for the fox;
    /// `false` for every other entity.
    #[serde(default)]
    pub fox_is_sitting: bool,
    /// Vanilla `FoxRenderState.isPouncing` (`Fox.isPouncing()`, the synced `DATA_FLAGS_ID & 16`): a
    /// pouncing fox, whose `FoxModel.setupAnim` runs `setPouncingPose`. Projected only for the fox;
    /// `false` for every other entity. The `FoxRenderer.setupRotations` body-pitch flip stays deferred.
    #[serde(default)]
    pub fox_is_pouncing: bool,
    /// Vanilla `FoxRenderState.isFaceplanted` (`Fox.isFaceplanted()`, the synced `DATA_FLAGS_ID & 64`):
    /// a face-planted fox, whose `FoxModel.setupAnim` twitches all four legs. Projected only for the
    /// fox; `false` for every other entity. The `FoxRenderer.setupRotations` body-pitch flip stays
    /// deferred.
    #[serde(default)]
    pub fox_is_faceplanted: bool,
    /// Vanilla `VexRenderState.isCharging` (`Vex.isCharging`, the synced `DATA_FLAGS_ID & 1`):
    /// the vex is charging an attack, which `VexModel.setupAnim` shows by leveling the body
    /// (`xRot = 0`) and raising both arms (`setArmsCharging`). Projected only for the vex
    /// ([`vanilla_is_vex`](crate::entities::dimensions)); `false` for every other entity.
    #[serde(default)]
    pub vex_charging: bool,
    /// Vanilla `WitherRenderState.invulnerableTicks` (`WitherBoss.getInvulnerableTicks`, the synced
    /// `DATA_ID_INV` spawn countdown, lerped `invulnerableTicks - partialTicks`): the wither's
    /// spawn-charge progress. `WitherBossRenderer.scale` shrinks the model by
    /// `invulnerableTicks / 220 * 0.5` off the base `2.0` scale, and `getTextureLocation` swaps in
    /// `wither_invulnerable.png` (flickering every 5 ticks once `<= 80`). Projected only for the
    /// wither ([`vanilla_is_wither`](crate::entities::dimensions)); `0.0` for every other entity.
    #[serde(default)]
    pub wither_invulnerable_ticks: f32,
    /// Vanilla `LivingEntityRenderState.isCrouching` (`Entity.isCrouching`, the synced
    /// `Pose.CROUCHING`): a sneaking player, whose `HumanoidModel.setupAnim` leans the body,
    /// drops the head, tucks the legs and tilts the arms. Projected only for the player (the
    /// only entity the server puts in the crouch pose); `false` for every other entity.
    #[serde(default)]
    pub is_crouching: bool,
    /// Vanilla `LivingEntityRenderState.isAutoSpinAttack`
    /// (`LivingEntity.isAutoSpinAttack`, `DATA_LIVING_ENTITY_FLAGS & 4`): a living
    /// entity mid riptide-trident spin, which the renderer flips onto the spin
    /// axis in `LivingEntityRenderer.setupRotations`. `false` for non-living
    /// entities and any living entity that is not spinning.
    #[serde(default)]
    pub is_auto_spin_attack: bool,
    /// Vanilla `LivingEntityRenderState.isUpsideDown`
    /// (`LivingEntityRenderer.isEntityUpsideDown`): a living entity (other than a
    /// player) whose custom name is the `Dinnerbone`/`Grumm` easter egg, which the
    /// renderer flips upside down in `LivingEntityRenderer.setupRotations`. `false`
    /// for non-living entities and any entity that is not named so.
    #[serde(default)]
    pub is_upside_down: bool,
    /// Vanilla `EntityRenderState.boundingBoxHeight` (`Entity.getBbHeight`): the
    /// entity's current AABB height in world units, used by the upside-down branch
    /// of `LivingEntityRenderer.setupRotations` to lift the model before flipping.
    #[serde(default)]
    pub bounding_box_height: f32,
    /// Vanilla `LivingEntityRenderState.hasPose(Pose.SLEEPING)`: the entity is
    /// lying down in a bed, which `LivingEntityRenderer.setupRotations` lays on its
    /// side. `false` for non-living entities and any entity that is not sleeping.
    #[serde(default)]
    pub is_sleeping: bool,
    /// Vanilla `setupRotations` sleeping `angle` from the resolved bed orientation
    /// (`sleepDirectionToRotation(BedBlock.getBedOrientation(...))`, degrees).
    /// `None` when the entity is sleeping but its sleeping position is not a bed, in
    /// which case the renderer falls back to the body yaw.
    #[serde(default)]
    pub sleeping_bed_yaw: Option<f32>,
    /// Vanilla `submit` bed head-offset translate `[-stepX * headOffset, -stepZ *
    /// headOffset]` in world units (`headOffset = eyeHeight(STANDING) - 0.1`). `[0,
    /// 0]` when the entity is not sleeping in a bed.
    #[serde(default)]
    pub sleeping_bed_offset: [f32; 2],
    /// Vanilla `LivingEntityRenderState.scale` (`LivingEntity.getScale`, the `SCALE`
    /// attribute): the uniform model scale applied before `setupRotations`. `1.0`
    /// for an entity at its default size (and every non-living entity).
    #[serde(default = "entity_model_source_default_scale")]
    pub scale: f32,
    #[serde(default)]
    pub sheep_eat_animation_tick: i32,
    #[serde(default)]
    pub polar_bear_stand_scale: f32,
    /// Stored block+sky light at the entity's light-probe block position,
    /// sampled like vanilla `EntityRenderer.getPackedLightCoords`. The renderer
    /// projection packs this (with the on-fire override) into the entity
    /// render-state light coords.
    #[serde(default = "entity_model_source_full_bright_light")]
    pub light: TerrainLight,
    /// Vanilla `LivingEntityRenderState.isInWater` (`Entity.isInWater()`): whether the
    /// entity's bounding box overlaps water, projected per frame from the world fluid state
    /// at the entity's interpolated AABB (the client does not run entity physics, but the
    /// overlap test is the vanilla `wasTouchingWater` algorithm). Drives the swim-pose
    /// branches of the fish renderers (`CodRenderer`/`SalmonRenderer`/`TropicalFishRenderer`
    /// out-of-water flop and tail thrash amplitude).
    #[serde(default)]
    pub in_water: bool,
    /// Vanilla `Entity.onGround()`: whether the entity's last synced movement landed it on the
    /// ground. Combined with [`in_water`](Self::in_water) to drive the vanilla
    /// `TurtleRenderer` `isOnLand = !isInWater && onGround` walk/swim leg branch. Defaults to
    /// `false` (the vanilla `Entity.onGround` default) until a movement packet sets it.
    #[serde(default)]
    pub on_ground: bool,
    /// Vanilla `DolphinRenderState.isMoving` (`Entity.getDeltaMovement().horizontalDistanceSqr() >
    /// 1.0e-7`): whether the entity is moving horizontally, projected from the synced
    /// `delta_movement`. Drives the `DolphinModel.setupAnim` swim body tilt / tail wave. `false`
    /// for a stationary entity (and any entity whose velocity has not been synced).
    #[serde(default)]
    pub is_moving: bool,
    /// Vanilla `LivingEntityRenderState.hasRedOverlay` (`hurtTime > 0 ||
    /// deathTime > 0`): drives the red damage overlay pass.
    #[serde(default)]
    pub has_red_overlay: bool,
    /// Vanilla `LivingEntityRenderState.deathTime` (`LivingEntity.deathTime`
    /// lerped): the death-animation counter that tips a dying living entity over
    /// in `LivingEntityRenderer.setupRotations`. `0.0` for every living entity
    /// that is alive (and every non-living entity).
    #[serde(default)]
    pub death_time: f32,
    /// Vanilla `CreeperRenderState.swelling` (`Creeper.getSwelling`): the lerped
    /// fuse progress that drives the renderer white swelling overlay. `0.0` for
    /// every non-creeper entity and for a creeper at rest.
    #[serde(default)]
    pub creeper_swelling: f32,
    /// Vanilla `ShulkerRenderState.peekAmount` (`Shulker.getClientPeekAmount(partialTick)`):
    /// the lerped client peek that drives `ShulkerModel.setupAnim`'s lid open/close
    /// (`lid.y = 16 + sin((0.5 + peek)·π)·8`, plus the `lid.yRot` twist above `0.3`). `0.0`
    /// (closed/bind pose) for every non-shulker entity and for a shut shulker.
    #[serde(default)]
    pub shulker_peek: f32,
    /// Vanilla `WardenRenderState.tendrilAnimation` (`Warden.getTendrilAnimation(partialTick)`):
    /// the lerped tendril pulse (`0..=1`) that drives `WardenModel.animateTendrils`. `0.0` for
    /// every non-warden entity and for a warden whose tendrils are at rest.
    #[serde(default)]
    pub tendril_animation: f32,
    /// Vanilla `WardenRenderState.heartAnimation` (`Warden.getHeartAnimation(partialTick)`): the
    /// lerped heartbeat pulse (`0..=1`) that drives the warden heart emissive overlay's alpha.
    /// `0.0` for every non-warden entity and between a warden's heartbeats.
    #[serde(default)]
    pub heart_animation: f32,
    /// Vanilla `Warden.roarAnimationState` elapsed seconds (`Pose.ROARING`-driven, the 4.2s
    /// `WARDEN_ROAR`), sampled by `WardenModel.setupAnim`'s `roarAnimation.apply`. `-1.0` (the
    /// stopped-animation sentinel) for a non-roaring warden and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub warden_roar_seconds: f32,
    /// Vanilla `Warden.sniffAnimationState` elapsed seconds (`Pose.SNIFFING`-driven, the 4.16s
    /// `WARDEN_SNIFF`), sampled by `WardenModel.setupAnim`'s `sniffAnimation.apply`. `-1.0`
    /// (stopped) for a non-sniffing warden and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub warden_sniff_seconds: f32,
    /// Vanilla `Warden.attackAnimationState` elapsed seconds (entity event `4`, the 0.33333s
    /// `WARDEN_ATTACK`), sampled by `WardenModel.setupAnim`'s `attackAnimation.apply`. `-1.0`
    /// (stopped) for a non-attacking warden and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub warden_attack_seconds: f32,
    /// Vanilla `Warden.sonicBoomAnimationState` elapsed seconds (entity event `62`, the 3.0s
    /// `WARDEN_SONIC_BOOM`), sampled by `WardenModel.setupAnim`'s `sonicBoomAnimation.apply`.
    /// `-1.0` (stopped) for a non-booming warden and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub warden_sonic_boom_seconds: f32,
    /// Vanilla `LivingEntityRenderState.walkAnimationPos`
    /// (`WalkAnimationState.position(partialTick)`): the lerped limb-swing position
    /// that sways the model's legs/arms. `0.0` for a standing entity, every
    /// non-living entity, and the entities whose `updateWalkAnimation` override is
    /// deferred.
    #[serde(default)]
    pub walk_animation_position: f32,
    /// Vanilla `LivingEntityRenderState.walkAnimationSpeed`
    /// (`WalkAnimationState.speed(partialTick)`): the lerped limb-swing speed
    /// amplitude that scales the leg/arm sway. `0.0` for a standing entity and
    /// every non-living entity.
    #[serde(default)]
    pub walk_animation_speed: f32,
    /// Vanilla `SquidRenderState.tentacleAngle` (`Squid.tentacleAngle` lerped):
    /// the tentacle flex angle `SquidModel.setupAnim` writes to every tentacle's
    /// `xRot`. `0.0` for a floating squid at rest and every non-squid entity.
    #[serde(default)]
    pub squid_tentacle_angle: f32,
    /// Vanilla `SquidRenderState.xBodyRot` (`Squid.xBodyRot` lerped, degrees): the
    /// squid swim pitch `SquidRenderer.setupRotations` applies to the root. `0.0`
    /// at rest and for every non-squid entity.
    #[serde(default)]
    pub squid_x_body_rot: f32,
    /// Vanilla `SquidRenderState.zBodyRot` (`Squid.zBodyRot` lerped, degrees): the
    /// squid swim roll `SquidRenderer.setupRotations` applies to the root. `0.0` at
    /// rest and for every non-squid entity.
    #[serde(default)]
    pub squid_z_body_rot: f32,
    /// Vanilla `GuardianRenderState.tailAnimation` (`Guardian.getTailAnimation`
    /// lerped): the tail-sway phase `GuardianModel.setupAnim` feeds to the three
    /// tail segments' `yRot` (`sin(swim) * π * {0.05, 0.1, 0.15}`). `0.0` for an
    /// unticked guardian and every non-guardian entity.
    #[serde(default)]
    pub guardian_tail_animation: f32,
    /// Vanilla `ChickenRenderState.flap` (`Chicken.flap` lerped): the wing-flap
    /// phase `ChickenModel.setupAnim` feeds to `(sin(flap) + 1) * flapSpeed`. `0.0`
    /// for an unticked/still chicken and every non-chicken entity.
    #[serde(default)]
    pub chicken_flap: f32,
    /// Vanilla `ChickenRenderState.flapSpeed` (`Chicken.flapSpeed` lerped): the
    /// wing-flap amplitude `ChickenModel.setupAnim` multiplies the flap phase by.
    /// `0.0` (wings held) at rest and for every non-chicken entity.
    #[serde(default)]
    pub chicken_flap_speed: f32,
    /// Vanilla `ParrotRenderState.flapAngle` (`ParrotRenderer.extractRenderState`:
    /// `(sin(lerp(flap)) + 1) * lerp(flapSpeed)`): the combined wing-flap angle
    /// `ParrotModel.setupAnim` feeds to the wing `zRot` (`±(0.0873 + flapAngle)`) and
    /// the body/head/tail bob (`flapAngle * 0.3`). `0.0` (wings held) for a
    /// grounded/still parrot and every non-parrot entity.
    #[serde(default)]
    pub parrot_flap_angle: f32,
    /// Vanilla `HumanoidArmorLayer` worn armor, the equipment-asset material per armor slot, resolved
    /// from the entity's `SetEquipment` items against the item registry. `None` leaves the slot bare.
    /// Only humanoid armor-wearers carry these; the renderer drapes the matching inflated armor piece.
    #[serde(default)]
    pub head_armor: Option<ArmorMaterialKind>,
    #[serde(default)]
    pub chest_armor: Option<ArmorMaterialKind>,
    #[serde(default)]
    pub legs_armor: Option<ArmorMaterialKind>,
    #[serde(default)]
    pub feet_armor: Option<ArmorMaterialKind>,
    /// Vanilla `DyedItemColor` per worn armor slot: the worn item's `dyed_color` component (a packed
    /// RGB), projected alongside the slot's material. Only leather is dyeable, so the renderer applies
    /// this as the leather layer's tint; `None` leaves leather at its default undyed brown and is
    /// ignored for every other material. Paired one-to-one with `head_armor` .. `feet_armor`.
    #[serde(default)]
    pub head_armor_dye: Option<i32>,
    #[serde(default)]
    pub chest_armor_dye: Option<i32>,
    #[serde(default)]
    pub legs_armor_dye: Option<i32>,
    #[serde(default)]
    pub feet_armor_dye: Option<i32>,
    /// Vanilla `GuardianRenderer` attack beam: present when a guardian has an active attack target.
    /// Mirrors the renderer's `GuardianBeamRenderState`; `None` for a guardian with no target and every
    /// other entity.
    #[serde(default)]
    pub guardian_beam: Option<GuardianBeamSource>,
    pub data_values: Vec<ProtocolEntityDataValue>,
}

/// Vanilla `GuardianRenderer` attack-beam projection (`GuardianRenderState.attackTargetPosition`
/// present). `eye_to_target` is the world-space vector from the guardian's eye to the target center
/// (`attackTargetPosition − eyePosition`); `eye_height` lifts the beam origin from the entity feet to
/// the eye; `attack_time` is `clientSideAttackTime + partialTicks`; `attack_scale` is
/// `getAttackAnimationScale`. The native layer maps this 1:1 onto the renderer's `GuardianBeamRenderState`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GuardianBeamSource {
    pub eye_to_target: [f32; 3],
    pub eye_height: f32,
    pub attack_time: f32,
    pub attack_scale: f32,
}

/// A humanoid armor equipment-asset material (vanilla `ArmorMaterials.<MAT>` → `EquipmentAssets.<MAT>`),
/// resolved from a worn armor item's registry id. Mirrors the renderer's `EntityArmorMaterial`; the
/// native projection maps between the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArmorMaterialKind {
    Leather,
    Copper,
    Chainmail,
    Iron,
    Gold,
    Diamond,
    TurtleScute,
    Netherite,
}

impl ArmorMaterialKind {
    /// Parses the vanilla equipment-asset name (`bbb_pack` `ItemRegistryCatalog::humanoid_armor_asset`,
    /// the lowercased `ArmorMaterials.<MAT>` name) into a material kind.
    pub fn from_equipment_asset(asset: &str) -> Option<Self> {
        Some(match asset {
            "leather" => Self::Leather,
            "copper" => Self::Copper,
            "chainmail" => Self::Chainmail,
            "iron" => Self::Iron,
            "gold" => Self::Gold,
            "diamond" => Self::Diamond,
            "turtle_scute" => Self::TurtleScute,
            "netherite" => Self::Netherite,
            _ => return None,
        })
    }
}

/// Vanilla `EntityRenderer` light-probe full-bright fallback
/// (`LightCoordsUtil.FULL_BRIGHT` = block 15, sky 15), used when an entity's
/// chunk light is unavailable so it renders bright rather than dark, matching
/// the `EntityRenderState.lightCoords` default.
pub(crate) const ENTITY_LIGHT_PROBE_FULL_BRIGHT: TerrainLight = TerrainLight { sky: 15, block: 15 };

fn entity_model_source_full_bright_light() -> TerrainLight {
    ENTITY_LIGHT_PROBE_FULL_BRIGHT
}

/// Vanilla default `LivingEntity.getScale` (the unmodified `SCALE` attribute) used
/// when an `EntityModelSourceState` is deserialized without a recorded scale.
fn entity_model_source_default_scale() -> f32 {
    1.0
}

fn entity_model_source_default_true() -> bool {
    true
}

/// The stopped-animation sentinel for the frog croak (`-1.0`, meaning "not croaking") used when an
/// `EntityModelSourceState` is deserialized without a recorded croak time.
fn entity_model_source_default_neg_one() -> f32 {
    -1.0
}

/// The "no triggered animation" sentinel (`-1`) for the sniffer animation id, used when an
/// `EntityModelSourceState` is deserialized without a recorded sniffer state.
fn entity_model_source_default_neg_one_i32() -> i32 {
    -1
}

impl EntityTransformState {
    pub(crate) fn from_components(identity: &EntityIdentity, transform: EntityTransform) -> Self {
        Self {
            id: identity.id,
            uuid: identity.uuid,
            entity_type_id: identity.entity_type_id,
            data: identity.data,
            position: transform.position,
            position_base: transform.position_base,
            delta_movement: transform.delta_movement,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            y_head_rot: transform.y_head_rot,
            on_ground: transform.on_ground,
        }
    }
}

impl WorldStore {
    pub fn apply_add_entity(&mut self, packet: ProtocolAddEntity) {
        self.counters.entities_received += 1;
        let packet_position = entity_vec3(packet.position);
        let entity = EntityState {
            id: packet.id,
            uuid: packet.uuid,
            entity_type_id: packet.entity_type_id,
            data: packet.data,
            position: vanilla_client_position_for_entity_data(
                packet.entity_type_id,
                packet_position,
                packet.data,
                &[],
            )
            .unwrap_or(packet_position),
            position_base: packet_position,
            delta_movement: entity_vec3(packet.delta_movement),
            y_rot: packet.y_rot,
            x_rot: packet.x_rot,
            y_head_rot: packet.y_head_rot,
            on_ground: None,
            data_values: Vec::new(),
            equipment: Vec::new(),
            attributes: Vec::new(),
            vehicle_id: None,
            passengers: Vec::new(),
            leash_holder_id: None,
            last_animation_action: None,
            last_event_id: None,
            last_hurt_yaw: None,
            mob_effects: BTreeMap::new(),
            client_animations: EntityClientAnimationState::default(),
            last_damage: None,
            minecart_lerp_steps: Vec::new(),
            hurting_projectile: initial_hurting_projectile_state(packet.entity_type_id),
        };

        self.entities.insert_or_replace(entity);
        self.update_entity_count();
        self.update_active_mob_effect_count();
    }

    pub fn apply_take_item_entity(&mut self, packet: ProtocolTakeItemEntity) -> bool {
        self.counters.take_item_entities_received += 1;
        let Some(entity_type_id) = self.entities.entity_type_id(packet.item_id) else {
            self.counters.take_item_entities_ignored += 1;
            return false;
        };

        self.counters.take_item_entities_applied += 1;
        if entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID {
            return true;
        }

        if entity_type_id == VANILLA_ENTITY_TYPE_ITEM_ID {
            let mut stack_shrank = false;
            let keep_entity = self
                .entities
                .with_metadata_mut(packet.item_id, |metadata| {
                    if let Some(stack) = item_entity_stack_mut(&mut metadata.data_values) {
                        if stack.count > 0 && packet.amount > 0 {
                            stack.count = stack.count.saturating_sub(packet.amount).max(0);
                            stack_shrank = true;
                        }
                        return stack.count > 0;
                    }
                    false
                })
                .unwrap_or(false);
            if stack_shrank {
                self.counters.item_entity_stack_shrinks += 1;
            }
            if keep_entity {
                return true;
            }
        }

        let removed = self.remove_entities_by_ids(&[packet.item_id]);
        self.counters.take_item_entities_removed += removed;
        true
    }

    pub fn apply_remove_entities(&mut self, packet: ProtocolRemoveEntities) -> usize {
        let received = packet.entity_ids.len();
        self.counters.entity_removes_received += received;
        let removed = self.remove_entities_by_ids(&packet.entity_ids);
        self.counters.entity_removes_ignored += received.saturating_sub(removed);
        removed
    }

    fn remove_entities_by_ids(&mut self, removed_ids: &[i32]) -> usize {
        let removed = self.entities.remove_ids(removed_ids);
        if self
            .local_player_vehicle_id
            .is_some_and(|vehicle_id| removed_ids.contains(&vehicle_id))
        {
            self.local_player_vehicle_id = None;
        }
        self.entities.for_each_mount_mut(|_, mount| {
            if mount
                .vehicle_id
                .is_some_and(|vehicle_id| removed_ids.contains(&vehicle_id))
            {
                mount.vehicle_id = None;
            }
            mount
                .passengers
                .retain(|passenger_id| !removed_ids.contains(passenger_id));
        });
        self.entities.for_each_leash_mut(|_, leash| {
            if leash
                .holder_id
                .is_some_and(|holder_id| removed_ids.contains(&holder_id))
            {
                leash.holder_id = None;
            }
        });
        self.counters.entities_removed += removed;
        self.update_entity_count();
        self.update_active_mob_effect_count();
        removed
    }

    pub fn probe_entity(&self, id: i32) -> Option<EntityState> {
        self.entities.get(id)
    }

    pub fn probe_entity_status(&self, id: i32) -> Option<EntityStatusProbeState> {
        let entity = self.probe_entity(id)?;
        Some(EntityStatusProbeState {
            id: entity.id,
            entity_type_id: entity.entity_type_id,
            last_animation_action: entity.last_animation_action,
            last_event_id: entity.last_event_id,
            last_hurt_yaw: entity.last_hurt_yaw,
            mob_effects: entity.mob_effects,
            last_damage: entity.last_damage,
        })
    }

    pub fn probe_entity_transform(&self, id: i32) -> Option<EntityTransformState> {
        self.entities.transform_state(id)
    }

    pub fn probe_entity_camera_pose(&self, id: i32) -> Option<EntityCameraPoseState> {
        self.entities.camera_pose_state(id)
    }

    pub fn probe_entity_pick_bounds(&self, id: i32) -> Option<EntityPickBoundsState> {
        let identity = self.entities.identity(id)?;
        if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            && self
                .player_info_entry(identity.uuid)
                .is_some_and(|info| info.is_spectator())
        {
            return None;
        }
        self.entities.pick_bounds(id)
    }

    pub fn entity_pick_targets(&self) -> Vec<EntityPickTargetState> {
        self.entity_pick_targets_at_partial_tick(1.0)
    }

    pub fn entity_pick_targets_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<EntityPickTargetState> {
        self.entities
            .pick_targets_at_partial_tick(partial_ticks)
            .into_iter()
            .filter(|target| {
                let Some(identity) = self.entities.identity(target.entity_id) else {
                    return true;
                };
                identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
                    || !self
                        .player_info_entry(identity.uuid)
                        .is_some_and(|info| info.is_spectator())
            })
            .collect()
    }

    pub fn entity_model_sources_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<EntityModelSourceState> {
        self.entity_pick_targets_at_partial_tick(partial_ticks)
            .into_iter()
            .filter_map(|target| {
                let mut source = self.entities.model_source(
                    target.entity_id,
                    target.position,
                    partial_ticks,
                    &self.default_item_armor_materials,
                )?;
                source.light = self
                    .sample_block_light(entity_light_block_pos(target.position))
                    .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT);
                // Vanilla `LivingEntityRenderState.isInWater = entity.isInWater()`: project
                // the `wasTouchingWater` overlap from the entity's interpolated world AABB
                // (`position + EntityDimensions`) against the chunk fluid state.
                let aabb_min = [
                    target.position.x + f64::from(target.bounds.min[0]),
                    target.position.y + f64::from(target.bounds.min[1]),
                    target.position.z + f64::from(target.bounds.min[2]),
                ];
                let aabb_max = [
                    target.position.x + f64::from(target.bounds.max[0]),
                    target.position.y + f64::from(target.bounds.max[1]),
                    target.position.z + f64::from(target.bounds.max[2]),
                ];
                source.in_water = crate::fluid::world_aabb_in_water(self, aabb_min, aabb_max);
                if source.is_sleeping {
                    if let Some((yaw, offset)) = self.resolve_sleeping_bed(target.entity_id) {
                        source.sleeping_bed_yaw = Some(yaw);
                        source.sleeping_bed_offset = offset;
                    }
                }
                if !source.is_upside_down && self.resolve_player_upside_down(target.entity_id) {
                    source.is_upside_down = true;
                }
                Some(source)
            })
            .collect()
    }

    /// Resolves the vanilla `AvatarRenderer.isEntityUpsideDown` player path: a player
    /// whose `CAPE` model part is shown and whose GameProfile name (from the
    /// player-info list) is `Dinnerbone`/`Grumm`. The non-player living path is
    /// handled in `EntityStore::model_source` from the custom name instead.
    fn resolve_player_upside_down(&self, entity_id: i32) -> bool {
        let Some((profile_id, cape_shown)) = self.entities.avatar_upside_down_inputs(entity_id)
        else {
            return false;
        };
        cape_shown
            && self.player_info_entry(profile_id).is_some_and(|entry| {
                VANILLA_UPSIDE_DOWN_NAMES.contains(&entry.profile.name.as_str())
            })
    }

    /// Resolves the vanilla `LivingEntityRenderer` sleeping bed orientation and head
    /// offset for an entity sleeping in a bed: `BedBlock.getBedOrientation` looks up
    /// the bed block at the synced sleeping position and reads its `FACING`. Returns
    /// the `sleepDirectionToRotation` yaw (degrees) and the world-space bed offset,
    /// or `None` when the sleeping position is not a bed (the renderer then falls
    /// back to the body yaw with no offset).
    fn resolve_sleeping_bed(&self, entity_id: i32) -> Option<(f32, [f32; 2])> {
        let bed_pos = self.entities.sleeping_pos(entity_id)?;
        let probe = self.probe_block(bed_pos)?;
        let block_name = probe.block_name.as_deref()?;
        let standing_eye_height = self.entities.standing_eye_height(entity_id)?;
        sleeping_bed_yaw_and_offset(block_name, &probe.block_properties, standing_eye_height)
    }

    pub fn entity_transforms(&self) -> Vec<EntityTransformState> {
        self.entities.transform_states()
    }

    pub fn item_entity_stacks(&self) -> Vec<ItemEntityStackState> {
        self.entities.item_entity_stacks()
    }

    /// Collects the `DATA_ITEM_STACK` carried by every entity whose type id is in `type_ids`. The
    /// thrown-item projectiles (snowball, egg, ender pearl, potions, …) that vanilla's
    /// `ThrownItemRenderer` draws as an item sprite share the dropped item's data layout, so this feeds
    /// the same billboard path. The caller (which owns the vanilla type ids) passes the projectile set.
    pub fn item_stacks_for_entity_types(&self, type_ids: &[i32]) -> Vec<ItemEntityStackState> {
        self.entities.item_stacks_for_entity_types(type_ids)
    }

    pub fn local_player_id(&self) -> Option<i32> {
        self.local_player_id
    }

    pub fn local_player_vehicle_id(&self) -> Option<i32> {
        self.local_player_vehicle_id
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn hurting_projectile(&self, id: i32) -> Option<HurtingProjectileState> {
        self.entities
            .hurting_projectile(id)
            .map(HurtingProjectileState::from)
    }

    pub fn last_projectile_power_update(&self) -> Option<&ProjectilePowerUpdateState> {
        self.last_projectile_power.as_ref()
    }

    pub(crate) fn update_entity_count(&mut self) {
        self.counters.entities_tracked = self.entities.len();
    }
}

/// Vanilla `BedBlock.getBedOrientation` + `LivingEntityRenderer.sleepDirectionToRotation`
/// + the `submit` bed head offset: given the bed block at the entity's sleeping
/// position and the entity's standing eye height, returns the sleeping yaw (degrees)
/// and the world-space head-offset translate `[-stepX * headOffset, -stepZ *
/// headOffset]` (`headOffset = eyeHeight - 0.1`). `None` when the block is not a bed.
fn sleeping_bed_yaw_and_offset(
    block_name: &str,
    properties: &BTreeMap<String, String>,
    standing_eye_height: f32,
) -> Option<(f32, [f32; 2])> {
    if !is_bed_block_name(block_name) {
        return None;
    }
    let (yaw, step_x, step_z) = sleep_direction_rotation_and_step(properties.get("facing")?)?;
    let head_offset = standing_eye_height - 0.1;
    Some((yaw, [-step_x * head_offset, -step_z * head_offset]))
}

/// Vanilla bed blocks are the `<color>_bed` family that `BedBlock.getBedOrientation`
/// matches with `instanceof BedBlock`.
fn is_bed_block_name(block_name: &str) -> bool {
    block_name
        .rsplit(':')
        .next()
        .is_some_and(|path| path.ends_with("_bed"))
}

/// Vanilla `LivingEntityRenderer.sleepDirectionToRotation` (the sleeping yaw, in
/// degrees) paired with `Direction.getStepX`/`getStepZ` for the horizontal bed
/// `FACING`. `None` for a non-horizontal facing value.
fn sleep_direction_rotation_and_step(facing: &str) -> Option<(f32, f32, f32)> {
    match facing {
        "south" => Some((90.0, 0.0, 1.0)),
        "west" => Some((0.0, -1.0, 0.0)),
        "north" => Some((270.0, 0.0, -1.0)),
        "east" => Some((180.0, 1.0, 0.0)),
        _ => None,
    }
}

/// Vanilla `BlockPos.containing(entity.getLightProbePosition(partialTick))`:
/// the light-probe position defaults to the entity's interpolated feet
/// position, floored per axis.
fn entity_light_block_pos(position: EntityVec3) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

fn item_entity_stack_mut(
    data_values: &mut [ProtocolEntityDataValue],
) -> Option<&mut ProtocolItemStackSummary> {
    data_values.iter_mut().find_map(|value| {
        if value.data_id == VANILLA_ITEM_ENTITY_STACK_DATA_ID {
            if let EntityDataValueKind::ItemStack(stack) = &mut value.value {
                return Some(stack);
            }
        }
        None
    })
}

#[cfg(test)]
mod tests;
