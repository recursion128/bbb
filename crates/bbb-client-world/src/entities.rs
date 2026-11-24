use std::collections::BTreeMap;

use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_CAMERA_DISTANCE_ID, VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
};
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID;
use bbb_protocol::packets::{
    AttributeSnapshot as ProtocolAttributeSnapshot, EntityDataValue as ProtocolEntityDataValue,
    EntityDataValueKind, EquipmentSlot as ProtocolEquipmentSlot,
    EquipmentSlotUpdate as ProtocolEquipmentSlotUpdate,
    FireworkExplosionSummary as ProtocolFireworkExplosionSummary,
    ItemStackSummary as ProtocolItemStackSummary, MinecartStep as ProtocolMinecartStep,
    RemoveEntities as ProtocolRemoveEntities, TakeItemEntity as ProtocolTakeItemEntity,
};
pub(crate) use bbb_render_types::EntityEquipmentAssetId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::terrain::{
    block_name_has_invisible_render_shape, block_name_is_bed, block_name_is_rail,
};
use crate::{
    ArmorTrimSprite, BlockPos, ChunkPos, PaintingVariantClientProjection, TerrainLight, WorldStore,
};

mod world_items;
mod world_particles;
mod world_player;
mod world_render;

mod model_source_defaults;
mod values;
pub use bbb_render_types::entity_render_state::*;
pub use model_source_defaults::*;
pub use values::*;

mod animations;
mod block_effects;
mod client_movement;
pub(crate) mod content_facts;
mod record;
pub(crate) use record::{EntityBoatStatus, EntityRecord};
mod dimensions;
mod display;
mod dragon;
pub(crate) mod entity_data;
pub(crate) mod events;
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

