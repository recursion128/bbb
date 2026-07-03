use std::{
    collections::{BTreeMap, HashMap},
    fmt,
};

use bbb_protocol::packets::AttributeSnapshot as ProtocolAttributeSnapshot;
use hecs::{Entity, World};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use bbb_protocol::packets::EntityDataValueKind;
use bbb_protocol::packets::EquipmentSlot as ProtocolEquipmentSlot;
use bbb_protocol::packets::ItemStackSummary;

use super::{
    is_vanilla_abstract_nautilus_type, is_vanilla_can_wear_horse_armor_type, is_vanilla_llama_type,
    is_vanilla_minecart_type, ArmorMaterialKind, EntityAttachmentFace, EntityAttributes,
    EntityBlockModelState, EntityCameraPoseState, EntityClientAnimations, EntityDamage,
    EntityEquipment, EntityHurtingProjectile, EntityIdentity, EntityLeash, EntityMetadata,
    EntityMinecartLerp, EntityMobEffects, EntityModelSourceState, EntityMount, EntityState,
    EntityTransform, EntityTransformState, EntityTransientEvents, FallingBlockModelState,
    FireworkRocketItemState, ItemEntityStackState, ItemFrameRenderState, LlamaBodyDecorColor,
    LocalPlayerAttackSwingState, MinecartDisplayBlockState, OminousItemSpawnerItemState,
    RavagerRoarParticleState, WolfArmorCrackiness, VANILLA_ENTITY_NO_GRAVITY_DATA_ID,
    VANILLA_ENTITY_SILENT_DATA_ID, VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
    VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID,
    VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
    VANILLA_ENTITY_TYPE_DONKEY_ID, VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
    VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID, VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
    VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID, VANILLA_ENTITY_TYPE_HORSE_ID,
    VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_MINECART_ID, VANILLA_ENTITY_TYPE_MULE_ID,
    VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID, VANILLA_ENTITY_TYPE_PANDA_ID,
    VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_SHULKER_ID,
    VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
    VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID, VANILLA_ENTITY_TYPE_SQUID_ID,
    VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_TNT_ID,
    VANILLA_ENTITY_TYPE_TNT_MINECART_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
    VANILLA_ITEM_ENTITY_STACK_DATA_ID, VANILLA_UPSIDE_DOWN_NAMES,
};
use crate::entities::animations::{
    allay_is_dancing, axolotl_is_playing_dead, boat_bubble_time, boat_paddle_states,
    camel_is_dashing, cat_is_lying, cat_relax_state_one, copper_golem_is_dropping_item,
    copper_golem_is_dropping_no_item, copper_golem_is_getting_item,
    copper_golem_is_getting_no_item, copper_golem_is_idle, creaking_can_move,
    creaking_is_tearing_down, entity_animation_uses_in_water, entity_is_fall_flying,
    guardian_attack_duration, guardian_attack_target_id, guardian_is_moving,
    is_guardian_entity_type, piglin_is_charging_crossbow, pillager_is_charging_crossbow,
    player_is_using_item, warden_heartbeat_delay, wither_side_head_target_ids,
    wither_side_head_target_rotation, wolf_is_interested, WitherHeadTargetRotations,
};
use crate::entities::dimensions::{
    entity_data_pose, item_frame_facing, item_frame_item, item_frame_map_id, item_frame_rotation,
    vanilla_body_anchor_y_offset_for_entity_data, vanilla_client_position_for_entity_data,
    vanilla_eye_height_for_entity_data, vanilla_feline_family,
    vanilla_illager_aggressive_arm_pose_family, vanilla_is_baby, vanilla_is_bat, vanilla_is_bee,
    vanilla_is_cat, vanilla_is_enderman, vanilla_is_fox, vanilla_is_vex, vanilla_is_wither,
    vanilla_living_entity_type, vanilla_model_source_bounds_for_entity_data,
    vanilla_pick_bounds_for_entity_data, vanilla_piglin_melee_attack_family, vanilla_render_scale,
    vanilla_zombie_model_family, ENTITY_DATA_POSE_ID, ITEM_FRAME_ENTITY_TYPE_IDS,
    VANILLA_POSE_CROUCHING_ID, VANILLA_POSE_SLEEPING_ID,
};
use crate::entities::dragon::ender_dragon_part_pick_targets_at_partial_tick;
use crate::entities::projectiles::entity_hurting_projectile_from_state;
use crate::registries::RegistrySet;
use crate::ItemEquipmentSlot;
use bbb_protocol::entity_types::*;

/// Vanilla `AbstractVillager.DATA_UNHAPPY_COUNTER` (INT): `AgeableMob` consumes
/// baby id 16 and age-locked id 17, so the abstract-villager counter is id 18.
const ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID: u8 = 18;

/// Vanilla `Entity.getTicksRequiredToFreeze()`: the powder-snow freeze threshold
/// at which `isFullyFrozen()` becomes true and the body starts shaking.
const VANILLA_TICKS_REQUIRED_TO_FREEZE: i32 = 140;
/// Vanilla `Entity.DATA_SHARED_FLAGS_ID` id 0 bit 5 (`Entity.isInvisible()`).
const ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
/// Vanilla `Entity.DATA_SHARED_FLAGS_ID` id 0 bit 3 (`Entity.isSprinting()`).
const ENTITY_SHARED_FLAG_SPRINTING: i8 = 1 << 3;
const ENTITY_SHARED_FLAG_INVISIBLE: i8 = 1 << 5;
/// Vanilla `Entity.DATA_SHARED_FLAGS_ID` id 0 bit 6 (`Entity.isCurrentlyGlowing()` on the client).
const ENTITY_SHARED_FLAG_GLOWING: i8 = 1 << 6;
/// Vanilla `TamableAnimal.DATA_FLAGS_ID`: Cat extends TamableAnimal, while the ocelot does not.
const TAMABLE_ANIMAL_FLAGS_DATA_ID: u8 = 18;
/// Vanilla `TamableAnimal.isInSittingPose()` reads `DATA_FLAGS_ID & 1`.
const TAMABLE_ANIMAL_SITTING_FLAG: i8 = 0x01;

/// Vanilla 26.1 `EntityType.PIG` registry id, used to gate `PigRenderState.saddle`.
/// Vanilla 26.1 `EntityType.WOLF` registry id, used to gate `WolfArmorLayer`.
/// Vanilla 26.1 skeleton-family ids whose renderers extend `HumanoidMobRenderer`
/// and therefore feed `ItemInHandLayer` through `getItemHeldByArm`.
/// Vanilla 26.1 `EvokerRenderer` installs an `ItemInHandLayer` over the illager model.
/// Vanilla 26.1 `ZombifiedPiglinRenderer` extends `HumanoidMobRenderer`.
/// Vanilla `Pose.STANDING` ordinal, used by camel walk-animation gating.
const VANILLA_POSE_STANDING_ID: i32 = 0;
/// Vanilla `Pose.SWIMMING` ordinal, used by player cape bob suppression.
const VANILLA_POSE_SWIMMING_ID: i32 = 3;
/// Vanilla `Shulker.DATA_ATTACH_FACE_ID` (Direction), declared before peek and color metadata.
const SHULKER_ATTACH_FACE_DATA_ID: u8 = 16;
/// Vanilla `AbstractMinecart.DATA_ID_CUSTOM_DISPLAY_BLOCK` optional block-state metadata.
const MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID: u8 = 11;
/// Vanilla `AbstractMinecart.DATA_ID_DISPLAY_OFFSET` int metadata.
const MINECART_DISPLAY_OFFSET_DATA_ID: u8 = 12;
/// Vanilla `MinecartFurnace.DATA_ID_FUEL` boolean metadata, declared after abstract minecart fields.
const FURNACE_MINECART_FUEL_DATA_ID: u8 = 13;
const DEFAULT_MINECART_DISPLAY_OFFSET: i32 = 6;
/// Vanilla `PrimedTnt.DATA_FUSE_ID` int metadata: `PrimedTnt` directly extends
/// `Entity`, whose base accessors occupy ids `0..=7`.
const PRIMED_TNT_FUSE_DATA_ID: u8 = 8;
/// Vanilla `PrimedTnt.DATA_BLOCK_STATE_ID` block-state metadata, declared after fuse.
const PRIMED_TNT_BLOCK_STATE_DATA_ID: u8 = 9;
const DEFAULT_PRIMED_TNT_FUSE: i32 = 80;
/// Vanilla `FireworkRocketEntity.DATA_ATTACHED_TO_TARGET` optional unsigned int, after the firework
/// item stack at id 8. Present rockets are attached to an elytra user and `shouldRender` returns false.
const FIREWORK_ROCKET_ATTACHED_TO_TARGET_DATA_ID: u8 = 9;
/// Vanilla `FireworkRocketEntity.DATA_SHOT_AT_ANGLE` boolean, declared after attached-target.
const FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID: u8 = 10;

fn wolf_armor_crackiness(
    item: &ItemStackSummary,
    default_item_max_damage: &BTreeMap<i32, i32>,
) -> WolfArmorCrackiness {
    if item.component_patch.unbreakable {
        return WolfArmorCrackiness::None;
    }
    let Some(item_id) = item.item_id else {
        return WolfArmorCrackiness::None;
    };
    let Some(max_damage) = item
        .component_patch
        .max_damage
        .or_else(|| default_item_max_damage.get(&item_id).copied())
        .filter(|max_damage| *max_damage > 0)
    else {
        return WolfArmorCrackiness::None;
    };
    let damage = item
        .component_patch
        .damage
        .unwrap_or(0)
        .clamp(0, max_damage);
    let fraction = (max_damage - damage) as f32 / max_damage as f32;
    if fraction < 0.32 {
        WolfArmorCrackiness::High
    } else if fraction < 0.69 {
        WolfArmorCrackiness::Medium
    } else if fraction < 0.95 {
        WolfArmorCrackiness::Low
    } else {
        WolfArmorCrackiness::None
    }
}

fn end_crystal_renderer_y(time_in_ticks: f32) -> f32 {
    let hh = (time_in_ticks * 0.2).sin() / 2.0 + 0.5;
    (hh * hh + hh) * 0.4 - 1.4
}

fn end_crystal_intersects_ender_dragon_search_box(
    dragon_position: super::EntityVec3,
    crystal_position: super::EntityVec3,
) -> bool {
    // Vanilla 26.1 `EntityType.ENDER_DRAGON.sized(16, 8)` and
    // `EntityType.END_CRYSTAL.sized(2, 2)`, with `EnderDragon.checkCrystals` searching
    // `getBoundingBox().inflate(32)`.
    const DRAGON_HALF_WIDTH: f64 = 8.0;
    const DRAGON_HEIGHT: f64 = 8.0;
    const CRYSTAL_HALF_WIDTH: f64 = 1.0;
    const CRYSTAL_HEIGHT: f64 = 2.0;
    const SEARCH_INFLATE: f64 = 32.0;

    let dragon_min = [
        dragon_position.x - DRAGON_HALF_WIDTH - SEARCH_INFLATE,
        dragon_position.y - SEARCH_INFLATE,
        dragon_position.z - DRAGON_HALF_WIDTH - SEARCH_INFLATE,
    ];
    let dragon_max = [
        dragon_position.x + DRAGON_HALF_WIDTH + SEARCH_INFLATE,
        dragon_position.y + DRAGON_HEIGHT + SEARCH_INFLATE,
        dragon_position.z + DRAGON_HALF_WIDTH + SEARCH_INFLATE,
    ];
    let crystal_min = [
        crystal_position.x - CRYSTAL_HALF_WIDTH,
        crystal_position.y,
        crystal_position.z - CRYSTAL_HALF_WIDTH,
    ];
    let crystal_max = [
        crystal_position.x + CRYSTAL_HALF_WIDTH,
        crystal_position.y + CRYSTAL_HEIGHT,
        crystal_position.z + CRYSTAL_HALF_WIDTH,
    ];

    crystal_max[0] >= dragon_min[0]
        && crystal_min[0] <= dragon_max[0]
        && crystal_max[1] >= dragon_min[1]
        && crystal_min[1] <= dragon_max[1]
        && crystal_max[2] >= dragon_min[2]
        && crystal_min[2] <= dragon_max[2]
}

fn vanilla_equine_saddle_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_HORSE_ID
            | VANILLA_ENTITY_TYPE_DONKEY_ID
            | VANILLA_ENTITY_TYPE_MULE_ID
            | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
    )
}

fn vanilla_camel_saddle_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_CAMEL_ID | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
    )
}

fn vanilla_mob_item_in_hand_layer_type(entity_type_id: i32) -> bool {
    vanilla_zombie_model_family(entity_type_id)
        || vanilla_piglin_melee_attack_family(entity_type_id)
        || vanilla_illager_aggressive_arm_pose_family(entity_type_id)
        || vanilla_is_vex(entity_type_id)
        || matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_BOGGED_ID
                | VANILLA_ENTITY_TYPE_EVOKER_ID
                | VANILLA_ENTITY_TYPE_PARCHED_ID
                | VANILLA_ENTITY_TYPE_SKELETON_ID
                | VANILLA_ENTITY_TYPE_STRAY_ID
                | VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID
                | VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
        )
}

/// Vanilla `LivingEntity.DATA_LIVING_ENTITY_FLAGS` data id (8): the byte holding
/// the using-item / off-hand / spin-attack flags.
const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;

/// Vanilla `LivingEntity.LIVING_ENTITY_FLAG_SPIN_ATTACK` (4): the
/// `DATA_LIVING_ENTITY_FLAGS` bit set while a riptide trident spin is active
/// (`LivingEntity.isAutoSpinAttack`).
const LIVING_ENTITY_FLAG_SPIN_ATTACK: i8 = 4;

/// Vanilla `LivingEntity.LIVING_ENTITY_FLAG_IS_USING` (1): the `DATA_LIVING_ENTITY_FLAGS` bit set while
/// the entity is actively using an item (`LivingEntity.isUsingItem`).
const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;

/// Vanilla `LivingEntity.LIVING_ENTITY_FLAG_OFF_HAND` (2): the `DATA_LIVING_ENTITY_FLAGS` bit selecting
/// the off hand as the one using the item (`LivingEntity.getUsedItemHand`).
const LIVING_ENTITY_FLAG_OFF_HAND: i8 = 2;

/// Vanilla `Mob.DATA_MOB_FLAGS_ID` data id (15): the byte holding the no-AI /
/// left-handed / aggressive flags.
const VANILLA_MOB_FLAGS_DATA_ID: u8 = 15;

/// Vanilla `Mob.MOB_FLAG_AGGRESSIVE` (4): the `DATA_MOB_FLAGS_ID` bit set while a mob is
/// in its aggressive AI state (`Mob.isAggressive`).
const MOB_FLAG_AGGRESSIVE: i8 = 4;
/// Vanilla `Mob.MOB_FLAG_LEFTHANDED` (2): flips `Mob.getMainArm()` from RIGHT to LEFT.
const MOB_FLAG_LEFTHANDED: i8 = 2;
/// Vanilla `Avatar.DATA_PLAYER_MAIN_HAND` data id (15): `HumanoidArm.LEFT` (0) or RIGHT (1).
const VANILLA_AVATAR_MAIN_HAND_DATA_ID: u8 = 15;
const VANILLA_HUMANOID_ARM_LEFT_ID: i32 = 0;
const VANILLA_HUMANOID_ARM_RIGHT_ID: i32 = 1;

/// Vanilla `SnowGolem.DATA_PUMPKIN_ID` data id (16): the byte holding the carved-pumpkin-head flag.
const VANILLA_SNOW_GOLEM_PUMPKIN_DATA_ID: u8 = 16;

/// Vanilla `SnowGolem.PUMPKIN_FLAG` (16). `defineSynchedData` initializes the byte to this value.
const SNOW_GOLEM_PUMPKIN_FLAG: i8 = 16;

/// Vanilla `Enderman.DATA_CARRY_STATE` data id (16): the optional carried `BlockState`.
/// The enderman is the first `Monster` accessor after `Mob.DATA_MOB_FLAGS_ID` (15).
const VANILLA_ENDERMAN_CARRY_STATE_DATA_ID: u8 = 16;

/// Vanilla `Enderman.DATA_CREEPY` data id (17): the boolean staring/aggressive flag,
/// defined right after `DATA_CARRY_STATE` (16).
const VANILLA_ENDERMAN_CREEPY_DATA_ID: u8 = 17;

