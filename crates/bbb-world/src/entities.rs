use std::collections::BTreeMap;

use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID;
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID;
use bbb_protocol::packets::{
    AttributeSnapshot as ProtocolAttributeSnapshot, EntityDataValue as ProtocolEntityDataValue,
    EntityDataValueKind, EquipmentSlot as ProtocolEquipmentSlot,
    EquipmentSlotUpdate as ProtocolEquipmentSlotUpdate,
    FireworkExplosionSummary as ProtocolFireworkExplosionSummary,
    ItemStackSummary as ProtocolItemStackSummary, MinecartStep as ProtocolMinecartStep,
    RemoveEntities as ProtocolRemoveEntities, TakeItemEntity as ProtocolTakeItemEntity,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::terrain::{
    block_name_has_invisible_render_shape, block_name_is_bed, block_name_is_rail,
};
use crate::{
    ArmorTrimSprite, BlockPos, ChunkPos, PaintingVariantClientProjection, TerrainLight, WorldStore,
};

mod animations;
mod block_effects;
mod client_movement;
mod components;
pub(crate) use components::EntityBoatStatus;
mod dimensions;
mod display;
mod dragon;
mod events;
mod leash;
mod metadata;
mod movement;
mod name_tags;
mod passengers;
mod projectiles;
mod renderer_catalog;
mod schema;
mod shadows;
mod sound_variants;
mod spawn;
pub(crate) mod state;
mod status;
mod store;
mod updates;

use animations::{cat_is_lying, guardian_attack_target_id};
pub(crate) use animations::{spawn_random_unit_float, ATTACK_SWING_DURATION};
pub use animations::{EntityClientAnimationState, PolarBearStandingAnimationState};
pub(crate) use bbb_protocol::entity_types::*;
pub(crate) use bbb_protocol::item_types::{VANILLA_ITEM_EGG_ID, VANILLA_ITEM_SNOWBALL_ID};
#[cfg(test)]
pub(crate) use block_effects::EntityBlockEffectsDebugState;
pub(crate) use block_effects::{
    EntityBlockEffectBaseTickReset, EntityBlockEffectEvaluation, EntityBlockEffectMovement,
    EntityBlockEffectMovementIter, EntityBlockEffectMovements, EntityBlockEffectQuery,
    EntityBlockEffects,
};
pub(crate) use client_movement::calculate_view_vector;
pub(crate) use components::{
    EntityAttributes, EntityClientAnimations, EntityDamage, EntityEquipment,
    EntityHurtingProjectile, EntityIdentity, EntityInterpolationCollisionInput, EntityLeash,
    EntityLivingLifecycle, EntityMetadata, EntityMinecartCommandBlock, EntityMinecartLerp,
    EntityMobEffects, EntityMount, EntityPlayerSleep, EntityProjectileLifecycle,
    EntitySupportingBlock, EntityTransform, EntityTransientEvents, LivingDeathWakeTransition,
};
pub(crate) use dimensions::vanilla_bounds_for_type;
pub use dimensions::EntityPickBoundsState;
pub(crate) use dimensions::{
    hanging_facing, prospective_item_frame_geometry, prospective_painting_geometry,
    vanilla_animal_entity_type, vanilla_living_entity_type, vanilla_pick_bounds_for_type,
};
pub use display::{
    DisplayBillboardConstraints, DisplayEntityCullingState, DisplayEntityRenderContent,
    DisplayEntityRenderSourceState, DisplayItemContext, DisplayTransformation,
};
pub use dragon::{DragonFlightHistorySample, DragonFlightHistoryState, EnderDragonAnimationState};
pub(crate) use events::{vanilla_entity_event_kind, VanillaEntityEventKind};
pub use leash::EntityLeashRenderState;
use movement::entity_vec3;
pub use name_tags::{
    EntityNameTagLivingVisibilityState, EntityNameTagRenderSourceState,
    EntityNameTagTextSegmentState, EntityNameTagTextState, EntityNameTagVisibilityPolicy,
};
pub use renderer_catalog::{
    vanilla_entity_renderer_boat_kind, vanilla_entity_renderer_camel_kind,
    vanilla_entity_renderer_culling_kind, vanilla_entity_renderer_extracts_parrot_pose,
    vanilla_entity_renderer_extracts_sheep_head_eat, vanilla_entity_renderer_extracts_wolf_sitting,
    vanilla_entity_renderer_extracts_zombie_underwater_converting,
    vanilla_entity_renderer_has_static_full_block_light, vanilla_entity_renderer_horse_kind,
    vanilla_entity_renderer_illager_kind, vanilla_entity_renderer_is_noop,
    vanilla_entity_renderer_llama_kind, vanilla_entity_renderer_minimum_block_light,
    vanilla_entity_renderer_skeleton_kind, vanilla_entity_renderer_thrown_item_scale,
    vanilla_entity_renderer_thrown_item_scales, vanilla_entity_renderer_undead_horse_kind,
    vanilla_entity_renderer_uses_native_non_model_pass, vanilla_entity_renderer_zombie_kind,
    VanillaBoatRendererFamily, VanillaBoatRendererKind, VanillaCamelRendererKind,
    VanillaEntityRendererCullingKind, VanillaHorseRendererKind, VanillaIllagerRendererKind,
    VanillaLlamaRendererKind, VanillaSkeletonRendererKind, VanillaUndeadHorseRendererKind,
    VanillaZombieRendererKind,
};
pub use renderer_catalog::{vanilla_entity_renderer_guardian_kind, VanillaGuardianRendererKind};
pub use renderer_catalog::{vanilla_entity_renderer_hoglin_kind, VanillaHoglinRendererKind};
pub use renderer_catalog::{vanilla_entity_renderer_piglin_kind, VanillaPiglinRendererKind};
pub use renderer_catalog::{
    vanilla_entity_renderer_presentation_owner, VanillaEntityRendererPresentationOwner,
};
pub use renderer_catalog::{vanilla_entity_renderer_squid_kind, VanillaSquidRendererKind};
pub use schema::vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type;
pub(crate) use schema::vanilla_entity_metadata_interact_owner;
pub(crate) use schema::vanilla_entity_metadata_mob_interact_owner;
pub(crate) use schema::vanilla_entity_metadata_type_accepts_local_attack_hurt;
pub(crate) use schema::vanilla_entity_metadata_type_is_living;
pub use schema::vanilla_entity_metadata_type_is_llama;
pub(crate) use schema::vanilla_entity_metadata_type_is_projectile;
pub(crate) use schema::vanilla_entity_metadata_type_uses_fishing_hook_entity_event_handler;
pub use schema::VanillaAbstractBoatMetadataAccessors;
pub use schema::VanillaAbstractHorseMetadataAccessors;
pub use schema::VanillaAbstractMinecartMetadataAccessors;
pub use schema::VanillaArrowMetadataAccessors;
pub use schema::VanillaCompanionVariantMetadataAccessors;
pub use schema::VanillaCopperGolemMetadataAccessors;
pub use schema::VanillaCreeperMetadataAccessors;
pub use schema::VanillaEndermanRenderMetadataAccessors;
pub use schema::VanillaFarmAnimalVariantMetadataAccessors;
pub use schema::VanillaFoxMetadataAccessors;
pub use schema::VanillaGoatMetadataAccessors;
pub use schema::VanillaHappyGhastMetadataAccessors;
pub use schema::VanillaInteractionMetadataAccessors;
pub use schema::VanillaLivingEntityMetadataAccessors;
pub use schema::VanillaLlamaMetadataAccessors;
pub use schema::VanillaMobMetadataAccessors;
pub use schema::VanillaPandaMetadataAccessors;
pub use schema::VanillaShulkerMetadataAccessors;
pub use schema::VanillaTamableAnimalMetadataAccessors;
pub use schema::VanillaThrownTridentMetadataAccessors;
pub use schema::VanillaTropicalFishVariantAccessors;
pub use schema::VanillaTurtleMetadataAccessors;
pub use schema::VanillaVehicleMetadataAccessors;
pub use schema::VanillaZombieMetadataAccessors;
pub use schema::VANILLA_ENDER_DRAGON_PHASE_HOLDING_PATTERN_ID;
pub use schema::VANILLA_IRON_GOLEM_FLAG_PLAYER_CREATED;
pub use schema::VANILLA_RABBIT_VARIANT_EVIL_ID;
pub use schema::{
    vanilla_entity_camera_post_effect_id,
    vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_boat_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_fish_from_bucket_accessor_for_entity_type,
    vanilla_entity_metadata_abstract_horse_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_horse_flags_accessor_for_entity_type,
    vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type,
    vanilla_entity_metadata_accessor_for_entity_type,
    vanilla_entity_metadata_age_locked_accessor_for_entity_type,
    vanilla_entity_metadata_allay_can_duplicate_accessor_for_entity_type,
    vanilla_entity_metadata_allay_dancing_accessor_for_entity_type,
    vanilla_entity_metadata_area_effect_cloud_radius_accessor_for_entity_type,
    vanilla_entity_metadata_armadillo_state_accessor_for_entity_type,
    vanilla_entity_metadata_armor_stand_accessors_for_entity_type,
    vanilla_entity_metadata_arrow_accessors_for_entity_type,
    vanilla_entity_metadata_axolotl_from_bucket_accessor_for_entity_type,
    vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type,
    vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type,
    vanilla_entity_metadata_baby_accessor_for_entity_type,
    vanilla_entity_metadata_bat_flags_accessor_for_entity_type,
    vanilla_entity_metadata_bee_flags_accessor_for_entity_type,
    vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type,
    vanilla_entity_metadata_camel_dash_accessor_for_entity_type,
    vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type,
    vanilla_entity_metadata_cat_lying_accessor_for_entity_type,
    vanilla_entity_metadata_cat_relax_state_one_accessor_for_entity_type,
    vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type,
    vanilla_entity_metadata_companion_variant_accessors_for_entity_type,
    vanilla_entity_metadata_copper_golem_accessors_for_entity_type,
    vanilla_entity_metadata_copper_golem_state_accessor_for_entity_type,
    vanilla_entity_metadata_creaking_active_accessor_for_entity_type,
    vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type,
    vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type,
    vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type,
    vanilla_entity_metadata_creeper_accessors_for_entity_type,
    vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type,
    vanilla_entity_metadata_enderman_render_accessors_for_entity_type,
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_eye_of_ender_item_accessor_for_entity_type,
    vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type,
    vanilla_entity_metadata_field, vanilla_entity_metadata_fox_accessors_for_entity_type,
    vanilla_entity_metadata_fox_flags_accessor_for_entity_type,
    vanilla_entity_metadata_fox_variant_accessor_for_entity_type,
    vanilla_entity_metadata_frog_variant_accessor_for_entity_type,
    vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type,
    vanilla_entity_metadata_ghast_charging_accessor_for_entity_type,
    vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type,
    vanilla_entity_metadata_goat_accessors_for_entity_type,
    vanilla_entity_metadata_happy_ghast_accessors_for_entity_type,
    vanilla_entity_metadata_horse_variant_accessor_for_entity_type,
    vanilla_entity_metadata_interaction_accessors_for_entity_type,
    vanilla_entity_metadata_iron_golem_flags_accessor_for_entity_type,
    vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type,
    vanilla_entity_metadata_living_entity_accessors_for_entity_type,
    vanilla_entity_metadata_llama_accessors_for_entity_type,
    vanilla_entity_metadata_mob_accessors_for_entity_type,
    vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type,
    vanilla_entity_metadata_ocelot_trusting_accessor_for_entity_type,
    vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type,
    vanilla_entity_metadata_panda_accessors_for_entity_type,
    vanilla_entity_metadata_panda_flags_accessor_for_entity_type,
    vanilla_entity_metadata_parrot_variant_accessor_for_entity_type,
    vanilla_entity_metadata_phantom_size_accessor_for_entity_type,
    vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type,
    vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type,
    vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type,
    vanilla_entity_metadata_player_absorption_accessor_for_entity_type,
    vanilla_entity_metadata_player_score_accessor_for_entity_type,
    vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type,
    vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type,
    vanilla_entity_metadata_rabbit_type_accessor_for_entity_type,
    vanilla_entity_metadata_raider_celebrating_accessor_for_entity_type,
    vanilla_entity_metadata_salmon_variant_accessor_for_entity_type,
    vanilla_entity_metadata_schema, vanilla_entity_metadata_sheep_wool_accessor_for_entity_type,
    vanilla_entity_metadata_shulker_accessors_for_entity_type,
    vanilla_entity_metadata_skeleton_freeze_conversion_accessor_for_entity_type,
    vanilla_entity_metadata_slime_size_accessor_for_entity_type,
    vanilla_entity_metadata_sniffer_drop_seed_at_tick_accessor_for_entity_type,
    vanilla_entity_metadata_sniffer_state_accessor_for_entity_type,
    vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type,
    vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type,
    vanilla_entity_metadata_strider_boost_time_accessor_for_entity_type,
    vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type,
    vanilla_entity_metadata_tamable_animal_accessors_for_entity_type,
    vanilla_entity_metadata_tamable_animal_flags_accessor_for_entity_type,
    vanilla_entity_metadata_thrown_trident_accessors_for_entity_type,
    vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type,
    vanilla_entity_metadata_turtle_accessors_for_entity_type,
    vanilla_entity_metadata_type_is_abstract_chested_horse, vanilla_entity_metadata_type_is_camel,
    vanilla_entity_metadata_type_is_guardian, vanilla_entity_metadata_type_is_leashable,
    vanilla_entity_metadata_type_is_slime,
    vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer,
    vanilla_entity_metadata_vehicle_accessors_for_entity_type,
    vanilla_entity_metadata_vex_flags_accessor_for_entity_type,
    vanilla_entity_metadata_villager_data_accessor_for_entity_type,
    vanilla_entity_metadata_villager_data_finalized_accessor_for_entity_type,
    vanilla_entity_metadata_warden_anger_level_accessor_for_entity_type,
    vanilla_entity_metadata_witch_using_item_accessor_for_entity_type,
    vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type,
    vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type,
    vanilla_entity_metadata_wolf_interested_accessor_for_entity_type,
    vanilla_entity_metadata_zombie_accessors_for_entity_type,
    vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type,
    vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type,
    vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type,
    vanilla_entity_type_has_attribute, VanillaAbstractArrowMetadataAccessors,
    VanillaArmorStandMetadataAccessors, VanillaEntityInterpolationPolicy,
    VanillaEntityMetadataAccessorCatalogEntry, VanillaEntityMetadataAccessors,
    VanillaEntityMetadataClassCatalogEntry, VanillaEntityMetadataDefault,
    VanillaEntityMetadataFamily, VanillaEntityMetadataFamilySchema,
    VanillaEntityMetadataFieldSchema, VanillaEntityMetadataFlagCatalogEntry,
    VanillaEntityMetadataFlagSchema, VanillaEntityMetadataFlagStatus,
    VanillaEntityMetadataResolverEvidence, VanillaEntityTypeMetadataCatalogEntry,
    VANILLA_AGEABLE_MOB_AGE_LOCKED_DATA_ID, VANILLA_AGEABLE_MOB_BABY_DATA_ID,
    VANILLA_AGEABLE_MOB_DEFAULT_AGE_LOCKED, VANILLA_AGEABLE_MOB_DEFAULT_BABY,
    VANILLA_ARMADILLO_STATE_IDLE_ID, VANILLA_ARMADILLO_STATE_ROLLING_ID,
    VANILLA_ARMADILLO_STATE_SCARED_ID, VANILLA_ARMADILLO_STATE_UNROLLING_ID,
    VANILLA_ARMOR_STAND_BODY_POSE_DATA_ID, VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID,
    VANILLA_ARMOR_STAND_CLIENT_FLAG_MARKER, VANILLA_ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE,
    VANILLA_ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS, VANILLA_ARMOR_STAND_CLIENT_FLAG_SMALL,
    VANILLA_ARMOR_STAND_DEFAULT_BODY_POSE, VANILLA_ARMOR_STAND_DEFAULT_CLIENT_FLAGS,
    VANILLA_ARMOR_STAND_DEFAULT_HEAD_POSE, VANILLA_ARMOR_STAND_DEFAULT_LEFT_ARM_POSE,
    VANILLA_ARMOR_STAND_DEFAULT_LEFT_LEG_POSE, VANILLA_ARMOR_STAND_DEFAULT_RIGHT_ARM_POSE,
    VANILLA_ARMOR_STAND_DEFAULT_RIGHT_LEG_POSE, VANILLA_ARMOR_STAND_HEAD_POSE_DATA_ID,
    VANILLA_ARMOR_STAND_LEFT_ARM_POSE_DATA_ID, VANILLA_ARMOR_STAND_LEFT_LEG_POSE_DATA_ID,
    VANILLA_ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID, VANILLA_ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID,
    VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION, VANILLA_AVATAR_MAIN_HAND_DATA_ID,
    VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID, VANILLA_AVATAR_MODEL_PART_CAPE,
    VANILLA_AVATAR_MODEL_PART_HAT, VANILLA_AVATAR_MODEL_PART_JACKET,
    VANILLA_AVATAR_MODEL_PART_LEFT_PANTS_LEG, VANILLA_AVATAR_MODEL_PART_LEFT_SLEEVE,
    VANILLA_AVATAR_MODEL_PART_RIGHT_PANTS_LEG, VANILLA_AVATAR_MODEL_PART_RIGHT_SLEEVE,
    VANILLA_ENTITY_AIR_SUPPLY_DATA_ID, VANILLA_ENTITY_CUSTOM_NAME_DATA_ID,
    VANILLA_ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID, VANILLA_ENTITY_DEFAULT_MAX_AIR_SUPPLY_TICKS,
    VANILLA_ENTITY_NO_GRAVITY_DATA_ID, VANILLA_ENTITY_POSE_DATA_ID,
    VANILLA_ENTITY_SHARED_FLAGS_DATA_ID, VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING,
    VANILLA_ENTITY_SHARED_FLAG_GLOWING, VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
    VANILLA_ENTITY_SHARED_FLAG_ON_FIRE, VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
    VANILLA_ENTITY_SHARED_FLAG_SPRINTING, VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
    VANILLA_ENTITY_SILENT_DATA_ID, VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
    VANILLA_FISHING_HOOK_BITING_DATA_ID, VANILLA_FISHING_HOOK_DEFAULT_BITING,
    VANILLA_FISHING_HOOK_DEFAULT_HOOKED_ENTITY, VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID,
    VANILLA_HANGING_ENTITY_DEFAULT_DIRECTION_ID, VANILLA_HANGING_ENTITY_DIRECTION_DATA_ID,
    VANILLA_HUMANOID_ARM_LEFT_ID, VANILLA_HUMANOID_ARM_RIGHT_ID,
    VANILLA_ITEM_FRAME_DEFAULT_ROTATION, VANILLA_ITEM_FRAME_ITEM_DATA_ID,
    VANILLA_ITEM_FRAME_ROTATION_DATA_ID, VANILLA_LIVING_ENTITY_ARROW_COUNT_DATA_ID,
    VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID, VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID,
    VANILLA_LIVING_ENTITY_FLAGS_DATA_ID, VANILLA_LIVING_ENTITY_FLAG_IS_USING,
    VANILLA_LIVING_ENTITY_FLAG_OFF_HAND, VANILLA_LIVING_ENTITY_FLAG_SPIN_ATTACK,
    VANILLA_LIVING_ENTITY_HEALTH_DATA_ID, VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID,
    VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID, VANILLA_MOB_FLAGS_DATA_ID,
    VANILLA_MOB_FLAG_AGGRESSIVE, VANILLA_MOB_FLAG_LEFT_HANDED, VANILLA_MOB_FLAG_NO_AI,
    VANILLA_OMINOUS_ITEM_SPAWNER_ITEM_DATA_ID, VANILLA_PAINTING_VARIANT_DATA_ID,
    VANILLA_PLAYER_ABSORPTION_DATA_ID, VANILLA_PLAYER_DEFAULT_ABSORPTION,
    VANILLA_PLAYER_DEFAULT_SCORE, VANILLA_PLAYER_SCORE_DATA_ID,
    VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID, VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID,
};
pub use schema::{
    vanilla_entity_metadata_bee_accessors_for_entity_type, VanillaBeeMetadataAccessors,
};
pub use schema::{
    vanilla_entity_metadata_cat_accessors_for_entity_type, VanillaCatMetadataAccessors,
};
pub use schema::{
    vanilla_entity_metadata_dolphin_accessors_for_entity_type, VanillaDolphinMetadataAccessors,
};
pub(crate) use schema::{
    vanilla_entity_metadata_eating_sound_policy, vanilla_entity_metadata_food_predicate,
    VanillaEntityEatingSoundPolicy, VanillaEntityFoodPredicate,
};
pub use schema::{
    vanilla_entity_metadata_end_crystal_accessors_for_entity_type,
    vanilla_entity_metadata_hanging_direction_accessor_for_entity_type,
    vanilla_entity_metadata_item_frame_accessors_for_entity_type,
    vanilla_entity_metadata_painting_variant_accessor_for_entity_type,
    VanillaEndCrystalMetadataAccessors, VanillaHangingEntityMetadataAccessor,
    VanillaItemFrameMetadataAccessors, VANILLA_END_CRYSTAL_BEAM_TARGET_DATA_ID,
    VANILLA_END_CRYSTAL_DEFAULT_SHOW_BOTTOM, VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID,
};
pub use schema::{
    vanilla_entity_metadata_firework_rocket_accessors_for_entity_type,
    VanillaFireworkRocketMetadataAccessors,
};
pub use schema::{
    vanilla_entity_metadata_primed_tnt_accessors_for_entity_type, VanillaPrimedTntMetadataAccessors,
};
pub use schema::{
    vanilla_entity_metadata_type_block_effect_tick_policy, VanillaEntityBlockEffectTickPolicy,
};
pub use schema::{
    vanilla_entity_metadata_type_collidability_kind,
    vanilla_entity_metadata_type_death_sound_policy,
    vanilla_entity_metadata_type_hurt_sound_policy, vanilla_entity_metadata_type_pushability_kind,
    vanilla_entity_metadata_type_sound_source_kind,
    vanilla_entity_metadata_type_sound_volume_policy, VanillaEntityCollidabilityKind,
    VanillaEntityDeathSoundPolicy, VanillaEntityHurtSoundPolicy, VanillaEntityPushabilityKind,
    VanillaEntitySoundCondition, VanillaEntitySoundSourceKind, VanillaEntitySoundVariantFamily,
    VanillaEntitySoundVolumePolicy,
};
pub use schema::{
    vanilla_entity_metadata_wolf_accessors_for_entity_type, VanillaWolfMetadataAccessors,
};
pub use schema::{vanilla_entity_scale_sanitizer, VanillaEntityScaleSanitizer};
pub use schema::{
    vanilla_minecart_default_display_offset,
    VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID,
    VANILLA_ABSTRACT_MINECART_DISPLAY_OFFSET_DATA_ID,
    VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
    VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID, VANILLA_FURNACE_MINECART_DEFAULT_FUEL,
    VANILLA_FURNACE_MINECART_FUEL_DATA_ID,
};
pub use schema::{VanillaEntityEventHandlerCatalogEntry, VANILLA_ENTITY_EVENT_HANDLERS_26_1};
pub use schema::{
    VANILLA_ABSTRACT_ARROW_DEFAULT_FLAGS, VANILLA_ABSTRACT_ARROW_DEFAULT_IN_GROUND,
    VANILLA_ABSTRACT_ARROW_DEFAULT_PIERCE_LEVEL, VANILLA_ABSTRACT_ARROW_FLAGS_DATA_ID,
    VANILLA_ABSTRACT_ARROW_FLAG_CRIT, VANILLA_ABSTRACT_ARROW_FLAG_NO_PHYSICS,
    VANILLA_ABSTRACT_ARROW_IN_GROUND_DATA_ID, VANILLA_ABSTRACT_ARROW_PIERCE_LEVEL_DATA_ID,
    VANILLA_ARROW_DEFAULT_EFFECT_COLOR, VANILLA_ARROW_EFFECT_COLOR_DATA_ID,
    VANILLA_TRIDENT_DEFAULT_FOIL, VANILLA_TRIDENT_DEFAULT_LOYALTY, VANILLA_TRIDENT_FOIL_DATA_ID,
    VANILLA_TRIDENT_LOYALTY_DATA_ID, VANILLA_WITHER_SKULL_DANGEROUS_DATA_ID,
    VANILLA_WITHER_SKULL_DEFAULT_DANGEROUS,
};
pub use schema::{
    VANILLA_ABSTRACT_BOAT_BUBBLE_TIME_DATA_ID, VANILLA_ABSTRACT_BOAT_DEFAULT_BUBBLE_TIME,
    VANILLA_ABSTRACT_BOAT_DEFAULT_PADDLE_LEFT, VANILLA_ABSTRACT_BOAT_DEFAULT_PADDLE_RIGHT,
    VANILLA_ABSTRACT_BOAT_PADDLE_LEFT_DATA_ID, VANILLA_ABSTRACT_BOAT_PADDLE_RIGHT_DATA_ID,
};
pub use schema::{
    VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID, VANILLA_ABSTRACT_CHESTED_HORSE_DEFAULT_CHEST,
    VANILLA_HORSE_COLOR_MASK, VANILLA_HORSE_DEFAULT_VARIANT, VANILLA_HORSE_MARKINGS_MASK,
    VANILLA_HORSE_MARKINGS_SHIFT, VANILLA_HORSE_VARIANT_DATA_ID, VANILLA_LLAMA_DEFAULT_STRENGTH,
    VANILLA_LLAMA_DEFAULT_VARIANT, VANILLA_LLAMA_STRENGTH_DATA_ID, VANILLA_LLAMA_VARIANT_DATA_ID,
};
pub use schema::{
    VANILLA_ABSTRACT_FISH_DEFAULT_FROM_BUCKET, VANILLA_ABSTRACT_FISH_FROM_BUCKET_DATA_ID,
};
pub use schema::{
    VANILLA_ABSTRACT_HORSE_DEFAULT_FLAGS, VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID,
    VANILLA_ABSTRACT_HORSE_FLAG_BRED, VANILLA_ABSTRACT_HORSE_FLAG_EATING,
    VANILLA_ABSTRACT_HORSE_FLAG_OPEN_MOUTH, VANILLA_ABSTRACT_HORSE_FLAG_STANDING,
    VANILLA_ABSTRACT_HORSE_FLAG_TAME,
};
pub use schema::{
    VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID, VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH,
    VANILLA_ZOMBIE_NAUTILUS_DEFAULT_VARIANT_RESOURCE_ID, VANILLA_ZOMBIE_NAUTILUS_VARIANT_DATA_ID,
};
pub use schema::{
    VANILLA_ABSTRACT_VILLAGER_DEFAULT_UNHAPPY_COUNTER,
    VANILLA_ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID,
};
pub use schema::{VANILLA_ALLAY_CAN_DUPLICATE_DATA_ID, VANILLA_ALLAY_DEFAULT_CAN_DUPLICATE};
pub use schema::{VANILLA_AXOLOTL_DEFAULT_FROM_BUCKET, VANILLA_AXOLOTL_FROM_BUCKET_DATA_ID};
pub use schema::{VANILLA_BAT_DEFAULT_FLAGS, VANILLA_BAT_FLAGS_DATA_ID, VANILLA_BAT_FLAG_RESTING};
pub use schema::{
    VANILLA_BEE_ANGER_END_TIME_DATA_ID, VANILLA_BEE_DEFAULT_ANGER_END_TIME,
    VANILLA_BEE_DEFAULT_FLAGS, VANILLA_BEE_FLAGS_DATA_ID, VANILLA_BEE_FLAG_HAS_NECTAR,
    VANILLA_BEE_FLAG_HAS_STUNG, VANILLA_BEE_FLAG_ROLL,
};
pub use schema::{
    VANILLA_CAT_DEFAULT_SOUND_VARIANT_RESOURCE_ID, VANILLA_CAT_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_CAT_SOUND_VARIANT_DATA_ID, VANILLA_CAT_VARIANT_DATA_ID,
};
pub use schema::{
    VANILLA_CHICKEN_DEFAULT_SOUND_VARIANT_RESOURCE_ID, VANILLA_CHICKEN_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_CHICKEN_SOUND_VARIANT_DATA_ID, VANILLA_CHICKEN_VARIANT_DATA_ID,
};
pub use schema::{
    VANILLA_COPPER_GOLEM_STATE_DROPPING_ITEM_ID, VANILLA_COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_GETTING_ITEM_ID, VANILLA_COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_IDLE_ID, VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID,
};
pub use schema::{
    VANILLA_COW_DEFAULT_SOUND_VARIANT_RESOURCE_ID, VANILLA_COW_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_COW_SOUND_VARIANT_DATA_ID, VANILLA_COW_VARIANT_DATA_ID,
};
pub use schema::{
    VANILLA_CREEPER_DEFAULT_IGNITED, VANILLA_CREEPER_DEFAULT_POWERED,
    VANILLA_CREEPER_DEFAULT_SWELL_DIR, VANILLA_CREEPER_IGNITED_DATA_ID,
    VANILLA_CREEPER_POWERED_DATA_ID, VANILLA_CREEPER_SWELL_DIR_DATA_ID,
};
pub use schema::{
    VANILLA_DOLPHIN_DEFAULT_GOT_FISH, VANILLA_DOLPHIN_DEFAULT_MOISTNESS,
    VANILLA_DOLPHIN_GOT_FISH_DATA_ID, VANILLA_DOLPHIN_MOISTNESS_DATA_ID,
};
pub use schema::{
    VANILLA_ENDERMAN_CARRY_STATE_DATA_ID, VANILLA_ENDERMAN_CREEPY_DATA_ID,
    VANILLA_ENDERMAN_DEFAULT_CARRY_STATE, VANILLA_ENDERMAN_DEFAULT_CREEPY,
    VANILLA_ENDERMAN_DEFAULT_STARED_AT, VANILLA_ENDERMAN_STARED_AT_DATA_ID,
};
pub use schema::{
    VANILLA_ENTITY_METADATA_ACCESSORS_26_1, VANILLA_ENTITY_METADATA_CLASSES_26_1,
    VANILLA_ENTITY_METADATA_DEFAULT_RESOLVER_EVIDENCE_26_1, VANILLA_ENTITY_METADATA_FLAGS_26_1,
    VANILLA_ENTITY_METADATA_FLAG_RESOLVER_EVIDENCE_26_1, VANILLA_ENTITY_TYPE_METADATA_26_1,
};
pub use schema::{VANILLA_EXPERIENCE_ORB_DEFAULT_VALUE, VANILLA_EXPERIENCE_ORB_VALUE_DATA_ID};
pub use schema::{
    VANILLA_FIREWORK_ROCKET_ATTACHED_TO_TARGET_DATA_ID,
    VANILLA_FIREWORK_ROCKET_DEFAULT_ITEM_RESOURCE_ID,
    VANILLA_FIREWORK_ROCKET_DEFAULT_SHOT_AT_ANGLE, VANILLA_FIREWORK_ROCKET_ITEM_DATA_ID,
    VANILLA_FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID,
};
pub use schema::{
    VANILLA_FOX_DEFAULT_FLAGS, VANILLA_FOX_DEFAULT_TYPE, VANILLA_FOX_FLAGS_DATA_ID,
    VANILLA_FOX_FLAG_CROUCHING, VANILLA_FOX_FLAG_DEFENDING, VANILLA_FOX_FLAG_FACEPLANTED,
    VANILLA_FOX_FLAG_INTERESTED, VANILLA_FOX_FLAG_POUNCING, VANILLA_FOX_FLAG_SITTING,
    VANILLA_FOX_FLAG_SLEEPING, VANILLA_FOX_TRUSTED_DATA_ID_0, VANILLA_FOX_TRUSTED_DATA_ID_1,
    VANILLA_FOX_TYPE_DATA_ID,
};
pub use schema::{
    VANILLA_FROG_DEFAULT_VARIANT_RESOURCE_ID, VANILLA_FROG_TONGUE_TARGET_DATA_ID,
    VANILLA_FROG_VARIANT_DATA_ID,
};
pub use schema::{VANILLA_GHAST_CHARGING_DATA_ID, VANILLA_GHAST_DEFAULT_CHARGING};
pub use schema::{VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID, VANILLA_GLOW_SQUID_DEFAULT_DARK_TICKS};
pub use schema::{
    VANILLA_GOAT_DEFAULT_HAS_LEFT_HORN, VANILLA_GOAT_DEFAULT_HAS_RIGHT_HORN,
    VANILLA_GOAT_DEFAULT_SCREAMING, VANILLA_GOAT_LEFT_HORN_DATA_ID,
    VANILLA_GOAT_RIGHT_HORN_DATA_ID, VANILLA_GOAT_SCREAMING_DATA_ID,
};
pub use schema::{
    VANILLA_HAPPY_GHAST_DEFAULT_IS_LEASH_HOLDER, VANILLA_HAPPY_GHAST_DEFAULT_STAYS_STILL,
    VANILLA_HAPPY_GHAST_IS_LEASH_HOLDER_DATA_ID, VANILLA_HAPPY_GHAST_STAYS_STILL_DATA_ID,
};
pub use schema::{
    VANILLA_HOGLIN_DEFAULT_IMMUNE_TO_ZOMBIFICATION, VANILLA_HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID,
};
pub use schema::{
    VANILLA_INTERACTION_DEFAULT_HEIGHT, VANILLA_INTERACTION_DEFAULT_RESPONSE,
    VANILLA_INTERACTION_DEFAULT_WIDTH, VANILLA_INTERACTION_HEIGHT_DATA_ID,
    VANILLA_INTERACTION_RESPONSE_DATA_ID, VANILLA_INTERACTION_WIDTH_DATA_ID,
};
pub use schema::{VANILLA_OCELOT_DEFAULT_TRUSTING, VANILLA_OCELOT_TRUSTING_DATA_ID};
pub use schema::{
    VANILLA_PANDA_DEFAULT_EAT_COUNTER, VANILLA_PANDA_DEFAULT_FLAGS,
    VANILLA_PANDA_DEFAULT_HIDDEN_GENE, VANILLA_PANDA_DEFAULT_MAIN_GENE,
    VANILLA_PANDA_DEFAULT_SNEEZE_COUNTER, VANILLA_PANDA_DEFAULT_UNHAPPY_COUNTER,
    VANILLA_PANDA_EAT_COUNTER_DATA_ID, VANILLA_PANDA_FLAGS_DATA_ID, VANILLA_PANDA_FLAG_ON_BACK,
    VANILLA_PANDA_FLAG_ROLLING, VANILLA_PANDA_FLAG_SITTING, VANILLA_PANDA_FLAG_SNEEZING,
    VANILLA_PANDA_HIDDEN_GENE_DATA_ID, VANILLA_PANDA_MAIN_GENE_DATA_ID,
    VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID, VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID,
};
pub use schema::{
    VANILLA_PIG_BOOST_TIME_DATA_ID, VANILLA_PIG_DEFAULT_BOOST_TIME,
    VANILLA_PIG_DEFAULT_SOUND_VARIANT_RESOURCE_ID, VANILLA_PIG_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_PIG_SOUND_VARIANT_DATA_ID, VANILLA_PIG_VARIANT_DATA_ID,
};
pub use schema::{
    VANILLA_PRIMED_TNT_BLOCK_STATE_DATA_ID, VANILLA_PRIMED_TNT_DEFAULT_BLOCK_STATE,
    VANILLA_PRIMED_TNT_DEFAULT_FUSE, VANILLA_PRIMED_TNT_FUSE_DATA_ID,
};
pub use schema::{
    VANILLA_SKELETON_DEFAULT_STRAY_CONVERSION, VANILLA_SKELETON_STRAY_CONVERSION_DATA_ID,
};
pub use schema::{
    VANILLA_SNIFFER_DEFAULT_DROP_SEED_AT_TICK, VANILLA_SNIFFER_DROP_SEED_AT_TICK_DATA_ID,
    VANILLA_SNIFFER_STATE_DIGGING_ID, VANILLA_SNIFFER_STATE_FEELING_HAPPY_ID,
    VANILLA_SNIFFER_STATE_IDLING_ID, VANILLA_SNIFFER_STATE_RISING_ID,
    VANILLA_SNIFFER_STATE_SCENTING_ID, VANILLA_SNIFFER_STATE_SEARCHING_ID,
    VANILLA_SNIFFER_STATE_SNIFFING_ID,
};
pub use schema::{
    VANILLA_STRIDER_BOOST_TIME_DATA_ID, VANILLA_STRIDER_DEFAULT_BOOST_TIME,
    VANILLA_STRIDER_DEFAULT_SUFFOCATING, VANILLA_STRIDER_SUFFOCATING_DATA_ID,
};
pub use schema::{VANILLA_TADPOLE_AGE_LOCKED_DATA_ID, VANILLA_TADPOLE_DEFAULT_AGE_LOCKED};
pub use schema::{
    VANILLA_TAMABLE_ANIMAL_DEFAULT_FLAGS, VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID,
    VANILLA_TAMABLE_ANIMAL_FLAG_SITTING, VANILLA_TAMABLE_ANIMAL_FLAG_TAME,
    VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID,
};
pub use schema::{
    VANILLA_TURTLE_DEFAULT_HAS_EGG, VANILLA_TURTLE_DEFAULT_LAYING_EGG,
    VANILLA_TURTLE_HAS_EGG_DATA_ID, VANILLA_TURTLE_LAYING_EGG_DATA_ID,
};
pub use schema::{
    VANILLA_VEHICLE_DAMAGE_DATA_ID, VANILLA_VEHICLE_DEFAULT_DAMAGE,
    VANILLA_VEHICLE_DEFAULT_HURT_DIR, VANILLA_VEHICLE_DEFAULT_HURT_TIME,
    VANILLA_VEHICLE_HURT_DIR_DATA_ID, VANILLA_VEHICLE_HURT_TIME_DATA_ID,
};
pub use schema::{VANILLA_VEX_DEFAULT_FLAGS, VANILLA_VEX_FLAGS_DATA_ID, VANILLA_VEX_FLAG_CHARGING};
pub use schema::{
    VANILLA_VILLAGER_DATA_DATA_ID, VANILLA_VILLAGER_DATA_FINALIZED_DATA_ID,
    VANILLA_VILLAGER_DEFAULT_DATA_FINALIZED, VANILLA_ZOMBIE_VILLAGER_CONVERTING_DATA_ID,
    VANILLA_ZOMBIE_VILLAGER_DATA_DATA_ID, VANILLA_ZOMBIE_VILLAGER_DATA_FINALIZED_DATA_ID,
    VANILLA_ZOMBIE_VILLAGER_DEFAULT_CONVERTING, VANILLA_ZOMBIE_VILLAGER_DEFAULT_DATA_FINALIZED,
};
pub use schema::{
    VANILLA_WITHER_DEFAULT_INVULNERABLE_TICKS, VANILLA_WITHER_DEFAULT_TARGET_A,
    VANILLA_WITHER_INVULNERABLE_TICKS_DATA_ID, VANILLA_WITHER_TARGET_A_DATA_ID,
};
pub use schema::{
    VANILLA_WOLF_DEFAULT_SOUND_VARIANT_RESOURCE_ID, VANILLA_WOLF_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_WOLF_SOUND_VARIANT_DATA_ID, VANILLA_WOLF_VARIANT_DATA_ID,
};
pub use schema::{VANILLA_ZOGLIN_BABY_DATA_ID, VANILLA_ZOGLIN_DEFAULT_BABY};
pub use schema::{
    VANILLA_ZOMBIE_BABY_DATA_ID, VANILLA_ZOMBIE_DEFAULT_BABY,
    VANILLA_ZOMBIE_DEFAULT_DROWNED_CONVERSION, VANILLA_ZOMBIE_DEFAULT_SPECIAL_TYPE,
    VANILLA_ZOMBIE_DROWNED_CONVERSION_DATA_ID, VANILLA_ZOMBIE_SPECIAL_TYPE_DATA_ID,
};
pub use shadows::{EntityShadowPieceState, EntityShadowRenderSourceState};
pub use sound_variants::{
    CatSoundVariantIdentity, ChickenSoundVariantIdentity, CowSoundVariantIdentity,
    PigSoundVariantIdentity, WolfSoundVariantIdentity,
};
use status::{EntityDamageEventState, MobEffectState};
pub(crate) use store::EntityStore;
pub(crate) use store::LocalPlayerBlockEffectStep;
pub use store::{EntityClientTickWork, EntityPublicationSnapshotObservation};

// IDs are the vanilla 26.1 EntityType registry order from EntityType.java.
const VANILLA_DEFAULT_ENTITY_OUTLINE_RGB: u32 = 0x00ff_ffff;
const VANILLA_OPAQUE_ALPHA: u32 = 0xff00_0000;

pub(crate) use VANILLA_ENTITY_DEFAULT_MAX_AIR_SUPPLY_TICKS as VANILLA_MAX_AIR_SUPPLY_TICKS;
pub fn vanilla_entity_type_is_living(entity_type_id: i32) -> bool {
    vanilla_living_entity_type(entity_type_id)
}

/// Vanilla `Attributes.MAX_HEALTH` `RangedAttribute` default value
/// (`20.0`, Attributes.java:59).
const VANILLA_DEFAULT_MAX_HEALTH: f64 = 20.0;

/// The local player's living vehicle health pair for the HUD vehicle hearts
/// (vanilla `Gui.extractVehicleHealth` inputs): the synced `DATA_HEALTH_ID`
/// float and the resolved MAX_HEALTH attribute value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalPlayerVehicleHealth {
    /// Vanilla `LivingEntity.getHealth()` (ceil is applied at draw time,
    /// Gui.java:979).
    pub health: f32,
    /// Vanilla `LivingEntity.getMaxHealth()` = the MAX_HEALTH attribute value.
    pub max_health: f64,
}