pub(crate) use animations::ATTACK_SWING_DURATION;
use animations::{cat_is_lying, guardian_attack_target_id};
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
pub use dragon::{DragonFlightHistoryState, EnderDragonAnimationState};
pub(crate) use events::VanillaEntityEventKind;
pub use leash::EntityLeashRenderState;
pub use name_tags::{
    EntityNameTagLivingVisibilityState, EntityNameTagRenderSourceState,
    EntityNameTagTextSegmentState, EntityNameTagTextState, EntityNameTagVisibilityPolicy,
};
pub(crate) use passengers::clamp_local_player_boat_yaw;
pub(crate) use record::{
    EntityAbstractArrowRuntime, EntityAttributes, EntityDamage, EntityEquipment, EntityIdentity,
    EntityInterpolationCollisionInput, EntityLeash, EntityMetadata, EntityMinecartCommandBlock,
    EntityMinecartLerp, EntityMobEffects, EntityMount, EntitySupportingBlock, EntityTransform,
    EntityTransientEvents, LivingDeathWakeTransition,
};
use renderer_catalog::vanilla_entity_renderer_block_light_floor;
pub use renderer_catalog::{
    vanilla_entity_renderer_culling_kind, vanilla_entity_renderer_extracts_parrot_pose,
    vanilla_entity_renderer_extracts_sheep_head_eat, vanilla_entity_renderer_extracts_wolf_sitting,
    vanilla_entity_renderer_extracts_zombie_underwater_converting,
    vanilla_entity_renderer_thrown_item_scales, VanillaBoatRendererKind,
    VanillaEntityRendererCullingKind, VanillaHorseRendererKind,
};
pub use renderer_catalog::{vanilla_entity_renderer_model_kind, VanillaEntityRendererModelKind};
pub use renderer_catalog::{
    vanilla_entity_renderer_presentation_owner, VanillaEntityRendererPresentationOwner,
};
pub use schema::vanilla_entity_metadata_dolphin_accessors_for_entity_type;
pub use schema::vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type;
pub(crate) use schema::vanilla_entity_metadata_interact_owner;
pub(crate) use schema::vanilla_entity_metadata_mob_interact_owner;
pub(crate) use schema::vanilla_entity_metadata_type_accepts_local_attack_hurt;
pub(crate) use schema::vanilla_entity_metadata_type_is_living;
pub(crate) use schema::vanilla_entity_metadata_type_is_menu_provider;
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
pub use schema::VANILLA_ENTITY_TYPE_METADATA_26_1;
pub use schema::VANILLA_IRON_GOLEM_FLAG_PLAYER_CREATED;
pub use schema::VANILLA_RABBIT_VARIANT_EVIL_ID;
pub use schema::{
    vanilla_entity_camera_post_effect_id,
    vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_boat_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_horse_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type,
    vanilla_entity_metadata_accessor_for_entity_type,
    vanilla_entity_metadata_accessor_for_entity_type_and_data_id,
    vanilla_entity_metadata_age_locked_accessor_for_entity_type,
    vanilla_entity_metadata_allay_can_duplicate_accessor_for_entity_type,
    vanilla_entity_metadata_allay_dancing_accessor_for_entity_type,
    vanilla_entity_metadata_armadillo_state_accessor_for_entity_type,
    vanilla_entity_metadata_armor_stand_accessors_for_entity_type,
    vanilla_entity_metadata_arrow_accessors_for_entity_type,
    vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type,
    vanilla_entity_metadata_baby_accessor_for_entity_type,
    vanilla_entity_metadata_bat_flags_accessor_for_entity_type,
    vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type,
    vanilla_entity_metadata_camel_dash_accessor_for_entity_type,
    vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type,
    vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type,
    vanilla_entity_metadata_companion_variant_accessors_for_entity_type,
    vanilla_entity_metadata_copper_golem_accessors_for_entity_type,
    vanilla_entity_metadata_creaking_active_accessor_for_entity_type,
    vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type,
    vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type,
    vanilla_entity_metadata_creeper_accessors_for_entity_type,
    vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type,
    vanilla_entity_metadata_enderman_render_accessors_for_entity_type,
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type,
    vanilla_entity_metadata_fox_accessors_for_entity_type,
    vanilla_entity_metadata_frog_variant_accessor_for_entity_type,
    vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type,
    vanilla_entity_metadata_ghast_charging_accessor_for_entity_type,
    vanilla_entity_metadata_goat_accessors_for_entity_type,
    vanilla_entity_metadata_happy_ghast_accessors_for_entity_type,
    vanilla_entity_metadata_horse_variant_accessor_for_entity_type,
    vanilla_entity_metadata_interaction_accessors_for_entity_type,
    vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type,
    vanilla_entity_metadata_living_entity_accessors_for_entity_type,
    vanilla_entity_metadata_llama_accessors_for_entity_type,
    vanilla_entity_metadata_mob_accessors_for_entity_type,
    vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type,
    vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type,
    vanilla_entity_metadata_panda_accessors_for_entity_type,
    vanilla_entity_metadata_parrot_variant_accessor_for_entity_type,
    vanilla_entity_metadata_phantom_size_accessor_for_entity_type,
    vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type,
    vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type,
    vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type,
    vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type,
    vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type,
    vanilla_entity_metadata_rabbit_type_accessor_for_entity_type,
    vanilla_entity_metadata_raider_celebrating_accessor_for_entity_type,
    vanilla_entity_metadata_salmon_variant_accessor_for_entity_type,
    vanilla_entity_metadata_sheep_wool_accessor_for_entity_type,
    vanilla_entity_metadata_shulker_accessors_for_entity_type,
    vanilla_entity_metadata_skeleton_freeze_conversion_accessor_for_entity_type,
    vanilla_entity_metadata_slime_size_accessor_for_entity_type,
    vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type,
    vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type,
    vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type,
    vanilla_entity_metadata_tamable_animal_accessors_for_entity_type,
    vanilla_entity_metadata_thrown_trident_accessors_for_entity_type,
    vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type,
    vanilla_entity_metadata_turtle_accessors_for_entity_type,
    vanilla_entity_metadata_type_is_guardian, vanilla_entity_metadata_type_is_leashable,
    vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer,
    vanilla_entity_metadata_vehicle_accessors_for_entity_type,
    vanilla_entity_metadata_vex_flags_accessor_for_entity_type,
    vanilla_entity_metadata_villager_data_accessor_for_entity_type,
    vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type,
    vanilla_entity_metadata_zombie_accessors_for_entity_type,
    vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type,
    vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type,
    vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type,
    VanillaAbstractArrowMetadataAccessors, VanillaArmorStandMetadataAccessors,
    VanillaEntityInterpolationPolicy, VanillaEntityMetadataAccessorCatalogEntry,
    VanillaEntityMetadataAccessors, VanillaEntityMetadataDefault, VanillaEntityMetadataFamily,
    VanillaEntityMetadataFamilySchema, VanillaEntityMetadataFieldSchema,
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
    VANILLA_ENTITY_DEFAULT_MAX_AIR_SUPPLY_TICKS, VANILLA_ENTITY_NO_GRAVITY_DATA_ID,
    VANILLA_ENTITY_POSE_DATA_ID, VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
    VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, VANILLA_ENTITY_SHARED_FLAG_GLOWING,
    VANILLA_ENTITY_SHARED_FLAG_INVISIBLE, VANILLA_ENTITY_SHARED_FLAG_ON_FIRE,
    VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
    VANILLA_ENTITY_SHARED_FLAG_SWIMMING, VANILLA_ENTITY_SILENT_DATA_ID,
    VANILLA_ENTITY_TICKS_FROZEN_DATA_ID, VANILLA_FISHING_HOOK_BITING_DATA_ID,
    VANILLA_FISHING_HOOK_DEFAULT_BITING, VANILLA_FISHING_HOOK_DEFAULT_HOOKED_ENTITY,
    VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID, VANILLA_HANGING_ENTITY_DEFAULT_DIRECTION_ID,
    VANILLA_HANGING_ENTITY_DIRECTION_DATA_ID, VANILLA_HUMANOID_ARM_LEFT_ID,
    VANILLA_HUMANOID_ARM_RIGHT_ID, VANILLA_ITEM_FRAME_DEFAULT_ROTATION,
    VANILLA_ITEM_FRAME_ITEM_DATA_ID, VANILLA_ITEM_FRAME_ROTATION_DATA_ID,
    VANILLA_LIVING_ENTITY_ARROW_COUNT_DATA_ID, VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID,
    VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID, VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
    VANILLA_LIVING_ENTITY_FLAG_IS_USING, VANILLA_LIVING_ENTITY_FLAG_OFF_HAND,
    VANILLA_LIVING_ENTITY_FLAG_SPIN_ATTACK, VANILLA_LIVING_ENTITY_HEALTH_DATA_ID,
    VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID, VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID,
    VANILLA_MOB_FLAGS_DATA_ID, VANILLA_MOB_FLAG_AGGRESSIVE, VANILLA_MOB_FLAG_LEFT_HANDED,
    VANILLA_MOB_FLAG_NO_AI, VANILLA_OMINOUS_ITEM_SPAWNER_ITEM_DATA_ID,
    VANILLA_PAINTING_VARIANT_DATA_ID, VANILLA_PLAYER_ABSORPTION_DATA_ID,
    VANILLA_PLAYER_DEFAULT_ABSORPTION, VANILLA_PLAYER_DEFAULT_SCORE, VANILLA_PLAYER_SCORE_DATA_ID,
    VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID, VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID,
};
#[cfg(test)]
pub(crate) use schema::{
    vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type,
    vanilla_entity_metadata_area_effect_cloud_radius_accessor_for_entity_type,
    vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type,
    vanilla_entity_metadata_creaking_home_pos_accessor_for_entity_type,
    vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type,
    vanilla_entity_metadata_player_absorption_accessor_for_entity_type,
    vanilla_entity_metadata_player_score_accessor_for_entity_type,
    vanilla_entity_metadata_sniffer_state_accessor_for_entity_type,
    vanilla_entity_metadata_warden_anger_level_accessor_for_entity_type,
    vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type,
};
pub use schema::{
    vanilla_entity_metadata_bee_accessors_for_entity_type, VanillaBeeMetadataAccessors,
};
pub use schema::{
    vanilla_entity_metadata_cat_accessors_for_entity_type, VanillaCatMetadataAccessors,
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
pub(crate) use schema::{vanilla_entity_metadata_food_predicate, VanillaEntityFoodPredicate};
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
    vanilla_entity_metadata_type_sound_volume_policy, VanillaEntityCollidabilityKind,
    VanillaEntityDeathSoundPolicy, VanillaEntityHurtSoundPolicy, VanillaEntityPushabilityKind,
    VanillaEntitySoundCondition, VanillaEntitySoundVariantFamily, VanillaEntitySoundVolumePolicy,
};
pub use schema::{
    vanilla_entity_metadata_wolf_accessors_for_entity_type, VanillaWolfMetadataAccessors,
};
pub use schema::{
    VanillaEntityEventHandlerCatalogEntry, VanillaEntityOwner, VANILLA_ENTITY_EVENT_HANDLERS_26_1,
};
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
    VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID,
    VANILLA_ABSTRACT_MINECART_DISPLAY_OFFSET_DATA_ID,
    VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
    VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID, VANILLA_FURNACE_MINECART_DEFAULT_FUEL,
    VANILLA_FURNACE_MINECART_FUEL_DATA_ID,
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
pub(crate) use shadows::vanilla_lightmap_brightness;
pub use shadows::{EntityShadowPieceState, EntityShadowRenderSourceState};
pub use sound_variants::{
    CatSoundVariantIdentity, ChickenSoundVariantIdentity, CowSoundVariantIdentity,
    PigSoundVariantIdentity, WolfSoundVariantIdentity,
};
use status::{EntityDamageEventState, MobEffectState};
pub(crate) use store::LocalPlayerBlockEffectStep;
pub use store::{EntityClientTickWork, EntityPublicationSnapshotObservation};
pub(crate) use store::{EntityStore, LocalPlayerOwnedData};

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

