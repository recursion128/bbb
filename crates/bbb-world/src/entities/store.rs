use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
    time::{Duration, Instant},
};

use bbb_protocol::attribute_modifier_types::{
    VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
    VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
};
use bbb_protocol::attribute_types::{vanilla_attribute_type, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID};
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID, VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
};
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_FALL_FLYING_ID,
    VANILLA_ENTITY_POSE_SLEEPING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
    VANILLA_ENTITY_POSE_SWIMMING_ID,
};
use bbb_protocol::packets::{
    AttributeModifier as ProtocolAttributeModifier, AttributeSnapshot as ProtocolAttributeSnapshot,
};
use hecs::{Entity, World};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use bbb_protocol::entity_types::{
    vanilla_entity_type, vanilla_entity_type_dimensions, VANILLA_ENTITY_TYPE_BLAZE_ID,
    VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
    VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_CARROT_ON_A_STICK_ID, VANILLA_ITEM_DRAGON_HEAD_ID, VANILLA_ITEM_FISHING_ROD_ID,
    VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
};
use bbb_protocol::packets::EquipmentSlot as ProtocolEquipmentSlot;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::packets::{
    EntityDataRegistryHolder, EntityDataRegistryReference,
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
};

use super::components::{
    EntityBoatMovement, EntityClientMovementCollisionFacts, EntityClientMovementFluidFacts,
    EntityExperienceOrbMovement, EntityExperienceOrbPlayerTargetFacts, EntityFishingHookMovement,
    EntityIncarnation,
};
use super::display::{display_entity_culling_state, display_entity_render_source};
use super::renderer_catalog::vanilla_living_entity_renderer_name_is_upside_down;
use super::schema::{
    vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type,
    vanilla_entity_metadata_accessor_for_entity_type,
    vanilla_entity_metadata_bat_flags_accessor_for_entity_type,
    vanilla_entity_metadata_bee_accessors_for_entity_type,
    vanilla_entity_metadata_camel_dash_accessor_for_entity_type,
    vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type,
    vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type,
    vanilla_entity_metadata_end_crystal_accessors_for_entity_type,
    vanilla_entity_metadata_enderman_render_accessors_for_entity_type,
    vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type,
    vanilla_entity_metadata_firework_rocket_accessors_for_entity_type,
    vanilla_entity_metadata_fox_accessors_for_entity_type,
    vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type,
    vanilla_entity_metadata_hanging_direction_accessor_for_entity_type,
    vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type,
    vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type,
    vanilla_entity_metadata_item_frame_accessors_for_entity_type,
    vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type,
    vanilla_entity_metadata_player_absorption_accessor_for_entity_type,
    vanilla_entity_metadata_primed_tnt_accessors_for_entity_type,
    vanilla_entity_metadata_shulker_accessors_for_entity_type,
    vanilla_entity_metadata_slime_size_accessor_for_entity_type,
    vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type,
    vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type,
    vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type,
    vanilla_entity_metadata_tamable_animal_accessors_for_entity_type,
    vanilla_entity_metadata_type_collidability_kind, vanilla_entity_metadata_type_is_camel,
    vanilla_entity_metadata_type_is_guardian, vanilla_entity_metadata_type_is_slime,
    vanilla_entity_metadata_type_pushability_kind,
    vanilla_entity_metadata_type_uses_player_entity_event_handler,
    vanilla_entity_metadata_vex_flags_accessor_for_entity_type,
    vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type,
    vanilla_entity_type_default_attribute, VanillaEntityCollidabilityKind,
    VanillaEntityPushabilityKind, VanillaFireworkRocketMetadataAccessors,
    VANILLA_FISHING_HOOK_BITING_DATA_ID, VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID,
};

use super::{
    calculate_view_vector, is_vanilla_abstract_nautilus_type, is_vanilla_boat_type,
    is_vanilla_can_wear_horse_armor_type, is_vanilla_llama_type, is_vanilla_minecart_type,
    is_vanilla_vehicle_entity_type, vanilla_entity_event_kind, vanilla_entity_type_is_squid,
    ArmorMaterialKind, BlazeAmbientParticleState, DisplayEntityRenderSourceState,
    EnderAmbientParticleState, EntityAttachmentFace, EntityAttributes,
    EntityBlockEffectBaseTickReset, EntityBlockEffectMovement, EntityBlockEffectMovementIter,
    EntityBlockEffectQuery, EntityBlockModelState, EntityCameraHurtState, EntityCameraPoseState,
    EntityClientAnimationState, EntityClientAnimations, EntityClientMovementEffectState,
    EntityClientMovementParticleState, EntityClientMovementSimpleParticleKind,
    EntityClientMovementSimpleParticleState, EntityDamage, EntityDamageEventState, EntityEquipment,
    EntityHurtingProjectile, EntityIdentity, EntityInterpolationCollisionInput, EntityLeash,
    EntityMetadata, EntityMinecartCommandBlock, EntityMinecartLerp, EntityMobEffects,
    EntityModelSourceState, EntityMount, EntityProjectileLifecycle, EntityState,
    EntitySupportingBlock, EntityTransform, EntityTransformState, EntityTransientEvents,
    EntityVec3, EvokerFangsCritParticleState, FallingBlockModelState,
    FireworkRocketExplosionParticleState, FireworkRocketItemState, FishingHookRenderSourceState,
    FoxEatParticleState, GuardianAmbientParticleState, GuardianAttackParticleState,
    HappyGhastHarnessColor, ItemEntityStackState, ItemFrameRenderState, LlamaBodyDecorColor,
    LocalPlayerAttackSwingState, MinecartDisplayBlockState, OminousItemSpawnerItemState,
    OminousItemSpawnerParticleState, PowderSnowFlakeParticleState, RavagerRoarParticleState,
    RavagerStunParticleState, SlimeLandingParticleState, VanillaEntityBlockEffectTickPolicy,
    WolfArmorCrackiness, VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
    VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH, VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
    VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID, VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
    VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
    VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID,
    VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
    VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID, VANILLA_ENTITY_TYPE_PANDA_ID,
    VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID,
    VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
    VANILLA_MAX_AIR_SUPPLY_TICKS,
};
use crate::entities::animations::{
    allay_is_dancing, axolotl_is_playing_dead, bat_is_resting, boat_bubble_time,
    boat_paddle_states, camel_is_dashing, cat_is_lying, cat_relax_state_one,
    copper_golem_animation_flags, creaking_can_move, creaking_is_tearing_down,
    entity_animation_uses_in_water, entity_chunk_coordinate, entity_data_bool, entity_data_int,
    entity_data_long, entity_is_fall_flying, entity_is_swimming, guardian_attack_duration,
    guardian_attack_target_id, guardian_is_moving, piglin_is_charging_crossbow,
    pillager_is_charging_crossbow, player_is_using_item, sniffer_can_play_digging_sound,
    warden_heartbeat_delay, wither_side_head_target_ids, wither_side_head_target_rotation,
    wolf_is_interested, EntityClientTickParticleTriggers, GuardianAttackTargetTickInput,
    RiddenJumpState, WitherHeadTargetRotations,
};
use crate::entities::dimensions::{
    entity_data_pose, item_frame_facing, item_frame_item, item_frame_map_id, item_frame_rotation,
    vanilla_age_scale, vanilla_body_anchor_y_offset_for_entity_data,
    vanilla_bounds_for_entity_data, vanilla_camel_leash_offset_for_entity_data,
    vanilla_client_position_for_entity_data, vanilla_eye_height_for_entity_data, vanilla_is_baby,
    vanilla_is_cat, vanilla_living_entity_type,
    vanilla_local_player_passenger_position_for_entity_data,
    vanilla_model_source_bounds_for_entity_data, vanilla_passenger_position_for_entity_data,
    vanilla_passenger_vehicle_debug_target_for_entity_data, vanilla_pick_bounds_for_entity_data,
    vanilla_render_scale, vanilla_shulker_attached_position,
};
use crate::entities::dragon::{
    ender_dragon_parent_pick_target, ender_dragon_part_parent_id,
    ender_dragon_part_pick_targets_at_partial_tick,
};
use crate::entities::projectiles::{
    entity_hurting_projectile_from_state, entity_projectile_lifecycle_from_state,
};
use crate::entities::renderer_catalog::{
    vanilla_entity_renderer_extracts_casting_spell,
    vanilla_entity_renderer_extracts_mob_aggressive,
    vanilla_entity_renderer_extracts_unhappy_counter, vanilla_entity_renderer_minimum_block_light,
    vanilla_entity_renderer_uses_abstract_feline_model,
    vanilla_entity_renderer_uses_abstract_horse_renderer,
    vanilla_entity_renderer_uses_mob_item_in_hand_layer,
};
use crate::entities::schema::{
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_happy_ghast_accessors_for_entity_type,
    vanilla_entity_metadata_leash_offset_owner,
    vanilla_entity_metadata_living_entity_accessors_for_entity_type,
    vanilla_entity_metadata_mob_accessors_for_entity_type,
    vanilla_entity_metadata_rope_hold_position_owner,
    vanilla_entity_metadata_type_interpolation_steps,
    vanilla_entity_metadata_type_is_abstract_piglin, vanilla_entity_metadata_type_is_living,
    vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer,
    VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION, VANILLA_AVATAR_MAIN_HAND_DATA_ID,
    VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
    VANILLA_AVATAR_MODEL_PART_CAPE as VANILLA_AVATAR_CAPE_PART_MASK, VANILLA_HUMANOID_ARM_LEFT_ID,
    VANILLA_HUMANOID_ARM_RIGHT_ID, VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID,
    VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID,
};
use crate::registries::{armor_trim_sprite, ArmorTrimLayer, RegistrySet};
use crate::ItemEquipmentSlot;
use crate::LocalPlayerPoseState;
use bbb_protocol::entity_types::*;

fn avatar_flight_y_rotation(
    previous_delta_movement: EntityVec3,
    delta_movement: EntityVec3,
    look_angle: EntityVec3,
    partial_ticks: f32,
) -> (bool, f32) {
    let partial_ticks = f64::from(partial_ticks);
    let movement_x =
        previous_delta_movement.x + (delta_movement.x - previous_delta_movement.x) * partial_ticks;
    let movement_z =
        previous_delta_movement.z + (delta_movement.z - previous_delta_movement.z) * partial_ticks;
    let movement_length_squared = movement_x * movement_x + movement_z * movement_z;
    let look_length_squared = look_angle.x * look_angle.x + look_angle.z * look_angle.z;
    if movement_length_squared <= 1.0e-5 || look_length_squared <= 1.0e-5 {
        return (false, 0.0);
    }

    let movement_length = movement_length_squared.sqrt();
    let look_length = look_length_squared.sqrt();
    let dot = movement_x / movement_length * (look_angle.x / look_length)
        + movement_z / movement_length * (look_angle.z / look_length);
    let cross = movement_x * look_angle.z - movement_z * look_angle.x;
    (true, (cross.signum() * dot.abs().min(1.0).acos()) as f32)
}

fn within_death_tick_distance(
    position: super::EntityVec3,
    player_chunk: Option<(i32, i32)>,
    simulation_distance: Option<i32>,
) -> bool {
    let (Some((player_chunk_x, player_chunk_z)), Some(simulation_distance)) =
        (player_chunk, simulation_distance)
    else {
        return true;
    };
    let entity_chunk_x = entity_chunk_coordinate(position.x);
    let entity_chunk_z = entity_chunk_coordinate(position.z);
    let x_distance = (i64::from(entity_chunk_x) - i64::from(player_chunk_x)).abs();
    let z_distance = (i64::from(entity_chunk_z) - i64::from(player_chunk_z)).abs();
    x_distance.max(z_distance) <= i64::from(simulation_distance)
}

/// Vanilla `Entity.getTicksRequiredToFreeze()`: the powder-snow freeze threshold
/// at which `isFullyFrozen()` becomes true and the body starts shaking.
const VANILLA_TICKS_REQUIRED_TO_FREEZE: i32 = 140;
/// Vanilla `ItemTags.HARNESSES`, consumed by `RopesLayer` independently of equipment-asset lookup.
const VANILLA_HAPPY_GHAST_HARNESSES_ITEM_TAG: &str = "minecraft:harnesses";

/// Vanilla 26.1 `EntityType.PIG` registry id, used to gate `PigRenderState.saddle`.
/// Vanilla 26.1 `EntityType.WOLF` registry id, used to gate `WolfArmorLayer`.
/// Vanilla 26.1 skeleton-family ids whose renderers extend `HumanoidMobRenderer`
/// and therefore feed `ItemInHandLayer` through `getItemHeldByArm`.
/// Vanilla 26.1 `EvokerRenderer` installs an `ItemInHandLayer` over the illager model.
/// Vanilla 26.1 `ZombifiedPiglinRenderer` extends `HumanoidMobRenderer`.
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

fn vanilla_camel_saddle_type(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_is_camel(entity_type_id)
}

fn equipment_saddle_item<'a>(
    equipment: Option<&'a EntityEquipment>,
    equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
) -> Option<&'a ItemStackSummary> {
    let item = equipment?
        .equipment
        .iter()
        .find(|update| update.slot == ProtocolEquipmentSlot::Saddle)
        .map(|update| &update.item)?;
    if item.count <= 0 {
        return None;
    }
    let item_id = item.item_id?;
    (equipment_slots.get(&item_id).copied() == Some(ItemEquipmentSlot::Saddle)).then_some(item)
}

fn equipment_contains_saddle_item(
    equipment: Option<&EntityEquipment>,
    equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
) -> bool {
    equipment_saddle_item(equipment, equipment_slots).is_some()
}

fn upsert_metadata_value(metadata: &mut EntityMetadata, value: ProtocolEntityDataValue) {
    if metadata
        .data_values
        .iter()
        .any(|existing| *existing == value)
    {
        return;
    }
    if let Some(existing) = metadata
        .data_values
        .iter_mut()
        .find(|existing| existing.data_id == value.data_id)
    {
        *existing = value;
    } else {
        metadata.data_values.push(value);
        metadata.data_values.sort_by_key(|value| value.data_id);
    }
}

/// Vanilla `WitherBoss.DATA_ID_INV` data id (19): the int spawn-invulnerability countdown
/// (`getInvulnerableTicks`), the fourth `WitherBoss` accessor after `Mob.DATA_MOB_FLAGS_ID` (15) and
/// the three `DATA_TARGET_A/B/C` ints (16..=18).
pub(crate) struct EntityStore {
    pub(super) ecs: World,
    pub(super) by_protocol_id: BTreeMap<i32, Entity>,
    pub(super) protocol_id_by_uuid: HashMap<Uuid, i32>,
    pub(super) order: Vec<i32>,
    next_incarnation: u64,
    pending_ravager_stun_particles: Vec<RavagerStunParticleState>,
    pending_evoker_fangs_crit_particles: Vec<EvokerFangsCritParticleState>,
    pub(super) pending_entity_client_movement_effects: Vec<EntityClientMovementEffectState>,
    pending_ominous_item_spawner_particles: Vec<OminousItemSpawnerParticleState>,
    client_tick_scratch: EntityClientTickScratch,
    publication_snapshot_observation: EntityPublicationSnapshotObservation,
}

#[derive(Default)]
struct EntityClientTickScratch {
    scheduled_set: HashSet<i32>,
    scheduled_entities: Vec<Entity>,
    player_passenger_ancestors: HashSet<i32>,
    guardian_dry_jump_entities: Vec<i32>,
    camel_clear_dash_entities: Vec<i32>,
    camel_stand_instantly_entities: Vec<i32>,
    nautilus_clear_dash_entities: Vec<i32>,
    /// Vanilla `movementThisTick` for the directly controlled vehicle: the
    /// local travel owner records its one `Entity.move` here and the
    /// same-call `applyEffectsFromBlocks` takes it. Never retained across
    /// ticks or publications.
    local_vehicle_move: Option<(i32, EntityBlockEffectMovement)>,
}

/// One local-player physics step's `applyEffectsFromBlocks` inputs: the pose
/// integrator owns position/delta/fall distance and the step's recorded
/// clipped move, while the entity store owns the private effect state.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LocalPlayerBlockEffectStep {
    pub(crate) player_id: i32,
    pub(crate) position: EntityVec3,
    pub(crate) delta_movement: EntityVec3,
    pub(crate) fall_distance: f64,
    pub(crate) bounds: crate::EntityPickBoundsState,
    pub(crate) on_ground: bool,
    pub(crate) stepping_carefully: bool,
    /// `LivingEntity.canFreeze` extras beyond the type tag: spectator mode
    /// and freeze-immune wearables, both resolved by the local-player owner.
    pub(crate) extra_freeze_immune: bool,
    pub(crate) movement: EntityBlockEffectMovement,
}

/// Pose write-backs from one local-player block-effect callback.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LocalPlayerBlockEffectOutcome {
    pub(crate) delta_movement: EntityVec3,
    pub(crate) fall_distance: f64,
}

/// Block-effect work performed inside the direct client movement pass (the
/// explicit arrow callback slot), folded into the tick observation beside the
/// ordered common-pass counters.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct EntityBlockEffectWork {
    pub(super) entities: usize,
    pub(super) blocks: usize,
    pub(super) callbacks: usize,
    pub(super) duration: std::time::Duration,
}

impl EntityClientTickScratch {
    fn clear(&mut self) {
        self.scheduled_set.clear();
        self.scheduled_entities.clear();
        self.player_passenger_ancestors.clear();
        self.guardian_dry_jump_entities.clear();
        self.camel_clear_dash_entities.clear();
        self.camel_stand_instantly_entities.clear();
        self.nautilus_clear_dash_entities.clear();
    }
}

pub(crate) struct BeeSoundState {
    pub(crate) transform: EntityTransformState,
    pub(crate) is_baby: bool,
    pub(crate) angry: bool,
}

/// Logical work performed by one batched entity client-tick advance.
///
/// `scanned_entities` counts tracked rows considered across all ticks in the
/// batch. `mutated_entities` counts the ordered root/passenger schedule whose
/// animation state advances, without comparing the large component afterward.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EntityClientTickWork {
    pub scanned_entities: usize,
    pub mutated_entities: usize,
    pub preparation_duration: Duration,
    pub schedule_duration: Duration,
    pub projectile_duration: Duration,
    pub interpolation_duration: Duration,
    pub block_effect_duration: Duration,
    pub block_effect_entities: usize,
    pub block_effect_blocks_visited: usize,
    pub block_effect_callbacks_applied: usize,
    pub animation_duration: Duration,
}

pub(crate) struct EntityInWaterTickInput {
    pub(crate) entity_id: i32,
    pub(crate) entity_type_id: i32,
    pub(crate) aabb_min: [f64; 3],
    pub(crate) aabb_max: [f64; 3],
    pub(crate) position: EntityVec3,
    pub(crate) eye_position: [f64; 3],
    pub(crate) guardian_below_pos: Option<crate::BlockPos>,
    pub(crate) guardian_attack_target_id: i32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EntityBoatClientTickInput {
    pub(crate) entity_id: i32,
    pub(crate) entity_type_id: i32,
    pub(crate) uuid: Uuid,
    pub(crate) position: EntityVec3,
    pub(crate) relative_min: [f64; 3],
    pub(crate) relative_max: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityBoatClientTickWorldFacts {
    pub(crate) entity_id: i32,
    pub(crate) status: super::EntityBoatStatus,
    pub(crate) water_level: f64,
    pub(crate) land_friction: Option<f32>,
    pub(crate) can_push_local_player: bool,
    pub(crate) relative_min: [f64; 3],
    pub(crate) relative_max: [f64; 3],
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct EntityClientTickFluidFacts {
    pub(crate) in_water: bool,
    pub(crate) in_lava: bool,
    pub(crate) in_liquid: bool,
    pub(crate) eye_in_water: bool,
}

impl EntityClientTickWork {
    pub(crate) fn accumulate(&mut self, other: Self) {
        self.scanned_entities = self.scanned_entities.saturating_add(other.scanned_entities);
        self.mutated_entities = self.mutated_entities.saturating_add(other.mutated_entities);
        self.preparation_duration += other.preparation_duration;
        self.schedule_duration += other.schedule_duration;
        self.projectile_duration += other.projectile_duration;
        self.interpolation_duration += other.interpolation_duration;
        self.block_effect_duration += other.block_effect_duration;
        self.block_effect_entities = self
            .block_effect_entities
            .saturating_add(other.block_effect_entities);
        self.block_effect_blocks_visited = self
            .block_effect_blocks_visited
            .saturating_add(other.block_effect_blocks_visited);
        self.block_effect_callbacks_applied = self
            .block_effect_callbacks_applied
            .saturating_add(other.block_effect_callbacks_applied);
        self.animation_duration += other.animation_duration;
    }
}

/// Cost and logical work of rebuilding the entity shell for one immutable
/// world generation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EntityPublicationSnapshotObservation {
    pub duration: Duration,
    pub rebuilt_entities: usize,
}

#[cfg(test)]
pub(crate) struct EntityPublicationPayloadSharing {
    pub(crate) data_values: bool,
    pub(crate) painting_variant: bool,
    pub(crate) equipment: bool,
    pub(crate) attributes: bool,
    pub(crate) passengers: bool,
    pub(crate) effects: bool,
    pub(crate) damage: bool,
}

fn sync_derived_marker<T: hecs::Component + Default>(
    ecs: &mut World,
    entity: Entity,
    enabled: bool,
) {
    if enabled {
        let already_present = ecs.get::<&T>(entity).is_ok();
        if !already_present {
            let _ = ecs.insert_one(entity, T::default());
        }
    } else {
        let _ = ecs.remove_one::<T>(entity);
    }
}

/// Vanilla `FallingBlockEntity.recreateFromPacket` seeds the synchronized
/// `DATA_START_POS` from the add-entity position before the first metadata
/// update. Retaining that client default keeps weighted block-model selection
/// stable even before (or in the absence of) the server's dirty metadata item.
fn entity_spawn_metadata(entity_type_id: i32, transform: EntityTransform) -> EntityMetadata {
    let mut metadata = EntityMetadata::default();
    if entity_type_id != VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID {
        return metadata;
    }
    let Some(accessor) =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_START_POS")
    else {
        return metadata;
    };
    metadata.data_values.push(ProtocolEntityDataValue {
        data_id: accessor.data_id,
        serializer_id: accessor.serializer_id,
        value: EntityDataValueKind::BlockPos(bbb_protocol::packets::BlockPos {
            x: transform.position.x.floor() as i32,
            y: transform.position.y.floor() as i32,
            z: transform.position.z.floor() as i32,
        }),
    });
    metadata
}

impl EntityStore {
    #[cfg(test)]
    pub(crate) fn block_effects_debug_state(
        &self,
        id: i32,
    ) -> Option<super::EntityBlockEffectsDebugState> {
        let entity = *self.by_protocol_id.get(&id)?;
        let supporting = self.ecs.get::<&EntitySupportingBlock>(entity).ok()?;
        let collision_facts = self
            .ecs
            .get::<&EntityClientMovementCollisionFacts>(entity)
            .ok();
        Some(
            supporting.block_effects.debug_state(
                collision_facts
                    .as_deref()
                    .map(|facts| &facts.block_effect_movements),
            ),
        )
    }

    fn allocate_incarnation(&mut self) -> EntityIncarnation {
        let incarnation = EntityIncarnation(self.next_incarnation);
        self.next_incarnation = self
            .next_incarnation
            .checked_add(1)
            .expect("entity incarnation space exhausted");
        incarnation
    }

    #[cfg(test)]
    pub(crate) fn publication_payload_sharing(
        &self,
        other: &Self,
        id: i32,
    ) -> Option<EntityPublicationPayloadSharing> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let other_entity = other.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let other_metadata = other.ecs.get::<&EntityMetadata>(other_entity).ok()?;
        let data_values = metadata.data_values.ptr_eq(&other_metadata.data_values);
        let painting_variant = metadata
            .painting_variant_projection
            .ptr_eq(&other_metadata.painting_variant_projection);
        drop(metadata);
        drop(other_metadata);
        let equipment = self.ecs.get::<&EntityEquipment>(entity).ok()?;
        let other_equipment = other.ecs.get::<&EntityEquipment>(other_entity).ok()?;
        let equipment = equipment.equipment.ptr_eq(&other_equipment.equipment);
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let other_attributes = other.ecs.get::<&EntityAttributes>(other_entity).ok()?;
        let attributes = attributes.attributes.ptr_eq(&other_attributes.attributes);
        let mount = self.ecs.get::<&EntityMount>(entity).ok()?;
        let other_mount = other.ecs.get::<&EntityMount>(other_entity).ok()?;
        let passengers = mount.passengers.ptr_eq(&other_mount.passengers);
        let effects = self.ecs.get::<&EntityMobEffects>(entity).ok()?;
        let other_effects = other.ecs.get::<&EntityMobEffects>(other_entity).ok()?;
        let effects = effects.effects.ptr_eq(&other_effects.effects);
        let damage = self.ecs.get::<&EntityDamage>(entity).ok()?;
        let other_damage = other.ecs.get::<&EntityDamage>(other_entity).ok()?;
        let damage = damage.last_damage.ptr_eq(&other_damage.last_damage);
        Some(EntityPublicationPayloadSharing {
            data_values,
            painting_variant,
            equipment,
            attributes,
            passengers,
            effects,
            damage,
        })
    }

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
        mut identity: EntityIdentity,
        transform: EntityTransform,
    ) {
        let fresh_incarnation = self.allocate_incarnation();
        identity.incarnation = fresh_incarnation;
        let id = identity.id;
        let uuid = identity.uuid;
        let entity_type_id = identity.entity_type_id;
        let spawn_owner_uuid = self.uuid_for_protocol_id(identity.data);
        let fresh_projectile = entity_hurting_projectile_from_state(entity_type_id, None);
        let fresh_projectile_lifecycle =
            entity_projectile_lifecycle_from_state(entity_type_id, None, spawn_owner_uuid);
        let fresh_experience_orb_movement = (entity_type_id
            == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID)
            .then(|| super::client_movement::experience_orb_movement_from_uuid(uuid));
        let fresh_boat_movement =
            is_vanilla_boat_type(entity_type_id).then(EntityBoatMovement::default);
        let fresh_minecart_command_block = (entity_type_id
            == VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID)
            .then(EntityMinecartCommandBlock::default);
        let fresh_fishing_hook_movement = (entity_type_id == VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID)
            .then(|| {
                super::client_movement::fishing_hook_movement_from_uuid(uuid, spawn_owner_uuid)
            });
        let uses_collision_facts =
            super::client_movement::client_movement_uses_collision_facts(entity_type_id);
        let uses_fluid_facts =
            super::client_movement::client_movement_uses_fluid_contact(entity_type_id);
        let is_experience_orb_player_target = entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID;
        let fresh_client_animations = EntityClientAnimations {
            animations: {
                let mut animations = EntityClientAnimationState {
                    spawn_body_y_rot: vanilla_living_entity_type(entity_type_id).then_some(
                        if entity_type_id == VANILLA_ENTITY_TYPE_SHULKER_ID {
                            0.0
                        } else {
                            transform.y_head_rot
                        },
                    ),
                    ..EntityClientAnimationState::default()
                };
                animations.initialize_spawn_defaults(entity_type_id, identity.uuid);
                animations
            },
        };
        let fresh_metadata = entity_spawn_metadata(entity_type_id, transform);

        if let Some(entity) = self.by_protocol_id.get(&id).copied() {
            let previous_uuid = self
                .ecs
                .get::<&EntityIdentity>(entity)
                .ok()
                .map(|identity| identity.uuid);
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
            if let Ok(mut existing) = self.ecs.get::<&mut EntityMetadata>(entity) {
                *existing = fresh_metadata.clone();
            }
            reset::<EntityEquipment>(&mut self.ecs, entity);
            reset::<EntityAttributes>(&mut self.ecs, entity);
            reset::<EntityTransientEvents>(&mut self.ecs, entity);
            reset::<EntityMount>(&mut self.ecs, entity);
            reset::<EntityLeash>(&mut self.ecs, entity);
            reset::<EntityMobEffects>(&mut self.ecs, entity);
            if let Ok(mut existing) = self.ecs.get::<&mut EntityClientAnimations>(entity) {
                *existing = fresh_client_animations;
            }
            reset::<EntityDamage>(&mut self.ecs, entity);
            reset::<EntitySupportingBlock>(&mut self.ecs, entity);
            reset::<EntityClientMovementCollisionFacts>(&mut self.ecs, entity);
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
            if let Some(command_block) = fresh_minecart_command_block {
                let updated = self
                    .ecs
                    .get::<&mut EntityMinecartCommandBlock>(entity)
                    .map(|mut existing| *existing = command_block.clone())
                    .is_ok();
                if !updated {
                    let _ = self.ecs.insert_one(entity, command_block);
                }
            } else {
                let _ = self.ecs.remove_one::<EntityMinecartCommandBlock>(entity);
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
            if let Some(projectile) = fresh_projectile_lifecycle {
                let updated = {
                    if let Ok(mut existing) = self.ecs.get::<&mut EntityProjectileLifecycle>(entity)
                    {
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
                let _ = self.ecs.remove_one::<EntityProjectileLifecycle>(entity);
            }
            if let Some(movement) = fresh_experience_orb_movement {
                let updated = {
                    if let Ok(mut existing) =
                        self.ecs.get::<&mut EntityExperienceOrbMovement>(entity)
                    {
                        *existing = movement;
                        true
                    } else {
                        false
                    }
                };
                if !updated {
                    let _ = self.ecs.insert_one(entity, movement);
                }
            } else {
                let _ = self.ecs.remove_one::<EntityExperienceOrbMovement>(entity);
            }
            if let Some(movement) = fresh_boat_movement {
                let updated = self
                    .ecs
                    .get::<&mut EntityBoatMovement>(entity)
                    .map(|mut existing| *existing = movement)
                    .is_ok();
                if !updated {
                    let _ = self.ecs.insert_one(entity, movement);
                }
            } else {
                let _ = self.ecs.remove_one::<EntityBoatMovement>(entity);
            }
            if let Some(movement) = fresh_fishing_hook_movement {
                let updated = self
                    .ecs
                    .get::<&mut EntityFishingHookMovement>(entity)
                    .map(|mut existing| *existing = movement)
                    .is_ok();
                if !updated {
                    let _ = self.ecs.insert_one(entity, movement);
                }
            } else {
                let _ = self.ecs.remove_one::<EntityFishingHookMovement>(entity);
            }
            sync_derived_marker::<EntityClientMovementCollisionFacts>(
                &mut self.ecs,
                entity,
                uses_collision_facts,
            );
            sync_derived_marker::<EntityClientMovementFluidFacts>(
                &mut self.ecs,
                entity,
                uses_fluid_facts,
            );
            sync_derived_marker::<EntityExperienceOrbPlayerTargetFacts>(
                &mut self.ecs,
                entity,
                is_experience_orb_player_target,
            );
            if let Some(previous_uuid) = previous_uuid.filter(|previous| *previous != uuid) {
                if self.protocol_id_by_uuid.get(&previous_uuid) == Some(&id) {
                    self.protocol_id_by_uuid.remove(&previous_uuid);
                }
            }
            self.protocol_id_by_uuid.insert(uuid, id);
            return;
        }

        if !self.order.contains(&id) {
            self.order.push(id);
        }
        let entity = self.ecs.spawn((
            identity,
            transform,
            fresh_metadata,
            EntityEquipment::default(),
            EntityAttributes::default(),
            EntityTransientEvents::default(),
            EntityMount::default(),
            EntityLeash::default(),
            EntityMobEffects::default(),
            fresh_client_animations,
            EntityDamage::default(),
            EntitySupportingBlock::default(),
        ));
        if is_vanilla_minecart_type(entity_type_id) {
            let _ = self.ecs.insert_one(entity, EntityMinecartLerp::default());
        }
        if let Some(command_block) = fresh_minecart_command_block {
            let _ = self.ecs.insert_one(entity, command_block);
        }
        if let Some(projectile) = fresh_projectile {
            let _ = self.ecs.insert_one(entity, projectile);
        }
        if let Some(projectile) = fresh_projectile_lifecycle {
            let _ = self.ecs.insert_one(entity, projectile);
        }
        if let Some(movement) = fresh_experience_orb_movement {
            let _ = self.ecs.insert_one(entity, movement);
        }
        if let Some(movement) = fresh_boat_movement {
            let _ = self.ecs.insert_one(entity, movement);
        }
        if let Some(movement) = fresh_fishing_hook_movement {
            let _ = self.ecs.insert_one(entity, movement);
        }
        sync_derived_marker::<EntityClientMovementCollisionFacts>(
            &mut self.ecs,
            entity,
            uses_collision_facts,
        );
        sync_derived_marker::<EntityClientMovementFluidFacts>(
            &mut self.ecs,
            entity,
            uses_fluid_facts,
        );
        sync_derived_marker::<EntityExperienceOrbPlayerTargetFacts>(
            &mut self.ecs,
            entity,
            is_experience_orb_player_target,
        );
        self.by_protocol_id.insert(id, entity);
        self.protocol_id_by_uuid.insert(uuid, id);
    }

    fn spawn_components(&mut self, state: EntityState) {
        let id = state.id;
        let uuid = state.uuid;
        let entity_type_id = state.entity_type_id;
        let spawn_owner_uuid = self.uuid_for_protocol_id(state.data);
        let incarnation = self.allocate_incarnation();
        let mut identity = EntityIdentity::from(&state);
        identity.incarnation = incarnation;
        let entity = self.ecs.spawn((
            identity,
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
            EntitySupportingBlock::from(&state),
        ));
        // Only vanilla minecart types run the new-behavior lerp, so attach `EntityMinecartLerp`
        // conditionally. This keeps the per-client-tick `query_mut::<&mut EntityMinecartLerp>` scan
        // and the lerp-step accounting limited to actual minecarts instead of every entity.
        if is_vanilla_minecart_type(state.entity_type_id) {
            let _ = self
                .ecs
                .insert_one(entity, EntityMinecartLerp::from(&state));
        }
        if state.entity_type_id == VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID {
            let _ = self
                .ecs
                .insert_one(entity, EntityMinecartCommandBlock::from(&state));
        }
        if let Some(projectile) =
            entity_hurting_projectile_from_state(state.entity_type_id, state.hurting_projectile)
        {
            let _ = self.ecs.insert_one(entity, projectile);
        }
        if let Some(projectile) = entity_projectile_lifecycle_from_state(
            state.entity_type_id,
            state.projectile_lifecycle,
            spawn_owner_uuid,
        ) {
            let _ = self.ecs.insert_one(entity, projectile);
        }
        if state.entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID {
            let _ = self.ecs.insert_one(
                entity,
                super::client_movement::experience_orb_movement_from_uuid(uuid),
            );
        }
        if is_vanilla_boat_type(state.entity_type_id) {
            let _ = self.ecs.insert_one(entity, EntityBoatMovement::default());
        }
        if state.entity_type_id == VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID {
            let _ = self.ecs.insert_one(
                entity,
                super::client_movement::fishing_hook_movement_from_uuid(uuid, spawn_owner_uuid),
            );
        }
        sync_derived_marker::<EntityClientMovementCollisionFacts>(
            &mut self.ecs,
            entity,
            super::client_movement::client_movement_uses_collision_facts(entity_type_id),
        );
        sync_derived_marker::<EntityClientMovementFluidFacts>(
            &mut self.ecs,
            entity,
            super::client_movement::client_movement_uses_fluid_contact(entity_type_id),
        );
        sync_derived_marker::<EntityExperienceOrbPlayerTargetFacts>(
            &mut self.ecs,
            entity,
            entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID,
        );
        self.by_protocol_id.insert(id, entity);
        self.protocol_id_by_uuid.insert(uuid, id);
    }

    fn replace_existing_components(&mut self, entity: Entity, state: EntityState) {
        let uuid = state.uuid;
        let previous_uuid = self
            .ecs
            .get::<&EntityIdentity>(entity)
            .ok()
            .map(|identity| identity.uuid);
        let incarnation = self.allocate_incarnation();
        self.sync_components_from_state(entity, &state);
        if let Ok(mut identity) = self.ecs.get::<&mut EntityIdentity>(entity) {
            identity.incarnation = incarnation;
        }
        if let Some(previous_uuid) = previous_uuid.filter(|previous| *previous != uuid) {
            if self.protocol_id_by_uuid.get(&previous_uuid) == Some(&state.id) {
                self.protocol_id_by_uuid.remove(&previous_uuid);
            }
        }
        self.protocol_id_by_uuid.insert(uuid, state.id);
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

    pub(crate) fn incarnation(&self, id: i32) -> Option<u64> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityIdentity>(entity)
            .ok()
            .map(|identity| identity.incarnation.0)
    }

    pub(crate) fn minecart_command_block(&self, id: i32) -> Option<EntityMinecartCommandBlock> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityMinecartCommandBlock>(entity)
            .ok()
            .map(|command_block| (*command_block).clone())
    }

    pub(crate) fn sync_minecart_command_block_from_metadata_update(
        &mut self,
        id: i32,
        values: &[ProtocolEntityDataValue],
    ) -> bool {
        let Some(entity) = self.by_protocol_id.get(&id).copied() else {
            return false;
        };
        let Ok(mut command_block) = self.ecs.get::<&mut EntityMinecartCommandBlock>(entity) else {
            return false;
        };
        let mut updated = false;
        for value in values {
            match (value.data_id, &value.value) {
                (
                    VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
                    EntityDataValueKind::String(command),
                ) => {
                    command_block.command.clone_from(command);
                    updated = true;
                }
                (
                    VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID,
                    EntityDataValueKind::Component(output),
                ) => {
                    command_block.last_output = Some(output.clone());
                    updated = true;
                }
                _ => {}
            }
        }
        updated
    }

    pub(crate) fn clear_minecart_command_block_last_output(&mut self, id: i32) -> bool {
        let Some(entity) = self.by_protocol_id.get(&id).copied() else {
            return false;
        };
        let Ok(mut command_block) = self.ecs.get::<&mut EntityMinecartCommandBlock>(entity) else {
            return false;
        };
        command_block.last_output = None;
        true
    }

    pub(crate) fn set_minecart_command_block_track_output(
        &mut self,
        id: i32,
        track_output: bool,
    ) -> bool {
        let Some(entity) = self.by_protocol_id.get(&id).copied() else {
            return false;
        };
        let Ok(mut command_block) = self.ecs.get::<&mut EntityMinecartCommandBlock>(entity) else {
            return false;
        };
        command_block.track_output = track_output;
        true
    }

    pub(crate) fn uuid_for_protocol_id(&self, id: i32) -> Option<Uuid> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityIdentity>(entity)
            .ok()
            .map(|identity| identity.uuid)
    }

    pub(crate) fn player_rideable_jumping_cooldown(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        if vanilla_entity_metadata_type_is_camel(identity.entity_type_id) {
            return Some(
                client_animations
                    .animations
                    .camel_jump_cooldown(partial_ticks),
            );
        }
        if is_vanilla_abstract_nautilus_type(identity.entity_type_id) {
            return Some(
                client_animations
                    .animations
                    .nautilus_jump_cooldown(partial_ticks),
            );
        }
        Some(0.0)
    }

    pub(crate) fn ridden_jump_state(&self, id: i32) -> Option<RiddenJumpState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        animations.animations.ridden_jump
    }