/// Vanilla `Bat.DATA_ID_FLAGS` data id (16): the byte holding the bat flags. The bat is the
/// first `AmbientCreature` accessor after `Mob.DATA_MOB_FLAGS_ID` (15).
const VANILLA_BAT_FLAGS_DATA_ID: u8 = 16;

/// Vanilla `Bat.FLAG_RESTING` (1): the `DATA_ID_FLAGS` bit set while the bat hangs at rest
/// (`Bat.isResting`).
const BAT_FLAG_RESTING: i8 = 1;

/// Vanilla `Bee.DATA_FLAGS_ID` data id (18): the byte holding the bee flags, the first `Bee`
/// accessor after the two `AgeableMob` accessors `DATA_BABY_ID` (16) / `AGE_LOCKED` (17).
const VANILLA_BEE_FLAGS_DATA_ID: u8 = 18;

/// Vanilla `Bee.FLAG_HAS_STUNG` (4): the `DATA_FLAGS_ID` bit set once the bee has stung and
/// lost its stinger (`Bee.hasStung`).
const BEE_FLAG_HAS_STUNG: i8 = 4;

/// Vanilla `Vex.DATA_FLAGS_ID` data id (16): the byte holding the vex flags, the first own
/// `Vex` accessor after `Mob.DATA_MOB_FLAGS_ID` (15).
const VANILLA_VEX_FLAGS_DATA_ID: u8 = 16;

/// Vanilla `Fox.DATA_FLAGS_ID` data id (19): the byte holding the fox flags, defined after the
/// `AgeableMob` accessors (`16..=17`) and the fox's own `DATA_TYPE_ID` variant int (18).
const VANILLA_FOX_FLAGS_DATA_ID: u8 = 19;

/// Vanilla `Fox` flag masks within `DATA_FLAGS_ID`: `FLAG_SITTING=1`, `FLAG_CROUCHING=4`,
/// `FLAG_POUNCING=16`, `FLAG_SLEEPING=32`, `FLAG_FACEPLANTED=64` (`getFlag(mask) = (byte & mask) != 0`).
/// `FLAG_INTERESTED=8` drives only the head-tilt accumulator and so is read by the animation layer, not
/// projected as a render-state bool.
const FOX_FLAG_SITTING: i8 = 1;
const FOX_FLAG_CROUCHING: i8 = 4;
const FOX_FLAG_POUNCING: i8 = 16;
const FOX_FLAG_SLEEPING: i8 = 32;
const FOX_FLAG_FACEPLANTED: i8 = 64;

/// Vanilla `Vex.FLAG_IS_CHARGING` (1): the `DATA_FLAGS_ID` bit set while the vex charges an
/// attack (`Vex.isCharging`).
const VEX_FLAG_IS_CHARGING: i8 = 1;

/// Vanilla `WitherBoss.DATA_ID_INV` data id (19): the int spawn-invulnerability countdown
/// (`getInvulnerableTicks`), the fourth `WitherBoss` accessor after `Mob.DATA_MOB_FLAGS_ID` (15) and
/// the three `DATA_TARGET_A/B/C` ints (16..=18).
const VANILLA_WITHER_INV_DATA_ID: u8 = 19;

/// Vanilla `Entity.DATA_CUSTOM_NAME` data id (2): the optional custom name
/// component (the name-tag text), used by the Dinnerbone/Grumm upside-down check.
const VANILLA_ENTITY_CUSTOM_NAME_DATA_ID: u8 = 2;

/// Vanilla `LivingEntity.SLEEPING_POS_ID` data id (14): the optional bed position
/// the entity is sleeping in (`getSleepingPos`).
const VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID: u8 = 14;

/// Vanilla `EndCrystal.DATA_BEAM_TARGET` data id (8): optional block target for the dragon-healing
/// beam. `DATA_SHOW_BOTTOM` follows at id 9.
const END_CRYSTAL_BEAM_TARGET_DATA_ID: u8 = 8;

/// Vanilla `Avatar.DATA_PLAYER_MODE_CUSTOMISATION` data id (16): the byte of shown
/// player model parts, read by the player upside-down check for the cape part.
const VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID: u8 = 16;
/// Vanilla `Player.DATA_SHOULDER_PARROT_LEFT` data id (19): an `OPTIONAL_UNSIGNED_INT`
/// storing the left-shoulder `Parrot.Variant` id. The wire value is already decoded by
/// `bbb-protocol` as `None` for zero or `Some(id)` for value minus one.
const VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID: u8 = 19;
/// Vanilla `Player.DATA_SHOULDER_PARROT_RIGHT` data id (20): the mirrored right-shoulder
/// `OPTIONAL_UNSIGNED_INT` variant id.
const VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID: u8 = 20;

/// Vanilla `PlayerModelPart.CAPE` mask (`1 << 0`): the cape-shown bit that gates
/// `AvatarRenderer.isEntityUpsideDown` to `isPlayerUpsideDown`.
const VANILLA_AVATAR_CAPE_PART_MASK: i8 = 0x01;

pub(crate) struct EntityStore {
    ecs: World,
    by_protocol_id: BTreeMap<i32, Entity>,
    order: Vec<i32>,
}

impl EntityStore {
    pub(crate) fn insert_or_replace(&mut self, state: EntityState) {
        if let Some(entity) = self.by_protocol_id.get(&state.id).copied() {
            self.replace_existing_components(entity, state);
            return;
        }

        if !self.order.contains(&state.id) {
            self.order.push(state.id);
        }

        self.spawn_components(state);
    }

    /// Spawns (or re-add-resets) an entity directly from `AddEntity` packet
    /// components without projecting through a full `EntityState`. A vanilla
    /// re-add of an existing protocol id resets every component to its
    /// spawn-fresh value.
    pub(crate) fn apply_add_entity_components(
        &mut self,
        identity: EntityIdentity,
        transform: EntityTransform,
    ) {
        let id = identity.id;
        let entity_type_id = identity.entity_type_id;
        let fresh_projectile = entity_hurting_projectile_from_state(entity_type_id, None);

        if let Some(entity) = self.by_protocol_id.get(&id).copied() {
            if let Ok(mut existing) = self.ecs.get::<&mut EntityIdentity>(entity) {
                *existing = identity;
            }
            if let Ok(mut existing) = self.ecs.get::<&mut EntityTransform>(entity) {
                *existing = transform;
            }
            fn reset<T: hecs::Component + Default>(ecs: &mut World, entity: Entity) {
                if let Ok(mut existing) = ecs.get::<&mut T>(entity) {
                    *existing = T::default();
                }
            }
            reset::<EntityMetadata>(&mut self.ecs, entity);
            reset::<EntityEquipment>(&mut self.ecs, entity);
            reset::<EntityAttributes>(&mut self.ecs, entity);
            reset::<EntityTransientEvents>(&mut self.ecs, entity);
            reset::<EntityMount>(&mut self.ecs, entity);
            reset::<EntityLeash>(&mut self.ecs, entity);
            reset::<EntityMobEffects>(&mut self.ecs, entity);
            reset::<EntityClientAnimations>(&mut self.ecs, entity);
            reset::<EntityDamage>(&mut self.ecs, entity);
            if is_vanilla_minecart_type(entity_type_id) {
                let updated = {
                    if let Ok(mut lerp) = self.ecs.get::<&mut EntityMinecartLerp>(entity) {
                        *lerp = EntityMinecartLerp::default();
                        true
                    } else {
                        false
                    }
                };
                if !updated {
                    let _ = self.ecs.insert_one(entity, EntityMinecartLerp::default());
                }
            } else {
                let _ = self.ecs.remove_one::<EntityMinecartLerp>(entity);
            }
            if let Some(projectile) = fresh_projectile {
                let updated = {
                    if let Ok(mut existing) = self.ecs.get::<&mut EntityHurtingProjectile>(entity) {
                        *existing = projectile;
                        true
                    } else {
                        false
                    }
                };
                if !updated {
                    let _ = self.ecs.insert_one(entity, projectile);
                }
            } else {
                let _ = self.ecs.remove_one::<EntityHurtingProjectile>(entity);
            }
            return;
        }

        if !self.order.contains(&id) {
            self.order.push(id);
        }
        let entity = self.ecs.spawn((
            identity,
            transform,
            EntityMetadata::default(),
            EntityEquipment::default(),
            EntityAttributes::default(),
            EntityTransientEvents::default(),
            EntityMount::default(),
            EntityLeash::default(),
            EntityMobEffects::default(),
            EntityClientAnimations::default(),
            EntityDamage::default(),
        ));
        if is_vanilla_minecart_type(entity_type_id) {
            let _ = self.ecs.insert_one(entity, EntityMinecartLerp::default());
        }
        if let Some(projectile) = fresh_projectile {
            let _ = self.ecs.insert_one(entity, projectile);
        }
        self.by_protocol_id.insert(id, entity);
    }

    fn spawn_components(&mut self, state: EntityState) {
        let id = state.id;
        let entity = self.ecs.spawn((
            EntityIdentity::from(&state),
            EntityTransform::from(&state),
            EntityMetadata::from(&state),
            EntityEquipment::from(&state),
            EntityAttributes::from(&state),
            EntityTransientEvents::from(&state),
            EntityMount::from(&state),
            EntityLeash::from(&state),
            EntityMobEffects::from(&state),
            EntityClientAnimations::from(&state),
            EntityDamage::from(&state),
        ));
        // Only vanilla minecart types run the new-behavior lerp, so attach `EntityMinecartLerp`
        // conditionally. This keeps the per-client-tick `query_mut::<&mut EntityMinecartLerp>` scan
        // and the lerp-step accounting limited to actual minecarts instead of every entity.
        if is_vanilla_minecart_type(state.entity_type_id) {
            let _ = self
                .ecs
                .insert_one(entity, EntityMinecartLerp::from(&state));
        }
        if let Some(projectile) =
            entity_hurting_projectile_from_state(state.entity_type_id, state.hurting_projectile)
        {
            let _ = self.ecs.insert_one(entity, projectile);
        }
        self.by_protocol_id.insert(id, entity);
    }

    fn replace_existing_components(&mut self, entity: Entity, state: EntityState) {
        self.sync_components_from_state(entity, &state);
    }

