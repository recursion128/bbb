use bbb_client_clock::Instant;
use bbb_protocol::content_registry::ContentRegistry;
use std::{
    fmt,
    sync::{Arc, OnceLock},
    time::Duration,
};

use bbb_protocol::attribute_modifier_types::{
    VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
    VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
};
use bbb_protocol::attribute_types::{
    vanilla_attribute_type, VANILLA_ATTRIBUTE_MAX_HEALTH_ID, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
};
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID, VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID, VANILLA_ENTITY_DATA_SERIALIZER_SNIFFER_STATE_ID,
};
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_DYING_ID,
    VANILLA_ENTITY_POSE_FALL_FLYING_ID, VANILLA_ENTITY_POSE_SLEEPING_ID,
    VANILLA_ENTITY_POSE_STANDING_ID, VANILLA_ENTITY_POSE_SWIMMING_ID,
};
use bbb_protocol::packets::{
    AttributeModifier as ProtocolAttributeModifier, AttributeSnapshot as ProtocolAttributeSnapshot,
    Vec3d as ProtocolVec3d,
};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use bbb_protocol::entity_types::{
    vanilla_entity_type_dimensions, VANILLA_ENTITY_TYPE_BLAZE_ID,
    VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
    VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_TRIDENT_ID,
    VANILLA_ENTITY_TYPE_VILLAGER_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_CARROT_ON_A_STICK_ID, VANILLA_ITEM_DRAGON_HEAD_ID, VANILLA_ITEM_FISHING_ROD_ID,
    VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
};
use bbb_protocol::packets::EquipmentSlot as ProtocolEquipmentSlot;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::packets::{
    EntityDataEnumSerializer, EntityDataRegistryHolder, EntityDataRegistryReference,
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
};

use super::display::{display_entity_culling_state, display_entity_render_source};
use super::entity_data::entity_data_byte;
use super::record::{
    EntityBoatMovement, EntityIncarnation, EntityInstanceReference, EntityRecord, EntitySpawnOwner,
};
use super::renderer_catalog::vanilla_living_entity_renderer_name_is_upside_down;
use super::schema::{
    vanilla_entity_metadata_abstract_horse_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type,
    vanilla_entity_metadata_accessor_for_entity_type,
    vanilla_entity_metadata_age_locked_accessor_for_entity_type,
    vanilla_entity_metadata_area_effect_cloud_particle_accessor_for_entity_type,
    vanilla_entity_metadata_area_effect_cloud_radius_accessor_for_entity_type,
    vanilla_entity_metadata_area_effect_cloud_waiting_accessor_for_entity_type,
    vanilla_entity_metadata_armor_stand_accessors_for_entity_type,
    vanilla_entity_metadata_bat_flags_accessor_for_entity_type,
    vanilla_entity_metadata_bee_accessors_for_entity_type,
    vanilla_entity_metadata_blaze_flags_accessor_for_entity_type,
    vanilla_entity_metadata_camel_dash_accessor_for_entity_type,
    vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type,
    vanilla_entity_metadata_copper_golem_accessors_for_entity_type,
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
    vanilla_entity_metadata_living_entity_effect_ambience_accessor_for_entity_type,
    vanilla_entity_metadata_living_entity_effect_particles_accessor_for_entity_type,
    vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type,
    vanilla_entity_metadata_panda_accessors_for_entity_type,
    vanilla_entity_metadata_player_absorption_accessor_for_entity_type,
    vanilla_entity_metadata_player_score_accessor_for_entity_type,
    vanilla_entity_metadata_primed_tnt_accessors_for_entity_type,
    vanilla_entity_metadata_shulker_accessors_for_entity_type,
    vanilla_entity_metadata_slime_size_accessor_for_entity_type,
    vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type,
    vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type,
    vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type,
    vanilla_entity_metadata_tamable_animal_accessors_for_entity_type,
    vanilla_entity_metadata_type_collidability_kind, vanilla_entity_metadata_type_is_camel,
    vanilla_entity_metadata_type_is_guardian, vanilla_entity_metadata_type_is_leashable,
    vanilla_entity_metadata_type_is_slime, vanilla_entity_metadata_type_pushability_kind,
    vanilla_entity_metadata_type_pushes_local_player_on_client_tick,
    vanilla_entity_metadata_type_uses_player_entity_event_handler,
    vanilla_entity_metadata_vex_flags_accessor_for_entity_type,
    vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type,
    vanilla_entity_type_default_attribute, VanillaEntityCollidabilityKind,
    VanillaEntityPushabilityKind, VanillaFireworkRocketMetadataAccessors,
    VANILLA_ENDERMAN_CREEPY_DATA_ID, VANILLA_ENDERMAN_DEFAULT_STARED_AT,
    VANILLA_ENDERMAN_STARED_AT_DATA_ID, VANILLA_FISHING_HOOK_BITING_DATA_ID,
    VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID,
};

