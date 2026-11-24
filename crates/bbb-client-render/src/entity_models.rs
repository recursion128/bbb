use crate::render_core::decorated_pot_patterns;

mod banner_patterns;
mod block_attachment;
mod cat_variants;
mod catalog;
mod data_driven;
mod dispatch;
mod entity_name_tag;
mod equipment;
mod features;
mod geometry;
mod gpu;
mod gui_preview;
mod held_item;
mod instances;
mod keyframe;
mod model;
#[allow(dead_code)]
mod model_data_26_1;
mod model_layer;
mod model_layers;
mod special_item;
mod temperature_animal_variants;
mod textured;
mod transforms;
mod wolf_variants;
mod zombie_nautilus_variants;

#[cfg(test)]
use crate::texture_sampling::rgba_offset;
pub(crate) use banner_patterns::banner_pattern_texture_ref;
pub use banner_patterns::BannerPatternKind;
pub use bbb_render_types::{VillagerModelProfession, VillagerModelType};
pub use block_attachment::{
    copper_golem_antenna_block_transform, enderman_carried_block_transform,
    falling_block_transform, iron_golem_flower_block_transform, minecart_display_block_transform,
    minecart_tnt_display_block_transform, mooshroom_mushroom_block_transforms,
    primed_tnt_block_transform, snow_golem_head_block_transform,
};
pub use cat_variants::CatModelVariant;
pub use catalog::*;
pub(crate) use data_driven::BakedDataDrivenRig;
pub use decorated_pot_patterns::DecoratedPotPattern;
pub use entity_name_tag::{
    bake_entity_name_tag_surfaces, entity_name_tag_root_position, EntityNameTagLayerSubmission,
    EntityNameTagLineKind, EntityNameTagRenderType, EntityNameTagSubmission, EntityNameTagSurface,
    ENTITY_NAME_TAG_BACKGROUND_OPACITY, ENTITY_NAME_TAG_FONT_TEXTURE, ENTITY_NAME_TAG_SCALE,
};
pub(crate) use entity_name_tag::{next_name_tag_layout_frame, EntityNameTagRenderState};
pub use equipment::{
    EntityEquipmentAsset, EntityEquipmentAssetCatalog, EntityEquipmentAssetLayer,
    EntityEquipmentAssetLayerTexture, EntityEquipmentDyeable, EntityEquipmentLayerType,
};
pub use features::{
    bake_entity_flame_surface, bake_entity_shadow_surface, EntityFlameSubmission,
    EntityFlameSurface, EntityShadowPiece, EntityShadowSubmission, EntityShadowSurface,
    ENTITY_FLAME_FIRE_0_SPRITE, ENTITY_FLAME_FIRE_1_SPRITE, ENTITY_SHADOW_TEXTURE_PATH,
};
pub(crate) use features::{EntityFeatureMesh, EntityShadowTextureGpu};
use geometry::*;
mod retained;
#[cfg(test)]
use glam::{Mat4, Vec3};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use gpu::{build_dynamic_player_skin_atlas, build_dynamic_player_texture_atlas};
#[cfg(test)]
pub(crate) use gpu::{build_entity_model_texture_atlas, create_entity_model_textured_pipeline};
pub(crate) use gpu::{
    create_entity_core_pipelines, create_entity_model_armor_entity_glint_pipeline,
    create_entity_model_beacon_beam_opaque_pipeline,
    create_entity_model_beacon_beam_translucent_pipeline,
    create_entity_model_dragon_rays_depth_pipeline, create_entity_model_dragon_rays_pipeline,
    create_entity_model_end_crystal_beam_pipeline, create_entity_model_end_gateway_pipeline,
    create_entity_model_end_portal_pipeline, create_entity_model_entity_glint_pipeline,
    create_entity_model_guardian_beam_pipeline, create_entity_model_leash_pipeline,
    create_entity_model_outline_cull_pipeline, create_entity_model_outline_pipeline,
    create_entity_model_scroll_additive_pipeline, create_entity_model_scroll_pipeline,
    create_entity_model_water_mask_pipeline, create_win_screen_end_portal_pipeline,
    upload_elder_guardian_particle_textured_mesh,
    upload_experience_orb_pickup_particle_textured_mesh,
    upload_projectile_pickup_particle_textured_mesh, EntityCorePipelines,
};
#[cfg(test)]
use gpu::{
    entity_model_vertex_layout, sanitize_entity_model_instances, ENTITY_MODEL_BANNER_PATTERN_BLEND,
    ENTITY_MODEL_BANNER_PATTERN_CULL_MODE, ENTITY_MODEL_BANNER_PATTERN_DEPTH_COMPARE,
    ENTITY_MODEL_BANNER_PATTERN_DEPTH_WRITE_ENABLED, ENTITY_MODEL_BANNER_PATTERN_SHADER,
    ENTITY_MODEL_EYES_SHADER, ENTITY_MODEL_SCROLL_EMISSIVE_SHADER, ENTITY_MODEL_SCROLL_SHADER,
    ENTITY_MODEL_TEXTURED_SHADER, ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES,
    ENTITY_MODEL_VERTEX_ATTRIBUTES,
};
pub(super) use gpu::{
    EntityDynamicPlayerSkinAtlasGpu, EntityDynamicPlayerTextureAtlasGpu, EntityModelMeshGpu,
    EntityModelTextureAtlasGpu,
};
#[cfg(test)]
pub(crate) use gui_preview::bake_hud_entity_preview_pip_geometry;
pub(crate) use gui_preview::HudEntityPreviewPipTarget;
pub use held_item::{
    allay_hand_attach_transform, copper_golem_hand_attach_transform, custom_head_item_transform,
    custom_head_item_transforms, dolphin_carried_item_transform, fox_held_item_transform,
    humanoid_hand_attach_transform, humanoid_hand_attach_transforms, panda_held_item_transform,
    player_held_to_eye_transform, villager_crossed_arms_item_transform, witch_held_item_transform,
};
pub use instances::*;
#[cfg(test)]
use model_layers::*;
pub use model_layers::{entity_model_texture_refs, SheepHeadEatPose};
pub(crate) use retained::{EntityGeometryResidency, EntityPosePipelines};
pub(crate) use special_item::{bake_special_item_geometry, ResolvedSpecialItemTexture};
pub use temperature_animal_variants::{
    ChickenModelVariant, CowModelVariant, FrogModelVariant, PigModelVariant,
};
pub(crate) use textured::fishing_hook_line_segments;
pub(crate) use textured::EntityExtractionCache;
#[cfg(test)]
use textured::{
    armadillo_textured_layer_passes, arrow_textured_layer_passes, axolotl_textured_layer_passes,
    banner_textured_layer_passes, bed_textured_layer_passes, bell_textured_layer_passes,
    blaze_textured_layer_passes, boat_textured_layer_passes, book_textured_layer_passes,
    breeze_textured_layer_passes, camel_textured_layer_passes, chest_textured_layer_passes,
    chicken_textured_layer_passes, conduit_textured_layer_passes,
    copper_golem_textured_layer_passes, cow_textured_layer_passes, creaking_textured_layer_passes,
    creeper_textured_layer_passes, custom_head_skull_layer_pass,
    decorated_pot_textured_layer_passes, donkey_textured_layer_passes,
    drowned_textured_layer_passes, end_crystal_textured_layer_passes,
    ender_dragon_textured_layer_passes, enderman_textured_layer_passes,
    endermite_textured_layer_passes, equipment_layer_pass, evoker_fangs_textured_layer_passes,
    feline_textured_layer_passes, fox_textured_layer_passes, frog_textured_layer_passes,
    ghast_textured_layer_passes, goat_textured_layer_passes, guardian_textured_layer_passes,
    happy_ghast_textured_layer_passes, hoglin_textured_layer_passes, horse_textured_layer_passes,
    husk_textured_layer_passes, illager_textured_layer_passes, iron_golem_textured_layer_passes,
    leash_knot_textured_layer_passes, llama_spit_textured_layer_passes,
    llama_textured_layer_passes, magma_cube_textured_layer_passes, minecart_textured_layer_passes,
    mooshroom_textured_layer_passes, nautilus_textured_layer_passes, panda_textured_layer_passes,
    parrot_textured_layer_passes, phantom_textured_layer_passes, pig_textured_layer_passes,
    piglin_textured_layer_passes, player_cape_layer_pass,
    player_extra_ears_layer_pass_with_texture, player_parrot_on_shoulder_layer_pass,
    player_spin_attack_effect_layer_pass, player_textured_layer_passes,
    polar_bear_textured_layer_passes, rabbit_textured_layer_passes, ravager_textured_layer_passes,
    salmon_textured_layer_passes, sheep_textured_layer_passes, shulker_box_textured_layer_passes,
    shulker_bullet_textured_layer_passes, shulker_textured_layer_passes,
    silverfish_textured_layer_passes, skeleton_textured_layer_passes, slime_textured_layer_passes,
    sniffer_textured_layer_passes, snow_golem_textured_layer_passes, spider_textured_layer_passes,
    squid_textured_layer_passes, tadpole_textured_layer_passes, trident_textured_layer_passes,
    tropical_fish_textured_layer_passes, undead_horse_textured_layer_passes,
    villager_data_textured_layer_passes, villager_textured_layer_passes,
    wandering_trader_textured_layer_passes, warden_pulsating_spots_alpha,
    warden_textured_layer_passes, wind_charge_textured_layer_passes, witch_textured_layer_passes,
    wither_skull_textured_layer_passes, wither_textured_layer_passes, wolf_textured_layer_passes,
    zombie_nautilus_textured_layer_passes, zombie_textured_layer_passes,
    zombie_villager_data_textured_layer_passes, zombie_villager_textured_layer_passes,
    EntityModelLayerKind, EntityModelLayerPass, EntityModelLayerRenderBucket,
    EntityModelLayerVisibility,
};
#[cfg(test)]
use textured::{
    dynamic_player_texture_test_meshes, elder_guardian_particle_textured_meshes,
    entity_model_textured_meshes, entity_model_textured_meshes_with_dynamic_skins,
    entity_model_textured_meshes_with_dynamic_textures,
    entity_model_textured_meshes_with_dynamic_textures_for_camera,
    experience_orb_pickup_particle_textured_mesh, projectile_pickup_particle_textured_meshes,
    EntityModelTexturedMeshes,
};
use textured::{entity_model_water_mask_mesh, first_person_player_arm_textured_meshes};
pub use textured::{
    ElderGuardianParticleRenderInstance, ExperienceOrbPickupParticleRenderInstance,
    ProjectilePickupParticleRenderInstance,
};
pub(crate) use textured::{
    EntityModelDrawRange, EntityModelLayerRenderType, EntityModelTexturedDrawAtlas,
    EntityModelTexturedDrawRange, EntityModelTranslucentDrawRange,
};
pub(crate) use transforms::entity_dispatch_render_offset;
#[cfg(test)]
use transforms::{
    boat_bubble_transform, boat_damage_roll_degrees, death_fall_factor,
    ender_dragon_model_root_transform, entity_flip_degrees, feline_model_root_transform,
    iron_golem_model_root_transform, minecart_damage_roll_degrees, minecart_model_root_transform,
    minecart_render_jitter_offset, shulker_model_root_transform,
};
use transforms::{entity_model_root_transform, player_model_root_transform};
pub use wolf_variants::WolfModelVariant;
pub use zombie_nautilus_variants::ZombieNautilusModelVariant;

#[cfg(test)]
mod tests;

#[cfg(test)]
use textured::entity_model_textured_meshes_with_equipment_for_camera;