/// Local-player melee swing state sampled for first-person item rendering.
///
/// Vanilla `ItemInHandRenderer.renderHandsWithItems` reads
/// `LocalPlayer.getAttackAnim(partialTick)` and `swingingArm` before dispatching each
/// hand. `off_hand = false` means the main hand receives `attack_anim`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalPlayerAttackSwingState {
    pub attack_anim: f32,
    pub off_hand: bool,
}

pub(crate) fn is_vanilla_boat_type(entity_type_id: i32) -> bool {
    vanilla_entity_type_is_boat(entity_type_id)
}

pub fn vanilla_entity_type_is_boat(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_boat(entity_type_id)
}

pub fn vanilla_entity_type_is_chest_boat(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_abstract_chest_boat(entity_type_id)
}

pub(crate) fn is_vanilla_minecart_type(entity_type_id: i32) -> bool {
    vanilla_entity_type_is_minecart(entity_type_id)
}

/// Vanilla 26.1 constructor defaults for `Entity.blocksBuilding`.
///
/// Armor stands are included through `LivingEntity`; their synchronized marker
/// flag is applied by the entity-store query. Spectator players are filtered by
/// [`WorldStore`] because that state belongs to the player-info owner rather
/// than the entity type.
pub(crate) fn vanilla_entity_type_blocks_building(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_is_living(entity_type_id)
        || is_vanilla_boat_type(entity_type_id)
        || is_vanilla_minecart_type(entity_type_id)
        || matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_END_CRYSTAL_ID
                | VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID
                | VANILLA_ENTITY_TYPE_TNT_ID
        )
}

pub fn vanilla_entity_type_is_minecart(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_minecart(entity_type_id)
}

pub fn vanilla_entity_type_is_item_frame(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_item_frame(entity_type_id)
}

pub fn vanilla_entity_type_is_abstract_piglin(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_abstract_piglin(entity_type_id)
}

pub fn vanilla_entity_type_is_spellcaster_illager(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_spellcaster_illager(entity_type_id)
}

pub fn vanilla_entity_type_illager_arm_pose_uses_celebrating(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_illager_arm_pose_uses_celebrating(entity_type_id)
}

pub fn vanilla_entity_type_is_spider(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_spider(entity_type_id)
}

pub fn vanilla_entity_type_is_wind_charge(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_wind_charge(entity_type_id)
}

pub fn vanilla_entity_type_is_hurting_projectile(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_hurting_projectile(entity_type_id)
}

pub(crate) fn is_vanilla_vehicle_entity_type(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_vehicle(entity_type_id)
}

pub(crate) fn is_vanilla_abstract_horse_type(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_abstract_horse(entity_type_id)
}

pub(crate) fn is_vanilla_abstract_nautilus_type(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_abstract_nautilus(entity_type_id)
}

pub(crate) fn is_vanilla_taming_particle_entity_type(entity_type_id: i32) -> bool {
    is_vanilla_abstract_horse_type(entity_type_id)
        || schema::vanilla_entity_metadata_type_is_tamable_animal(entity_type_id)
}

pub(crate) fn is_vanilla_can_equip_saddle_type(entity_type_id: i32) -> bool {
    vanilla_entity_type_can_equip_saddle(entity_type_id)
}

pub(crate) fn is_vanilla_can_wear_horse_armor_type(entity_type_id: i32) -> bool {
    vanilla_entity_type_can_wear_horse_armor(entity_type_id)
}

pub(crate) fn vanilla_entity_type_can_use_every_equipment_slot(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_can_use_every_equipment_slot(entity_type_id)
}

pub(crate) fn is_vanilla_llama_type(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_llama(entity_type_id)
}