use super::{
    calculate_view_vector, is_vanilla_abstract_nautilus_type, is_vanilla_boat_type,
    is_vanilla_can_wear_horse_armor_type, is_vanilla_llama_type, is_vanilla_minecart_type,
    is_vanilla_vehicle_entity_type, vanilla_entity_renderer_extracts_sheep_head_eat,
    vanilla_entity_type_is_squid, AbstractArrowRuntimeState, BlazeAmbientParticleState,
    DisplayEntityRenderSourceState, EnderAmbientParticleState, EntityAbstractArrowRuntime,
    EntityArmorTrimSprites, EntityAttachmentFace, EntityAttributes, EntityBlockEffectBaseTickReset,
    EntityBlockEffectMovement, EntityBlockEffectMovementIter, EntityBlockEffectQuery,
    EntityCameraHurtState, EntityCameraPoseState, EntityCameraViewBobState,
    EntityClientAnimationState, EntityClientMovementEffectState, EntityClientMovementParticleState,
    EntityClientMovementSimpleParticleKind, EntityClientMovementSimpleParticleState, EntityDamage,
    EntityDamageEventState, EntityEquipment, EntityEquipmentAssetId, EntityIdentity,
    EntityInterpolationCollisionInput, EntityLeash, EntityMetadata, EntityMinecartCommandBlock,
    EntityMinecartLerp, EntityMobEffects, EntityMount, EntityRenderSource, EntityRenderState,
    EntityRuntimeReferenceState, EntityState, EntitySupportingBlock, EntityTransform,
    EntityTransformState, EntityTransientEvents, EntityVec3, EvokerFangsCritParticleState,
    FallingBlockModelState, FireworkRocketExplosionParticleState, FireworkRocketItemState,
    FishingHookRenderSourceState, FoxEatParticleState, GuardianAmbientParticleState,
    GuardianAttackParticleState, ItemEntityStackState, ItemFrameRenderState,
    LightningBoltRenderSourceState, LivingDeathWakeTransition, LocalPlayerAttackSwingState,
    MinecartDisplayBlockState, OminousItemSpawnerItemState, OminousItemSpawnerParticleState,
    ParrotModelVariant, PowderSnowFlakeParticleState, RavagerRoarParticleState,
    RavagerStunParticleState, SlimeLandingParticleState, VanillaEntityBlockEffectTickPolicy,
    WolfArmorCrackiness, VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
    VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH, VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
    VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
    VANILLA_ENTITY_TYPE_END_CRYSTAL_ID, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
    VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
    VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
    VANILLA_ENTITY_TYPE_PANDA_ID, VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID,
    VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
    VANILLA_ENTITY_TYPE_TNT_MINECART_ID, VANILLA_ENTITY_TYPE_WOLF_ID, VANILLA_MAX_AIR_SUPPLY_TICKS,
};
use crate::entities::animations::{
    allay_is_dancing, axolotl_is_playing_dead, bat_is_resting, boat_bubble_time,
    boat_paddle_states, camel_is_dashing, cat_is_lying, cat_relax_state_one,
    copper_golem_animation_flags, creaking_can_move, creaking_is_tearing_down,
    entity_animation_uses_in_water, entity_chunk_coordinate, entity_is_fall_flying,
    entity_is_swimming, guardian_attack_duration, guardian_attack_target_id, guardian_is_moving,
    piglin_is_charging_crossbow, pillager_is_charging_crossbow, player_is_using_item,
    sniffer_can_play_digging_sound, warden_heartbeat_delay, wither_side_head_target_ids,
    wither_side_head_target_rotation, wolf_is_interested, EntityClientTickParticleTriggers,
    GuardianAttackTargetTickInput, RiddenJumpState, WitherHeadTargetRotations,
};
use crate::entities::dimensions::{
    entity_data_pose, item_frame_facing, item_frame_item, item_frame_map_id, item_frame_rotation,
    vanilla_age_scale, vanilla_body_anchor_y_offset_for_entity_data,
    vanilla_bounds_for_entity_data, vanilla_camel_leash_offset_for_entity_data,
    vanilla_client_position_for_entity_data, vanilla_eye_height_for_entity_data, vanilla_is_baby,
    vanilla_is_cat, vanilla_model_source_bounds_for_entity_data,
    vanilla_passenger_position_for_entity_data,
    vanilla_passenger_vehicle_debug_target_for_entity_data, vanilla_pick_bounds_for_entity_data,
    vanilla_pick_bounds_for_type, vanilla_render_scale, vanilla_shulker_attached_position,
};
use crate::entities::dragon::{
    ender_dragon_parent_pick_target, ender_dragon_part_parent_id,
    ender_dragon_part_pick_targets_at_partial_tick,
};
use crate::entities::entity_data::{entity_data_bool, entity_data_int, entity_data_long};
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
    vanilla_entity_metadata_sniffer_state_accessor_for_entity_type,
    vanilla_entity_metadata_type_is_abstract_piglin,
    vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer,
    VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION, VANILLA_AVATAR_MAIN_HAND_DATA_ID,
    VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
    VANILLA_AVATAR_MODEL_PART_CAPE as VANILLA_AVATAR_CAPE_PART_MASK, VANILLA_HUMANOID_ARM_LEFT_ID,
    VANILLA_HUMANOID_ARM_RIGHT_ID, VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID,
    VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID, VANILLA_SNIFFER_STATE_IDLING_ID,
};
use crate::registries::{armor_trim_sprite, ArmorTrimLayer, BlockStateInfo, RegistrySet};
use crate::LocalPlayerPoseState;
use bbb_protocol::entity_types::*;

mod block_defaults;
mod equipment;
mod render_helpers;
mod tick_triggers;
use block_defaults::*;
use equipment::*;
use render_helpers::*;
use tick_triggers::*;

mod animation_sync;
mod beams_and_camera;
mod block_effect_ticks;
mod block_states_and_particles;
mod bounds_and_candidates;
mod client_animation_tick;
mod client_tick_particles;
mod component_access;
mod local_player;
pub(crate) use local_player::LocalPlayerOwnedData;
mod metadata_queries;
mod minecarts_and_steering;
mod model_source;
mod mounts_leashes_items;
mod render_positions;
mod tick_inputs;

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