    pub(crate) fn get(&self, id: i32) -> Option<EntityState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.project_entity(entity)
    }

    pub(crate) fn contains(&self, id: i32) -> bool {
        self.by_protocol_id.contains_key(&id)
    }

    pub(crate) fn entity_type_id(&self, id: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityIdentity>(entity)
            .ok()
            .map(|identity| identity.entity_type_id)
    }

    pub(crate) fn first_entity_id_with_type_and_data(
        &self,
        entity_type_id: i32,
        data: i32,
    ) -> Option<i32> {
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            if identity.entity_type_id == entity_type_id && identity.data == data {
                return Some(identity.id);
            }
        }
        None
    }

    pub(crate) fn identity(&self, id: i32) -> Option<EntityIdentity> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityIdentity>(entity)
            .ok()
            .map(|identity| (*identity).clone())
    }

    pub(crate) fn main_arm_left(&self, id: i32) -> Option<bool> {
        let entity_type_id = self.entity_type_id(id)?;
        Some(self.entity_main_arm_left(id, entity_type_id))
    }

    pub(crate) fn is_silent(&self, id: i32) -> Option<bool> {
        self.metadata_bool(id, VANILLA_ENTITY_SILENT_DATA_ID, false)
    }

    pub(crate) fn no_gravity(&self, id: i32) -> Option<bool> {
        self.metadata_bool(id, VANILLA_ENTITY_NO_GRAVITY_DATA_ID, false)
    }

    pub(crate) fn ticks_frozen(&self, id: i32) -> Option<i32> {
        self.metadata_int(id, VANILLA_ENTITY_TICKS_FROZEN_DATA_ID, 0)
    }

    fn metadata_byte(&self, id: i32, data_id: u8, default: i8) -> Option<i8> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::Byte(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(default),
        )
    }

    fn metadata_optional_component(&self, id: i32, data_id: u8) -> Option<String> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        metadata
            .data_values
            .iter()
            .find(|value| value.data_id == data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::OptionalComponent(component) => component.clone(),
                _ => None,
            })
    }

    /// Vanilla `LivingEntity.getSleepingPos`: the synced bed position the entity is
    /// sleeping in (`SLEEPING_POS_ID`), or `None` when it is not in a bed.
    pub(crate) fn sleeping_pos(&self, id: i32) -> Option<crate::BlockPos> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        metadata
            .data_values
            .iter()
            .find(|value| value.data_id == VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID)
            .and_then(|value| match &value.value {
                EntityDataValueKind::OptionalBlockPos(Some(pos)) => Some(crate::BlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                }),
                _ => None,
            })
    }

    /// Vanilla `AvatarRenderer.isEntityUpsideDown` inputs for a player entity: its
    /// profile UUID (to resolve the GameProfile name from the player-info list) and
    /// whether the `CAPE` model part is shown (`DATA_PLAYER_MODE_CUSTOMISATION & 1`).
    /// `None` for non-player entities, which never use the avatar upside-down path.
    pub(crate) fn avatar_upside_down_inputs(&self, id: i32) -> Option<(Uuid, bool)> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
            return None;
        }
        let cape_shown = self
            .metadata_byte(id, VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID, 0)
            .unwrap_or(0)
            & VANILLA_AVATAR_CAPE_PART_MASK
            != 0;
        Some((identity.uuid, cape_shown))
    }

    fn entity_main_arm_left(&self, id: i32, entity_type_id: i32) -> bool {
        if entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            return self
                .metadata_humanoid_arm(
                    id,
                    VANILLA_AVATAR_MAIN_HAND_DATA_ID,
                    VANILLA_HUMANOID_ARM_RIGHT_ID,
                )
                .is_some_and(|arm| arm == VANILLA_HUMANOID_ARM_LEFT_ID);
        }
        if vanilla_mob_item_in_hand_layer_type(entity_type_id) {
            return self
                .metadata_byte(id, VANILLA_MOB_FLAGS_DATA_ID, 0)
                .is_some_and(|flags| flags & MOB_FLAG_LEFTHANDED != 0);
        }
        false
    }

    /// Vanilla `Entity.getEyeHeight(Pose.STANDING)` used by the sleeping bed
    /// head-offset translate: the eye height resolved with the synced pose stripped
    /// so the dimensions fall back to standing rather than `SLEEPING_DIMENSIONS`.
    pub(crate) fn standing_eye_height(&self, id: i32) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        let standing_data: Vec<_> = metadata
            .data_values
            .iter()
            .filter(|value| value.data_id != ENTITY_DATA_POSE_ID)
            .cloned()
            .collect();
        vanilla_eye_height_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &standing_data,
            &attributes.attributes,
            Some(client_animations.animations),
        )
    }

    fn metadata_bool(&self, id: i32, data_id: u8, default: bool) -> Option<bool> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::Boolean(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(default),
        )
    }

    fn metadata_int(&self, id: i32, data_id: u8, default: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::Int(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(default),
        )
    }

    fn metadata_humanoid_arm(&self, id: i32, data_id: u8, default: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::HumanoidArm(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(default),
        )
    }

    fn metadata_optional_unsigned_int(&self, id: i32, data_id: u8) -> Option<Option<i32>> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::OptionalUnsignedInt(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(None),
        )
    }

    /// The block-state id carried by an `OPTIONAL_BLOCK_STATE` accessor at `data_id`. The
    /// protocol decodes wire id `0` to `None`, mirroring vanilla's empty optional; an absent or
    /// wrong-typed metadata entry also defaults to `None`.
    fn metadata_optional_block_state(&self, id: i32, data_id: u8) -> Option<Option<i32>> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::OptionalBlockState(state) => Some(*state),
                    _ => None,
                })
                .unwrap_or(None),
        )
    }

    fn metadata_block_state(&self, id: i32, data_id: u8) -> Option<Option<i32>> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(metadata.data_values.iter().find_map(|value| {
            if value.data_id != data_id {
                return None;
            }
            match &value.value {
                EntityDataValueKind::BlockState(state) => Some(*state),
                _ => None,
            }
        }))
    }

    pub(crate) fn enderman_carried_block_state_id(&self, id: i32) -> Option<Option<i32>> {
        let identity = self.identity(id)?;
        if !vanilla_is_enderman(identity.entity_type_id) {
            return Some(None);
        }
        self.metadata_optional_block_state(id, VANILLA_ENDERMAN_CARRY_STATE_DATA_ID)
    }

    pub(crate) fn minecart_display_block_state(
        &self,
        id: i32,
        registries: &RegistrySet,
    ) -> Option<MinecartDisplayBlockState> {
        let identity = self.identity(id)?;
        if !is_vanilla_minecart_type(identity.entity_type_id) {
            return None;
        }

        let custom_state_id = self
            .metadata_optional_block_state(id, MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID)
            .unwrap_or(None);
        let block = if let Some(state_id) = custom_state_id {
            let state = registries.block_state(state_id)?;
            if state.name == "minecraft:air" {
                return None;
            }
            EntityBlockModelState {
                name: state.name.clone(),
                properties: state.properties.clone(),
            }
        } else {
            let furnace_has_fuel = identity.entity_type_id
                == VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID
                && self
                    .metadata_bool(id, FURNACE_MINECART_FUEL_DATA_ID, false)
                    .unwrap_or(false);
            default_minecart_display_block_state(
                identity.entity_type_id,
                furnace_has_fuel,
                registries,
            )?
        };
        let default_offset = default_minecart_display_offset(identity.entity_type_id);
        let display_offset = self
            .metadata_int(id, MINECART_DISPLAY_OFFSET_DATA_ID, default_offset)
            .unwrap_or(default_offset);
        Some(MinecartDisplayBlockState {
            block,
            display_offset,
        })
    }

    pub(crate) fn falling_block_state(
        &self,
        id: i32,
        registries: &RegistrySet,
    ) -> Option<FallingBlockModelState> {
        let identity = self.identity(id)?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID {
            return None;
        }
        let state = registries.block_state(identity.data)?;
        if state.name == "minecraft:air" {
            return None;
        }
        Some(FallingBlockModelState {
            block_state_id: identity.data,
            block: EntityBlockModelState {
                name: state.name.clone(),
                properties: state.properties.clone(),
            },
        })
    }

    pub(crate) fn primed_tnt_block_state(
        &self,
        id: i32,
        registries: &RegistrySet,
    ) -> Option<EntityBlockModelState> {
        let identity = self.identity(id)?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_TNT_ID {
            return None;
        }

        let state_id = self
            .metadata_block_state(id, PRIMED_TNT_BLOCK_STATE_DATA_ID)
            .unwrap_or(None);
        if let Some(state_id) = state_id {
            let state = registries.block_state(state_id)?;
            if state.name == "minecraft:air" {
                return None;
            }
            return Some(EntityBlockModelState {
                name: state.name.clone(),
                properties: state.properties.clone(),
            });
        }

        default_primed_tnt_block_state(registries)
    }

    pub(crate) fn primed_tnt_fuse_remaining_in_ticks(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<f32> {
        let identity = self.identity(id)?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_TNT_ID {
            return None;
        }
        let fuse = self
            .metadata_int(id, PRIMED_TNT_FUSE_DATA_ID, DEFAULT_PRIMED_TNT_FUSE)
            .unwrap_or(DEFAULT_PRIMED_TNT_FUSE);
        Some(fuse as f32 - partial_ticks + 1.0)
    }

    pub(crate) fn primed_tnt_smoke_particle_states(
        &self,
    ) -> Vec<super::PrimedTntSmokeParticleState> {
        let mut states = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(mut query) = self
                .ecs
                .query_one::<(&EntityIdentity, &EntityTransform)>(entity)
            else {
                continue;
            };
            let Some((identity, transform)) = query.get() else {
                continue;
            };
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_TNT_ID {
                continue;
            }
            let fuse = self
                .metadata_int(
                    identity.id,
                    PRIMED_TNT_FUSE_DATA_ID,
                    DEFAULT_PRIMED_TNT_FUSE,
                )
                .unwrap_or(DEFAULT_PRIMED_TNT_FUSE);
            // Vanilla `PrimedTnt.tick` decrements fuse first and only emits smoke when the
            // resulting fuse remains positive.
            if fuse <= 1 {
                continue;
            }
            states.push(super::PrimedTntSmokeParticleState {
                entity_id: identity.id,
                position: transform.position,
            });
        }
        states
    }

    pub(crate) fn ravager_roar_particle_state(&self, id: i32) -> Option<RavagerRoarParticleState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_RAVAGER_ID {
            return None;
        }
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let bounds = self.pick_bounds(id)?;
        let center_offset = [
            f64::from((bounds.min[0] + bounds.max[0]) * 0.5),
            f64::from((bounds.min[1] + bounds.max[1]) * 0.5),
            f64::from((bounds.min[2] + bounds.max[2]) * 0.5),
        ];
        Some(RavagerRoarParticleState {
            entity_id: id,
            center: super::EntityVec3 {
                x: transform.position.x + center_offset[0],
                y: transform.position.y + center_offset[1],
                z: transform.position.z + center_offset[2],
            },
        })
    }

    pub(crate) fn vault_connection_player_target(
        &self,
        uuid: Uuid,
        vault_pos: crate::BlockPos,
        range: f64,
    ) -> Option<(i32, [f64; 3])> {
        if !range.is_finite() || range < 0.0 {
            return None;
        }
        let range_sqr = range * range;
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let (entity_id, position) = {
                let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                    continue;
                };
                if identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID || identity.uuid != uuid
                {
                    continue;
                }
                let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                    return None;
                };
                (identity.id, transform.position)
            };
            if block_pos_dist_sqr(entity_block_pos(position), vault_pos) > range_sqr {
                return None;
            }
            let bounds = self.pick_bounds(entity_id)?;
            let height = f64::from(bounds.max[1] - bounds.min[1]);
            return Some((
                entity_id,
                [position.x, position.y + height / 2.0, position.z],
            ));
        }
        None
    }

    pub(crate) fn pose(&self, id: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(entity_data_pose(&metadata.data_values))
    }

    pub(crate) fn pick_bounds(&self, id: i32) -> Option<super::EntityPickBoundsState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        vanilla_pick_bounds_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(client_animations.animations),
        )
    }

    pub(crate) fn body_anchor_y_offset(
        &self,
        id: i32,
        game_time: i64,
        is_front: bool,
        partial_ticks: f32,
    ) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        vanilla_body_anchor_y_offset_for_entity_data(
            identity.entity_type_id,
            &metadata.data_values,
            &attributes.attributes,
            game_time,
            is_front,
            partial_ticks,
        )
    }

    fn model_source_bounds(
        &self,
        id: i32,
        registries: &RegistrySet,
    ) -> Option<super::EntityPickBoundsState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        let mut bounds = vanilla_model_source_bounds_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(client_animations.animations),
        )?;
        if let Some(display) = self.minecart_display_block_state(identity.id, registries) {
            bounds = bounds.expand_towards_y(display.display_offset as f32 * 0.75 / 16.0);
        }
        Some(bounds)
    }

    pub(crate) fn pick_targets_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<super::EntityPickTargetState> {
        let mut targets = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(mut query) = self.ecs.query_one::<(
                &EntityIdentity,
                &EntityTransform,
                Option<&EntityClientAnimations>,
            )>(entity) else {
                continue;
            };
            let Some((identity, transform, client_animations)) = query.get() else {
                continue;
            };
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
                let dragon_animation =
                    client_animations.and_then(|animations| animations.animations.ender_dragon);
                targets.extend(ender_dragon_part_pick_targets_at_partial_tick(
                    identity.id,
                    *transform,
                    dragon_animation,
                    partial_ticks,
                ));
            } else if let Some(bounds) = self.pick_bounds(identity.id) {
                targets.push(super::EntityPickTargetState {
                    entity_id: identity.id,
                    position: transform.position,
                    bounds,
                });
            }
        }
        targets
    }

    pub(crate) fn model_targets_at_partial_tick(
        &self,
        _partial_ticks: f32,
        registries: &RegistrySet,
    ) -> Vec<super::EntityModelTargetState> {
        let mut targets = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(mut query) = self
                .ecs
                .query_one::<(&EntityIdentity, &EntityTransform)>(entity)
            else {
                continue;
            };
            let Some((identity, transform)) = query.get() else {
                continue;
            };
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
                continue;
            }
            let Some(bounds) = self.model_source_bounds(identity.id, registries) else {
                continue;
            };
            targets.push(super::EntityModelTargetState {
                entity_id: identity.id,
                position: transform.position,
                bounds,
            });
        }
        targets
    }

    pub(crate) fn refresh_client_position_from_entity_data(&mut self, id: i32) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let (identity, metadata, transform) = self
            .ecs
            .query_one_mut::<(&EntityIdentity, &EntityMetadata, &mut EntityTransform)>(entity)
            .ok()?;
        let position = vanilla_client_position_for_entity_data(
            identity.entity_type_id,
            transform.position_base,
            identity.data,
            &metadata.data_values,
        )?;
        transform.position = position;
        Some(())
    }

    pub(crate) fn transform(&self, id: i32) -> Option<EntityTransform> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityTransform>(entity)
            .ok()
            .map(|transform| *transform)
    }

    pub(crate) fn attribute_value(&self, id: i32, attribute_id: i32) -> Option<f64> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        attributes
            .attributes
            .iter()
            .find(|attribute| attribute.attribute_id == attribute_id)
            .map(vanilla_attribute_value)
    }

    pub(crate) fn attribute_has_modifier(
        &self,
        id: i32,
        attribute_id: i32,
        modifier_id: &str,
    ) -> bool {
        let Some(entity) = self.by_protocol_id.get(&id).copied() else {
            return false;
        };
        let Ok(attributes) = self.ecs.get::<&EntityAttributes>(entity) else {
            return false;
        };
        attributes
            .attributes
            .iter()
            .find(|attribute| attribute.attribute_id == attribute_id)
            .is_some_and(|attribute| {
                attribute
                    .modifiers
                    .iter()
                    .any(|modifier| modifier.id == modifier_id)
            })
    }

    pub(crate) fn transform_state(&self, id: i32) -> Option<EntityTransformState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.transform_state_for_entity(entity)
    }

    fn new_minecart_passenger_render_offset(
        &self,
        vehicle_id: i32,
        partial_ticks: f32,
    ) -> Option<super::EntityVec3> {
        let vehicle = self.by_protocol_id.get(&vehicle_id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(vehicle).ok()?;
        if !is_vanilla_minecart_type(identity.entity_type_id) {
            return None;
        }
        let transform = self.ecs.get::<&EntityTransform>(vehicle).ok()?;
        let minecart_lerp = self.ecs.get::<&EntityMinecartLerp>(vehicle).ok()?;
        let old_step = minecart_lerp.old_step?;
        let render_step = minecart_lerp.render_step(partial_ticks)?;
        let cart_lerp_position = lerp_entity_vec3(
            partial_ticks,
            super::movement::entity_vec3(old_step.position),
            transform.position,
        );
        Some(sub_entity_vec3(
            super::movement::entity_vec3(render_step.position),
            cart_lerp_position,
        ))
    }

    pub(crate) fn model_source(
        &self,
        id: i32,
        position: super::EntityVec3,
        partial_ticks: f32,
        registries: &RegistrySet,
        default_item_max_damage: &BTreeMap<i32, i32>,
        armor_materials: &BTreeMap<i32, ArmorMaterialKind>,
        equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
        llama_body_decor_colors: &BTreeMap<i32, LlamaBodyDecorColor>,
        nautilus_body_armor_materials: &BTreeMap<i32, ArmorMaterialKind>,
        horse_body_armor_materials: &BTreeMap<i32, ArmorMaterialKind>,
        wolf_body_armor_materials: &BTreeMap<i32, ArmorMaterialKind>,
    ) -> Option<EntityModelSourceState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        // Resolve the entity's storage location once and fetch every component this projection reads
        // in a single `query_one`. The required components fail the query (yielding `None`) if any is
        // missing; optional ones stay `Option<&_>` so a bare entity resolves them to `None`.
        //
        // Vanilla `HumanoidArmorLayer` worn armor: resolve the item worn in each armor slot to its
        // equipment-asset material. The `EntityEquipment` component holds the synced `SetEquipment`
        // items; a bare entity (no equipment component / empty slot / non-armor item) resolves to None.
        let mut query = self
            .ecs
            .query_one::<(
                &EntityIdentity,
                &EntityTransform,
                &EntityMetadata,
                &EntityAttributes,
                &EntityClientAnimations,
                Option<&EntityEquipment>,
                Option<&EntityMount>,
                Option<&EntityMinecartLerp>,
            )>(entity)
            .ok()?;
        let (
            identity,
            transform,
            metadata,
            attributes,
            client_animations,
            equipment,
            mount,
            minecart_lerp,
        ) = query.get()?;
        let armor_item = |slot: ProtocolEquipmentSlot| -> Option<&ItemStackSummary> {
            equipment
                .as_ref()?
                .equipment
                .iter()
                .find(|update| update.slot == slot)
                .map(|update| &update.item)
        };
        let armor_material = |slot: ProtocolEquipmentSlot| -> Option<ArmorMaterialKind> {
            let item_id = armor_item(slot)?.item_id?;
            armor_materials.get(&item_id).copied()
        };
        // Vanilla `DyedItemColor.getOrDefault`: the per-slot worn item's `dyed_color` component (a
        // packed RGB), carried alongside the material. Only leather consumes it client-side; a slot
        // with no equipment / no dye component resolves to None.
        let armor_dye = |slot: ProtocolEquipmentSlot| -> Option<i32> {
            armor_item(slot)?.component_patch.dyed_color
        };
        let armor_foil = |slot: ProtocolEquipmentSlot| -> bool {
            armor_material(slot)
                .and_then(|_| armor_item(slot))
                .is_some_and(ItemStackSummary::has_foil)
        };
        // Vanilla `SimpleEquipmentLayer` saddle users copy `EquipmentSlot.SADDLE` into render state,
        // then render only a non-empty equippable saddle item. bbb resolves the default saddle item
        // from the item equipment-slot map.
        let saddle_slot_contains_saddle_item = || -> bool {
            let Some(equipment) = equipment.as_ref() else {
                return false;
            };
            let Some(item) = equipment
                .equipment
                .iter()
                .find(|update| update.slot == ProtocolEquipmentSlot::Saddle)
                .map(|update| &update.item)
            else {
                return false;
            };
            if item.count <= 0 {
                return false;
            }
            item.item_id
                .and_then(|item_id| equipment_slots.get(&item_id).copied())
                == Some(ItemEquipmentSlot::Saddle)
        };
        let pig_saddle = || -> bool {
            identity.entity_type_id == VANILLA_ENTITY_TYPE_PIG_ID
                && saddle_slot_contains_saddle_item()
        };
        let equine_saddle = || -> bool {
            vanilla_equine_saddle_type(identity.entity_type_id)
                && saddle_slot_contains_saddle_item()
        };
        let equine_saddle = equine_saddle();
        let equine_saddle_ridden = equine_saddle
            && mount
                .as_ref()
                .is_some_and(|mount| !mount.passengers.is_empty());
        let strider_ridden = identity.entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID
            && mount
                .as_ref()
                .is_some_and(|mount| !mount.passengers.is_empty());
        let strider_saddle = identity.entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID
            && saddle_slot_contains_saddle_item();
        let camel_saddle = vanilla_camel_saddle_type(identity.entity_type_id)
            && saddle_slot_contains_saddle_item();
        let camel_saddle_ridden = camel_saddle
            && mount
                .as_ref()
                .is_some_and(|mount| !mount.passengers.is_empty());
        let nautilus_saddle = is_vanilla_abstract_nautilus_type(identity.entity_type_id)
            && saddle_slot_contains_saddle_item();
        let body_slot_item = || -> Option<&ItemStackSummary> {
            equipment
                .as_ref()
                .and_then(|equipment| {
                    equipment
                        .equipment
                        .iter()
                        .find(|update| update.slot == ProtocolEquipmentSlot::Body)
                })
                .map(|update| &update.item)
                .filter(|item| item.count > 0)
        };
        let body_slot_item_id =
            || -> Option<i32> { body_slot_item().and_then(|item| item.item_id) };
        let body_slot_armor_dye =
            || -> Option<i32> { body_slot_item().and_then(|item| item.component_patch.dyed_color) };
        let llama_body_decor = if is_vanilla_llama_type(identity.entity_type_id)
            && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values)
        {
            body_slot_item_id().and_then(|item_id| llama_body_decor_colors.get(&item_id).copied())
        } else {
            None
        };
        let nautilus_body_armor = if is_vanilla_abstract_nautilus_type(identity.entity_type_id)
            && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values)
        {
            body_slot_item_id()
                .and_then(|item_id| nautilus_body_armor_materials.get(&item_id).copied())
        } else {
            None
        };
        let horse_body_armor = if is_vanilla_can_wear_horse_armor_type(identity.entity_type_id)
            && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values)
        {
            body_slot_item_id()
                .and_then(|item_id| horse_body_armor_materials.get(&item_id).copied())
        } else {
            None
        };
        let horse_body_armor_dye = horse_body_armor.and_then(|_| body_slot_armor_dye());
        let wolf_body_armor = if identity.entity_type_id == VANILLA_ENTITY_TYPE_WOLF_ID
            && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values)
        {
            body_slot_item_id().and_then(|item_id| wolf_body_armor_materials.get(&item_id).copied())
        } else {
            None
        };
        let wolf_body_armor_dye = wolf_body_armor.and_then(|_| body_slot_armor_dye());
        let wolf_body_armor_crackiness = wolf_body_armor
            .and_then(|_| body_slot_item())
            .map(|item| wolf_armor_crackiness(item, default_item_max_damage))
            .unwrap_or(WolfArmorCrackiness::None);
        let wolf_body_armor_foil = wolf_body_armor
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        // Vanilla `LivingEntityRenderer.isShaking` (base) is `Entity.isFullyFrozen`
        // (`getTicksFrozen() >= 140`), and only living entities shake.
        let is_fully_frozen = vanilla_living_entity_type(identity.entity_type_id)
            && self.ticks_frozen(id).unwrap_or(0) >= VANILLA_TICKS_REQUIRED_TO_FREEZE;
        // Vanilla `LivingEntityRenderer.extractRenderState`: `isInvisibleToPlayer =
        // state.isInvisible && entity.isInvisibleTo(minecraft.player)`. The store-local baseline is
        // the non-spectator / no-friendly-team-override branch, so the world aggregation can clear it
        // for spectator viewers.
        let invisible_to_player = self
            .metadata_byte(id, ENTITY_SHARED_FLAGS_DATA_ID, 0)
            .unwrap_or(0)
            & ENTITY_SHARED_FLAG_INVISIBLE
            != 0;
        let appears_glowing = self
            .metadata_byte(id, ENTITY_SHARED_FLAGS_DATA_ID, 0)
            .unwrap_or(0)
            & ENTITY_SHARED_FLAG_GLOWING
            != 0;
        // Vanilla `Mob.isAggressive()` (`DATA_MOB_FLAGS_ID & 4`): the zombie-model family
        // consumes it (their held-out `animateZombieArms` arm drop deepens when aggressive),
        // the piglin/brute drive `ATTACKING_WITH_MELEE_WEAPON` (raise + swing a melee weapon)
        // with it, and the vindicator / pillager (`ATTACKING`) plus illusioner (`BOW_AND_ARROW` aim)
        // resolve their arm pose from it. Every such type is a Mob carrying the flags byte;
        // other entities have no mob-flags byte (or do not use those arms), so they stay calm.
        let is_aggressive = (vanilla_zombie_model_family(identity.entity_type_id)
            || vanilla_piglin_melee_attack_family(identity.entity_type_id)
            || vanilla_illager_aggressive_arm_pose_family(identity.entity_type_id))
            && self
                .metadata_byte(id, VANILLA_MOB_FLAGS_DATA_ID, 0)
                .unwrap_or(0)
                & MOB_FLAG_AGGRESSIVE
                != 0;
        // Vanilla `VillagerRenderer` / `WanderingTraderRenderer.extractRenderState`:
        // `state.isUnhappy = entity.getUnhappyCounter() > 0`. The counter is defined on
        // `AbstractVillager` at synced int id 18, so only villager-family renderers read it.
        let villager_unhappy = matches!(
            identity.entity_type_id,
            VANILLA_ENTITY_TYPE_VILLAGER_ID | VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID
        ) && self
            .metadata_int(id, ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID, 0)
            .unwrap_or(0)
            > 0;
        // Vanilla `EndermanModel.setupAnim`: a carried block (`!carriedBlock.isEmpty()`,
        // the synced `DATA_CARRY_STATE`) poses both arms forward, and `isCreepy` (the
        // synced `DATA_CREEPY`) drops the head / raises the hat. Only the enderman defines
        // these accessors, so the projections are gated to it and default off otherwise.
        let is_enderman = vanilla_is_enderman(identity.entity_type_id);
        let enderman_carried_block_state_id = if is_enderman {
            self.metadata_optional_block_state(id, VANILLA_ENDERMAN_CARRY_STATE_DATA_ID)
                .unwrap_or(None)
        } else {
            None
        };
        let enderman_carrying = enderman_carried_block_state_id.is_some();
        let enderman_carried_block = enderman_carried_block_state_id
            .and_then(|state_id| registries.block_state(state_id))
            .map(|state| EntityBlockModelState {
                name: state.name.clone(),
                properties: state.properties.clone(),
            });
        let enderman_creepy = is_enderman
            && self
                .metadata_bool(id, VANILLA_ENDERMAN_CREEPY_DATA_ID, false)
                .unwrap_or(false);
        // Vanilla `BatModel.setupAnim` swaps to the `BAT_RESTING` hanging pose while
        // `Bat.isResting` (`DATA_ID_FLAGS & 1`). Only the bat defines that flags byte, so
        // the projection is gated to it and defaults to flying otherwise.
        let bat_resting = vanilla_is_bat(identity.entity_type_id)
            && self
                .metadata_byte(id, VANILLA_BAT_FLAGS_DATA_ID, 0)
                .unwrap_or(0)
                & BAT_FLAG_RESTING
                != 0;
        // Vanilla `BeeModel.setupAnim` hides the stinger cube once `Bee.hasStung`
        // (`DATA_FLAGS_ID & 4`). Only the bee defines that flags byte; every other entity keeps
        // its stinger field at the `true` default (and never renders a stinger anyway).
        let bee_has_stinger = !vanilla_is_bee(identity.entity_type_id)
            || self
                .metadata_byte(id, VANILLA_BEE_FLAGS_DATA_ID, 0)
                .unwrap_or(0)
                & BEE_FLAG_HAS_STUNG
                == 0;
        // Vanilla `FoxModel.setupAnim` reads the fox's synced `DATA_FLAGS_ID` (19) crouch / sleep /
        // sit / pounce / faceplant bits directly (no easing) to pick its pose branch. Only the fox
        // defines that flags byte, so the five bool projections are gated to it and default off. The
        // interest bit (8) is read by the animation layer instead, to drive the `headRollAngle`
        // accumulator below.
        let is_fox = vanilla_is_fox(identity.entity_type_id);
        let fox_flags = if is_fox {
            self.metadata_byte(id, VANILLA_FOX_FLAGS_DATA_ID, 0)
                .unwrap_or(0)
        } else {
            0
        };
        let fox_is_crouching = fox_flags & FOX_FLAG_CROUCHING != 0;
        let fox_is_sleeping = fox_flags & FOX_FLAG_SLEEPING != 0;
        let fox_is_sitting = fox_flags & FOX_FLAG_SITTING != 0;
        let fox_is_pouncing = fox_flags & FOX_FLAG_POUNCING != 0;
        let fox_is_faceplanted = fox_flags & FOX_FLAG_FACEPLANTED != 0;
        let is_feline = vanilla_feline_family(identity.entity_type_id);
        // Vanilla `CatRenderer` / `OcelotRenderer.extractRenderState` copies
        // `Entity.isCrouching()` and `Entity.isSprinting()` into `FelineRenderState`. The former is the
        // synced `Pose.CROUCHING`; the latter is shared entity flags bit 3. `CatRenderer` additionally
        // projects `Cat.isInSittingPose()` from `TamableAnimal.DATA_FLAGS_ID & 1`; `OcelotRenderer`
        // leaves `isSitting` at its default.
        let feline_is_crouching =
            is_feline && entity_data_pose(&metadata.data_values) == VANILLA_POSE_CROUCHING_ID;
        let feline_is_sprinting = is_feline
            && self
                .metadata_byte(id, ENTITY_SHARED_FLAGS_DATA_ID, 0)
                .unwrap_or(0)
                & ENTITY_SHARED_FLAG_SPRINTING
                != 0;
        let feline_is_sitting = vanilla_is_cat(identity.entity_type_id)
            && self
                .metadata_byte(id, TAMABLE_ANIMAL_FLAGS_DATA_ID, 0)
                .unwrap_or(0)
                & TAMABLE_ANIMAL_SITTING_FLAG
                != 0;
        // Vanilla `VexModel.setupAnim` levels the body (`xRot = 0`) and raises both arms
        // (`setArmsCharging`) while `Vex.isCharging` (`DATA_FLAGS_ID & 1`). Only the vex
        // defines that flags byte, so the projection is gated to it and defaults to idle.
        let vex_charging = vanilla_is_vex(identity.entity_type_id)
            && self
                .metadata_byte(id, VANILLA_VEX_FLAGS_DATA_ID, 0)
                .unwrap_or(0)
                & VEX_FLAG_IS_CHARGING
                != 0;
        // Vanilla `SnowGolemRenderer.extractRenderState`: when `SnowGolem.hasPumpkin()` is true, the
        // renderer resolves `Blocks.CARVED_PUMPKIN.defaultBlockState()` into `state.headBlock`. The
        // snow golem's synced byte defaults to 16, so a newly spawned golem shows the pumpkin until the
        // server clears that bit.
        let snow_golem_pumpkin = identity.entity_type_id == VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID
            && self
                .metadata_byte(
                    id,
                    VANILLA_SNOW_GOLEM_PUMPKIN_DATA_ID,
                    SNOW_GOLEM_PUMPKIN_FLAG,
                )
                .unwrap_or(SNOW_GOLEM_PUMPKIN_FLAG)
                & SNOW_GOLEM_PUMPKIN_FLAG
                != 0;
        // Vanilla `WitherBossRenderer.extractRenderState`: `state.invulnerableTicks =
        // invulnerableTicks > 0 ? invulnerableTicks - partialTicks : 0.0` (the synced `DATA_ID_INV`
        // spawn countdown, lerped against the partial tick). It drives both the `scale()` spawn-charge
        // shrink and the `getTextureLocation` `wither_invulnerable.png` flicker. Only the wither
        // defines that accessor, so every other entity holds `0.0`.
        let wither_invulnerable_ticks = if vanilla_is_wither(identity.entity_type_id) {
            let ticks = self
                .metadata_int(id, VANILLA_WITHER_INV_DATA_ID, 0)
                .unwrap_or(0);
            if ticks > 0 {
                ticks as f32 - partial_ticks
            } else {
                0.0
            }
        } else {
            0.0
        };
        // Vanilla `LivingEntity.isAutoSpinAttack` (`DATA_LIVING_ENTITY_FLAGS & 4`):
        // a living entity mid riptide-trident spin. Non-living entities have no
        // living-entity flags byte, so they never spin.
        let is_auto_spin_attack = vanilla_living_entity_type(identity.entity_type_id)
            && self
                .metadata_byte(id, VANILLA_LIVING_ENTITY_FLAGS_DATA_ID, 0)
                .unwrap_or(0)
                & LIVING_ENTITY_FLAG_SPIN_ATTACK
                != 0;
        // Vanilla `LivingEntity.isUsingItem` (`DATA_LIVING_ENTITY_FLAGS & 1`) + `getUsedItemHand`
        // (`& 2` → off hand): the use-item arm poses (spyglass / horn / brush) read both. Non-living
        // entities have no flags byte, so they never count as using an item.
        let living_entity_flags = vanilla_living_entity_type(identity.entity_type_id)
            .then(|| {
                self.metadata_byte(id, VANILLA_LIVING_ENTITY_FLAGS_DATA_ID, 0)
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        let is_using_item = living_entity_flags & LIVING_ENTITY_FLAG_IS_USING != 0;
        let use_item_off_hand = living_entity_flags & LIVING_ENTITY_FLAG_OFF_HAND != 0;
        let main_arm_left = self.entity_main_arm_left(id, identity.entity_type_id);
        // Vanilla `LivingEntityRenderer.isEntityUpsideDown`: a non-player living
        // entity whose custom name is `Dinnerbone`/`Grumm`. The player variant
        // (`AvatarRenderer.isPlayerUpsideDown`) keys off the GameProfile name and
        // the cape model part instead, which need the player-info list, so the
        // player type is excluded here and that path stays deferred.
        let is_upside_down = vanilla_living_entity_type(identity.entity_type_id)
            && identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
            && self
                .metadata_optional_component(id, VANILLA_ENTITY_CUSTOM_NAME_DATA_ID)
                .is_some_and(|name| VANILLA_UPSIDE_DOWN_NAMES.contains(&name.as_str()));
        // Vanilla `EntityRenderState.boundingBoxHeight` (`Entity.getBbHeight`): the
        // pick-bounds AABB height, used to lift the upside-down model before flipping.
        let bounding_box_height = self
            .pick_bounds(id)
            .map(|bounds| bounds.max[1] - bounds.min[1])
            .unwrap_or(0.0);
        // Vanilla `LivingEntityRenderState.hasPose(Pose.SLEEPING)`: only living
        // entities lie down. The bed orientation/offset are resolved spatially by
        // the WorldStore aggregation (which owns the block data); the per-entity
        // source defaults to the no-bed fallback.
        let is_sleeping = vanilla_living_entity_type(identity.entity_type_id)
            && entity_data_pose(&metadata.data_values) == VANILLA_POSE_SLEEPING_ID;
        // Vanilla `Entity.isCrouching` (`Pose.CROUCHING`): only the player is ever put into the
        // crouch pose by the server, and only the player model has the `HumanoidModel.setupAnim`
        // crouch, so the projection is gated to the player.
        let is_crouching = identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            && entity_data_pose(&metadata.data_values) == VANILLA_POSE_CROUCHING_ID;
        // Vanilla `LivingEntityRenderState.scale` (`LivingEntity.getScale`, the SCALE
        // attribute): only living entities carry a render scale; everything else
        // renders at its default size.
        let scale = if vanilla_living_entity_type(identity.entity_type_id) {
            vanilla_render_scale(identity.entity_type_id, &attributes.attributes)
        } else {
            1.0
        };
        // Vanilla `Sniffer.onSyncedDataUpdated`: the active `Sniffer.State` one-shot `AnimationState`
        // as `(state ordinal, elapsed seconds)`, or `(-1, -1.0)` for an idling/searching sniffer and
        // every non-sniffer (only the sniffer is given a sniffer animation state).
        let (sniffer_animation_id, sniffer_animation_seconds) = client_animations
            .animations
            .sniffer_animation(partial_ticks);
        let walk_animation_position = client_animations
            .animations
            .walk_animation_position(partial_ticks);
        let (player_cape_flap, player_cape_lean, player_cape_lean2) =
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
                client_animations
                    .animations
                    .player_cape_state(partial_ticks, transform.y_rot)
            } else {
                (0.0, 0.0, 0.0)
            };
        let (player_left_shoulder_parrot, player_right_shoulder_parrot) =
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
                (
                    self.metadata_optional_unsigned_int(
                        id,
                        VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID,
                    )
                    .unwrap_or(None),
                    self.metadata_optional_unsigned_int(
                        id,
                        VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID,
                    )
                    .unwrap_or(None),
                )
            } else {
                (None, None)
            };
        let is_baby = vanilla_is_baby(identity.entity_type_id, &metadata.data_values);
        let is_panda = identity.entity_type_id == VANILLA_ENTITY_TYPE_PANDA_ID;
        let panda_roll_amount = if is_panda && !is_baby {
            client_animations
                .animations
                .panda_roll_amount(partial_ticks)
        } else {
            0.0
        };
        let is_passenger = mount
            .as_ref()
            .is_some_and(|mount| mount.vehicle_id.is_some());
        let boat_paddles = mount.as_ref().map_or([false; 2], |mount| {
            boat_paddle_states(&metadata.data_values, !mount.passengers.is_empty())
        });
        let boat_rowing_times = client_animations.animations.boat_rowing_times(
            partial_ticks,
            boat_paddles[0],
            boat_paddles[1],
        );
        let (boat_hurt_time, boat_hurt_dir, boat_damage_time) = client_animations
            .animations
            .boat_damage_state(partial_ticks);
        let minecart_new_render = is_vanilla_minecart_type(identity.entity_type_id)
            && minecart_lerp
                .as_ref()
                .is_some_and(|lerp| !lerp.steps.is_empty() || lerp.old_step.is_some());
        let mut source_position = position;
        let mut source_y_rot = transform.y_rot;
        let mut source_x_rot = transform.x_rot;
        if minecart_new_render {
            if let Some(render_step) = minecart_lerp
                .as_ref()
                .and_then(|lerp| lerp.render_step(partial_ticks))
            {
                source_position = super::movement::entity_vec3(render_step.position);
                source_y_rot = render_step.y_rot;
                source_x_rot = render_step.x_rot;
            }
        }
        if let Some(offset) =
            mount
                .as_ref()
                .and_then(|mount| mount.vehicle_id)
                .and_then(|vehicle_id| {
                    self.new_minecart_passenger_render_offset(vehicle_id, partial_ticks)
                })
        {
            // Vanilla `EntityRenderer.extractRenderState` adds this via `passengerOffset`
            // when a new-behavior minecart's weighted render position diverges from its
            // normal xOld/getX interpolation.
            source_position = add_entity_vec3(source_position, offset);
        }
        let boat_bubble_angle = client_animations
            .animations
            .boat_bubble_angle(partial_ticks);
        let (wither_x_head_rots, wither_y_head_rots) = if vanilla_is_wither(identity.entity_type_id)
        {
            client_animations.animations.wither_head_rotations()
        } else {
            ([0.0; 2], [0.0; 2])
        };
        // Vanilla `LivingEntityRenderer.extractRenderState`: worn skull animation normally mirrors
        // the entity walk position, but while riding a living entity it reads the vehicle's walk
        // animation position even though the passenger's own limb swing is stopped.
        let worn_head_animation_position = mount
            .as_ref()
            .and_then(|mount| mount.vehicle_id)
            .and_then(|vehicle_id| {
                self.living_entity_walk_animation_position(vehicle_id, partial_ticks)
            })
            .unwrap_or(walk_animation_position);
        // Vanilla `LivingEntity.recreateFromPacket` seeds squid `yBodyRot` from
        // the add-entity head yaw; `Squid.aiStep` then refines it from movement
        // while in water. Keep it as renderer-only body yaw so the synced
        // transform yaw remains canonical.
        let squid_y_body_rot = if matches!(
            identity.entity_type_id,
            VANILLA_ENTITY_TYPE_SQUID_ID | VANILLA_ENTITY_TYPE_GLOW_SQUID_ID
        ) {
            client_animations
                .animations
                .squid_y_body_rot(partial_ticks)
                .unwrap_or(transform.y_head_rot)
        } else {
            0.0
        };
        Some(EntityModelSourceState {
            entity_id: identity.id,
            uuid: identity.uuid,
            entity_type_id: identity.entity_type_id,
            position: source_position,
            y_rot: source_y_rot,
            x_rot: source_x_rot,
            y_head_rot: transform.y_head_rot,
            arrow_shake: client_animations.animations.arrow_shake(partial_ticks),
            is_passenger,
            age_ticks: client_animations.animations.age_ticks,
            boat_rowing_time_left: boat_rowing_times[0],
            boat_rowing_time_right: boat_rowing_times[1],
            boat_hurt_time,
            boat_hurt_dir,
            boat_damage_time,
            minecart_new_render,
            minecart_pos_on_rail: None,
            minecart_front_pos: None,
            minecart_back_pos: None,
            minecart_tnt_fuse_remaining_in_ticks: client_animations
                .animations
                .minecart_tnt_fuse_remaining_in_ticks(partial_ticks),
            boat_bubble_angle,
            boat_underwater: false,
            wither_x_head_rots,
            wither_y_head_rots,
            is_fully_frozen,
            invisible_to_player,
            appears_glowing,
            outline_color: 0,
            is_aggressive,
            villager_unhappy,
            enderman_carrying,
            enderman_carried_block,
            enderman_creepy,
            bat_resting,
            bee_has_stinger,
            bee_roll_amount: client_animations.animations.bee_roll_amount(partial_ticks),
            panda_sit_amount: client_animations.animations.panda_sit_amount(partial_ticks),
            panda_lie_on_back_amount: client_animations
                .animations
                .panda_lie_on_back_amount(partial_ticks),
            panda_roll_amount,
            panda_roll_time: client_animations.animations.panda_roll_time(partial_ticks),
            // Vanilla `Frog.croakAnimationState`: the elapsed seconds since `Pose.CROAKING` started,
            // or `-1.0` for a non-croaking frog (only the frog is given a croak animation state).
            frog_croak_seconds: client_animations
                .animations
                .frog_croak_seconds(partial_ticks),
            // Vanilla `Frog.tongueAnimationState`: the elapsed seconds since `Pose.USING_TONGUE`
            // started, or `-1.0` for a frog not using its tongue (only the frog is given a tongue
            // animation state).
            frog_tongue_seconds: client_animations
                .animations
                .frog_tongue_seconds(partial_ticks),
            // Vanilla `Frog.jumpAnimationState`: the elapsed seconds since `Pose.LONG_JUMPING`
            // started, or `-1.0` for a non-jumping frog (only the frog is given a jump animation
            // state).
            frog_jump_seconds: client_animations
                .animations
                .frog_jump_seconds(partial_ticks),
            // Vanilla `Frog.swimIdleAnimationState`: the elapsed seconds since the in-water idle
            // started (`Frog.tick` drives it off the per-tick `isInWater()` + `!isMoving()`), or
            // `-1.0` for a dry/moving frog (only the frog is given a swim-idle animation state).
            frog_swim_idle_seconds: client_animations
                .animations
                .frog_swim_idle_seconds(partial_ticks),
            // Vanilla `Camel.dashAnimationState`: the elapsed seconds since the dash started (the
            // synced `DASH` boolean rising edge), or `-1.0` for a non-dashing camel and every other
            // entity (only the camel is given a dash animation state).
            camel_dash_seconds: client_animations
                .animations
                .camel_dash_seconds(partial_ticks),
            // Vanilla `Camel.idleAnimationState`: client-random timeout-driven idle keyframe elapsed
            // seconds, or `-1.0` for every non-camel.
            camel_idle_seconds: client_animations
                .animations
                .camel_idle_seconds(partial_ticks),
            copper_golem_idle_seconds: client_animations
                .animations
                .copper_golem_idle_seconds(partial_ticks),
            copper_golem_get_item_seconds: client_animations
                .animations
                .copper_golem_get_item_seconds(partial_ticks),
            copper_golem_get_no_item_seconds: client_animations
                .animations
                .copper_golem_get_no_item_seconds(partial_ticks),
            copper_golem_drop_item_seconds: client_animations
                .animations
                .copper_golem_drop_item_seconds(partial_ticks),
            copper_golem_drop_no_item_seconds: client_animations
                .animations
                .copper_golem_drop_no_item_seconds(partial_ticks),
            camel_jump_cooldown: client_animations
                .animations
                .camel_jump_cooldown(partial_ticks),
            sniffer_animation_id,
            sniffer_animation_seconds,
            // Vanilla `Sniffer.isSearching()`: gates the renderer's swap of the base walk for the
            // looping `SNIFFER_SNIFF_SEARCH` search-walk. `false` for every non-searching sniffer and
            // every other entity.
            sniffer_is_searching: client_animations.animations.sniffer_is_searching(),
            // Vanilla `Armadillo.shouldHideInShell` + the rollUp/rollOut transition timers: the
            // shell-ball hide window and the curl-in / un-curl elapsed seconds. `false`/`-1.0` for
            // every non-armadillo (only the armadillo is given a roll animation state).
            armadillo_is_hiding_in_shell: client_animations
                .animations
                .armadillo_is_hiding_in_shell(),
            armadillo_roll_up_seconds: client_animations
                .animations
                .armadillo_roll_up_seconds(partial_ticks),
            armadillo_roll_out_seconds: client_animations
                .animations
                .armadillo_roll_out_seconds(partial_ticks),
            armadillo_peek_seconds: client_animations
                .animations
                .armadillo_peek_seconds(partial_ticks),
            // Vanilla `Fox.getHeadRollAngle` / `getCrouchAmount`: the lerped client accumulators that
            // drive the head tilt and the crouch body drop. `0.0` for every non-fox (only the fox is
            // given a fox animation state).
            fox_head_roll_angle: client_animations
                .animations
                .fox_head_roll_angle(partial_ticks),
            fox_crouch_amount: client_animations
                .animations
                .fox_crouch_amount(partial_ticks),
            wolf_wet_shade: client_animations.animations.wolf_wet_shade(partial_ticks),
            wolf_shake_anim: client_animations.animations.wolf_shake_anim(partial_ticks),
            wolf_head_roll_angle: client_animations
                .animations
                .wolf_head_roll_angle(partial_ticks),
            fox_is_crouching,
            fox_is_sleeping,
            fox_is_sitting,
            fox_is_pouncing,
            fox_is_faceplanted,
            feline_is_crouching,
            feline_is_sprinting,
            feline_is_sitting,
            feline_lie_down_amount: client_animations
                .animations
                .feline_lie_down_amount(partial_ticks),
            feline_lie_down_amount_tail: client_animations
                .animations
                .feline_lie_down_amount_tail(partial_ticks),
            feline_relax_state_one_amount: client_animations
                .animations
                .feline_relax_state_one_amount(partial_ticks),
            feline_is_lying_on_top_of_sleeping_player: false,
            vex_charging,
            wither_invulnerable_ticks,
            is_crouching,
            // Vanilla `HumanoidMobRenderer.extractHumanoidRenderState` copies
            // `LivingEntity.elytraAnimationState.getRotX/Y/Z(partialTick)` into
            // `HumanoidRenderState`; `WingsLayer` consumes those rotations when
            // chest equipment renders a WINGS layer.
            elytra_rot_x: client_animations.animations.elytra_rot_x(partial_ticks),
            elytra_rot_y: client_animations.animations.elytra_rot_y(partial_ticks),
            elytra_rot_z: client_animations.animations.elytra_rot_z(partial_ticks),
            player_cape_flap,
            player_cape_lean,
            player_cape_lean2,
            player_left_shoulder_parrot,
            player_right_shoulder_parrot,
            show_extra_ears: false,
            is_auto_spin_attack,
            is_using_item,
            use_item_off_hand,
            is_upside_down,
            bounding_box_height,
            is_sleeping,
            sleeping_bed_yaw: None,
            sleeping_bed_offset: [0.0, 0.0],
            scale,
            swim_amount: client_animations
                .animations
                .living_swim_amount(partial_ticks),
            sheep_eat_animation_tick: client_animations.animations.sheep_eat_animation_tick(),
            goat_lower_head_tick: client_animations.animations.goat_lower_head_tick(),
            iron_golem_attack_ticks_remaining: client_animations
                .animations
                .iron_golem_attack_ticks_remaining(partial_ticks),
            iron_golem_offer_flower_tick: client_animations
                .animations
                .iron_golem_offer_flower_tick(),
            snow_golem_pumpkin,
            ravager_stunned_ticks_remaining: client_animations
                .animations
                .ravager_stunned_ticks_remaining(partial_ticks),
            ravager_attack_ticks_remaining: client_animations
                .animations
                .ravager_attack_ticks_remaining(partial_ticks),
            ravager_roar_animation: client_animations
                .animations
                .ravager_roar_animation(partial_ticks),
            hoglin_attack_animation_tick: client_animations
                .animations
                .hoglin_attack_animation_tick(),
            armor_stand_wiggle: client_animations
                .animations
                .armor_stand_wiggle(partial_ticks),
            polar_bear_stand_scale: client_animations
                .animations
                .polar_bear_stand_scale(partial_ticks),
            // Vanilla `Shulker.getClientPeekAmount(partialTick)`: the lerped client peek that
            // drives `ShulkerModel.setupAnim`'s lid open/close. `0.0` (closed/bind pose) for
            // every non-shulker (only the shulker is given a peek animation state).
            shulker_peek: client_animations
                .animations
                .shulker_peek_amount(partial_ticks),
            shulker_attach_face: shulker_attach_face(
                identity.entity_type_id,
                &metadata.data_values,
            ),
            // Spatial light is sampled by the WorldStore aggregation, which owns
            // the chunk light data; the per-entity source defaults to full bright.
            light: super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
            // `in_water` is likewise resolved by the WorldStore aggregation (it owns the
            // chunk fluid data); the per-entity source defaults to dry.
            in_water: false,
            // Vanilla `Entity.onGround()`: the last synced movement ground flag (default
            // `false` until a movement packet sets it).
            on_ground: transform.on_ground.unwrap_or(false),
            // Vanilla `DolphinRenderState.isMoving` (`getDeltaMovement().horizontalDistanceSqr() >
            // 1.0e-7`): projected from the synced velocity.
            is_moving: {
                let delta = transform.delta_movement;
                delta.x * delta.x + delta.z * delta.z > 1.0e-7
            },
            has_red_overlay: client_animations.animations.has_red_overlay(),
            death_time: client_animations.animations.death_time(partial_ticks),
            ender_dragon_death_time: client_animations
                .animations
                .ender_dragon_death_time(partial_ticks),
            creeper_swelling: client_animations.animations.creeper_swelling(partial_ticks),
            tendril_animation: client_animations
                .animations
                .warden_tendril_animation(partial_ticks),
            heart_animation: client_animations
                .animations
                .warden_heart_animation(partial_ticks),
            // Vanilla `Warden`'s four triggered combat keyframe one-shots (roar/sniff pose-driven,
            // attack/sonic-boom event-driven): the elapsed seconds since each started, or `-1.0`
            // when stopped (only the warden is given a combat animation state).
            warden_roar_seconds: client_animations
                .animations
                .warden_roar_seconds(partial_ticks),
            warden_sniff_seconds: client_animations
                .animations
                .warden_sniff_seconds(partial_ticks),
            warden_attack_seconds: client_animations
                .animations
                .warden_attack_seconds(partial_ticks),
            warden_sonic_boom_seconds: client_animations
                .animations
                .warden_sonic_boom_seconds(partial_ticks),
            warden_emerge_seconds: client_animations
                .animations
                .warden_emerge_seconds(partial_ticks),
            warden_dig_seconds: client_animations
                .animations
                .warden_dig_seconds(partial_ticks),
            rabbit_hop_seconds: client_animations
                .animations
                .rabbit_hop_seconds(partial_ticks),
            // Vanilla `Creaking`: the directly-synced `canMove()` walk gate plus the three triggered
            // keyframe one-shots (attack/invulnerable event-driven, death on the synced
            // `isTearingDown()`), each the elapsed seconds since it started or `-1.0` when stopped. The
            // `canMove` read is gated on the creaking type (its synced slot `16` is the `CAN_MOVE`
            // boolean; other entities' slot `16` is an unrelated field), defaulting to `true`.
            creaking_can_move: identity.entity_type_id != VANILLA_ENTITY_TYPE_CREAKING_ID
                || creaking_can_move(&metadata.data_values),
            creaking_attack_seconds: client_animations
                .animations
                .creaking_attack_seconds(partial_ticks),
            creaking_invulnerable_seconds: client_animations
                .animations
                .creaking_invulnerable_seconds(partial_ticks),
            creaking_death_seconds: client_animations
                .animations
                .creaking_death_seconds(partial_ticks),
            walk_animation_position,
            walk_animation_speed: client_animations
                .animations
                .walk_animation_speed(partial_ticks),
            worn_head_animation_position,
            // Vanilla `HumanoidRenderState.attackTime` / `ArmedEntityRenderState.mainArm` /
            // `attackArm`: the lerped melee swing progress, which arm is the main arm, and which arm
            // swings. Attack progress is `0.0` for an entity that is not mid-swing.
            attack_anim: client_animations.animations.attack_anim(partial_ticks),
            main_arm_left,
            attack_arm_off_hand: client_animations.animations.attack_arm_off_hand(),
            is_swinging: client_animations.animations.is_swinging(),
            // Vanilla `SquidRenderer.extractRenderState`: the lerped tentacle flex
            // angle plus body pitch/yaw/roll. `0.0` for every non-squid entity
            // (only the squid/glow squid is given a squid animation state).
            squid_tentacle_angle: client_animations
                .animations
                .squid_tentacle_angle(partial_ticks),
            squid_x_body_rot: client_animations.animations.squid_x_body_rot(partial_ticks),
            squid_y_body_rot,
            squid_z_body_rot: client_animations.animations.squid_z_body_rot(partial_ticks),
            // Vanilla `GuardianRenderer.extractRenderState`: the lerped tail-sway
            // phase. `0.0` for every non-guardian entity (only the guardian / elder
            // guardian is given a tail animation state).
            guardian_tail_animation: client_animations
                .animations
                .guardian_tail_animation(partial_ticks),
            // Vanilla `GuardianRenderState.spikesAnimation`: the lerped spike-withdrawal phase
            // (`withdrawal = (1 - it) · 0.55`). `1.0` (withdrawal `0`, fully extended) for every
            // non-guardian entity (only the guardian / elder guardian is given a spikes state).
            guardian_spikes_animation: client_animations
                .animations
                .guardian_spikes_animation(partial_ticks),
            // Vanilla `Breeze`'s pose-driven action one-shots (shoot/slide/slideBack/inhale/longJump):
            // the elapsed seconds since each started, or `-1.0` when stopped (only the breeze is given
            // a breeze animation state).
            breeze_shoot_seconds: client_animations
                .animations
                .breeze_shoot_seconds(partial_ticks),
            breeze_slide_seconds: client_animations
                .animations
                .breeze_slide_seconds(partial_ticks),
            breeze_slide_back_seconds: client_animations
                .animations
                .breeze_slide_back_seconds(partial_ticks),
            breeze_inhale_seconds: client_animations
                .animations
                .breeze_inhale_seconds(partial_ticks),
            breeze_long_jump_seconds: client_animations
                .animations
                .breeze_long_jump_seconds(partial_ticks),
            // Vanilla `ChickenRenderer.extractRenderState`: the lerped wing-flap
            // phase and amplitude. `0.0` for every non-chicken entity (only the
            // chicken is given a flap animation state).
            chicken_flap: client_animations.animations.chicken_flap(partial_ticks),
            chicken_flap_speed: client_animations
                .animations
                .chicken_flap_speed(partial_ticks),
            // Vanilla `SlimeRenderer.extractRenderState`: the lerped squish amount
            // driving the slime/magma-cube body's non-uniform stretch. `0.0` for
            // every other entity (only the slime/magma cube is given a squish state).
            slime_squish: client_animations.animations.slime_squish(partial_ticks),
            // Vanilla `EvokerFangsRenderer.extractRenderState`: the `0..1` bite ramp
            // driving the fang's jaw snap, rise, and vanish. `0.0` (hidden) for every
            // other entity and a fang whose attack has not started.
            evoker_fangs_bite_progress: client_animations
                .animations
                .evoker_fangs_bite_progress(partial_ticks),
            // Vanilla `AllayModel.setupAnim`: the dance state driven by the synced
            // `DATA_DANCING` flag. `dancing` gates the dance pose, `spinning` selects
            // the spin sub-pose, and `spinning_progress` is the `0..1` lerped spin
            // blend. All inert (`false`/`0.0`) for every non-allay entity.
            allay_dancing: client_animations.animations.allay_is_dancing(),
            allay_spinning: client_animations.animations.allay_is_spinning(),
            allay_spinning_progress: client_animations
                .animations
                .allay_spinning_progress(partial_ticks),
            allay_holding_item_progress: client_animations
                .animations
                .allay_holding_item_progress(partial_ticks),
            // Vanilla render-state `ticksUsingItem`, reconstructed from the shared charge/use counter.
            // Crossbow draw poses and player spear kinetic use sway consume it with their own durations.
            // `0.0` for anything not mid-draw/use.
            crossbow_charge_ticks: client_animations
                .animations
                .crossbow_charge_ticks_using_item(partial_ticks),
            // Vanilla `LivingEntityRenderState.ticksSinceKineticHitFeedback`
            // (`LivingEntity.getTicksSinceLastKineticHitFeedback(partialTicks)`): entity event `2`
            // starts the local feedback timer that `SpearAnimations.thirdPersonUseItem` consumes.
            ticks_since_kinetic_hit_feedback: client_animations
                .animations
                .ticks_since_kinetic_hit_feedback(partial_ticks),
            // Vanilla `AxolotlRenderer.extractRenderState`: the four `BinaryAnimator` factors
            // (`Axolotl.{playingDead,inWater,onGround,moving}Animator.getFactor`) that
            // `AdultAxolotlModel.setupAnim` blends into its swim / hover / crawl / lay-still /
            // play-dead sub-animations. All `0.0` for every non-axolotl entity.
            axolotl_playing_dead_factor: client_animations
                .animations
                .axolotl_playing_dead_factor(partial_ticks),
            axolotl_in_water_factor: client_animations
                .animations
                .axolotl_in_water_factor(partial_ticks),
            axolotl_on_ground_factor: client_animations
                .animations
                .axolotl_on_ground_factor(partial_ticks),
            axolotl_moving_factor: client_animations
                .animations
                .axolotl_moving_factor(partial_ticks),
            // Vanilla `ParrotRenderer.extractRenderState`: the lerped, combined
            // wing-flap angle. `0.0` for every non-parrot entity (only the parrot is
            // given a flap animation state).
            parrot_flap_angle: client_animations
                .animations
                .parrot_flap_angle(partial_ticks),
            parrot_party: false,
            // Vanilla `HumanoidArmorLayer`: the worn armor item in each armor slot resolved to its
            // equipment-asset material against the item registry map (threaded from the WorldStore).
            head_armor: armor_material(ProtocolEquipmentSlot::Head),
            chest_armor: armor_material(ProtocolEquipmentSlot::Chest),
            legs_armor: armor_material(ProtocolEquipmentSlot::Legs),
            feet_armor: armor_material(ProtocolEquipmentSlot::Feet),
            head_armor_dye: armor_dye(ProtocolEquipmentSlot::Head),
            chest_armor_dye: armor_dye(ProtocolEquipmentSlot::Chest),
            legs_armor_dye: armor_dye(ProtocolEquipmentSlot::Legs),
            feet_armor_dye: armor_dye(ProtocolEquipmentSlot::Feet),
            head_armor_foil: armor_foil(ProtocolEquipmentSlot::Head),
            chest_armor_foil: armor_foil(ProtocolEquipmentSlot::Chest),
            legs_armor_foil: armor_foil(ProtocolEquipmentSlot::Legs),
            feet_armor_foil: armor_foil(ProtocolEquipmentSlot::Feet),
            wolf_body_armor,
            wolf_body_armor_dye,
            wolf_body_armor_crackiness,
            wolf_body_armor_foil,
            pig_saddle: pig_saddle(),
            equine_saddle,
            equine_saddle_ridden,
            equine_animate_tail: client_animations.animations.equine_animate_tail(),
            equine_eat_animation: client_animations
                .animations
                .equine_eat_animation(partial_ticks),
            equine_stand_animation: client_animations
                .animations
                .equine_stand_animation(partial_ticks),
            equine_feeding_animation: client_animations
                .animations
                .equine_feeding_animation(partial_ticks),
            equine_body_armor: horse_body_armor,
            equine_body_armor_dye: horse_body_armor_dye,
            strider_ridden,
            strider_saddle,
            camel_saddle,
            camel_saddle_ridden,
            nautilus_saddle,
            guardian_beam: self.guardian_beam_source(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                client_animations.animations,
                position,
                partial_ticks,
            ),
            end_crystal_beam: self.end_crystal_beam_source(
                identity.entity_type_id,
                &metadata.data_values,
                position,
            ),
            ender_dragon_beam: self.ender_dragon_beam_source(
                identity.entity_type_id,
                position,
                partial_ticks,
            ),
            llama_body_decor,
            nautilus_body_armor,
            data_values: metadata.data_values.clone(),
        })
    }

    /// Vanilla `GuardianRenderer.extractRenderState` attack beam: when this entity is a guardian whose
    /// synced `DATA_ID_ATTACK_TARGET` names a live target, project the world eye→target vector and the
    /// attack timing. `position` is the guardian's interpolated feet position; the target is resolved
    /// cross-entity by its protocol id. `None` for a guardian with no target (or a missing target) and
    /// every non-guardian.
    #[allow(clippy::too_many_arguments)]
    fn guardian_beam_source(
        &self,
        entity_type_id: i32,
        add_entity_data: i32,
        data_values: &[bbb_protocol::packets::EntityDataValue],
        attributes: &[ProtocolAttributeSnapshot],
        client_animations: super::EntityClientAnimationState,
        position: super::EntityVec3,
        partial_ticks: f32,
    ) -> Option<super::GuardianBeamSource> {
        if !is_guardian_entity_type(entity_type_id) {
            return None;
        }
        let attack_target_id = guardian_attack_target_id(data_values);
        if attack_target_id == 0 {
            return None;
        }
        // The target may not be tracked client-side (out of range / not yet spawned); then no beam.
        let target_transform = self.transform(attack_target_id)?;
        let target_bounds = self.pick_bounds(attack_target_id)?;
        let eye_height = vanilla_eye_height_for_entity_data(
            entity_type_id,
            add_entity_data,
            data_values,
            attributes,
            Some(client_animations),
        )?;
        let attack = client_animations.guardian_attack.unwrap_or_default();
        let attack_duration = guardian_attack_duration(entity_type_id);
        // Vanilla `getPosition(target, getBbHeight() * 0.5)` is the target center; `getEyePosition()`
        // is the guardian eye. bbb uses the latest (un-interpolated) positions, so the lerp collapses.
        let target_half_height = f64::from((target_bounds.max[1] - target_bounds.min[1]) * 0.5);
        let eye_to_target = [
            (target_transform.position.x - position.x) as f32,
            (target_transform.position.y + target_half_height
                - (position.y + f64::from(eye_height))) as f32,
            (target_transform.position.z - position.z) as f32,
        ];
        Some(super::GuardianBeamSource {
            eye_to_target,
            eye_height,
            attack_time: attack.attack_time(partial_ticks),
            attack_scale: attack.attack_scale(partial_ticks, attack_duration),
        })
    }

    /// Vanilla `EndCrystalRenderer.extractRenderState`: when an end crystal has a synced
    /// `DATA_BEAM_TARGET`, project the target block center relative to the crystal's interpolated
    /// position. `None` for crystals without a target and every non-crystal entity.
    fn end_crystal_beam_source(
        &self,
        entity_type_id: i32,
        data_values: &[bbb_protocol::packets::EntityDataValue],
        position: super::EntityVec3,
    ) -> Option<super::EndCrystalBeamSource> {
        if entity_type_id != VANILLA_ENTITY_TYPE_END_CRYSTAL_ID {
            return None;
        }
        let target = data_values
            .iter()
            .find(|value| value.data_id == END_CRYSTAL_BEAM_TARGET_DATA_ID)
            .and_then(|value| match &value.value {
                EntityDataValueKind::OptionalBlockPos(Some(pos)) => Some(pos),
                _ => None,
            })?;
        Some(super::EndCrystalBeamSource {
            beam_offset: [
                (f64::from(target.x) + 0.5 - position.x) as f32,
                (f64::from(target.y) + 0.5 - position.y) as f32,
                (f64::from(target.z) + 0.5 - position.z) as f32,
            ],
        })
    }

    /// Vanilla `EnderDragon.checkCrystals` tracks the nearest end crystal from
    /// `getBoundingBox().inflate(32)`, and `EnderDragonRenderer.extractRenderState` projects that
    /// crystal's bobbed render position relative to the dragon position as `beamOffset`. The exact
    /// random 10-tick refresh cadence is not replayed client-side here; the rendered state is projected
    /// from the nearest currently tracked crystal intersecting the vanilla search box.
    fn ender_dragon_beam_source(
        &self,
        entity_type_id: i32,
        position: super::EntityVec3,
        partial_ticks: f32,
    ) -> Option<super::EnderDragonBeamSource> {
        if entity_type_id != VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
            return None;
        }

        let mut nearest: Option<(f64, [f32; 3])> = None;
        let mut query = self
            .ecs
            .query::<(&EntityIdentity, &EntityTransform, &EntityClientAnimations)>();
        for (_, (identity, transform, client_animations)) in query.iter() {
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_END_CRYSTAL_ID {
                continue;
            }
            if !end_crystal_intersects_ender_dragon_search_box(position, transform.position) {
                continue;
            }
            let dx = transform.position.x - position.x;
            let dy = transform.position.y - position.y;
            let dz = transform.position.z - position.z;
            let distance_sqr = dx * dx + dy * dy + dz * dz;
            if nearest.is_some_and(|(nearest_distance_sqr, _)| distance_sqr >= nearest_distance_sqr)
            {
                continue;
            }
            let crystal_age = client_animations.animations.age_ticks as f32 + partial_ticks;
            let crystal_y = transform.position.y + f64::from(end_crystal_renderer_y(crystal_age));
            nearest = Some((
                distance_sqr,
                [dx as f32, (crystal_y - position.y) as f32, dz as f32],
            ));
        }

        nearest.map(|(_, beam_offset)| super::EnderDragonBeamSource { beam_offset })
    }

    pub(crate) fn camera_pose_state(&self, id: i32) -> Option<EntityCameraPoseState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        let eye_height = vanilla_eye_height_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(client_animations.animations),
        )?;
        Some(EntityCameraPoseState {
            id: identity.id,
            position: transform.position,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            eye_height,
        })
    }

    fn living_entity_walk_animation_position(&self, id: i32, partial_ticks: f32) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if !vanilla_living_entity_type(identity.entity_type_id) {
            return None;
        }
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(
            client_animations
                .animations
                .walk_animation_position(partial_ticks),
        )
    }

    fn wither_head_targets_by_id(&self) -> HashMap<i32, [WitherHeadTargetRotations; 2]> {
        let mut eye_positions = HashMap::new();
        let mut withers = Vec::new();
        let mut query = self.ecs.query::<(
            &EntityIdentity,
            &EntityTransform,
            &EntityMetadata,
            &EntityAttributes,
            &EntityClientAnimations,
        )>();
        for (_, (identity, transform, metadata, attributes, client_animations)) in query.iter() {
            if let Some(eye_height) = vanilla_eye_height_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(client_animations.animations),
            ) {
                eye_positions.insert(identity.id, (transform.position, eye_height));
            }

            if vanilla_is_wither(identity.entity_type_id) {
                withers.push((
                    identity.id,
                    transform.position,
                    transform.y_rot,
                    vanilla_render_scale(identity.entity_type_id, &attributes.attributes),
                    wither_side_head_target_ids(&metadata.data_values),
                ));
            }
        }

        withers
            .into_iter()
            .map(|(id, position, y_body_rot, scale, target_ids)| {
                let mut targets = [
                    WitherHeadTargetRotations::fallback_to_body(y_body_rot),
                    WitherHeadTargetRotations::fallback_to_body(y_body_rot),
                ];
                for (head_index, target_id) in target_ids.into_iter().enumerate() {
                    if target_id <= 0 {
                        continue;
                    }
                    if let Some((target_position, target_eye_height)) =
                        eye_positions.get(&target_id).copied()
                    {
                        targets[head_index] = wither_side_head_target_rotation(
                            position,
                            y_body_rot,
                            scale,
                            head_index,
                            target_position,
                            target_eye_height,
                        );
                    }
                }
                (id, targets)
            })
            .collect()
    }

    pub(crate) fn mount(&self, id: i32) -> Option<EntityMount> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityMount>(entity)
            .ok()
            .map(|mount| (*mount).clone())
    }

    #[cfg(test)]
    pub(crate) fn leash(&self, id: i32) -> Option<EntityLeash> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityLeash>(entity)
            .ok()
            .map(|leash| *leash)
    }

    pub(crate) fn mob_effects(&self, id: i32) -> Option<EntityMobEffects> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityMobEffects>(entity)
            .ok()
            .map(|effects| (*effects).clone())
    }

    pub(crate) fn damage(&self, id: i32) -> Option<EntityDamage> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityDamage>(entity)
            .ok()
            .map(|damage| *damage)
    }

    #[cfg(test)]
    pub(crate) fn minecart_lerp(&self, id: i32) -> Option<EntityMinecartLerp> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityMinecartLerp>(entity)
            .ok()
            .map(|lerp| (*lerp).clone())
    }

    pub(crate) fn hurting_projectile(&self, id: i32) -> Option<EntityHurtingProjectile> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityHurtingProjectile>(entity)
            .ok()
            .map(|projectile| *projectile)
    }

    pub(crate) fn transform_states(&self) -> Vec<EntityTransformState> {
        let mut transforms = Vec::with_capacity(self.by_protocol_id.len());
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            if let Some(transform) = self.transform_state_for_entity(entity) {
                transforms.push(transform);
            }
        }
        transforms
    }

    pub(crate) fn item_entity_stacks(&self) -> Vec<ItemEntityStackState> {
        self.item_stacks_for_entity_types(&[VANILLA_ENTITY_TYPE_ITEM_ID])
    }

    pub(crate) fn item_stack_for_entity(&self, id: i32) -> Option<ItemStackSummary> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        item_entity_render_stack(&metadata.data_values).cloned()
    }

    /// The render state of every item-frame / glow-item-frame entity: its resolved wall center, the
    /// facing wall, the `0..=7` item rotation, glow/invisible flags, the framed item, and framed map id.
    /// Drives the 3D item-frame render (vanilla `ItemFrameRenderer`).
    pub(crate) fn item_frame_render_states(&self) -> Vec<ItemFrameRenderState> {
        let mut frames = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            if !ITEM_FRAME_ENTITY_TYPE_IDS.contains(&identity.entity_type_id) {
                continue;
            }
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            let Ok(metadata) = self.ecs.get::<&EntityMetadata>(entity) else {
                continue;
            };
            frames.push(ItemFrameRenderState {
                entity_id: identity.id,
                center: transform.position,
                facing: item_frame_facing(identity.data, &metadata.data_values),
                light: super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
                rotation: item_frame_rotation(&metadata.data_values),
                glow: identity.entity_type_id == VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
                invisible: self
                    .metadata_byte(identity.id, ENTITY_SHARED_FLAGS_DATA_ID, 0)
                    .unwrap_or(0)
                    & ENTITY_SHARED_FLAG_INVISIBLE
                    != 0,
                item: item_frame_item(&metadata.data_values).cloned(),
                map_id: item_frame_map_id(&metadata.data_values),
            });
        }
        frames
    }

    /// The item a humanoid entity holds in its main (`off_hand = false`) or off hand (vanilla
    /// `EntityEquipment` `MainHand`/`OffHand` slot), or `None` for an empty hand / no equipment. Drives
    /// the third-person held-item render.
    pub(crate) fn held_item(&self, id: i32, off_hand: bool) -> Option<ItemStackSummary> {
        let slot = if off_hand {
            ProtocolEquipmentSlot::OffHand
        } else {
            ProtocolEquipmentSlot::MainHand
        };
        self.equipment_item(id, slot)
    }

    /// The item in an arbitrary synced equipment slot, or `None` for an empty slot / no equipment.
    pub(crate) fn equipment_item(
        &self,
        id: i32,
        slot: ProtocolEquipmentSlot,
    ) -> Option<ItemStackSummary> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let equipment = self.ecs.get::<&EntityEquipment>(entity).ok()?;
        let item = equipment
            .equipment
            .iter()
            .find(|update| update.slot == slot)?
            .item
            .clone();
        item.item_id.map(|_| item)
    }

    /// Collects the `DATA_ITEM_STACK` carried by every entity whose type id is in `type_ids`. Used both
    /// for the dropped `item` entity (rendered as an item-sprite billboard) and for the thrown-item
    /// projectiles (snowball, egg, ender pearl, potions, …) that vanilla's `ThrownItemRenderer` draws as
    /// the same item sprite. The data id is shared (`VANILLA_ITEM_ENTITY_STACK_DATA_ID = 8`).
    pub(crate) fn item_stacks_for_entity_types(
        &self,
        type_ids: &[i32],
    ) -> Vec<ItemEntityStackState> {
        let mut items = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            if !type_ids.contains(&identity.entity_type_id) {
                continue;
            }
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            let Ok(metadata) = self.ecs.get::<&EntityMetadata>(entity) else {
                continue;
            };
            let Some(stack) = item_entity_render_stack(&metadata.data_values) else {
                continue;
            };
            items.push(ItemEntityStackState {
                entity_id: identity.id,
                position: transform.position,
                light: super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
                stack: stack.clone(),
            });
        }
        items
    }

    /// Collects the firework rocket item render state. Vanilla
    /// `FireworkRocketEntity.DATA_ID_FIREWORKS_ITEM` uses item-stack metadata id 8, followed by
    /// `DATA_ATTACHED_TO_TARGET` (id 9) and `DATA_SHOT_AT_ANGLE` (id 10).
    pub(crate) fn firework_rocket_item_states(&self) -> Vec<FireworkRocketItemState> {
        let mut items = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID {
                continue;
            }
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            let Ok(metadata) = self.ecs.get::<&EntityMetadata>(entity) else {
                continue;
            };
            if firework_rocket_attached_to_entity(&metadata.data_values) {
                continue;
            }
            let Some(stack) = item_entity_render_stack(&metadata.data_values) else {
                continue;
            };
            items.push(FireworkRocketItemState {
                entity_id: identity.id,
                position: transform.position,
                light: super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
                stack: stack.clone(),
                shot_at_angle: firework_rocket_shot_at_angle(&metadata.data_values),
            });
        }
        items
    }

    pub(crate) fn firework_rocket_empty_explosions_position(
        &self,
        id: i32,
    ) -> Option<super::EntityVec3> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID {
            return None;
        }
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let has_explosions = self
            .ecs
            .get::<&EntityMetadata>(entity)
            .ok()
            .and_then(|metadata| item_entity_render_stack(&metadata.data_values).cloned())
            .is_some_and(|stack| firework_rocket_stack_has_explosions(&stack));
        if has_explosions {
            return None;
        }
        Some(transform.position)
    }

    /// Collects the ominous item spawner item-cluster render state. Vanilla
    /// `OminousItemSpawner.DATA_ITEM` is the first accessor after `Entity` and therefore uses
    /// item-stack metadata id 8.
    pub(crate) fn ominous_item_spawner_item_states_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<OminousItemSpawnerItemState> {
        let mut items = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID {
                continue;
            }
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            let Ok(metadata) = self.ecs.get::<&EntityMetadata>(entity) else {
                continue;
            };
            let Ok(client_animations) = self.ecs.get::<&EntityClientAnimations>(entity) else {
                continue;
            };
            let Some(stack) = item_entity_render_stack(&metadata.data_values) else {
                continue;
            };
            items.push(OminousItemSpawnerItemState {
                entity_id: identity.id,
                position: transform.position,
                age_ticks: client_animations.animations.age_ticks as f32 + partial_ticks,
                stack: stack.clone(),
            });
        }
        items
    }

    #[cfg(test)]
    pub(crate) fn metadata(&self, id: i32) -> Option<EntityMetadata> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityMetadata>(entity)
            .ok()
            .map(|metadata| (*metadata).clone())
    }

    #[cfg(test)]
    pub(crate) fn equipment(&self, id: i32) -> Option<EntityEquipment> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityEquipment>(entity)
            .ok()
            .map(|equipment| (*equipment).clone())
    }

    #[cfg(test)]
    pub(crate) fn attributes(&self, id: i32) -> Option<EntityAttributes> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityAttributes>(entity)
            .ok()
            .map(|attributes| (*attributes).clone())
    }

    #[cfg(test)]
    pub(crate) fn transient_events(&self, id: i32) -> Option<EntityTransientEvents> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityTransientEvents>(entity)
            .ok()
            .map(|events| *events)
    }

    pub(crate) fn with_transform_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityTransform) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut transform = self.ecs.get::<&mut EntityTransform>(entity).ok()?;
        let result = update(&mut transform);
        Some(result)
    }

    pub(crate) fn with_metadata_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMetadata) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut metadata = self.ecs.get::<&mut EntityMetadata>(entity).ok()?;
        let result = update(&mut metadata);
        Some(result)
    }

    pub(crate) fn with_equipment_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityEquipment) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut equipment = self.ecs.get::<&mut EntityEquipment>(entity).ok()?;
        let result = update(&mut equipment);
        Some(result)
    }

    pub(crate) fn with_attributes_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityAttributes) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut attributes = self.ecs.get::<&mut EntityAttributes>(entity).ok()?;
        let result = update(&mut attributes);
        Some(result)
    }

    pub(crate) fn with_transient_events_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityTransientEvents) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut events = self.ecs.get::<&mut EntityTransientEvents>(entity).ok()?;
        let result = update(&mut events);
        Some(result)
    }

    pub(crate) fn with_mount_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMount) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut mount = self.ecs.get::<&mut EntityMount>(entity).ok()?;
        let result = update(&mut mount);
        Some(result)
    }

    pub(crate) fn with_leash_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityLeash) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut leash = self.ecs.get::<&mut EntityLeash>(entity).ok()?;
        let result = update(&mut leash);
        Some(result)
    }

    pub(crate) fn with_mob_effects_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMobEffects) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut effects = self.ecs.get::<&mut EntityMobEffects>(entity).ok()?;
        let result = update(&mut effects);
        Some(result)
    }

    pub(crate) fn with_damage_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityDamage) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut damage = self.ecs.get::<&mut EntityDamage>(entity).ok()?;
        let result = update(&mut damage);
        Some(result)
    }

    pub(crate) fn with_minecart_lerp_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMinecartLerp) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut lerp = self.ecs.get::<&mut EntityMinecartLerp>(entity).ok()?;
        let result = update(&mut lerp);
        Some(result)
    }

    pub(crate) fn with_hurting_projectile_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityHurtingProjectile) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut projectile = self.ecs.get::<&mut EntityHurtingProjectile>(entity).ok()?;
        let result = update(&mut projectile);
        Some(result)
    }

    pub(crate) fn sync_client_animation_targets_from_metadata(&mut self, id: i32) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?.clone();
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?.clone();
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations
            .animations
            .sync_targets_from_metadata(identity.entity_type_id, &metadata.data_values);
        Some(())
    }

    pub(crate) fn sync_client_animation_events_from_metadata_update(
        &mut self,
        id: i32,
        updated_values: &[bbb_protocol::packets::EntityDataValue],
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?.clone();
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?.clone();
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations.animations.sync_events_from_metadata_update(
            identity.entity_type_id,
            updated_values,
            &metadata.data_values,
        );
        Some(())
    }

    pub(crate) fn apply_client_animation_entity_event(
        &mut self,
        id: i32,
        event_id: i8,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let entity_type_id = self.ecs.get::<&EntityIdentity>(entity).ok()?.entity_type_id;
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations
            .animations
            .handle_entity_event(entity_type_id, event_id);
        Some(())
    }

    pub(crate) fn trigger_client_animation_hurt(&mut self, id: i32) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations.animations.trigger_hurt();
        Some(())
    }

    /// Arms the melee swing for `id` (vanilla `LivingEntity.swing`), triggered by the
    /// `ClientboundAnimate` packet's main-hand (`off_hand = false`) / off-hand actions.
    pub(crate) fn trigger_client_animation_swing(
        &mut self,
        id: i32,
        off_hand: bool,
        duration: i32,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations.animations.trigger_swing(off_hand, duration);
        Some(())
    }

    pub(crate) fn active_swing_off_hand(&self, id: i32) -> Option<bool> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        animations.animations.active_swing_off_hand()
    }

    pub(crate) fn attack_swing_state(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<LocalPlayerAttackSwingState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(LocalPlayerAttackSwingState {
            attack_anim: animations.animations.attack_anim(partial_ticks),
            off_hand: animations.animations.attack_arm_off_hand(),
        })
    }

    pub(crate) fn ticks_since_kinetic_hit_feedback(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(
            animations
                .animations
                .ticks_since_kinetic_hit_feedback(partial_ticks),
        )
    }

    pub(crate) fn refresh_client_animation_swing_duration(
        &mut self,
        id: i32,
        duration: i32,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations.animations.refresh_swing_duration(duration);
        Some(())
    }

    /// Gathers the world AABBs the per-entity `in_water` map is built from, for the
    /// entity types whose client-tick animation reads `isInWater()`
    /// ([`entity_animation_uses_in_water`]). Each tuple is `(entity_id, aabb_min,
    /// aabb_max)` with the AABB resolved from the SAME bounds source the projection
    /// uses ([`Self::pick_bounds`] → `vanilla_pick_bounds_for_entity_data`), so the
    /// fluid overlap matches the projected `in_water`. `WorldStore` (which holds the
    /// chunk/fluid data) turns each AABB into a `bool`.
    pub(crate) fn in_water_aabb_inputs(&self) -> Vec<(i32, [f64; 3], [f64; 3])> {
        let mut inputs = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            if !entity_animation_uses_in_water(identity.entity_type_id) {
                continue;
            }
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            let Some(bounds) = self.pick_bounds(identity.id) else {
                continue;
            };
            let aabb_min = [
                transform.position.x + f64::from(bounds.min[0]),
                transform.position.y + f64::from(bounds.min[1]),
                transform.position.z + f64::from(bounds.min[2]),
            ];
            let aabb_max = [
                transform.position.x + f64::from(bounds.max[0]),
                transform.position.y + f64::from(bounds.max[1]),
                transform.position.z + f64::from(bounds.max[2]),
            ];
            inputs.push((identity.id, aabb_min, aabb_max));
        }
        inputs
    }

    pub(crate) fn advance_client_animations(
        &mut self,
        ticks: u32,
        in_water_by_id: &HashMap<i32, bool>,
    ) {
        let wither_head_targets_by_id = self.wither_head_targets_by_id();
        for _ in 0..ticks {
            for (_, (identity, transform, mount, metadata, equipment, animations)) in
                self.ecs.query_mut::<(
                    &EntityIdentity,
                    &EntityTransform,
                    &EntityMount,
                    &EntityMetadata,
                    Option<&EntityEquipment>,
                    &mut EntityClientAnimations,
                )>()
            {
                // Vanilla `LivingEntity.calculateEntityAnimation` gates the limb
                // swing on `!isPassenger()` and scales the position by `3` for a
                // baby (`updateWalkAnimation`).
                let is_passenger = mount.vehicle_id.is_some();
                let is_baby = vanilla_is_baby(identity.entity_type_id, &metadata.data_values);
                let is_fall_flying = entity_is_fall_flying(&metadata.data_values);
                let is_crouching =
                    entity_data_pose(&metadata.data_values) == VANILLA_POSE_CROUCHING_ID;
                let is_swimming =
                    entity_data_pose(&metadata.data_values) == VANILLA_POSE_SWIMMING_ID;
                // The per-tick world fact (`isInWater()`) the world resolved before
                // this mutable pass, defaulting to `false` for non-consumers, and the
                // synced `Guardian.DATA_ID_MOVING` flag read from the metadata here.
                let in_water = in_water_by_id.get(&identity.id).copied().unwrap_or(false);
                let is_moving = guardian_is_moving(&metadata.data_values);
                let warden_heartbeat_delay = warden_heartbeat_delay(&metadata.data_values);
                let guardian_attack_target_id = guardian_attack_target_id(&metadata.data_values);
                let wither_head_targets = wither_head_targets_by_id.get(&identity.id).copied();
                let camel_is_standing =
                    entity_data_pose(&metadata.data_values) == VANILLA_POSE_STANDING_ID;
                let camel_is_dashing = camel_is_dashing(&metadata.data_values);
                let copper_golem_is_idle = copper_golem_is_idle(&metadata.data_values);
                let copper_golem_is_getting_item =
                    copper_golem_is_getting_item(&metadata.data_values);
                let copper_golem_is_getting_no_item =
                    copper_golem_is_getting_no_item(&metadata.data_values);
                let copper_golem_is_dropping_item =
                    copper_golem_is_dropping_item(&metadata.data_values);
                let copper_golem_is_dropping_no_item =
                    copper_golem_is_dropping_no_item(&metadata.data_values);
                let allay_is_dancing = allay_is_dancing(&metadata.data_values);
                let allay_has_item_in_hand = equipment.is_some_and(|equipment| {
                    equipment.equipment.iter().any(|update| {
                        update.slot == ProtocolEquipmentSlot::MainHand
                            && update.item.item_id.is_some()
                    })
                });
                let axolotl_is_playing_dead = axolotl_is_playing_dead(&metadata.data_values);
                let creaking_is_tearing_down = creaking_is_tearing_down(&metadata.data_values);
                let pillager_is_charging_crossbow =
                    pillager_is_charging_crossbow(&metadata.data_values);
                let piglin_is_charging_crossbow =
                    piglin_is_charging_crossbow(&metadata.data_values);
                let player_is_using_item = player_is_using_item(&metadata.data_values);
                let wolf_is_interested = wolf_is_interested(&metadata.data_values);
                let cat_is_lying = cat_is_lying(&metadata.data_values);
                let cat_relax_state_one = cat_relax_state_one(&metadata.data_values);
                let boat_paddles =
                    boat_paddle_states(&metadata.data_values, !mount.passengers.is_empty());
                let boat_bubble_time_value = boat_bubble_time(&metadata.data_values);
                animations.animations.advance_client_tick(
                    identity.entity_type_id,
                    identity.id,
                    *transform,
                    is_passenger,
                    is_baby,
                    is_fall_flying,
                    is_crouching,
                    in_water,
                    is_moving,
                    warden_heartbeat_delay,
                    guardian_attack_target_id,
                    wither_head_targets,
                    camel_is_standing,
                    camel_is_dashing,
                    copper_golem_is_idle,
                    copper_golem_is_getting_item,
                    copper_golem_is_getting_no_item,
                    copper_golem_is_dropping_item,
                    copper_golem_is_dropping_no_item,
                    allay_is_dancing,
                    allay_has_item_in_hand,
                    axolotl_is_playing_dead,
                    creaking_is_tearing_down,
                    pillager_is_charging_crossbow,
                    piglin_is_charging_crossbow,
                    player_is_using_item,
                    wolf_is_interested,
                    cat_is_lying,
                    cat_relax_state_one,
                    boat_paddles[0],
                    boat_paddles[1],
                    boat_bubble_time_value,
                    is_swimming,
                );
            }
            for (_, minecart_lerp) in self.ecs.query_mut::<&mut EntityMinecartLerp>() {
                minecart_lerp.advance_client_tick();
            }
        }
    }

    pub(crate) fn for_each_mount_mut(&mut self, mut update: impl FnMut(i32, &mut EntityMount)) {
        // Every entity spawns with an `EntityMount`, so iterating the ecs directly visits the same
        // set as `self.order` without cloning it. All callers apply an order-independent per-entity
        // mutation, so the archetype iteration order is not observable.
        for (_, (identity, mount)) in self.ecs.query_mut::<(&EntityIdentity, &mut EntityMount)>() {
            update(identity.id, mount);
        }
    }

    pub(crate) fn for_each_leash_mut(&mut self, mut update: impl FnMut(i32, &mut EntityLeash)) {
        let ids = self.order.clone();
        for id in ids {
            let _ = self.with_leash_mut(id, |leash| update(id, leash));
        }
    }

    pub(crate) fn states(&self) -> Vec<EntityState> {
        self.order
            .iter()
            .filter_map(|id| self.by_protocol_id.get(id).copied())
            .filter_map(|entity| self.project_entity(entity))
            .collect()
    }

    pub(crate) fn total_mob_effects(&self) -> usize {
        self.order
            .iter()
            .filter_map(|id| self.by_protocol_id.get(id).copied())
            .filter_map(|entity| self.ecs.get::<&EntityMobEffects>(entity).ok())
            .map(|effects| effects.effects.len())
            .sum()
    }

    pub(crate) fn total_minecart_lerp_steps(&self) -> usize {
        self.order
            .iter()
            .filter_map(|id| self.by_protocol_id.get(id).copied())
            .filter_map(|entity| self.ecs.get::<&EntityMinecartLerp>(entity).ok())
            .map(|lerp| lerp.steps.len())
            .sum()
    }

    pub(crate) fn len(&self) -> usize {
        self.by_protocol_id.len()
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn remove_ids(&mut self, ids: &[i32]) -> usize {
        let mut removed = 0;
        for id in ids {
            let Some(entity) = self.by_protocol_id.remove(id) else {
                continue;
            };
            let _ = self.ecs.despawn(entity);
            removed += 1;
        }
        if removed > 0 {
            self.order.retain(|id| self.by_protocol_id.contains_key(id));
        }
        removed
    }

    fn transform_state_for_entity(&self, entity: Entity) -> Option<EntityTransformState> {
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        Some(EntityTransformState::from_components(&identity, *transform))
    }

    fn sync_components_from_state(&mut self, entity: Entity, state: &EntityState) {
        if let Ok(mut identity) = self.ecs.get::<&mut EntityIdentity>(entity) {
            *identity = EntityIdentity::from(state);
        }
        if let Ok(mut transform) = self.ecs.get::<&mut EntityTransform>(entity) {
            *transform = EntityTransform::from(state);
        }
        if let Ok(mut metadata) = self.ecs.get::<&mut EntityMetadata>(entity) {
            *metadata = EntityMetadata::from(state);
        }
        if let Ok(mut equipment) = self.ecs.get::<&mut EntityEquipment>(entity) {
            *equipment = EntityEquipment::from(state);
        }
        if let Ok(mut attributes) = self.ecs.get::<&mut EntityAttributes>(entity) {
            *attributes = EntityAttributes::from(state);
        }
        if let Ok(mut events) = self.ecs.get::<&mut EntityTransientEvents>(entity) {
            *events = EntityTransientEvents::from(state);
        }
        if let Ok(mut mount) = self.ecs.get::<&mut EntityMount>(entity) {
            *mount = EntityMount::from(state);
        }
        if let Ok(mut leash) = self.ecs.get::<&mut EntityLeash>(entity) {
            *leash = EntityLeash::from(state);
        }
        if let Ok(mut effects) = self.ecs.get::<&mut EntityMobEffects>(entity) {
            *effects = EntityMobEffects::from(state);
        }
        if let Ok(mut animations) = self.ecs.get::<&mut EntityClientAnimations>(entity) {
            *animations = EntityClientAnimations::from(state);
        }
        if let Ok(mut damage) = self.ecs.get::<&mut EntityDamage>(entity) {
            *damage = EntityDamage::from(state);
        }
        // Keep `EntityMinecartLerp` attached only while the entity is a vanilla minecart. A re-add
        // packet can in principle change an existing id's type, so mirror
        // `sync_hurting_projectile_from_state` and insert / update / remove the component to match.
        if is_vanilla_minecart_type(state.entity_type_id) {
            let updated = {
                if let Ok(mut lerp) = self.ecs.get::<&mut EntityMinecartLerp>(entity) {
                    *lerp = EntityMinecartLerp::from(state);
                    true
                } else {
                    false
                }
            };
            if !updated {
                let _ = self.ecs.insert_one(entity, EntityMinecartLerp::from(state));
            }
        } else {
            let _ = self.ecs.remove_one::<EntityMinecartLerp>(entity);
        }
        self.sync_hurting_projectile_from_state(entity, state);
    }

    fn sync_hurting_projectile_from_state(&mut self, entity: Entity, state: &EntityState) {
        if let Some(projectile) =
            entity_hurting_projectile_from_state(state.entity_type_id, state.hurting_projectile)
        {
            let updated = {
                if let Ok(mut existing) = self.ecs.get::<&mut EntityHurtingProjectile>(entity) {
                    *existing = projectile;
                    true
                } else {
                    false
                }
            };
            if !updated {
                let _ = self.ecs.insert_one(entity, projectile);
            }
        } else {
            let _ = self.ecs.remove_one::<EntityHurtingProjectile>(entity);
        }
    }

    fn project_entity(&self, entity: Entity) -> Option<EntityState> {
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let equipment = self.ecs.get::<&EntityEquipment>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let events = self.ecs.get::<&EntityTransientEvents>(entity).ok()?;
        let mount = self.ecs.get::<&EntityMount>(entity).ok()?;
        let leash = self.ecs.get::<&EntityLeash>(entity).ok()?;
        let effects = self.ecs.get::<&EntityMobEffects>(entity).ok()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        let damage = self.ecs.get::<&EntityDamage>(entity).ok()?;
        let minecart_lerp = self.ecs.get::<&EntityMinecartLerp>(entity).ok();
        let hurting_projectile = self.ecs.get::<&EntityHurtingProjectile>(entity).ok();

        let mut state = EntityState {
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
            client_animations: Default::default(),
            last_damage: None,
            minecart_lerp_steps: Vec::new(),
            minecart_lerp_old_step: None,
            minecart_lerp_delay: 0,
            hurting_projectile: None,
        };
        (*transform).write_to_state(&mut state);
        (*metadata).clone().write_to_state(&mut state);
        (*equipment).clone().write_to_state(&mut state);
        (*attributes).clone().write_to_state(&mut state);
        (*events).write_to_state(&mut state);
        (*mount).clone().write_to_state(&mut state);
        (*leash).write_to_state(&mut state);
        (*effects).clone().write_to_state(&mut state);
        (*client_animations).write_to_state(&mut state);
        (*damage).write_to_state(&mut state);
        if let Some(minecart_lerp) = minecart_lerp {
            (*minecart_lerp).clone().write_to_state(&mut state);
        }
        if let Some(projectile) = hurting_projectile {
            (*projectile).write_to_state(&mut state);
        }
        Some(state)
    }
}

