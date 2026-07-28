pub use crate::advancements::*;
pub use crate::client::state::*;
pub use crate::climate::{biome_height_adjusted_temperature, biome_info_noise};
pub use crate::command_blocks::*;
pub use crate::entities::experience_orb_icon;
pub use crate::entities::state::*;
pub use crate::entities::vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type;
pub use crate::entities::vanilla_entity_metadata_type_death_sound_policy;
pub use crate::entities::vanilla_entity_renderer_guardian_kind;
pub use crate::entities::vanilla_entity_renderer_hoglin_kind;
pub use crate::entities::vanilla_entity_renderer_piglin_kind;
pub use crate::entities::vanilla_entity_renderer_squid_kind;
pub use crate::entities::EntityLeashRenderState;
pub use crate::entities::EntityNameTagLivingVisibilityState;
pub use crate::entities::EntityNameTagRenderSourceState;
pub use crate::entities::EntityNameTagTextSegmentState;
pub use crate::entities::EntityNameTagTextState;
pub use crate::entities::EntityNameTagVisibilityPolicy;
pub use crate::entities::EntityRenderCandidateState;
pub use crate::entities::EntityRendererSpecialCullingState;
pub use crate::entities::FishingHookRenderSourceState;
pub use crate::entities::LightningBoltRenderSourceState;
pub use crate::entities::VanillaAbstractBoatMetadataAccessors;
pub use crate::entities::VanillaAbstractHorseMetadataAccessors;
pub use crate::entities::VanillaAbstractMinecartMetadataAccessors;
pub use crate::entities::VanillaArrowMetadataAccessors;
pub use crate::entities::VanillaBoatRendererFamily;
pub use crate::entities::VanillaBoatRendererKind;
pub use crate::entities::VanillaCamelRendererKind;
pub use crate::entities::VanillaCompanionVariantMetadataAccessors;
pub use crate::entities::VanillaCopperGolemMetadataAccessors;
pub use crate::entities::VanillaCreeperMetadataAccessors;
pub use crate::entities::VanillaEndermanRenderMetadataAccessors;
pub use crate::entities::VanillaEntityBlockEffectTickPolicy;
pub use crate::entities::VanillaEntityDeathSoundPolicy;
pub use crate::entities::VanillaEntityHurtSoundPolicy;
pub use crate::entities::VanillaEntitySoundCondition;
pub use crate::entities::VanillaEntitySoundVariantFamily;
pub use crate::entities::VanillaEntitySoundVolumePolicy;
pub use crate::entities::VanillaFarmAnimalVariantMetadataAccessors;
pub use crate::entities::VanillaFoxMetadataAccessors;
pub use crate::entities::VanillaGoatMetadataAccessors;
pub use crate::entities::VanillaGuardianRendererKind;
pub use crate::entities::VanillaHappyGhastMetadataAccessors;
pub use crate::entities::VanillaHoglinRendererKind;
pub use crate::entities::VanillaHorseRendererKind;
pub use crate::entities::VanillaIllagerRendererKind;
pub use crate::entities::VanillaInteractionMetadataAccessors;
pub use crate::entities::VanillaLivingEntityMetadataAccessors;
pub use crate::entities::VanillaLlamaMetadataAccessors;
pub use crate::entities::VanillaLlamaRendererKind;
pub use crate::entities::VanillaMobMetadataAccessors;
pub use crate::entities::VanillaPandaMetadataAccessors;
pub use crate::entities::VanillaPiglinRendererKind;
pub use crate::entities::VanillaShulkerMetadataAccessors;
pub use crate::entities::VanillaSkeletonRendererKind;
pub use crate::entities::VanillaSquidRendererKind;
pub use crate::entities::VanillaTamableAnimalMetadataAccessors;
pub use crate::entities::VanillaThrownTridentMetadataAccessors;
pub use crate::entities::VanillaTropicalFishVariantAccessors;
pub use crate::entities::VanillaTurtleMetadataAccessors;
pub use crate::entities::VanillaUndeadHorseRendererKind;
pub use crate::entities::VanillaVehicleMetadataAccessors;
pub use crate::entities::VanillaZombieMetadataAccessors;
pub use crate::entities::VanillaZombieRendererKind;
pub use crate::entities::VANILLA_ENDER_DRAGON_PHASE_HOLDING_PATTERN_ID;
pub use crate::entities::VANILLA_EXPERIENCE_ORB_DEFAULT_VALUE;
pub use crate::entities::VANILLA_IRON_GOLEM_FLAG_PLAYER_CREATED;
pub use crate::entities::{
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
    vanilla_entity_metadata_type_block_effect_tick_policy,
    vanilla_entity_metadata_type_hurt_sound_policy,
    vanilla_entity_metadata_type_is_abstract_chested_horse, vanilla_entity_metadata_type_is_camel,
    vanilla_entity_metadata_type_is_guardian, vanilla_entity_metadata_type_is_leashable,
    vanilla_entity_metadata_type_is_llama, vanilla_entity_metadata_type_is_slime,
    vanilla_entity_metadata_type_sound_volume_policy,
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
    vanilla_entity_type_has_attribute, vanilla_entity_type_illager_arm_pose_uses_celebrating,
    vanilla_entity_type_is_abstract_piglin, vanilla_entity_type_is_boat,
    vanilla_entity_type_is_chest_boat, vanilla_entity_type_is_hurting_projectile,
    vanilla_entity_type_is_item_frame, vanilla_entity_type_is_living,
    vanilla_entity_type_is_minecart, vanilla_entity_type_is_spellcaster_illager,
    vanilla_entity_type_is_spider, vanilla_entity_type_is_squid,
    vanilla_entity_type_is_wind_charge, EntityCameraHurtState, LocalPlayerVehicleHealth,
    RemovedPlayerVehicleTeleportRecovery, VanillaAbstractArrowMetadataAccessors,
    VanillaArmorStandMetadataAccessors, VanillaEntityInterpolationPolicy,
    VanillaEntityMetadataAccessorCatalogEntry, VanillaEntityMetadataAccessors,
    VanillaEntityMetadataClassCatalogEntry, VanillaEntityMetadataDefault,
    VanillaEntityMetadataFamily, VanillaEntityMetadataFamilySchema,
    VanillaEntityMetadataFieldSchema, VanillaEntityMetadataFlagCatalogEntry,
    VanillaEntityMetadataFlagSchema, VanillaEntityMetadataFlagStatus,
    VanillaEntityMetadataResolverEvidence, VanillaEntityRendererCullingKind,
    VanillaEntityTypeMetadataCatalogEntry, VANILLA_AGEABLE_MOB_AGE_LOCKED_DATA_ID,
    VANILLA_AGEABLE_MOB_BABY_DATA_ID, VANILLA_AGEABLE_MOB_DEFAULT_AGE_LOCKED,
    VANILLA_AGEABLE_MOB_DEFAULT_BABY, VANILLA_ARMADILLO_STATE_IDLE_ID,
    VANILLA_ARMADILLO_STATE_ROLLING_ID, VANILLA_ARMADILLO_STATE_SCARED_ID,
    VANILLA_ARMADILLO_STATE_UNROLLING_ID, VANILLA_ARMOR_STAND_BODY_POSE_DATA_ID,
    VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID, VANILLA_ARMOR_STAND_CLIENT_FLAG_MARKER,
    VANILLA_ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE, VANILLA_ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS,
    VANILLA_ARMOR_STAND_CLIENT_FLAG_SMALL, VANILLA_ARMOR_STAND_DEFAULT_BODY_POSE,
    VANILLA_ARMOR_STAND_DEFAULT_CLIENT_FLAGS, VANILLA_ARMOR_STAND_DEFAULT_HEAD_POSE,
    VANILLA_ARMOR_STAND_DEFAULT_LEFT_ARM_POSE, VANILLA_ARMOR_STAND_DEFAULT_LEFT_LEG_POSE,
    VANILLA_ARMOR_STAND_DEFAULT_RIGHT_ARM_POSE, VANILLA_ARMOR_STAND_DEFAULT_RIGHT_LEG_POSE,
    VANILLA_ARMOR_STAND_HEAD_POSE_DATA_ID, VANILLA_ARMOR_STAND_LEFT_ARM_POSE_DATA_ID,
    VANILLA_ARMOR_STAND_LEFT_LEG_POSE_DATA_ID, VANILLA_ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID,
    VANILLA_ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID, VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION,
    VANILLA_AVATAR_MAIN_HAND_DATA_ID, VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
    VANILLA_AVATAR_MODEL_PART_CAPE, VANILLA_AVATAR_MODEL_PART_HAT,
    VANILLA_AVATAR_MODEL_PART_JACKET, VANILLA_AVATAR_MODEL_PART_LEFT_PANTS_LEG,
    VANILLA_AVATAR_MODEL_PART_LEFT_SLEEVE, VANILLA_AVATAR_MODEL_PART_RIGHT_PANTS_LEG,
    VANILLA_AVATAR_MODEL_PART_RIGHT_SLEEVE, VANILLA_ENTITY_AIR_SUPPLY_DATA_ID,
    VANILLA_ENTITY_CUSTOM_NAME_DATA_ID, VANILLA_ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID,
    VANILLA_ENTITY_DEFAULT_MAX_AIR_SUPPLY_TICKS, VANILLA_ENTITY_METADATA_ACCESSORS_26_1,
    VANILLA_ENTITY_METADATA_CLASSES_26_1, VANILLA_ENTITY_METADATA_DEFAULT_RESOLVER_EVIDENCE_26_1,
    VANILLA_ENTITY_METADATA_FLAGS_26_1, VANILLA_ENTITY_METADATA_FLAG_RESOLVER_EVIDENCE_26_1,
    VANILLA_ENTITY_NO_GRAVITY_DATA_ID, VANILLA_ENTITY_POSE_DATA_ID,
    VANILLA_ENTITY_SHARED_FLAGS_DATA_ID, VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING,
    VANILLA_ENTITY_SHARED_FLAG_GLOWING, VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
    VANILLA_ENTITY_SHARED_FLAG_ON_FIRE, VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
    VANILLA_ENTITY_SHARED_FLAG_SPRINTING, VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
    VANILLA_ENTITY_SILENT_DATA_ID, VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
    VANILLA_ENTITY_TYPE_METADATA_26_1, VANILLA_EXPERIENCE_ORB_VALUE_DATA_ID,
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
pub use crate::entities::{
    vanilla_entity_metadata_bee_accessors_for_entity_type, VanillaBeeMetadataAccessors,
};
pub use crate::entities::{
    vanilla_entity_metadata_cat_accessors_for_entity_type, VanillaCatMetadataAccessors,
};
pub use crate::entities::{
    vanilla_entity_metadata_dolphin_accessors_for_entity_type, VanillaDolphinMetadataAccessors,
};
pub use crate::entities::{
    vanilla_entity_metadata_end_crystal_accessors_for_entity_type,
    vanilla_entity_metadata_hanging_direction_accessor_for_entity_type,
    vanilla_entity_metadata_item_frame_accessors_for_entity_type,
    vanilla_entity_metadata_painting_variant_accessor_for_entity_type,
    VanillaEndCrystalMetadataAccessors, VanillaHangingEntityMetadataAccessor,
    VanillaItemFrameMetadataAccessors, VANILLA_END_CRYSTAL_BEAM_TARGET_DATA_ID,
    VANILLA_END_CRYSTAL_DEFAULT_SHOW_BOTTOM, VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID,
};
pub use crate::entities::{
    vanilla_entity_metadata_firework_rocket_accessors_for_entity_type,
    VanillaFireworkRocketMetadataAccessors,
};
pub use crate::entities::{
    vanilla_entity_metadata_primed_tnt_accessors_for_entity_type, VanillaPrimedTntMetadataAccessors,
};
pub use crate::entities::{
    vanilla_entity_metadata_wolf_accessors_for_entity_type, VanillaWolfMetadataAccessors,
};
pub use crate::entities::{
    vanilla_entity_renderer_presentation_owner, VanillaEntityRendererPresentationOwner,
};
pub use crate::entities::{vanilla_entity_scale_sanitizer, VanillaEntityScaleSanitizer};
pub use crate::entities::{
    vanilla_minecart_default_display_offset,
    VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID,
    VANILLA_ABSTRACT_MINECART_DISPLAY_OFFSET_DATA_ID,
    VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
    VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID, VANILLA_FURNACE_MINECART_DEFAULT_FUEL,
    VANILLA_FURNACE_MINECART_FUEL_DATA_ID,
};
pub use crate::entities::{
    CatSoundVariantIdentity, ChickenSoundVariantIdentity, CowSoundVariantIdentity,
    PigSoundVariantIdentity, WolfSoundVariantIdentity,
};
pub use crate::entities::{
    DisplayBillboardConstraints, DisplayEntityCullingState, DisplayEntityRenderContent,
    DisplayEntityRenderSourceState, DisplayItemContext, DisplayTransformation,
};
pub use crate::entities::{EntityClientTickWork, EntityPublicationSnapshotObservation};
pub use crate::entities::{EntityShadowPieceState, EntityShadowRenderSourceState};
pub use crate::entities::{
    VanillaEntityEventHandlerCatalogEntry, VANILLA_ENTITY_EVENT_HANDLERS_26_1,
};
pub use crate::entities::{
    VANILLA_ABSTRACT_ARROW_DEFAULT_FLAGS, VANILLA_ABSTRACT_ARROW_DEFAULT_IN_GROUND,
    VANILLA_ABSTRACT_ARROW_DEFAULT_PIERCE_LEVEL, VANILLA_ABSTRACT_ARROW_FLAGS_DATA_ID,
    VANILLA_ABSTRACT_ARROW_FLAG_CRIT, VANILLA_ABSTRACT_ARROW_FLAG_NO_PHYSICS,
    VANILLA_ABSTRACT_ARROW_IN_GROUND_DATA_ID, VANILLA_ABSTRACT_ARROW_PIERCE_LEVEL_DATA_ID,
    VANILLA_ARROW_DEFAULT_EFFECT_COLOR, VANILLA_ARROW_EFFECT_COLOR_DATA_ID,
    VANILLA_TRIDENT_DEFAULT_FOIL, VANILLA_TRIDENT_DEFAULT_LOYALTY, VANILLA_TRIDENT_FOIL_DATA_ID,
    VANILLA_TRIDENT_LOYALTY_DATA_ID, VANILLA_WITHER_SKULL_DANGEROUS_DATA_ID,
    VANILLA_WITHER_SKULL_DEFAULT_DANGEROUS,
};
pub use crate::entities::{
    VANILLA_ABSTRACT_BOAT_BUBBLE_TIME_DATA_ID, VANILLA_ABSTRACT_BOAT_DEFAULT_BUBBLE_TIME,
    VANILLA_ABSTRACT_BOAT_DEFAULT_PADDLE_LEFT, VANILLA_ABSTRACT_BOAT_DEFAULT_PADDLE_RIGHT,
    VANILLA_ABSTRACT_BOAT_PADDLE_LEFT_DATA_ID, VANILLA_ABSTRACT_BOAT_PADDLE_RIGHT_DATA_ID,
};
pub use crate::entities::{
    VANILLA_ABSTRACT_CHESTED_HORSE_CHEST_DATA_ID, VANILLA_ABSTRACT_CHESTED_HORSE_DEFAULT_CHEST,
    VANILLA_HORSE_COLOR_MASK, VANILLA_HORSE_DEFAULT_VARIANT, VANILLA_HORSE_MARKINGS_MASK,
    VANILLA_HORSE_MARKINGS_SHIFT, VANILLA_HORSE_VARIANT_DATA_ID, VANILLA_LLAMA_DEFAULT_STRENGTH,
    VANILLA_LLAMA_DEFAULT_VARIANT, VANILLA_LLAMA_STRENGTH_DATA_ID, VANILLA_LLAMA_VARIANT_DATA_ID,
};
pub use crate::entities::{
    VANILLA_ABSTRACT_FISH_DEFAULT_FROM_BUCKET, VANILLA_ABSTRACT_FISH_FROM_BUCKET_DATA_ID,
};
pub use crate::entities::{
    VANILLA_ABSTRACT_HORSE_DEFAULT_FLAGS, VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID,
    VANILLA_ABSTRACT_HORSE_FLAG_BRED, VANILLA_ABSTRACT_HORSE_FLAG_EATING,
    VANILLA_ABSTRACT_HORSE_FLAG_OPEN_MOUTH, VANILLA_ABSTRACT_HORSE_FLAG_STANDING,
    VANILLA_ABSTRACT_HORSE_FLAG_TAME,
};
pub use crate::entities::{
    VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID, VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH,
    VANILLA_ZOMBIE_NAUTILUS_DEFAULT_VARIANT_RESOURCE_ID, VANILLA_ZOMBIE_NAUTILUS_VARIANT_DATA_ID,
};
pub use crate::entities::{
    VANILLA_ABSTRACT_VILLAGER_DEFAULT_UNHAPPY_COUNTER,
    VANILLA_ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID,
};
pub use crate::entities::{
    VANILLA_ALLAY_CAN_DUPLICATE_DATA_ID, VANILLA_ALLAY_DEFAULT_CAN_DUPLICATE,
};
pub use crate::entities::{
    VANILLA_AXOLOTL_DEFAULT_FROM_BUCKET, VANILLA_AXOLOTL_FROM_BUCKET_DATA_ID,
};
pub use crate::entities::{
    VANILLA_BAT_DEFAULT_FLAGS, VANILLA_BAT_FLAGS_DATA_ID, VANILLA_BAT_FLAG_RESTING,
};
pub use crate::entities::{
    VANILLA_BEE_ANGER_END_TIME_DATA_ID, VANILLA_BEE_DEFAULT_ANGER_END_TIME,
    VANILLA_BEE_DEFAULT_FLAGS, VANILLA_BEE_FLAGS_DATA_ID, VANILLA_BEE_FLAG_HAS_NECTAR,
    VANILLA_BEE_FLAG_HAS_STUNG, VANILLA_BEE_FLAG_ROLL,
};
pub use crate::entities::{
    VANILLA_CAT_DEFAULT_SOUND_VARIANT_RESOURCE_ID, VANILLA_CAT_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_CAT_SOUND_VARIANT_DATA_ID, VANILLA_CAT_VARIANT_DATA_ID,
};
pub use crate::entities::{
    VANILLA_CHICKEN_DEFAULT_SOUND_VARIANT_RESOURCE_ID, VANILLA_CHICKEN_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_CHICKEN_SOUND_VARIANT_DATA_ID, VANILLA_CHICKEN_VARIANT_DATA_ID,
};
pub use crate::entities::{
    VANILLA_COPPER_GOLEM_STATE_DROPPING_ITEM_ID, VANILLA_COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_GETTING_ITEM_ID, VANILLA_COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_IDLE_ID, VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_WEATHERED_ID,
};
pub use crate::entities::{
    VANILLA_COW_DEFAULT_SOUND_VARIANT_RESOURCE_ID, VANILLA_COW_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_COW_SOUND_VARIANT_DATA_ID, VANILLA_COW_VARIANT_DATA_ID,
};
pub use crate::entities::{
    VANILLA_CREEPER_DEFAULT_IGNITED, VANILLA_CREEPER_DEFAULT_POWERED,
    VANILLA_CREEPER_DEFAULT_SWELL_DIR, VANILLA_CREEPER_IGNITED_DATA_ID,
    VANILLA_CREEPER_POWERED_DATA_ID, VANILLA_CREEPER_SWELL_DIR_DATA_ID,
};
pub use crate::entities::{
    VANILLA_DOLPHIN_DEFAULT_GOT_FISH, VANILLA_DOLPHIN_DEFAULT_MOISTNESS,
    VANILLA_DOLPHIN_GOT_FISH_DATA_ID, VANILLA_DOLPHIN_MOISTNESS_DATA_ID,
};
pub use crate::entities::{
    VANILLA_ENDERMAN_CARRY_STATE_DATA_ID, VANILLA_ENDERMAN_CREEPY_DATA_ID,
    VANILLA_ENDERMAN_DEFAULT_CARRY_STATE, VANILLA_ENDERMAN_DEFAULT_CREEPY,
    VANILLA_ENDERMAN_DEFAULT_STARED_AT, VANILLA_ENDERMAN_STARED_AT_DATA_ID,
};
pub use crate::entities::{
    VANILLA_FIREWORK_ROCKET_ATTACHED_TO_TARGET_DATA_ID,
    VANILLA_FIREWORK_ROCKET_DEFAULT_ITEM_RESOURCE_ID,
    VANILLA_FIREWORK_ROCKET_DEFAULT_SHOT_AT_ANGLE, VANILLA_FIREWORK_ROCKET_ITEM_DATA_ID,
    VANILLA_FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID,
};
pub use crate::entities::{
    VANILLA_FOX_DEFAULT_FLAGS, VANILLA_FOX_DEFAULT_TYPE, VANILLA_FOX_FLAGS_DATA_ID,
    VANILLA_FOX_FLAG_CROUCHING, VANILLA_FOX_FLAG_DEFENDING, VANILLA_FOX_FLAG_FACEPLANTED,
    VANILLA_FOX_FLAG_INTERESTED, VANILLA_FOX_FLAG_POUNCING, VANILLA_FOX_FLAG_SITTING,
    VANILLA_FOX_FLAG_SLEEPING, VANILLA_FOX_TRUSTED_DATA_ID_0, VANILLA_FOX_TRUSTED_DATA_ID_1,
    VANILLA_FOX_TYPE_DATA_ID,
};
pub use crate::entities::{
    VANILLA_FROG_DEFAULT_VARIANT_RESOURCE_ID, VANILLA_FROG_TONGUE_TARGET_DATA_ID,
    VANILLA_FROG_VARIANT_DATA_ID,
};
pub use crate::entities::{VANILLA_GHAST_CHARGING_DATA_ID, VANILLA_GHAST_DEFAULT_CHARGING};
pub use crate::entities::{
    VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID, VANILLA_GLOW_SQUID_DEFAULT_DARK_TICKS,
};
pub use crate::entities::{
    VANILLA_GOAT_DEFAULT_HAS_LEFT_HORN, VANILLA_GOAT_DEFAULT_HAS_RIGHT_HORN,
    VANILLA_GOAT_DEFAULT_SCREAMING, VANILLA_GOAT_LEFT_HORN_DATA_ID,
    VANILLA_GOAT_RIGHT_HORN_DATA_ID, VANILLA_GOAT_SCREAMING_DATA_ID,
};
pub use crate::entities::{
    VANILLA_HAPPY_GHAST_DEFAULT_IS_LEASH_HOLDER, VANILLA_HAPPY_GHAST_DEFAULT_STAYS_STILL,
    VANILLA_HAPPY_GHAST_IS_LEASH_HOLDER_DATA_ID, VANILLA_HAPPY_GHAST_STAYS_STILL_DATA_ID,
};
pub use crate::entities::{
    VANILLA_HOGLIN_DEFAULT_IMMUNE_TO_ZOMBIFICATION, VANILLA_HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID,
};
pub use crate::entities::{
    VANILLA_INTERACTION_DEFAULT_HEIGHT, VANILLA_INTERACTION_DEFAULT_RESPONSE,
    VANILLA_INTERACTION_DEFAULT_WIDTH, VANILLA_INTERACTION_HEIGHT_DATA_ID,
    VANILLA_INTERACTION_RESPONSE_DATA_ID, VANILLA_INTERACTION_WIDTH_DATA_ID,
};
pub use crate::entities::{VANILLA_OCELOT_DEFAULT_TRUSTING, VANILLA_OCELOT_TRUSTING_DATA_ID};
pub use crate::entities::{
    VANILLA_PANDA_DEFAULT_EAT_COUNTER, VANILLA_PANDA_DEFAULT_FLAGS,
    VANILLA_PANDA_DEFAULT_HIDDEN_GENE, VANILLA_PANDA_DEFAULT_MAIN_GENE,
    VANILLA_PANDA_DEFAULT_SNEEZE_COUNTER, VANILLA_PANDA_DEFAULT_UNHAPPY_COUNTER,
    VANILLA_PANDA_EAT_COUNTER_DATA_ID, VANILLA_PANDA_FLAGS_DATA_ID, VANILLA_PANDA_FLAG_ON_BACK,
    VANILLA_PANDA_FLAG_ROLLING, VANILLA_PANDA_FLAG_SITTING, VANILLA_PANDA_FLAG_SNEEZING,
    VANILLA_PANDA_HIDDEN_GENE_DATA_ID, VANILLA_PANDA_MAIN_GENE_DATA_ID,
    VANILLA_PANDA_SNEEZE_COUNTER_DATA_ID, VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID,
};
pub use crate::entities::{
    VANILLA_PIG_BOOST_TIME_DATA_ID, VANILLA_PIG_DEFAULT_BOOST_TIME,
    VANILLA_PIG_DEFAULT_SOUND_VARIANT_RESOURCE_ID, VANILLA_PIG_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_PIG_SOUND_VARIANT_DATA_ID, VANILLA_PIG_VARIANT_DATA_ID,
};
pub use crate::entities::{
    VANILLA_PRIMED_TNT_BLOCK_STATE_DATA_ID, VANILLA_PRIMED_TNT_DEFAULT_BLOCK_STATE,
    VANILLA_PRIMED_TNT_DEFAULT_FUSE, VANILLA_PRIMED_TNT_FUSE_DATA_ID,
};
pub use crate::entities::{
    VANILLA_SKELETON_DEFAULT_STRAY_CONVERSION, VANILLA_SKELETON_STRAY_CONVERSION_DATA_ID,
};
pub use crate::entities::{
    VANILLA_SNIFFER_DEFAULT_DROP_SEED_AT_TICK, VANILLA_SNIFFER_DROP_SEED_AT_TICK_DATA_ID,
    VANILLA_SNIFFER_STATE_DIGGING_ID, VANILLA_SNIFFER_STATE_FEELING_HAPPY_ID,
    VANILLA_SNIFFER_STATE_IDLING_ID, VANILLA_SNIFFER_STATE_RISING_ID,
    VANILLA_SNIFFER_STATE_SCENTING_ID, VANILLA_SNIFFER_STATE_SEARCHING_ID,
    VANILLA_SNIFFER_STATE_SNIFFING_ID,
};
pub use crate::entities::{
    VANILLA_STRIDER_BOOST_TIME_DATA_ID, VANILLA_STRIDER_DEFAULT_BOOST_TIME,
    VANILLA_STRIDER_DEFAULT_SUFFOCATING, VANILLA_STRIDER_SUFFOCATING_DATA_ID,
};
pub use crate::entities::{VANILLA_TADPOLE_AGE_LOCKED_DATA_ID, VANILLA_TADPOLE_DEFAULT_AGE_LOCKED};
pub use crate::entities::{
    VANILLA_TAMABLE_ANIMAL_DEFAULT_FLAGS, VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID,
    VANILLA_TAMABLE_ANIMAL_FLAG_SITTING, VANILLA_TAMABLE_ANIMAL_FLAG_TAME,
    VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID,
};
pub use crate::entities::{
    VANILLA_TURTLE_DEFAULT_HAS_EGG, VANILLA_TURTLE_DEFAULT_LAYING_EGG,
    VANILLA_TURTLE_HAS_EGG_DATA_ID, VANILLA_TURTLE_LAYING_EGG_DATA_ID,
};
pub use crate::entities::{
    VANILLA_VEHICLE_DAMAGE_DATA_ID, VANILLA_VEHICLE_DEFAULT_DAMAGE,
    VANILLA_VEHICLE_DEFAULT_HURT_DIR, VANILLA_VEHICLE_DEFAULT_HURT_TIME,
    VANILLA_VEHICLE_HURT_DIR_DATA_ID, VANILLA_VEHICLE_HURT_TIME_DATA_ID,
};
pub use crate::entities::{
    VANILLA_VEX_DEFAULT_FLAGS, VANILLA_VEX_FLAGS_DATA_ID, VANILLA_VEX_FLAG_CHARGING,
};
pub use crate::entities::{
    VANILLA_VILLAGER_DATA_DATA_ID, VANILLA_VILLAGER_DATA_FINALIZED_DATA_ID,
    VANILLA_VILLAGER_DEFAULT_DATA_FINALIZED, VANILLA_ZOMBIE_VILLAGER_CONVERTING_DATA_ID,
    VANILLA_ZOMBIE_VILLAGER_DATA_DATA_ID, VANILLA_ZOMBIE_VILLAGER_DATA_FINALIZED_DATA_ID,
    VANILLA_ZOMBIE_VILLAGER_DEFAULT_CONVERTING, VANILLA_ZOMBIE_VILLAGER_DEFAULT_DATA_FINALIZED,
};
pub use crate::entities::{
    VANILLA_WITHER_DEFAULT_INVULNERABLE_TICKS, VANILLA_WITHER_DEFAULT_TARGET_A,
    VANILLA_WITHER_INVULNERABLE_TICKS_DATA_ID, VANILLA_WITHER_TARGET_A_DATA_ID,
};
pub use crate::entities::{
    VANILLA_WOLF_DEFAULT_SOUND_VARIANT_RESOURCE_ID, VANILLA_WOLF_DEFAULT_VARIANT_RESOURCE_ID,
    VANILLA_WOLF_SOUND_VARIANT_DATA_ID, VANILLA_WOLF_VARIANT_DATA_ID,
};
pub use crate::entities::{VANILLA_ZOGLIN_BABY_DATA_ID, VANILLA_ZOGLIN_DEFAULT_BABY};
pub use crate::entities::{
    VANILLA_ZOMBIE_BABY_DATA_ID, VANILLA_ZOMBIE_DEFAULT_BABY,
    VANILLA_ZOMBIE_DEFAULT_DROWNED_CONVERSION, VANILLA_ZOMBIE_DEFAULT_SPECIAL_TYPE,
    VANILLA_ZOMBIE_DROWNED_CONVERSION_DATA_ID, VANILLA_ZOMBIE_SPECIAL_TYPE_DATA_ID,
};
pub use crate::item_profiles::FurnaceFuelRule;
pub use crate::jigsaw_blocks::*;
pub use crate::maps::*;
pub use crate::minecart_command_blocks::*;
pub use crate::painting_variants::*;
pub use crate::play_apply::{
    advance_dragon_fireball_explode_level_event_particle_randoms,
    advance_growth_level_event_particle_randoms, advance_potion_break_level_event_particle_randoms,
    advance_wax_on_level_event_particle_randoms, EntityTrackingEmitterParticleKind,
    LevelEventGrowthRandomMode, NoPlayApplyEffects, PlayApplyEffects,
};
pub use crate::recipe_book::*;
pub use crate::recipes::*;
pub use crate::registries::{
    direct_banner_pattern_presentation, direct_trim_material_presentation,
    direct_trim_pattern_presentation, painting_variant_translation_key, ArmorTrimSprite,
    BannerPatternPresentation, BannerPatternProjectionError, ChatTypeDecoration,
    ChatTypeDecorationParameter, ChatTypeProjectionError, EnchantmentHolderSetDefinition,
    EnchantmentTooltipDefinition, GameTestInstanceDefinition, GameTestInstanceKind,
    GameTestRegistryProjectionError, InstrumentPresentation, InstrumentProjectionError,
    InstrumentSoundEvent, JukeboxSongPresentation, JukeboxSongProjectionError, JukeboxSoundEvent,
    PaintingVariantClientProjection, PaintingVariantPresentation, PaintingVariantProjectionError,
    PersistentHolderDecodeError, PersistentHolderSetDecodeError, RegistryElementData,
    RegistryElementDecodeError, RegistryElementSource, RegistryHolderSetIdentity,
    TestEnvironmentDefinition, TestEnvironmentHolder, TestEnvironmentKind, TestGameRuleTypedValue,
    TestGameRuleValue, TestRegistryReference, TestRotation, TestWeather, TimelineClockReference,
    TimelineDefinition, TimelineEasing, TimelineKeyframe, TimelineModifier,
    TimelineProjectionError, TimelineTimeMarker, TimelineTrackDefinition, TrimMaterialPresentation,
    TrimPatternPresentation, TrimRegistryProjectionError, VanillaEnchantmentIdentity,
    VanillaInstrumentIdentity, VanillaTrimMaterialIdentity, VanillaTrimPatternIdentity,
    WorldClockDefinition, BANNER_PATTERN_REGISTRY, CHAT_TYPE_REGISTRY, DIMENSION_TYPE_REGISTRY,
    INSTRUMENT_REGISTRY, JUKEBOX_SONG_REGISTRY, PAINTING_VARIANT_REGISTRY,
    TEST_ENVIRONMENT_REGISTRY, TEST_INSTANCE_REGISTRY, TIMELINE_REGISTRY, TRIM_MATERIAL_REGISTRY,
    TRIM_PATTERN_REGISTRY, VANILLA_ENCHANTMENT_TOOLTIP_ORDER, WORLD_CLOCK_REGISTRY,
};
pub use crate::structure_blocks::*;
pub use crate::terrain_state::*;
pub use crate::test_blocks::*;
pub use crate::test_instance_blocks::*;
pub use crate::toasts::*;
pub use crate::tutorial::*;
pub use crate::world_state::*;