fn upsert_metadata_value(metadata: &mut EntityMetadata, value: ProtocolEntityDataValue) {
    if metadata.data_values.contains(&value) {
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

/// Lazily built section-level spatial index for tracked entities, shared via
/// `Arc` across immutable `WorldStore` publication snapshots and invalidated on
/// any mutable access to `EntityStore`.
#[derive(Debug, Default)]
pub(super) struct EntityChunkSpatialIndex {
    pub(super) sections: FxHashMap<(i32, i32, i32), Vec<usize>>,
    /// Slots whose bounds cannot be safely restricted to <= 4 sections per axis
    /// (e.g. EnderDragon multi-part boxes, missing canonical bounds, huge boxes,
    /// or non-finite coordinates). Always visited alongside section matches.
    pub(super) oversized_slots: Vec<usize>,
}

/// Tracked client entities in Vanilla `ClientLevel` insertion order
/// (`entitiesById` + `EntityTickList`): a re-add of a live id keeps its slot,
/// removal leaves a hole that compaction closes without reordering survivors.
pub(crate) struct EntityStore {
    slots: Vec<Option<EntityRecord>>,
    slot_by_id: FxHashMap<i32, usize>,
    pub(super) protocol_id_by_uuid: FxHashMap<Uuid, i32>,
    pub(super) collidable_entity_count: usize,
    pub(super) pushable_entity_count: usize,
    next_incarnation: u64,
    pending_ravager_stun_particles: Vec<RavagerStunParticleState>,
    pending_evoker_fangs_crit_particles: Vec<EvokerFangsCritParticleState>,
    pub(super) pending_entity_client_movement_effects: Vec<EntityClientMovementEffectState>,
    pub(super) pending_client_particle_spawns: Vec<crate::particles::ParticleSpawn>,
    pub(super) pending_local_discards: Vec<i32>,
    pending_ominous_item_spawner_particles: Vec<OminousItemSpawnerParticleState>,
    client_tick_scratch: EntityClientTickScratch,
    publication_snapshot_observation: EntityPublicationSnapshotObservation,
    spatial_index: OnceLock<Arc<EntityChunkSpatialIndex>>,
    pub(crate) content_registry: Arc<ContentRegistry>,
}

#[derive(Default)]
struct EntityClientTickScratch {
    scheduled_set: FxHashSet<i32>,
    scheduled_ids: Vec<i32>,
    passenger_dfs_stack: Vec<i32>,
    player_passenger_ancestors: FxHashSet<i32>,
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
    pub(crate) honey_block_sliding: bool,
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
        self.scheduled_ids.clear();
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
    pub(crate) position: EntityVec3,
    pub(crate) relative_min: [f64; 3],
    pub(crate) relative_max: [f64; 3],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EntityLocalPlayerPushTickInput {
    pub(crate) entity_id: i32,
    pub(crate) entity_type_id: i32,
    pub(crate) uuid: Uuid,
    pub(crate) position: EntityVec3,
}

#[derive(Debug, Default)]
pub(crate) struct EntityClientTickScheduleInputs {
    pub(crate) boats: Vec<EntityBoatClientTickInput>,
    pub(crate) local_player_pushers: Vec<EntityLocalPlayerPushTickInput>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityBoatClientTickWorldFacts {
    pub(crate) entity_id: i32,
    pub(crate) status: super::EntityBoatStatus,
    pub(crate) water_level: f64,
    pub(crate) land_friction: Option<f32>,
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

impl EntityStore {
    pub(super) fn invalidate_spatial_index(&mut self) {
        self.spatial_index.take();
    }

    pub(crate) fn set_content_registry(&mut self, registry: Arc<ContentRegistry>) {
        self.spatial_index.take();
        self.content_registry = registry;
    }

    pub(crate) fn record(&self, id: i32) -> Option<&EntityRecord> {
        self.slots[*self.slot_by_id.get(&id)?].as_ref()
    }

    pub(super) fn record_mut(&mut self, id: i32) -> Option<&mut EntityRecord> {
        let &slot = self.slot_by_id.get(&id)?;
        self.spatial_index.take();
        self.slots[slot].as_mut()
    }

    /// Records in insertion order.
    pub(crate) fn records(&self) -> impl Iterator<Item = &EntityRecord> + '_ {
        self.slots.iter().flatten()
    }

    pub(super) fn records_mut(&mut self) -> impl Iterator<Item = &mut EntityRecord> + '_ {
        self.spatial_index.take();
        self.slots.iter_mut().flatten()
    }

    /// Ids in insertion order.
    pub(super) fn ids(&self) -> impl Iterator<Item = i32> + '_ {
        self.records().map(|record| record.identity.id)
    }

    fn record_collision_flags(entity_type_id: i32) -> (bool, bool) {
        let collidable = !matches!(
            vanilla_entity_metadata_type_collidability_kind(entity_type_id),
            None | Some(VanillaEntityCollidabilityKind::Never)
        );
        let pushable = !matches!(
            vanilla_entity_metadata_type_pushability_kind(entity_type_id),
            None | Some(VanillaEntityPushabilityKind::Never)
        );
        (collidable, pushable)
    }

    fn adjust_collision_counts(&mut self, entity_type_id: i32, added: bool) {
        let (collidable, pushable) = Self::record_collision_flags(entity_type_id);
        if collidable {
            self.collidable_entity_count = if added {
                self.collidable_entity_count.saturating_add(1)
            } else {
                self.collidable_entity_count.saturating_sub(1)
            };
        }
        if pushable {
            self.pushable_entity_count = if added {
                self.pushable_entity_count.saturating_add(1)
            } else {
                self.pushable_entity_count.saturating_sub(1)
            };
        }
    }

    /// Inserts a new id at the end of the insertion order, or replaces a live
    /// id's record in place.
    fn put_record(&mut self, record: EntityRecord) {
        self.spatial_index.take();
        let id = record.identity.id;
        let uuid = record.identity.uuid;
        let entity_type_id = record.identity.entity_type_id;
        if let Some(&slot) = self.slot_by_id.get(&id) {
            if let Some(previous) = self.slots[slot].replace(record) {
                self.adjust_collision_counts(previous.identity.entity_type_id, false);
                if previous.identity.uuid != uuid
                    && self.protocol_id_by_uuid.get(&previous.identity.uuid) == Some(&id)
                {
                    self.protocol_id_by_uuid.remove(&previous.identity.uuid);
                }
            }
        } else {
            self.slot_by_id.insert(id, self.slots.len());
            self.slots.push(Some(record));
        }
        self.adjust_collision_counts(entity_type_id, true);
        self.protocol_id_by_uuid.insert(uuid, id);
    }

    fn spawn_owner(&self, owner_id: i32) -> EntitySpawnOwner {
        let owner = self.record(owner_id);
        EntitySpawnOwner {
            uuid: owner.map(|owner| owner.identity.uuid),
            is_player: owner.is_some_and(|owner| {
                owner.identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            }),
        }
    }

    /// Spawns (or re-add-resets) an entity from `AddEntity` packet values; a
    /// re-add of a live id resets every client-owned value.
    pub(crate) fn apply_add_entity_components(
        &mut self,
        mut identity: EntityIdentity,
        transform: EntityTransform,
    ) {
        identity.incarnation = self.allocate_incarnation();
        let owner = self.spawn_owner(identity.data);
        self.put_record(EntityRecord::spawned_with_registry(
            identity,
            transform,
            owner,
            &self.content_registry,
        ));
    }

    #[cfg(test)]
    pub(crate) fn block_effects_debug_state(
        &self,
        id: i32,
    ) -> Option<super::EntityBlockEffectsDebugState> {
        let record = self.record(id)?;
        Some(
            record.supporting_block.block_effects.debug_state(
                record
                    .family
                    .collision_facts()
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
        let record = self.record(id)?;
        let other = other.record(id)?;
        Some(EntityPublicationPayloadSharing {
            data_values: record
                .metadata
                .data_values
                .ptr_eq(&other.metadata.data_values),
            painting_variant: record
                .metadata
                .painting_variant_projection
                .ptr_eq(&other.metadata.painting_variant_projection),
            equipment: record
                .equipment
                .equipment
                .ptr_eq(&other.equipment.equipment),
            attributes: record
                .attributes
                .attributes
                .ptr_eq(&other.attributes.attributes),
            passengers: record.mount.passengers.ptr_eq(&other.mount.passengers),
            effects: record.effects.effects.ptr_eq(&other.effects.effects),
            damage: record.damage.last_damage.ptr_eq(&other.damage.last_damage),
        })
    }

    pub(crate) fn insert_or_replace(&mut self, state: EntityState) {
        let incarnation = self.allocate_incarnation();
        let owner = self.spawn_owner(state.data);
        self.put_record(EntityRecord::from_state_with_registry(
            &state,
            incarnation,
            owner,
            &self.content_registry,
        ));
        self.resolve_abstract_arrow_runtime_references();
    }

    pub(crate) fn get(&self, id: i32) -> Option<EntityState> {
        self.record(id).map(|record| self.project_entity(record))
    }

    pub(crate) fn contains(&self, id: i32) -> bool {
        self.slot_by_id.contains_key(&id)
    }

    pub(crate) fn entity_type_id(&self, id: i32) -> Option<i32> {
        self.record(id).map(|record| record.identity.entity_type_id)
    }

    pub(crate) fn uuid_for_protocol_id(&self, id: i32) -> Option<Uuid> {
        self.record(id).map(|record| record.identity.uuid)
    }

    /// Player-type rows for the `ClientLevel` "All players" report detail:
    /// protocol id, profile uuid, and current position in insertion order.
    pub(crate) fn player_debug_rows(&self) -> Vec<(i32, Uuid, EntityVec3)> {
        self.records()
            .filter(|record| record.identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID)
            .map(|record| {
                (
                    record.identity.id,
                    record.identity.uuid,
                    record.transform.position,
                )
            })
            .collect()
    }

    fn strider_suffocating_for_client_tick(
        &self,
        id: i32,
        fluid_facts_by_id: &FxHashMap<i32, EntityClientTickFluidFacts>,
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

    pub(crate) fn is_silent(&self, id: i32) -> Option<bool> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        self.metadata_bool(id, accessors.silent_data_id, accessors.silent_default)
    }

    pub(crate) fn creaking_is_heart_bound(&self, id: i32) -> Option<bool> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let data_id = vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type(
            identity.entity_type_id,
        )?;
        let metadata = &entity.metadata;
        Some(metadata.data_values.iter().any(|value| {
            value.data_id == data_id
                && matches!(&value.value, EntityDataValueKind::OptionalBlockPos(Some(_)))
        }))
    }

    /// Vanilla `LivingEntity.getHealth()`: the synced `DATA_HEALTH_ID` float,
    /// defaulting to the define-time `1.0F` (LivingEntity.java:314) while the
    /// server has not synced the spawn-time health.
    pub(crate) fn living_entity_health(&self, id: i32) -> Option<f32> {
        let entity_type_id = self.entity_type_id(id)?;
        let accessors =
            super::content_facts::entity_living_accessors(entity_type_id, &self.content_registry)?;
        self.metadata_float(id, accessors.health_data_id, accessors.health_default)
    }

    /// Applies Vanilla `LivingEntity.heal`/`setHealth` to canonical client state and reports
    /// whether the synchronized health float changed. Server metadata remains authoritative.
    pub(crate) fn apply_local_living_heal(&mut self, id: i32, amount: f32) -> Option<bool> {
        let entity_type_id = self.entity_type_id(id)?;
        let accessors =
            super::content_facts::entity_living_accessors(entity_type_id, &self.content_registry)?;
        let health = self.metadata_float(id, accessors.health_data_id, accessors.health_default)?;
        if health <= 0.0 {
            return Some(false);
        }
        let max_health = self.attribute_value(id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)? as f32;
        let healed = (health + amount).min(max_health).max(0.0);
        if healed == health {
            return Some(false);
        }
        let metadata = &mut self.record_mut(id)?.metadata;
        upsert_metadata_value(
            metadata,
            ProtocolEntityDataValue {
                data_id: accessors.health_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(healed),
            },
        );
        Some(true)
    }

    /// Applies `AbstractVillager.setUnhappyCounter` to synchronized client
    /// metadata. Vanilla's baby-villager interaction writes 40 on both sides;
    /// later server metadata remains authoritative through the same storage.
    pub(crate) fn set_local_abstract_villager_unhappy_counter(
        &mut self,
        id: i32,
        value: i32,
    ) -> Option<()> {
        let entity = self.record_mut(id)?;
        let entity_type_id = entity.identity.entity_type_id;
        let (data_id, _) =
            vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type(
                entity_type_id,
            )?;
        let metadata = &mut entity.metadata;
        upsert_metadata_value(
            metadata,
            ProtocolEntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(value),
            },
        );
        Some(())
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
        if vanilla_entity_renderer_uses_mob_item_in_hand_layer(entity_type_id)
            || super::content_facts::mod_entity_spec(entity_type_id, &self.content_registry)
                .is_some_and(|spec| spec.is_mob)
        {
            let Some(accessors) =
                super::content_facts::entity_mob_accessors(entity_type_id, &self.content_registry)
            else {
                return false;
            };
            return self
                .metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                .is_some_and(|flags| flags & accessors.left_handed_mask != 0);
        }
        false
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
            state
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
        let entity = self.record(id)?;
        let identity = &entity.identity;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID {
            return None;
        }
        let state = registries.block_state(identity.data)?;
        if state.name == "minecraft:air" {
            return None;
        }
        let transform = &entity.transform;
        let metadata = &entity.metadata;
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
            block: state,
            random_seed_pos,
            block_pos,
        })
    }

    pub(crate) fn primed_tnt_block_state(
        &self,
        id: i32,
        registries: &RegistrySet,
    ) -> Option<&'static BlockStateInfo> {
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
            return Some(state);
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
        for entity in self.records() {
            let (identity, transform) = (&entity.identity, &entity.transform);
            let Some(accessors) = vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(
                identity.entity_type_id,
            ) else {
                continue;
            };
            let fuse = entity_data_int(
                &entity.metadata.data_values,
                accessors.fuse_data_id,
                accessors.fuse_default,
            );
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

    pub(crate) fn item_stack_for_entity(&self, id: i32) -> Option<&ItemStackSummary> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let data_id = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
            identity.entity_type_id,
        )?;
        let metadata = &entity.metadata;
        metadata_item_stack(&metadata.data_values, data_id)
            .filter(|stack| stack.item_id.is_some() && stack.count > 0)
    }

    pub(crate) fn raw_item_stack_for_entity(&self, id: i32) -> Option<&ItemStackSummary> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let accessor = vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer(
            identity.entity_type_id,
            "ITEM_STACK",
        )?;
        let metadata = &entity.metadata;
        metadata_item_stack(&metadata.data_values, accessor.data_id)
    }

    pub(crate) fn experience_orb_icon_for_entity(&self, id: i32) -> Option<i32> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let (data_id, default) =
            vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type(
                identity.entity_type_id,
            )?;
        let value = entity
            .metadata
            .data_values
            .iter()
            .find(|value| value.data_id == data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::Int(value) => Some(*value),
                _ => None,
            })
            .unwrap_or(default);
        Some(super::experience_orb_icon(value))
    }

    /// The render state of every item-frame / glow-item-frame entity: its resolved wall center, the
    /// facing wall, the `0..=7` item rotation, glow/invisible flags, the framed item, and framed
    /// map id. Drives the 3D item-frame render (vanilla `ItemFrameRenderer`).
    pub(crate) fn item_frame_render_states(
        &self,
    ) -> impl Iterator<Item = ItemFrameRenderState<'_>> {
        self.records()
            .filter_map(|entity| self.item_frame_render_state_for_record(entity))
    }

    pub(crate) fn item_frame_render_state(&self, id: i32) -> Option<ItemFrameRenderState<'_>> {
        let entity = self.record(id)?;
        self.item_frame_render_state_for_record(entity)
    }

    fn item_frame_render_state_for_record<'a>(
        &self,
        entity: &'a EntityRecord,
    ) -> Option<ItemFrameRenderState<'a>> {
        let entity_accessors = vanilla_entity_metadata_entity_accessors()
            .expect("generated base Entity metadata accessors must resolve");
        let identity = &entity.identity;
        let accessors =
            vanilla_entity_metadata_item_frame_accessors_for_entity_type(identity.entity_type_id)?;
        let transform = &entity.transform;
        let metadata = &entity.metadata;
        Some(ItemFrameRenderState {
            entity_id: identity.id,
            center: transform.position,
            facing: item_frame_facing(accessors, identity.data, &metadata.data_values),
            light: super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
            rotation: item_frame_rotation(accessors, &metadata.data_values),
            glow: vanilla_entity_renderer_minimum_block_light(identity.entity_type_id).is_some(),
            invisible: entity_data_byte(
                &metadata.data_values,
                entity_accessors.shared_flags_data_id,
                entity_accessors.shared_flags_default,
            ) & entity_accessors.invisible_mask
                != 0,
            item: item_frame_item(accessors, &metadata.data_values),
            map_id: item_frame_map_id(accessors, &metadata.data_values),
        })
    }

    /// The item a humanoid entity holds in its main (`off_hand = false`) or off hand (vanilla
    /// `EntityEquipment` `MainHand`/`OffHand` slot), or `None` for an empty hand / no equipment.
    /// Drives the third-person held-item render.
    pub(crate) fn held_item(&self, id: i32, off_hand: bool) -> Option<&ItemStackSummary> {
        let slot = if off_hand {
            ProtocolEquipmentSlot::OffHand
        } else {
            ProtocolEquipmentSlot::MainHand
        };
        self.equipment_item(id, slot)
    }

    /// Collects the firework rocket item render state. Vanilla
    /// `FireworkRocketEntity.DATA_ID_FIREWORKS_ITEM` uses item-stack metadata id 8, followed by
    /// `DATA_ATTACHED_TO_TARGET` (id 9) and `DATA_SHOT_AT_ANGLE` (id 10).
    pub(crate) fn firework_rocket_item_states(
        &self,
    ) -> impl Iterator<Item = FireworkRocketItemState<'_>> {
        self.records()
            .filter_map(|entity| self.firework_rocket_item_state_for_record(entity))
    }

    pub(crate) fn firework_rocket_item_state(
        &self,
        id: i32,
    ) -> Option<FireworkRocketItemState<'_>> {
        let entity = self.record(id)?;
        self.firework_rocket_item_state_for_record(entity)
    }

    fn firework_rocket_item_state_for_record<'a>(
        &self,
        entity: &'a EntityRecord,
    ) -> Option<FireworkRocketItemState<'a>> {
        let identity = &entity.identity;
        let accessors = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
            identity.entity_type_id,
        )?;
        let transform = &entity.transform;
        let metadata = &entity.metadata;
        if firework_rocket_attached_to_entity(&metadata.data_values, accessors) {
            return None;
        }
        let stack = firework_rocket_render_stack(&metadata.data_values, accessors)?;
        Some(FireworkRocketItemState {
            entity_id: identity.id,
            position: transform.position,
            light: super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
            stack,
            shot_at_angle: firework_rocket_shot_at_angle(&metadata.data_values, accessors),
        })
    }

    /// Collects the ominous item spawner item-cluster render state. The generated
    /// `OminousItemSpawner.DATA_ITEM` accessor owns membership and the item-stack wire index.
    pub(crate) fn ominous_item_spawner_item_states_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> impl Iterator<Item = OminousItemSpawnerItemState<'_>> {
        self.records().filter_map(move |entity| {
            let identity = &entity.identity;
            let transform = &entity.transform;
            let metadata = &entity.metadata;
            let client_animations = &entity.animations;
            let stack =
                ominous_item_spawner_render_stack(identity.entity_type_id, &metadata.data_values)?;
            Some(OminousItemSpawnerItemState {
                entity_id: identity.id,
                position: transform.position,
                age_ticks: client_animations.age_ticks as f32 + partial_ticks,
                stack,
            })
        })
    }

    pub(crate) fn append_ominous_item_spawner_particle_states(&mut self) {
        let spawners = self
            .records()
            .filter(|entity| {
                entity.identity.entity_type_id == VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID
            })
            .map(|entity| OminousItemSpawnerParticleState {
                entity_id: entity.identity.id,
                position: entity.transform.position,
            })
            .collect::<Vec<_>>();
        self.pending_ominous_item_spawner_particles.extend(spawners);
    }

    /// Apply the client-observable body of Vanilla
    /// `LivingEntity.handleDamageEvent` in declaration order. Admission has
    /// already proved this is a living entity.
    pub(crate) fn apply_living_damage_event_presentation(
        &mut self,
        id: i32,
        event: EntityDamageEventState,
    ) -> Option<()> {
        let entity = self.record_mut(id)?;
        entity.animations.set_damage_event_walk_speed();
        entity.damage.invulnerable_time = 20;
        entity.animations.trigger_hurt();
        *entity.damage.last_damage = Some(event);
        Some(())
    }

    /// Applies `Player.stopSleepInBed(false, false)` for animate action 2.
    ///
    /// The caller resolves the loaded-bed stand-up transition before this
    /// mutation. Standing pose, sleeping-position removal, and counter 100 are
    /// unconditional for a known Player, even when no valid bed was resolved.
    pub(crate) fn apply_client_player_wake_animation(
        &mut self,
        id: i32,
        wake: Option<LivingDeathWakeTransition>,
    ) -> Option<()> {
        let entity = self.record_mut(id)?;
        if entity.identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
            return None;
        }
        let living_accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            entity.identity.entity_type_id,
        )?;
        if let Some(wake) = wake {
            let transform = &mut entity.transform;
            transform.position = wake.position;
            transform.set_y_rot(wake.y_rot);
            transform.set_x_rot(0.0);
        }

        let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
        let metadata = &mut entity.metadata;
        upsert_metadata_value(
            metadata,
            ProtocolEntityDataValue {
                data_id: entity_accessors.pose_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
                value: EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_STANDING_ID),
            },
        );
        upsert_metadata_value(
            metadata,
            ProtocolEntityDataValue {
                data_id: living_accessors.sleeping_pos_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
                value: EntityDataValueKind::OptionalBlockPos(None),
            },
        );
        *entity.family.player_sleep_counter_mut()? = 100;
        Some(())
    }

    pub(crate) fn apply_client_animation_hurt(&mut self, id: i32) -> Option<()> {
        let content_registry = Arc::clone(&self.content_registry);
        let entity = self.record_mut(id)?;
        let entity_type_id = entity.identity.entity_type_id;
        if super::content_facts::entity_metadata_is_living(entity_type_id, &content_registry) {
            entity.animations.trigger_hurt();
            Some(())
        } else if is_vanilla_vehicle_entity_type(entity_type_id) {
            entity.animations.trigger_vehicle_hurt();
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
        let content_registry = Arc::clone(&self.content_registry);
        let entity = self.record_mut(id)?;
        if super::content_facts::entity_metadata_is_living(
            entity.identity.entity_type_id,
            &content_registry,
        ) {
            entity
                .animations
                .start_head_rotation_lerp(target_y_head_rot, steps);
        } else {
            entity.transform.y_head_rot = target_y_head_rot;
        }
        Some(())
    }

    pub(crate) fn publication_snapshot_observation(&self) -> EntityPublicationSnapshotObservation {
        self.publication_snapshot_observation
    }

    pub(crate) fn for_each_mount_mut(&mut self, mut update: impl FnMut(i32, &mut EntityMount)) {
        for record in self.records_mut() {
            update(record.identity.id, &mut record.mount);
        }
    }

    pub(crate) fn for_each_leash_mut(&mut self, mut update: impl FnMut(i32, &mut EntityLeash)) {
        for record in self.records_mut() {
            update(record.identity.id, &mut record.leash);
        }
    }

    pub(crate) fn states(&self) -> Vec<EntityState> {
        self.records()
            .map(|record| self.project_entity(record))
            .collect()
    }

    pub(crate) fn total_mob_effects(&self) -> usize {
        self.records()
            .map(|record| record.effects.effects.len())
            .sum()
    }

    pub(crate) fn total_minecart_lerp_steps(&self) -> usize {
        self.records()
            .filter_map(|record| record.family.minecart_lerp())
            .map(|lerp| lerp.pending_steps.len() + lerp.steps.len())
            .sum()
    }

    pub(crate) fn take_pending_client_particle_spawns(
        &mut self,
    ) -> Vec<crate::particles::ParticleSpawn> {
        std::mem::take(&mut self.pending_client_particle_spawns)
    }

    pub(crate) fn len(&self) -> usize {
        self.slot_by_id.len()
    }

    pub(crate) fn clear(&mut self) {
        // Preserve session-scoped `ContentRegistry` across `Login` and cross-dimension `Respawn`.
        let content_registry = Arc::clone(&self.content_registry);
        *self = Self::default();
        self.content_registry = content_registry;
    }

    /// Removes one record and hands it to the caller.
    pub(crate) fn take_record(&mut self, id: i32) -> Option<EntityRecord> {
        let slot = self.slot_by_id.remove(&id)?;
        self.spatial_index.take();
        let record = self.slots[slot].take()?;
        self.adjust_collision_counts(record.identity.entity_type_id, false);
        if self.protocol_id_by_uuid.get(&record.identity.uuid) == Some(&id) {
            self.protocol_id_by_uuid.remove(&record.identity.uuid);
        }
        // Compacting once holes outnumber live records keeps removal amortised
        // O(1) without reordering the survivors.
        if self.slots.len() > 2 * self.slot_by_id.len() + 16 {
            self.slots.retain(Option::is_some);
            for (slot, record) in self.slots.iter().enumerate() {
                if let Some(record) = record {
                    self.slot_by_id.insert(record.identity.id, slot);
                }
            }
        }
        Some(record)
    }

    pub(crate) fn remove_ids(&mut self, ids: &[i32]) -> usize {
        ids.iter()
            .filter(|&&id| self.take_record(id).is_some())
            .count()
    }

    fn project_entity(&self, record: &EntityRecord) -> EntityState {
        record.to_state(
            record
                .family
                .abstract_arrow()
                .map(|runtime| self.project_abstract_arrow_runtime(runtime)),
        )
    }

    pub(super) fn resolve_abstract_arrow_runtime_references(&mut self) {
        let identities = self
            .records()
            .map(|record| {
                (
                    (record.identity.id, record.identity.uuid),
                    record.identity.incarnation,
                )
            })
            .collect::<FxHashMap<_, _>>();
        for runtime in self
            .records_mut()
            .filter_map(|record| record.family.abstract_arrow_mut())
        {
            let Some(reference) = runtime.pending_last_deflected_by else {
                continue;
            };
            let Some(&incarnation) = identities.get(&(reference.entity_id, reference.uuid)) else {
                continue;
            };
            runtime.last_deflected_by = Some(EntityInstanceReference {
                entity_id: reference.entity_id,
                incarnation,
            });
            runtime.pending_last_deflected_by = None;
        }
    }

    fn project_abstract_arrow_runtime(
        &self,
        runtime: &EntityAbstractArrowRuntime,
    ) -> AbstractArrowRuntimeState {
        let last_deflected_by = runtime
            .last_deflected_by
            .and_then(|reference| {
                let identity = &self.record(reference.entity_id)?.identity;
                (identity.incarnation == reference.incarnation).then_some(
                    EntityRuntimeReferenceState {
                        entity_id: identity.id,
                        uuid: identity.uuid,
                    },
                )
            })
            .or(runtime.pending_last_deflected_by);
        AbstractArrowRuntimeState {
            random_seed: runtime.random_seed,
            last_deflected_by,
            piercing_ignore_entity_ids: runtime.piercing_ignore_entity_ids.clone(),
        }
    }
}