impl Default for EntityStore {
    fn default() -> Self {
        Self {
            ecs: World::new(),
            by_protocol_id: BTreeMap::new(),
            order: Vec::new(),
        }
    }
}

impl Clone for EntityStore {
    fn clone(&self) -> Self {
        // Clone component-by-component instead of round-tripping every entity
        // through a full `EntityState` projection; this runs on every control
        // snapshot publish.
        let mut store = Self::default();
        for (&id, &entity) in &self.by_protocol_id {
            let (
                Ok(identity),
                Ok(transform),
                Ok(metadata),
                Ok(equipment),
                Ok(attributes),
                Ok(events),
                Ok(mount),
                Ok(leash),
                Ok(effects),
                Ok(animations),
                Ok(damage),
            ) = (
                self.ecs.get::<&EntityIdentity>(entity),
                self.ecs.get::<&EntityTransform>(entity),
                self.ecs.get::<&EntityMetadata>(entity),
                self.ecs.get::<&EntityEquipment>(entity),
                self.ecs.get::<&EntityAttributes>(entity),
                self.ecs.get::<&EntityTransientEvents>(entity),
                self.ecs.get::<&EntityMount>(entity),
                self.ecs.get::<&EntityLeash>(entity),
                self.ecs.get::<&EntityMobEffects>(entity),
                self.ecs.get::<&EntityClientAnimations>(entity),
                self.ecs.get::<&EntityDamage>(entity),
            )
            else {
                continue;
            };
            let cloned = store.ecs.spawn((
                (*identity).clone(),
                *transform,
                (*metadata).clone(),
                (*equipment).clone(),
                (*attributes).clone(),
                *events,
                (*mount).clone(),
                *leash,
                (*effects).clone(),
                *animations,
                *damage,
            ));
            if let Ok(lerp) = self.ecs.get::<&EntityMinecartLerp>(entity) {
                let _ = store.ecs.insert_one(cloned, (*lerp).clone());
            }
            if let Ok(projectile) = self.ecs.get::<&EntityHurtingProjectile>(entity) {
                let _ = store.ecs.insert_one(cloned, *projectile);
            }
            store.by_protocol_id.insert(id, cloned);
        }
        store.order = self.order.clone();
        store
    }
}

