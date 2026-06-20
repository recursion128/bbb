mod catalog;
mod colored;
mod geometry;
mod gpu;
mod model_layers;
mod textured;

pub use catalog::*;
use colored::{
    boat_model_root_transform, entity_model_colored_runtime_mesh, entity_model_root_transform,
    player_model_root_transform,
};
#[cfg(test)]
use colored::{chicken_model_parts, cow_model_parts, entity_model_mesh, pig_model_parts};
use geometry::*;
#[cfg(test)]
use glam::Vec3;
#[cfg(test)]
use gpu::{
    build_entity_model_texture_atlas, entity_model_vertex_layout, rgba_offset,
    sanitize_entity_model_instances, ENTITY_MODEL_TEXTURED_SHADER,
    ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES, ENTITY_MODEL_VERTEX_ATTRIBUTES,
};
pub(crate) use gpu::{create_entity_model_pipeline, create_entity_model_textured_pipeline};
pub(super) use gpu::{EntityModelMeshGpu, EntityModelTextureAtlasGpu, EntityModelTexturedMeshGpu};
#[cfg(test)]
use model_layers::*;
pub use model_layers::{
    boat_entity_texture_refs, chicken_entity_texture_refs, cow_entity_texture_refs,
    entity_model_texture_refs, pig_entity_texture_refs, player_entity_texture_refs,
    sheep_entity_texture_refs, wolf_entity_texture_refs,
};
use textured::entity_model_textured_mesh;
#[cfg(test)]
use textured::{
    boat_textured_layer_passes, chicken_textured_layer_passes, cow_textured_layer_passes,
    pig_textured_layer_passes, player_textured_layer_passes, sheep_textured_layer_passes,
    wolf_textured_layer_passes, EntityModelLayerKind,
};

#[cfg(test)]
mod tests;