impl Default for EntityStore {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            slot_by_id: FxHashMap::default(),
            protocol_id_by_uuid: FxHashMap::default(),
            collidable_entity_count: 0,
            pushable_entity_count: 0,
            next_incarnation: 1,
            pending_ravager_stun_particles: Vec::new(),
            pending_evoker_fangs_crit_particles: Vec::new(),
            pending_entity_client_movement_effects: Vec::new(),
            pending_client_particle_spawns: Vec::new(),
            pending_local_discards: Vec::new(),
            pending_ominous_item_spawner_particles: Vec::new(),
            client_tick_scratch: EntityClientTickScratch::default(),
            publication_snapshot_observation: EntityPublicationSnapshotObservation::default(),
            spatial_index: OnceLock::new(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
        }
    }
}

impl Clone for EntityStore {
    fn clone(&self) -> Self {
        let snapshot_started_at = Instant::now();
        let slots = self.slots.clone();
        let rebuilt_entities = self.slot_by_id.len();
        Self {
            slots,
            slot_by_id: self.slot_by_id.clone(),
            protocol_id_by_uuid: self.protocol_id_by_uuid.clone(),
            collidable_entity_count: self.collidable_entity_count,
            pushable_entity_count: self.pushable_entity_count,
            next_incarnation: self.next_incarnation,
            pending_ravager_stun_particles: self.pending_ravager_stun_particles.clone(),
            pending_evoker_fangs_crit_particles: self.pending_evoker_fangs_crit_particles.clone(),
            pending_entity_client_movement_effects: self
                .pending_entity_client_movement_effects
                .clone(),
            pending_client_particle_spawns: Vec::new(),
            pending_local_discards: Vec::new(),
            pending_ominous_item_spawner_particles: self
                .pending_ominous_item_spawner_particles
                .clone(),
            client_tick_scratch: EntityClientTickScratch::default(),
            publication_snapshot_observation: EntityPublicationSnapshotObservation {
                duration: snapshot_started_at.elapsed(),
                rebuilt_entities,
            },
            spatial_index: self.spatial_index.clone(),
            content_registry: Arc::clone(&self.content_registry),
        }
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

fn apply_boat_push_to_local_player(
    boat_transform: &mut EntityTransform,
    boat_mount: &EntityMount,
    facts: &EntityBoatClientTickWorldFacts,
    pusher_is_pushable: Option<bool>,
    local_player_pose: &mut LocalPlayerPoseState,
    local_player_dimensions: crate::LocalPlayerDimensions,
    local_player_is_vehicle: bool,
) {
    let Some(pusher_is_pushable) = pusher_is_pushable else {
        return;
    };

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
    let (player_min, player_max) = local_player_dimensions.aabb_at(local_player_pose.position);
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

    apply_local_player_push_impulse(
        boat_transform,
        !boat_mount.passengers.is_empty(),
        pusher_is_pushable,
        local_player_pose,
        local_player_is_vehicle,
    );
}

fn apply_living_push_to_local_player(
    pusher_transform: &mut EntityTransform,
    pusher_mount: &EntityMount,
    pusher_bounds: super::EntityPickBoundsState,
    pusher_is_pushable: bool,
    local_player_pose: &mut LocalPlayerPoseState,
    local_player_dimensions: crate::LocalPlayerDimensions,
    local_player_is_vehicle: bool,
) {
    let pusher_min = [
        pusher_transform.position.x + f64::from(pusher_bounds.min[0]),
        pusher_transform.position.y + f64::from(pusher_bounds.min[1]),
        pusher_transform.position.z + f64::from(pusher_bounds.min[2]),
    ];
    let pusher_max = [
        pusher_transform.position.x + f64::from(pusher_bounds.max[0]),
        pusher_transform.position.y + f64::from(pusher_bounds.max[1]),
        pusher_transform.position.z + f64::from(pusher_bounds.max[2]),
    ];
    let (player_min, player_max) = local_player_dimensions.aabb_at(local_player_pose.position);
    if !strict_aabb_intersection(player_min, player_max, pusher_min, pusher_max) {
        return;
    }
    apply_local_player_push_impulse(
        pusher_transform,
        !pusher_mount.passengers.is_empty(),
        pusher_is_pushable,
        local_player_pose,
        local_player_is_vehicle,
    );
}

fn strict_aabb_intersection(
    first_min: [f64; 3],
    first_max: [f64; 3],
    second_min: [f64; 3],
    second_max: [f64; 3],
) -> bool {
    first_min[0] < second_max[0]
        && first_max[0] > second_min[0]
        && first_min[1] < second_max[1]
        && first_max[1] > second_min[1]
        && first_min[2] < second_max[2]
        && first_max[2] > second_min[2]
}

fn apply_local_player_push_impulse(
    pusher_transform: &mut EntityTransform,
    pusher_is_vehicle: bool,
    pusher_is_pushable: bool,
    local_player_pose: &mut LocalPlayerPoseState,
    local_player_is_vehicle: bool,
) {
    let mut xa = local_player_pose.position.x - pusher_transform.position.x;
    let mut za = local_player_pose.position.z - pusher_transform.position.z;
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

    if !pusher_is_vehicle && pusher_is_pushable {
        pusher_transform.delta_movement.x -= xa;
        pusher_transform.delta_movement.z -= za;
    }
    if !local_player_is_vehicle {
        local_player_pose.delta_movement.x += xa;
        local_player_pose.delta_movement.z += za;
    }
}

/// Exact Vanilla `DefaultAttributes` supplier value, used until a
/// `ClientboundUpdateAttributesPacket` overrides it. Membership and family
/// overrides come from the generated EntityType profile; the Attribute
/// registry default is consulted only for the one-argument builder overload.
fn vanilla_default_attribute_base(entity_type_id: i32, attribute_id: i32) -> Option<f64> {
    let (_, base_override) = vanilla_entity_type_default_attribute(entity_type_id, attribute_id)?;
    base_override.or_else(|| {
        vanilla_attribute_type(attribute_id)
            .map(|attribute| f64::from_bits(attribute.default_value_bits))
    })
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