impl fmt::Debug for EntityStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let states = self.states();
        f.debug_struct("EntityStore")
            .field("entities", &states)
            .finish()
    }
}

fn default_minecart_display_block_state(
    entity_type_id: i32,
    furnace_has_fuel: bool,
    registries: &RegistrySet,
) -> Option<EntityBlockModelState> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID => registered_block_model_state(
            registries,
            "minecraft:chest",
            BTreeMap::from([
                ("facing".to_string(), "north".to_string()),
                ("type".to_string(), "single".to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]),
        ),
        VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID => registered_block_model_state(
            registries,
            "minecraft:command_block",
            BTreeMap::from([
                ("conditional".to_string(), "false".to_string()),
                ("facing".to_string(), "north".to_string()),
            ]),
        ),
        VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID => registered_block_model_state(
            registries,
            "minecraft:furnace",
            BTreeMap::from([
                ("facing".to_string(), "north".to_string()),
                ("lit".to_string(), furnace_has_fuel.to_string()),
            ]),
        ),
        VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID => registered_block_model_state(
            registries,
            "minecraft:hopper",
            BTreeMap::from([
                ("enabled".to_string(), "true".to_string()),
                ("facing".to_string(), "down".to_string()),
            ]),
        ),
        VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID => {
            registered_block_model_state(registries, "minecraft:spawner", BTreeMap::new())
        }
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID => registered_block_model_state(
            registries,
            "minecraft:tnt",
            BTreeMap::from([("unstable".to_string(), "false".to_string())]),
        ),
        VANILLA_ENTITY_TYPE_MINECART_ID => None,
        _ => None,
    }
}