pub fn vanilla_entity_type_is_squid(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_squid(entity_type_id)
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
    #[serde(default = "entity_model_source_full_bright_light")]
    pub light: TerrainLight,
    /// Constructor-stable `ItemEntity.bobOffs`; zero for non-item users of the
    /// generic ITEM_STACK projection.
    #[serde(default)]
    pub bob_offset: f32,
    pub stack: ProtocolItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FireworkRocketItemState {
    pub entity_id: i32,
    pub position: EntityVec3,
    #[serde(default = "entity_model_source_full_bright_light")]
    pub light: TerrainLight,
    pub stack: ProtocolItemStackSummary,
    #[serde(default)]
    pub shot_at_angle: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FireworkRocketExplosionParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub delta_movement: EntityVec3,
    #[serde(default)]
    pub has_explosions: bool,
    #[serde(default)]
    pub explosions: Vec<ProtocolFireworkExplosionSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FireworkRocketTrailParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub delta_movement: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EyeOfEnderTrailParticleState {
    pub entity_id: i32,
    pub origin: EntityVec3,
    pub delta_movement: EntityVec3,
    #[serde(default)]
    pub in_water: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShulkerBulletTrailParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EntityClientMovementSimpleParticleKind {
    Bubble,
    Crit,
    Smoke,
    LargeSmoke,
    DragonBreath,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityClientMovementSimpleParticleState {
    pub entity_id: i32,
    pub kind: EntityClientMovementSimpleParticleKind,
    pub position: EntityVec3,
    pub velocity: EntityVec3,
}

/// One `PowderSnowBlock.entityInside` snowflake chance. The consumer owns the
/// level random, so Vanilla's `nextBoolean` gate and velocity rolls happen at
/// spawn time; only the anchored position is decided here.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PowderSnowFlakeParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
}

/// One Blaze `aiStep` particle submission. Keeping both smoke positions in one
/// typed owner preserves the Vanilla per-entity ordering while avoiding two
/// maximum-sized FIFO entries and duplicate renderer template lookup.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BlazeAmbientParticleState {
    pub entity_id: i32,
    pub positions: [EntityVec3; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnderAmbientParticleState {
    pub entity_id: i32,
    pub positions: [EntityVec3; 2],
    pub velocities: [EntityVec3; 2],
}

/// One moving, submerged Guardian `aiStep` bubble pair. Keeping the fixed pair
/// under one FIFO owner preserves per-entity ordering and performs one particle
/// template lookup in the native resolver.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GuardianAmbientParticleState {
    pub entity_id: i32,
    pub positions: [EntityVec3; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuardianAttackParticleState {
    pub entity_id: i32,
    pub positions: Vec<EntityVec3>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlimeLandingParticleKind {
    ItemSlime,
    Flame,
}

/// The complete ordered particle submission from one `Slime.tick` landing.
/// Vanilla rolls every radial position before evaluating the squish sound.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlimeLandingParticleState {
    pub entity_id: i32,
    pub kind: SlimeLandingParticleKind,
    pub positions: Vec<EntityVec3>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntityClientMovementParticleState {
    BlazeAmbient(BlazeAmbientParticleState),
    EnderAmbient(EnderAmbientParticleState),
    GuardianAmbient(GuardianAmbientParticleState),
    GuardianAttack(GuardianAttackParticleState),
    FireworkRocket(FireworkRocketTrailParticleState),
    EyeOfEnder(EyeOfEnderTrailParticleState),
    ShulkerBullet(ShulkerBulletTrailParticleState),
    SlimeLanding(SlimeLandingParticleState),
    Simple(EntityClientMovementSimpleParticleState),
    PowderSnowFlake(PowderSnowFlakeParticleState),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EntityClientMovementSoundKind {
    BoatPaddleLeft,
    BoatPaddleRight,
    BlazeBurn {
        volume: f32,
        pitch: f32,
    },
    GuardianFlop {
        elder: bool,
    },
    SlimeSquish {
        magma_cube: bool,
        tiny: bool,
        volume: f32,
        pitch: f32,
    },
    CamelDashReady {
        husk: bool,
    },
    CamelStand {
        husk: bool,
        pitch: f32,
    },
    NautilusDashReady {
        zombie: bool,
        underwater: bool,
        pitch: f32,
    },
    BreezeDeflect,
    FireworkRocketLaunch,
    TridentReturn,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityClientMovementSoundState {
    pub entity_id: i32,
    pub kind: EntityClientMovementSoundKind,
    pub position: EntityVec3,
}

/// Client-owned particle and sound effects emitted by entity movement in exact
/// vanilla tick order. The queue is drained into the lossless simulation effect
/// batch before the corresponding immutable publication becomes visible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntityClientMovementEffectState {
    Particle(EntityClientMovementParticleState),
    Sound(EntityClientMovementSoundState),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LlamaSpitRecreationParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub delta_movement: EntityVec3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OminousItemSpawnerItemState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub age_ticks: f32,
    pub stack: ProtocolItemStackSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OminousItemSpawnerParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PrimedTntSmokeParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
}

/// The wall the front of an item frame faces (vanilla `ItemFrame.getDirection`). Drives the frame's
/// render orientation: horizontal facings rotate the frame about Y, vertical facings tilt it about X.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemFrameFacing {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

/// Everything needed to render one item-frame entity (vanilla `ItemFrameRenderState`): the resolved
/// wall-mounted center, the facing wall, the sampled entity-renderer light, the `0..=7` item rotation,
/// whether it is a glow frame, whether the entity is invisible, the framed item (`None` for an empty
/// frame), and the framed map id (which vanilla renders as a full-frame map only when map data exists).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemFrameRenderState {
    pub entity_id: i32,
    pub center: EntityVec3,
    pub facing: ItemFrameFacing,
    #[serde(default = "entity_model_source_full_bright_light")]
    pub light: TerrainLight,
    pub rotation: u8,
    pub glow: bool,
    #[serde(default)]
    pub invisible: bool,
    pub item: Option<ProtocolItemStackSummary>,
    #[serde(default)]
    pub map_id: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityInterpolationState {
    pub remaining_steps: u8,
    pub target_position: EntityVec3,
    pub target_y_rot: f32,
    pub target_x_rot: f32,
    pub previous_position: EntityVec3,
    pub previous_y_rot: f32,
    pub previous_x_rot: f32,
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
    /// Active vanilla `InterpolationHandler` target and previous-tick state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpolation: Option<EntityInterpolationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_supporting_block_pos: Option<BlockPos>,
    #[serde(default, skip_serializing_if = "bool_is_false")]
    pub on_ground_no_blocks: bool,
    pub data_values: Vec<ProtocolEntityDataValue>,
    /// Client fields projected from the painting variant holder through the active registry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub painting_variant_projection: Option<PaintingVariantClientProjection>,
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
    /// Vanilla client-side `LivingEntity.invulnerableTime`, reset by
    /// DamageEvent and decremented by scheduled living client ticks.
    #[serde(default)]
    pub damage_invulnerable_time: i32,
    /// Client-side `LivingEntity.dead`. Unlike synchronized health, this becomes
    /// true only after the guarded base `die` transition has run.
    #[serde(default, skip_serializing_if = "bool_is_false")]
    pub living_dead: bool,
    /// Client-owned `Player.sleepCounter`. `None` identifies every non-player
    /// entity; player incarnations always project `Some(0..=109)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_sleep_counter: Option<i32>,
    #[serde(default)]
    pub minecart_lerp_pending_steps: Vec<ProtocolMinecartStep>,
    #[serde(default)]
    pub minecart_lerp_steps: Vec<ProtocolMinecartStep>,
    #[serde(default)]
    pub minecart_lerp_old_step: Option<ProtocolMinecartStep>,
    #[serde(default)]
    pub minecart_lerp_delay: i32,
    /// Client-local `BaseCommandBlock.trackOutput` for command-block
    /// minecarts. `None` identifies every other entity family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minecart_command_block_track_output: Option<bool>,
    #[serde(default)]
    pub hurting_projectile: Option<HurtingProjectileState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile_lifecycle: Option<ProjectileLifecycleState>,
}

fn bool_is_false(value: &bool) -> bool {
    !*value
}

fn u32_is_zero(value: &u32) -> bool {
    *value == 0
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HurtingProjectileState {
    pub acceleration_power: f64,
}

/// Client-owned state from vanilla `Projectile` and its saveable subclasses.
/// Besides the spawn-resolved parent fields, this retains the private trident
/// tick state needed to reproduce `ThrownTrident.addAdditionalSaveData`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectileLifecycleState {
    pub owner_uuid: Option<Uuid>,
    pub left_owner: bool,
    pub has_been_shot: bool,
    #[serde(default, skip_serializing_if = "u32_is_zero")]
    pub trident_in_ground_time: u32,
    #[serde(default, skip_serializing_if = "bool_is_false")]
    pub trident_dealt_damage: bool,
    #[serde(default, skip_serializing_if = "u32_is_zero")]
    pub trident_return_ticks: u32,
    /// Client-side `Entity.noPhysics` field mirror: only `ThrownTrident.tick`
    /// sets it via `setNoPhysics(true)`, and no client path ever clears it.
    /// `isAffectedByBlocks` reads this field, not the synced arrow flag.
    #[serde(default, skip_serializing_if = "bool_is_false")]
    pub arrow_no_physics: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RemovedPlayerVehicleTeleportRecovery {
    pub removed_vehicle_id: i32,
    pub player_pose: crate::LocalPlayerPoseState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TakeItemEntityPickupParticleState {
    pub item_entity_id: i32,
    pub item_entity_type_id: i32,
    pub item_position: EntityVec3,
    pub item_delta_movement: EntityVec3,
    /// The picked entity's yaw at extraction (vanilla `ItemPickupParticle` runs
    /// `extractEntity(entity, 1.0F)`; `ArrowRenderer`/`ThrownTridentRenderer`
    /// orient the projectile with `state.yRot`/`state.xRot`).
    #[serde(default)]
    pub item_y_rot: f32,
    #[serde(default)]
    pub item_x_rot: f32,
    #[serde(default)]
    pub item_age_ticks: f32,
    #[serde(default)]
    pub item_bob_offset: f32,
    #[serde(default = "entity_model_source_full_bright_light")]
    pub item_light: TerrainLight,
    pub target_entity_id: i32,
    pub target_position: EntityVec3,
    pub target_eye_height: f32,
    #[serde(default)]
    pub item_stack: Option<ProtocolItemStackSummary>,
    #[serde(default)]
    pub experience_orb_icon: Option<i32>,
    #[serde(default)]
    pub projectile_model: Option<TakeItemEntityPickupProjectileModel>,
}

/// The carried 3D model when the picked-up entity is an `AbstractArrow` /
/// thrown trident. Vanilla `ClientboundTakeItemEntity` spawns an
/// `ItemPickupParticle` for any entity kind and renders the extracted
/// `EntityRenderState` through `EntityRenderDispatcher.submit`
/// (`ItemPickupParticleGroup.State.submit`); the three picked-up families are
/// item entities (the `item_stack` field), experience orbs
/// (`experience_orb_icon`), and these projectiles. The shape mirrors bbb's
/// entity-scene kind projection (`EntityModelKind::Arrow { texture }` /
/// `EntityModelKind::Trident`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TakeItemEntityPickupProjectileModel {
    Arrow {
        /// Vanilla `TippableArrowRenderer.isTipped = getColor() > 0`
        /// (`Arrow.ID_EFFECT_COLOR`): swaps `arrow.png` for `arrow_tipped.png`.
        tipped: bool,
    },
    SpectralArrow,
    Trident {
        /// Vanilla `ThrownTrident.isFoil()` (`ID_FOIL`): the enchant glint pass.
        foil: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RavagerRoarParticleState {
    pub entity_id: i32,
    pub center: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RavagerStunParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EvokerFangsCritParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub velocity: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WitchMagicParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub bounding_box_max_y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LivingEntityPoofParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
}

/// Vanilla `LivingEntity.breakItem` inputs captured when entity events 47–52/65/68 arrive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LivingEntityItemBreakState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub eye_height: f32,
    pub y_rot: f32,
    pub x_rot: f32,
    pub item_stack: ProtocolItemStackSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LivingEntityDrownParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub delta_movement: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LivingEntityPortalParticleState {
    pub entity_id: i32,
    pub previous_position: EntityVec3,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ArrowEffectParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
    pub color_rgb: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnimalLoveParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AllayDuplicationParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityTamingParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VillagerParticleKind {
    Heart,
    Angry,
    Happy,
    Splash,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VillagerParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
    pub kind: VillagerParticleKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DolphinHappyParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoxEatParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub item_stack: ProtocolItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnowballHitParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    /// `None` matches vanilla's empty-stack `ParticleTypes.ITEM_SNOWBALL` branch;
    /// missing metadata uses `Items.SNOWBALL` as the default projectile item.
    pub item_stack: Option<ProtocolItemStackSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThrownEggHitParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    /// Missing metadata uses `Items.EGG`; explicit empty stacks do not emit particles.
    pub item_stack: ProtocolItemStackSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HoneyBlockParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub count: u32,
    pub block_state_id: i32,
}

/// One `Rabbit.spawnSprintParticle` block-particle submission for entity event 1.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RabbitSprintParticleState {
    pub entity_id: i32,
    pub block_state_id: i32,
    pub position: EntityVec3,
    pub velocity: EntityVec3,
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
    #[serde(default)]
    pub damage_invulnerable_time: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityCameraPoseState {
    pub id: i32,
    pub position: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub eye_height: f32,
}

/// The living-camera inputs consumed by vanilla `GameRenderer.bobHurt`.
///
/// This is client-observable presentation state only: packet-driven hurt yaw,
/// client-ticked hurt/death counters, and the generated LivingEntity capability.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityCameraHurtState {
    pub hurt_time: f32,
    pub hurt_duration: f32,
    pub hurt_dir: f32,
    pub death_time: f32,
    pub is_dead_or_dying: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityPickTargetState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub bounds: EntityPickBoundsState,
}

/// Narrow immutable row used to decide renderer visibility before projecting
/// the heap-heavy [`EntityModelSourceState`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityRenderCandidateState {
    pub entity_id: i32,
    pub uuid: Uuid,
    pub entity_type_id: i32,
    pub position: EntityVec3,
    /// Current, non-interpolated render position used by
    /// `EntityRenderDispatcher.distanceToSqr` during shadow extraction. For a
    /// passenger this is projected from the current root pose without mutating
    /// its packet-owned transform.
    pub current_position: EntityVec3,
    pub bounds: EntityPickBoundsState,
    pub vehicle_id: Option<i32>,
    pub leash_holder_id: Option<i32>,
    pub fishing_hook_owner_id: Option<i32>,
    pub is_spectator: bool,
    pub is_invisible: bool,
    pub is_sleeping: bool,
    pub wears_dragon_head: bool,
    pub display_culling: Option<DisplayEntityCullingState>,
}

/// Per-frame world state consumed by vanilla `FishingHookRenderer.extractRenderState`.
///
/// The hook and owner transforms are sampled at one partial tick so native can derive
/// the renderer's hand-to-hook offset without mixing generations. Item handedness,
/// crouching, scale, eye height, and attack animation all remain owned by the canonical
/// entity store.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FishingHookRenderSourceState {
    pub hook_position: EntityVec3,
    pub owner_id: i32,
    pub owner_eye_position: EntityVec3,
    pub owner_body_y_rot: f32,
    pub owner_scale: f32,
    pub owner_is_crouching: bool,
    pub holding_arm_left: bool,
    pub owner_attack_anim: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MinecartRailShape {
    NorthSouth,
    EastWest,
    AscendingEast,
    AscendingWest,
    AscendingNorth,
    AscendingSouth,
    SouthEast,
    SouthWest,
    NorthWest,
    NorthEast,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MinecartRailRenderState {
    pos_on_rail: [f32; 3],
    front_pos: [f32; 3],
    back_pos: [f32; 3],
}

impl MinecartRailShape {
    fn from_vanilla_name(name: &str) -> Option<Self> {
        match name {
            "north_south" => Some(Self::NorthSouth),
            "east_west" => Some(Self::EastWest),
            "ascending_east" => Some(Self::AscendingEast),
            "ascending_west" => Some(Self::AscendingWest),
            "ascending_north" => Some(Self::AscendingNorth),
            "ascending_south" => Some(Self::AscendingSouth),
            "south_east" => Some(Self::SouthEast),
            "south_west" => Some(Self::SouthWest),
            "north_west" => Some(Self::NorthWest),
            "north_east" => Some(Self::NorthEast),
            _ => None,
        }
    }

    fn is_slope(self) -> bool {
        matches!(
            self,
            Self::AscendingEast | Self::AscendingWest | Self::AscendingNorth | Self::AscendingSouth
        )
    }

    fn exits(self) -> ([i32; 3], [i32; 3]) {
        match self {
            Self::NorthSouth => ([0, 0, -1], [0, 0, 1]),
            Self::EastWest => ([-1, 0, 0], [1, 0, 0]),
            Self::AscendingEast => ([-1, -1, 0], [1, 0, 0]),
            Self::AscendingWest => ([-1, 0, 0], [1, -1, 0]),
            Self::AscendingNorth => ([0, 0, -1], [0, -1, 1]),
            Self::AscendingSouth => ([0, -1, -1], [0, 0, 1]),
            Self::SouthEast => ([0, 0, 1], [1, 0, 0]),
            Self::SouthWest => ([0, 0, 1], [-1, 0, 0]),
            Self::NorthWest => ([0, 0, -1], [-1, 0, 0]),
            Self::NorthEast => ([0, 0, -1], [1, 0, 0]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityBlockModelState {
    pub name: String,
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FallingBlockModelState {
    pub block_state_id: i32,
    pub block: EntityBlockModelState,
    /// Vanilla `MovingBlockRenderState.randomSeedPos`: the synchronized
    /// `FallingBlockEntity.DATA_START_POS`, initialized from the add-entity
    /// position by `recreateFromPacket`.
    pub random_seed_pos: BlockPos,
    /// Vanilla `MovingBlockRenderState.blockPos`: `BlockPos.containing(x,
    /// boundingBox.maxY, z)` at extraction time. Biome tint and moving-block
    /// light lookup use this position, independently of `random_seed_pos`.
    pub block_pos: BlockPos,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinecartDisplayBlockState {
    pub block: EntityBlockModelState,
    pub display_offset: i32,
}

/// Generates the [`EntityModelSourceState`] struct from one `pub $name: $ty`
/// declaration per field, forwarding each field's doc comments and
/// `#[serde(...)]` attributes verbatim so serialization stays byte-identical and
/// other crates keep plain `pub` field access. Adding a projected source field is
/// a single declaration here instead of a struct field plus a matching serde
/// default. Fields carry `pub` in the declaration so the invocation reads like the
/// struct body and the byte-for-byte serde forwarding is obvious; `macro_rules!`
/// forwards the attributes but cannot itself synthesize the field names, so each
/// field is spelled out once.
///
/// # Adding a projected entity field
/// A field that flows world -> native -> renderer needs one declaration per crate
/// layer, because bbb-world and bbb-renderer are independent and bbb-native depends
/// on both, so no single shared declaration is possible:
/// 1. world: add one `pub <name>: <ty>` line to this macro invocation.
/// 2. native: add one `with_<name> <name>` line to the pure list in
///    `entity_render_state_passthrough!` in `bbb-native/src/entity_scene.rs`
///    (or a hand-written builder line if the value is derived).
/// 3. renderer: add one `(with_<name>) <name>: <ty> = <default>` line to the
///    `entity_render_state!` macro in `bbb-renderer/src/entity_models/instances.rs`.
macro_rules! entity_model_source_state {
    (
        $(
            $(#[$meta:meta])*
            pub $name:ident : $ty:ty
        ),* $(,)?
    ) => {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct EntityModelSourceState {
            $(
                $(#[$meta])*
                pub $name: $ty,
            )*
        }
    };
}

entity_model_source_state! {
    pub entity_id: i32,
    #[serde(default)]
    pub uuid: uuid::Uuid,
    pub entity_type_id: i32,
    /// Raw interpolated entity position used by
    /// `EntityRenderDispatcher.distanceToSqr`. Unlike `position`, this excludes
    /// passenger/new-minecart/Shulker render offsets owned by the dispatch pose.
    #[serde(default)]
    pub distance_position: EntityVec3,
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
    /// Vanilla `ArrowRenderState.shake` (`AbstractArrow.shakeTime - partialTick`):
    /// the impact wobble amount that `ArrowModel.setupAnim` converts into a root
    /// z-rotation. `0.0` for non-arrow entities and arrows that are not shaking.
    #[serde(default)]
    pub arrow_shake: f32,
    /// Vanilla `Entity.isPassenger()`: whether this entity is mounted on another
    /// entity. World animation uses it to stop passenger limb swing, and renderer
    /// projections that have a seated passenger branch consume the same fact.
    #[serde(default)]
    pub is_passenger: bool,
    #[serde(default)]
    pub age_ticks: u32,
    /// Vanilla `BoatRenderState.rowingTimeLeft` / `AbstractBoat.getRowingTime(0,
    /// partialTick)`: the left paddle swing accumulator projected from synced
    /// paddle metadata plus the controlling-passenger gate. `0.0` for non-boats
    /// and boats whose left paddle is inactive.
    #[serde(default)]
    pub boat_rowing_time_left: f32,
    /// Vanilla `BoatRenderState.rowingTimeRight` / `AbstractBoat.getRowingTime(1,
    /// partialTick)`: the right paddle swing accumulator projected from synced
    /// paddle metadata plus the controlling-passenger gate. `0.0` for non-boats
    /// and boats whose right paddle is inactive.
    #[serde(default)]
    pub boat_rowing_time_right: f32,
    /// Vanilla shared `VehicleEntity.getHurtTime() - partialTick`: positive while a boat or
    /// minecart rolls from recent damage. The field keeps its historical `boat_*` name because
    /// boats were the first renderer consumer.
    #[serde(default)]
    pub boat_hurt_time: f32,
    /// Vanilla shared `VehicleEntity.getHurtDir()`: boat/minecart damage roll direction,
    /// defaulting to `1`.
    #[serde(default = "entity_model_source_default_one_i32")]
    pub boat_hurt_dir: i32,
    /// Vanilla shared `max(VehicleEntity.getDamage() - partialTick, 0)`: boat/minecart damage
    /// magnitude used with hurt time to scale the roll.
    #[serde(default)]
    pub boat_damage_time: f32,
    /// True after a tracked `ClientboundMoveMinecartPacket` gives this cart new-behavior
    /// `MinecartStep` data. While the packet's 3-tick interpolation is active, the
    /// renderer source position/rotations are projected with vanilla
    /// `NewMinecartBehavior.getCartLerpPosition` / `getCartLerp*Rot`.
    #[serde(default)]
    pub minecart_new_render: bool,
    /// Vanilla `MinecartRenderState.posOnRail`, projected by `OldMinecartBehavior.getPos`
    /// from the current world rail block. `None` keeps the old-render no-rail fallback.
    #[serde(default)]
    pub minecart_pos_on_rail: Option<[f32; 3]>,
    /// Vanilla `MinecartRenderState.frontPos`, projected by `OldMinecartBehavior.getPosOffs(...,
    /// 0.3F)` or falling back to `posOnRail`.
    #[serde(default)]
    pub minecart_front_pos: Option<[f32; 3]>,
    /// Vanilla `MinecartRenderState.backPos`, projected by `OldMinecartBehavior.getPosOffs(...,
    /// -0.3F)` or falling back to `posOnRail`.
    #[serde(default)]
    pub minecart_back_pos: Option<[f32; 3]>,
    /// Vanilla `MinecartTntRenderState.fuseRemainingInTicks`: `MinecartTNT.getFuse() - partialTick
    /// + 1.0`, or `-1.0` when the TNT minecart is not primed. Drives the TNT display block's
    /// renderer-owned scale pulse and white overlay.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub minecart_tnt_fuse_remaining_in_ticks: f32,
    /// Vanilla `BoatRenderState.bubbleAngle` (`AbstractBoat.getBubbleAngle(partialTick)`):
    /// the bubble-column wobble angle in degrees, before the renderer's underwater gate.
    #[serde(default)]
    pub boat_bubble_angle: f32,
    /// Vanilla `BoatRenderState.isUnderWater` (`AbstractBoat.isUnderWater()`): true
    /// when the water surface is strictly above the boat AABB top. It gates the
    /// bubble-column wobble and the above-water water-mask submission.
    #[serde(default)]
    pub boat_underwater: bool,
    /// Vanilla `LivingEntityRenderState.isFullyFrozen` (`Entity.isFullyFrozen`,
    /// `ticksFrozen >= 140`): a living entity frozen solid in powder snow, whose
    /// body the renderer shakes (`LivingEntityRenderer.isShaking`). `false` for
    /// non-living entities and any entity that is not frozen solid.
    #[serde(default)]
    pub is_fully_frozen: bool,
    /// Vanilla `LivingEntityRenderState.isInvisibleToPlayer`: `state.isInvisible
    /// && entity.isInvisibleTo(minecraft.player)`. This is `false` for spectator
    /// viewers (and later same-team viewers that can see friendly invisibles);
    /// deserialized legacy rows default to `true` to preserve the old hidden
    /// invisible-entity behavior when paired with an invisible shared flag.
    #[serde(default = "entity_model_source_default_true")]
    pub invisible_to_player: bool,
    /// Vanilla `EntityRenderState.appearsGlowing()` / client-side
    /// `Entity.isCurrentlyGlowing()`: synced shared-flags bit 6. This lets
    /// `LivingEntityRenderer.getRenderType` use the outline render type for an
    /// invisible entity that is otherwise hidden from this client.
    #[serde(default)]
    pub appears_glowing: bool,
    /// Vanilla `EntityRenderState.outlineColor`: `0` when the entity does not
    /// appear glowing, otherwise `ARGB.opaque(entity.getTeamColor())`. Team
    /// color uses the scoreboard member keyed by `Entity.getScoreboardName()`
    /// (UUID string for ordinary entities, GameProfile name for players);
    /// missing/reset team color falls back to opaque white.
    #[serde(default)]
    pub outline_color: u32,
    /// Vanilla `Mob.isAggressive()` (`DATA_MOB_FLAGS_ID & 4`): whether the mob is in its
    /// aggressive AI state. Renderer-owned extraction projects it for zombie, skeleton, and
    /// illager render states; bbb also resolves the regular piglin/brute arm pose from this flag.
    /// `false` for entity render paths that do not consume the synced Mob bit.
    #[serde(default)]
    pub is_aggressive: bool,
    /// Vanilla `EvokerRenderState` / `IllusionerRenderState.isCastingSpell` from
    /// `SpellcasterIllager.isCastingSpell()` (synced `DATA_SPELL_CASTING_ID` byte > 0). `false` for
    /// idle spellcasters and every renderer that does not extract this field.
    #[serde(default)]
    pub illager_spellcasting: bool,
    /// Vanilla `VillagerRenderState.isUnhappy` (`AbstractVillager.getUnhappyCounter() > 0`, synced INT id
    /// 18), consumed by `VillagerModel.setupAnim` to shake the villager-family head. `false` for happy
    /// villagers/traders and every non-villager entity.
    #[serde(default)]
    pub villager_unhappy: bool,
    /// Vanilla `EndermanRenderState.carriedBlock` non-empty (`Enderman.getCarriedBlock`
    /// present, the synced `DATA_CARRY_STATE`): the enderman is holding a block, which
    /// `EndermanModel.setupAnim` poses both arms forward to carry (`xRot = -0.5`, `zRot =
    /// ±0.05`). Projected only for the entity carrying the generated EnderMan render accessors;
    /// `false` for every other entity.
    #[serde(default)]
    pub enderman_carrying: bool,
    /// Vanilla `EndermanRenderState.carriedBlock`: the concrete block state carried by the
    /// enderman, resolved from synced `DATA_CARRY_STATE` through the vanilla block-state registry so
    /// native can bake the matching block model. `None` for no carried block, every non-enderman, or an
    /// unknown block-state id.
    #[serde(default)]
    pub enderman_carried_block: Option<EntityBlockModelState>,
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
    /// for the entity carrying the generated Bat flags accessor; `false` for every other entity.
    #[serde(default)]
    pub bat_resting: bool,
    /// Vanilla `BatRenderState.flyAnimationState`: elapsed seconds since the current flying run
    /// started. `-1.0` before the first flying tick, while resting, and for every non-bat entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub bat_flying_seconds: f32,
    /// Vanilla `BeeRenderState.hasStinger` (`!Bee.hasStung()`, the synced `DATA_FLAGS_ID & 4`):
    /// whether the bee still carries its stinger, which `BeeModel.setupAnim` toggles as the
    /// stinger cube's `visible` flag. Projected only for the entity carrying the generated Bee
    /// flags accessor and defaults to `true` (a bee that has not stung still has its stinger;
    /// every other entity is unaffected).
    #[serde(default = "entity_model_source_default_true")]
    pub bee_has_stinger: bool,
    /// Vanilla `BeeRenderState.rollAmount` (`Bee.getRollAmount(partialTick)`, the lerped client
    /// accumulator driven by the synced `DATA_FLAGS_ID & 2` roll flag): a rolling bee tips onto its
    /// back, which `BeeModel.setupAnim` applies as a near-π `bone.xRot` flip. Projected only for the
    /// bee and `0.0` (upright) for every other entity.
    #[serde(default)]
    pub bee_roll_amount: f32,
    /// Vanilla `PandaRenderState.sitAmount` (`Panda.getSitAmount(partialTick)`): the 0..1 eased
    /// sitting amount that both `PandaRenderer.setupRotations` and `PandaModel.setupAnim` consume.
    /// Projected only for the panda and `0.0` for every other entity.
    #[serde(default)]
    pub panda_sit_amount: f32,
    /// Vanilla `PandaRenderState.lieOnBackAmount` (`Panda.getLieOnBackAmount(partialTick)`): the
    /// 0..1 eased back-lying amount used by the panda root transform and head/limb pose. Projected
    /// only for the panda and `0.0` otherwise.
    #[serde(default)]
    pub panda_lie_on_back_amount: f32,
    /// Vanilla `PandaRenderState.rollAmount` (`Panda.getRollAmount(partialTick)`, but forced to
    /// `0.0` for baby pandas by `PandaRenderer.extractRenderState`): the adult model rolling pose
    /// blend. Projected only for adult pandas and `0.0` otherwise.
    #[serde(default)]
    pub panda_roll_amount: f32,
    /// Vanilla `PandaRenderState.rollTime = rollCounter > 0 ? rollCounter + partialTick : 0.0`:
    /// whole-model tumble timing consumed by `PandaRenderer.setupRotations`. Projected for adult and
    /// baby pandas; `0.0` outside an active roll and for every other entity.
    #[serde(default)]
    pub panda_roll_time: f32,
    /// Vanilla frog croak timing (`FrogRenderState.croakAnimationState`, the triggered
    /// `AnimationState` started/stopped by the synced `Pose.CROAKING`): the elapsed seconds since the
    /// croak started, which `FrogModel.setupAnim` uses to show the `croaking_body` pouch and sample
    /// the `FrogAnimation.FROG_CROAK` animation. Projected only for the frog and `-1.0` (the
    /// stopped-animation sentinel, so the pouch stays hidden) for a non-croaking frog and every other
    /// entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub frog_croak_seconds: f32,
    /// Vanilla frog tongue timing (`FrogRenderState.tongueAnimationState`, the triggered
    /// `AnimationState` started/stopped by the synced `Pose.USING_TONGUE`): the elapsed seconds since
    /// the tongue lash started, which `FrogModel.setupAnim` uses to dip the head and z-scale the
    /// `tongue` part forward (the `FrogAnimation.FROG_TONGUE` animation). Projected only for the frog
    /// and `-1.0` (the stopped-animation sentinel, so no keyframe is applied) for a frog not using its
    /// tongue and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub frog_tongue_seconds: f32,
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
    /// Vanilla camel dash timing (`CamelRenderState.dashAnimationState`, the triggered `AnimationState`
    /// `Camel.setupAnimationStates` drives via `animateWhen(isDashing(), tickCount)`): the elapsed
    /// seconds since the dash started, which `CamelModel.setupAnim` uses to sample the looping
    /// `CamelAnimation.CAMEL_DASH` gallop. Projected only for the camel and `-1.0` (the
    /// stopped-animation sentinel) for a non-dashing camel and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub camel_dash_seconds: f32,
    /// Vanilla camel idle timing (`CamelRenderState.idleAnimationState`): elapsed seconds since
    /// `Camel.setupAnimationStates` last restarted the non-looping 4.0 s `CAMEL_IDLE` keyframe from
    /// its `random.nextInt(40) + 80` timeout. Projected only for a ticking camel and `-1.0` for
    /// every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub camel_idle_seconds: f32,
    /// Vanilla copper golem idle head-spin timing (`CopperGolemRenderState.idleAnimationState`):
    /// elapsed seconds since the delayed `idleAnimationStartTick` fired while
    /// `COPPER_GOLEM_STATE == IDLE`. Projected only for copper golems and `-1.0` for a stopped idle
    /// animation, every interaction state, and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub copper_golem_idle_seconds: f32,
    /// Vanilla copper golem chest interaction timing for `CopperGolemState.GETTING_ITEM`, which
    /// drives `CopperGolemAnimation.COPPER_GOLEM_CHEST_INTERACTION_NOITEM_GET`. `-1.0` while stopped
    /// and for every non-copper golem.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub copper_golem_get_item_seconds: f32,
    /// Vanilla copper golem chest interaction timing for `CopperGolemState.GETTING_NO_ITEM`, which
    /// drives `CopperGolemAnimation.COPPER_GOLEM_CHEST_INTERACTION_NOITEM_NOGET`. `-1.0` while
    /// stopped and for every non-copper golem.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub copper_golem_get_no_item_seconds: f32,
    /// Vanilla copper golem chest interaction timing for `CopperGolemState.DROPPING_ITEM`, which
    /// drives `CopperGolemAnimation.COPPER_GOLEM_CHEST_INTERACTION_ITEM_DROP`. `-1.0` while stopped
    /// and for every non-copper golem.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub copper_golem_drop_item_seconds: f32,
    /// Vanilla copper golem chest interaction timing for `CopperGolemState.DROPPING_NO_ITEM`, which
    /// drives `CopperGolemAnimation.COPPER_GOLEM_CHEST_INTERACTION_ITEM_NODROP`. `-1.0` while
    /// stopped and for every non-copper golem.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub copper_golem_drop_no_item_seconds: f32,
    /// Vanilla `CamelRenderState.jumpCooldown` (`max(Camel.getJumpCooldown() - partialTicks, 0)`):
    /// a post-dash cooldown counter that adds extra upward head pitch in `CamelModel.applyHeadRotation`.
    /// Projected only for the camel and `0.0` for every other entity or an expired cooldown.
    #[serde(default)]
    pub camel_jump_cooldown: f32,
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
    /// Vanilla `SnifferRenderState.isSearching` (`Sniffer.isSearching()`, the synced `DATA_STATE` ==
    /// `SEARCHING`): gates `SnifferModel.setupAnim`'s swap of the base `SNIFFER_WALK` for the looping
    /// `SNIFFER_SNIFF_SEARCH` search-walk. `false` for every non-searching sniffer and every other
    /// entity.
    #[serde(default)]
    pub sniffer_is_searching: bool,
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
    /// Vanilla armadillo peek timing (`Armadillo.peekAnimationState`): the elapsed seconds for
    /// `ARMADILLO_PEEK`, including the first SCARED setup tick's `fastForward(50, 1.0F)` baseline
    /// and entity event `64` restart. `-1.0` when no peek is running and for every other entity.
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
    /// `false` for every other entity. The renderer root transform applies the body-pitch flip.
    #[serde(default)]
    pub fox_is_pouncing: bool,
    /// Vanilla `FoxRenderState.isFaceplanted` (`Fox.isFaceplanted()`, the synced `DATA_FLAGS_ID & 64`):
    /// a face-planted fox, whose `FoxModel.setupAnim` twitches all four legs. Projected only for the
    /// fox; `false` for every other entity. The renderer root transform applies the body-pitch flip.
    #[serde(default)]
    pub fox_is_faceplanted: bool,
    /// Vanilla `FelineRenderState.isCrouching` (`Entity.isCrouching()`, pose `CROUCHING`): a stalking
    /// cat or ocelot lowers the body/head and switches the lower-tail walk wobble amplitude.
    /// `false` for non-felines and upright felines.
    #[serde(default)]
    pub feline_is_crouching: bool,
    /// Vanilla `FelineRenderState.isSprinting` (`Entity.isSprinting()`, shared flags bit 3): a sprinting
    /// cat or ocelot uses the sprint leg phase offsets and lower-tail amplitude. `false` for non-felines
    /// and non-sprinting felines.
    #[serde(default)]
    pub feline_is_sprinting: bool,
    /// Vanilla `CatRenderer.extractRenderState` projects `Cat.isInSittingPose()` into
    /// `FelineRenderState.isSitting`; `OcelotRenderer` never writes the field. The synced source is
    /// `TamableAnimal.DATA_FLAGS_ID` id 18 bit 0. Projected only for cats and `false` for ocelots and
    /// every other entity.
    #[serde(default)]
    pub feline_is_sitting: bool,
    /// Vanilla `CatRenderer.extractRenderState` projects `Cat.getLieDownAmount(partialTick)` into
    /// `FelineRenderState.lieDownAmount`; `OcelotRenderer` leaves it `0.0`. Projected only for cats.
    #[serde(default)]
    pub feline_lie_down_amount: f32,
    /// Vanilla `CatRenderer.extractRenderState` projects `Cat.getLieDownAmountTail(partialTick)` for
    /// the lying tail pose. Projected only for cats and `0.0` otherwise.
    #[serde(default)]
    pub feline_lie_down_amount_tail: f32,
    /// Vanilla `CatRenderer.extractRenderState` projects `Cat.getRelaxStateOneAmount(partialTick)` for
    /// the relaxed head pitch. Projected only for cats and `0.0` otherwise.
    #[serde(default)]
    pub feline_relax_state_one_amount: f32,
    /// Vanilla `CatRenderer.extractRenderState` projects `Cat.isLyingOnTopOfSleepingPlayer()`, which
    /// `Cat.handleLieDown` sets while a lying cat has a sleeping player inside
    /// `new AABB(cat.blockPosition()).inflate(2)`. This gates the extra `setupRotations`
    /// x-translate after the lie-down roll. Projected only for cats.
    #[serde(default)]
    pub feline_is_lying_on_top_of_sleeping_player: bool,
    /// Vanilla `WolfRenderState.wetShade` (`Wolf.getWetShade(partialTick)`): the base-model
    /// grayscale tint multiplier returned by `WolfRenderer.getModelTint`. A wet wolf starts at
    /// `0.75`, then brightens with the shake/drying timer; `1.0` for a dry wolf and every non-wolf.
    #[serde(default = "entity_model_source_default_scale")]
    pub wolf_wet_shade: f32,
    /// Vanilla `WolfRenderState.shakeAnim` (`Wolf.getShakeAnim(partialTick)`): the same
    /// partial-lerped water-shake timer, used by `WolfRenderState.getBodyRollAngle(offset)` for
    /// the body / mane / tail roll. `0.0` for a dry wolf and every non-wolf.
    #[serde(default)]
    pub wolf_shake_anim: f32,
    /// Vanilla `WolfRenderState.headRollAngle` (`Wolf.getHeadRollAngle(partialTick)`): the
    /// interested/begging head tilt, reconstructed from the synced `DATA_INTERESTED_ID` client
    /// accumulator and scaled by `0.15π`. `0.0` for a level-headed wolf and every non-wolf.
    #[serde(default)]
    pub wolf_head_roll_angle: f32,
    /// Vanilla `WolfRenderState.bodyArmorItem`: an adult wolf body equipment item whose equipment asset
    /// has a `wolf_body` layer. Baby wolves skip it because `WolfArmorLayer` renders only the adult
    /// `ModelLayers.WOLF_ARMOR` model.
    #[serde(default)]
    pub wolf_body_equipment_asset: Option<EquipmentAssetId>,
    /// Legacy vanilla-material projection retained until the generic equipment
    /// catalog consumer owns the wolf armor texture.
    #[serde(default)]
    pub wolf_body_armor: Option<ArmorMaterialKind>,
    /// Vanilla `ArmorTrim.layerAssetId` for the adult wolf's `WOLF_BODY` equipment layer. The
    /// resolved sprite uses `trims/entity/wolf_body`; native binds it from the startup trim atlas.
    #[serde(default)]
    pub wolf_body_armor_trim: Option<ArmorTrimSprite>,
    /// Vanilla `DyedItemColor.getOrDefault` for the wolf body armor item: a packed RGB dye. The
    /// `armadillo_scute_overlay` wolf-body equipment layer renders only when this is present.
    #[serde(default)]
    pub wolf_body_armor_dye: Option<i32>,
    /// Vanilla `Crackiness.WOLF_ARMOR.byDamage(bodyArmorItem)`: the damage crack overlay tier for
    /// wolf armor. `None` draws no crack texture.
    #[serde(default)]
    pub wolf_body_armor_crackiness: WolfArmorCrackiness,
    /// Vanilla `ItemStack.hasFoil()` for `WolfRenderState.bodyArmorItem`. Equipment rendering emits the
    /// armor glint submission immediately after the first non-empty wolf body armor layer, then disables
    /// foil for later layers.
    #[serde(default)]
    pub wolf_body_armor_foil: bool,
    /// Vanilla `VexRenderState.isCharging` (`Vex.isCharging`, the synced `DATA_FLAGS_ID & 1`):
    /// the vex is charging an attack, which `VexModel.setupAnim` shows by leveling the body
    /// (`xRot = 0`) and raising both arms (`setArmsCharging`). Projected only for the entity
    /// carrying the generated Vex flags accessor; `false` for every other entity.
    #[serde(default)]
    pub vex_charging: bool,
    /// Vanilla `WitherRenderState.invulnerableTicks` (`WitherBoss.getInvulnerableTicks`, the synced
    /// `DATA_ID_INV` spawn countdown, lerped `invulnerableTicks - partialTicks`): the wither's
    /// spawn-charge progress. `WitherBossRenderer.scale` shrinks the model by
    /// `invulnerableTicks / 220 * 0.5` off the base `2.0` scale, and `getTextureLocation` swaps in
    /// `wither_invulnerable.png` (flickering every 5 ticks once `<= 80`). Projected only for the
    /// entity carrying the generated WitherBoss accessor; `0.0` for every other entity.
    #[serde(default)]
    pub wither_invulnerable_ticks: f32,
    /// Vanilla `WitherRenderState.xHeadRots`, copied from `WitherBoss.xRotHeads`
    /// for the two side heads `[right_head, left_head]`. Projected only for the
    /// wither; `[0.0; 2]` for every other entity.
    #[serde(default)]
    pub wither_x_head_rots: [f32; 2],
    /// Vanilla `WitherRenderState.yHeadRots`, copied from `WitherBoss.yRotHeads`
    /// for the two side heads `[right_head, left_head]`. The model applies these
    /// as `yHeadRot - bodyRot`; `[0.0; 2]` for every non-wither.
    #[serde(default)]
    pub wither_y_head_rots: [f32; 2],
    /// Vanilla `LivingEntityRenderState.isCrouching` (`Entity.isCrouching`, the synced
    /// `Pose.CROUCHING`): a crouching player or mannequin, whose `HumanoidModel.setupAnim`
    /// leans the body, drops the head, tucks the legs and tilts the arms. `false` for every
    /// non-Avatar entity.
    #[serde(default)]
    pub is_crouching: bool,
    /// Vanilla `HumanoidRenderState.isFallFlying`, sourced from the shared
    /// fall-flying entity flag. Humanoid head animation and the avatar renderer
    /// consume it independently of whether a WINGS equipment layer is visible.
    #[serde(default)]
    pub is_fall_flying: bool,
    /// Vanilla `HumanoidRenderState.isVisuallySwimming`: true for pose SWIMMING,
    /// or for pose FALL_FLYING while the actual fall-flying flag is clear.
    #[serde(default)]
    pub is_visually_swimming: bool,
    /// Vanilla `HumanoidRenderState.elytraRotX`, sampled from
    /// `LivingEntity.elytraAnimationState.getRotX(partialTick)`. The renderer consumes
    /// it only for humanoid WINGS layers; non-humanoid source rows keep the default.
    #[serde(default = "entity_model_source_default_elytra_rot_x")]
    pub elytra_rot_x: f32,
    /// Vanilla `HumanoidRenderState.elytraRotY`, sampled from
    /// `LivingEntity.elytraAnimationState.getRotY(partialTick)`.
    #[serde(default)]
    pub elytra_rot_y: f32,
    /// Vanilla `HumanoidRenderState.elytraRotZ`, sampled from
    /// `LivingEntity.elytraAnimationState.getRotZ(partialTick)`. The right wing
    /// mirrors this value in `ElytraModel.setupAnim`.
    #[serde(default = "entity_model_source_default_elytra_rot_z")]
    pub elytra_rot_z: f32,
    /// Vanilla `AvatarRenderState.capeFlap`, produced by `AvatarRenderer.extractCapeState` from
    /// the lerped cloak lag plus walk bob. `0.0` for non-player entities and for a player before
    /// the first client cloak tick.
    #[serde(default)]
    pub player_cape_flap: f32,
    /// Vanilla `AvatarRenderState.capeLean`, the forward/backward cloak lag component clamped to
    /// `0..=150` and suppressed while fall-flying. `0.0` for non-player entities.
    #[serde(default)]
    pub player_cape_lean: f32,
    /// Vanilla `AvatarRenderState.capeLean2`, the side-to-side cloak lag component clamped to
    /// `-20..=20`. `0.0` for non-player entities.
    #[serde(default)]
    pub player_cape_lean2: f32,
    /// Vanilla `AvatarRenderState.fallFlyingTimeInTicks` = the client tick
    /// counter plus the current partial tick. `0.0` for non-player entities.
    #[serde(default)]
    pub player_fall_flying_time_in_ticks: f32,
    /// Whether `AvatarRenderer.extractFlightData` found non-trivial horizontal
    /// movement and look vectors and therefore applies [`player_flying_y_rot`].
    #[serde(default)]
    pub player_should_apply_flying_y_rot: bool,
    /// Vanilla signed horizontal angle between the player's interpolated
    /// movement and look vector, in radians.
    #[serde(default)]
    pub player_flying_y_rot: f32,
    /// Vanilla `AvatarRenderState.arrowCount`, read from LivingEntity's generated synchronized
    /// `DATA_ARROW_COUNT_ID`. Only player render sources retain it because `ArrowLayer` is registered
    /// by `AvatarRenderer`; non-player living entities keep zero.
    #[serde(default)]
    pub player_arrow_count: i32,
    /// Vanilla `AvatarRenderState.stingerCount`, read from LivingEntity's generated synchronized
    /// `DATA_STINGER_COUNT_ID`. Only player render sources retain it because `BeeStingerLayer` is
    /// registered by `AvatarRenderer`; non-player living entities keep zero.
    #[serde(default)]
    pub player_stinger_count: i32,
    /// Vanilla `AvatarRenderState.parrotOnLeftShoulder`, decoded from Player metadata id 19
    /// (`DATA_SHOULDER_PARROT_LEFT`, `OPTIONAL_UNSIGNED_INT`): the parrot variant id placed on the
    /// left shoulder, or `None` when that shoulder is empty. Non-player entities always project
    /// `None`.
    #[serde(default)]
    pub player_left_shoulder_parrot: Option<i32>,
    /// Vanilla `AvatarRenderState.parrotOnRightShoulder`, decoded from Player metadata id 20
    /// (`DATA_SHOULDER_PARROT_RIGHT`, `OPTIONAL_UNSIGNED_INT`): the mirrored right-shoulder parrot
    /// variant id, or `None`.
    #[serde(default)]
    pub player_right_shoulder_parrot: Option<i32>,
    /// Vanilla `AvatarRenderState.showExtraEars`: the exact lowercase `deadmau5`
    /// profile-name easter egg from `AbstractClientPlayer.showExtraEars`. Projected
    /// only for real player entities; mannequins and non-players stay false.
    #[serde(default)]
    pub show_extra_ears: bool,
    /// Vanilla `LivingEntityRenderState.isAutoSpinAttack`
    /// (`LivingEntity.isAutoSpinAttack`, `DATA_LIVING_ENTITY_FLAGS & 4`): a living
    /// entity mid riptide-trident spin, which the renderer flips onto the spin
    /// axis in `LivingEntityRenderer.setupRotations`. `false` for non-living
    /// entities and any living entity that is not spinning.
    #[serde(default)]
    pub is_auto_spin_attack: bool,
    /// Vanilla `LivingEntity.isUsingItem()` (`DATA_LIVING_ENTITY_FLAGS & 1`): a living entity actively
    /// using/holding-right-click an item, which drives `HumanoidModel.setupAnim`'s use-item arm poses
    /// (spyglass to the eye, horn to the mouth, …). `false` for non-living entities and any not using.
    #[serde(default)]
    pub is_using_item: bool,
    /// Vanilla `LivingEntity.getUsedItemHand()` (`DATA_LIVING_ENTITY_FLAGS & 2` → off hand): which hand
    /// holds the item being used, so the renderer poses the correct arm. Only meaningful while
    /// [`is_using_item`](Self::is_using_item); `false` (main hand) otherwise.
    #[serde(default)]
    pub use_item_off_hand: bool,
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
    /// Vanilla `EntityRenderState.boundingBoxWidth` (`Entity.getBbWidth`): the
    /// current AABB width in world units. `FlameFeatureRenderer` multiplies this
    /// by `1.4` to size the dispatcher-owned fire layers.
    #[serde(default)]
    pub bounding_box_width: f32,
    /// Vanilla `EntityRenderState.displayFireAnimation`, extracted from
    /// `Entity.displayFireAnimation()`. This applies the synchronized on-fire
    /// flag and generated `EntityType.fireImmune` catalog; spectator players are
    /// removed before model-source projection.
    #[serde(default)]
    pub display_fire_animation: bool,
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
    /// Vanilla `LivingEntityRenderState.ageScale` (`LivingEntity.getAgeScale`), used only by model
    /// `setupAnim` proportions. This is independent of the SCALE attribute-backed [`scale`](Self::scale):
    /// adults and non-living entities use `1.0`, while baby values follow the generated class override.
    #[serde(default = "entity_model_source_default_scale")]
    pub age_scale: f32,
    /// Vanilla `LivingEntityRenderState.swimAmount` (`LivingEntity.getSwimAmount(partialTick)`): the
    /// eased 0..1 blend toward `Entity.isVisuallySwimming()`. The drowned render path currently
    /// consumes it for the body pitch and limb swim overrides; `0.0` for dry/non-swimming entities.
    #[serde(default)]
    pub swim_amount: f32,
    /// Vanilla `HumanoidRenderState.speedValue`: fall-flying squared velocity
    /// normalized by `/ 0.2`, then cubed and clamped to at least `1.0`. It slows
    /// humanoid walk swing while moving quickly through the air.
    #[serde(default = "entity_model_source_default_scale")]
    pub humanoid_speed_value: f32,
    #[serde(default)]
    pub sheep_eat_animation_tick: i32,
    /// Vanilla `Goat.lowerHeadTick` (the `0..=20` ram counter, advanced from entity events 58/59): the
    /// native layer scales it by the adult/baby max head pitch to derive `getRammingXHeadRot`. `0` for a
    /// goat not ramming and every non-goat.
    #[serde(default)]
    pub goat_lower_head_tick: i32,
    /// Vanilla `IronGolemRenderState.attackTicksRemaining` (the partial-lerped `attackAnimationTick`,
    /// from entity event 4): drives `IronGolemModel.setupAnim`'s two-fisted smash arm wave. `0.0` for a
    /// golem not attacking and every non-golem.
    #[serde(default)]
    pub iron_golem_attack_ticks_remaining: f32,
    /// Vanilla `IronGolemRenderState.offerFlowerTick` (`offerFlowerTick`, from entity events 11/34): the
    /// golem holds a poppy out to a villager. `0` when not offering and for every non-golem.
    #[serde(default)]
    pub iron_golem_offer_flower_tick: i32,
    /// Vanilla `SnowGolemRenderState.headBlock`: whether the snow golem has its carved pumpkin head
    /// block. Projected from the generated `SnowGolem.DATA_PUMPKIN_ID` byte mask and default.
    /// `false` for every non-snow-golem entity.
    #[serde(default)]
    pub snow_golem_pumpkin: bool,
    /// Vanilla `RavagerRenderState.stunnedTicksRemaining` (partial-lerped `stunnedTick`, from entity
    /// event 39): the post-shield-block stun head shake. `0.0` when not stunned and for every non-ravager.
    #[serde(default)]
    pub ravager_stunned_ticks_remaining: f32,
    /// Vanilla `RavagerRenderState.attackTicksRemaining` (partial-lerped `attackTick`, from entity event
    /// 4): the bite neck lunge / mouth open. `0.0` when not attacking and for every non-ravager.
    #[serde(default)]
    pub ravager_attack_ticks_remaining: f32,
    /// Vanilla `RavagerRenderState.roarAnimation` (the `0..1` roar ramp, armed when a stun ends): the
    /// roar mouth open. `0.0` when not roaring and for every non-ravager.
    #[serde(default)]
    pub ravager_roar_animation: f32,
    /// Vanilla `HoglinRenderState.attackAnimationRemainingTicks` (the RAW headbutt timer, from entity
    /// event 4): the hoglin / zoglin head-down ram. `0` when not mid-headbutt and for every other entity.
    #[serde(default)]
    pub hoglin_attack_animation_tick: i32,
    /// Vanilla `ArmorStandRenderState.wiggle`: `level.getGameTime() - lastHit + partialTick`, reset by
    /// armor-stand entity event `32` and consumed by `ArmorStandRenderer.setupRotations` while `< 5`.
    /// Defaults to `5.0`, the first rest value, for every non-wobbling armor stand and other entity.
    #[serde(default = "entity_model_source_default_armor_stand_wiggle")]
    pub armor_stand_wiggle: f32,
    #[serde(default)]
    pub polar_bear_stand_scale: f32,
    /// Vanilla `PaintingRenderState.variant`: the synchronized or inline painting fields retained
    /// from entity metadata. `None` until a painting variant is available and for non-paintings.
    #[serde(default)]
    pub painting_variant_projection: Option<PaintingVariantClientProjection>,
    /// Vanilla `PaintingRenderState.direction`: the horizontal hanging direction resolved from
    /// synchronized metadata or the add-entity data fallback.
    #[serde(default)]
    pub painting_facing: Option<ItemFrameFacing>,
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
    /// Vanilla `EnderDragonRenderState.deathTime` (`EnderDragon.dragonDeathTime`
    /// lerped): drives the dragon-only dying dissolve body submission and death
    /// rays. `0.0` for living dragons and every non-dragon entity.
    #[serde(default)]
    pub ender_dragon_death_time: f32,
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
    /// Vanilla `ShulkerRenderState.attachFace` (`Shulker.DATA_ATTACH_FACE_ID`, default `DOWN`):
    /// the block face the shulker is attached to. `ShulkerRenderer.setupRotations` rotates the model
    /// around `(0, 0.5, 0)` by `attachFace.getOpposite().getRotation()`.
    #[serde(default)]
    pub shulker_attach_face: EntityAttachmentFace,
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
    /// Vanilla `Warden.emergeAnimationState` elapsed seconds (`Pose.EMERGING`, the 6.68s
    /// `WARDEN_EMERGE` spawn rise), sampled by `WardenModel.setupAnim`'s `emergeAnimation.apply`.
    /// `-1.0` (stopped) for a non-emerging warden and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub warden_emerge_seconds: f32,
    /// Vanilla `Warden.diggingAnimationState` elapsed seconds (`Pose.DIGGING`, the 5.0s
    /// `WARDEN_DIG` despawn burrow), sampled by `WardenModel.setupAnim`'s `diggingAnimation.apply`.
    /// `-1.0` (stopped) for a non-digging warden and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub warden_dig_seconds: f32,
    /// Vanilla `Rabbit.hopAnimationState` elapsed seconds (entity event `1`-seeded, the 0.75s looping
    /// `RabbitAnimation.HOP`), sampled by `RabbitModel.setupAnim`'s `hopAnimation.apply`. `-1.0`
    /// (stopped) for a resting rabbit and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub rabbit_hop_seconds: f32,
    /// Vanilla `Creaking.canMove()` (synced `CAN_MOVE`, default `true`): gates `CreakingModel`'s
    /// looping walk. A creaking frozen while observed turns to a statue. `true` for every entity
    /// without the flag (the field only feeds the creaking model).
    #[serde(default = "entity_model_source_default_true")]
    pub creaking_can_move: bool,
    /// Vanilla `Creaking.hasGlowingEyes()`: the client-local tearing-down eye-flicker bit. It is
    /// consumed only while synced `isTearingDown()` is true; otherwise `CreakingRenderer` uses the
    /// synced `isActive()` flag. `false` for every other entity and before teardown begins.
    #[serde(default)]
    pub creaking_eyes_glowing: bool,
    /// Vanilla `Creaking.attackAnimationState` elapsed seconds (entity event `4`-seeded, the 0.7083s
    /// looping `CREAKING_ATTACK` lunge), sampled by `CreakingModel.setupAnim`'s `attackAnimation.apply`.
    /// `-1.0` (stopped) for a non-attacking creaking and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub creaking_attack_seconds: f32,
    /// Vanilla `Creaking.invulnerabilityAnimationState` elapsed seconds (entity event `66`-seeded, the
    /// 0.2917s `CREAKING_INVULNERABLE` stagger), sampled by `CreakingModel.setupAnim`'s
    /// `invulnerableAnimation.apply`. `-1.0` (stopped) for a non-staggering creaking and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub creaking_invulnerable_seconds: f32,
    /// Vanilla `Creaking.deathAnimationState` elapsed seconds (synced `isTearingDown()`-driven, the
    /// 2.25s `CREAKING_DEATH` collapse), sampled by `CreakingModel.setupAnim`'s `deathAnimation.apply`.
    /// `-1.0` (stopped) for a non-tearing-down creaking and every other entity.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub creaking_death_seconds: f32,
    /// Vanilla `LivingEntityRenderState.walkAnimationPos`
    /// (`WalkAnimationState.position(partialTick)`): the lerped limb-swing position
    /// that sways the model's legs/arms. `0.0` for a standing entity, every
    /// non-living entity, and entities whose vanilla `updateWalkAnimation` produces no limb swing.
    #[serde(default)]
    pub walk_animation_position: f32,
    /// Vanilla `LivingEntityRenderState.walkAnimationSpeed`
    /// (`WalkAnimationState.speed(partialTick)`): the lerped limb-swing speed
    /// amplitude that scales the leg/arm sway. `0.0` for a standing entity and
    /// every non-living entity.
    #[serde(default)]
    pub walk_animation_speed: f32,
    /// Vanilla `LivingEntityRenderState.wornHeadAnimationPos`: the animation position
    /// sent to `SkullBlockRenderer.submitSkull` for worn skulls. It mirrors the
    /// entity's own walk position unless the entity rides a living entity, in which
    /// case vanilla reads the vehicle's walk position.
    #[serde(default)]
    pub worn_head_animation_position: f32,
    /// Vanilla `HumanoidRenderState.attackTime` (`LivingEntity.getAttackAnim(partialTick)`):
    /// the lerped `0..1` melee swing progress `HumanoidModel.setupAttackAnimation` feeds the
    /// body twist + arm whack. `0.0` for an entity that is not mid-swing.
    #[serde(default)]
    pub attack_anim: f32,
    /// Vanilla `ArmedEntityRenderState.mainArm`: whether the entity's main arm is
    /// left. Players read `Avatar.DATA_PLAYER_MAIN_HAND`; mobs that use
    /// `ItemInHandLayer` read `Mob.MOB_FLAG_LEFTHANDED`. `false` is vanilla's
    /// right-handed default.
    #[serde(default)]
    pub main_arm_left: bool,
    /// Vanilla `HumanoidRenderState.attackArm` (`LivingEntity.swingingArm`): whether the active
    /// swing is the off (left) hand. `false` for a main-hand swing and every non-swinging entity.
    #[serde(default)]
    pub attack_arm_off_hand: bool,
    /// Vanilla `LivingEntity.swinging`: whether a melee swing is in progress. Gates the player's
    /// `CROSSBOW_HOLD` arm pose (`AvatarRenderer.getArmPose`: `!swinging && crossbow && charged`), which
    /// yields to the swing while attacking. `false` for an entity that is not mid-swing.
    #[serde(default)]
    pub is_swinging: bool,
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
    /// Vanilla `LivingEntityRenderState.bodyRot` for squid (`Squid.yBodyRot`
    /// lerped, degrees): the movement-derived body yaw `SquidRenderer.setupRotations`
    /// receives before applying its squid-specific pitch/roll. `0.0` for every
    /// non-squid entity.
    #[serde(default)]
    pub squid_y_body_rot: f32,
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
    /// Vanilla `GuardianRenderState.spikesAnimation` (`Guardian.getSpikesAnimation` lerped): the
    /// spike-withdrawal phase `GuardianModel.setupAnim` turns into `withdrawal = (1 - it) · 0.55`,
    /// the per-spike inset. `1.0` (withdrawal `0`, fully extended) for an unticked guardian and every
    /// non-guardian entity.
    #[serde(default = "entity_model_source_default_scale")]
    pub guardian_spikes_animation: f32,
    /// Vanilla `Breeze`'s pose-driven action one-shots, each the elapsed seconds since it started
    /// (`Breeze.shoot`/`slide`/`slideBack`/`inhale`/`longJump`, sampled by `BreezeModel.setupAnim`),
    /// or `-1.0` (stopped) for a breeze not in that action and every non-breeze entity. The
    /// non-looping actions clamp past their length to the final frame.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub breeze_shoot_seconds: f32,
    #[serde(default = "entity_model_source_default_neg_one")]
    pub breeze_slide_seconds: f32,
    #[serde(default = "entity_model_source_default_neg_one")]
    pub breeze_slide_back_seconds: f32,
    #[serde(default = "entity_model_source_default_neg_one")]
    pub breeze_inhale_seconds: f32,
    #[serde(default = "entity_model_source_default_neg_one")]
    pub breeze_long_jump_seconds: f32,
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
    /// Vanilla `SlimeRenderState.squish` (`Slime.squish` lerped): the squish amount
    /// `SlimeRenderer.scale` turns into the slime/magma-cube body's non-uniform
    /// stretch. `0.0` (undeformed cube) at rest and for every other entity.
    #[serde(default)]
    pub slime_squish: f32,
    /// Vanilla `EvokerFangsRenderState.biteProgress` (`EvokerFangs.getAnimationProgress`):
    /// the `0..1` attack ramp `EvokerFangsModel.setupAnim` turns into the jaw snap, the
    /// rise out of the ground, and the final vanish. `0.0` (hidden) for an un-attacked
    /// fang and every other entity.
    #[serde(default)]
    pub evoker_fangs_bite_progress: f32,
    /// Vanilla `AllayModel.setupAnim`: `true` while the allay's synced `DATA_DANCING`
    /// flag is set, gating the dance pose (head tilt + body sway/spin) over the normal
    /// head-look. `false` for a non-dancing allay and every other entity.
    #[serde(default)]
    pub allay_dancing: bool,
    /// Vanilla `AllayModel`: `true` during the spin sub-window of the dance
    /// (`danceAnimation % 55 < 15`), selecting the `4π * progress` body spin. `false`
    /// otherwise and for every non-allay entity.
    #[serde(default)]
    pub allay_spinning: bool,
    /// Vanilla `AllayModel`: the `0..1` lerped spin blend (`spinningAnimation / 15`)
    /// that cross-fades the body sway into the spin. `0.0` for a non-spinning allay and
    /// every other entity.
    #[serde(default)]
    pub allay_spinning_progress: f32,
    /// Vanilla `AllayRenderState.holdingAnimationProgress`
    /// (`Allay.getHoldingItemAnimationProgress(partialTick)`): a `0..1` client-side
    /// ease-in/out driven by whether the allay has a non-empty main-hand item. The
    /// model uses it to raise and turn both arms for the held item. `0.0` for an
    /// empty-handed allay and every other entity.
    #[serde(default)]
    pub allay_holding_item_progress: f32,
    /// Vanilla render-state `ticksUsingItem` reconstructed from the synced charging / using bit
    /// (ticks since it rose, + partial). Crossbow draw poses and player spear kinetic use sway consume
    /// it with their own durations; `0.0` for anything not mid-draw/use.
    #[serde(default)]
    pub crossbow_charge_ticks: f32,
    /// Vanilla `LivingEntityRenderState.ticksSinceKineticHitFeedback`
    /// (`LivingEntity.getTicksSinceLastKineticHitFeedback(partialTicks)`): driven by
    /// entity event `2` and consumed by spear item use feedback. `0.0` before any
    /// accepted kinetic-hit event and for every non-living entity.
    #[serde(default)]
    pub ticks_since_kinetic_hit_feedback: f32,
    /// Vanilla `AxolotlRenderState.playingDeadFactor` (`Axolotl.playingDeadAnimator.getFactor`): the
    /// `0..1` eased blend `AdultAxolotlModel.setupPlayDeadAnimation` scales the limp-on-its-side pose
    /// by. `0.0` (awake) for every other entity.
    #[serde(default)]
    pub axolotl_playing_dead_factor: f32,
    /// Vanilla `AxolotlRenderState.inWaterFactor` (`Axolotl.inWaterAnimator.getFactor`): the `0..1`
    /// eased blend gating the swimming and water-hovering sub-animations. `0.0` (out of water) for
    /// every other entity.
    #[serde(default)]
    pub axolotl_in_water_factor: f32,
    /// Vanilla `AxolotlRenderState.onGroundFactor` (`Axolotl.onGroundAnimator.getFactor`): the `0..1`
    /// eased blend gating the ground-crawling and lay-still sub-animations. `0.0` for every other
    /// entity.
    #[serde(default)]
    pub axolotl_on_ground_factor: f32,
    /// Vanilla `AxolotlRenderState.movingFactor` (`Axolotl.movingAnimator.getFactor`): the `0..1`
    /// eased blend separating the moving sub-animations (swim, crawl) from the still ones and gating
    /// the mirror-leg copy. `0.0` (still) for every other entity.
    #[serde(default)]
    pub axolotl_moving_factor: f32,
    /// Vanilla baby Axolotl `AnimationState` elapsed seconds. Each field is `-1.0` while stopped;
    /// `tickBabyAnimations` keeps exactly one timer running. The renderer preserves the official
    /// 26.1 model's unusual use of `walkAnimationState` for both floor-walk definitions.
    #[serde(default = "entity_model_source_default_neg_one")]
    pub axolotl_baby_swim_seconds: f32,
    #[serde(default = "entity_model_source_default_neg_one")]
    pub axolotl_baby_walk_seconds: f32,
    #[serde(default = "entity_model_source_default_neg_one")]
    pub axolotl_baby_walk_underwater_seconds: f32,
    #[serde(default = "entity_model_source_default_neg_one")]
    pub axolotl_baby_idle_underwater_seconds: f32,
    #[serde(default = "entity_model_source_default_neg_one")]
    pub axolotl_baby_idle_underwater_on_ground_seconds: f32,
    #[serde(default = "entity_model_source_default_neg_one")]
    pub axolotl_baby_idle_on_ground_seconds: f32,
    #[serde(default = "entity_model_source_default_neg_one")]
    pub axolotl_baby_play_dead_seconds: f32,
    /// Vanilla `ParrotRenderState.flapAngle` (`ParrotRenderer.extractRenderState`:
    /// `(sin(lerp(flap)) + 1) * lerp(flapSpeed)`): the combined wing-flap angle
    /// `ParrotModel.setupAnim` feeds to the wing `zRot` (`±(0.0873 + flapAngle)`) and
    /// the body/head/tail bob (`flapAngle * 0.3`). `0.0` (wings held) for a
    /// grounded/still parrot and every non-parrot entity.
    #[serde(default)]
    pub parrot_flap_angle: f32,
    /// Vanilla `ParrotRenderState.pose == PARTY` (`Parrot.isPartyParrot()`): true
    /// while the parrot has been notified about a playing jukebox and remains
    /// within `BlockPos.closerToCenterThan(entity.position(), 3.46)`. The renderer
    /// consumes this before sitting/flying, matching `ParrotModel.getPose`.
    #[serde(default)]
    pub parrot_party: bool,
    /// Vanilla `HumanoidArmorLayer` worn armor, the equipment-asset material per armor slot, resolved
    /// from the entity's `SetEquipment` items against the item registry. `None` leaves the slot bare.
    /// Only humanoid armor-wearers carry these; the renderer drapes the matching inflated armor piece.
    #[serde(default)]
    pub head_equipment_asset: Option<EquipmentAssetId>,
    #[serde(default)]
    pub chest_equipment_asset: Option<EquipmentAssetId>,
    #[serde(default)]
    pub legs_equipment_asset: Option<EquipmentAssetId>,
    #[serde(default)]
    pub feet_equipment_asset: Option<EquipmentAssetId>,
    /// Legacy vanilla-material projections retained while humanoid geometry
    /// migrates to the generic equipment catalog.
    #[serde(default)]
    pub head_armor: Option<ArmorMaterialKind>,
    #[serde(default)]
    pub chest_armor: Option<ArmorMaterialKind>,
    #[serde(default)]
    pub legs_armor: Option<ArmorMaterialKind>,
    #[serde(default)]
    pub feet_armor: Option<ArmorMaterialKind>,
    /// Vanilla `ArmorTrim.layerAssetId` per worn humanoid armor slot. The world resolves the trim
    /// holders, equipment-asset suffix override, and humanoid/leggings atlas prefix while native
    /// resolves this resource id to the startup pack's generated armor-trim sprite handle.
    #[serde(default)]
    pub head_armor_trim: Option<ArmorTrimSprite>,
    #[serde(default)]
    pub chest_armor_trim: Option<ArmorTrimSprite>,
    #[serde(default)]
    pub legs_armor_trim: Option<ArmorTrimSprite>,
    #[serde(default)]
    pub feet_armor_trim: Option<ArmorTrimSprite>,
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
    /// Vanilla `ItemStack.hasFoil()` per worn armor slot. `EquipmentLayerRenderer` emits one
    /// `armorEntityGlint` submission immediately after the slot's first rendered armor layer.
    #[serde(default)]
    pub head_armor_foil: bool,
    #[serde(default)]
    pub chest_armor_foil: bool,
    #[serde(default)]
    pub legs_armor_foil: bool,
    #[serde(default)]
    pub feet_armor_foil: bool,
    /// Vanilla WINGS `ArmorTrim.layerAssetId`, resolved from a non-empty chest item's equipment
    /// asset. Unlike `HUMANOID_BABY`, the WINGS layer retains trims on baby elytra models.
    #[serde(default)]
    pub chest_wings_equipment_asset: Option<EquipmentAssetId>,
    #[serde(default)]
    pub chest_wings_trim: Option<ArmorTrimSprite>,
    /// Vanilla `ItemStack.hasFoil()` for a chest item whose equipment asset has a WINGS layer.
    #[serde(default)]
    pub chest_wings_foil: bool,
    /// Vanilla `PigRenderState.saddle`: true when a pig carries a non-empty saddle item in
    /// `EquipmentSlot.SADDLE`. The renderer consumes this to draw the `PIG_SADDLE` equipment layer.
    #[serde(default)]
    pub pig_saddle_equipment_asset: Option<EquipmentAssetId>,
    #[serde(default)]
    pub pig_saddle: bool,
    /// Vanilla `ArmorTrim.layerAssetId` for an adult pig's `PIG_SADDLE` equipment layer.
    #[serde(default)]
    pub pig_saddle_trim: Option<ArmorTrimSprite>,
    /// Vanilla `ItemStack.hasFoil()` for the saddle item. The equipment renderer inserts the glint
    /// immediately after the saddle base layer and before its optional trim.
    #[serde(default)]
    pub pig_saddle_foil: bool,
    /// Vanilla `EquineRenderState.saddle`: true when an adult horse/donkey/mule/skeleton-horse/
    /// zombie-horse carries a non-empty saddle item in `EquipmentSlot.SADDLE`.
    #[serde(default)]
    pub equine_saddle_equipment_asset: Option<EquipmentAssetId>,
    #[serde(default)]
    pub equine_saddle: bool,
    /// Vanilla armor-trim sprite for the family-specific equine saddle equipment layer.
    #[serde(default)]
    pub equine_saddle_trim: Option<ArmorTrimSprite>,
    /// Vanilla `ItemStack.hasFoil()` for the equine saddle item.
    #[serde(default)]
    pub equine_saddle_foil: bool,
    /// Vanilla `EquineRenderState.isRidden`: true when an equine saddle wearer has passengers, which
    /// makes `EquineSaddleModel` show the two bridle line parts.
    #[serde(default)]
    pub equine_saddle_ridden: bool,
    /// Vanilla `EquineRenderState.animateTail` (`AbstractHorse.tailCounter > 0`): true while the
    /// client-side random idle tail counter is active, driving `tail.yRot = cos(ageInTicks * 0.7)`.
    #[serde(default)]
    pub equine_animate_tail: bool,
    /// Vanilla `EquineRenderState.eatAnimation`: the partial-tick lerped `AbstractHorse.eatAnim`
    /// amount, driven by the synced eating flag and consumed by the equine head placement.
    #[serde(default)]
    pub equine_eat_animation: f32,
    /// Vanilla `EquineRenderState.standAnimation`: the partial-tick lerped `AbstractHorse.standAnim`
    /// amount, driving the rearing body, head, and leg placement.
    #[serde(default)]
    pub equine_stand_animation: f32,
    /// Vanilla `EquineRenderState.feedingAnimation`: the partial-tick lerped `AbstractHorse.mouthAnim`
    /// amount, adding the subtle open-mouth head bob while the horse is otherwise idle.
    #[serde(default)]
    pub equine_feeding_animation: f32,
    /// Vanilla `EquineRenderState.bodyArmorItem`: the horse armor material from an adult horse /
    /// zombie horse body equipment item whose equipment asset has a `horse_body` layer. Baby horses
    /// skip it because `SimpleEquipmentLayer` supplies no baby armor model; skeleton horses are
    /// excluded by the vanilla armor-wearer tag.
    #[serde(default)]
    pub equine_body_equipment_asset: Option<EquipmentAssetId>,
    #[serde(default)]
    pub equine_body_armor: Option<ArmorMaterialKind>,
    /// Vanilla `ArmorTrim.layerAssetId` for an adult horse or zombie horse's `HORSE_BODY`
    /// equipment layer. Native resolves the `trims/entity/horse_body` sprite at the atlas boundary.
    #[serde(default)]
    pub equine_body_armor_trim: Option<ArmorTrimSprite>,
    /// Vanilla `DyedItemColor` for leather horse armor in the body slot: a packed RGB dye carried
    /// alongside [`Self::equine_body_armor`]. Non-leather horse armor ignores it; undyed leather uses
    /// the vanilla leather default color in the renderer.
    #[serde(default)]
    pub equine_body_armor_dye: Option<i32>,
    /// Vanilla `ItemStack.hasFoil()` for the horse body armor item. `EquipmentLayerRenderer`
    /// inserts one armor glint immediately after the first rendered equipment layer.
    #[serde(default)]
    pub equine_body_armor_foil: bool,
    /// Vanilla `StriderRenderState.isRidden`: true when a strider has passengers, which makes
    /// `StriderModel.setupAnim` zero body pitch/yaw.
    #[serde(default)]
    pub strider_ridden: bool,
    /// Vanilla `StriderRenderState.saddle`: true when a strider carries a non-empty saddle item in
    /// `EquipmentSlot.SADDLE`. The renderer consumes this to draw the `STRIDER_SADDLE` equipment
    /// layer on adult striders.
    #[serde(default)]
    pub strider_saddle_equipment_asset: Option<EquipmentAssetId>,
    #[serde(default)]
    pub strider_saddle: bool,
    /// Vanilla `ArmorTrim.layerAssetId` for an adult strider's `STRIDER_SADDLE` equipment layer.
    #[serde(default)]
    pub strider_saddle_trim: Option<ArmorTrimSprite>,
    /// Vanilla `ItemStack.hasFoil()` for the strider saddle item.
    #[serde(default)]
    pub strider_saddle_foil: bool,
    /// Vanilla `CamelRenderState.saddle`: true when a camel/camel_husk carries a non-empty saddle item
    /// in `EquipmentSlot.SADDLE`. The renderer consumes this to draw the adult `CamelSaddleModel`
    /// equipment layer.
    #[serde(default)]
    pub camel_saddle_equipment_asset: Option<EquipmentAssetId>,
    #[serde(default)]
    pub camel_saddle: bool,
    /// Vanilla armor-trim sprite for a camel's family-specific `CAMEL*_SADDLE` equipment layer.
    #[serde(default)]
    pub camel_saddle_trim: Option<ArmorTrimSprite>,
    /// Vanilla `ItemStack.hasFoil()` for the camel saddle item.
    #[serde(default)]
    pub camel_saddle_foil: bool,
    /// Vanilla `CamelRenderState.isRidden`: true when a saddled camel/camel_husk has passengers, which
    /// makes `CamelSaddleModel` show the `reins` part.
    #[serde(default)]
    pub camel_saddle_ridden: bool,
    /// Vanilla `NautilusRenderState.saddle`: true when a living nautilus or zombie nautilus carries a
    /// non-empty saddle item in `EquipmentSlot.SADDLE`. The renderer consumes this to draw the adult
    /// `NautilusSaddleModel` equipment layer.
    #[serde(default)]
    pub nautilus_saddle_equipment_asset: Option<EquipmentAssetId>,
    #[serde(default)]
    pub nautilus_saddle: bool,
    /// Vanilla armor-trim sprite for the adult `NAUTILUS_SADDLE` equipment layer.
    #[serde(default)]
    pub nautilus_saddle_trim: Option<ArmorTrimSprite>,
    /// Vanilla `ItemStack.hasFoil()` for the nautilus saddle item.
    #[serde(default)]
    pub nautilus_saddle_foil: bool,
    /// Vanilla `HappyGhastRenderState.bodyItem`: any non-empty BODY-slot item. The base model uses
    /// this independently of asset recognition to squeeze its body subtree to 0.9375.
    #[serde(default)]
    pub happy_ghast_body_item: bool,
    /// Effective BODY-stack equipment asset, independent of the legacy
    /// vanilla harness-color classification.
    #[serde(default)]
    pub happy_ghast_body_equipment_asset: Option<EquipmentAssetId>,
    /// Recognized vanilla happy-ghast harness color derived from the BODY item's equipment asset.
    #[serde(default)]
    pub happy_ghast_harness: Option<HappyGhastHarnessColor>,
    /// Vanilla armor-trim sprite for the harness `HAPPY_GHAST_BODY` layer.
    #[serde(default)]
    pub happy_ghast_harness_trim: Option<ArmorTrimSprite>,
    /// Vanilla `ItemStack.hasFoil()` for a recognized happy-ghast harness item.
    #[serde(default)]
    pub happy_ghast_harness_foil: bool,
    /// Whether the happy ghast's current BODY item belongs to the synchronized
    /// `minecraft:harnesses` item tag. This is deliberately independent of whether bbb can resolve
    /// the item's equipment asset for the harness model.
    #[serde(default)]
    pub happy_ghast_body_item_is_harness: bool,
    /// Vanilla `HappyGhastRenderState.isLeashHolder`, copied from the generated
    /// `IS_LEASH_HOLDER` synchronized Boolean. `RopesLayer` combines this with the harness tag gate.
    #[serde(default)]
    pub happy_ghast_is_leash_holder: bool,
    /// Vanilla `HappyGhastRenderState.isRidden`: true while the happy ghast has passengers; the
    /// harness model uses it to lower the goggles into the ridden pose.
    #[serde(default)]
    pub happy_ghast_ridden: bool,
    /// Vanilla `GuardianRenderer` attack beam: present when a guardian has an active attack target.
    /// Mirrors the renderer's `GuardianBeamRenderState`; `None` for a guardian with no target and every
    /// other entity.
    #[serde(default)]
    pub guardian_beam: Option<GuardianBeamSource>,
    /// Vanilla `GuardianRenderer.extractRenderState` eye-tracking inputs. The active attack target
    /// takes priority over the current camera entity; `None` when either selected entity is absent
    /// and for every non-guardian.
    #[serde(default)]
    pub guardian_eye_tracking: Option<GuardianEyeTrackingSource>,
    /// Vanilla `EndCrystalRenderState.beamOffset`: present when an end crystal has a synced
    /// `DATA_BEAM_TARGET` block position. This is the target block center minus the crystal's
    /// interpolated position; `None` for a crystal without a target and every non-crystal.
    #[serde(default)]
    pub end_crystal_beam: Option<EndCrystalBeamSource>,
    /// Vanilla `EnderDragonRenderState.beamOffset`: present when an ender dragon has a nearest
    /// healing end crystal. This is the bobbed crystal position minus the dragon position; `None`
    /// for dragons without a tracked nearby crystal and every non-dragon.
    #[serde(default)]
    pub ender_dragon_beam: Option<EnderDragonBeamSource>,
    /// Vanilla `EntityRenderState.leashStates`: one rope per entry when this entity is leashed (one
    /// for a normal leash, four for a ghast-to-boat quad connection). Empty for unleashed entities.
    #[serde(default)]
    pub leash_ropes: Vec<LeashRopeSource>,
    /// Vanilla `EnderDragonRenderState` procedural-animation inputs (`flapTime`, phase flags, egg
    /// distance, interpolated flight history). `None` for every non-dragon.
    #[serde(default)]
    pub ender_dragon_animation: Option<EnderDragonAnimationSource>,
    /// Vanilla `LlamaRenderState.bodyItem`: the carpet color from an adult llama/trader-llama body
    /// equipment item. Baby llamas ignore body items for the decor layer; trader llamas still get their
    /// built-in trader decor in the renderer when this is `None`.
    #[serde(default)]
    pub llama_body_equipment_asset: Option<EquipmentAssetId>,
    #[serde(default)]
    pub llama_body_decor: Option<LlamaBodyDecorColor>,
    /// Vanilla `ArmorTrim.layerAssetId` for an adult carpet item's `LLAMA_BODY` equipment layer.
    /// Trader-llama built-in decor uses `ItemStack.EMPTY` and therefore has no trim.
    #[serde(default)]
    pub llama_body_decor_trim: Option<ArmorTrimSprite>,
    /// Vanilla `ItemStack.hasFoil()` for an adult llama's carpet body item. Trader-llama built-in
    /// decor and baby decor use no item foil.
    #[serde(default)]
    pub llama_body_decor_foil: bool,
    /// Vanilla `NautilusRenderState.bodyArmorItem`: an adult living nautilus / zombie nautilus body
    /// equipment item whose equipment asset has a `nautilus_body` layer. Baby living nautilus skip it
    /// because `SimpleEquipmentLayer` supplies no baby armor model.
    #[serde(default)]
    pub nautilus_body_equipment_asset: Option<EquipmentAssetId>,
    #[serde(default)]
    pub nautilus_body_armor: Option<ArmorMaterialKind>,
    /// Vanilla `ArmorTrim.layerAssetId` for the `NAUTILUS_BODY` equipment layer. Native resolves
    /// the `trims/entity/nautilus_body` sprite through the shared runtime trim atlas.
    #[serde(default)]
    pub nautilus_body_armor_trim: Option<ArmorTrimSprite>,
    /// Vanilla `ItemStack.hasFoil()` for the nautilus body armor item. The equipment renderer emits
    /// one armor glint after the base body-armor layer.
    #[serde(default)]
    pub nautilus_body_armor_foil: bool,
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

/// Vanilla `GuardianRenderState` eye-tracking projection consumed by `GuardianModel.setupAnim`.
/// Positions and the guardian view vector remain world-space doubles, matching Vanilla `Vec3` until
/// the renderer derives the eye part's two model-space offsets.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GuardianEyeTrackingSource {
    pub eye_position: [f64; 3],
    pub look_direction: [f64; 3],
    pub look_at_position: [f64; 3],
}

/// Vanilla `EndCrystalRenderer.extractRenderState` beam projection. `beam_offset` is
/// `Vec3.atCenterOf(DATA_BEAM_TARGET) - entity.getPosition(partialTicks)`; the renderer combines it
/// with `EndCrystalRenderer.getY(ageInTicks)` when submitting the dragon-healing beam.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EndCrystalBeamSource {
    pub beam_offset: [f32; 3],
}

/// Vanilla `EnderDragonRenderer.extractRenderState` beam projection. `beam_offset` is
/// `nearestCrystal.getPosition(partialTicks) + EndCrystalRenderer.getY(nearestCrystal.time +
/// partialTicks) - entity.getPosition(partialTicks)`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnderDragonBeamSource {
    pub beam_offset: [f32; 3],
}

/// Vanilla `EnderDragonRenderState`'s procedural-animation inputs for `EnderDragonModel.setupAnim`.
/// `flap_time` is the lerped `oFlapTime → flapTime` wing-beat phase; the two phase flags mirror
/// `isLandingOrTakingOff` (`LANDING`/`TAKEOFF`) and `phase.isSitting()`; `distance_to_egg` is the
/// squared center distance to the podium heightmap position (`getHeadPartYOffset`'s landing droop
/// divisor); `historical` carries `getHistoricalPos(delay)` pre-interpolated for delays `0..=23`
/// (every delay `setupAnim` samples), with the dead-dragon `partialTicks = 0` freeze applied.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnderDragonAnimationSource {
    pub flap_time: f32,
    pub is_landing_or_taking_off: bool,
    pub is_sitting: bool,
    pub distance_to_egg: f64,
    pub historical: [DragonFlightHistorySample; 24],
}

/// One rope of a leash connection, projected for the renderer from
/// [`EntityLeashRenderState`]. `offset` is the rotated attach offset (the rope start relative to
/// the entity render position) and `delta` is the world-space `end − start` span, both truncated
/// to `f32` exactly where vanilla `LeashFeatureRenderer.renderLeash` casts its doubles. The
/// endpoint block/sky lights feed the per-step light lerp and `slack` selects the sag curve over
/// the taut quad-connection line.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LeashRopeSource {
    pub offset: [f32; 3],
    pub delta: [f32; 3],
    pub start_block_light: u8,
    pub start_sky_light: u8,
    pub end_block_light: u8,
    pub end_sky_light: u8,
    pub slack: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EntityAttachmentFace {
    #[default]
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl EntityAttachmentFace {
    pub(crate) fn from_3d_data(data: i32) -> Self {
        match data.rem_euclid(6) {
            0 => Self::Down,
            1 => Self::Up,
            2 => Self::North,
            3 => Self::South,
            4 => Self::West,
            _ => Self::East,
        }
    }
}

/// Vanilla `DyeColor` carried by `Equippable.llamaSwag(color)` carpet body items. The renderer maps
/// this to `textures/entity/equipment/llama_body/<color>.png`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LlamaBodyDecorColor {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
}

/// Vanilla `DyeColor` encoded by the sixteen `EquipmentAssets.HARNESSES` body-equipment assets.
/// The renderer uses the color to select
/// `textures/entity/equipment/happy_ghast_body/<color>_harness.png`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HappyGhastHarnessColor {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
}

impl HappyGhastHarnessColor {
    pub(crate) const fn equipment_asset_id(self) -> &'static str {
        match self {
            Self::White => "minecraft:white_harness",
            Self::Orange => "minecraft:orange_harness",
            Self::Magenta => "minecraft:magenta_harness",
            Self::LightBlue => "minecraft:light_blue_harness",
            Self::Yellow => "minecraft:yellow_harness",
            Self::Lime => "minecraft:lime_harness",
            Self::Pink => "minecraft:pink_harness",
            Self::Gray => "minecraft:gray_harness",
            Self::LightGray => "minecraft:light_gray_harness",
            Self::Cyan => "minecraft:cyan_harness",
            Self::Purple => "minecraft:purple_harness",
            Self::Blue => "minecraft:blue_harness",
            Self::Brown => "minecraft:brown_harness",
            Self::Green => "minecraft:green_harness",
            Self::Red => "minecraft:red_harness",
            Self::Black => "minecraft:black_harness",
        }
    }
}

impl LlamaBodyDecorColor {
    pub(crate) const fn equipment_asset_id(self) -> &'static str {
        match self {
            Self::White => "minecraft:white_carpet",
            Self::Orange => "minecraft:orange_carpet",
            Self::Magenta => "minecraft:magenta_carpet",
            Self::LightBlue => "minecraft:light_blue_carpet",
            Self::Yellow => "minecraft:yellow_carpet",
            Self::Lime => "minecraft:lime_carpet",
            Self::Pink => "minecraft:pink_carpet",
            Self::Gray => "minecraft:gray_carpet",
            Self::LightGray => "minecraft:light_gray_carpet",
            Self::Cyan => "minecraft:cyan_carpet",
            Self::Purple => "minecraft:purple_carpet",
            Self::Blue => "minecraft:blue_carpet",
            Self::Brown => "minecraft:brown_carpet",
            Self::Green => "minecraft:green_carpet",
            Self::Red => "minecraft:red_carpet",
            Self::Black => "minecraft:black_carpet",
        }
    }
}

/// Compact identity for one selected resource generation's `equipment/*.json`
/// asset. The native resource owner replaces the complete name/id table on
/// reload, so the index is never reused without also changing `generation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EquipmentAssetId {
    pub generation: u64,
    pub index: u32,
}

impl EquipmentAssetId {
    pub const fn new(generation: u64, index: u32) -> Self {
        Self { generation, index }
    }
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
    ArmadilloScute,
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
            "armadillo_scute" => Self::ArmadilloScute,
            _ => return None,
        })
    }

    /// The vanilla equipment-asset registry key consumed by armor-trim material overrides.
    pub const fn equipment_asset_id(self) -> &'static str {
        match self {
            Self::Leather => "minecraft:leather",
            Self::Copper => "minecraft:copper",
            Self::Chainmail => "minecraft:chainmail",
            Self::Iron => "minecraft:iron",
            Self::Gold => "minecraft:gold",
            Self::Diamond => "minecraft:diamond",
            Self::TurtleScute => "minecraft:turtle_scute",
            Self::Netherite => "minecraft:netherite",
            Self::ArmadilloScute => "minecraft:armadillo_scute",
        }
    }
}

/// Vanilla `Crackiness.Level` for wolf armor (`Crackiness.WOLF_ARMOR.byDamage`): low / medium /
/// high damage cracks overlaid on the adult wolf armor model, with `None` for intact armor or any
/// body item that is not damageable client-side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WolfArmorCrackiness {
    #[default]
    None,
    Low,
    Medium,
    High,
}

/// Vanilla `EntityRenderer` light-probe full-bright fallback
/// (`LightCoordsUtil.FULL_BRIGHT` = block 15, sky 15), used when an entity's
/// chunk light is unavailable so it renders bright rather than dark, matching
/// the `EntityRenderState.lightCoords` default.
pub(crate) const ENTITY_LIGHT_PROBE_FULL_BRIGHT: TerrainLight = TerrainLight { sky: 15, block: 15 };
fn entity_model_source_full_bright_light() -> TerrainLight {
    ENTITY_LIGHT_PROBE_FULL_BRIGHT
}

/// Vanilla `ExperienceOrb.getIcon()`: the 4×4 sheet cell picked from the synced orb value.
pub fn experience_orb_icon(value: i32) -> i32 {
    if value >= 2477 {
        10
    } else if value >= 1237 {
        9
    } else if value >= 617 {
        8
    } else if value >= 307 {
        7
    } else if value >= 149 {
        6
    } else if value >= 73 {
        5
    } else if value >= 37 {
        4
    } else if value >= 17 {
        3
    } else if value >= 7 {
        2
    } else if value >= 3 {
        1
    } else {
        0
    }
}

/// Vanilla per-renderer `getBlockLightLevel` additive overrides applied to a sampled light
/// probe: `ExperienceOrbRenderer` returns `clamp(super + 7, 0, 15)`. Shared by the entity
/// model-source projection and the pickup-particle light capture.
fn vanilla_entity_renderer_block_light(entity_type_id: i32, light: TerrainLight) -> TerrainLight {
    if entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID {
        TerrainLight {
            block: light.block.saturating_add(7).min(15),
            sky: light.sky,
        }
    } else {
        light
    }
}

/// Vanilla default `LivingEntity.getScale` (the unmodified `SCALE` attribute) used
/// when an `EntityModelSourceState` is deserialized without a recorded scale.
fn entity_model_source_default_scale() -> f32 {
    1.0
}

fn entity_model_source_default_elytra_rot_x() -> f32 {
    std::f32::consts::PI / 12.0
}

fn entity_model_source_default_elytra_rot_z() -> f32 {
    -std::f32::consts::PI / 12.0
}

fn entity_model_source_default_armor_stand_wiggle() -> f32 {
    5.0
}

fn entity_model_source_default_true() -> bool {
    true
}

fn entity_model_source_default_one_i32() -> i32 {
    1
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
            if let Some(item_data_id) =
                vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(entity_type_id)
            {
                let mut stack_shrank = false;
                let keep_entity = self
                    .entities
                    .with_metadata_mut(packet.item_id, |metadata| {
                        if let Some(stack) =
                            item_entity_stack_mut(&mut metadata.data_values, item_data_id)
                        {
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

    /// Applies the client half of an entity's own `discard()` call, used by
    /// Vanilla bucketable interaction prediction. A later server remove or
    /// spawn correction remains authoritative.
    pub(crate) fn discard_local_entity(&mut self, entity_id: i32) -> bool {
        self.remove_entities_by_ids(&[entity_id]) == 1
    }

    fn remove_entities_by_ids(&mut self, removed_ids: &[i32]) -> usize {
        self.remove_tickable_entity_sounds(removed_ids);
        self.close_minecart_command_block_editor_for_removed_entities(removed_ids);
        let removed_uuids = removed_ids
            .iter()
            .filter_map(|id| self.entities.uuid_for_protocol_id(*id))
            .collect::<Vec<_>>();
        if let Some(removed_vehicle_id) = removed_ids
            .iter()
            .copied()
            .find(|id| self.local_player_vehicle_chain_contains(*id))
        {
            self.removed_player_vehicle_id = Some(removed_vehicle_id);
            self.local_player_vehicle_id = None;
        }
        let removed = self.entities.remove_ids(removed_ids);
        for uuid in removed_uuids {
            self.drop_debug_entity_uuid(uuid);
        }
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

    pub fn entity_type_id(&self, id: i32) -> Option<i32> {
        self.entities.entity_type_id(id)
    }

    pub fn entity_debug_passenger_vehicle_target(
        &self,
        passenger_id: i32,
    ) -> Option<EntityPickTargetState> {
        let game_time = self.world_time().map(|time| time.game_time).unwrap_or(0);
        self.entities
            .debug_passenger_vehicle_target(passenger_id, game_time)
    }

    /// Vanilla `EntityRenderer.extractRenderState` leash branch: the rope states for a
    /// leashed entity — one per rope, four for a quad connection — with the light levels
    /// sampled at both endpoints' eye positions (burning entities read block light 15).
    /// Vanilla `EnderDragonRenderState.distanceToEgg`: the squared center distance from the dragon
    /// to `getHeightmapPos(MOTION_BLOCKING_NO_LEAVES, EndPodiumFeature.getLocation(fightOrigin))`.
    /// A remote client's `fightOrigin` is never synced and keeps its `BlockPos.ZERO` default
    /// (`EnderDragon.setFightOrigin` only runs server-side), so the probe column is always `(0, 0)`.
    /// `ClientboundLevelChunkWithLightPacket` sends the client-owned
    /// `MOTION_BLOCKING_NO_LEAVES` map, and `LevelChunk.setBlockState` keeps it current while
    /// excluding `LeavesBlock`. Outside loaded chunks vanilla `Level.getHeight` degrades to the
    /// dimension floor.
    fn ender_dragon_distance_to_egg_sqr(&self, position: EntityVec3) -> f64 {
        let egg_y = self
            .chunk_heightmap_sampler(ChunkPos { x: 0, z: 0 })
            .and_then(|sampler| {
                sampler.first_available(
                    bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
                    0,
                    0,
                )
            })
            .unwrap_or(self.dimension.min_y);
        let dx = 0.5 - position.x;
        let dy = f64::from(egg_y) + 0.5 - position.y;
        let dz = 0.5 - position.z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn entity_leash_render_states_at_partial_tick(
        &self,
        entity_id: i32,
        partial_ticks: f32,
    ) -> Option<Vec<EntityLeashRenderState>> {
        let partial_ticks = partial_ticks.clamp(0.0, 1.0);
        let game_time = self.world_time().map_or(0, |time| time.game_time);
        let minecart_improvements_enabled =
            self.is_feature_enabled("minecraft:minecart_improvements");
        let holder_id = self.entities.leash_holder_id(entity_id)?;
        let leashed = self.entities.leash_endpoint_inputs(
            entity_id,
            partial_ticks,
            game_time,
            minecart_improvements_enabled,
        )?;
        let holder = self.entities.leash_endpoint_inputs(
            holder_id,
            partial_ticks,
            game_time,
            minecart_improvements_enabled,
        )?;
        let mut states = leash::leash_render_states(&leashed, &holder);
        let eye_light = |inputs: &leash::LeashEndpointInputs| {
            let light = self
                .sample_block_light(entity_light_block_pos(EntityVec3 {
                    x: inputs.position.x,
                    y: inputs.position.y + f64::from(inputs.eye_height),
                    z: inputs.position.z,
                }))
                .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT);
            let block = if inputs.on_fire { 15 } else { light.block };
            (block, light.sky)
        };
        let (start_block, start_sky) = eye_light(&leashed);
        let (end_block, end_sky) = eye_light(&holder);
        for state in &mut states {
            state.start_block_light = start_block;
            state.end_block_light = end_block;
            state.start_sky_light = start_sky;
            state.end_sky_light = end_sky;
        }
        Some(states)
    }

    pub fn ender_dragon_part_parent_id(&self, part_id: i32) -> Option<i32> {
        self.entities.ender_dragon_part_parent_id(part_id)
    }

    pub fn probe_entity_status(&self, id: i32) -> Option<EntityStatusProbeState> {
        let last_damage = self.entity_last_damage(id);
        let entity = self.probe_entity(id)?;
        Some(EntityStatusProbeState {
            id: entity.id,
            entity_type_id: entity.entity_type_id,
            last_animation_action: entity.last_animation_action,
            last_event_id: entity.last_event_id,
            last_hurt_yaw: entity.last_hurt_yaw,
            mob_effects: entity.mob_effects,
            last_damage,
            damage_invulnerable_time: entity.damage_invulnerable_time,
        })
    }

    pub fn probe_entity_transform(&self, id: i32) -> Option<EntityTransformState> {
        self.entities.transform_state(id)
    }

    /// Vanilla client-owned `Player.sleepCounter` for one exact player entity.
    /// Non-player and absent ids return `None`.
    pub fn player_sleep_counter(&self, id: i32) -> Option<i32> {
        self.entities.player_sleep_counter(id)
    }

    /// The local player's sleep counter used by `Gui.extractSleepOverlay`.
    pub fn local_player_sleep_counter(&self) -> i32 {
        self.local_player_id
            .and_then(|id| self.entities.player_sleep_counter(id))
            .unwrap_or_default()
    }

    pub fn llama_spit_recreation_particle_state(
        &self,
        id: i32,
    ) -> Option<LlamaSpitRecreationParticleState> {
        let transform = self.probe_entity_transform(id)?;
        (transform.entity_type_id == VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID).then_some(
            LlamaSpitRecreationParticleState {
                entity_id: id,
                position: transform.position,
                delta_movement: transform.delta_movement,
            },
        )
    }

    pub fn take_item_entity_pickup_particle_state(
        &self,
        item_entity_id: i32,
        target_entity_id: i32,
    ) -> Option<TakeItemEntityPickupParticleState> {
        let item = self.probe_entity_transform(item_entity_id)?;
        let target = self
            .probe_entity_camera_pose(target_entity_id)
            .map(|pose| (pose.id, pose.position, pose.eye_height))
            .or_else(|| {
                let local_id = self.local_player_id()?;
                let pose = self.local_player_pose()?;
                Some((
                    local_id,
                    entity_vec3(pose.position),
                    pose.eye_height() as f32,
                ))
            })?;
        let item_stack = (item.entity_type_id == VANILLA_ENTITY_TYPE_ITEM_ID)
            .then(|| self.entities.item_stack_for_entity(item_entity_id))
            .flatten();
        let experience_orb_icon = if item.entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID {
            Some(
                self.entities
                    .experience_orb_icon_for_entity(item_entity_id)
                    .unwrap_or(0),
            )
        } else {
            None
        };
        let item_age_ticks = self
            .entities
            .entity_age_ticks(item_entity_id)
            .map(|age| age as f32 + 1.0)
            .unwrap_or(1.0);
        let item_bob_offset = self.entities.item_bob_offset(item_entity_id).unwrap_or(0.0);
        let item_light = vanilla_entity_renderer_block_light(
            item.entity_type_id,
            self.sample_block_light(entity_light_block_pos(item.position))
                .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT),
        );
        let projectile_model = match item.entity_type_id {
            // Vanilla `TippableArrowRenderer.isTipped = getColor() > 0`.
            VANILLA_ENTITY_TYPE_ARROW_ID => Some(TakeItemEntityPickupProjectileModel::Arrow {
                tipped: vanilla_entity_metadata_arrow_accessors_for_entity_type(
                    item.entity_type_id,
                )
                .map(|accessors| {
                    self.entities
                        .metadata_int_for_entity(
                            item_entity_id,
                            accessors.effect_color_data_id,
                            accessors.effect_color_default,
                        )
                        .unwrap_or(accessors.effect_color_default)
                        > 0
                })
                .unwrap_or(false),
            }),
            VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID => {
                Some(TakeItemEntityPickupProjectileModel::SpectralArrow)
            }
            VANILLA_ENTITY_TYPE_TRIDENT_ID => Some(TakeItemEntityPickupProjectileModel::Trident {
                foil: vanilla_entity_metadata_thrown_trident_accessors_for_entity_type(
                    item.entity_type_id,
                )
                .map(|accessors| {
                    self.entities
                        .metadata_bool_for_entity(
                            item_entity_id,
                            accessors.foil_data_id,
                            accessors.foil_default,
                        )
                        .unwrap_or(accessors.foil_default)
                })
                .unwrap_or(false),
            }),
            _ => None,
        };

        Some(TakeItemEntityPickupParticleState {
            item_entity_id,
            item_entity_type_id: item.entity_type_id,
            item_position: item.position,
            item_delta_movement: item.delta_movement,
            item_y_rot: item.y_rot,
            item_x_rot: item.x_rot,
            item_age_ticks,
            item_bob_offset,
            item_light,
            target_entity_id: target.0,
            target_position: target.1,
            target_eye_height: target.2,
            item_stack,
            experience_orb_icon,
            projectile_model,
        })
    }

    pub fn ravager_roar_particle_state(&self, entity_id: i32) -> Option<RavagerRoarParticleState> {
        self.entities.ravager_roar_particle_state(entity_id)
    }

    pub fn witch_magic_particle_state(&self, entity_id: i32) -> Option<WitchMagicParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_WITCH_ID {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        Some(WitchMagicParticleState {
            entity_id,
            position: transform.position,
            bounding_box_max_y: transform.position.y + f64::from(bounds.max[1]),
        })
    }

    pub fn living_entity_poof_particle_state(
        &self,
        entity_id: i32,
    ) -> Option<LivingEntityPoofParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if !vanilla_living_entity_type(transform.entity_type_id) {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        Some(LivingEntityPoofParticleState {
            entity_id,
            position: transform.position,
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
        })
    }

    /// Projects the selected equipment stack and current view pose for vanilla
    /// `LivingEntity.handleEntityEvent(47..=52, 65, 68)` -> `breakItem`.
    pub fn living_entity_item_break_state(
        &self,
        entity_id: i32,
        slot: ProtocolEquipmentSlot,
    ) -> Option<LivingEntityItemBreakState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if !vanilla_living_entity_type(transform.entity_type_id) {
            return None;
        }
        let pose = self.probe_entity_camera_pose(entity_id)?;
        let item_stack = self.entities.equipment_item(entity_id, slot)?;
        if item_stack.count <= 0 {
            return None;
        }
        Some(LivingEntityItemBreakState {
            entity_id,
            position: transform.position,
            eye_height: pose.eye_height,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            item_stack,
        })
    }

    /// Projects vanilla `Mob.handleEntityEvent(20)` spawn feedback.
    ///
    /// `Mob.spawnAnim` calls `LivingEntity.makePoofParticles`, so this shares the exact event-60
    /// state/resolver while retaining the narrower generated Mob class membership.
    pub fn mob_spawn_particle_state(
        &self,
        entity_id: i32,
    ) -> Option<LivingEntityPoofParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if !schema::vanilla_entity_metadata_type_is_mob(transform.entity_type_id) {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        Some(LivingEntityPoofParticleState {
            entity_id,
            position: transform.position,
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
        })
    }

    pub fn living_entity_drown_particle_state(
        &self,
        entity_id: i32,
    ) -> Option<LivingEntityDrownParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if !vanilla_living_entity_type(transform.entity_type_id) {
            return None;
        }
        Some(LivingEntityDrownParticleState {
            entity_id,
            position: transform.position,
            delta_movement: transform.delta_movement,
        })
    }

    pub fn living_entity_portal_particle_state(
        &self,
        entity_id: i32,
    ) -> Option<LivingEntityPortalParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if !vanilla_living_entity_type(transform.entity_type_id) {
            return None;
        }
        let previous_position = self
            .entities
            .living_entity_previous_feet_position(entity_id)
            .flatten()
            .unwrap_or(transform.position);
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        Some(LivingEntityPortalParticleState {
            entity_id,
            previous_position,
            position: transform.position,
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
        })
    }

    pub fn arrow_effect_particle_state(&self, entity_id: i32) -> Option<ArrowEffectParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_ARROW_ID {
            return None;
        }
        let accessors =
            vanilla_entity_metadata_arrow_accessors_for_entity_type(transform.entity_type_id)?;
        let color = self.entities.metadata_int_for_entity(
            entity_id,
            accessors.effect_color_data_id,
            accessors.effect_color_default,
        )?;
        if color == accessors.effect_color_default {
            return None;
        }
        let bounds = self
            .entities
            .model_source_bounds(entity_id, &self.registries)?;
        Some(ArrowEffectParticleState {
            entity_id,
            position: transform.position,
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
            color_rgb: color as u32 & 0x00ff_ffff,
        })
    }

    pub fn animal_love_particle_state(&self, entity_id: i32) -> Option<AnimalLoveParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if !vanilla_animal_entity_type(transform.entity_type_id) {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        Some(AnimalLoveParticleState {
            entity_id,
            position: transform.position,
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
        })
    }

    pub fn allay_duplication_particle_state(
        &self,
        entity_id: i32,
    ) -> Option<AllayDuplicationParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_ALLAY_ID {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        Some(AllayDuplicationParticleState {
            entity_id,
            position: transform.position,
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
        })
    }

    pub fn entity_taming_particle_state(
        &self,
        entity_id: i32,
        success: bool,
    ) -> Option<EntityTamingParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if !is_vanilla_taming_particle_entity_type(transform.entity_type_id) {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        Some(EntityTamingParticleState {
            entity_id,
            position: transform.position,
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
            success,
        })
    }

    /// Projects vanilla `Ocelot.handleEntityEvent(40/41)` trusting feedback.
    ///
    /// `Ocelot.spawnTrustingParticles` intentionally has the same seven-particle HEART/SMOKE
    /// algorithm as tamable/horse feedback, so both paths share [`EntityTamingParticleState`].
    pub fn ocelot_trusting_particle_state(
        &self,
        entity_id: i32,
        success: bool,
    ) -> Option<EntityTamingParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if !schema::vanilla_entity_metadata_type_uses_ocelot_entity_event_handler(
            transform.entity_type_id,
        ) {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        Some(EntityTamingParticleState {
            entity_id,
            position: transform.position,
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
            success,
        })
    }

    pub fn villager_particle_state(
        &self,
        entity_id: i32,
        kind: VillagerParticleKind,
    ) -> Option<VillagerParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_VILLAGER_ID {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        Some(VillagerParticleState {
            entity_id,
            position: transform.position,
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
            kind,
        })
    }

    pub fn dolphin_happy_particle_state(
        &self,
        entity_id: i32,
    ) -> Option<DolphinHappyParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_DOLPHIN_ID {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        Some(DolphinHappyParticleState {
            entity_id,
            position: transform.position,
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
        })
    }

    pub fn snowball_hit_particle_state(&self, entity_id: i32) -> Option<SnowballHitParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_SNOWBALL_ID {
            return None;
        }
        let item_stack = match self.entities.raw_item_stack_for_entity(entity_id) {
            Some(stack) if stack.item_id.is_some() && stack.count > 0 => Some(stack),
            Some(_) => None,
            None => Some(ProtocolItemStackSummary {
                item_id: Some(VANILLA_ITEM_SNOWBALL_ID),
                count: 1,
                component_patch: Default::default(),
            }),
        };
        Some(SnowballHitParticleState {
            entity_id,
            position: transform.position,
            item_stack,
        })
    }

    pub fn thrown_egg_hit_particle_state(
        &self,
        entity_id: i32,
    ) -> Option<ThrownEggHitParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_EGG_ID {
            return None;
        }
        let item_stack = match self.entities.raw_item_stack_for_entity(entity_id) {
            Some(stack) if stack.item_id.is_some() && stack.count > 0 => stack,
            Some(_) => return None,
            None => ProtocolItemStackSummary {
                item_id: Some(VANILLA_ITEM_EGG_ID),
                count: 1,
                component_patch: Default::default(),
            },
        };
        Some(ThrownEggHitParticleState {
            entity_id,
            position: transform.position,
            item_stack,
        })
    }

    pub fn honey_block_particle_state(
        &self,
        entity_id: i32,
        count: u32,
        living_only: bool,
    ) -> Option<HoneyBlockParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if living_only && !vanilla_living_entity_type(transform.entity_type_id) {
            return None;
        }
        Some(HoneyBlockParticleState {
            entity_id,
            position: transform.position,
            count,
            block_state_id: honey_block_state_id()?,
        })
    }

    pub(crate) fn rabbit_sprint_particle_state(
        &self,
        entity_id: i32,
        mut next_double: impl FnMut() -> f64,
    ) -> Option<RabbitSprintParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_RABBIT_ID {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        let on_pos = self.legacy_on_pos(
            self.entity_main_supporting_block_pos(entity_id),
            transform.position,
        );
        let block = self.probe_block(on_pos)?;
        if block
            .block_name
            .as_deref()
            .is_some_and(block_name_has_invisible_render_shape)
        {
            return None;
        }

        let width = f64::from(bounds.max[0] - bounds.min[0]);
        let mut x = transform.position.x + (next_double().clamp(0.0, 1.0) - 0.5) * width;
        let mut z = transform.position.z + (next_double().clamp(0.0, 1.0) - 0.5) * width;
        let entity_pos = BlockPos {
            x: transform.position.x.floor() as i32,
            y: transform.position.y.floor() as i32,
            z: transform.position.z.floor() as i32,
        };
        if entity_pos.x != on_pos.x {
            x = x.clamp(f64::from(on_pos.x), f64::from(on_pos.x) + 1.0);
        }
        if entity_pos.z != on_pos.z {
            z = z.clamp(f64::from(on_pos.z), f64::from(on_pos.z) + 1.0);
        }

        Some(RabbitSprintParticleState {
            entity_id,
            block_state_id: block.block_state_id,
            position: EntityVec3 {
                x,
                y: transform.position.y + 0.1,
                z,
            },
            velocity: EntityVec3 {
                x: transform.delta_movement.x * -4.0,
                y: 1.5,
                z: transform.delta_movement.z * -4.0,
            },
        })
    }

    pub fn probe_entity_camera_pose(&self, id: i32) -> Option<EntityCameraPoseState> {
        self.entities.camera_pose_state(id)
    }

    pub fn probe_entity_camera_pose_at_partial_tick(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<EntityCameraPoseState> {
        self.entities
            .camera_pose_state_at_partial_tick(id, partial_ticks)
    }

    pub fn probe_entity_camera_hurt_state(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<EntityCameraHurtState> {
        self.entities.camera_hurt_state(id, partial_ticks)
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

    pub fn entity_debug_hitbox_targets_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<EntityPickTargetState> {
        self.entities
            .debug_hitbox_targets_at_partial_tick(partial_ticks)
            .into_iter()
            .filter(|target| self.entity_passes_spectator_filter(target.entity_id))
            .collect()
    }

    pub fn entity_model_sources_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<EntityModelSourceState> {
        let candidates = self.entity_render_candidates_at_partial_tick(partial_ticks);
        let survivors = candidates
            .iter()
            .copied()
            .filter(|candidate| !candidate.is_spectator)
            .collect::<Vec<_>>();
        self.entity_model_sources_for_candidates_at_partial_tick(
            &survivors,
            &candidates,
            partial_ticks,
        )
    }

    /// Performs one narrow ECS query and annotates the spectator flag for
    /// render culling. No full model source, item/effect projection, or
    /// light/fluid sampling is performed here.
    pub fn entity_render_candidates_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<EntityRenderCandidateState> {
        let mut candidates = Vec::new();
        self.fill_entity_render_candidates_at_partial_tick(partial_ticks, &mut candidates);
        // Preserve the established public projection order for control/debug
        // consumers. The renderer uses the allocation-retaining `fill_*`
        // surface directly because culling is order-independent.
        self.entities
            .restore_protocol_order_for_model_candidates(&mut candidates);
        candidates
    }

    /// Fills render candidates while retaining the caller's allocation.
    pub fn fill_entity_render_candidates_at_partial_tick(
        &self,
        partial_ticks: f32,
        candidates: &mut Vec<EntityRenderCandidateState>,
    ) {
        let game_time = self.world_time().map_or(0, |time| time.game_time);
        self.entities.fill_model_candidates_at_partial_tick(
            partial_ticks,
            game_time,
            self.is_feature_enabled("minecraft:minecart_improvements"),
            &self.registries,
            candidates,
        );
        for candidate in candidates {
            candidate.is_spectator = candidate.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
                && self
                    .player_info_entry(candidate.uuid)
                    .is_some_and(|info| info.is_spectator());
        }
    }

    /// Projects current synchronized Display state only for already-visible
    /// candidates. Camera-dependent billboard transforms remain native-owned;
    /// display transformation interpolation remains in the animation cell.
    pub fn fill_display_entity_render_sources_for_candidates_at_partial_tick(
        &self,
        survivors: &[EntityRenderCandidateState],
        partial_ticks: f32,
        sources: &mut Vec<DisplayEntityRenderSourceState>,
    ) {
        sources.clear();
        sources.reserve(survivors.len());
        sources.extend(survivors.iter().filter_map(|candidate| {
            let mut source = self
                .entities
                .display_render_source_at_partial_tick(candidate.entity_id, partial_ticks)?;
            source.position = candidate.position;
            source.light = self
                .sample_block_light(entity_light_block_pos(source.position))
                .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT);
            Some(source)
        }));
    }

    /// Full-projects only `survivors`, while retaining the complete narrow
    /// candidate set for relationship rules such as cats searching sleeping
    /// players. Each survivor is projected exactly once.
    pub fn entity_model_sources_for_candidates_at_partial_tick(
        &self,
        survivors: &[EntityRenderCandidateState],
        all_candidates: &[EntityRenderCandidateState],
        partial_ticks: f32,
    ) -> Vec<EntityModelSourceState> {
        let mut sources = Vec::new();
        self.fill_entity_model_sources_for_candidates_at_partial_tick(
            survivors,
            all_candidates,
            partial_ticks,
            &mut sources,
        );
        sources
    }

    /// Full-projects survivors while retaining the caller's outer allocation.
    pub fn fill_entity_model_sources_for_candidates_at_partial_tick(
        &self,
        survivors: &[EntityRenderCandidateState],
        all_candidates: &[EntityRenderCandidateState],
        partial_ticks: f32,
        sources: &mut Vec<EntityModelSourceState>,
    ) {
        sources.clear();
        sources.reserve(survivors.len());
        sources.extend(survivors.iter().copied().filter_map(|candidate| {
            self.project_entity_model_source(candidate, partial_ticks, all_candidates)
        }));
    }

    /// Extracts vanilla dispatcher shadow pieces for the exact visible model-source generation.
    pub fn fill_entity_shadow_render_sources_for_model_sources(
        &self,
        survivors: &[EntityRenderCandidateState],
        model_sources: &[EntityModelSourceState],
        partial_ticks: f32,
        camera_position: EntityVec3,
        sky_darken: u8,
        shadows_enabled: bool,
        sources: &mut Vec<EntityShadowRenderSourceState>,
    ) {
        shadows::fill_entity_shadow_render_sources(
            self,
            survivors,
            model_sources,
            partial_ticks,
            camera_position,
            sky_darken,
            shadows_enabled,
            sources,
        );
    }

    /// Whether an entity should appear in the rendered model-source set. Vanilla hides spectator
    /// players from the entity render list; every non-player and every non-spectator player is kept.
    pub(crate) fn entity_passes_spectator_filter(&self, entity_id: i32) -> bool {
        let Some(identity) = self.entities.identity(entity_id) else {
            return true;
        };
        identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
            || !self
                .player_info_entry(identity.uuid)
                .is_some_and(|info| info.is_spectator())
    }

    /// Mirrors `EntityGetter.isUnobstructed(null, shape)` for a prospective
    /// single-box block collision shape. Missing canonical entity dimensions
    /// keep the caller's local prediction explicit.
    pub(crate) fn local_block_placement_is_unobstructed(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
    ) -> Option<bool> {
        let obstructing = self
            .entities
            .block_placement_obstruction_ids_intersecting(query_min, query_max)?;
        if obstructing
            .into_iter()
            .any(|entity_id| self.entity_passes_spectator_filter(entity_id))
        {
            return Some(false);
        }
        self.local_player_bounds_intersect(query_min, query_max)
            .map(|intersects| !intersects)
    }

    /// Mirrors the `getEntities(null, box).isEmpty()` half of
    /// `ArmorStandItem.useOn`, including Vanilla's spectator exclusion.
    pub(crate) fn local_non_spectator_entity_bounds_are_empty(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
    ) -> Option<bool> {
        let intersecting = self
            .entities
            .all_entity_bounds_intersection_ids(query_min, query_max)?;
        if intersecting
            .into_iter()
            .any(|entity_id| self.entity_passes_spectator_filter(entity_id))
        {
            return Some(false);
        }
        self.local_player_bounds_intersect(query_min, query_max)
            .map(|intersects| !intersects)
    }

    /// Narrow single-entity variant of [`Self::entity_model_sources_at_partial_tick`]. Reuses the
    /// same target list, spectator filter, and per-entity projection helper as the list path, so the
    /// returned state is identical to the list entry for `entity_id` (or `None` when it is filtered
    /// out or absent). Avoids building the full projected `Vec` when only one entity is needed.
    pub fn entity_model_source_at_partial_tick(
        &self,
        entity_id: i32,
        partial_ticks: f32,
    ) -> Option<EntityModelSourceState> {
        let candidates = self.entity_render_candidates_at_partial_tick(partial_ticks);
        let candidate = candidates
            .iter()
            .copied()
            .find(|candidate| candidate.entity_id == entity_id)?;
        if candidate.is_spectator {
            return None;
        }
        self.project_entity_model_source(candidate, partial_ticks, &candidates)
    }

    /// Per-entity projection shared by [`Self::entity_model_sources_at_partial_tick`] and
    /// [`Self::entity_model_source_at_partial_tick`]. Turns one model target into a fully populated
    /// [`EntityModelSourceState`], sampling light, water overlap, and the
    /// boat/minecart/parrot/sleeping/cat specials. `targets` is the full (unfiltered) target list,
    /// needed by the cat "lying on a sleeping player" search.
    fn project_entity_model_source(
        &self,
        target: EntityRenderCandidateState,
        partial_ticks: f32,
        targets: &[EntityRenderCandidateState],
    ) -> Option<EntityModelSourceState> {
        let mut source = self.entities.model_source(
            target.entity_id,
            target.position,
            partial_ticks,
            self.is_feature_enabled("minecraft:minecart_improvements"),
            &self.registries,
            &self.items.default_item_max_damage,
            &self.items.default_item_equipment_assets,
            self.items.equipment_asset_generation,
            &self.items.equipment_asset_names,
            &self.items.default_item_armor_materials,
            &self.items.default_item_wings_equipment_assets,
            &self.items.default_item_equipment_slots,
            &self.items.default_llama_body_decor_colors,
            &self.items.default_happy_ghast_harness_colors,
            &self.items.default_nautilus_body_armor_materials,
            &self.items.default_horse_body_armor_materials,
            &self.items.default_wolf_body_armor_materials,
        )?;
        source.light = vanilla_entity_renderer_block_light(
            source.entity_type_id,
            self.sample_block_light(entity_light_block_pos(target.position))
                .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT),
        );
        if let Some(dragon) = source.ender_dragon_animation.as_mut() {
            dragon.distance_to_egg = self.ender_dragon_distance_to_egg_sqr(target.position);
        }
        source.guardian_eye_tracking =
            self.guardian_eye_tracking_source(&source, partial_ticks, targets);
        // The rope states need world light samples at both endpoints, so the leash
        // projection lives here on the store-and-light layer rather than in `model_source`.
        source.leash_ropes = self
            .entity_leash_render_states_at_partial_tick(target.entity_id, partial_ticks)
            .map(|states| {
                states
                    .iter()
                    .map(|state| LeashRopeSource {
                        offset: state.offset.map(|value| value as f32),
                        delta: [
                            (state.end[0] - state.start[0]) as f32,
                            (state.end[1] - state.start[1]) as f32,
                            (state.end[2] - state.start[2]) as f32,
                        ],
                        start_block_light: state.start_block_light,
                        start_sky_light: state.start_sky_light,
                        end_block_light: state.end_block_light,
                        end_sky_light: state.end_sky_light,
                        slack: state.slack,
                    })
                    .collect()
            })
            .unwrap_or_default();
        if source.invisible_to_player
            && vanilla_living_entity_type(source.entity_type_id)
            && (self.local_player_is_spectator()
                || self.local_player_can_see_friendly_invisible(&source))
        {
            // Vanilla `Entity.isInvisibleTo(player)`: spectator viewers and same-team
            // viewers whose team enables `canSeeFriendlyInvisibles` can see invisible
            // living entities, so the renderer takes the force-transparent branch instead
            // of the hidden branch.
            source.invisible_to_player = false;
        }
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
        if is_vanilla_boat_type(source.entity_type_id) {
            // Vanilla `AbstractBoatRenderer.extractRenderState`: `state.isUnderWater`
            // comes from `AbstractBoat.isUnderWater()`, which is a top-slice water
            // surface test, not the broader `Entity.isInWater()` overlap above.
            source.boat_underwater =
                crate::fluid::world_aabb_boat_underwater(self, aabb_min, aabb_max);
        }
        if is_vanilla_minecart_type(source.entity_type_id) && !source.minecart_new_render {
            if let Some(rail) = self.old_minecart_rail_render_state(source.position) {
                source.minecart_pos_on_rail = Some(rail.pos_on_rail);
                source.minecart_front_pos = Some(rail.front_pos);
                source.minecart_back_pos = Some(rail.back_pos);
            }
        }
        if source.entity_type_id == VANILLA_ENTITY_TYPE_PARROT_ID {
            // Vanilla `LevelEventHandler.playJukeboxSong` notifies nearby living entities,
            // and `Parrot.aiStep` keeps PARTY only while its jukebox block position is within
            // `BlockPos.closerToCenterThan(entity.position(), 3.46)`.
            source.parrot_party = self.parrot_party_near_playing_jukebox(target.position);
        }
        if source.is_sleeping {
            if let Some((yaw, offset)) = self.resolve_sleeping_bed(target.entity_id) {
                source.sleeping_bed_yaw = Some(yaw);
                source.sleeping_bed_offset = offset;
            }
        }
        source.feline_is_lying_on_top_of_sleeping_player =
            self.cat_is_lying_on_top_of_sleeping_player(&source, target, targets);
        if !source.is_upside_down && self.resolve_player_upside_down(target.entity_id) {
            source.is_upside_down = true;
        }
        source.outline_color = self.entity_outline_color(&source);
        source.show_extra_ears = self.resolve_player_extra_ears(target.entity_id);
        Some(source)
    }

    /// Mirrors `GuardianRenderer.getEntityToLookAt` and its render-state extraction. An active
    /// attack-target id deliberately does not fall back to the camera when the referenced entity is
    /// missing or is not living: Vanilla's `getActiveAttackTarget()` returns `null` in that case.
    fn guardian_eye_tracking_source(
        &self,
        source: &EntityModelSourceState,
        partial_ticks: f32,
        targets: &[EntityRenderCandidateState],
    ) -> Option<GuardianEyeTrackingSource> {
        if !vanilla_entity_metadata_type_is_guardian(source.entity_type_id) {
            return None;
        }
        let guardian =
            self.probe_entity_camera_pose_at_partial_tick(source.entity_id, partial_ticks)?;
        let attack_target_id =
            guardian_attack_target_id(source.entity_type_id, &source.data_values);
        let look_at_id = if attack_target_id != 0 {
            let target = targets
                .iter()
                .find(|target| target.entity_id == attack_target_id)?;
            if !vanilla_living_entity_type(target.entity_type_id) {
                return None;
            }
            attack_target_id
        } else {
            self.local_player.camera.entity_id?
        };
        let look_at = self.probe_entity_camera_pose_at_partial_tick(look_at_id, partial_ticks)?;
        let look_direction = calculate_view_vector(guardian.x_rot, guardian.y_rot);
        Some(GuardianEyeTrackingSource {
            eye_position: [
                guardian.position.x,
                guardian.position.y + f64::from(guardian.eye_height),
                guardian.position.z,
            ],
            look_direction: [look_direction.x, look_direction.y, look_direction.z],
            look_at_position: [
                look_at.position.x,
                look_at.position.y + f64::from(look_at.eye_height),
                look_at.position.z,
            ],
        })
    }

    fn old_minecart_rail_render_state(
        &self,
        position: EntityVec3,
    ) -> Option<MinecartRailRenderState> {
        let pos_on_rail = self.old_minecart_get_pos(position.x, position.y, position.z)?;
        let front_pos = self
            .old_minecart_get_pos_offs(position.x, position.y, position.z, 0.3)
            .unwrap_or(pos_on_rail);
        let back_pos = self
            .old_minecart_get_pos_offs(position.x, position.y, position.z, -0.3)
            .unwrap_or(pos_on_rail);
        Some(MinecartRailRenderState {
            pos_on_rail: vec3_f64_to_f32_array(pos_on_rail),
            front_pos: vec3_f64_to_f32_array(front_pos),
            back_pos: vec3_f64_to_f32_array(back_pos),
        })
    }

    /// Vanilla `OldMinecartBehavior.getPosOffs`: sample the current rail, step
    /// along its exit direction by `offs`, adjust the query Y for slopes, then
    /// resolve through `getPos`.
    fn old_minecart_get_pos_offs(
        &self,
        mut x: f64,
        mut y: f64,
        mut z: f64,
        offs: f64,
    ) -> Option<[f64; 3]> {
        let xt = floor_to_i32(x);
        let mut yt = floor_to_i32(y);
        let zt = floor_to_i32(z);
        if self
            .rail_shape_at(BlockPos {
                x: xt,
                y: yt - 1,
                z: zt,
            })
            .is_some()
        {
            yt -= 1;
        }

        let shape = self.rail_shape_at(BlockPos {
            x: xt,
            y: yt,
            z: zt,
        })?;
        y = f64::from(yt);
        if shape.is_slope() {
            y = f64::from(yt + 1);
        }
        let (exit0, exit1) = shape.exits();
        let mut x_d = f64::from(exit1[0] - exit0[0]);
        let mut z_d = f64::from(exit1[2] - exit0[2]);
        let dd = (x_d * x_d + z_d * z_d).sqrt();
        x_d /= dd;
        z_d /= dd;
        x += x_d * offs;
        z += z_d * offs;

        if exit0[1] != 0 && floor_to_i32(x) - xt == exit0[0] && floor_to_i32(z) - zt == exit0[2] {
            y += f64::from(exit0[1]);
        } else if exit1[1] != 0
            && floor_to_i32(x) - xt == exit1[0]
            && floor_to_i32(z) - zt == exit1[2]
        {
            y += f64::from(exit1[1]);
        }

        self.old_minecart_get_pos(x, y, z)
    }

    /// Vanilla `OldMinecartBehavior.getPos`: project the minecart's interpolated
    /// entity position onto the current rail segment, including the rail's
    /// `0.0625` Y lift and slope endpoint correction.
    fn old_minecart_get_pos(&self, mut x: f64, y: f64, mut z: f64) -> Option<[f64; 3]> {
        let xt = floor_to_i32(x);
        let mut yt = floor_to_i32(y);
        let zt = floor_to_i32(z);
        if self
            .rail_shape_at(BlockPos {
                x: xt,
                y: yt - 1,
                z: zt,
            })
            .is_some()
        {
            yt -= 1;
        }

        let shape = self.rail_shape_at(BlockPos {
            x: xt,
            y: yt,
            z: zt,
        })?;
        let (exit0, exit1) = shape.exits();
        let x0 = f64::from(xt) + 0.5 + f64::from(exit0[0]) * 0.5;
        let y0 = f64::from(yt) + 0.0625 + f64::from(exit0[1]) * 0.5;
        let z0 = f64::from(zt) + 0.5 + f64::from(exit0[2]) * 0.5;
        let x1 = f64::from(xt) + 0.5 + f64::from(exit1[0]) * 0.5;
        let y1 = f64::from(yt) + 0.0625 + f64::from(exit1[1]) * 0.5;
        let z1 = f64::from(zt) + 0.5 + f64::from(exit1[2]) * 0.5;
        let x_d = x1 - x0;
        let y_d = (y1 - y0) * 2.0;
        let z_d = z1 - z0;
        let progress = if x_d == 0.0 {
            z - f64::from(zt)
        } else if z_d == 0.0 {
            x - f64::from(xt)
        } else {
            let xx = x - x0;
            let zz = z - z0;
            (xx * x_d + zz * z_d) * 2.0
        };

        x = x0 + x_d * progress;
        let mut projected_y = y0 + y_d * progress;
        z = z0 + z_d * progress;
        if y_d < 0.0 {
            projected_y += 1.0;
        } else if y_d > 0.0 {
            projected_y += 0.5;
        }
        Some([x, projected_y, z])
    }

    fn rail_shape_at(&self, pos: BlockPos) -> Option<MinecartRailShape> {
        let block = self.probe_block(pos)?;
        if !block_name_is_rail(block.block_name.as_deref()?) {
            return None;
        }
        MinecartRailShape::from_vanilla_name(block.block_properties.get("shape")?)
    }

    fn cat_is_lying_on_top_of_sleeping_player(
        &self,
        source: &EntityModelSourceState,
        cat_target: EntityRenderCandidateState,
        targets: &[EntityRenderCandidateState],
    ) -> bool {
        if !cat_is_lying(source.entity_type_id, &source.data_values) {
            return false;
        }

        let (search_min, search_max) = cat_sleeping_player_search_aabb(cat_target.position);
        targets.iter().copied().any(|target| {
            target.entity_id != cat_target.entity_id
                && self.entities.entity_type_id(target.entity_id)
                    == Some(VANILLA_ENTITY_TYPE_PLAYER_ID)
                && self.entities.pose(target.entity_id) == Some(VANILLA_ENTITY_POSE_SLEEPING_ID)
                && {
                    let (player_min, player_max) = model_target_world_aabb(target);
                    aabb_intersects(search_min, search_max, player_min, player_max)
                }
        })
    }

    fn entity_outline_color(&self, source: &EntityModelSourceState) -> u32 {
        if !source.appears_glowing {
            return 0;
        }

        let scoreboard_name = self.entity_scoreboard_name(source);
        let rgb = self
            .scoreboard
            .team_color_rgb_for_scoreboard_name(&scoreboard_name)
            .unwrap_or(VANILLA_DEFAULT_ENTITY_OUTLINE_RGB);
        VANILLA_OPAQUE_ALPHA | rgb
    }

    fn parrot_party_near_playing_jukebox(&self, position: EntityVec3) -> bool {
        const VANILLA_PARROT_PARTY_JUKEBOX_DISTANCE: f64 = 3.46;
        self.playing_jukebox_songs().iter().any(|song| {
            block_pos_closer_to_center_than(
                song.pos,
                position,
                VANILLA_PARROT_PARTY_JUKEBOX_DISTANCE,
            )
        })
    }

    fn local_player_can_see_friendly_invisible(&self, source: &EntityModelSourceState) -> bool {
        let Some(local_player_name) = self.local_player_scoreboard_name() else {
            return false;
        };
        let target_name = self.entity_scoreboard_name(source);
        self.scoreboard
            .same_team_can_see_friendly_invisibles(&local_player_name, &target_name)
    }

    pub(crate) fn local_player_scoreboard_name(&self) -> Option<String> {
        let local_player_id = self.local_player_id?;
        let identity = self.entities.identity(local_player_id)?;
        Some(self.scoreboard_name_for_identity(identity.entity_type_id, identity.uuid))
    }

    fn entity_scoreboard_name(&self, source: &EntityModelSourceState) -> String {
        self.scoreboard_name_for_identity(source.entity_type_id, source.uuid)
    }

    pub(crate) fn scoreboard_name_for_identity(
        &self,
        entity_type_id: i32,
        uuid: uuid::Uuid,
    ) -> String {
        if entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            if let Some(player) = self.player_info_entry(uuid) {
                return player.profile.name.clone();
            }
        }

        uuid.to_string()
    }

    pub fn enderman_carried_block_state(&self, entity_id: i32) -> Option<EntityBlockModelState> {
        let state_id = self.entities.enderman_carried_block_state_id(entity_id)??;
        self.registries
            .block_state(state_id)
            .map(|state| EntityBlockModelState {
                name: state.name.clone(),
                properties: state.properties.clone(),
            })
    }

    /// Resolves one synchronized block-state id into the model identity and
    /// properties consumed by entity-owned block submissions such as
    /// `BlockDisplayRenderer`.
    pub fn entity_block_model_state(&self, state_id: i32) -> Option<EntityBlockModelState> {
        self.registries
            .block_state(state_id)
            .map(|state| EntityBlockModelState {
                name: state.name.clone(),
                properties: state.properties.clone(),
            })
    }

    pub fn minecart_display_block_state(
        &self,
        entity_id: i32,
    ) -> Option<MinecartDisplayBlockState> {
        self.entities
            .minecart_display_block_state(entity_id, &self.registries)
    }

    pub fn falling_block_state(&self, entity_id: i32) -> Option<FallingBlockModelState> {
        self.entities
            .falling_block_state(entity_id, &self.registries)
    }

    pub fn primed_tnt_block_state(&self, entity_id: i32) -> Option<EntityBlockModelState> {
        self.entities
            .primed_tnt_block_state(entity_id, &self.registries)
    }

    pub fn primed_tnt_fuse_remaining_in_ticks(
        &self,
        entity_id: i32,
        partial_ticks: f32,
    ) -> Option<f32> {
        self.entities
            .primed_tnt_fuse_remaining_in_ticks(entity_id, partial_ticks)
    }

    pub fn primed_tnt_smoke_particle_states(&self) -> Vec<PrimedTntSmokeParticleState> {
        self.entities.primed_tnt_smoke_particle_states()
    }

    pub fn take_ravager_stun_particle_states(&mut self) -> Vec<RavagerStunParticleState> {
        self.entities.take_ravager_stun_particle_states()
    }

    pub fn take_evoker_fangs_crit_particle_states(&mut self) -> Vec<EvokerFangsCritParticleState> {
        self.entities.take_evoker_fangs_crit_particle_states()
    }

    pub fn drain_entity_client_movement_effect_states(
        &mut self,
    ) -> std::vec::Drain<'_, EntityClientMovementEffectState> {
        self.entities.drain_entity_client_movement_effect_states()
    }

    pub fn take_entity_client_movement_effect_states(
        &mut self,
    ) -> Vec<EntityClientMovementEffectState> {
        self.entities.take_entity_client_movement_effect_states()
    }

    pub fn recycle_entity_client_movement_effect_states(
        &mut self,
        effects: Vec<EntityClientMovementEffectState>,
    ) {
        self.entities
            .recycle_entity_client_movement_effect_states(effects);
    }

    pub fn take_ominous_item_spawner_particle_states(
        &mut self,
    ) -> Vec<OminousItemSpawnerParticleState> {
        self.entities.take_ominous_item_spawner_particle_states()
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
                renderer_catalog::vanilla_living_entity_renderer_name_is_upside_down(
                    entry.profile.name.as_str(),
                )
            })
    }

    /// Resolves vanilla `AbstractClientPlayer.showExtraEars`: only a real player
    /// whose GameProfile name is exactly lowercase `deadmau5` enables
    /// `Deadmau5EarsLayer`.
    fn resolve_player_extra_ears(&self, entity_id: i32) -> bool {
        let Some(identity) = self.entities.identity(entity_id) else {
            return false;
        };
        identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            && self
                .player_info_entry(identity.uuid)
                .is_some_and(|entry| entry.profile.name == "deadmau5")
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
        self.item_stacks_with_sampled_light(self.entities.item_entity_stacks())
    }

    /// The render state of every item-frame / glow-item-frame entity (center, facing, sampled light, item
    /// rotation, glow/invisible flags, framed item, map id). Drives the 3D item-frame render (vanilla
    /// `ItemFrameRenderer`).
    pub fn item_frame_render_states(&self) -> Vec<ItemFrameRenderState> {
        let mut states = self.entities.item_frame_render_states();
        for state in &mut states {
            state.light = self
                .sample_block_light(entity_light_block_pos(state.center))
                .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT);
        }
        states
    }

    /// The item a humanoid entity holds in its main (`off_hand = false`) or off hand, or `None` for an
    /// empty hand. Drives the third-person held-item 3D render.
    pub fn held_item(&self, id: i32, off_hand: bool) -> Option<ProtocolItemStackSummary> {
        self.entities.held_item(id, off_hand)
    }

    /// The item in an arbitrary vanilla equipment slot, or `None` for an empty / absent slot. This is
    /// used by entity render layers whose source slot is not a hand, such as the copper golem's antenna
    /// block in `EquipmentSlot.SADDLE`.
    pub fn equipment_item(
        &self,
        id: i32,
        slot: ProtocolEquipmentSlot,
    ) -> Option<ProtocolItemStackSummary> {
        self.entities.equipment_item(id, slot)
    }

    /// Collects the `DATA_ITEM_STACK` carried by every entity whose type id is in `type_ids`. The
    /// thrown-item projectiles (snowball, egg, ender pearl, potions, …) that vanilla's
    /// `ThrownItemRenderer` draws as an item sprite share the dropped item's data layout, so this feeds
    /// the same billboard path. The caller (which owns the vanilla type ids) passes the projectile set.
    pub fn item_stacks_for_entity_types(&self, type_ids: &[i32]) -> Vec<ItemEntityStackState> {
        self.item_stacks_with_sampled_light(self.entities.item_stacks_for_entity_types(type_ids))
    }

    /// The item-model render state for firework rocket entities (vanilla `FireworkEntityRenderer`).
    /// Attached elytra-boost rockets intentionally do not render, matching
    /// `FireworkRocketEntity.shouldRender` / `shouldRenderAtSqrDistance`.
    pub fn firework_rocket_item_states(&self) -> Vec<FireworkRocketItemState> {
        let mut items = self.entities.firework_rocket_item_states();
        for item in &mut items {
            item.light = self
                .sample_block_light(entity_light_block_pos(item.position))
                .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT);
        }
        items
    }

    /// Firework rocket event `17` uses `ClientLevel.createFireworks` with the
    /// rocket's current position, delta movement, and decoded `minecraft:fireworks`
    /// explosion list.
    pub fn firework_rocket_explosion_particle_state(
        &self,
        id: i32,
    ) -> Option<FireworkRocketExplosionParticleState> {
        self.entities.firework_rocket_explosion_particle_state(id)
    }

    /// Fox entity event `45` breaks the item held in the fox's main hand at the
    /// mouth, using the current body pitch/yaw for the velocity fan.
    pub fn fox_eat_particle_state(&self, id: i32) -> Option<FoxEatParticleState> {
        self.entities.fox_eat_particle_state(id)
    }

    /// The item-cluster render state for ominous item spawner entities (vanilla
    /// `OminousItemSpawnerRenderer`), projected with entity `ageInTicks` for scale-in and spin.
    pub fn ominous_item_spawner_item_states_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<OminousItemSpawnerItemState> {
        self.entities
            .ominous_item_spawner_item_states_at_partial_tick(partial_ticks)
    }

    fn item_stacks_with_sampled_light(
        &self,
        mut items: Vec<ItemEntityStackState>,
    ) -> Vec<ItemEntityStackState> {
        for item in &mut items {
            item.light = self
                .sample_block_light(entity_light_block_pos(item.position))
                .unwrap_or(ENTITY_LIGHT_PROBE_FULL_BRIGHT);
        }
        items
    }

    pub fn local_player_id(&self) -> Option<i32> {
        self.local_player_id
    }

    pub fn local_player_main_arm_left(&self) -> Option<bool> {
        self.entities.main_arm_left(self.local_player_id?)
    }

    pub fn local_player_attack_swing(
        &self,
        partial_ticks: f32,
    ) -> Option<LocalPlayerAttackSwingState> {
        self.entities
            .attack_swing_state(self.local_player_id?, partial_ticks)
    }

    pub fn local_player_ticks_since_kinetic_hit_feedback(&self, partial_ticks: f32) -> f32 {
        self.local_player_id
            .and_then(|id| {
                self.entities
                    .ticks_since_kinetic_hit_feedback(id, partial_ticks)
            })
            .unwrap_or(0.0)
    }

    /// The fishing bobber currently owned by the local player, if any. Vanilla
    /// `FishingHook.getAddEntityPacket` stores the owner id in
    /// `ClientboundAddEntityPacket.data`, and `FishingHook.setOwner` writes it
    /// back to `Player.fishing` on the client.
    pub fn local_player_fishing_bobber_id(&self) -> Option<i32> {
        let local_player_id = self.local_player_id?;
        self.entities.first_entity_id_with_type_and_data(
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            local_player_id,
        )
    }

    /// Vanilla `FishingHookRenderer.extractRenderState` inputs for one tracked
    /// fishing bobber and its player owner, sampled at the same partial tick.
    pub fn fishing_hook_render_source_at_partial_tick(
        &self,
        hook_id: i32,
        partial_ticks: f32,
    ) -> Option<FishingHookRenderSourceState> {
        let partial_ticks = partial_ticks.clamp(0.0, 1.0);
        let mut source = self
            .entities
            .fishing_hook_render_source_at_partial_tick(hook_id, partial_ticks)?;
        let game_time = self.world_time().map_or(0, |time| time.game_time);
        let minecart_improvements_enabled =
            self.is_feature_enabled("minecraft:minecart_improvements");
        source.hook_position = self.entities.render_position_at_partial_tick(
            hook_id,
            partial_ticks,
            game_time,
            minecart_improvements_enabled,
        )?;
        let owner_transform = self.entities.transform(source.owner_id)?;
        let owner_position = owner_transform.render_position(partial_ticks);
        let projected_owner_position = self.entities.render_position_at_partial_tick(
            source.owner_id,
            partial_ticks,
            game_time,
            minecart_improvements_enabled,
        )?;
        source.owner_eye_position.x += projected_owner_position.x - owner_position.x;
        source.owner_eye_position.y += projected_owner_position.y - owner_position.y;
        source.owner_eye_position.z += projected_owner_position.z - owner_position.z;
        Some(source)
    }

    pub fn local_player_vehicle_id(&self) -> Option<i32> {
        self.local_player_vehicle_id
    }

    pub fn removed_player_vehicle_id(&self) -> Option<i32> {
        self.removed_player_vehicle_id
    }

    /// Vanilla `Gui.getPlayerVehicleWithHealth` + the `extractVehicleHealth` /
    /// `getVehicleMaxHearts` inputs (Gui.java:709-737,974-979): the local
    /// player's direct vehicle when it is a `LivingEntity` (which is exactly
    /// vanilla's `showVehicleHealth()` — the `Entity` base returns
    /// `this instanceof LivingEntity` (Entity.java:2349-2351) and 26.1 has no
    /// overrides). `health` is the synced `DATA_HEALTH_ID` float
    /// (`LivingEntity.getHealth`); `max_health` is the synced MAX_HEALTH
    /// attribute folded through `AttributeInstance.calculateValue`
    /// (`vehicle.getMaxHealth()`, Gui.java:727). Boats/minecarts (non-living
    /// vehicles) yield `None`, matching vanilla's hidden vehicle bar.
    pub fn local_player_vehicle_health(&self) -> Option<LocalPlayerVehicleHealth> {
        let vehicle_id = self.local_player_vehicle_id?;
        let entity_type_id = self.entities.entity_type_id(vehicle_id)?;
        if !vanilla_living_entity_type(entity_type_id) {
            return None;
        }
        let health = self.entities.living_entity_health(vehicle_id)?;
        // Vanilla reads the attribute map; every living entity's syncable
        // MAX_HEALTH instance is sent on tracking start
        // (`ServerEntity.sendPairingData`, ServerEntity.java:282-284). The
        // 20.0 fallback is the `Attributes.MAX_HEALTH` `RangedAttribute`
        // default (Attributes.java:58-60) for the not-yet-synced window.
        let max_health = self
            .entities
            .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            .unwrap_or(VANILLA_DEFAULT_MAX_HEALTH);
        Some(LocalPlayerVehicleHealth { health, max_health })
    }

    /// Vanilla `player.getAttributeValue(Attributes.MAX_HEALTH)`, the max the
    /// heart row scales its container count / row count against
    /// (`Gui.extractPlayerHealth`, Gui.java:768). Reads the synced MAX_HEALTH
    /// attribute (registry index 19) folded through
    /// `AttributeInstance.calculateValue`, falling back to the
    /// `Attributes.MAX_HEALTH` `RangedAttribute` default 20.0 while unsynced.
    pub fn local_player_max_health(&self) -> f32 {
        self.local_player_id
            .and_then(|id| {
                self.entities
                    .attribute_value(id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            })
            .unwrap_or(VANILLA_DEFAULT_MAX_HEALTH) as f32
    }

    /// Vanilla `player.isFullyFrozen()` (`getTicksFrozen() >= 140`), the
    /// `Gui.HeartType.forPlayer` frozen-heart gate (Gui.java:1444). Reads the
    /// local player's synced `DATA_TICKS_FROZEN_ID`.
    pub fn local_player_is_fully_frozen(&self) -> bool {
        self.local_player_id
            .map(|id| self.entities.is_fully_frozen(id))
            .unwrap_or(false)
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

/// Vanilla `Cat.handleLieDown`: starts with `new AABB(cat.blockPosition()).inflate(2.0, 2.0, 2.0)`
/// and checks for sleeping players inside that search box.
fn cat_sleeping_player_search_aabb(cat_position: EntityVec3) -> ([f64; 3], [f64; 3]) {
    let x = cat_position.x.floor();
    let y = cat_position.y.floor();
    let z = cat_position.z.floor();
    ([x - 2.0, y - 2.0, z - 2.0], [x + 3.0, y + 3.0, z + 3.0])
}

fn model_target_world_aabb(target: EntityRenderCandidateState) -> ([f64; 3], [f64; 3]) {
    (
        [
            target.position.x + f64::from(target.bounds.min[0]),
            target.position.y + f64::from(target.bounds.min[1]),
            target.position.z + f64::from(target.bounds.min[2]),
        ],
        [
            target.position.x + f64::from(target.bounds.max[0]),
            target.position.y + f64::from(target.bounds.max[1]),
            target.position.z + f64::from(target.bounds.max[2]),
        ],
    )
}

fn floor_to_i32(value: f64) -> i32 {
    value.floor() as i32
}

fn vec3_f64_to_f32_array(value: [f64; 3]) -> [f32; 3] {
    [value[0] as f32, value[1] as f32, value[2] as f32]
}

fn aabb_intersects(min: [f64; 3], max: [f64; 3], other_min: [f64; 3], other_max: [f64; 3]) -> bool {
    min[0] < other_max[0]
        && max[0] > other_min[0]
        && min[1] < other_max[1]
        && max[1] > other_min[1]
        && min[2] < other_max[2]
        && max[2] > other_min[2]
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
    if !block_name_is_bed(block_name) {
        return None;
    }
    let (yaw, step_x, step_z) = sleep_direction_rotation_and_step(properties.get("facing")?)?;
    let head_offset = standing_eye_height - 0.1;
    Some((yaw, [-step_x * head_offset, -step_z * head_offset]))
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

fn block_pos_closer_to_center_than(pos: BlockPos, point: EntityVec3, distance: f64) -> bool {
    // Vanilla `Vec3i.closerToCenterThan`: strict squared-distance comparison from
    // the block center `(x + 0.5, y + 0.5, z + 0.5)`.
    let dx = f64::from(pos.x) + 0.5 - point.x;
    let dy = f64::from(pos.y) + 0.5 - point.y;
    let dz = f64::from(pos.z) + 0.5 - point.z;
    dx * dx + dy * dy + dz * dz < distance * distance
}

fn honey_block_state_id() -> Option<i32> {
    crate::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:honey_block", &BTreeMap::new())
        .map(|state| state.id)
}

fn item_entity_stack_mut(
    data_values: &mut [ProtocolEntityDataValue],
    data_id: u8,
) -> Option<&mut ProtocolItemStackSummary> {
    data_values.iter_mut().find_map(|value| {
        if value.data_id == data_id {
            if let EntityDataValueKind::ItemStack(stack) = &mut value.value {
                return Some(stack);
            }
        }
        None
    })
}

#[cfg(test)]
mod tests;