pub fn vanilla_entity_type_illager_arm_pose_uses_celebrating(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_illager_arm_pose_uses_celebrating(entity_type_id)
}

pub fn vanilla_entity_type_is_wind_charge(entity_type_id: i32) -> bool {
    schema::vanilla_entity_metadata_type_is_wind_charge(entity_type_id)
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

/// One entity projected for rendering at a partial tick: the renderer-facing
/// [`EntityRenderState`] plus the inputs the native scene still resolves against
/// item, texture, and registry state it owns.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityRenderSource {
    pub(crate) light_probe_eye_height: Option<f32>,
    pub entity_id: i32,
    pub uuid: uuid::Uuid,
    pub entity_type_id: i32,
    /// Raw interpolated entity position used by
    /// `EntityRenderDispatcher.distanceToSqr`. Unlike `position`, this excludes
    /// passenger/new-minecart/Shulker render offsets owned by the dispatch pose.
    pub distance_position: EntityVec3,
    pub position: EntityVec3,
    pub y_rot: f32,
    /// Canonical entity head pitch (`Entity.getXRot`), applied as the model head
    /// `xRot` look.
    pub x_rot: f32,
    /// Canonical entity head yaw (`Entity.yHeadRot`). The renderer projection
    /// derives the net head-look yaw `Mth.wrapDegrees(y_head_rot - y_rot)`.
    pub y_head_rot: f32,
    pub age_ticks: u32,
    /// Vanilla `LivingEntityRenderState.isFullyFrozen` (`Entity.isFullyFrozen`,
    /// `ticksFrozen >= 140`): a living entity frozen solid in powder snow, whose
    /// body the renderer shakes (`LivingEntityRenderer.isShaking`). `false` for
    /// non-living entities and any entity that is not frozen solid.
    pub is_fully_frozen: bool,
    /// Vanilla `LivingEntityRenderState.hasPose(Pose.SLEEPING)`: the entity is
    /// lying down in a bed, which `LivingEntityRenderer.setupRotations` lays on its
    /// side. `false` for non-living entities and any entity that is not sleeping.
    pub is_sleeping: bool,
    /// Vanilla `setupRotations` sleeping `angle` from the resolved bed orientation
    /// (`sleepDirectionToRotation(BedBlock.getBedOrientation(...))`, degrees).
    /// `None` when the entity is sleeping but its sleeping position is not a bed, in
    /// which case the renderer falls back to the body yaw.
    pub sleeping_bed_yaw: Option<f32>,
    /// Vanilla `submit` bed head-offset translate `[-stepX * headOffset, -stepZ *
    /// headOffset]` in world units (`headOffset = eyeHeight(STANDING) - 0.1`). `[0,
    /// 0]` when the entity is not sleeping in a bed.
    pub sleeping_bed_offset: [f32; 2],
    /// Vanilla `Goat.lowerHeadTick` (the `0..=20` ram counter, advanced from entity events 58/59): the
    /// native layer scales it by the adult/baby max head pitch to derive `getRammingXHeadRot`. `0` for a
    /// goat not ramming and every non-goat.
    pub goat_lower_head_tick: i32,
    /// Vanilla `Creaking.hasGlowingEyes()`: the client-local tearing-down eye-flicker bit. It is
    /// consumed only while synced `isTearingDown()` is true; otherwise `CreakingRenderer` uses the
    /// synced `isActive()` flag. `false` for every other entity and before teardown begins.
    pub creaking_eyes_glowing: bool,
    /// Vanilla `LivingEntity.swinging`: whether a melee swing is in progress. Gates the player's
    /// `CROSSBOW_HOLD` arm pose (`AvatarRenderer.getArmPose`: `!swinging && crossbow && charged`), which
    /// yields to the swing while attacking. `false` for an entity that is not mid-swing.
    pub is_swinging: bool,
    /// Vanilla `LivingEntityRenderState.bodyRot` for squid (`Squid.yBodyRot`
    /// lerped, degrees): the movement-derived body yaw `SquidRenderer.setupRotations`
    /// receives before applying its squid-specific pitch/roll. `0.0` for every
    /// non-squid entity.
    pub squid_y_body_rot: f32,
    /// Vanilla `PaintingRenderState.variant`: the synchronized or inline painting fields retained
    /// from entity metadata. `None` until a painting variant is available and for non-paintings.
    pub painting_variant_projection: Option<PaintingVariantClientProjection>,
    /// Vanilla `PaintingRenderState.direction`: the horizontal hanging direction resolved from
    /// synchronized metadata or the add-entity data fallback.
    pub painting_facing: Option<ItemFrameFacing>,
    /// Armor-trim sprites per equipment layer, resolved to uploaded textures by native.
    pub armor_trims: EntityArmorTrimSprites,
    /// Shares the entity's synchronized values; projection never copies them.
    pub data_values: crate::PublicationShared<Vec<ProtocolEntityDataValue>>,
    pub render: EntityRenderState,
}