fn default_primed_tnt_block_state(registries: &RegistrySet) -> Option<EntityBlockModelState> {
    registered_block_model_state(
        registries,
        "minecraft:tnt",
        BTreeMap::from([("unstable".to_string(), "false".to_string())]),
    )
}

fn registered_block_model_state(
    registries: &RegistrySet,
    name: &str,
    properties: BTreeMap<String, String>,
) -> Option<EntityBlockModelState> {
    registries.block_state_id_by_name_and_properties(name, &properties)?;
    Some(EntityBlockModelState {
        name: name.to_string(),
        properties,
    })
}

fn default_minecart_display_offset(entity_type_id: i32) -> i32 {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID => 8,
        VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID => 1,
        _ => DEFAULT_MINECART_DISPLAY_OFFSET,
    }
}

fn entity_block_pos(position: super::EntityVec3) -> crate::BlockPos {
    crate::BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

fn block_pos_dist_sqr(left: crate::BlockPos, right: crate::BlockPos) -> f64 {
    let dx = f64::from(left.x) - f64::from(right.x);
    let dy = f64::from(left.y) - f64::from(right.y);
    let dz = f64::from(left.z) - f64::from(right.z);
    dx * dx + dy * dy + dz * dz
}

fn add_entity_vec3(left: super::EntityVec3, right: super::EntityVec3) -> super::EntityVec3 {
    super::EntityVec3 {
        x: left.x + right.x,
        y: left.y + right.y,
        z: left.z + right.z,
    }
}

fn sub_entity_vec3(left: super::EntityVec3, right: super::EntityVec3) -> super::EntityVec3 {
    super::EntityVec3 {
        x: left.x - right.x,
        y: left.y - right.y,
        z: left.z - right.z,
    }
}

fn lerp_entity_vec3(
    alpha: f32,
    from: super::EntityVec3,
    to: super::EntityVec3,
) -> super::EntityVec3 {
    let alpha = f64::from(alpha);
    super::EntityVec3 {
        x: from.x + (to.x - from.x) * alpha,
        y: from.y + (to.y - from.y) * alpha,
        z: from.z + (to.z - from.z) * alpha,
    }
}

fn vanilla_attribute_value(attribute: &ProtocolAttributeSnapshot) -> f64 {
    let mut base = attribute.base;
    for modifier in &attribute.modifiers {
        if modifier.operation_id != 1 && modifier.operation_id != 2 {
            base += modifier.amount;
        }
    }

    let mut result = base;
    for modifier in &attribute.modifiers {
        if modifier.operation_id == 1 {
            result += base * modifier.amount;
        }
    }
    for modifier in &attribute.modifiers {
        if modifier.operation_id == 2 {
            result *= 1.0 + modifier.amount;
        }
    }
    result
}

fn shulker_attach_face(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityAttachmentFace {
    if entity_type_id != VANILLA_ENTITY_TYPE_SHULKER_ID {
        return EntityAttachmentFace::Down;
    }
    data_values
        .iter()
        .find_map(|value| {
            if value.data_id != SHULKER_ATTACH_FACE_DATA_ID {
                return None;
            }
            let EntityDataValueKind::Direction(direction) = &value.value else {
                return None;
            };
            Some(EntityAttachmentFace::from_3d_data(*direction))
        })
        .unwrap_or_default()
}

fn item_entity_render_stack(
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> Option<&bbb_protocol::packets::ItemStackSummary> {
    data_values.iter().find_map(|value| {
        if value.data_id != VANILLA_ITEM_ENTITY_STACK_DATA_ID {
            return None;
        }
        let EntityDataValueKind::ItemStack(stack) = &value.value else {
            return None;
        };
        if stack.item_id.is_some() && stack.count > 0 {
            Some(stack)
        } else {
            None
        }
    })
}

fn firework_rocket_attached_to_entity(
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    data_values.iter().any(|value| {
        value.data_id == FIREWORK_ROCKET_ATTACHED_TO_TARGET_DATA_ID
            && matches!(
                &value.value,
                EntityDataValueKind::OptionalUnsignedInt(Some(_))
            )
    })
}

fn firework_rocket_shot_at_angle(data_values: &[bbb_protocol::packets::EntityDataValue]) -> bool {
    data_values.iter().any(|value| {
        value.data_id == FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID
            && matches!(&value.value, EntityDataValueKind::Boolean(true))
    })
}

fn firework_rocket_stack_has_explosions(stack: &ItemStackSummary) -> bool {
    stack
        .component_patch
        .fireworks_explosions_count
        .unwrap_or(stack.component_patch.fireworks_explosions.len())
        > 0
}

impl Serialize for EntityStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.states().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EntityStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let states = Vec::<EntityState>::deserialize(deserializer)?;
        let mut store = EntityStore::default();
        for state in states {
            store.insert_or_replace(state);
        }
        Ok(store)
    }
}