    pub(crate) fn camel_jump_cooldown_ticks(&self, id: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(
            animations
                .animations
                .camel_dash_cooldown
                .map(|cooldown| cooldown.cooldown_ticks())
                .unwrap_or(0),
        )
    }

    pub(crate) fn nautilus_jump_cooldown_ticks(&self, id: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(
            animations
                .animations
                .nautilus_dash_cooldown
                .map(|cooldown| cooldown.cooldown_ticks())
                .unwrap_or(0),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn item_based_steering_controller_rotation(
        &self,
        vehicle_id: i32,
        entity_type_id: i32,
        default_item_equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
        local_player_id: Option<i32>,
        local_player_rotation: Option<(f32, f32)>,
        local_player_holds_carrot_on_a_stick: bool,
        local_player_holds_warped_fungus_on_a_stick: bool,
    ) -> Option<(f32, f32)> {
        vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
            entity_type_id,
        )?;
        if !self.saddle_slot_contains_saddle_item(vehicle_id, default_item_equipment_slots) {
            return None;
        }
        let controller_id = self.mount(vehicle_id)?.passengers.first().copied()?;
        let required_item_id = match entity_type_id {
            VANILLA_ENTITY_TYPE_PIG_ID => VANILLA_ITEM_CARROT_ON_A_STICK_ID,
            VANILLA_ENTITY_TYPE_STRIDER_ID => VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
            _ => return None,
        };
        if local_player_id == Some(controller_id) {
            let holds_required_item = match entity_type_id {
                VANILLA_ENTITY_TYPE_PIG_ID => local_player_holds_carrot_on_a_stick,
                VANILLA_ENTITY_TYPE_STRIDER_ID => local_player_holds_warped_fungus_on_a_stick,
                _ => false,
            };
            if !holds_required_item {
                return None;
            }
            return local_player_rotation;
        }
        if self.entity_type_id(controller_id) != Some(VANILLA_ENTITY_TYPE_PLAYER_ID)
            || ![false, true].into_iter().any(|off_hand| {
                self.held_item(controller_id, off_hand)
                    .and_then(|item| item.item_id)
                    == Some(required_item_id)
            })
        {
            return None;
        }
        let transform = self.transform(controller_id)?;
        Some((transform.x_rot, transform.y_rot))
    }

    fn strider_suffocating_for_client_tick(
        &self,
        id: i32,
        fluid_facts_by_id: &HashMap<i32, EntityClientTickFluidFacts>,
    ) -> Option<bool> {
        if self.entity_type_id(id)? != VANILLA_ENTITY_TYPE_STRIDER_ID {
            return None;
        }
        let mob_accessors =
            vanilla_entity_metadata_mob_accessors_for_entity_type(VANILLA_ENTITY_TYPE_STRIDER_ID)?;
        let mob_flags =
            self.metadata_byte(id, mob_accessors.flags_data_id, mob_accessors.flags_default)?;
        if mob_flags & mob_accessors.no_ai_mask != 0 {
            return None;
        }
        let on_warm_strider = self.mount(id)?.vehicle_id.is_some_and(|vehicle_id| {
            self.entity_type_id(vehicle_id) == Some(VANILLA_ENTITY_TYPE_STRIDER_ID)
                && vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type(
                    VANILLA_ENTITY_TYPE_STRIDER_ID,
                )
                .is_some_and(|(data_id, default)| {
                    !self
                        .metadata_bool_for_entity(vehicle_id, data_id, default)
                        .unwrap_or(default)
                })
        });
        Some(
            !fluid_facts_by_id
                .get(&id)
                .is_some_and(|facts| facts.in_lava)
                && !on_warm_strider,
        )
    }

    #[cfg(test)]
    pub(crate) fn item_based_steering_debug_state(&self, id: i32) -> Option<(bool, i32)> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        animations
            .animations
            .item_based_steering
            .map(|steering| steering.debug_state())
    }

