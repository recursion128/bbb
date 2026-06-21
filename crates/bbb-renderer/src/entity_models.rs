mod catalog;
mod colored;
mod geometry;
mod gpu;
mod instances;
mod model_layers;
mod textured;

pub use catalog::*;
use colored::{
    boat_model_root_transform, cave_spider_model_root_transform, entity_model_colored_runtime_mesh,
    entity_model_root_transform, magma_cube_model_root_transform, player_model_root_transform,
    polar_bear_model_root_transform, slime_model_root_transform,
    villager_adult_model_root_transform, wither_skeleton_model_root_transform,
};
#[cfg(test)]
use colored::{
    chicken_model_parts, cow_model_parts, death_fall_factor, entity_flip_degrees,
    entity_model_mesh, humanoid_arm_swing_parts, humanoid_limb_swing_parts, pig_model_parts,
    quadruped_leg_x_rotations, quadruped_limb_swing_parts, HUMANOID_ARM_PART_INDICES,
    HUMANOID_LEG_PART_INDICES, QUADRUPED_LEG_PART_INDICES,
};
use geometry::*;
#[cfg(test)]
use glam::Vec3;
#[cfg(test)]
use gpu::{
    build_entity_model_texture_atlas, entity_model_vertex_layout, rgba_offset,
    sanitize_entity_model_instances, ENTITY_MODEL_EYES_SHADER, ENTITY_MODEL_SHADER,
    ENTITY_MODEL_TEXTURED_SHADER, ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES,
    ENTITY_MODEL_VERTEX_ATTRIBUTES,
};
pub(crate) use gpu::{
    create_entity_model_eyes_pipeline, create_entity_model_pipeline,
    create_entity_model_textured_pipeline, create_entity_model_translucent_pipeline,
};
pub(super) use gpu::{EntityModelMeshGpu, EntityModelTextureAtlasGpu, EntityModelTexturedMeshGpu};
pub use instances::*;
#[cfg(test)]
use model_layers::*;
pub use model_layers::{
    boat_entity_texture_refs, chicken_entity_texture_refs, cow_entity_texture_refs,
    creeper_entity_texture_refs, enderman_entity_texture_refs, entity_model_texture_refs,
    goat_entity_texture_refs, hoglin_entity_texture_refs, pig_entity_texture_refs,
    player_entity_texture_refs, polar_bear_entity_texture_refs, ravager_entity_texture_refs,
    sheep_entity_texture_refs, skeleton_entity_texture_refs, slime_entity_texture_refs,
    spider_entity_texture_refs, villager_entity_texture_refs, witch_entity_texture_refs,
    wolf_entity_texture_refs, SheepHeadEatPose,
};
#[cfg(test)]
use textured::entity_model_textured_mesh;
use textured::entity_model_textured_meshes;
#[cfg(test)]
use textured::{
    boat_textured_layer_passes, chicken_textured_layer_passes, cow_textured_layer_passes,
    creeper_textured_layer_passes, enderman_textured_layer_passes, goat_textured_layer_passes,
    hoglin_textured_layer_passes, iron_golem_textured_layer_passes,
    magma_cube_textured_layer_passes, pig_textured_layer_passes, player_textured_layer_passes,
    polar_bear_textured_layer_passes, ravager_textured_layer_passes, sheep_textured_layer_passes,
    skeleton_textured_layer_passes, slime_textured_layer_passes, snow_golem_textured_layer_passes,
    spider_textured_layer_passes, villager_textured_layer_passes,
    wandering_trader_textured_layer_passes, witch_textured_layer_passes,
    wolf_textured_layer_passes, EntityModelLayerKind, EntityModelLayerRenderType,
    EntityModelLayerVisibility,
};

#[cfg(test)]
mod tests;