/// Vanilla `EquipmentLayerRenderer` trim sprites, one per equipment layer the
/// render state draws.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntityArmorTrimSprites {
    pub wolf_body_armor_trim: Option<ArmorTrimSprite>,
    pub head_armor_trim: Option<ArmorTrimSprite>,
    pub chest_armor_trim: Option<ArmorTrimSprite>,
    pub legs_armor_trim: Option<ArmorTrimSprite>,
    pub feet_armor_trim: Option<ArmorTrimSprite>,
    pub chest_wings_trim: Option<ArmorTrimSprite>,
    pub pig_saddle_trim: Option<ArmorTrimSprite>,
    pub equine_saddle_trim: Option<ArmorTrimSprite>,
    pub equine_body_armor_trim: Option<ArmorTrimSprite>,
    pub strider_saddle_trim: Option<ArmorTrimSprite>,
    pub camel_saddle_trim: Option<ArmorTrimSprite>,
    pub nautilus_saddle_trim: Option<ArmorTrimSprite>,
    pub happy_ghast_harness_trim: Option<ArmorTrimSprite>,
    pub llama_body_decor_trim: Option<ArmorTrimSprite>,
    pub nautilus_body_armor_trim: Option<ArmorTrimSprite>,
}

/// Vanilla `EntityRenderer` light-probe full-bright fallback
/// (`LightCoordsUtil.FULL_BRIGHT` = block 15, sky 15), used when an entity's
/// chunk light is unavailable so it renders bright rather than dark, matching
/// the `EntityRenderState.lightCoords` default.
pub(crate) const ENTITY_LIGHT_PROBE_FULL_BRIGHT: TerrainLight = TerrainLight { sky: 15, block: 15 };

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
        let removed = if let Some(local_id) = self.local_player_id {
            if packet.entity_ids.contains(&local_id) {
                let filtered: Vec<i32> = packet
                    .entity_ids
                    .into_iter()
                    .filter(|&id| id != local_id)
                    .collect();
                self.remove_entities_by_ids(&filtered)
            } else {
                self.remove_entities_by_ids(&packet.entity_ids)
            }
        } else {
            self.remove_entities_by_ids(&packet.entity_ids)
        };
        self.counters.entity_removes_ignored += received.saturating_sub(removed);
        removed
    }

    pub(crate) fn unlink_entity_relationships_before_replace(&mut self, entity_id: i32) {
        if self.local_player_vehicle_chain_contains(entity_id) {
            self.local_player_vehicle_id = None;
        }
        if let Some(old_vehicle_id) = self
            .entities
            .mount(entity_id)
            .and_then(|mount| mount.vehicle_id)
        {
            self.entities.with_mount_mut(old_vehicle_id, |vehicle| {
                vehicle.passengers.retain(|&id| id != entity_id);
            });
        }
        let old_passengers = self
            .entities
            .mount(entity_id)
            .map(|mount| mount.passengers.clone())
            .unwrap_or_default();
        for passenger_id in old_passengers {
            self.entities.with_mount_mut(passenger_id, |passenger| {
                if passenger.vehicle_id == Some(entity_id) {
                    passenger.vehicle_id = None;
                }
            });
        }
        self.entities.for_each_leash_mut(|_, leash| {
            if leash.holder_id == Some(entity_id) {
                leash.holder_id = None;
            }
        });
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
        if removed > 0 {
            self.advance_scoreboard_display_revision();
        }
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

    /// Every entity this client is tracking, in protocol-id order.
    ///
    /// Read-only, and ordered by id rather than by arrival: the store keeps
    /// entities in the order the server introduced them, which is a property
    /// of when each server decided to pair a tracker rather than of what the
    /// world holds. A caller comparing two clients needs an order neither
    /// server chose.
    /// The entities the server tracks for this session: everything but the
    /// session's own player.
    pub fn probe_entities(&self) -> Vec<EntityState> {
        let mut states = self.entities.states();
        states.retain(|state| Some(state.id) != self.local_player_id);
        states.sort_by_key(|state| state.id);
        states
    }

    /// Vanilla `Camera.setup` detached-camera distance before block clipping.
    ///
    /// A living camera entity contributes `getScale() * CAMERA_DISTANCE`; a
    /// living vehicle contributes the same product and the larger value wins.
    /// Non-living entities retain Camera's scale-1, distance-4 defaults.
    pub fn camera_entity_third_person_distance(&self) -> Option<f32> {
        let camera = self.local_player().camera;
        let camera_id = if camera.follows_player {
            self.local_player_id()?
        } else {
            camera.entity_id?
        };
        let camera_distance = self.living_entity_camera_distance(camera_id).unwrap_or(4.0);
        let mount_distance = self
            .entities
            .mount(camera_id)
            .and_then(|mount| mount.vehicle_id)
            .and_then(|vehicle_id| self.living_entity_camera_distance(vehicle_id))
            .unwrap_or(camera_distance);
        Some(camera_distance.max(mount_distance))
    }

    fn living_entity_camera_distance(&self, entity_id: i32) -> Option<f32> {
        let entity_type_id = self.entities.entity_type_id(entity_id)?;
        if !vanilla_living_entity_type(entity_type_id) {
            return None;
        }
        let scale = self.entities.render_scale(entity_id)?;
        let distance = self
            .entities
            .attribute_value(entity_id, VANILLA_ATTRIBUTE_CAMERA_DISTANCE_ID)?;
        let distance = if distance.is_nan() {
            0.0
        } else {
            distance.clamp(0.0, 32.0) as f32
        };
        Some(scale * distance)
    }

    pub fn entity_type_id(&self, id: i32) -> Option<i32> {
        self.entities.entity_type_id(id)
    }

    pub fn entity_uuid(&self, id: i32) -> Option<uuid::Uuid> {
        self.entities.identity(id).map(|identity| identity.uuid)
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

    pub fn take_item_entity_pickup_particle_state(
        &self,
        item_entity_id: i32,
        target_entity_id: i32,
    ) -> Option<TakeItemEntityPickupParticleState> {
        let item = self.probe_entity_transform(item_entity_id)?;
        let target = self
            .probe_entity_camera_pose(target_entity_id)
            .map(|pose| (pose.id, pose.position, pose.eye_height))?;
        let item_stack = (item.entity_type_id == VANILLA_ENTITY_TYPE_ITEM_ID)
            .then(|| self.entities.item_stack_for_entity(item_entity_id))
            .flatten()
            .cloned();
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
    ) -> Option<LivingEntityItemParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if !vanilla_living_entity_type(transform.entity_type_id) {
            return None;
        }
        let pose = self.probe_entity_camera_pose(entity_id)?;
        let item_stack = self.entities.equipment_item(entity_id, slot)?;
        if item_stack.count <= 0 {
            return None;
        }
        Some(LivingEntityItemParticleState {
            count: 5,
            entity_id,
            position: transform.position,
            eye_height: pose.eye_height,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            item_stack: item_stack.clone(),
        })
    }

    pub fn local_consume_particles(
        &self,
        hand: bbb_protocol::packets::InteractionHand,
        count: usize,
    ) -> Option<LivingEntityItemParticleState> {
        let entity_id = self.local_player_id()?;
        let stack = self.local_item_in_hand(hand)?;
        let consumable = self.items.component_source(
            stack.item_id,
            &stack.component_patch,
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        );
        if consumable.consumable_has_consume_particles != Some(true) {
            return None;
        }
        let pose = self.local_player_pose()?;
        Some(LivingEntityItemParticleState {
            count,
            entity_id,
            position: pose.position,
            eye_height: self.local_player_eye_height(pose) as f32,
            y_rot: pose.y_rot,
            x_rot: pose.x_rot,
            item_stack: stack.clone(),
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

    pub fn entity_model_sources_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<EntityRenderSource> {
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

    fn cat_is_lying_on_top_of_sleeping_player(
        &self,
        source: &EntityRenderSource,
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

    fn entity_outline_color(&self, source: &EntityRenderSource) -> u32 {
        // Vanilla `Minecraft.shouldEntityAppearGlowing`: the synced glowing
        // flag, or every player glows while the spectating local player holds
        // `key.spectatorOutlines`.
        let spectator_outline = self.spectator_outlines_key_down
            && self.local_player_is_spectator()
            && source.entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
        if !source.render.appears_glowing && !spectator_outline {
            return 0;
        }

        let scoreboard_name = self.entity_scoreboard_name(source);
        let rgb = self
            .scoreboard
            .team_color_rgb_for_scoreboard_name(&scoreboard_name)
            .unwrap_or(VANILLA_DEFAULT_ENTITY_OUTLINE_RGB);
        VANILLA_OPAQUE_ALPHA | rgb
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

    /// The same projection for any camera player: vanilla resolves
    /// `getCameraPlayer().getVehicle()` rather than the local player's mount.
    pub fn player_vehicle_health(&self, id: i32) -> Option<LocalPlayerVehicleHealth> {
        self.vehicle_health(self.entities.mount(id)?.vehicle_id?)
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
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

/// Vanilla `BedBlock.getBedOrientation` + `LivingEntityRenderer.sleepDirectionToRotation` +
/// the `submit` bed head offset: given the bed block at the entity's sleeping
/// position and the entity's standing eye height, returns the sleeping yaw (degrees) and
/// the world-space head-offset translate `[-stepX * headOffset, -stepZ * headOffset]`
/// (`headOffset = eyeHeight - 0.1`). `None` when the block is not a bed.
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

/// Floors an already-resolved world light-probe position per axis.
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