    pub(crate) fn advance_item_based_steering_tick(&mut self, id: i32) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let (identity, metadata, animations) = self
            .ecs
            .query_one_mut::<(
                &EntityIdentity,
                &EntityMetadata,
                &mut EntityClientAnimations,
            )>(entity)
            .ok()?;
        let (data_id, default) =
            vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
                identity.entity_type_id,
            )?;
        let total = entity_data_int(&metadata.data_values, data_id, default);
        Some(
            animations
                .animations
                .advance_item_based_steering_tick(total),
        )
    }

    /// Mirrors `Strider.setSuffocating`: the synced render flag and the
    /// transient movement-speed modifier are one canonical client-side update.
    /// A server attribute snapshot may already contain the named modifier, so
    /// replace by semantic id instead of stacking a second copy.
    pub(crate) fn set_strider_suffocating(&mut self, id: i32, suffocating: bool) -> Option<()> {
        const MODIFIER_ID: &str = "minecraft:suffocating";
        const MODIFIER_AMOUNT: f64 = -0.34_f32 as f64;

        let entity = self.by_protocol_id.get(&id).copied()?;
        let entity_type_id = self.ecs.get::<&EntityIdentity>(entity).ok()?.entity_type_id;
        let (data_id, default) =
            vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type(entity_type_id)?;
        let default_base =
            vanilla_default_attribute_base(entity_type_id, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)?;
        let current_flag = self.metadata_bool_for_entity(id, data_id, default)?;
        let current_modifier = self
            .ecs
            .get::<&EntityAttributes>(entity)
            .ok()?
            .attributes
            .iter()
            .find(|attribute| attribute.attribute_id == VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
            .and_then(|attribute| {
                attribute
                    .modifiers
                    .iter()
                    .find(|modifier| modifier.id == MODIFIER_ID)
            })
            .map(|modifier| (modifier.amount, modifier.operation_id));
        let expected_modifier = suffocating.then_some((
            MODIFIER_AMOUNT,
            VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
        ));
        if current_flag == suffocating && current_modifier == expected_modifier {
            return Some(());
        }
        let (metadata, attributes) = self
            .ecs
            .query_one_mut::<(&mut EntityMetadata, &mut EntityAttributes)>(entity)
            .ok()?;
        upsert_metadata_value(
            metadata,
            ProtocolEntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(suffocating),
            },
        );
        let attributes = &mut *attributes.attributes;
        let movement = if let Some(movement) = attributes
            .iter_mut()
            .find(|attribute| attribute.attribute_id == VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        {
            movement
        } else {
            attributes.push(ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                base: default_base,
                modifiers: Vec::new(),
            });
            attributes.last_mut()?
        };
        movement
            .modifiers
            .retain(|modifier| modifier.id != MODIFIER_ID);
        if suffocating {
            movement.modifiers.push(ProtocolAttributeModifier {
                id: MODIFIER_ID.to_string(),
                amount: MODIFIER_AMOUNT,
                operation_id: VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
            });
        }
        attributes.sort_by_key(|attribute| attribute.attribute_id);
        Some(())
    }

    pub(crate) fn start_local_nautilus_dash(&mut self, id: i32) -> Option<()> {
        self.with_client_animations_mut(id, |animations| {
            animations.animations.start_local_nautilus_dash();
        })
    }

    pub(crate) fn advance_deferred_nautilus_subclass_tick(
        &mut self,
        id: i32,
        in_water: bool,
        underwater: bool,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let (identity, transform, triggers) = {
            let (identity, transform, metadata, animations) = self
                .ecs
                .query_one_mut::<(
                    &EntityIdentity,
                    &EntityTransform,
                    &mut EntityMetadata,
                    &mut EntityClientAnimations,
                )>(entity)
                .ok()?;
            let is_dashing = entity_data_bool(
                &metadata.data_values,
                VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
                VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH,
            );
            let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
            let is_silent = entity_data_bool(
                &metadata.data_values,
                entity_accessors.silent_data_id,
                entity_accessors.silent_default,
            );
            let is_baby = vanilla_is_baby(identity.entity_type_id, &metadata.data_values);
            let triggers = animations.animations.advance_nautilus_subclass_tick(
                identity.id,
                is_dashing,
                in_water,
                underwater,
                is_silent,
                is_baby,
                transform.delta_movement,
            );
            if triggers.nautilus_clear_dash {
                upsert_metadata_value(
                    metadata,
                    ProtocolEntityDataValue {
                        data_id: VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                        value: EntityDataValueKind::Boolean(false),
                    },
                );
            }
            (identity.clone(), *transform, triggers)
        };
        append_nautilus_tick_triggers(
            &mut self.pending_entity_client_movement_effects,
            &identity,
            &transform,
            &triggers,
        );
        Some(())
    }

    pub(crate) fn start_local_camel_dash_after_tick(&mut self, id: i32) -> Option<()> {
        self.with_client_animations_mut(id, |animations| {
            animations.animations.start_local_camel_dash_after_tick();
        })
    }

    pub(crate) fn stop_local_camel_dash(&mut self, id: i32) -> Option<()> {
        self.with_client_animations_mut(id, |animations| {
            animations.animations.stop_local_camel_dash();
        })
    }

    pub(crate) fn next_camel_voice_pitch(&mut self, id: i32) -> Option<f32> {
        let is_baby = self.is_baby(id)?;
        self.with_client_animations_mut(id, |animations| {
            animations.animations.next_camel_voice_pitch(id, is_baby)
        })
    }

    pub(crate) fn with_ridden_jump_state_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut RiddenJumpState) -> R,
    ) -> Option<R> {
        self.with_client_animations_mut(id, |animations| {
            animations.animations.ridden_jump.as_mut().map(update)
        })?
    }

    pub(crate) fn saddle_slot_contains_saddle_item(
        &self,
        id: i32,
        equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
    ) -> bool {
        let Some(entity) = self.by_protocol_id.get(&id).copied() else {
            return false;
        };
        let Ok(equipment) = self.ecs.get::<&EntityEquipment>(entity) else {
            return false;
        };
        equipment_contains_saddle_item(Some(&*equipment), equipment_slots)
    }

    pub(crate) fn equipment_slot_contains_nonempty_item(
        &self,
        id: i32,
        slot: ProtocolEquipmentSlot,
    ) -> bool {
        let Some(entity) = self.by_protocol_id.get(&id).copied() else {
            return false;
        };
        let Ok(equipment) = self.ecs.get::<&EntityEquipment>(entity) else {
            return false;
        };
        equipment.equipment.iter().any(|update| {
            update.slot == slot && update.item.count > 0 && update.item.item_id.is_some()
        })
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
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        self.metadata_bool(id, accessors.silent_data_id, accessors.silent_default)
    }

    pub(crate) fn creaking_is_heart_bound(&self, id: i32) -> Option<bool> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let data_id = vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type(
            identity.entity_type_id,
        )?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(metadata.data_values.iter().any(|value| {
            value.data_id == data_id
                && matches!(&value.value, EntityDataValueKind::OptionalBlockPos(Some(_)))
        }))
    }

    pub(crate) fn guardian_attack_sound_state(&self, id: i32) -> Option<(super::EntityVec3, f32)> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if !vanilla_entity_metadata_type_is_guardian(identity.entity_type_id) {
            return None;
        }
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        let attack = animations.animations.guardian_attack.unwrap_or_default();
        let duration = guardian_attack_duration(identity.entity_type_id);
        Some((transform.position, attack.attack_scale(0.0, duration)))
    }

    pub(crate) fn sniffer_can_play_digging_sound(&self, id: i32) -> Option<bool> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(sniffer_can_play_digging_sound(
            identity.entity_type_id,
            &metadata.data_values,
        ))
    }

    pub(crate) fn no_gravity(&self, id: i32) -> Option<bool> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        self.metadata_bool(
            id,
            accessors.no_gravity_data_id,
            accessors.no_gravity_default,
        )
    }

    pub(crate) fn ticks_frozen(&self, id: i32) -> Option<i32> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        self.metadata_int(
            id,
            accessors.ticks_frozen_data_id,
            accessors.ticks_frozen_default,
        )
    }

    /// Vanilla `Entity.isFullyFrozen()` = `getTicksFrozen() >=
    /// getTicksRequiredToFreeze()` (140): the powder-snow freeze is complete
    /// (drives the shaking body and, for the local player, the frozen heart
    /// sprites in `Gui.HeartType.forPlayer`).
    pub(crate) fn is_fully_frozen(&self, id: i32) -> bool {
        self.ticks_frozen(id).unwrap_or(0) >= VANILLA_TICKS_REQUIRED_TO_FREEZE
    }

    /// Vanilla `Entity.getAirSupply()`: the synced `DATA_AIR_SUPPLY_ID` int,
    /// defaulting to the define-time `getMaxAirSupply()` (300, Entity.java:312)
    /// while the server has not synced a non-default value.
    pub(crate) fn air_supply(&self, id: i32) -> Option<i32> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        self.metadata_int(
            id,
            accessors.air_supply_data_id,
            VANILLA_MAX_AIR_SUPPLY_TICKS,
        )
    }

    /// Vanilla `LivingEntity.getHealth()`: the synced `DATA_HEALTH_ID` float,
    /// defaulting to the define-time `1.0F` (LivingEntity.java:314) while the
    /// server has not synced the spawn-time health.
    pub(crate) fn living_entity_health(&self, id: i32) -> Option<f32> {
        let entity_type_id = self.entity_type_id(id)?;
        let accessors =
            vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)?;
        self.metadata_float(id, accessors.health_data_id, accessors.health_default)
    }

    pub(crate) fn is_baby(&self, id: i32) -> Option<bool> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(vanilla_is_baby(
            identity.entity_type_id,
            &metadata.data_values,
        ))
    }

    pub(crate) fn is_fall_flying(&self, id: i32) -> Option<bool> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(entity_is_fall_flying(&metadata.data_values))
    }

    pub(crate) fn is_descending(&self, id: i32) -> Option<bool> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        Some(
            self.metadata_byte(
                id,
                accessors.shared_flags_data_id,
                accessors.shared_flags_default,
            )? & accessors.shift_key_down_mask
                != 0,
        )
    }

    /// Vanilla `Player.getAbsorptionAmount()`: the synced
    /// `DATA_PLAYER_ABSORPTION_ID` float, defaulting to the define-time `0.0F`
    /// (Player.java:224).
    pub(crate) fn player_absorption(&self, id: i32) -> Option<f32> {
        let entity_type_id = self.entity_type_id(id)?;
        let (data_id, default) =
            vanilla_entity_metadata_player_absorption_accessor_for_entity_type(entity_type_id)?;
        self.metadata_float(id, data_id, default)
    }

    pub(crate) fn metadata_int_for_entity(
        &self,
        id: i32,
        data_id: u8,
        default: i32,
    ) -> Option<i32> {
        self.metadata_int(id, data_id, default)
    }

    pub(crate) fn metadata_bool_for_entity(
        &self,
        id: i32,
        data_id: u8,
        default: bool,
    ) -> Option<bool> {
        self.metadata_bool(id, data_id, default)
    }

    pub(crate) fn metadata_registry_reference_for_entity(
        &self,
        id: i32,
        data_id: u8,
        serializer: EntityDataRegistryHolder,
    ) -> Option<Option<EntityDataRegistryReference>> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::RegistryId {
                        serializer: actual,
                        value,
                    } if *actual == serializer => Some(value.clone()),
                    _ => None,
                }),
        )
    }

    pub(super) fn metadata_byte(&self, id: i32, data_id: u8, default: i8) -> Option<i8> {
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

    pub(crate) fn metadata_optional_rich_component_override(
        &self,
        id: i32,
        data_id: u8,
    ) -> Option<Option<bbb_protocol::RichTextComponent>> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        metadata
            .data_values
            .iter()
            .find(|value| value.data_id == data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::OptionalComponent(component) => Some(component.clone()),
                _ => None,
            })
    }

    fn metadata_optional_component(&self, id: i32, data_id: u8) -> Option<String> {
        self.metadata_optional_rich_component_override(id, data_id)?
            .map(|component| component.text().to_string())
    }

    pub(crate) fn entity_has_passengers(&self, id: i32) -> bool {
        let Some(entity) = self.by_protocol_id.get(&id).copied() else {
            return false;
        };
        self.ecs
            .get::<&EntityMount>(entity)
            .is_ok_and(|mount| !mount.passengers.is_empty())
    }

    /// Vanilla `LivingEntity.getSleepingPos`: the synced bed position the entity is
    /// sleeping in (`SLEEPING_POS_ID`), or `None` when it is not in a bed.
    pub(crate) fn sleeping_pos(&self, id: i32) -> Option<crate::BlockPos> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            identity.entity_type_id,
        )?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        metadata
            .data_values
            .iter()
            .find(|value| value.data_id == accessors.sleeping_pos_data_id)
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
            .metadata_byte(
                id,
                VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
                VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION,
            )
            .unwrap_or(VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION)
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
        if vanilla_entity_renderer_uses_mob_item_in_hand_layer(entity_type_id) {
            let Some(accessors) =
                vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id)
            else {
                return false;
            };
            return self
                .metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                .is_some_and(|flags| flags & accessors.left_handed_mask != 0);
        }
        false
    }

    /// Vanilla `Entity.getEyeHeight(Pose.STANDING)` used by the sleeping bed
    /// head-offset translate: the eye height resolved with the synced pose stripped
    /// so the dimensions fall back to standing rather than `SLEEPING_DIMENSIONS`.
    pub(crate) fn standing_eye_height(&self, id: i32) -> Option<f32> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        let standing_data: Vec<_> = metadata
            .data_values
            .iter()
            .filter(|value| value.data_id != accessors.pose_data_id)
            .cloned()
            .collect();
        vanilla_eye_height_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &standing_data,
            &attributes.attributes,
            Some(client_animations.animations),
            metadata.painting_variant_projection.as_ref(),
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

    pub(super) fn metadata_long(&self, id: i32, data_id: u8, default: i64) -> Option<i64> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::Long(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(default),
        )
    }

    pub(crate) fn bee_sound_state(&self, id: i32, game_time: i64) -> Option<BeeSoundState> {
        let transform = self.transform_state(id)?;
        let accessors =
            vanilla_entity_metadata_bee_accessors_for_entity_type(transform.entity_type_id)?;
        let anger_end_time = self.metadata_long(
            id,
            accessors.anger_end_time_data_id,
            accessors.anger_end_time_default,
        )?;
        Some(BeeSoundState {
            transform,
            is_baby: self.is_baby(id)?,
            angry: anger_end_time > 0 && anger_end_time.wrapping_sub(game_time) > 0,
        })
    }

    fn metadata_float(&self, id: i32, data_id: u8, default: f32) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::Float(value) => Some(*value),
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
        let Some(accessors) = vanilla_entity_metadata_enderman_render_accessors_for_entity_type(
            identity.entity_type_id,
        ) else {
            return Some(None);
        };
        self.metadata_optional_block_state(id, accessors.carried_block_state_data_id)
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
        let accessors = vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type(
            identity.entity_type_id,
        )?;

        let custom_state_id = self
            .metadata_optional_block_state(id, accessors.custom_display_block_data_id)
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
            let furnace_has_fuel =
                vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type(
                    identity.entity_type_id,
                )
                .is_some_and(|(data_id, default)| {
                    self.metadata_bool(id, data_id, default).unwrap_or(default)
                });
            default_minecart_display_block_state(
                identity.entity_type_id,
                furnace_has_fuel,
                registries,
            )?
        };
        let display_offset = self
            .metadata_int(
                id,
                accessors.display_offset_data_id,
                accessors.display_offset_default,
            )
            .unwrap_or(accessors.display_offset_default);
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
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID {
            return None;
        }
        let state = registries.block_state(identity.data)?;
        if state.name == "minecraft:air" {
            return None;
        }
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let [_, height] = vanilla_entity_type_dimensions(identity.entity_type_id)?;
        let block_pos = crate::BlockPos {
            x: transform.position.x.floor() as i32,
            y: (transform.position.y + f64::from(height)).floor() as i32,
            z: transform.position.z.floor() as i32,
        };
        let start_pos_data_id = vanilla_entity_metadata_accessor_for_entity_type(
            identity.entity_type_id,
            "DATA_START_POS",
        )?
        .data_id;
        let random_seed_pos = metadata
            .data_values
            .iter()
            .find(|value| value.data_id == start_pos_data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::BlockPos(pos) => Some(crate::BlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                }),
                _ => None,
            })
            .unwrap_or(block_pos);
        Some(FallingBlockModelState {
            block_state_id: identity.data,
            block: EntityBlockModelState {
                name: state.name.clone(),
                properties: state.properties.clone(),
            },
            random_seed_pos,
            block_pos,
        })
    }

    pub(crate) fn primed_tnt_block_state(
        &self,
        id: i32,
        registries: &RegistrySet,
    ) -> Option<EntityBlockModelState> {
        let identity = self.identity(id)?;
        let accessors =
            vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(identity.entity_type_id)?;

        let state_id = self
            .metadata_block_state(id, accessors.block_state_data_id)
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

        default_primed_tnt_block_state(registries, accessors.block_state_default)
    }

    pub(crate) fn primed_tnt_fuse_remaining_in_ticks(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<f32> {
        let identity = self.identity(id)?;
        let accessors =
            vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(identity.entity_type_id)?;
        let fuse = self
            .metadata_int(id, accessors.fuse_data_id, accessors.fuse_default)
            .unwrap_or(accessors.fuse_default);
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
            let Some(accessors) = vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(
                identity.entity_type_id,
            ) else {
                continue;
            };
            let fuse = self
                .metadata_int(identity.id, accessors.fuse_data_id, accessors.fuse_default)
                .unwrap_or(accessors.fuse_default);
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

    pub(crate) fn take_ravager_stun_particle_states(&mut self) -> Vec<RavagerStunParticleState> {
        std::mem::take(&mut self.pending_ravager_stun_particles)
    }

    pub(crate) fn take_evoker_fangs_crit_particle_states(
        &mut self,
    ) -> Vec<EvokerFangsCritParticleState> {
        std::mem::take(&mut self.pending_evoker_fangs_crit_particles)
    }

    pub(crate) fn drain_entity_client_movement_effect_states(
        &mut self,
    ) -> std::vec::Drain<'_, EntityClientMovementEffectState> {
        self.pending_entity_client_movement_effects.drain(..)
    }

    pub(crate) fn take_entity_client_movement_effect_states(
        &mut self,
    ) -> Vec<EntityClientMovementEffectState> {
        std::mem::take(&mut self.pending_entity_client_movement_effects)
    }

    pub(crate) fn recycle_entity_client_movement_effect_states(
        &mut self,
        mut effects: Vec<EntityClientMovementEffectState>,
    ) {
        debug_assert!(effects.is_empty());
        effects.clear();
        self.pending_entity_client_movement_effects = effects;
    }

    pub(crate) fn take_ominous_item_spawner_particle_states(
        &mut self,
    ) -> Vec<OminousItemSpawnerParticleState> {
        std::mem::take(&mut self.pending_ominous_item_spawner_particles)
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
            metadata.painting_variant_projection.as_ref(),
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

    pub(crate) fn model_source_bounds(
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
            metadata.painting_variant_projection.as_ref(),
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
                    position: transform.render_position(partial_ticks),
                    bounds,
                });
            }
        }
        targets
    }

    /// Vanilla `EntityHitboxDebugRenderer.emitGizmos` walks every rendered entity and draws its
    /// bounding box unless `entity.isInvisible()`; the pickability gate only applies to actual
    /// pick targeting, so non-pickable entities (dropped items, experience orbs, area effect
    /// clouds) still show their boxes here.
    pub(crate) fn debug_hitbox_targets_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<super::EntityPickTargetState> {
        let entity_accessors = vanilla_entity_metadata_entity_accessors();
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
            if let Some(accessors) = entity_accessors {
                let invisible = self
                    .metadata_byte(
                        identity.id,
                        accessors.shared_flags_data_id,
                        accessors.shared_flags_default,
                    )
                    .unwrap_or(accessors.shared_flags_default)
                    & accessors.invisible_mask
                    != 0;
                if invisible {
                    continue;
                }
            }
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
                targets.push(ender_dragon_parent_pick_target(
                    identity.id,
                    *transform,
                    partial_ticks,
                ));
                let dragon_animation =
                    client_animations.and_then(|animations| animations.animations.ender_dragon);
                targets.extend(ender_dragon_part_pick_targets_at_partial_tick(
                    identity.id,
                    *transform,
                    dragon_animation,
                    partial_ticks,
                ));
            } else if let Some(bounds) = self.debug_hitbox_bounds(identity.id) {
                targets.push(super::EntityPickTargetState {
                    entity_id: identity.id,
                    position: transform.render_position(partial_ticks),
                    bounds,
                });
            }
        }
        targets
    }

    /// The debug-hitbox bounds skip the `isPickable` gate that `pick_bounds` applies.
    pub(crate) fn debug_hitbox_bounds(&self, id: i32) -> Option<super::EntityPickBoundsState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        vanilla_bounds_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(client_animations.animations),
            metadata.painting_variant_projection.as_ref(),
        )
    }

    pub(crate) fn fill_model_candidates_at_partial_tick(
        &self,
        partial_ticks: f32,
        game_time: i64,
        minecart_improvements_enabled: bool,
        registries: &RegistrySet,
        candidates: &mut Vec<super::EntityRenderCandidateState>,
    ) {
        candidates.clear();
        candidates.reserve(self.ecs.len() as usize);
        let mut query = self.ecs.query::<(
            &EntityIdentity,
            &EntityTransform,
            &EntityMetadata,
            &EntityEquipment,
            &EntityAttributes,
            &EntityClientAnimations,
            Option<&EntityMount>,
            Option<&EntityLeash>,
        )>();
        for (_, (identity, transform, metadata, equipment, attributes, animations, mount, leash)) in
            query.iter()
        {
            let is_invisible =
                vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
                    metadata
                        .data_values
                        .iter()
                        .find(|value| value.data_id == accessors.shared_flags_data_id)
                        .and_then(|value| match &value.value {
                            EntityDataValueKind::Byte(value) => Some(*value),
                            _ => None,
                        })
                        .unwrap_or(accessors.shared_flags_default)
                        & accessors.invisible_mask
                        != 0
                });
            let Some(mut bounds) = vanilla_model_source_bounds_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                continue;
            };
            if let Some(display_offset) = minecart_display_offset_for_components(
                identity.entity_type_id,
                &metadata.data_values,
                registries,
            ) {
                bounds = bounds.expand_towards_y(display_offset as f32 * 0.75 / 16.0);
            }
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_SHULKER_ID {
                let scale = vanilla_render_scale(identity.entity_type_id, &attributes.attributes);
                if let Some(offset) =
                    animations
                        .animations
                        .shulker_render_offset(transform.position, 0.0, scale)
                {
                    // `ShulkerRenderer.shouldRender` tests the complete old-to-
                    // current transition at partial tick zero, independently
                    // of the render state's current interpolation fraction.
                    bounds = expand_bounds_towards_offset(bounds, offset);
                }
            }
            candidates.push(super::EntityRenderCandidateState {
                entity_id: identity.id,
                uuid: identity.uuid,
                entity_type_id: identity.entity_type_id,
                position: transform.render_position(partial_ticks),
                current_position: transform.position,
                bounds,
                vehicle_id: mount.and_then(|mount| mount.vehicle_id),
                leash_holder_id: leash.and_then(|leash| leash.holder_id),
                fishing_hook_owner_id: (identity.entity_type_id
                    == VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID)
                    .then_some(identity.data),
                is_spectator: false,
                is_invisible,
                is_sleeping: vanilla_living_entity_type(identity.entity_type_id)
                    && entity_data_pose(&metadata.data_values) == VANILLA_ENTITY_POSE_SLEEPING_ID,
                wears_dragon_head: equipment.equipment.iter().any(|update| {
                    update.slot == ProtocolEquipmentSlot::Head
                        && update.item.item_id == Some(VANILLA_ITEM_DRAGON_HEAD_ID)
                }),
                display_culling: display_entity_culling_state(
                    identity.entity_type_id,
                    &metadata.data_values,
                ),
            });
        }
        drop(query);
        let candidate_ids = candidates
            .iter()
            .map(|candidate| candidate.entity_id)
            .collect::<Vec<_>>();
        let render_positions = self.render_positions_at_partial_tick(
            &candidate_ids,
            partial_ticks,
            game_time,
            minecart_improvements_enabled,
        );
        let current_positions = self.render_positions_at_partial_tick(
            &candidate_ids,
            1.0,
            game_time,
            minecart_improvements_enabled,
        );
        for candidate in candidates.iter_mut() {
            candidate.position = render_positions
                .get(&candidate.entity_id)
                .copied()
                .unwrap_or(candidate.position);
            candidate.current_position = current_positions
                .get(&candidate.entity_id)
                .copied()
                .unwrap_or(candidate.current_position);
        }
    }

    pub(crate) fn display_render_source_at_partial_tick(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<DisplayEntityRenderSourceState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let transform = *self.ecs.get::<&EntityTransform>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        display_entity_render_source(&identity, transform, &metadata, partial_ticks)
    }

    pub(crate) fn fishing_hook_render_source_at_partial_tick(
        &self,
        hook_id: i32,
        partial_ticks: f32,
    ) -> Option<FishingHookRenderSourceState> {
        let hook = self.by_protocol_id.get(&hook_id).copied()?;
        let hook_identity = self.ecs.get::<&EntityIdentity>(hook).ok()?;
        if hook_identity.entity_type_id != VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID {
            return None;
        }
        let owner_id = hook_identity.data;
        let hook_transform = self.ecs.get::<&EntityTransform>(hook).ok()?;

        let owner = self.by_protocol_id.get(&owner_id).copied()?;
        let owner_identity = self.ecs.get::<&EntityIdentity>(owner).ok()?;
        if owner_identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
            return None;
        }
        let owner_transform = self.ecs.get::<&EntityTransform>(owner).ok()?;
        let owner_metadata = self.ecs.get::<&EntityMetadata>(owner).ok()?;
        let owner_attributes = self.ecs.get::<&EntityAttributes>(owner).ok()?;
        let owner_animations = self.ecs.get::<&EntityClientAnimations>(owner).ok()?;
        let owner_equipment = self.ecs.get::<&EntityEquipment>(owner).ok()?;

        let owner_position = owner_transform.render_position(partial_ticks);
        let eye_height = vanilla_eye_height_for_entity_data(
            owner_identity.entity_type_id,
            owner_identity.data,
            &owner_metadata.data_values,
            &owner_attributes.attributes,
            Some(owner_animations.animations),
            owner_metadata.painting_variant_projection.as_ref(),
        )?;
        let owner_eye_position = EntityVec3 {
            y: owner_position.y + f64::from(eye_height),
            ..owner_position
        };
        let owner_scale =
            vanilla_render_scale(owner_identity.entity_type_id, &owner_attributes.attributes);
        let main_arm_left = self.entity_main_arm_left(owner_id, owner_identity.entity_type_id);
        let main_hand_holds_rod = owner_equipment.equipment.iter().any(|update| {
            update.slot == ProtocolEquipmentSlot::MainHand
                && update.item.item_id == Some(VANILLA_ITEM_FISHING_ROD_ID)
        });

        Some(FishingHookRenderSourceState {
            hook_position: hook_transform.render_position(partial_ticks),
            owner_id,
            owner_eye_position,
            owner_body_y_rot: owner_transform.render_body_y_rot(partial_ticks),
            owner_scale,
            owner_is_crouching: entity_data_pose(&owner_metadata.data_values)
                == VANILLA_ENTITY_POSE_CROUCHING_ID,
            holding_arm_left: if main_hand_holds_rod {
                main_arm_left
            } else {
                !main_arm_left
            },
            owner_attack_anim: owner_animations.animations.attack_anim(partial_ticks),
        })
    }

    pub(crate) fn restore_protocol_order_for_model_candidates(
        &self,
        candidates: &mut Vec<super::EntityRenderCandidateState>,
    ) {
        let mut by_id = candidates
            .drain(..)
            .map(|candidate| (candidate.entity_id, candidate))
            .collect::<HashMap<_, _>>();
        candidates.reserve(by_id.len());
        candidates.extend(
            self.order
                .iter()
                .filter_map(|entity_id| by_id.remove(entity_id)),
        );
    }

    #[cfg(test)]
    pub(crate) fn model_candidates_at_partial_tick(
        &self,
        partial_ticks: f32,
        registries: &RegistrySet,
    ) -> Vec<super::EntityRenderCandidateState> {
        let mut candidates = Vec::new();
        self.fill_model_candidates_at_partial_tick(
            partial_ticks,
            0,
            false,
            registries,
            &mut candidates,
        );
        candidates
    }

    pub(crate) fn refresh_client_position_from_entity_data(&mut self, id: i32) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let old_position = {
            let (identity, metadata, transform) = self
                .ecs
                .query_one_mut::<(&EntityIdentity, &EntityMetadata, &mut EntityTransform)>(entity)
                .ok()?;
            let position = vanilla_client_position_for_entity_data(
                identity.entity_type_id,
                transform.position_base,
                identity.data,
                &metadata.data_values,
                metadata.painting_variant_projection.as_ref(),
            )?;
            let old_position = transform.position;
            transform.position = position;
            old_position
        };
        self.finish_shulker_position_change(id, old_position);
        Some(())
    }

    pub(crate) fn finish_shulker_position_change(
        &mut self,
        id: i32,
        old_position: super::EntityVec3,
    ) -> Option<bool> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut query = self
            .ecs
            .query_one::<(
                &EntityIdentity,
                &EntityMount,
                &mut EntityTransform,
                &mut EntityClientAnimations,
            )>(entity)
            .ok()?;
        let (identity, mount, transform, animations) = query.get()?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_SHULKER_ID {
            return Some(false);
        }
        if mount.vehicle_id.is_none() {
            // Vanilla `Shulker.setPos` keeps the packet codec base untouched,
            // but snaps a non-passenger's canonical position to its attachment
            // block center before starting the client render offset.
            transform.position = vanilla_shulker_attached_position(transform.position);
        }
        let started = animations.animations.record_shulker_position_change(
            old_position,
            transform.position,
            mount.vehicle_id.is_some(),
        );
        if started {
            // Vanilla Shulker.setPos makes the generic position endpoints equal
            // and delegates the visible transition to getRenderPosition.
            transform.old_position = transform.position;
        }
        Some(started)
    }

    pub(crate) fn shulker_started_riding(&mut self, id: i32) {
        if self.entity_type_id(id) != Some(VANILLA_ENTITY_TYPE_SHULKER_ID) {
            return;
        }
        let _ = self.with_client_animations_mut(id, |animations| {
            animations.animations.shulker_started_riding();
        });
    }

    pub(crate) fn shulker_stopped_riding(&mut self, id: i32) {
        if self.entity_type_id(id) != Some(VANILLA_ENTITY_TYPE_SHULKER_ID) {
            return;
        }
        let Some(position) = self.transform(id).map(|transform| transform.position) else {
            return;
        };
        let _ = self.with_client_animations_mut(id, |animations| {
            animations.animations.shulker_stopped_riding(position);
        });
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
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        attributes
            .attributes
            .iter()
            .find(|attribute| attribute.attribute_id == attribute_id)
            .map(vanilla_attribute_value)
            .or_else(|| vanilla_default_attribute_base(identity.entity_type_id, attribute_id))
    }

    pub(crate) fn attribute_base_value(&self, id: i32, attribute_id: i32) -> Option<f64> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        attributes
            .attributes
            .iter()
            .find(|attribute| attribute.attribute_id == attribute_id)
            .map(|attribute| attribute.base)
            .or_else(|| vanilla_default_attribute_base(identity.entity_type_id, attribute_id))
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

    fn render_position_and_y_rot_at_partial_tick(
        &self,
        id: i32,
        partial_ticks: f32,
        minecart_improvements_enabled: bool,
    ) -> Option<(super::EntityVec3, f32)> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut query = self
            .ecs
            .query_one::<(
                &EntityIdentity,
                &EntityTransform,
                Option<&EntityMinecartLerp>,
            )>(entity)
            .ok()?;
        let (identity, transform, minecart_lerp) = query.get()?;
        if minecart_improvements_enabled && is_vanilla_minecart_type(identity.entity_type_id) {
            if let Some(render_step) =
                minecart_lerp.and_then(|lerp| lerp.render_step(partial_ticks))
            {
                return Some((
                    super::movement::entity_vec3(render_step.position),
                    render_step.y_rot,
                ));
            }
        }
        Some((
            transform.render_position(partial_ticks),
            transform.render_y_rot(partial_ticks),
        ))
    }

    fn passenger_position_from_vehicle_render_pose(
        &self,
        passenger_id: i32,
        vehicle_id: i32,
        vehicle_position: super::EntityVec3,
        vehicle_y_rot: f32,
        partial_ticks: f32,
        game_time: i64,
    ) -> Option<super::EntityVec3> {
        let passenger_bounds = self.pick_bounds(passenger_id)?;
        let passenger_entity = self.by_protocol_id.get(&passenger_id).copied()?;
        let mut passenger_query = self
            .ecs
            .query_one::<(&EntityIdentity, &EntityTransform, &EntityMetadata)>(passenger_entity)
            .ok()?;
        let (passenger_identity, passenger_transform, passenger_metadata) =
            passenger_query.get()?;

        let vehicle_bounds = self.pick_bounds(vehicle_id)?;
        let vehicle_entity = self.by_protocol_id.get(&vehicle_id).copied()?;
        let mut vehicle_query = self
            .ecs
            .query_one::<(
                &EntityIdentity,
                &EntityMount,
                &EntityMetadata,
                &EntityAttributes,
                Option<&EntityClientAnimations>,
            )>(vehicle_entity)
            .ok()?;
        let (
            vehicle_identity,
            vehicle_mount,
            vehicle_metadata,
            vehicle_attributes,
            vehicle_client_animations,
        ) = vehicle_query.get()?;
        vanilla_passenger_position_for_entity_data(
            vehicle_identity.entity_type_id,
            vehicle_position,
            vehicle_y_rot,
            vehicle_bounds,
            &vehicle_metadata.data_values,
            &vehicle_attributes.attributes,
            vehicle_client_animations.map(|animations| animations.animations),
            &vehicle_mount.passengers,
            passenger_id,
            passenger_identity.entity_type_id,
            passenger_transform.render_y_rot(partial_ticks),
            passenger_bounds,
            &passenger_metadata.data_values,
            game_time,
        )
    }

    fn render_positions_at_partial_tick(
        &self,
        ids: &[i32],
        partial_ticks: f32,
        game_time: i64,
        minecart_improvements_enabled: bool,
    ) -> HashMap<i32, super::EntityVec3> {
        let mut resolved = HashMap::with_capacity(ids.len());
        let mut path = Vec::new();
        let mut path_ids = HashSet::new();
        for &id in ids {
            if resolved.contains_key(&id) {
                continue;
            }
            path.clear();
            path_ids.clear();
            let mut cursor = id;
            let base = loop {
                if let Some(position) = resolved.get(&cursor).copied() {
                    break Some(position);
                }
                if !path_ids.insert(cursor) || path.len() >= self.len() {
                    break None;
                }
                let Some(own_position) = self
                    .render_position_and_y_rot_at_partial_tick(
                        cursor,
                        partial_ticks,
                        minecart_improvements_enabled,
                    )
                    .map(|(position, _)| position)
                else {
                    break None;
                };
                let Some(vehicle_id) = self.mount(cursor).and_then(|mount| mount.vehicle_id) else {
                    resolved.insert(cursor, own_position);
                    break Some(own_position);
                };
                path.push((cursor, vehicle_id, own_position));
                cursor = vehicle_id;
            };

            let Some(mut position) = base else {
                // Broken/missing/cyclic graphs preserve every packet-owned
                // render endpoint already discovered on this walk.
                for &(entity_id, _, own_position) in &path {
                    resolved.entry(entity_id).or_insert(own_position);
                }
                continue;
            };
            for &(passenger_id, vehicle_id, own_position) in path.iter().rev() {
                let next_position = self
                    .render_position_and_y_rot_at_partial_tick(
                        vehicle_id,
                        partial_ticks,
                        minecart_improvements_enabled,
                    )
                    .and_then(|(_, vehicle_y_rot)| {
                        self.passenger_position_from_vehicle_render_pose(
                            passenger_id,
                            vehicle_id,
                            position,
                            vehicle_y_rot,
                            partial_ticks,
                            game_time,
                        )
                    })
                    .unwrap_or(own_position);
                resolved.insert(passenger_id, next_position);
                position = next_position;
            }
        }
        resolved
    }

    /// Projects one entity from the current render pose of the root vehicle
    /// through each immediate passenger edge. The packet-owned transforms are
    /// never mutated. A malformed cycle or broken edge falls back to the
    /// entity's own interpolated transform after at most one pass over the
    /// loaded entity count.
    pub(crate) fn render_position_at_partial_tick(
        &self,
        id: i32,
        partial_ticks: f32,
        game_time: i64,
        minecart_improvements_enabled: bool,
    ) -> Option<super::EntityVec3> {
        self.render_positions_at_partial_tick(
            &[id],
            partial_ticks,
            game_time,
            minecart_improvements_enabled,
        )
        .remove(&id)
    }

    fn living_body_render_y_rot(&self, id: i32, partial_ticks: f32) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut query = self
            .ecs
            .query_one::<(&EntityIdentity, &EntityTransform, &EntityClientAnimations)>(entity)
            .ok()?;
        let (identity, transform, animations) = query.get()?;
        if !vanilla_living_entity_type(identity.entity_type_id) {
            return None;
        }
        Some(if animations.animations.age_ticks == 0 {
            animations
                .animations
                .spawn_body_y_rot
                .unwrap_or_else(|| transform.render_body_y_rot(partial_ticks))
        } else {
            transform.render_body_y_rot(partial_ticks)
        })
    }

    pub(crate) fn passenger_position(
        &self,
        passenger_id: i32,
        vehicle_id: i32,
        game_time: i64,
    ) -> Option<super::EntityVec3> {
        let passenger_bounds = self.pick_bounds(passenger_id)?;
        let passenger_entity = self.by_protocol_id.get(&passenger_id).copied()?;
        let mut passenger_query = self
            .ecs
            .query_one::<(&EntityIdentity, &EntityTransform, &EntityMetadata)>(passenger_entity)
            .ok()?;
        let (passenger_identity, passenger_transform, passenger_metadata) =
            passenger_query.get()?;

        let vehicle_bounds = self.pick_bounds(vehicle_id)?;
        let vehicle_entity = self.by_protocol_id.get(&vehicle_id).copied()?;
        let mut vehicle_query = self
            .ecs
            .query_one::<(
                &EntityIdentity,
                &EntityTransform,
                &EntityMount,
                &EntityMetadata,
                &EntityAttributes,
                Option<&EntityClientAnimations>,
            )>(vehicle_entity)
            .ok()?;
        let (
            vehicle_identity,
            vehicle_transform,
            vehicle_mount,
            vehicle_metadata,
            vehicle_attributes,
            vehicle_client_animations,
        ) = vehicle_query.get()?;

        vanilla_passenger_position_for_entity_data(
            vehicle_identity.entity_type_id,
            vehicle_transform.position,
            vehicle_transform.y_rot,
            vehicle_bounds,
            &vehicle_metadata.data_values,
            &vehicle_attributes.attributes,
            vehicle_client_animations.map(|animations| animations.animations),
            &vehicle_mount.passengers,
            passenger_id,
            passenger_identity.entity_type_id,
            passenger_transform.y_rot,
            passenger_bounds,
            &passenger_metadata.data_values,
            game_time,
        )
    }

    pub(crate) fn local_player_passenger_position(
        &self,
        vehicle_id: i32,
        passenger_id: i32,
        passenger_is_swimming: bool,
        game_time: i64,
    ) -> Option<super::EntityVec3> {
        let vehicle_bounds = self.pick_bounds(vehicle_id)?;
        let vehicle_entity = self.by_protocol_id.get(&vehicle_id).copied()?;
        let mut vehicle_query = self
            .ecs
            .query_one::<(
                &EntityIdentity,
                &EntityTransform,
                &EntityMount,
                &EntityMetadata,
                &EntityAttributes,
                Option<&EntityClientAnimations>,
            )>(vehicle_entity)
            .ok()?;
        let (
            vehicle_identity,
            vehicle_transform,
            vehicle_mount,
            vehicle_metadata,
            vehicle_attributes,
            vehicle_client_animations,
        ) = vehicle_query.get()?;
        vanilla_local_player_passenger_position_for_entity_data(
            vehicle_identity.entity_type_id,
            vehicle_transform.position,
            vehicle_transform.y_rot,
            vehicle_bounds,
            &vehicle_metadata.data_values,
            &vehicle_attributes.attributes,
            vehicle_client_animations.map(|animations| animations.animations),
            &vehicle_mount.passengers,
            passenger_id,
            passenger_is_swimming,
            game_time,
        )
    }

    pub(crate) fn model_source(
        &self,
        id: i32,
        position: super::EntityVec3,
        partial_ticks: f32,
        minecart_improvements_enabled: bool,
        registries: &RegistrySet,
        default_item_max_damage: &BTreeMap<i32, i32>,
        armor_materials: &BTreeMap<i32, ArmorMaterialKind>,
        wings_equipment_assets: &BTreeMap<i32, String>,
        equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
        llama_body_decor_colors: &BTreeMap<i32, LlamaBodyDecorColor>,
        happy_ghast_harness_colors: &BTreeMap<i32, HappyGhastHarnessColor>,
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
        let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
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
        let armor_trim = |slot: ProtocolEquipmentSlot| {
            let material = armor_material(slot)?;
            armor_trim_sprite(
                registries,
                armor_item(slot)?,
                material.equipment_asset_id(),
                if slot == ProtocolEquipmentSlot::Legs {
                    ArmorTrimLayer::HumanoidLeggings
                } else {
                    ArmorTrimLayer::Humanoid
                },
            )
        };
        let chest_wings_equipment_asset = armor_item(ProtocolEquipmentSlot::Chest)
            .filter(|item| item.count > 0)
            .and_then(|item| item.item_id)
            .and_then(|item_id| wings_equipment_assets.get(&item_id));
        let chest_wings_trim = chest_wings_equipment_asset.and_then(|equipment_asset| {
            armor_trim_sprite(
                registries,
                armor_item(ProtocolEquipmentSlot::Chest)?,
                equipment_asset,
                ArmorTrimLayer::Wings,
            )
        });
        let chest_wings_foil = chest_wings_equipment_asset
            .and_then(|_| armor_item(ProtocolEquipmentSlot::Chest))
            .is_some_and(ItemStackSummary::has_foil);
        // Vanilla `SimpleEquipmentLayer` saddle users copy `EquipmentSlot.SADDLE` into render state,
        // then render only a non-empty equippable saddle item. bbb resolves the default saddle item
        // from the item equipment-slot map.
        let saddle_slot_item = || equipment_saddle_item(equipment, equipment_slots);
        let saddle_slot_contains_saddle_item = || saddle_slot_item().is_some();
        let saddle_trim = |enabled: bool, layer| {
            enabled.then_some(()).and_then(|()| {
                armor_trim_sprite(registries, saddle_slot_item()?, "minecraft:saddle", layer)
            })
        };
        let saddle_foil =
            |enabled: bool| enabled && saddle_slot_item().is_some_and(ItemStackSummary::has_foil);
        let pig_saddle = identity.entity_type_id == VANILLA_ENTITY_TYPE_PIG_ID
            && saddle_slot_contains_saddle_item();
        let pig_saddle_trim = saddle_trim(pig_saddle, ArmorTrimLayer::PigSaddle);
        let pig_saddle_foil = saddle_foil(pig_saddle);
        let equine_saddle = || -> bool {
            vanilla_entity_renderer_uses_abstract_horse_renderer(identity.entity_type_id)
                && saddle_slot_contains_saddle_item()
        };
        let equine_saddle = equine_saddle();
        let equine_saddle_trim_layer = match identity.entity_type_id {
            VANILLA_ENTITY_TYPE_HORSE_ID => Some(ArmorTrimLayer::HorseSaddle),
            VANILLA_ENTITY_TYPE_DONKEY_ID => Some(ArmorTrimLayer::DonkeySaddle),
            VANILLA_ENTITY_TYPE_MULE_ID => Some(ArmorTrimLayer::MuleSaddle),
            VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID => Some(ArmorTrimLayer::SkeletonHorseSaddle),
            VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => Some(ArmorTrimLayer::ZombieHorseSaddle),
            _ => None,
        };
        let equine_saddle_trim =
            equine_saddle_trim_layer.and_then(|layer| saddle_trim(equine_saddle, layer));
        let equine_saddle_foil = saddle_foil(equine_saddle);
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
        let strider_saddle_trim = saddle_trim(strider_saddle, ArmorTrimLayer::StriderSaddle);
        let strider_saddle_foil = saddle_foil(strider_saddle);
        let camel_saddle = vanilla_camel_saddle_type(identity.entity_type_id)
            && saddle_slot_contains_saddle_item();
        let camel_saddle_trim_layer =
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID {
                ArmorTrimLayer::CamelHuskSaddle
            } else {
                ArmorTrimLayer::CamelSaddle
            };
        let camel_saddle_trim = saddle_trim(camel_saddle, camel_saddle_trim_layer);
        let camel_saddle_foil = saddle_foil(camel_saddle);
        let camel_saddle_ridden = camel_saddle
            && mount
                .as_ref()
                .is_some_and(|mount| !mount.passengers.is_empty());
        let nautilus_saddle = is_vanilla_abstract_nautilus_type(identity.entity_type_id)
            && saddle_slot_contains_saddle_item();
        let nautilus_saddle_trim = saddle_trim(nautilus_saddle, ArmorTrimLayer::NautilusSaddle);
        let nautilus_saddle_foil = saddle_foil(nautilus_saddle);
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
        let happy_ghast_body_item = identity.entity_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            && body_slot_item().is_some();
        let happy_ghast_harness = if identity.entity_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
            body_slot_item_id()
                .and_then(|item_id| happy_ghast_harness_colors.get(&item_id).copied())
        } else {
            None
        };
        let happy_ghast_harness_trim = happy_ghast_harness.and_then(|color| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                color.equipment_asset_id(),
                ArmorTrimLayer::HappyGhastBody,
            )
        });
        let happy_ghast_harness_foil = happy_ghast_harness
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        let happy_ghast_body_item_is_harness = identity.entity_type_id
            == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            && body_slot_item_id().is_some_and(|item_id| {
                registries.holder_is_in_tag(
                    "minecraft:item",
                    VANILLA_HAPPY_GHAST_HARNESSES_ITEM_TAG,
                    item_id,
                )
            });
        let happy_ghast_is_leash_holder = if identity.entity_type_id
            == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
        {
            vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(identity.entity_type_id)
                .is_some_and(|accessors| {
                    entity_data_bool(
                        &metadata.data_values,
                        accessors.is_leash_holder_data_id,
                        accessors.is_leash_holder_default,
                    )
                })
        } else {
            false
        };
        let happy_ghast_ridden = identity.entity_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            && mount
                .as_ref()
                .is_some_and(|mount| !mount.passengers.is_empty());
        let llama_body_decor = if is_vanilla_llama_type(identity.entity_type_id)
            && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values)
        {
            body_slot_item_id().and_then(|item_id| llama_body_decor_colors.get(&item_id).copied())
        } else {
            None
        };
        let llama_body_decor_trim = llama_body_decor.and_then(|color| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                color.equipment_asset_id(),
                ArmorTrimLayer::LlamaBody,
            )
        });
        let llama_body_decor_foil = llama_body_decor
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        let nautilus_body_armor = if is_vanilla_abstract_nautilus_type(identity.entity_type_id)
            && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values)
        {
            body_slot_item_id()
                .and_then(|item_id| nautilus_body_armor_materials.get(&item_id).copied())
        } else {
            None
        };
        let nautilus_body_armor_trim = nautilus_body_armor.and_then(|material| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                material.equipment_asset_id(),
                ArmorTrimLayer::NautilusBody,
            )
        });
        let nautilus_body_armor_foil = nautilus_body_armor
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        let horse_body_armor = if is_vanilla_can_wear_horse_armor_type(identity.entity_type_id)
            && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values)
        {
            body_slot_item_id()
                .and_then(|item_id| horse_body_armor_materials.get(&item_id).copied())
        } else {
            None
        };
        let horse_body_armor_trim = horse_body_armor.and_then(|material| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                material.equipment_asset_id(),
                ArmorTrimLayer::HorseBody,
            )
        });
        let horse_body_armor_dye = horse_body_armor.and_then(|_| body_slot_armor_dye());
        let horse_body_armor_foil = horse_body_armor
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        let wolf_body_armor = if identity.entity_type_id == VANILLA_ENTITY_TYPE_WOLF_ID
            && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values)
        {
            body_slot_item_id().and_then(|item_id| wolf_body_armor_materials.get(&item_id).copied())
        } else {
            None
        };
        let wolf_body_armor_trim = wolf_body_armor.and_then(|material| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                material.equipment_asset_id(),
                ArmorTrimLayer::WolfBody,
            )
        });
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
        let is_fully_frozen =
            vanilla_living_entity_type(identity.entity_type_id) && self.is_fully_frozen(id);
        // Vanilla `LivingEntityRenderer.extractRenderState`: `isInvisibleToPlayer =
        // state.isInvisible && entity.isInvisibleTo(minecraft.player)`. The store-local baseline is
        // the non-spectator / no-friendly-team-override branch, so the world aggregation can clear it
        // for spectator viewers.
        let shared_flags = self
            .metadata_byte(
                id,
                entity_accessors.shared_flags_data_id,
                entity_accessors.shared_flags_default,
            )
            .unwrap_or(entity_accessors.shared_flags_default);
        let invisible_to_player = shared_flags & entity_accessors.invisible_mask != 0;
        let appears_glowing = shared_flags & entity_accessors.glowing_mask != 0;
        // Vanilla renderer `extractRenderState` assignments project `Mob.isAggressive()` for the
        // zombie/zombie-villager/zombified-piglin, skeleton, and illager paths. PiglinRenderer
        // instead projects `entity.getArmPose()`; bbb resolves that pose from the same synced Mob
        // flag plus held-item state, so the generated AbstractPiglin class membership is composed
        // here. Other entities do not consume the bit through a supported renderer pose.
        let is_aggressive =
            (vanilla_entity_renderer_extracts_mob_aggressive(identity.entity_type_id)
                || vanilla_entity_metadata_type_is_abstract_piglin(identity.entity_type_id))
                && vanilla_entity_metadata_mob_accessors_for_entity_type(identity.entity_type_id)
                    .is_some_and(|accessors| {
                        self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                            .unwrap_or(accessors.flags_default)
                            & accessors.aggressive_mask
                            != 0
                    });
        // Vanilla EvokerRenderer / IllusionerRenderer copy
        // `state.isCastingSpell = entity.isCastingSpell()`. Intersect that renderer assignment
        // with the generated SpellcasterIllager accessor owner before reading the shared byte id;
        // Pillager uses the same id for its BOOLEAN crossbow flag.
        let illager_spellcasting =
            vanilla_entity_renderer_extracts_casting_spell(identity.entity_type_id)
                && vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type(
                    identity.entity_type_id,
                )
                .is_some_and(|(data_id, default)| {
                    self.metadata_byte(id, data_id, default).unwrap_or(default) > 0
                });
        // Vanilla `VillagerRenderer` / `WanderingTraderRenderer.extractRenderState`:
        // `state.isUnhappy = entity.getUnhappyCounter() > 0`. PandaRenderer has the same assignment
        // for Panda.UNHAPPY_COUNTER, so intersect the renderer fact with the generated
        // AbstractVillager accessor owner rather than treating the shared field spelling as one type.
        let villager_unhappy =
            vanilla_entity_renderer_extracts_unhappy_counter(identity.entity_type_id)
                && vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type(
                    identity.entity_type_id,
                )
                .is_some_and(|(data_id, default)| {
                    self.metadata_int(id, data_id, default).unwrap_or(default) > 0
                });
        // Vanilla `EndermanModel.setupAnim`: a carried block (`!carriedBlock.isEmpty()`,
        // the synced `DATA_CARRY_STATE`) poses both arms forward, and `isCreepy` (the
        // synced `DATA_CREEPY`) drops the head / raises the hat. Only the enderman defines
        // these accessors, so the projections are gated to it and default off otherwise.
        let enderman_accessors = vanilla_entity_metadata_enderman_render_accessors_for_entity_type(
            identity.entity_type_id,
        );
        let enderman_carried_block_state_id = enderman_accessors.and_then(|accessors| {
            self.metadata_optional_block_state(id, accessors.carried_block_state_data_id)
                .unwrap_or(accessors.carried_block_state_default)
        });
        let enderman_carrying = enderman_carried_block_state_id.is_some();
        let enderman_carried_block = enderman_carried_block_state_id
            .and_then(|state_id| registries.block_state(state_id))
            .map(|state| EntityBlockModelState {
                name: state.name.clone(),
                properties: state.properties.clone(),
            });
        let enderman_creepy = enderman_accessors.is_some_and(|accessors| {
            self.metadata_bool(id, accessors.creepy_data_id, accessors.creepy_default)
                .unwrap_or(accessors.creepy_default)
        });
        // Vanilla `BatModel.setupAnim` swaps to the `BAT_RESTING` hanging pose while
        // `Bat.isResting` (`DATA_ID_FLAGS & FLAG_RESTING`). Only the bat defines that flags byte, so
        // the projection is gated to it and defaults to flying otherwise.
        let bat_resting =
            vanilla_entity_metadata_bat_flags_accessor_for_entity_type(identity.entity_type_id)
                .is_some_and(|(data_id, default, resting_mask)| {
                    self.metadata_byte(id, data_id, default).unwrap_or(default) & resting_mask != 0
                });
        // Vanilla `BeeModel.setupAnim` hides the stinger cube once `Bee.hasStung`
        // (`DATA_FLAGS_ID & 4`). Only the bee defines that flags byte; every other entity keeps
        // its stinger field at the `true` default (and never renders a stinger anyway).
        let bee_has_stinger =
            vanilla_entity_metadata_bee_accessors_for_entity_type(identity.entity_type_id)
                .is_none_or(|accessors| {
                    self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                        .unwrap_or(accessors.flags_default)
                        & accessors.has_stung_mask
                        == 0
                });
        // Vanilla `FoxModel.setupAnim` reads the fox's synced `DATA_FLAGS_ID` (19) crouch / sleep /
        // sit / pounce / faceplant bits directly (no easing) to pick its pose branch. Only the fox
        // defines that flags byte, so the five bool projections are gated to it and default off. The
        // interest bit (8) is read by the animation layer instead, to drive the `headRollAngle`
        // accumulator below.
        let fox_accessors =
            vanilla_entity_metadata_fox_accessors_for_entity_type(identity.entity_type_id);
        let fox_flags = fox_accessors
            .map(|accessors| {
                self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                    .unwrap_or(accessors.flags_default)
            })
            .unwrap_or(0);
        let fox_is_crouching =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.crouching_mask != 0);
        let fox_is_sleeping =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.sleeping_mask != 0);
        let fox_is_sitting =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.sitting_mask != 0);
        let fox_is_pouncing =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.pouncing_mask != 0);
        let fox_is_faceplanted =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.faceplanted_mask != 0);
        let is_feline = vanilla_entity_renderer_uses_abstract_feline_model(identity.entity_type_id);
        // Vanilla `CatRenderer` / `OcelotRenderer.extractRenderState` copies
        // `Entity.isCrouching()` and `Entity.isSprinting()` into `FelineRenderState`. The former is the
        // synced `Pose.CROUCHING`; the latter is shared entity flags bit 3. `CatRenderer` additionally
        // projects `Cat.isInSittingPose()` from the generated TamableAnimal sitting mask;
        // `OcelotRenderer` leaves `isSitting` at its default.
        let feline_is_crouching = is_feline
            && entity_data_pose(&metadata.data_values) == VANILLA_ENTITY_POSE_CROUCHING_ID;
        let feline_is_sprinting = is_feline && shared_flags & entity_accessors.sprinting_mask != 0;
        let feline_is_sitting = vanilla_is_cat(identity.entity_type_id)
            && vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
                identity.entity_type_id,
            )
            .is_some_and(|accessors| {
                self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                    .unwrap_or(accessors.flags_default)
                    & accessors.sitting_mask
                    != 0
            });
        // Vanilla `VexModel.setupAnim` levels the body (`xRot = 0`) and raises both arms
        // (`setArmsCharging`) while `Vex.isCharging` (`DATA_FLAGS_ID & FLAG_IS_CHARGING`). The
        // generated accessor owns the Vex membership, wire id, default, and named mask, so every
        // other type stays idle.
        let vex_charging =
            vanilla_entity_metadata_vex_flags_accessor_for_entity_type(identity.entity_type_id)
                .map(|(data_id, default, charging_mask)| {
                    self.metadata_byte(id, data_id, default).unwrap_or(default) & charging_mask != 0
                })
                .unwrap_or(false);
        // Vanilla `SnowGolemRenderer.extractRenderState`: when `SnowGolem.hasPumpkin()` is true, the
        // renderer resolves `Blocks.CARVED_PUMPKIN.defaultBlockState()` into `state.headBlock`. The
        // generated synced-byte default has the pumpkin mask set, so a newly spawned golem shows the
        // pumpkin until the server clears that bit.
        let snow_golem_pumpkin =
            vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
                identity.entity_type_id,
            )
            .is_some_and(|(data_id, default, pumpkin_mask)| {
                self.metadata_byte(id, data_id, default).unwrap_or(default) & pumpkin_mask != 0
            });
        // Vanilla `WitherBossRenderer.extractRenderState`: `state.invulnerableTicks =
        // invulnerableTicks > 0 ? invulnerableTicks - partialTicks : 0.0` (the synced `DATA_ID_INV`
        // spawn countdown, lerped against the partial tick). It drives both the `scale()` spawn-charge
        // shrink and the `getTextureLocation` `wither_invulnerable.png` flicker. Only the wither
        // defines that accessor, so every other entity holds `0.0`.
        let wither_invulnerable_accessor =
            vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type(
                identity.entity_type_id,
            );
        let wither_invulnerable_ticks =
            if let Some((data_id, default)) = wither_invulnerable_accessor {
                let ticks = self.metadata_int(id, data_id, default).unwrap_or(default);
                if ticks > 0 {
                    ticks as f32 - partial_ticks
                } else {
                    0.0
                }
            } else {
                0.0
            };
        // Vanilla `LivingEntity.isAutoSpinAttack` (the named spin-attack bit in
        // `DATA_LIVING_ENTITY_FLAGS`):
        // a living entity mid riptide-trident spin. Non-living entities have no
        // living-entity flags byte, so they never spin.
        let living_accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            identity.entity_type_id,
        );
        let is_auto_spin_attack = living_accessors.is_some_and(|accessors| {
            self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                .unwrap_or(accessors.flags_default)
                & accessors.spin_attack_mask
                != 0
        });
        // Vanilla `LivingEntity.isUsingItem` plus `getUsedItemHand` read the named using/off-hand
        // bits in `DATA_LIVING_ENTITY_FLAGS`; use-item arm poses (spyglass / horn / brush) read both. Non-living
        // entities have no flags byte, so they never count as using an item.
        let (living_entity_flags, is_using_mask, off_hand_mask) = living_accessors
            .map(|accessors| {
                (
                    self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                        .unwrap_or(accessors.flags_default),
                    accessors.is_using_mask,
                    accessors.off_hand_mask,
                )
            })
            .unwrap_or((0, 0, 0));
        let is_using_item = living_entity_flags & is_using_mask != 0;
        let use_item_off_hand = living_entity_flags & off_hand_mask != 0;
        let main_arm_left = self.entity_main_arm_left(id, identity.entity_type_id);
        // Vanilla `LivingEntityRenderer.isEntityUpsideDown`: a non-player living
        // entity whose custom name is `Dinnerbone`/`Grumm`. The player variant
        // (`AvatarRenderer.isPlayerUpsideDown`) keys off the GameProfile name and
        // the cape model part instead, which need the player-info list, so the
        // player type is excluded here and that path stays deferred.
        let is_upside_down = vanilla_living_entity_type(identity.entity_type_id)
            && identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
            && self
                .metadata_optional_component(id, entity_accessors.custom_name_data_id)
                .is_some_and(|name| {
                    vanilla_living_entity_renderer_name_is_upside_down(name.as_str())
                });
        // Vanilla `EntityRenderState.boundingBoxWidth` / `boundingBoxHeight`
        // project the current dimensions even for non-pickable renderers. Keep
        // this separate from `pick_bounds`, whose applicability filter would
        // collapse those entities to zero-sized flames.
        let bounding_box = vanilla_model_source_bounds_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(client_animations.animations),
            metadata.painting_variant_projection.as_ref(),
        );
        let bounding_box_height = bounding_box
            .map(|bounds| bounds.max[1] - bounds.min[1])
            .unwrap_or(0.0);
        let bounding_box_width = bounding_box
            .map(|bounds| bounds.max[0] - bounds.min[0])
            .unwrap_or(0.0);
        // Vanilla `LivingEntityRenderState.hasPose(Pose.SLEEPING)`: only living
        // entities lie down. The bed orientation/offset are resolved spatially by
        // the WorldStore aggregation (which owns the block data); the per-entity
        // source defaults to the no-bed fallback.
        let is_sleeping = vanilla_living_entity_type(identity.entity_type_id)
            && entity_data_pose(&metadata.data_values) == VANILLA_ENTITY_POSE_SLEEPING_ID;
        // Vanilla `Entity.isCrouching` (`Pose.CROUCHING`): AvatarRenderer consumes this for both
        // real players and client mannequins (the latter accepts CROUCHING in `VALID_POSES`).
        let is_crouching = matches!(
            identity.entity_type_id,
            VANILLA_ENTITY_TYPE_PLAYER_ID | VANILLA_ENTITY_TYPE_MANNEQUIN_ID
        ) && entity_data_pose(&metadata.data_values)
            == VANILLA_ENTITY_POSE_CROUCHING_ID;
        let is_fall_flying = vanilla_living_entity_type(identity.entity_type_id)
            && entity_is_fall_flying(&metadata.data_values);
        let pose = entity_data_pose(&metadata.data_values);
        let is_visually_swimming = vanilla_living_entity_type(identity.entity_type_id)
            && (pose == VANILLA_ENTITY_POSE_SWIMMING_ID
                || (pose == VANILLA_ENTITY_POSE_FALL_FLYING_ID && !is_fall_flying));
        let humanoid_speed_value = if is_fall_flying {
            let length_squared = (transform.delta_movement.x * transform.delta_movement.x
                + transform.delta_movement.y * transform.delta_movement.y
                + transform.delta_movement.z * transform.delta_movement.z)
                as f32;
            let normalized = length_squared / 0.2;
            (normalized * normalized * normalized).max(1.0)
        } else {
            1.0
        };
        // Vanilla `LivingEntityRenderState.scale` (`LivingEntity.getScale`, the SCALE
        // attribute): only living entities carry a render scale; everything else
        // renders at its default size.
        let scale = if vanilla_entity_metadata_type_is_living(identity.entity_type_id) {
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
        let (
            player_fall_flying_time_in_ticks,
            player_should_apply_flying_y_rot,
            player_flying_y_rot,
        ) = if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            let look_angle = calculate_view_vector(
                transform.render_x_rot(partial_ticks),
                transform.render_y_rot(partial_ticks),
            );
            let (should_apply, y_rot) = avatar_flight_y_rotation(
                client_animations
                    .animations
                    .player_previous_delta_movement(),
                transform.delta_movement,
                look_angle,
                partial_ticks,
            );
            (
                client_animations
                    .animations
                    .player_fall_flying_time(partial_ticks),
                should_apply,
                y_rot,
            )
        } else {
            (0.0, false, 0.0)
        };
        // Vanilla `AvatarRenderer.extractRenderState` copies the two inherited LivingEntity INT
        // accessors into `AvatarRenderState`; only the player renderer registers their layers.
        // Preserve the synchronized values here (including hostile/negative values); the renderer
        // owns the per-frame non-positive gate and bounded submission limit.
        let (player_arrow_count, player_stinger_count) =
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
                living_accessors
                    .map(|accessors| {
                        (
                            entity_data_int(
                                &metadata.data_values,
                                accessors.arrow_count_data_id,
                                accessors.arrow_count_default,
                            ),
                            entity_data_int(
                                &metadata.data_values,
                                accessors.stinger_count_data_id,
                                accessors.stinger_count_default,
                            ),
                        )
                    })
                    .unwrap_or((0, 0))
            } else {
                (0, 0)
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
            boat_paddle_states(
                identity.entity_type_id,
                &metadata.data_values,
                !mount.passengers.is_empty(),
            )
        });
        let boat_rowing_times = client_animations.animations.boat_rowing_times(
            partial_ticks,
            boat_paddles[0],
            boat_paddles[1],
        );
        let (boat_hurt_time, boat_hurt_dir, boat_damage_time) = client_animations
            .animations
            .boat_damage_state(partial_ticks);
        // `AbstractMinecart` chooses NewMinecartBehavior from the level's
        // enabled feature set at construction, before any step packet arrives.
        let minecart_new_render =
            is_vanilla_minecart_type(identity.entity_type_id) && minecart_improvements_enabled;
        let mut source_position = position;
        let is_shulker = identity.entity_type_id == VANILLA_ENTITY_TYPE_SHULKER_ID;
        let render_head_y_rot = if is_shulker {
            transform.y_head_rot
        } else {
            transform.render_y_head_rot(partial_ticks)
        };
        let mut source_y_rot = if client_animations.animations.age_ticks == 0 {
            client_animations
                .animations
                .spawn_body_y_rot
                .unwrap_or_else(|| transform.render_body_y_rot(partial_ticks))
        } else if is_shulker {
            transform.y_rot
        } else if vanilla_living_entity_type(identity.entity_type_id) {
            transform.render_body_y_rot(partial_ticks)
        } else {
            transform.render_y_rot(partial_ticks)
        };
        if vanilla_living_entity_type(identity.entity_type_id) {
            if let Some(vehicle_body_y_rot) = mount
                .as_ref()
                .and_then(|mount| mount.vehicle_id)
                .and_then(|vehicle_id| self.living_body_render_y_rot(vehicle_id, partial_ticks))
            {
                source_y_rot =
                    solve_living_passenger_body_y_rot(render_head_y_rot, vehicle_body_y_rot);
            }
        }
        let mut source_x_rot = transform.render_x_rot(partial_ticks);
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
        if is_shulker {
            if let Some(offset) = client_animations.animations.shulker_render_offset(
                transform.position,
                partial_ticks,
                scale,
            ) {
                source_position = add_entity_vec3(source_position, offset);
            }
        }
        let boat_bubble_angle = client_animations
            .animations
            .boat_bubble_angle(partial_ticks);
        let (wither_x_head_rots, wither_y_head_rots) = wither_invulnerable_accessor
            .map(|_| client_animations.animations.wither_head_rotations())
            .unwrap_or(([0.0; 2], [0.0; 2]));
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
        let squid_y_body_rot = if vanilla_entity_type_is_squid(identity.entity_type_id) {
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
            distance_position: position,
            position: source_position,
            y_rot: source_y_rot,
            x_rot: source_x_rot,
            y_head_rot: render_head_y_rot,
            arrow_shake: client_animations.animations.arrow_shake(partial_ticks),
            is_passenger,
            age_ticks: if identity.entity_type_id == VANILLA_ENTITY_TYPE_END_CRYSTAL_ID {
                client_animations.animations.end_crystal_age_ticks()
            } else {
                client_animations.animations.age_ticks
            },
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
            illager_spellcasting,
            villager_unhappy,
            enderman_carrying,
            enderman_carried_block,
            enderman_creepy,
            bat_resting,
            bat_flying_seconds: client_animations
                .animations
                .bat_flying_seconds(partial_ticks),
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
            is_fall_flying,
            is_visually_swimming,
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
            player_fall_flying_time_in_ticks,
            player_should_apply_flying_y_rot,
            player_flying_y_rot,
            player_arrow_count,
            player_stinger_count,
            player_left_shoulder_parrot,
            player_right_shoulder_parrot,
            show_extra_ears: false,
            is_auto_spin_attack,
            is_using_item,
            use_item_off_hand,
            is_upside_down,
            bounding_box_height,
            bounding_box_width,
            display_fire_animation: shared_flags & entity_accessors.on_fire_mask != 0
                && vanilla_entity_type(identity.entity_type_id)
                    .is_some_and(|entry| !entry.fire_immune),
            is_sleeping,
            sleeping_bed_yaw: None,
            sleeping_bed_offset: [0.0, 0.0],
            scale,
            age_scale: vanilla_age_scale(identity.entity_type_id, &metadata.data_values),
            swim_amount: client_animations
                .animations
                .living_swim_amount(partial_ticks),
            humanoid_speed_value,
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
            painting_variant_projection: (*metadata.painting_variant_projection).clone(),
            painting_facing: vanilla_entity_metadata_hanging_direction_accessor_for_entity_type(
                identity.entity_type_id,
            )
            .and_then(|accessor| {
                super::dimensions::painting_facing(accessor, identity.data, &metadata.data_values)
            }),
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
            // The generated accessor gates the `canMove` read on Creaking membership and defaults
            // every other entity to `true`, because this field only controls the Creaking model.
            creaking_can_move: creaking_can_move(identity.entity_type_id, &metadata.data_values),
            creaking_eyes_glowing: client_animations.animations.creaking_eyes_glowing(),
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
            // Vanilla `AxolotlRenderer.extractRenderState` copies the seven baby AnimationStates.
            // Stopped timers project `-1`; exactly one timer runs while `isBaby` ticks.
            axolotl_baby_swim_seconds: client_animations
                .animations
                .axolotl_baby_swim_seconds(partial_ticks),
            axolotl_baby_walk_seconds: client_animations
                .animations
                .axolotl_baby_walk_seconds(partial_ticks),
            axolotl_baby_walk_underwater_seconds: client_animations
                .animations
                .axolotl_baby_walk_underwater_seconds(partial_ticks),
            axolotl_baby_idle_underwater_seconds: client_animations
                .animations
                .axolotl_baby_idle_underwater_seconds(partial_ticks),
            axolotl_baby_idle_underwater_on_ground_seconds: client_animations
                .animations
                .axolotl_baby_idle_underwater_on_ground_seconds(partial_ticks),
            axolotl_baby_idle_on_ground_seconds: client_animations
                .animations
                .axolotl_baby_idle_on_ground_seconds(partial_ticks),
            axolotl_baby_play_dead_seconds: client_animations
                .animations
                .axolotl_baby_play_dead_seconds(partial_ticks),
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
            head_armor_trim: armor_trim(ProtocolEquipmentSlot::Head),
            chest_armor_trim: armor_trim(ProtocolEquipmentSlot::Chest),
            legs_armor_trim: armor_trim(ProtocolEquipmentSlot::Legs),
            feet_armor_trim: armor_trim(ProtocolEquipmentSlot::Feet),
            head_armor_dye: armor_dye(ProtocolEquipmentSlot::Head),
            chest_armor_dye: armor_dye(ProtocolEquipmentSlot::Chest),
            legs_armor_dye: armor_dye(ProtocolEquipmentSlot::Legs),
            feet_armor_dye: armor_dye(ProtocolEquipmentSlot::Feet),
            head_armor_foil: armor_foil(ProtocolEquipmentSlot::Head),
            chest_armor_foil: armor_foil(ProtocolEquipmentSlot::Chest),
            legs_armor_foil: armor_foil(ProtocolEquipmentSlot::Legs),
            feet_armor_foil: armor_foil(ProtocolEquipmentSlot::Feet),
            chest_wings_trim,
            chest_wings_foil,
            wolf_body_armor,
            wolf_body_armor_trim,
            wolf_body_armor_dye,
            wolf_body_armor_crackiness,
            wolf_body_armor_foil,
            pig_saddle,
            pig_saddle_trim,
            pig_saddle_foil,
            equine_saddle,
            equine_saddle_trim,
            equine_saddle_foil,
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
            equine_body_armor_trim: horse_body_armor_trim,
            equine_body_armor_dye: horse_body_armor_dye,
            equine_body_armor_foil: horse_body_armor_foil,
            strider_ridden,
            strider_saddle,
            strider_saddle_trim,
            strider_saddle_foil,
            camel_saddle,
            camel_saddle_trim,
            camel_saddle_foil,
            camel_saddle_ridden,
            nautilus_saddle,
            nautilus_saddle_trim,
            nautilus_saddle_foil,
            happy_ghast_body_item,
            happy_ghast_harness,
            happy_ghast_harness_trim,
            happy_ghast_harness_foil,
            happy_ghast_body_item_is_harness,
            happy_ghast_is_leash_holder,
            happy_ghast_ridden,
            guardian_beam: self.guardian_beam_source(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                client_animations.animations,
                position,
                partial_ticks,
            ),
            // Filled by `WorldStore::project_entity_model_source`; eye tracking selects either the
            // current camera entity or an active cross-entity attack target.
            guardian_eye_tracking: None,
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
            // Filled by `WorldStore::project_entity_model_source`; the rope projection
            // needs the world light probe the store cannot reach.
            leash_ropes: Vec::new(),
            // `distance_to_egg` is likewise finished by the projection layer's heightmap probe.
            ender_dragon_animation: client_animations
                .animations
                .ender_dragon_animation_source(partial_ticks),
            llama_body_decor,
            llama_body_decor_trim,
            llama_body_decor_foil,
            nautilus_body_armor,
            nautilus_body_armor_trim,
            nautilus_body_armor_foil,
            data_values: (*metadata.data_values).clone(),
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
        if !vanilla_entity_metadata_type_is_guardian(entity_type_id) {
            return None;
        }
        let attack_target_id = guardian_attack_target_id(entity_type_id, data_values);
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
            None,
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
        let accessors =
            vanilla_entity_metadata_end_crystal_accessors_for_entity_type(entity_type_id)?;
        let target = data_values
            .iter()
            .find(|value| value.data_id == accessors.beam_target_data_id)
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
            let crystal_age =
                client_animations.animations.end_crystal_age_ticks() as f32 + partial_ticks;
            let crystal_y = transform.position.y + f64::from(end_crystal_renderer_y(crystal_age));
            nearest = Some((
                distance_sqr,
                [dx as f32, (crystal_y - position.y) as f32, dz as f32],
            ));
        }

        nearest.map(|(_, beam_offset)| super::EnderDragonBeamSource { beam_offset })
    }

    pub(crate) fn camera_pose_state(&self, id: i32) -> Option<EntityCameraPoseState> {
        self.camera_pose_state_at_partial_tick(id, 1.0)
    }

    pub(crate) fn camera_pose_state_at_partial_tick(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<EntityCameraPoseState> {
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
            metadata.painting_variant_projection.as_ref(),
        )?;
        Some(EntityCameraPoseState {
            id: identity.id,
            position: transform.render_position(partial_ticks),
            y_rot: transform.render_y_rot(partial_ticks),
            x_rot: transform.render_x_rot(partial_ticks),
            eye_height,
        })
    }

    pub(crate) fn camera_hurt_state(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<EntityCameraHurtState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if !vanilla_entity_metadata_type_is_living(identity.entity_type_id) {
            return None;
        }
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        let (hurt_time, hurt_duration) = client_animations
            .animations
            .camera_hurt_time(partial_ticks)
            .unwrap_or((-1.0, 10.0));
        let death_time = client_animations.animations.death_time(partial_ticks);
        let hurt_dir = if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            self.ecs
                .get::<&EntityTransientEvents>(entity)
                .ok()
                .and_then(|events| events.last_hurt_yaw)
                .unwrap_or(0.0)
        } else {
            // `LivingEntity.getHurtDir()` is zero; only `Player` overrides it.
            0.0
        };
        Some(EntityCameraHurtState {
            hurt_time,
            hurt_duration,
            hurt_dir,
            death_time,
            is_dead_or_dying: death_time > 0.0,
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

    pub(crate) fn living_entity_previous_feet_position(
        &self,
        id: i32,
    ) -> Option<Option<super::EntityVec3>> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if !vanilla_living_entity_type(identity.entity_type_id) {
            return None;
        }
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(
            client_animations
                .animations
                .walk_animation
                .and_then(|walk| walk.previous_feet_position),
        )
    }

    fn wither_head_targets_by_id(&self) -> HashMap<i32, [WitherHeadTargetRotations; 2]> {
        let mut withers = Vec::new();
        let mut query = self.ecs.query::<(
            &EntityIdentity,
            &EntityTransform,
            &EntityMetadata,
            &EntityAttributes,
        )>();
        for (_, (identity, transform, metadata, attributes)) in query.iter() {
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_WITHER_ID {
                continue;
            }

            withers.push((
                identity.id,
                transform.position,
                transform.y_rot,
                vanilla_render_scale(identity.entity_type_id, &attributes.attributes),
                wither_side_head_target_ids(identity.entity_type_id, &metadata.data_values),
            ));
        }
        drop(query);

        let mut eye_positions = HashMap::new();
        for target_id in withers
            .iter()
            .flat_map(|(_, _, _, _, target_ids)| target_ids)
            .copied()
            .filter(|target_id| *target_id > 0)
        {
            if eye_positions.contains_key(&target_id) {
                continue;
            }
            let Some(entity) = self.by_protocol_id.get(&target_id).copied() else {
                continue;
            };
            let (Ok(identity), Ok(transform), Ok(metadata), Ok(attributes), Ok(client_animations)) = (
                self.ecs.get::<&EntityIdentity>(entity),
                self.ecs.get::<&EntityTransform>(entity),
                self.ecs.get::<&EntityMetadata>(entity),
                self.ecs.get::<&EntityAttributes>(entity),
                self.ecs.get::<&EntityClientAnimations>(entity),
            ) else {
                continue;
            };
            if let Some(eye_height) = vanilla_eye_height_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(client_animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) {
                eye_positions.insert(target_id, (transform.position, eye_height));
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

    pub(crate) fn debug_passenger_vehicle_target(
        &self,
        passenger_id: i32,
        game_time: i64,
    ) -> Option<super::EntityPickTargetState> {
        let passenger_identity = self.identity(passenger_id)?;
        let passenger_bounds = self.pick_bounds(passenger_id)?;
        let passenger_mount = self.mount(passenger_id)?;
        let vehicle_id = passenger_mount.vehicle_id?;
        let vehicle_bounds = self.pick_bounds(vehicle_id)?;
        let vehicle_entity = self.by_protocol_id.get(&vehicle_id).copied()?;
        let Ok(mut query) = self.ecs.query_one::<(
            &EntityIdentity,
            &EntityTransform,
            &EntityMount,
            &EntityMetadata,
            &EntityAttributes,
            Option<&EntityClientAnimations>,
        )>(vehicle_entity) else {
            return None;
        };
        let Some((
            vehicle_identity,
            vehicle_transform,
            vehicle_mount,
            vehicle_metadata,
            vehicle_attributes,
            vehicle_client_animations,
        )) = query.get()
        else {
            return None;
        };
        let (position, bounds) = vanilla_passenger_vehicle_debug_target_for_entity_data(
            vehicle_identity.entity_type_id,
            vehicle_transform.position,
            vehicle_transform.y_rot,
            vehicle_bounds,
            &vehicle_metadata.data_values,
            &vehicle_attributes.attributes,
            vehicle_client_animations.map(|animations| animations.animations),
            &vehicle_mount.passengers,
            passenger_id,
            passenger_identity.entity_type_id,
            passenger_bounds,
            game_time,
        )?;
        Some(super::EntityPickTargetState {
            entity_id: passenger_id,
            position,
            bounds,
        })
    }

    pub(crate) fn ender_dragon_part_parent_id(&self, part_id: i32) -> Option<i32> {
        ender_dragon_part_parent_id(part_id, |parent_id| {
            self.identity(parent_id).is_some_and(|identity| {
                identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
            })
        })
    }

    #[cfg(test)]
    pub(crate) fn leash(&self, id: i32) -> Option<EntityLeash> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityLeash>(entity)
            .ok()
            .map(|leash| *leash)
    }

    pub(crate) fn leash_holder_id(&self, id: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs.get::<&EntityLeash>(entity).ok()?.holder_id
    }

    pub(crate) fn leash_endpoint_inputs(
        &self,
        id: i32,
        partial_ticks: f32,
        game_time: i64,
        minecart_improvements_enabled: bool,
    ) -> Option<super::leash::LeashEndpointInputs> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let entity_type_id = self.ecs.get::<&EntityIdentity>(entity).ok()?.entity_type_id;
        let transform = *self.ecs.get::<&EntityTransform>(entity).ok()?;
        let bounds = self.debug_hitbox_bounds(id)?;
        let eye_height = self
            .camera_pose_state_at_partial_tick(id, partial_ticks)?
            .eye_height;
        let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
        let shared_flags = self
            .metadata_byte(
                id,
                entity_accessors.shared_flags_data_id,
                entity_accessors.shared_flags_default,
            )
            .unwrap_or(entity_accessors.shared_flags_default);
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let pose = metadata
            .data_values
            .iter()
            .find(|value| value.data_id == entity_accessors.pose_data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::Pose(pose) => Some(*pose),
                _ => None,
            })
            .unwrap_or(entity_accessors.pose_default);
        let is_auto_spin_attack =
            vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)
                .is_some_and(|accessors| {
                    self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                        .unwrap_or(accessors.flags_default)
                        & accessors.spin_attack_mask
                        != 0
                });
        // Vanilla `Avatar.DEFAULT_MAIN_HAND` is right (id 1).
        let main_arm_right = self
            .metadata_humanoid_arm(id, VANILLA_AVATAR_MAIN_HAND_DATA_ID, 1)
            .unwrap_or(1)
            != 0;
        Some(super::leash::LeashEndpointInputs {
            entity_type_id,
            leash_offset_owner: vanilla_entity_metadata_leash_offset_owner(entity_type_id),
            rope_hold_position_owner: vanilla_entity_metadata_rope_hold_position_owner(
                entity_type_id,
            )?,
            position: self.render_position_at_partial_tick(
                id,
                partial_ticks,
                game_time,
                minecart_improvements_enabled,
            )?,
            body_y_rot: transform.render_body_y_rot(partial_ticks),
            // Vanilla `Player.getRopeHoldPosition` pitch is
            // `Mth.lerp(partialTicks * 0.5, getXRot(), xRotO)` — the reversed
            // half lerp equals the ordinary render lerp at `1 - 0.5 * t`.
            rope_hold_x_rot: transform.render_x_rot(1.0 - 0.5 * partial_ticks.clamp(0.0, 1.0)),
            view_x_rot: transform.render_x_rot(partial_ticks),
            view_y_rot: transform.render_y_rot(partial_ticks),
            bounds_width: bounds.max[0] - bounds.min[0],
            bounds_height: bounds.max[1] - bounds.min[1],
            eye_height,
            camel_leash_offset: vanilla_camel_leash_offset_for_entity_data(
                entity_type_id,
                &metadata.data_values,
                &attributes.attributes,
                game_time,
                partial_ticks,
            ),
            on_fire: shared_flags & entity_accessors.on_fire_mask != 0,
            fall_flying_or_spin: shared_flags & entity_accessors.fall_flying_mask != 0
                || is_auto_spin_attack,
            visually_swimming: pose == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID,
            crouching: pose == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID,
            main_arm_right,
            delta_movement: transform.delta_movement,
        })
    }

    pub(crate) fn mob_effects(&self, id: i32) -> Option<EntityMobEffects> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityMobEffects>(entity)
            .ok()
            .map(|effects| (*effects).clone())
    }

    pub(crate) fn mob_effect_amplifier(&self, id: i32, effect_id: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let effects = self.ecs.get::<&EntityMobEffects>(entity).ok()?;
        effects
            .effects
            .get(&effect_id)
            .map(|effect| effect.amplifier)
    }

    pub(crate) fn damage(&self, id: i32) -> Option<EntityDamage> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityDamage>(entity)
            .ok()
            .map(|damage| (*damage).clone())
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
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let data_id = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
            identity.entity_type_id,
        )?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        metadata_item_stack(&metadata.data_values, data_id)
            .filter(|stack| stack.item_id.is_some() && stack.count > 0)
            .cloned()
    }

    pub(crate) fn raw_item_stack_for_entity(&self, id: i32) -> Option<ItemStackSummary> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let accessor = vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer(
            identity.entity_type_id,
            "ITEM_STACK",
        )?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        metadata_item_stack(&metadata.data_values, accessor.data_id).cloned()
    }

    pub(crate) fn entity_age_ticks(&self, id: i32) -> Option<u32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(client_animations.animations.age_ticks)
    }

    pub(crate) fn item_bob_offset(&self, id: i32) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        (identity.entity_type_id == VANILLA_ENTITY_TYPE_ITEM_ID).then_some(())?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(client_animations.animations.item_bob_offset())
    }

    pub(crate) fn experience_orb_icon_for_entity(&self, id: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let (data_id, default) =
            vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type(
                identity.entity_type_id,
            )?;
        let value = self
            .ecs
            .get::<&EntityMetadata>(entity)
            .ok()
            .and_then(|metadata| {
                metadata
                    .data_values
                    .iter()
                    .find(|value| value.data_id == data_id)
                    .and_then(|value| match &value.value {
                        EntityDataValueKind::Int(value) => Some(*value),
                        _ => None,
                    })
            })
            .unwrap_or(default);
        Some(super::experience_orb_icon(value))
    }

    /// The render state of every item-frame / glow-item-frame entity: its resolved wall center, the
    /// facing wall, the `0..=7` item rotation, glow/invisible flags, the framed item, and framed map id.
    /// Drives the 3D item-frame render (vanilla `ItemFrameRenderer`).
    pub(crate) fn item_frame_render_states(&self) -> Vec<ItemFrameRenderState> {
        let mut frames = Vec::new();
        let entity_accessors = vanilla_entity_metadata_entity_accessors()
            .expect("generated base Entity metadata accessors must resolve");
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            let Some(accessors) = vanilla_entity_metadata_item_frame_accessors_for_entity_type(
                identity.entity_type_id,
            ) else {
                continue;
            };
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            let Ok(metadata) = self.ecs.get::<&EntityMetadata>(entity) else {
                continue;
            };
            frames.push(ItemFrameRenderState {
                entity_id: identity.id,
                center: transform.position,
                facing: item_frame_facing(accessors, identity.data, &metadata.data_values),
                light: super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
                rotation: item_frame_rotation(accessors, &metadata.data_values),
                glow: vanilla_entity_renderer_minimum_block_light(identity.entity_type_id)
                    .is_some(),
                invisible: self
                    .metadata_byte(
                        identity.id,
                        entity_accessors.shared_flags_data_id,
                        entity_accessors.shared_flags_default,
                    )
                    .unwrap_or(entity_accessors.shared_flags_default)
                    & entity_accessors.invisible_mask
                    != 0,
                item: item_frame_item(accessors, &metadata.data_values).cloned(),
                map_id: item_frame_map_id(accessors, &metadata.data_values),
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

    /// Collects the unique generated ITEM_STACK accessor carried by every requested entity type.
    /// This covers dropped items and owner-specific thrown-item projectile accessors without treating
    /// their coincident wire indices as shared ownership.
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
            let Ok(client_animations) = self.ecs.get::<&EntityClientAnimations>(entity) else {
                continue;
            };
            let Some(data_id) = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
                identity.entity_type_id,
            )
            .or_else(|| {
                vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer(
                    identity.entity_type_id,
                    "ITEM_STACK",
                )
                .map(|accessor| accessor.data_id)
            }) else {
                continue;
            };
            let Some(stack) = metadata_item_stack(&metadata.data_values, data_id)
                .filter(|stack| stack.item_id.is_some() && stack.count > 0)
            else {
                continue;
            };
            items.push(ItemEntityStackState {
                entity_id: identity.id,
                position: transform.position,
                light: super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
                bob_offset: client_animations.animations.item_bob_offset(),
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
            let Some(accessors) = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
                identity.entity_type_id,
            ) else {
                continue;
            };
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            let Ok(metadata) = self.ecs.get::<&EntityMetadata>(entity) else {
                continue;
            };
            if firework_rocket_attached_to_entity(&metadata.data_values, accessors) {
                continue;
            }
            let Some(stack) = firework_rocket_render_stack(&metadata.data_values, accessors) else {
                continue;
            };
            items.push(FireworkRocketItemState {
                entity_id: identity.id,
                position: transform.position,
                light: super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
                stack: stack.clone(),
                shot_at_angle: firework_rocket_shot_at_angle(&metadata.data_values, accessors),
            });
        }
        items
    }

    pub(crate) fn firework_rocket_explosion_particle_state(
        &self,
        id: i32,
    ) -> Option<FireworkRocketExplosionParticleState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let accessors = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
            identity.entity_type_id,
        )?;
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let stack = self
            .ecs
            .get::<&EntityMetadata>(entity)
            .ok()
            .and_then(|metadata| {
                firework_rocket_render_stack(&metadata.data_values, accessors).cloned()
            });
        let has_explosions = stack
            .as_ref()
            .is_some_and(firework_rocket_stack_has_explosions);
        Some(FireworkRocketExplosionParticleState {
            entity_id: identity.id,
            position: transform.position,
            delta_movement: transform.delta_movement,
            has_explosions,
            explosions: stack
                .map(|stack| stack.component_patch.fireworks_explosions)
                .unwrap_or_default(),
        })
    }

    pub(crate) fn fox_eat_particle_state(&self, id: i32) -> Option<FoxEatParticleState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_FOX_ID {
            return None;
        }
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let item_stack = self.equipment_item(id, ProtocolEquipmentSlot::MainHand)?;
        Some(FoxEatParticleState {
            entity_id: identity.id,
            position: transform.position,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            item_stack,
        })
    }

    /// Collects the ominous item spawner item-cluster render state. The generated
    /// `OminousItemSpawner.DATA_ITEM` accessor owns membership and the item-stack wire index.
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
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            let Ok(metadata) = self.ecs.get::<&EntityMetadata>(entity) else {
                continue;
            };
            let Ok(client_animations) = self.ecs.get::<&EntityClientAnimations>(entity) else {
                continue;
            };
            let Some(stack) =
                ominous_item_spawner_render_stack(identity.entity_type_id, &metadata.data_values)
            else {
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

    pub(crate) fn append_ominous_item_spawner_particle_states(&mut self) {
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
            self.pending_ominous_item_spawner_particles
                .push(OminousItemSpawnerParticleState {
                    entity_id: identity.id,
                    position: transform.position,
                });
        }
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

    #[cfg(test)]
    pub(crate) fn experience_orb_movement(&self, id: i32) -> Option<EntityExperienceOrbMovement> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityExperienceOrbMovement>(entity)
            .ok()
            .map(|movement| *movement)
    }

    #[cfg(test)]
    pub(crate) fn fishing_hook_movement(&self, id: i32) -> Option<EntityFishingHookMovement> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityFishingHookMovement>(entity)
            .ok()
            .map(|movement| *movement)
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

    pub(crate) fn boat_movement(&self, id: i32) -> Option<EntityBoatMovement> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityBoatMovement>(entity)
            .ok()
            .map(|movement| *movement)
    }

    pub(crate) fn with_boat_movement_transform_and_support_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(
            &mut EntityBoatMovement,
            &mut EntityTransform,
            &mut EntitySupportingBlock,
        ) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let (movement, transform, support) = self
            .ecs
            .query_one_mut::<(
                &mut EntityBoatMovement,
                &mut EntityTransform,
                &mut EntitySupportingBlock,
            )>(entity)
            .ok()?;
        Some(update(movement, transform, support))
    }

    pub(crate) fn with_transform_and_support_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityTransform, &mut EntitySupportingBlock) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let (transform, support) = self
            .ecs
            .query_one_mut::<(&mut EntityTransform, &mut EntitySupportingBlock)>(entity)
            .ok()?;
        Some(update(transform, support))
    }

    fn with_client_animations_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityClientAnimations) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        Some(update(&mut animations))
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

    /// Apply the client-observable body of Vanilla
    /// `LivingEntity.handleDamageEvent` in declaration order. Admission has
    /// already proved this is a living entity.
    pub(crate) fn apply_living_damage_event_presentation(
        &mut self,
        id: i32,
        event: EntityDamageEventState,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let (damage, animations) = self
            .ecs
            .query_one_mut::<(&mut EntityDamage, &mut EntityClientAnimations)>(entity)
            .ok()?;
        animations.animations.set_damage_event_walk_speed();
        damage.invulnerable_time = 20;
        animations.animations.trigger_hurt();
        *damage.last_damage = Some(event);
        Some(())
    }

    pub(crate) fn supporting_block(&self, id: i32) -> Option<EntitySupportingBlock> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntitySupportingBlock>(entity)
            .ok()
            .map(|support| *support)
    }

    pub(crate) fn with_supporting_block_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntitySupportingBlock) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut support = self.ecs.get::<&mut EntitySupportingBlock>(entity).ok()?;
        let result = update(&mut support);
        Some(result)
    }

    pub(crate) fn rebuild_local_player_entity_for_respawn(
        &mut self,
        id: i32,
        keep_entity_data: bool,
        keep_attribute_modifiers: bool,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let incarnation = self.allocate_incarnation();

        if let Ok(mut identity) = self.ecs.get::<&mut EntityIdentity>(entity) {
            identity.data = 0;
            identity.incarnation = incarnation;
        }
        if let Ok(mut transform) = self.ecs.get::<&mut EntityTransform>(entity) {
            let (delta_movement, y_rot, x_rot) = if keep_entity_data {
                (transform.delta_movement, transform.y_rot, transform.x_rot)
            } else {
                (EntityVec3::default(), -180.0, 0.0)
            };
            *transform = EntityTransform {
                position: EntityVec3::default(),
                old_position: EntityVec3::default(),
                position_base: EntityVec3::default(),
                delta_movement,
                y_rot,
                old_y_rot: y_rot,
                x_rot,
                old_x_rot: x_rot,
                y_head_rot: 0.0,
                old_y_head_rot: 0.0,
                on_ground: None,
                interpolation: None,
            };
        }
        if !keep_entity_data {
            if let Ok(mut metadata) = self.ecs.get::<&mut EntityMetadata>(entity) {
                *metadata = EntityMetadata::default();
            }
        }
        if let Ok(mut equipment) = self.ecs.get::<&mut EntityEquipment>(entity) {
            *equipment = EntityEquipment::default();
        }
        if let Ok(mut attributes) = self.ecs.get::<&mut EntityAttributes>(entity) {
            if !keep_attribute_modifiers {
                for attribute in &mut *attributes.attributes {
                    attribute.modifiers.clear();
                }
            }
        }
        if let Ok(mut events) = self.ecs.get::<&mut EntityTransientEvents>(entity) {
            *events = EntityTransientEvents::default();
        }
        if let Ok(mut mount) = self.ecs.get::<&mut EntityMount>(entity) {
            *mount = EntityMount::default();
        }
        if let Ok(mut leash) = self.ecs.get::<&mut EntityLeash>(entity) {
            *leash = EntityLeash::default();
        }
        if let Ok(mut effects) = self.ecs.get::<&mut EntityMobEffects>(entity) {
            *effects = EntityMobEffects::default();
        }
        if let Ok(mut animations) = self.ecs.get::<&mut EntityClientAnimations>(entity) {
            *animations = EntityClientAnimations::default();
        }
        if let Ok(mut damage) = self.ecs.get::<&mut EntityDamage>(entity) {
            *damage = EntityDamage::default();
        }
        if let Ok(mut support) = self.ecs.get::<&mut EntitySupportingBlock>(entity) {
            *support = EntitySupportingBlock::default();
        }
        if keep_entity_data {
            let _ = self.sync_client_animation_targets_from_metadata(id);
        }
        Some(())
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

    /// Applies `FishingHook.onSyncedDataUpdated` in packet field order. The
    /// hooked target is captured by UUID so a later protocol-id reuse cannot
    /// retarget an old hook instance, while the biting impulse consumes the
    /// hook's retained synchronized RNG cursor immediately like Vanilla.
    pub(crate) fn sync_fishing_hook_from_metadata_update(
        &mut self,
        id: i32,
        updated_values: &[bbb_protocol::packets::EntityDataValue],
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        if self.ecs.get::<&EntityFishingHookMovement>(entity).is_err() {
            return None;
        }
        for value in updated_values {
            if value.data_id == VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID {
                let EntityDataValueKind::Int(target_data) = &value.value else {
                    continue;
                };
                let hooked_entity = target_data
                    .to_owned()
                    .checked_sub(1)
                    .and_then(|target_id| self.by_protocol_id.get(&target_id).copied())
                    .and_then(|target| self.ecs.get::<&EntityIdentity>(target).ok())
                    .map(|identity| (identity.uuid, identity.incarnation));
                let mut movement = self
                    .ecs
                    .get::<&mut EntityFishingHookMovement>(entity)
                    .ok()?;
                movement.hooked_entity_uuid = hooked_entity.map(|(uuid, _)| uuid);
                movement.hooked_entity_incarnation =
                    hooked_entity.map(|(_, incarnation)| incarnation);
            } else if value.data_id == VANILLA_FISHING_HOOK_BITING_DATA_ID {
                let EntityDataValueKind::Boolean(biting) = &value.value else {
                    continue;
                };
                let Ok((movement, transform)) = self
                    .ecs
                    .query_one_mut::<(&mut EntityFishingHookMovement, &mut EntityTransform)>(
                        entity,
                    )
                else {
                    return None;
                };
                movement.biting = *biting;
                if *biting {
                    let random = super::animations::java_random_next_float(
                        &mut movement.synchronized_random_seed,
                    );
                    let downward = -0.4_f32 * (0.6_f32 + random * (1.0_f32 - 0.6_f32));
                    transform.delta_movement.y = f64::from(downward);
                }
            }
        }
        Some(())
    }

    pub(crate) fn apply_client_animation_entity_event(
        &mut self,
        id: i32,
        event_id: i8,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let entity_type_id = self.ecs.get::<&EntityIdentity>(entity).ok()?.entity_type_id;
        let event_kind = vanilla_entity_event_kind(entity_type_id, event_id)?;
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations
            .animations
            .handle_entity_event(entity_type_id, id, event_kind);
        Some(())
    }

    #[cfg(test)]
    pub(crate) fn minecart_spawner_spawn_delay(&self, id: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID {
            return None;
        }
        let animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(animations.animations.minecart_spawner_spawn_delay())
    }

    /// Applies the canonical state mutation from `LivingEntity.handleEntityEvent(3)`.
    ///
    /// Vanilla plays the death sound for Players too, but deliberately skips `setHealth(0)` and
    /// `die(generic)` for every `Player` instance. The caller owns the sound side effect; this
    /// method owns only the non-Player health/death-state transition.
    pub(crate) fn apply_client_living_death_entity_event(&mut self, id: i32) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let entity_type_id = self.ecs.get::<&EntityIdentity>(entity).ok()?.entity_type_id;
        let accessors =
            vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)?;
        if vanilla_entity_metadata_type_uses_player_entity_event_handler(entity_type_id) {
            return Some(());
        }

        {
            let mut metadata = self.ecs.get::<&mut EntityMetadata>(entity).ok()?;
            let health = ProtocolEntityDataValue {
                data_id: accessors.health_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(0.0),
            };
            if let Some(existing) = metadata
                .data_values
                .iter_mut()
                .find(|value| value.data_id == accessors.health_data_id)
            {
                *existing = health;
            } else {
                metadata.data_values.push(health);
                metadata.data_values.sort_by_key(|value| value.data_id);
            }
        }

        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?.clone();
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations
            .animations
            .sync_targets_from_metadata(entity_type_id, &metadata.data_values);
        Some(())
    }

    pub(crate) fn apply_client_animation_hurt(&mut self, id: i32) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let entity_type_id = self.ecs.get::<&EntityIdentity>(entity).ok()?.entity_type_id;
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        if vanilla_entity_metadata_type_is_living(entity_type_id) {
            animations.animations.trigger_hurt();
            Some(())
        } else if is_vanilla_vehicle_entity_type(entity_type_id) {
            animations.animations.trigger_vehicle_hurt();
            Some(())
        } else {
            None
        }
    }

    pub(crate) fn lerp_head_to(
        &mut self,
        id: i32,
        target_y_head_rot: f32,
        steps: i32,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let entity_type_id = self.ecs.get::<&EntityIdentity>(entity).ok()?.entity_type_id;
        if vanilla_entity_metadata_type_is_living(entity_type_id) {
            let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
            animations
                .animations
                .start_head_rotation_lerp(target_y_head_rot, steps);
        } else {
            let mut transform = self.ecs.get::<&mut EntityTransform>(entity).ok()?;
            transform.y_head_rot = target_y_head_rot;
        }
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

    pub(crate) fn interpolation_collision_inputs(&self) -> Vec<EntityInterpolationCollisionInput> {
        let mut inputs = Vec::new();
        let mut query = self.ecs.query::<(
            &EntityIdentity,
            &EntityTransform,
            &EntityMetadata,
            &EntityAttributes,
            &EntityClientAnimations,
        )>();
        for (_, (identity, transform, metadata, attributes, animations)) in query.iter() {
            let Some(interpolation) = transform.interpolation else {
                continue;
            };
            let Some(bounds) = vanilla_bounds_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                continue;
            };
            let delta = EntityVec3 {
                x: transform.position.x - interpolation.previous_position.x,
                y: transform.position.y - interpolation.previous_position.y,
                z: transform.position.z - interpolation.previous_position.z,
            };
            inputs.push(EntityInterpolationCollisionInput {
                entity_id: identity.id,
                entity_type_id: identity.entity_type_id,
                current_position: transform.position,
                target_position: EntityVec3 {
                    x: interpolation.target_position.x + delta.x,
                    y: interpolation.target_position.y + delta.y,
                    z: interpolation.target_position.z + delta.z,
                },
                bounds,
            });
        }
        inputs
    }

    /// Gathers the world AABBs the per-entity `in_water` map is built from, for the
    /// entity types whose client-tick animation reads `isInWater()`
    /// ([`entity_animation_uses_in_water`]). Each input carries the entity id,
    /// AABB, and narrow Guardian-only support/target facts. The AABB is resolved from the SAME bounds source the projection
    /// uses ([`Self::pick_bounds`] → `vanilla_pick_bounds_for_entity_data`), so the
    /// fluid overlap matches the projected `in_water`. `WorldStore` (which holds the
    /// chunk/fluid data) turns each AABB into a `bool`.
    pub(crate) fn in_water_aabb_inputs(&self) -> Vec<EntityInWaterTickInput> {
        let mut inputs = Vec::new();
        let mut query = self.ecs.query::<(
            &EntityIdentity,
            &EntityTransform,
            &EntityMetadata,
            &EntityAttributes,
            &EntityClientAnimations,
        )>();
        for (_, (identity, transform, metadata, attributes, animations)) in query.iter() {
            if !entity_animation_uses_in_water(identity.entity_type_id)
                && !super::client_movement::client_movement_uses_in_water(identity.entity_type_id)
                && identity.entity_type_id != VANILLA_ENTITY_TYPE_STRIDER_ID
            {
                continue;
            }
            let Some(bounds) = vanilla_pick_bounds_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
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
            let eye_height = if is_vanilla_abstract_nautilus_type(identity.entity_type_id) {
                vanilla_eye_height_for_entity_data(
                    identity.entity_type_id,
                    identity.data,
                    &metadata.data_values,
                    &attributes.attributes,
                    Some(animations.animations),
                    metadata.painting_variant_projection.as_ref(),
                )
                .unwrap_or(bounds.max[1])
            } else {
                0.0
            };
            let guardian_below_pos =
                (vanilla_entity_metadata_type_is_guardian(identity.entity_type_id)
                    && transform.delta_movement.y < 0.0)
                    .then_some(crate::BlockPos {
                        x: transform.position.x.floor() as i32,
                        y: transform.position.y.floor() as i32 - 1,
                        z: transform.position.z.floor() as i32,
                    });
            let guardian_attack_target_id =
                if vanilla_entity_metadata_type_is_guardian(identity.entity_type_id) {
                    guardian_attack_target_id(identity.entity_type_id, &metadata.data_values)
                } else {
                    0
                };
            inputs.push(EntityInWaterTickInput {
                entity_id: identity.id,
                entity_type_id: identity.entity_type_id,
                aabb_min,
                aabb_max,
                position: transform.position,
                eye_position: [
                    transform.position.x,
                    transform.position.y + f64::from(eye_height),
                    transform.position.z,
                ],
                guardian_below_pos,
                guardian_attack_target_id,
            });
        }
        inputs
    }

    /// Gathers the fixed hull facts required by `AbstractBoat.tick` before the
    /// mutable ordered entity pass. Boat dimensions come from the generated
    /// Vanilla entity catalog and do not depend on entity metadata.
    pub(crate) fn prepare_client_tick_schedule<F>(
        &mut self,
        runs_normally: bool,
        is_ticking_entity: &F,
    ) -> Vec<EntityBoatClientTickInput>
    where
        F: Fn(i32, i32, EntityVec3) -> bool,
    {
        let mut schedule = std::mem::take(&mut self.client_tick_scratch);
        let mut inputs = Vec::new();
        self.fill_client_tick_schedule(
            &mut schedule,
            runs_normally,
            is_ticking_entity,
            Some(&mut inputs),
        );
        self.client_tick_scratch = schedule;
        inputs
    }

    /// Resolves only unique active Guardian targets collected by the existing
    /// water-input query. Guardians sharing a target reuse one protocol-id and
    /// component lookup; unrelated living entities are never projected.
    fn guardian_attack_target_facts(
        &self,
        target_ids: &HashSet<i32>,
    ) -> HashMap<i32, GuardianAttackTargetTickInput> {
        let mut targets = HashMap::with_capacity(target_ids.len());
        for &target_id in target_ids {
            let Some(&entity) = self.by_protocol_id.get(&target_id) else {
                continue;
            };
            let (Ok(identity), Ok(transform), Ok(metadata), Ok(attributes), Ok(animations)) = (
                self.ecs.get::<&EntityIdentity>(entity),
                self.ecs.get::<&EntityTransform>(entity),
                self.ecs.get::<&EntityMetadata>(entity),
                self.ecs.get::<&EntityAttributes>(entity),
                self.ecs.get::<&EntityClientAnimations>(entity),
            ) else {
                continue;
            };
            if !vanilla_living_entity_type(identity.entity_type_id) {
                continue;
            }
            let Some(bounds) = vanilla_bounds_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                continue;
            };
            let Some(eye_height) = vanilla_eye_height_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                continue;
            };
            targets.insert(
                target_id,
                GuardianAttackTargetTickInput {
                    position: transform.position,
                    eye_height,
                    half_height: (bounds.max[1] - bounds.min[1]) * 0.5,
                },
            );
        }
        targets
    }

    pub(crate) fn client_movement_fluid_inputs(
        &self,
        can_carry_interpolation_target: &HashSet<i32>,
    ) -> Vec<super::client_movement::EntityClientMovementFluidInput> {
        let mut inputs = Vec::new();
        let mut query = self.ecs.query::<(
            &EntityClientMovementFluidFacts,
            &EntityIdentity,
            &EntityTransform,
            &EntityMetadata,
            &EntityAttributes,
            &EntityClientAnimations,
        )>();
        for (_, (_, identity, transform, metadata, attributes, animations)) in query.iter() {
            debug_assert!(super::client_movement::client_movement_uses_fluid_contact(
                identity.entity_type_id
            ));
            let Some(bounds) = vanilla_bounds_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                continue;
            };
            let Some(eye_height) = vanilla_eye_height_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                continue;
            };
            let mut world_fact_transform = *transform;
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID {
                world_fact_transform.advance_interpolation_tick(
                    true,
                    true,
                    can_carry_interpolation_target.contains(&identity.id),
                );
            }
            let position = world_fact_transform.position;
            inputs.push(super::client_movement::EntityClientMovementFluidInput {
                entity_id: identity.id,
                entity_type_id: identity.entity_type_id,
                position,
                bounds,
                aabb_min: [
                    position.x + f64::from(bounds.min[0]),
                    position.y + f64::from(bounds.min[1]),
                    position.z + f64::from(bounds.min[2]),
                ],
                aabb_max: [
                    position.x + f64::from(bounds.max[0]),
                    position.y + f64::from(bounds.max[1]),
                    position.z + f64::from(bounds.max[2]),
                ],
                eye_position: [position.x, position.y + f64::from(eye_height), position.z],
            });
        }
        inputs
    }

    pub(crate) fn client_movement_has_experience_orb(&self) -> bool {
        self.ecs
            .query::<&EntityExperienceOrbMovement>()
            .iter()
            .next()
            .is_some()
    }

    pub(crate) fn client_movement_entity_collision_targets(
        &self,
    ) -> Vec<super::EntityPickTargetState> {
        self.entity_collision_targets(false)
    }

    pub(crate) fn vehicle_entity_collision_targets(&self) -> Vec<super::EntityPickTargetState> {
        self.entity_collision_targets(true)
    }

    fn entity_collision_targets(
        &self,
        include_pushable: bool,
    ) -> Vec<super::EntityPickTargetState> {
        let mut targets = Vec::new();
        let mut query = self.ecs.query::<(
            &EntityIdentity,
            &EntityTransform,
            &EntityMetadata,
            &EntityAttributes,
            &EntityClientAnimations,
        )>();
        for (_, (identity, transform, metadata, attributes, animations)) in query.iter() {
            let never_collidable = matches!(
                vanilla_entity_metadata_type_collidability_kind(identity.entity_type_id),
                None | Some(VanillaEntityCollidabilityKind::Never)
            );
            let never_pushable = matches!(
                vanilla_entity_metadata_type_pushability_kind(identity.entity_type_id),
                None | Some(VanillaEntityPushabilityKind::Never)
            );
            if never_collidable && (!include_pushable || never_pushable) {
                continue;
            }
            let Some(bounds) = vanilla_bounds_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                continue;
            };
            targets.push(super::EntityPickTargetState {
                entity_id: identity.id,
                position: transform.position,
                bounds,
            });
        }
        targets
    }

    fn apply_block_effect_tick_for_entity(
        &mut self,
        entity: Entity,
        policy: VanillaEntityBlockEffectTickPolicy,
        client_movement_collisions_by_id: &HashMap<
            i32,
            super::client_movement::EntityClientMovementCollisionResult,
        >,
        block_effect_world: &mut crate::client::EntityBlockEffectWorld<'_>,
        local_movement: Option<EntityBlockEffectMovement>,
        base_tick_reset: EntityBlockEffectBaseTickReset,
    ) -> (usize, usize, usize) {
        // `LocalAuthoritativeLiving` never runs from the ordered remote pass;
        // it is reachable only through the local-vehicle entry point, which
        // supplies the exact travel movement recorded by `Entity.move`.
        // `AbstractArrowSegments` likewise arrives only from the arrow
        // movement slot with its explicit per-tick flight segment.
        debug_assert!(
            policy != VanillaEntityBlockEffectTickPolicy::None
                && (!matches!(
                    policy,
                    VanillaEntityBlockEffectTickPolicy::LocalAuthoritativeLiving
                        | VanillaEntityBlockEffectTickPolicy::AbstractArrowSegments
                ) || local_movement.is_some())
        );
        debug_assert!(block_effect_world.has_loaded_chunks());

        let Ok((
            identity,
            transform,
            metadata,
            attributes,
            mob_effects,
            supporting_block,
            mut collision_facts,
            animations,
        )) = self.ecs.query_one_mut::<(
            &EntityIdentity,
            &mut EntityTransform,
            &mut EntityMetadata,
            &EntityAttributes,
            &EntityMobEffects,
            &mut EntitySupportingBlock,
            Option<&mut EntityClientMovementCollisionFacts>,
            &EntityClientAnimations,
        )>(entity)
        else {
            return (0, 0, 0);
        };
        if policy == VanillaEntityBlockEffectTickPolicy::DetachedFirework {
            let detached = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
                identity.entity_type_id,
            )
            .is_some_and(|accessors| {
                metadata
                    .data_values
                    .iter()
                    .find(|value| value.data_id == accessors.attached_to_target_data_id)
                    .and_then(|value| match value.value {
                        EntityDataValueKind::OptionalUnsignedInt(value) => Some(value),
                        _ => None,
                    })
                    .unwrap_or(accessors.attached_to_target_default)
                    .is_none()
            });
            if !detached {
                return (0, 0, 0);
            }
        }

        let supporting_block_position = supporting_block.main;
        let block_effects = &mut supporting_block.block_effects;

        let apply_count = usize::from(policy == VanillaEntityBlockEffectTickPolicy::BoatTwice) + 1;
        // Client-side `Entity.baseTick` clears a private positive fire timer
        // and resets the powder-snow marker. Most policies run it before any
        // subclass callback (a boat's two callback passes still share the
        // resulting same-tick state); a flying arrow runs `super.tick()` after
        // its effects, and an in-ground arrow never reaches it.
        if base_tick_reset == EntityBlockEffectBaseTickReset::BeforeCallbacks {
            block_effects.remaining_fire_ticks = block_effects.remaining_fire_ticks.min(0);
            block_effects.in_powder_snow = false;
        }
        let mut entities = 0usize;
        let mut blocks = 0usize;
        let mut callbacks = 0usize;
        for pass in 0..apply_count {
            let Some(bounds) = vanilla_bounds_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                break;
            };
            let entity_accessors = vanilla_entity_metadata_entity_accessors();
            let ticks_frozen = entity_accessors.map_or(0, |accessors| {
                entity_data_int(
                    &metadata.data_values,
                    accessors.ticks_frozen_data_id,
                    accessors.ticks_frozen_default,
                )
            });
            let state = (
                block_effects.stuck_speed_multiplier,
                block_effects.remaining_fire_ticks,
                block_effects.fall_distance,
            );
            let use_last_movements = policy
                == VanillaEntityBlockEffectTickPolicy::ItemWithLastMovements
                && client_movement_collisions_by_id
                    .get(&identity.id)
                    .is_some_and(|collision| collision.movement_skipped);
            let movements = if let Some(segment) = local_movement {
                // Vanilla's first call consumes the recorded travel move with
                // its axis-order original; a boat's second call sees an empty
                // recorder and falls back to the plain old-to-current segment.
                if pass == 0 {
                    EntityBlockEffectMovementIter::OneOwned(Some(segment))
                } else {
                    EntityBlockEffectMovementIter::direct(segment.from, transform.position)
                }
            } else if let Some(collision_facts) = collision_facts.as_deref_mut() {
                let movements = &mut collision_facts.block_effect_movements;
                if use_last_movements {
                    movements.last_movements()
                } else {
                    movements.prepare_current(transform.old_position, transform.position)
                }
            } else {
                if use_last_movements {
                    EntityBlockEffectMovementIter::Empty
                } else {
                    EntityBlockEffectMovementIter::direct(
                        transform.old_position,
                        transform.position,
                    )
                }
            };
            let evaluation = block_effect_world.evaluate(EntityBlockEffectQuery {
                position: transform.position,
                bounds,
                movements,
                supporting_block: supporting_block_position,
                on_ground: transform.on_ground.unwrap_or(false),
                stepping_carefully: entity_data_pose(&metadata.data_values)
                    == VANILLA_ENTITY_POSE_CROUCHING_ID,
                is_living: vanilla_entity_metadata_type_is_living(identity.entity_type_id),
                is_fox_or_bee: matches!(
                    identity.entity_type_id,
                    VANILLA_ENTITY_TYPE_FOX_ID | VANILLA_ENTITY_TYPE_BEE_ID
                ),
                has_weaving: mob_effects
                    .effects
                    .contains_key(&bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_WEAVING_ID),
                fire_immune: vanilla_entity_type_is_fire_immune(identity.entity_type_id),
                freeze_immune: vanilla_entity_type_is_freeze_immune(identity.entity_type_id),
                ticks_frozen,
                delta_movement: transform.delta_movement,
                stuck_speed_multiplier: state.0,
                remaining_fire_ticks: state.1,
                fall_distance: state.2,
            });
            transform.delta_movement = evaluation.delta_movement;
            block_effects.stuck_speed_multiplier = evaluation.stuck_speed_multiplier;
            block_effects.remaining_fire_ticks = evaluation.remaining_fire_ticks;
            block_effects.fall_distance = evaluation.fall_distance;
            block_effects.in_powder_snow |= evaluation.in_powder_snow;
            for &position in &evaluation.snowflakes {
                self.pending_entity_client_movement_effects.push(
                    EntityClientMovementEffectState::Particle(
                        EntityClientMovementParticleState::PowderSnowFlake(
                            PowderSnowFlakeParticleState {
                                entity_id: identity.id,
                                position,
                            },
                        ),
                    ),
                );
            }
            if ticks_frozen != evaluation.ticks_frozen {
                if let Some(accessors) = entity_accessors {
                    upsert_metadata_value(
                        metadata,
                        ProtocolEntityDataValue {
                            data_id: accessors.ticks_frozen_data_id,
                            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                            value: EntityDataValueKind::Int(evaluation.ticks_frozen),
                        },
                    );
                }
            }
            entities += 1;
            blocks = blocks.saturating_add(evaluation.visited_blocks);
            callbacks = callbacks.saturating_add(evaluation.applied_callbacks);
        }
        if base_tick_reset == EntityBlockEffectBaseTickReset::AfterCallbacks {
            block_effects.remaining_fire_ticks = block_effects.remaining_fire_ticks.min(0);
            block_effects.in_powder_snow = false;
        }
        if policy != VanillaEntityBlockEffectTickPolicy::ItemWithLastMovements {
            if let Some(collision_facts) = collision_facts.as_deref_mut() {
                collision_facts.block_effect_movements.clear_final();
            }
        }
        (entities, blocks, callbacks)
    }

    /// Records the directly controlled vehicle's travel move for the
    /// same-call `applyEffectsFromBlocks`; a later move overwrites, matching
    /// the single `Entity.move` each ridden travel performs per tick.
    pub(crate) fn record_local_vehicle_move(
        &mut self,
        id: i32,
        movement: EntityBlockEffectMovement,
    ) {
        self.client_tick_scratch.local_vehicle_move = Some((id, movement));
    }

    pub(crate) fn take_local_vehicle_move(&mut self, id: i32) -> Option<EntityBlockEffectMovement> {
        match self.client_tick_scratch.local_vehicle_move.take() {
            Some((recorded_id, movement)) if recorded_id == id => Some(movement),
            _ => None,
        }
    }

    /// `Entity.move` consumes the pending stuck-speed multiplier before
    /// collision clipping and zeroes the delta movement afterwards; the local
    /// vehicle travel owners call this at their exact move points.
    pub(crate) fn consume_block_effect_stuck_speed_multiplier(
        &mut self,
        id: i32,
    ) -> Option<EntityVec3> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut supporting_block = self.ecs.get::<&mut EntitySupportingBlock>(entity).ok()?;
        supporting_block
            .block_effects
            .consume_stuck_speed_multiplier()
    }

    /// Locally authoritative `applyEffectsFromBlocks` for the directly
    /// controlled vehicle: Vanilla runs it in `LivingEntity.aiStep` after
    /// `travelRidden` (one pass) and in `AbstractBoat.tick` after `move`
    /// (two passes). The caller supplies the exact travel move; remote
    /// instances keep the ordered scheduled pass.
    pub(crate) fn apply_local_vehicle_block_effect_tick(
        &mut self,
        vehicle_id: i32,
        movement: EntityBlockEffectMovement,
        client_movement_collisions_by_id: &HashMap<
            i32,
            super::client_movement::EntityClientMovementCollisionResult,
        >,
        block_effect_world: &mut crate::client::EntityBlockEffectWorld<'_>,
    ) -> (usize, usize, usize) {
        if !block_effect_world.has_loaded_chunks() {
            return (0, 0, 0);
        }
        let Some(&entity) = self.by_protocol_id.get(&vehicle_id) else {
            return (0, 0, 0);
        };
        let Ok(entity_type_id) = self
            .ecs
            .get::<&EntityIdentity>(entity)
            .map(|identity| identity.entity_type_id)
        else {
            return (0, 0, 0);
        };
        let policy = super::vanilla_entity_metadata_type_block_effect_tick_policy(entity_type_id)
            .unwrap_or(VanillaEntityBlockEffectTickPolicy::None);
        if !matches!(
            policy,
            VanillaEntityBlockEffectTickPolicy::LocalAuthoritativeLiving
                | VanillaEntityBlockEffectTickPolicy::BoatTwice
        ) {
            return (0, 0, 0);
        }
        self.apply_block_effect_tick_for_entity(
            entity,
            policy,
            client_movement_collisions_by_id,
            block_effect_world,
            Some(movement),
            EntityBlockEffectBaseTickReset::BeforeCallbacks,
        )
    }

    /// Locally authoritative `applyEffectsFromBlocks` for the local player's
    /// `aiStep` slot, driven per physics step from the pose integrator with
    /// the step's recorded clipped move. The pose owns position, delta, and
    /// fall distance, so those flow through the query as overrides while the
    /// private block-effect state and `ticksFrozen` live on the player
    /// entity; the returned delta and fall distance are the pose write-backs.
    pub(crate) fn apply_local_player_block_effect_step(
        &mut self,
        step: LocalPlayerBlockEffectStep,
        block_effect_world: &mut crate::client::EntityBlockEffectWorld<'_>,
    ) -> Option<LocalPlayerBlockEffectOutcome> {
        if !block_effect_world.has_loaded_chunks() {
            return None;
        }
        let &entity = self.by_protocol_id.get(&step.player_id)?;
        let (identity, metadata, mob_effects, supporting_block) = self
            .ecs
            .query_one_mut::<(
                &EntityIdentity,
                &mut EntityMetadata,
                &EntityMobEffects,
                &mut EntitySupportingBlock,
            )>(entity)
            .ok()?;
        let supporting_block_position = supporting_block.main;
        let block_effects = &mut supporting_block.block_effects;
        // The local player's client `Entity.baseTick` runs at tick start,
        // before the `aiStep` travel that precedes this callback.
        block_effects.remaining_fire_ticks = block_effects.remaining_fire_ticks.min(0);
        block_effects.in_powder_snow = false;
        let entity_accessors = vanilla_entity_metadata_entity_accessors();
        let ticks_frozen = entity_accessors.map_or(0, |accessors| {
            entity_data_int(
                &metadata.data_values,
                accessors.ticks_frozen_data_id,
                accessors.ticks_frozen_default,
            )
        });
        let evaluation = block_effect_world.evaluate(EntityBlockEffectQuery {
            position: step.position,
            bounds: step.bounds,
            movements: EntityBlockEffectMovementIter::OneOwned(Some(step.movement)),
            supporting_block: supporting_block_position,
            on_ground: step.on_ground,
            stepping_carefully: step.stepping_carefully,
            is_living: true,
            is_fox_or_bee: false,
            has_weaving: mob_effects
                .effects
                .contains_key(&bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_WEAVING_ID),
            fire_immune: vanilla_entity_type_is_fire_immune(identity.entity_type_id),
            freeze_immune: step.extra_freeze_immune
                || vanilla_entity_type_is_freeze_immune(identity.entity_type_id),
            ticks_frozen,
            delta_movement: step.delta_movement,
            stuck_speed_multiplier: block_effects.stuck_speed_multiplier,
            remaining_fire_ticks: block_effects.remaining_fire_ticks,
            fall_distance: step.fall_distance,
        });
        block_effects.stuck_speed_multiplier = evaluation.stuck_speed_multiplier;
        block_effects.remaining_fire_ticks = evaluation.remaining_fire_ticks;
        block_effects.fall_distance = evaluation.fall_distance;
        block_effects.in_powder_snow |= evaluation.in_powder_snow;
        for &position in &evaluation.snowflakes {
            self.pending_entity_client_movement_effects.push(
                EntityClientMovementEffectState::Particle(
                    EntityClientMovementParticleState::PowderSnowFlake(
                        PowderSnowFlakeParticleState {
                            entity_id: step.player_id,
                            position,
                        },
                    ),
                ),
            );
        }
        if ticks_frozen != evaluation.ticks_frozen {
            if let Some(accessors) = entity_accessors {
                upsert_metadata_value(
                    metadata,
                    ProtocolEntityDataValue {
                        data_id: accessors.ticks_frozen_data_id,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                        value: EntityDataValueKind::Int(evaluation.ticks_frozen),
                    },
                );
            }
        }
        Some(LocalPlayerBlockEffectOutcome {
            delta_movement: evaluation.delta_movement,
            fall_distance: evaluation.fall_distance,
        })
    }

    /// Explicit `AbstractArrow` callback from the arrow movement slot:
    /// Vanilla applies `applyEffectsFromBlocks(initialPosition, nextLocation)`
    /// between `setPos` and the post-move inertia (flight) or the no-arg
    /// degenerate segment while stuck in ground. The caller supplies the
    /// `Entity.baseTick` reset placement for its branch.
    pub(super) fn apply_arrow_block_effect_tick(
        &mut self,
        entity: Entity,
        segment: EntityBlockEffectMovement,
        base_tick_reset: EntityBlockEffectBaseTickReset,
        client_movement_collisions_by_id: &HashMap<
            i32,
            super::client_movement::EntityClientMovementCollisionResult,
        >,
        block_effect_world: &mut crate::client::EntityBlockEffectWorld<'_>,
    ) -> (usize, usize, usize) {
        if !block_effect_world.has_loaded_chunks() {
            return (0, 0, 0);
        }
        self.apply_block_effect_tick_for_entity(
            entity,
            VanillaEntityBlockEffectTickPolicy::AbstractArrowSegments,
            client_movement_collisions_by_id,
            block_effect_world,
            Some(segment),
            base_tick_reset,
        )
    }

    /// `AbstractArrow.tick` head: `if (this.isInWaterOrRain()) this.clearFire();`
    /// runs before the in-ground/flight branches and is not gated by
    /// `isAffectedByBlocks`, so a no-physics returning trident still clears.
    pub(super) fn apply_arrow_water_or_rain_fire_clear(
        &mut self,
        entity: Entity,
        in_water: bool,
        block_effect_world: &crate::client::EntityBlockEffectWorld<'_>,
    ) {
        let Ok((identity, transform, metadata, attributes, animations, supporting_block)) =
            self.ecs.query_one_mut::<(
                &EntityIdentity,
                &EntityTransform,
                &EntityMetadata,
                &EntityAttributes,
                &EntityClientAnimations,
                &mut EntitySupportingBlock,
            )>(entity)
        else {
            return;
        };
        if supporting_block.block_effects.remaining_fire_ticks == 0 {
            return;
        }
        let clears = in_water
            || (block_effect_world.has_loaded_chunks()
                && vanilla_bounds_for_entity_data(
                    identity.entity_type_id,
                    identity.data,
                    &metadata.data_values,
                    &attributes.attributes,
                    Some(animations.animations),
                    metadata.painting_variant_projection.as_ref(),
                )
                .is_some_and(|bounds| {
                    block_effect_world.is_in_rain(transform.position, f64::from(bounds.max[1]))
                }));
        if clears {
            supporting_block.block_effects.remaining_fire_ticks = 0;
        }
    }

    pub(crate) fn advance_client_animations<G>(
        &mut self,
        ticks: u32,
        game_time: i64,
        deferred_boat_paddle_entity_id: Option<i32>,
        deferred_local_camel_vehicle_id: Option<i32>,
        deferred_local_nautilus_vehicle_id: Option<i32>,
        deferred_local_item_based_steering_vehicle_id: Option<i32>,
        default_item_equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
        local_player_id: Option<i32>,
        local_player_rotation: Option<(f32, f32)>,
        local_player_holds_carrot_on_a_stick: bool,
        local_player_holds_warped_fungus_on_a_stick: bool,
        spectator_player_uuids: &HashSet<Uuid>,
        fluid_facts_by_id: &HashMap<i32, EntityClientTickFluidFacts>,
        boat_world_facts: &[EntityBoatClientTickWorldFacts],
        mut local_player_pose: Option<&mut LocalPlayerPoseState>,
        local_player_is_vehicle: bool,
        guardian_can_stand_on_below_by_id: &HashMap<i32, bool>,
        guardian_attack_target_ids: &HashSet<i32>,
        client_movement_collisions_by_id: &HashMap<
            i32,
            super::client_movement::EntityClientMovementCollisionResult,
        >,
        minecart_improvements_enabled: bool,
        death_tick_player_chunk: Option<(i32, i32)>,
        death_tick_simulation_distance: Option<i32>,
        can_carry_interpolation_target: &G,
        block_effect_world: &mut crate::client::EntityBlockEffectWorld<'_>,
    ) -> EntityClientTickWork
    where
        G: Fn(i32) -> bool,
    {
        let mut work = EntityClientTickWork::default();
        let preparation_started_at = Instant::now();
        let wither_head_targets_by_id = self.wither_head_targets_by_id();
        work.preparation_duration += preparation_started_at.elapsed();
        // WorldStore resolves terrain and local-player facts from this exact
        // schedule before entering the mutable pass. Its public batched API
        // normalizes catch-up to one call per raw tick.
        debug_assert_eq!(ticks, 1);
        for _ in 0..ticks {
            let mut tick_scratch = std::mem::take(&mut self.client_tick_scratch);
            let schedule_started_at = Instant::now();
            self.prepare_scheduled_passengers_for_ride_tick(&tick_scratch.scheduled_entities);
            work.schedule_duration += schedule_started_at.elapsed();
            let scheduled = &tick_scratch.scheduled_set;
            let projectile_started_at = Instant::now();
            self.advance_projectile_lifecycles(1, spectator_player_uuids, scheduled);
            work.projectile_duration += projectile_started_at.elapsed();
            let interpolation_started_at = Instant::now();
            for (_, (identity, transform)) in self
                .ecs
                .query_mut::<(&EntityIdentity, &mut EntityTransform)>()
            {
                let is_ticking = scheduled.contains(&identity.id);
                if !is_ticking {
                    transform.advance_interpolation_tick(true, false, false);
                    continue;
                }
                transform.begin_client_tick();
                let has_interpolation_handler = vanilla_entity_metadata_type_interpolation_steps(
                    identity.entity_type_id,
                    minecart_improvements_enabled,
                )
                .is_some();
                transform.advance_interpolation_tick(
                    has_interpolation_handler,
                    true,
                    can_carry_interpolation_target(identity.id),
                );
            }
            work.interpolation_duration += interpolation_started_at.elapsed();
            let movement_started_at = Instant::now();
            let arrow_block_effect_work = self.advance_direct_client_movement(
                &tick_scratch.scheduled_entities,
                fluid_facts_by_id,
                client_movement_collisions_by_id,
                block_effect_world,
            );
            work.projectile_duration +=
                movement_started_at.elapsed() - arrow_block_effect_work.duration;
            work.block_effect_duration += arrow_block_effect_work.duration;
            work.block_effect_entities = work
                .block_effect_entities
                .saturating_add(arrow_block_effect_work.entities);
            work.block_effect_blocks_visited = work
                .block_effect_blocks_visited
                .saturating_add(arrow_block_effect_work.blocks);
            work.block_effect_callbacks_applied = work
                .block_effect_callbacks_applied
                .saturating_add(arrow_block_effect_work.callbacks);
            let animation_started_at = Instant::now();
            let block_effect_duration_before = work.block_effect_duration;
            let guardian_attack_targets_by_id =
                self.guardian_attack_target_facts(guardian_attack_target_ids);
            work.scanned_entities = work
                .scanned_entities
                .saturating_add(self.by_protocol_id.len());
            let mut boat_world_facts_iter = boat_world_facts.iter();
            for &entity in &tick_scratch.scheduled_entities {
                let identity = self
                    .ecs
                    .get::<&EntityIdentity>(entity)
                    .ok()
                    .map(|identity| (identity.id, identity.entity_type_id));
                let strider_suffocating_update = identity.and_then(|(entity_id, _)| {
                    self.strider_suffocating_for_client_tick(entity_id, fluid_facts_by_id)
                        .map(|suffocating| (entity_id, suffocating))
                });
                if let Some((entity_id, suffocating)) = strider_suffocating_update {
                    let _ = self.set_strider_suffocating(entity_id, suffocating);
                }
                let item_based_steering_controller_rotation =
                    identity.and_then(|(entity_id, entity_type_id)| {
                        self.item_based_steering_controller_rotation(
                            entity_id,
                            entity_type_id,
                            default_item_equipment_slots,
                            local_player_id,
                            local_player_rotation,
                            local_player_holds_carrot_on_a_stick,
                            local_player_holds_warped_fungus_on_a_stick,
                        )
                    });
                let Some((entity_id, entity_type_id)) = identity else {
                    continue;
                };
                // `AbstractMinecart.tick` runs `NewMinecartBehavior.tick`
                // before any concrete minecart subclass work. TNT smoke and
                // every later animation therefore observe the post-lerp pose.
                if minecart_improvements_enabled && is_vanilla_minecart_type(entity_type_id) {
                    if let Ok((transform, minecart_lerp)) = self
                        .ecs
                        .query_one_mut::<(&mut EntityTransform, &mut EntityMinecartLerp)>(entity)
                    {
                        minecart_lerp.advance_client_tick(transform);
                    }
                }
                let is_boat = is_vanilla_boat_type(entity_type_id);
                let boat_facts = is_boat.then(|| boat_world_facts_iter.next()).flatten();
                debug_assert!(boat_facts.is_none_or(|facts| facts.entity_id == entity_id));
                if is_boat && deferred_boat_paddle_entity_id != Some(entity_id) {
                    if let Ok((transform, movement)) = self
                        .ecs
                        .query_one_mut::<(&mut EntityTransform, &mut EntityBoatMovement)>(entity)
                    {
                        if let Some(facts) = boat_facts {
                            movement.old_status = movement.status;
                            movement.status = Some(facts.status);
                            movement.water_level = facts.water_level;
                            if let Some(land_friction) = facts.land_friction {
                                movement.land_friction = land_friction;
                            }
                            if matches!(
                                facts.status,
                                super::EntityBoatStatus::UnderWater
                                    | super::EntityBoatStatus::UnderFlowingWater
                            ) {
                                movement.out_of_control_ticks += 1.0;
                            } else {
                                movement.out_of_control_ticks = 0.0;
                            }
                        }
                        // `AbstractBoat.tick` applies interpolation first,
                        // then the remote branch discards packet velocity.
                        transform.delta_movement = EntityVec3::default();
                    }
                }
                let block_effect_policy =
                    super::vanilla_entity_metadata_type_block_effect_tick_policy(entity_type_id)
                        .unwrap_or(VanillaEntityBlockEffectTickPolicy::None);
                let applies_block_effects = !matches!(
                    block_effect_policy,
                    VanillaEntityBlockEffectTickPolicy::None
                        | VanillaEntityBlockEffectTickPolicy::LocalAuthoritativeLiving
                        | VanillaEntityBlockEffectTickPolicy::AbstractArrowSegments
                ) && !(block_effect_policy
                    == VanillaEntityBlockEffectTickPolicy::BoatTwice
                    && deferred_boat_paddle_entity_id == Some(entity_id));
                if block_effect_world.has_loaded_chunks() && applies_block_effects {
                    let block_effect_started_at = Instant::now();
                    let (effect_entities, effect_blocks, effect_callbacks) = self
                        .apply_block_effect_tick_for_entity(
                            entity,
                            block_effect_policy,
                            client_movement_collisions_by_id,
                            block_effect_world,
                            None,
                            EntityBlockEffectBaseTickReset::BeforeCallbacks,
                        );
                    work.block_effect_duration += block_effect_started_at.elapsed();
                    work.block_effect_entities =
                        work.block_effect_entities.saturating_add(effect_entities);
                    work.block_effect_blocks_visited = work
                        .block_effect_blocks_visited
                        .saturating_add(effect_blocks);
                    work.block_effect_callbacks_applied = work
                        .block_effect_callbacks_applied
                        .saturating_add(effect_callbacks);
                }
                let Ok((
                    identity,
                    transform,
                    mount,
                    metadata,
                    attributes,
                    equipment,
                    damage,
                    animations,
                )) = self.ecs.query_one_mut::<(
                    &EntityIdentity,
                    &mut EntityTransform,
                    &EntityMount,
                    &EntityMetadata,
                    &EntityAttributes,
                    Option<&EntityEquipment>,
                    &mut EntityDamage,
                    &mut EntityClientAnimations,
                )>(entity)
                else {
                    continue;
                };
                animations
                    .animations
                    .advance_head_rotation_lerp(&mut transform.y_head_rot);
                // Vanilla `LivingEntity.calculateEntityAnimation` gates the limb
                // swing on `!isPassenger()` and scales the position by `3` for a
                // baby (`updateWalkAnimation`).
                let is_passenger = mount.vehicle_id.is_some();
                let should_tick_death = within_death_tick_distance(
                    transform.position,
                    death_tick_player_chunk,
                    death_tick_simulation_distance,
                );
                let entity_type_id = identity.entity_type_id;
                // Vanilla `Blaze.aiStep` damps only downward airborne velocity
                // before its client ambient effects and `super.aiStep`.
                if entity_type_id == VANILLA_ENTITY_TYPE_BLAZE_ID
                    && !transform.on_ground.unwrap_or(false)
                    && transform.delta_movement.y < 0.0
                {
                    transform.delta_movement.y *= 0.6;
                }
                let is_living = vanilla_entity_metadata_type_is_living(entity_type_id);
                if is_living && damage.invulnerable_time > 0 {
                    // Vanilla `LivingEntity.baseTick` decrements this for every
                    // client living instance (`ServerPlayer` is server-only).
                    damage.invulnerable_time -= 1;
                }
                let is_baby = is_living && vanilla_is_baby(entity_type_id, &metadata.data_values);
                let pose = is_living.then(|| entity_data_pose(&metadata.data_values));
                let is_fall_flying = is_living && entity_is_fall_flying(&metadata.data_values);
                let is_crouching = pose == Some(VANILLA_ENTITY_POSE_CROUCHING_ID);
                let bat_is_resting = entity_type_id == VANILLA_ENTITY_TYPE_BAT_ID
                    && bat_is_resting(entity_type_id, &metadata.data_values);
                let is_swimming = entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
                    && entity_is_swimming(&metadata.data_values);
                // The per-tick world fact (`isInWater()`) the world resolved before
                // this mutable pass, defaulting to `false` for non-consumers, and the
                // synced `Guardian.DATA_ID_MOVING` flag read from the metadata here.
                let in_water = entity_animation_uses_in_water(entity_type_id)
                    && fluid_facts_by_id
                        .get(&identity.id)
                        .is_some_and(|facts| facts.in_water);
                let is_guardian = vanilla_entity_metadata_type_is_guardian(entity_type_id);
                let is_moving =
                    is_guardian && guardian_is_moving(entity_type_id, &metadata.data_values);
                let guardian_can_stand_on_below = is_guardian
                    && guardian_can_stand_on_below_by_id
                        .get(&identity.id)
                        .copied()
                        .unwrap_or(false);
                let warden_heartbeat_delay = if entity_type_id == VANILLA_ENTITY_TYPE_WARDEN_ID {
                    warden_heartbeat_delay(entity_type_id, &metadata.data_values)
                } else {
                    0
                };
                let guardian_attack_target_id = if is_guardian {
                    guardian_attack_target_id(entity_type_id, &metadata.data_values)
                } else {
                    0
                };
                let guardian_attack_target = guardian_attack_targets_by_id
                    .get(&guardian_attack_target_id)
                    .copied();
                let guardian_eye_height = guardian_attack_target.and_then(|_| {
                    vanilla_eye_height_for_entity_data(
                        identity.entity_type_id,
                        identity.data,
                        &metadata.data_values,
                        &attributes.attributes,
                        Some(animations.animations),
                        metadata.painting_variant_projection.as_ref(),
                    )
                });
                let wither_head_targets = (entity_type_id == VANILLA_ENTITY_TYPE_WITHER_ID)
                    .then(|| wither_head_targets_by_id.get(&identity.id).copied())
                    .flatten();
                let is_camel = vanilla_entity_metadata_type_is_camel(entity_type_id);
                let camel_is_standing = is_camel && pose == Some(VANILLA_ENTITY_POSE_STANDING_ID);
                let mut camel_is_dashing =
                    is_camel && camel_is_dashing(entity_type_id, &metadata.data_values);
                if is_camel && deferred_local_camel_vehicle_id != Some(identity.id) {
                    let clear_dash = animations.animations.camel_dash_should_clear_before_tick(
                        camel_is_dashing,
                        transform.on_ground.unwrap_or(false),
                        fluid_facts_by_id
                            .get(&identity.id)
                            .is_some_and(|facts| facts.in_liquid),
                        is_passenger,
                    );
                    if clear_dash {
                        camel_is_dashing = false;
                        tick_scratch.camel_clear_dash_entities.push(identity.id);
                    }
                    let camel_is_sitting =
                        vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
                            entity_type_id,
                        )
                        .is_some_and(|(data_id, default)| {
                            entity_data_long(&metadata.data_values, data_id, default) < 0
                        });
                    if camel_is_sitting && in_water {
                        tick_scratch
                            .camel_stand_instantly_entities
                            .push(identity.id);
                    }
                }
                let is_nautilus = is_vanilla_abstract_nautilus_type(entity_type_id);
                let nautilus_is_dashing = is_nautilus
                    && entity_data_bool(
                        &metadata.data_values,
                        VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
                        VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH,
                    );
                let copper_golem_animation =
                    if entity_type_id == VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID {
                        copper_golem_animation_flags(entity_type_id, &metadata.data_values)
                    } else {
                        Default::default()
                    };
                let allay_is_dancing = entity_type_id == VANILLA_ENTITY_TYPE_ALLAY_ID
                    && allay_is_dancing(entity_type_id, &metadata.data_values);
                let allay_has_item_in_hand = entity_type_id == VANILLA_ENTITY_TYPE_ALLAY_ID
                    && equipment.is_some_and(|equipment| {
                        equipment.equipment.iter().any(|update| {
                            update.slot == ProtocolEquipmentSlot::MainHand
                                && update.item.item_id.is_some()
                        })
                    });
                let axolotl_is_playing_dead = entity_type_id == VANILLA_ENTITY_TYPE_AXOLOTL_ID
                    && axolotl_is_playing_dead(entity_type_id, &metadata.data_values);
                let creaking_is_tearing_down = entity_type_id == VANILLA_ENTITY_TYPE_CREAKING_ID
                    && creaking_is_tearing_down(entity_type_id, &metadata.data_values);
                let pillager_is_charging_crossbow = entity_type_id
                    == VANILLA_ENTITY_TYPE_PILLAGER_ID
                    && pillager_is_charging_crossbow(entity_type_id, &metadata.data_values);
                let piglin_is_charging_crossbow = entity_type_id == VANILLA_ENTITY_TYPE_PIGLIN_ID
                    && piglin_is_charging_crossbow(entity_type_id, &metadata.data_values);
                let player_is_using_item = entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
                    && player_is_using_item(entity_type_id, &metadata.data_values);
                let wolf_is_interested = entity_type_id == VANILLA_ENTITY_TYPE_WOLF_ID
                    && wolf_is_interested(entity_type_id, &metadata.data_values);
                let cat_is_lying = entity_type_id == VANILLA_ENTITY_TYPE_CAT_ID
                    && cat_is_lying(entity_type_id, &metadata.data_values);
                let cat_relax_state_one = entity_type_id == VANILLA_ENTITY_TYPE_CAT_ID
                    && cat_relax_state_one(entity_type_id, &metadata.data_values);
                let boat_paddles = if is_boat {
                    boat_paddle_states(
                        entity_type_id,
                        &metadata.data_values,
                        !mount.passengers.is_empty(),
                    )
                } else {
                    [false; 2]
                };
                let boat_bubble_time_value = if is_boat {
                    boat_bubble_time(entity_type_id, &metadata.data_values)
                } else {
                    0
                };
                let furnace_minecart_has_fuel = entity_type_id
                    == VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID
                    && vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type(
                        entity_type_id,
                    )
                    .is_some_and(|(data_id, default)| {
                        entity_data_bool(&metadata.data_values, data_id, default)
                    });
                let is_slime = vanilla_entity_metadata_type_is_slime(entity_type_id);
                let slime_size = if is_slime {
                    vanilla_entity_metadata_slime_size_accessor_for_entity_type(entity_type_id)
                        .map_or(1, |(data_id, default)| {
                            entity_data_int(&metadata.data_values, data_id, default)
                        })
                        .clamp(1, 127)
                } else {
                    1
                };
                let slime_width = if is_slime {
                    vanilla_bounds_for_entity_data(
                        entity_type_id,
                        identity.data,
                        &metadata.data_values,
                        &attributes.attributes,
                        Some(animations.animations),
                        metadata.painting_variant_projection.as_ref(),
                    )
                    .map_or(0.52 * slime_size as f32, |bounds| {
                        bounds.max[0] - bounds.min[0]
                    })
                } else {
                    0.52
                };
                let is_silent = (entity_type_id == VANILLA_ENTITY_TYPE_BLAZE_ID
                    || is_guardian
                    || is_nautilus
                    || is_slime)
                    && vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
                        entity_data_bool(
                            &metadata.data_values,
                            accessors.silent_data_id,
                            accessors.silent_default,
                        )
                    });
                if deferred_local_item_based_steering_vehicle_id != Some(identity.id) {
                    if let Some((controller_x_rot, controller_y_rot)) =
                        item_based_steering_controller_rotation
                    {
                        if let Some((boost_time_data_id, boost_time_default)) =
                            vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
                                entity_type_id,
                            )
                        {
                            transform.x_rot = controller_x_rot * 0.5;
                            transform.y_rot = controller_y_rot;
                            transform.old_y_rot = controller_y_rot;
                            transform.y_head_rot = controller_y_rot;
                            let boost_time_total = entity_data_int(
                                &metadata.data_values,
                                boost_time_data_id,
                                boost_time_default,
                            );
                            let _ = animations
                                .animations
                                .advance_item_based_steering_tick(boost_time_total);
                        }
                    }
                }
                let client_animations_before_tick = animations
                    .animations
                    .needs_particle_trigger_bounds_snapshot()
                    .then_some(animations.animations);
                let mut particle_triggers = animations.animations.advance_client_tick(
                    identity.entity_type_id,
                    identity.id,
                    *transform,
                    should_tick_death,
                    is_passenger,
                    is_baby,
                    is_fall_flying,
                    is_crouching,
                    bat_is_resting,
                    in_water,
                    is_moving,
                    guardian_can_stand_on_below,
                    warden_heartbeat_delay,
                    guardian_attack_target_id,
                    guardian_attack_target,
                    guardian_eye_height,
                    wither_head_targets,
                    camel_is_standing,
                    camel_is_dashing,
                    copper_golem_animation.idle,
                    copper_golem_animation.getting_item,
                    copper_golem_animation.getting_no_item,
                    copper_golem_animation.dropping_item,
                    copper_golem_animation.dropping_no_item,
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
                    deferred_boat_paddle_entity_id != Some(identity.id),
                    boat_paddles[0],
                    boat_paddles[1],
                    boat_bubble_time_value,
                    furnace_minecart_has_fuel,
                    slime_size,
                    slime_width,
                    is_silent,
                    is_swimming,
                );
                if is_nautilus && deferred_local_nautilus_vehicle_id != Some(identity.id) {
                    let fluid_facts = fluid_facts_by_id
                        .get(&identity.id)
                        .copied()
                        .unwrap_or_default();
                    let nautilus_triggers = animations.animations.advance_nautilus_subclass_tick(
                        identity.id,
                        nautilus_is_dashing,
                        fluid_facts.in_water,
                        fluid_facts.eye_in_water,
                        is_silent,
                        is_baby,
                        transform.delta_movement,
                    );
                    if nautilus_triggers.nautilus_clear_dash {
                        tick_scratch.nautilus_clear_dash_entities.push(identity.id);
                    }
                    particle_triggers.nautilus_dash_ready_sound =
                        nautilus_triggers.nautilus_dash_ready_sound;
                    particle_triggers.nautilus_bubble = nautilus_triggers.nautilus_bubble;
                }
                append_client_tick_particle_triggers(
                    &mut self.pending_ravager_stun_particles,
                    &mut self.pending_evoker_fangs_crit_particles,
                    &mut self.pending_entity_client_movement_effects,
                    identity,
                    transform,
                    &metadata.data_values,
                    &attributes.attributes,
                    client_animations_before_tick,
                    &mut particle_triggers,
                );
                if is_boat && deferred_boat_paddle_entity_id != Some(identity.id) {
                    if let (Some(facts), Some(pose)) =
                        (boat_facts, local_player_pose.as_deref_mut())
                    {
                        apply_boat_push_to_local_player(
                            transform,
                            mount,
                            facts,
                            pose,
                            local_player_is_vehicle,
                        );
                    }
                }
                if let Some(update) = particle_triggers.guardian_look_update {
                    transform.x_rot = update.x_rot;
                    transform.y_head_rot = update.y_head_rot;
                }
                if let Some(update) = particle_triggers.guardian_dry_jump {
                    transform.delta_movement.x += update.x_impulse;
                    transform.delta_movement.y += 0.5;
                    transform.delta_movement.z += update.z_impulse;
                    transform.y_rot = update.y_rot;
                    transform.on_ground = Some(false);
                    tick_scratch.guardian_dry_jump_entities.push(identity.id);
                }
                if guardian_attack_target_id != 0 {
                    // Guardian's final `hasActiveAttackTarget` branch follows
                    // both LookControl and the optional dry random body yaw.
                    transform.y_rot = transform.y_head_rot;
                }
                // `advance_client_tick` always advances `age_ticks`, including
                // entities without a specialized animation family.
                work.mutated_entities = work.mutated_entities.saturating_add(1);
            }
            for entity_id in tick_scratch.guardian_dry_jump_entities.drain(..) {
                if let Some(&entity) = self.by_protocol_id.get(&entity_id) {
                    if let Ok(mut supporting_block) =
                        self.ecs.get::<&mut EntitySupportingBlock>(entity)
                    {
                        supporting_block.clear_physical();
                    }
                }
            }
            for entity_id in tick_scratch.camel_clear_dash_entities.drain(..) {
                let Some(&entity) = self.by_protocol_id.get(&entity_id) else {
                    continue;
                };
                let Some(entity_type_id) = self
                    .ecs
                    .get::<&EntityIdentity>(entity)
                    .ok()
                    .map(|identity| identity.entity_type_id)
                else {
                    continue;
                };
                let Some((data_id, _)) =
                    vanilla_entity_metadata_camel_dash_accessor_for_entity_type(entity_type_id)
                else {
                    continue;
                };
                if let Ok(mut metadata) = self.ecs.get::<&mut EntityMetadata>(entity) {
                    upsert_metadata_value(
                        &mut metadata,
                        ProtocolEntityDataValue {
                            data_id,
                            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                            value: EntityDataValueKind::Boolean(false),
                        },
                    );
                }
            }
            for entity_id in tick_scratch.camel_stand_instantly_entities.drain(..) {
                let Some(&entity) = self.by_protocol_id.get(&entity_id) else {
                    continue;
                };
                let Some(entity_type_id) = self
                    .ecs
                    .get::<&EntityIdentity>(entity)
                    .ok()
                    .map(|identity| identity.entity_type_id)
                else {
                    continue;
                };
                let (Some(entity_accessors), Some((last_pose_data_id, _))) = (
                    vanilla_entity_metadata_entity_accessors(),
                    vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
                        entity_type_id,
                    ),
                ) else {
                    continue;
                };
                if let Ok(mut metadata) = self.ecs.get::<&mut EntityMetadata>(entity) {
                    upsert_metadata_value(
                        &mut metadata,
                        ProtocolEntityDataValue {
                            data_id: entity_accessors.pose_data_id,
                            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
                            value: EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_STANDING_ID),
                        },
                    );
                    upsert_metadata_value(
                        &mut metadata,
                        ProtocolEntityDataValue {
                            data_id: last_pose_data_id,
                            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
                            value: EntityDataValueKind::Long(game_time.saturating_sub(53).max(0)),
                        },
                    );
                }
            }
            for entity_id in tick_scratch.nautilus_clear_dash_entities.drain(..) {
                let Some(&entity) = self.by_protocol_id.get(&entity_id) else {
                    continue;
                };
                if let Ok(mut metadata) = self.ecs.get::<&mut EntityMetadata>(entity) {
                    upsert_metadata_value(
                        &mut metadata,
                        ProtocolEntityDataValue {
                            data_id: VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
                            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                            value: EntityDataValueKind::Boolean(false),
                        },
                    );
                }
            }
            let block_effect_duration_this_tick = work
                .block_effect_duration
                .saturating_sub(block_effect_duration_before);
            work.animation_duration += animation_started_at
                .elapsed()
                .saturating_sub(block_effect_duration_this_tick);
            self.client_tick_scratch = tick_scratch;
        }
        work
    }

    /// Advances the direct local root boat's paddles after its control/move
    /// owner. Remote boats advance in the ordered entity-tick query above.
    pub(crate) fn advance_deferred_boat_paddles(
        &mut self,
        entity_id: i32,
        left_active: bool,
        right_active: bool,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&entity_id).copied()?;
        let (identity, transform, metadata, animations) = self
            .ecs
            .query_one_mut::<(
                &EntityIdentity,
                &EntityTransform,
                &EntityMetadata,
                &mut EntityClientAnimations,
            )>(entity)
            .ok()?;
        let triggers = animations
            .animations
            .advance_boat_paddles(left_active, right_active);
        append_boat_paddle_sound_triggers(
            &mut self.pending_entity_client_movement_effects,
            identity,
            transform,
            &metadata.data_values,
            triggers,
        );
        Some(())
    }

    fn fill_client_tick_schedule<F>(
        &self,
        scratch: &mut EntityClientTickScratch,
        runs_normally: bool,
        is_ticking_entity: &F,
        mut boat_inputs: Option<&mut Vec<EntityBoatClientTickInput>>,
    ) where
        F: Fn(i32, i32, EntityVec3) -> bool,
    {
        scratch.clear();
        if !runs_normally {
            self.fill_player_passenger_ancestors(scratch);
        }
        for &id in &self.order {
            let Some(entity) = self.by_protocol_id.get(&id).copied() else {
                continue;
            };
            let (Ok(identity), Ok(transform), Ok(mount)) = (
                self.ecs.get::<&EntityIdentity>(entity),
                self.ecs.get::<&EntityTransform>(entity),
                self.ecs.get::<&EntityMount>(entity),
            ) else {
                continue;
            };
            if mount.vehicle_id.is_some()
                || !is_ticking_entity(identity.id, identity.entity_type_id, transform.position)
                || (!runs_normally
                    && identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
                    && !scratch.player_passenger_ancestors.contains(&identity.id))
            {
                continue;
            }
            self.append_client_tick_passenger_tree(
                id,
                scratch,
                is_ticking_entity,
                &mut boat_inputs,
            );
        }
    }

    fn prepare_scheduled_passengers_for_ride_tick(&mut self, scheduled_entities: &[Entity]) {
        for &entity in scheduled_entities {
            let Ok((mount, transform)) = self
                .ecs
                .query_one_mut::<(&EntityMount, &mut EntityTransform)>(entity)
            else {
                continue;
            };
            if mount.vehicle_id.is_some() {
                // Vanilla `Entity.rideTick` clears motion immediately before
                // calling the entity's ordinary `tick`. Specialized client
                // movement therefore starts from zero rather than retaining a
                // stale packet velocity. Root entities keep their velocity.
                transform.delta_movement = EntityVec3::default();
            }
        }
    }

    fn fill_player_passenger_ancestors(&self, scratch: &mut EntityClientTickScratch) {
        let max_depth = self.by_protocol_id.len();
        for &id in &self.order {
            let Some(entity) = self.by_protocol_id.get(&id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
                continue;
            }

            let mut passenger_id = identity.id;
            for _ in 0..max_depth {
                let Some(passenger) = self.by_protocol_id.get(&passenger_id).copied() else {
                    break;
                };
                let Ok(mount) = self.ecs.get::<&EntityMount>(passenger) else {
                    break;
                };
                let Some(vehicle_id) = mount.vehicle_id else {
                    break;
                };
                scratch.player_passenger_ancestors.insert(vehicle_id);
                passenger_id = vehicle_id;
            }
        }
    }

    fn append_client_tick_passenger_tree<F>(
        &self,
        id: i32,
        scratch: &mut EntityClientTickScratch,
        is_ticking_entity: &F,
        boat_inputs: &mut Option<&mut Vec<EntityBoatClientTickInput>>,
    ) where
        F: Fn(i32, i32, EntityVec3) -> bool,
    {
        let Some(entity) = self.by_protocol_id.get(&id).copied() else {
            return;
        };
        if !scratch.scheduled_set.insert(id) {
            return;
        }
        scratch.scheduled_entities.push(entity);
        let (Ok(identity), Ok(transform), Ok(mount)) = (
            self.ecs.get::<&EntityIdentity>(entity),
            self.ecs.get::<&EntityTransform>(entity),
            self.ecs.get::<&EntityMount>(entity),
        ) else {
            return;
        };
        if is_vanilla_boat_type(identity.entity_type_id) {
            if let (Some(inputs), Some([width, height])) = (
                boat_inputs.as_deref_mut(),
                vanilla_entity_type_dimensions(identity.entity_type_id),
            ) {
                let half_width = f64::from(width) * 0.5;
                inputs.push(EntityBoatClientTickInput {
                    entity_id: identity.id,
                    entity_type_id: identity.entity_type_id,
                    uuid: identity.uuid,
                    position: transform.position,
                    relative_min: [-half_width, 0.0, -half_width],
                    relative_max: [half_width, f64::from(height), half_width],
                });
            }
        }

        for &passenger_id in mount.passengers.iter() {
            let Some(passenger) = self.by_protocol_id.get(&passenger_id).copied() else {
                continue;
            };
            let (Ok(identity), Ok(transform), Ok(mount)) = (
                self.ecs.get::<&EntityIdentity>(passenger),
                self.ecs.get::<&EntityTransform>(passenger),
                self.ecs.get::<&EntityMount>(passenger),
            ) else {
                continue;
            };
            if mount.vehicle_id != Some(id)
                || (identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
                    && !is_ticking_entity(identity.id, identity.entity_type_id, transform.position))
            {
                continue;
            }
            self.append_client_tick_passenger_tree(
                passenger_id,
                scratch,
                is_ticking_entity,
                boat_inputs,
            );
        }
    }

    pub(crate) fn publication_snapshot_observation(&self) -> EntityPublicationSnapshotObservation {
        self.publication_snapshot_observation
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
            .map(|lerp| lerp.pending_steps.len() + lerp.steps.len())
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
            if let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) {
                if self.protocol_id_by_uuid.get(&identity.uuid) == Some(id) {
                    self.protocol_id_by_uuid.remove(&identity.uuid);
                }
            }
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
        if let Ok(mut support) = self.ecs.get::<&mut EntitySupportingBlock>(entity) {
            *support = EntitySupportingBlock::from(state);
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
        if state.entity_type_id == VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID {
            let command_block = EntityMinecartCommandBlock::from(state);
            let updated = self
                .ecs
                .get::<&mut EntityMinecartCommandBlock>(entity)
                .map(|mut existing| *existing = command_block.clone())
                .is_ok();
            if !updated {
                let _ = self.ecs.insert_one(entity, command_block);
            }
        } else {
            let _ = self.ecs.remove_one::<EntityMinecartCommandBlock>(entity);
        }
        self.sync_hurting_projectile_from_state(entity, state);
        self.sync_projectile_lifecycle_from_state(entity, state);
        if state.entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID {
            let movement = super::client_movement::experience_orb_movement_from_uuid(state.uuid);
            let updated = self
                .ecs
                .get::<&mut EntityExperienceOrbMovement>(entity)
                .map(|mut existing| *existing = movement)
                .is_ok();
            if !updated {
                let _ = self.ecs.insert_one(entity, movement);
            }
        } else {
            let _ = self.ecs.remove_one::<EntityExperienceOrbMovement>(entity);
        }
        if is_vanilla_boat_type(state.entity_type_id) {
            let updated = self
                .ecs
                .get::<&mut EntityBoatMovement>(entity)
                .map(|mut existing| *existing = EntityBoatMovement::default())
                .is_ok();
            if !updated {
                let _ = self.ecs.insert_one(entity, EntityBoatMovement::default());
            }
        } else {
            let _ = self.ecs.remove_one::<EntityBoatMovement>(entity);
        }
        if state.entity_type_id == VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID {
            let movement = super::client_movement::fishing_hook_movement_from_uuid(
                state.uuid,
                self.uuid_for_protocol_id(state.data),
            );
            let updated = self
                .ecs
                .get::<&mut EntityFishingHookMovement>(entity)
                .map(|mut existing| *existing = movement)
                .is_ok();
            if !updated {
                let _ = self.ecs.insert_one(entity, movement);
            }
        } else {
            let _ = self.ecs.remove_one::<EntityFishingHookMovement>(entity);
        }
        if let Ok(mut facts) = self
            .ecs
            .get::<&mut EntityClientMovementCollisionFacts>(entity)
        {
            *facts = EntityClientMovementCollisionFacts::default();
        }
        sync_derived_marker::<EntityClientMovementCollisionFacts>(
            &mut self.ecs,
            entity,
            super::client_movement::client_movement_uses_collision_facts(state.entity_type_id),
        );
        sync_derived_marker::<EntityClientMovementFluidFacts>(
            &mut self.ecs,
            entity,
            super::client_movement::client_movement_uses_fluid_contact(state.entity_type_id),
        );
        sync_derived_marker::<EntityExperienceOrbPlayerTargetFacts>(
            &mut self.ecs,
            entity,
            state.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID,
        );
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

    fn sync_projectile_lifecycle_from_state(&mut self, entity: Entity, state: &EntityState) {
        let projectile = entity_projectile_lifecycle_from_state(
            state.entity_type_id,
            state.projectile_lifecycle,
            self.uuid_for_protocol_id(state.data),
        );
        if let Some(projectile) = projectile {
            let updated = {
                if let Ok(mut existing) = self.ecs.get::<&mut EntityProjectileLifecycle>(entity) {
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
            let _ = self.ecs.remove_one::<EntityProjectileLifecycle>(entity);
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
        let supporting_block = self.ecs.get::<&EntitySupportingBlock>(entity).ok()?;
        let minecart_lerp = self.ecs.get::<&EntityMinecartLerp>(entity).ok();
        let minecart_command_block = self.ecs.get::<&EntityMinecartCommandBlock>(entity).ok();
        let hurting_projectile = self.ecs.get::<&EntityHurtingProjectile>(entity).ok();
        let projectile_lifecycle = self.ecs.get::<&EntityProjectileLifecycle>(entity).ok();

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
            interpolation: None,
            main_supporting_block_pos: None,
            on_ground_no_blocks: false,
            data_values: Vec::new(),
            painting_variant_projection: None,
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
            damage_invulnerable_time: 0,
            minecart_lerp_pending_steps: Vec::new(),
            minecart_lerp_steps: Vec::new(),
            minecart_lerp_old_step: None,
            minecart_lerp_delay: 0,
            minecart_command_block_track_output: None,
            hurting_projectile: None,
            projectile_lifecycle: None,
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
        (*damage).clone().write_to_state(&mut state);
        (*supporting_block).write_to_state(&mut state);
        if let Some(minecart_lerp) = minecart_lerp {
            (*minecart_lerp).clone().write_to_state(&mut state);
        }
        if let Some(command_block) = minecart_command_block {
            (*command_block).clone().write_to_state(&mut state);
        }
        if let Some(projectile) = hurting_projectile {
            (*projectile).write_to_state(&mut state);
        }
        if let Some(projectile) = projectile_lifecycle {
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
            protocol_id_by_uuid: HashMap::new(),
            order: Vec::new(),
            next_incarnation: 1,
            pending_ravager_stun_particles: Vec::new(),
            pending_evoker_fangs_crit_particles: Vec::new(),
            pending_entity_client_movement_effects: Vec::new(),
            pending_ominous_item_spawner_particles: Vec::new(),
            client_tick_scratch: EntityClientTickScratch::default(),
            publication_snapshot_observation: EntityPublicationSnapshotObservation::default(),
        }
    }
}

impl Clone for EntityStore {
    fn clone(&self) -> Self {
        let snapshot_started_at = Instant::now();
        // Clone component-by-component instead of round-tripping every entity
        // through a full `EntityState` projection. Immutable simulation
        // publications still rebuild this ECS explicitly: keeping that cost at
        // publication avoids a whole-store COW that would merely move the same
        // work into the next entity mutation.
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
                Ok(supporting_block),
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
                self.ecs.get::<&EntitySupportingBlock>(entity),
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
                (*damage).clone(),
                *supporting_block,
            ));
            let entity_type_id = identity.entity_type_id;
            if let Ok(lerp) = self.ecs.get::<&EntityMinecartLerp>(entity) {
                let _ = store.ecs.insert_one(cloned, (*lerp).clone());
            }
            if let Ok(command_block) = self.ecs.get::<&EntityMinecartCommandBlock>(entity) {
                let _ = store.ecs.insert_one(cloned, (*command_block).clone());
            }
            if let Ok(projectile) = self.ecs.get::<&EntityHurtingProjectile>(entity) {
                let _ = store.ecs.insert_one(cloned, *projectile);
            }
            if let Ok(projectile) = self.ecs.get::<&EntityProjectileLifecycle>(entity) {
                let _ = store.ecs.insert_one(cloned, *projectile);
            }
            if entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID {
                if let Ok(movement) = self.ecs.get::<&EntityExperienceOrbMovement>(entity) {
                    let _ = store.ecs.insert_one(cloned, *movement);
                }
            }
            if is_vanilla_boat_type(entity_type_id) {
                if let Ok(movement) = self.ecs.get::<&EntityBoatMovement>(entity) {
                    let _ = store.ecs.insert_one(cloned, *movement);
                }
            }
            if entity_type_id == VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID {
                if let Ok(movement) = self.ecs.get::<&EntityFishingHookMovement>(entity) {
                    let _ = store.ecs.insert_one(cloned, *movement);
                }
            }
            if super::client_movement::client_movement_uses_collision_facts(entity_type_id) {
                if let Ok(facts) = self.ecs.get::<&EntityClientMovementCollisionFacts>(entity) {
                    let _ = store.ecs.insert_one(cloned, (*facts).clone());
                }
            }
            if super::client_movement::client_movement_uses_fluid_contact(entity_type_id) {
                let _ = store.ecs.insert_one(cloned, EntityClientMovementFluidFacts);
            }
            if entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
                let _ = store
                    .ecs
                    .insert_one(cloned, EntityExperienceOrbPlayerTargetFacts);
            }
            store.by_protocol_id.insert(id, cloned);
        }
        store.protocol_id_by_uuid = self.protocol_id_by_uuid.clone();
        store.order = self.order.clone();
        store.next_incarnation = self.next_incarnation;
        store.pending_ravager_stun_particles = self.pending_ravager_stun_particles.clone();
        store.pending_evoker_fangs_crit_particles =
            self.pending_evoker_fangs_crit_particles.clone();
        store.pending_entity_client_movement_effects =
            self.pending_entity_client_movement_effects.clone();
        store.pending_ominous_item_spawner_particles =
            self.pending_ominous_item_spawner_particles.clone();
        store.publication_snapshot_observation = EntityPublicationSnapshotObservation {
            duration: snapshot_started_at.elapsed(),
            rebuilt_entities: store.by_protocol_id.len(),
        };
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

fn minecart_display_offset_for_components(
    entity_type_id: i32,
    data_values: &[ProtocolEntityDataValue],
    registries: &RegistrySet,
) -> Option<i32> {
    if !is_vanilla_minecart_type(entity_type_id) {
        return None;
    }
    let accessors =
        vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type(entity_type_id)?;
    let custom_state_id = data_values
        .iter()
        .find(|value| value.data_id == accessors.custom_display_block_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::OptionalBlockState(state) => *state,
            _ => None,
        });
    let has_display = if let Some(state_id) = custom_state_id {
        registries
            .block_state(state_id)
            .is_some_and(|state| state.name != "minecraft:air")
    } else {
        default_minecart_display_block_kind(entity_type_id).is_some()
    };
    has_display.then(|| {
        data_values
            .iter()
            .find(|value| value.data_id == accessors.display_offset_data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::Int(value) => Some(*value),
                _ => None,
            })
            .unwrap_or(accessors.display_offset_default)
    })
}

