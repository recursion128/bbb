mod block_attachment;
mod catalog;
mod colored;
mod dispatch;
mod geometry;
mod gpu;
mod held_item;
mod instances;
mod keyframe;
mod model;
mod model_layers;
mod textured;

pub use block_attachment::{
    copper_golem_antenna_block_transform, enderman_carried_block_transform,
    iron_golem_flower_block_transform, mooshroom_mushroom_block_transforms,
    snow_golem_head_block_transform,
};
pub use catalog::*;
#[cfg(test)]
use colored::{
    boat_bubble_transform, boat_damage_roll_degrees, creeper_model_root_transform,
    death_fall_factor, end_crystal_model_root_transform, ender_dragon_model_root_transform,
    entity_flip_degrees, entity_model_mesh, fox_model_root_transform, humanoid_arm_swing_parts,
    humanoid_limb_swing_parts, iron_golem_model_root_transform, magma_cube_model_root_transform,
    panda_model_root_transform, quadruped_leg_x_rotations, quadruped_limb_swing_parts,
    shulker_bullet_model_root_transform, shulker_model_root_transform, slime_model_root_transform,
    squid_model_root_transform, tropical_fish_model_root_transform,
    wind_charge_model_root_transform, wither_model_root_transform, HUMANOID_ARM_PART_INDICES,
    HUMANOID_LEG_PART_INDICES,
};
use colored::{
    entity_model_colored_runtime_mesh, entity_model_root_transform,
    mesh_transformer_scaled_model_root_transform, player_model_root_transform,
    wither_skeleton_model_root_transform, HUSK_SCALE,
};
use geometry::*;
#[cfg(test)]
use glam::{Mat4, Vec3};
#[cfg(test)]
use gpu::{
    build_dynamic_player_skin_atlas, build_dynamic_player_texture_atlas,
    build_entity_model_texture_atlas, entity_model_vertex_layout, rgba_offset,
    sanitize_entity_model_instances, ENTITY_MODEL_EYES_SHADER, ENTITY_MODEL_SCROLL_EMISSIVE_SHADER,
    ENTITY_MODEL_SCROLL_SHADER, ENTITY_MODEL_SHADER, ENTITY_MODEL_TEXTURED_SHADER,
    ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES, ENTITY_MODEL_VERTEX_ATTRIBUTES,
};
pub(crate) use gpu::{
    create_entity_model_eyes_pipeline, create_entity_model_pipeline,
    create_entity_model_scroll_additive_pipeline, create_entity_model_scroll_pipeline,
    create_entity_model_textured_pipeline, create_entity_model_translucent_pipeline,
};
pub(super) use gpu::{
    EntityDynamicPlayerSkinAtlasGpu, EntityDynamicPlayerTextureAtlasGpu, EntityModelMeshGpu,
    EntityModelScrollMeshGpu, EntityModelTextureAtlasGpu, EntityModelTexturedMeshGpu,
};
pub use held_item::{
    copper_golem_hand_attach_transform, custom_head_item_transform, dolphin_carried_item_transform,
    fox_held_item_transform, humanoid_hand_attach_transform, panda_held_item_transform,
    villager_crossed_arms_item_transform, witch_held_item_transform,
};
pub use instances::*;
use keyframe::*;
#[cfg(test)]
use model_layers::*;
pub use model_layers::{
    allay_entity_texture_refs, armadillo_entity_texture_refs, armor_stand_entity_texture_refs,
    arrow_entity_texture_refs, axolotl_entity_texture_refs, bat_entity_texture_refs,
    bee_entity_texture_refs, blaze_entity_texture_refs, boat_entity_texture_refs,
    breeze_entity_texture_refs, camel_entity_texture_refs, chicken_entity_texture_refs,
    cod_entity_texture_refs, copper_golem_entity_texture_refs, cow_entity_texture_refs,
    creaking_entity_texture_refs, creeper_entity_texture_refs, dolphin_entity_texture_refs,
    donkey_entity_texture_refs, drowned_entity_texture_refs, end_crystal_entity_texture_refs,
    ender_dragon_entity_texture_refs, enderman_entity_texture_refs, endermite_entity_texture_refs,
    entity_model_texture_refs, evoker_fangs_entity_texture_refs, feline_entity_texture_refs,
    fox_entity_texture_refs, frog_entity_texture_refs, ghast_entity_texture_refs,
    goat_entity_texture_refs, guardian_entity_texture_refs, happy_ghast_entity_texture_refs,
    hoglin_entity_texture_refs, horse_entity_texture_refs, husk_entity_texture_refs,
    illager_entity_texture_refs, leash_knot_entity_texture_refs, llama_entity_texture_refs,
    llama_spit_entity_texture_refs, minecart_entity_texture_refs, mooshroom_entity_texture_refs,
    nautilus_entity_texture_refs, panda_entity_texture_refs, parrot_entity_texture_refs,
    phantom_entity_texture_refs, pig_entity_texture_refs, piglin_entity_texture_refs,
    player_entity_texture_refs, polar_bear_entity_texture_refs, pufferfish_entity_texture_refs,
    rabbit_entity_texture_refs, ravager_entity_texture_refs, salmon_entity_texture_refs,
    sheep_entity_texture_refs, shulker_bullet_entity_texture_refs, shulker_entity_texture_refs,
    silverfish_entity_texture_refs, skeleton_entity_texture_refs, slime_entity_texture_refs,
    sniffer_entity_texture_refs, spider_entity_texture_refs, squid_entity_texture_refs,
    strider_entity_texture_refs, tadpole_entity_texture_refs, trident_entity_texture_refs,
    tropical_fish_entity_texture_refs, turtle_entity_texture_refs,
    undead_horse_entity_texture_refs, vex_entity_texture_refs, villager_entity_texture_refs,
    warden_entity_texture_refs, wind_charge_entity_texture_refs, witch_entity_texture_refs,
    wither_entity_texture_refs, wither_skull_entity_texture_refs, wolf_entity_texture_refs,
    zombie_entity_texture_refs, zombie_villager_entity_texture_refs, SheepHeadEatPose,
};
use textured::entity_model_textured_meshes_with_dynamic_textures;
#[cfg(test)]
use textured::{
    armadillo_textured_layer_passes, arrow_textured_layer_passes, axolotl_textured_layer_passes,
    blaze_textured_layer_passes, boat_textured_layer_passes, breeze_textured_layer_passes,
    camel_textured_layer_passes, chicken_textured_layer_passes, copper_golem_textured_layer_passes,
    cow_textured_layer_passes, creaking_textured_layer_passes, creeper_textured_layer_passes,
    custom_head_skull_layer_pass, donkey_textured_layer_passes, drowned_textured_layer_passes,
    end_crystal_textured_layer_passes, ender_dragon_textured_layer_passes,
    enderman_textured_layer_passes, endermite_textured_layer_passes, equipment_layer_pass,
    evoker_fangs_textured_layer_passes, feline_textured_layer_passes, fox_textured_layer_passes,
    frog_textured_layer_passes, ghast_textured_layer_passes, goat_textured_layer_passes,
    guardian_textured_layer_passes, happy_ghast_textured_layer_passes,
    hoglin_textured_layer_passes, horse_textured_layer_passes, humanoid_armor_layer_pass,
    husk_textured_layer_passes, illager_textured_layer_passes, iron_golem_textured_layer_passes,
    leash_knot_textured_layer_passes, llama_spit_textured_layer_passes,
    llama_textured_layer_passes, magma_cube_textured_layer_passes, minecart_textured_layer_passes,
    mooshroom_textured_layer_passes, nautilus_textured_layer_passes, panda_textured_layer_passes,
    parrot_textured_layer_passes, phantom_textured_layer_passes, pig_textured_layer_passes,
    piglin_textured_layer_passes, player_cape_layer_pass,
    player_extra_ears_layer_pass_with_texture, player_parrot_on_shoulder_layer_pass,
    player_spin_attack_effect_layer_pass, player_textured_layer_passes,
    polar_bear_textured_layer_passes, rabbit_textured_layer_passes, ravager_textured_layer_passes,
    salmon_textured_layer_passes, sheep_textured_layer_passes,
    shulker_bullet_textured_layer_passes, shulker_textured_layer_passes,
    silverfish_textured_layer_passes, skeleton_textured_layer_passes, slime_textured_layer_passes,
    sniffer_textured_layer_passes, snow_golem_textured_layer_passes, spider_textured_layer_passes,
    squid_textured_layer_passes, tadpole_textured_layer_passes, trident_textured_layer_passes,
    tropical_fish_textured_layer_passes, undead_horse_textured_layer_passes,
    villager_data_textured_layer_passes, villager_textured_layer_passes,
    wandering_trader_textured_layer_passes, warden_pulsating_spots_alpha,
    warden_textured_layer_passes, wind_charge_textured_layer_passes, wings_layer_pass,
    witch_textured_layer_passes, wither_skull_textured_layer_passes, wither_textured_layer_passes,
    wolf_textured_layer_passes, zombie_nautilus_textured_layer_passes,
    zombie_textured_layer_passes, zombie_villager_data_textured_layer_passes,
    zombie_villager_textured_layer_passes, EntityModelLayerKind, EntityModelLayerPass,
    EntityModelLayerRenderBucket, EntityModelLayerRenderType, EntityModelLayerVisibility,
};
#[cfg(test)]
use textured::{
    dynamic_player_texture_test_meshes, entity_model_textured_meshes,
    entity_model_textured_meshes_with_dynamic_skins, EntityModelTexturedMeshes,
};

#[cfg(test)]
mod tests;