fn default_minecart_display_block_state(
    entity_type_id: i32,
    furnace_has_fuel: bool,
    registries: &RegistrySet,
) -> Option<EntityBlockModelState> {
    match default_minecart_display_block_kind(entity_type_id)? {
        DefaultMinecartDisplayBlockKind::Chest => registered_block_model_state(
            registries,
            "minecraft:chest",
            BTreeMap::from([
                ("facing".to_string(), "north".to_string()),
                ("type".to_string(), "single".to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]),
        ),
        DefaultMinecartDisplayBlockKind::CommandBlock => registered_block_model_state(
            registries,
            "minecraft:command_block",
            BTreeMap::from([
                ("conditional".to_string(), "false".to_string()),
                ("facing".to_string(), "north".to_string()),
            ]),
        ),
        DefaultMinecartDisplayBlockKind::Furnace => registered_block_model_state(
            registries,
            "minecraft:furnace",
            BTreeMap::from([
                ("facing".to_string(), "north".to_string()),
                ("lit".to_string(), furnace_has_fuel.to_string()),
            ]),
        ),
        DefaultMinecartDisplayBlockKind::Hopper => registered_block_model_state(
            registries,
            "minecraft:hopper",
            BTreeMap::from([
                ("enabled".to_string(), "true".to_string()),
                ("facing".to_string(), "down".to_string()),
            ]),
        ),
        DefaultMinecartDisplayBlockKind::Spawner => {
            registered_block_model_state(registries, "minecraft:spawner", BTreeMap::new())
        }
        DefaultMinecartDisplayBlockKind::Tnt => registered_block_model_state(
            registries,
            "minecraft:tnt",
            BTreeMap::from([("unstable".to_string(), "false".to_string())]),
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DefaultMinecartDisplayBlockKind {
    Chest,
    CommandBlock,
    Furnace,
    Hopper,
    Spawner,
    Tnt,
}

fn default_minecart_display_block_kind(
    entity_type_id: i32,
) -> Option<DefaultMinecartDisplayBlockKind> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID => Some(DefaultMinecartDisplayBlockKind::Chest),
        VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID => {
            Some(DefaultMinecartDisplayBlockKind::CommandBlock)
        }
        VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID => Some(DefaultMinecartDisplayBlockKind::Furnace),
        VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID => Some(DefaultMinecartDisplayBlockKind::Hopper),
        VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID => Some(DefaultMinecartDisplayBlockKind::Spawner),
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID => Some(DefaultMinecartDisplayBlockKind::Tnt),
        _ => None,
    }
}

fn default_primed_tnt_block_state(
    registries: &RegistrySet,
    default: crate::entities::schema::VanillaEntityMetadataDefault,
) -> Option<EntityBlockModelState> {
    let crate::entities::schema::VanillaEntityMetadataDefault::BlockStateDefault {
        resource_id,
        properties,
    } = default
    else {
        unreachable!("generated PrimedTnt block-state default has the wrong kind")
    };
    registered_block_model_state(
        registries,
        resource_id,
        properties
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect(),
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

fn entity_block_pos(position: super::EntityVec3) -> crate::BlockPos {
    crate::BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

fn apply_boat_push_to_local_player(
    boat_transform: &mut EntityTransform,
    boat_mount: &EntityMount,
    facts: &EntityBoatClientTickWorldFacts,
    local_player_pose: &mut LocalPlayerPoseState,
    local_player_is_vehicle: bool,
) {
    if !facts.can_push_local_player {
        return;
    }

    // `ClientLevel.getPushableEntities` tests the local player's bounds
    // against `boat.getBoundingBox().inflate(0.2F, -0.01F, 0.2F)` before
    // `AbstractBoat.push` applies its additional vertical gate.
    let boat_min = [
        boat_transform.position.x + facts.relative_min[0],
        boat_transform.position.y + facts.relative_min[1],
        boat_transform.position.z + facts.relative_min[2],
    ];
    let boat_max = [
        boat_transform.position.x + facts.relative_max[0],
        boat_transform.position.y + facts.relative_max[1],
        boat_transform.position.z + facts.relative_max[2],
    ];
    let player_half_width = f64::from(0.6_f32) * 0.5;
    let player_min = [
        local_player_pose.position.x - player_half_width,
        local_player_pose.position.y,
        local_player_pose.position.z - player_half_width,
    ];
    let player_max = [
        local_player_pose.position.x + player_half_width,
        local_player_pose.position.y + local_player_pose.body_height(),
        local_player_pose.position.z + player_half_width,
    ];
    let query_min = [
        boat_min[0] - f64::from(0.2_f32),
        boat_min[1] + f64::from(0.01_f32),
        boat_min[2] - f64::from(0.2_f32),
    ];
    let query_max = [
        boat_max[0] + f64::from(0.2_f32),
        boat_max[1] - f64::from(0.01_f32),
        boat_max[2] + f64::from(0.2_f32),
    ];
    if !(player_min[0] < query_max[0]
        && player_max[0] > query_min[0]
        && player_min[1] < query_max[1]
        && player_max[1] > query_min[1]
        && player_min[2] < query_max[2]
        && player_max[2] > query_min[2]
        && player_min[1] <= boat_min[1])
    {
        return;
    }

    let mut xa = local_player_pose.position.x - boat_transform.position.x;
    let mut za = local_player_pose.position.z - boat_transform.position.z;
    let mut distance = xa.abs().max(za.abs());
    if distance < f64::from(0.01_f32) {
        return;
    }
    distance = distance.sqrt();
    xa /= distance;
    za /= distance;
    let power = (1.0_f64 / distance).min(1.0_f64);
    xa *= power * f64::from(0.05_f32);
    za *= power * f64::from(0.05_f32);

    if boat_mount.passengers.is_empty() {
        boat_transform.delta_movement.x -= xa;
        boat_transform.delta_movement.z -= za;
    }
    if !local_player_is_vehicle {
        local_player_pose.delta_movement.x += xa;
        local_player_pose.delta_movement.z += za;
    }
}

fn append_client_tick_particle_triggers(
    pending_ravager_stun_particles: &mut Vec<RavagerStunParticleState>,
    pending_evoker_fangs_crit_particles: &mut Vec<EvokerFangsCritParticleState>,
    pending_entity_client_movement_effects: &mut Vec<EntityClientMovementEffectState>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    attributes: &[ProtocolAttributeSnapshot],
    client_animations: Option<EntityClientAnimationState>,
    triggers: &mut EntityClientTickParticleTriggers,
) {
    if let Some(landing) = triggers.slime_landing_particles.take() {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::SlimeLanding(SlimeLandingParticleState {
                entity_id: identity.id,
                kind: landing.kind,
                positions: landing
                    .offsets
                    .into_iter()
                    .map(|offset| super::EntityVec3 {
                        x: transform.position.x + offset.x,
                        y: transform.position.y,
                        z: transform.position.z + offset.z,
                    })
                    .collect(),
            }),
        ));
    }
    if let Some(sound) = triggers.slime_squish_sound {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Sound(
            super::EntityClientMovementSoundState {
                entity_id: identity.id,
                kind: super::EntityClientMovementSoundKind::SlimeSquish {
                    magma_cube: sound.magma_cube,
                    tiny: sound.tiny,
                    volume: sound.volume,
                    pitch: sound.pitch,
                },
                position: transform.position,
            },
        ));
    }
    if triggers.camel_dash_ready_sound {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Sound(
            super::EntityClientMovementSoundState {
                entity_id: identity.id,
                kind: super::EntityClientMovementSoundKind::CamelDashReady {
                    husk: identity.entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
                },
                position: transform.position,
            },
        ));
    }
    append_nautilus_tick_triggers(
        pending_entity_client_movement_effects,
        identity,
        transform,
        triggers,
    );
    if triggers.minecart_tnt_smoke {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::Simple(EntityClientMovementSimpleParticleState {
                entity_id: identity.id,
                kind: EntityClientMovementSimpleParticleKind::Smoke,
                position: super::EntityVec3 {
                    x: transform.position.x,
                    y: transform.position.y + 0.5,
                    z: transform.position.z,
                },
                velocity: super::EntityVec3::default(),
            }),
        ));
    }
    if triggers.minecart_furnace_large_smoke {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::Simple(EntityClientMovementSimpleParticleState {
                entity_id: identity.id,
                kind: EntityClientMovementSimpleParticleKind::LargeSmoke,
                position: super::EntityVec3 {
                    x: transform.position.x,
                    y: transform.position.y + 0.8,
                    z: transform.position.z,
                },
                velocity: super::EntityVec3::default(),
            }),
        ));
    }
    if let Some(sound) = triggers.blaze_burn_sound {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Sound(
            super::EntityClientMovementSoundState {
                entity_id: identity.id,
                kind: super::EntityClientMovementSoundKind::BlazeBurn {
                    volume: sound.volume,
                    pitch: sound.pitch,
                },
                position: super::EntityVec3 {
                    x: transform.position.x + 0.5,
                    y: transform.position.y + 0.5,
                    z: transform.position.z + 0.5,
                },
            },
        ));
    }
    if let Some(offsets) = triggers.blaze_large_smoke {
        // Blaze has no pose/baby-specific dimensions. Avoid the general model
        // projection ladder in this every-tick path: generated type dimensions
        // times the canonical LivingEntity SCALE attribute are its exact box.
        let Some([base_width, base_height]) =
            vanilla_entity_type_dimensions(identity.entity_type_id)
        else {
            return;
        };
        let scale = vanilla_render_scale(identity.entity_type_id, attributes);
        let width = f64::from(base_width * scale);
        let height = f64::from(base_height * scale);
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::BlazeAmbient(BlazeAmbientParticleState {
                entity_id: identity.id,
                positions: offsets.map(|offset| super::EntityVec3 {
                    x: transform.position.x + offset.x_unit * width,
                    y: transform.position.y + offset.y_unit * height,
                    z: transform.position.z + offset.z_unit * width,
                }),
            }),
        ));
    }
    if let Some(offsets) = triggers.ender_portal {
        let Some([base_width, base_height]) =
            vanilla_entity_type_dimensions(identity.entity_type_id)
        else {
            return;
        };
        let scale = vanilla_render_scale(identity.entity_type_id, attributes);
        let width = f64::from(base_width * scale);
        let height = f64::from(base_height * scale);
        let y_offset = if identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDERMAN_ID {
            -0.25
        } else {
            0.0
        };
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::EnderAmbient(EnderAmbientParticleState {
                entity_id: identity.id,
                positions: offsets.map(|offset| super::EntityVec3 {
                    x: transform.position.x + offset.x_unit * width,
                    y: transform.position.y + offset.y_unit * height + y_offset,
                    z: transform.position.z + offset.z_unit * width,
                }),
                velocities: offsets.map(|offset| offset.velocity),
            }),
        ));
    }
    if triggers.guardian_flop_sound {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Sound(
            super::EntityClientMovementSoundState {
                entity_id: identity.id,
                kind: super::EntityClientMovementSoundKind::GuardianFlop {
                    elder: matches!(
                        super::vanilla_entity_renderer_guardian_kind(identity.entity_type_id),
                        Some(super::VanillaGuardianRendererKind::ElderGuardian)
                    ),
                },
                position: transform.position,
            },
        ));
    }
    if let Some(offsets) = triggers.guardian_moving_bubbles {
        let Some([base_width, base_height]) =
            vanilla_entity_type_dimensions(identity.entity_type_id)
        else {
            return;
        };
        let scale = vanilla_render_scale(identity.entity_type_id, attributes);
        let width = f64::from(base_width * scale);
        let height = f64::from(base_height * scale);
        // Guardian calls `getViewVector(0.0F)`: Vanilla samples the tick-start
        // rotation endpoints, even if this tick advanced interpolation.
        let view = super::calculate_view_vector(transform.old_x_rot, transform.old_y_rot);
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::GuardianAmbient(GuardianAmbientParticleState {
                entity_id: identity.id,
                positions: offsets.map(|offset| super::EntityVec3 {
                    x: transform.position.x + offset.x_unit * width - view.x * 1.5,
                    y: transform.position.y + offset.y_unit * height - view.y * 1.5,
                    z: transform.position.z + offset.z_unit * width - view.z * 1.5,
                }),
            }),
        ));
    }
    if let Some(positions) = triggers.guardian_attack_bubbles.take() {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::GuardianAttack(GuardianAttackParticleState {
                entity_id: identity.id,
                positions,
            }),
        ));
    }
    append_boat_paddle_sound_triggers(
        pending_entity_client_movement_effects,
        identity,
        transform,
        data_values,
        triggers.boat_paddle_sounds,
    );
    if triggers.ravager_stun.is_none() && triggers.evoker_fangs_crit.is_none() {
        return;
    }
    let Some(client_animations) = client_animations else {
        debug_assert!(false, "particle bounds trigger lacked its pre-tick state");
        return;
    };
    let Some(bounds) = vanilla_model_source_bounds_for_entity_data(
        identity.entity_type_id,
        identity.data,
        data_values,
        attributes,
        Some(client_animations),
        None,
    ) else {
        return;
    };

    if let Some(offset) = triggers.ravager_stun {
        let width = f64::from(bounds.max[0] - bounds.min[0]);
        let height = f64::from(bounds.max[1] - bounds.min[1]);
        let y_body_rot = f64::from(transform.y_rot).to_radians();
        pending_ravager_stun_particles.push(RavagerStunParticleState {
            entity_id: identity.id,
            position: super::EntityVec3 {
                x: transform.position.x - width * y_body_rot.sin() + offset.jitter_x,
                y: transform.position.y + height - 0.3,
                z: transform.position.z + width * y_body_rot.cos() + offset.jitter_z,
            },
        });
    }

    if let Some(offsets) = triggers.evoker_fangs_crit {
        let half_width = f64::from(bounds.max[0] - bounds.min[0]) * 0.5;
        pending_evoker_fangs_crit_particles.extend(offsets.into_iter().map(|offset| {
            EvokerFangsCritParticleState {
                entity_id: identity.id,
                position: super::EntityVec3 {
                    x: transform.position.x + offset.x_unit * half_width,
                    y: transform.position.y + offset.y_offset,
                    z: transform.position.z + offset.z_unit * half_width,
                },
                velocity: offset.velocity,
            }
        }));
    }
}

fn append_nautilus_tick_triggers(
    pending_entity_client_movement_effects: &mut Vec<EntityClientMovementEffectState>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    triggers: &EntityClientTickParticleTriggers,
) {
    if let Some(sound) = triggers.nautilus_dash_ready_sound {
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Sound(
            super::EntityClientMovementSoundState {
                entity_id: identity.id,
                kind: super::EntityClientMovementSoundKind::NautilusDashReady {
                    zombie: identity.entity_type_id == VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
                    underwater: sound.underwater,
                    pitch: sound.pitch,
                },
                position: transform.position,
            },
        ));
    }
    if let Some(velocity) = triggers.nautilus_bubble {
        let view =
            super::calculate_view_vector(transform.x_rot.clamp(-10.0, 10.0), transform.y_rot);
        pending_entity_client_movement_effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::Simple(EntityClientMovementSimpleParticleState {
                entity_id: identity.id,
                kind: EntityClientMovementSimpleParticleKind::Bubble,
                position: super::EntityVec3 {
                    x: transform.position.x - view.x * 1.1,
                    y: transform.position.y - view.y + 0.25,
                    z: transform.position.z - view.z * 1.1,
                },
                velocity,
            }),
        ));
    }
}

fn append_boat_paddle_sound_triggers(
    pending_effects: &mut Vec<EntityClientMovementEffectState>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    triggers: [bool; 2],
) {
    if triggers == [false; 2]
        || vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
            entity_data_bool(
                data_values,
                accessors.silent_data_id,
                accessors.silent_default,
            )
        })
    {
        return;
    }
    for (side, triggered) in triggers.into_iter().enumerate() {
        if triggered {
            pending_effects.push(EntityClientMovementEffectState::Sound(
                super::EntityClientMovementSoundState {
                    entity_id: identity.id,
                    kind: if side == 0 {
                        super::EntityClientMovementSoundKind::BoatPaddleLeft
                    } else {
                        super::EntityClientMovementSoundKind::BoatPaddleRight
                    },
                    position: transform.position,
                },
            ));
        }
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

fn expand_bounds_towards_offset(
    mut bounds: super::EntityPickBoundsState,
    offset: super::EntityVec3,
) -> super::EntityPickBoundsState {
    for (axis, value) in [offset.x, offset.y, offset.z].into_iter().enumerate() {
        let value = value as f32;
        bounds.min[axis] = bounds.min[axis].min(bounds.min[axis] + value);
        bounds.max[axis] = bounds.max[axis].max(bounds.max[axis] + value);
    }
    bounds
}

fn solve_living_passenger_body_y_rot(head_y_rot: f32, vehicle_body_y_rot: f32) -> f32 {
    let head_difference = wrap_degrees_f32(head_y_rot - vehicle_body_y_rot).clamp(-85.0, 85.0);
    let mut body_y_rot = head_y_rot - head_difference;
    if head_difference.abs() > 50.0 {
        body_y_rot += head_difference * 0.2;
    }
    body_y_rot
}

fn wrap_degrees_f32(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

fn vanilla_attribute_value(attribute: &ProtocolAttributeSnapshot) -> f64 {
    let mut base = attribute.base;
    for modifier in &attribute.modifiers {
        if modifier.operation_id != VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID
            && modifier.operation_id != VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID
        {
            base += modifier.amount;
        }
    }

    let mut result = base;
    for modifier in &attribute.modifiers {
        if modifier.operation_id == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID {
            result += base * modifier.amount;
        }
    }
    for modifier in &attribute.modifiers {
        if modifier.operation_id == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID {
            result *= 1.0 + modifier.amount;
        }
    }
    result
}

/// Exact Vanilla `DefaultAttributes` supplier value, used until a
/// `ClientboundUpdateAttributesPacket` overrides it. Membership and family
/// overrides come from the generated EntityType profile; the Attribute
/// registry default is consulted only for the one-argument builder overload.
fn vanilla_default_attribute_base(entity_type_id: i32, attribute_id: i32) -> Option<f64> {
    let entity_default = vanilla_entity_type_default_attribute(entity_type_id, attribute_id)?;
    entity_default
        .base_value_override_bits
        .map(f64::from_bits)
        .or_else(|| {
            vanilla_attribute_type(attribute_id)
                .map(|attribute| f64::from_bits(attribute.default_value_bits))
        })
}

fn shulker_attach_face(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityAttachmentFace {
    let Some(accessors) = vanilla_entity_metadata_shulker_accessors_for_entity_type(entity_type_id)
    else {
        return EntityAttachmentFace::Down;
    };
    data_values
        .iter()
        .find_map(|value| {
            if value.data_id != accessors.attach_face_data_id {
                return None;
            }
            let EntityDataValueKind::Direction(direction) = &value.value else {
                return None;
            };
            Some(EntityAttachmentFace::from_3d_data(*direction))
        })
        .unwrap_or_else(|| EntityAttachmentFace::from_3d_data(accessors.attach_face_default))
}

fn firework_rocket_render_stack(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    accessors: VanillaFireworkRocketMetadataAccessors,
) -> Option<&bbb_protocol::packets::ItemStackSummary> {
    metadata_item_stack(data_values, accessors.item_data_id)
        .filter(|stack| stack.item_id.is_some() && stack.count > 0)
}

fn ominous_item_spawner_render_stack(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> Option<&bbb_protocol::packets::ItemStackSummary> {
    let data_id =
        vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type(entity_type_id)?;
    metadata_item_stack(data_values, data_id)
        .filter(|stack| stack.item_id.is_some() && stack.count > 0)
}

fn metadata_item_stack(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
) -> Option<&bbb_protocol::packets::ItemStackSummary> {
    data_values.iter().find_map(|value| {
        if value.data_id != data_id {
            return None;
        }
        let EntityDataValueKind::ItemStack(stack) = &value.value else {
            return None;
        };
        Some(stack)
    })
}

fn firework_rocket_attached_to_entity(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    accessors: VanillaFireworkRocketMetadataAccessors,
) -> bool {
    data_values.iter().any(|value| {
        value.data_id == accessors.attached_to_target_data_id
            && matches!(
                &value.value,
                EntityDataValueKind::OptionalUnsignedInt(Some(_))
            )
    })
}

fn firework_rocket_shot_at_angle(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    accessors: VanillaFireworkRocketMetadataAccessors,
) -> bool {
    data_values.iter().any(|value| {
        value.data_id == accessors.shot_at_angle_data_id
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
