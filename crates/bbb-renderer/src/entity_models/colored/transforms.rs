use glam::{Mat4, Vec3};

use super::super::geometry::{part_pose_transform, PartPose};
use super::super::instances::EntityModelInstance;

const VANILLA_MODEL_ROOT_Y_OFFSET: f32 = 1.501;
const MESH_TRANSFORMER_ROOT_Y_OFFSET_PIXELS: f32 = 24.016;
const VILLAGER_LIKE_SCALE: f32 = 0.9375;
const WITHER_SKELETON_SCALE: f32 = 1.2;
const CAVE_SPIDER_SCALE: f32 = 0.7;
const AVATAR_RENDERER_SCALE: f32 = 0.9375;

pub(super) const HUSK_SCALE: f32 = 1.0625;
pub(super) const HORSE_SCALE: f32 = 1.1;
pub(super) const DONKEY_SCALE: f32 = 0.87;
pub(super) const MULE_SCALE: f32 = 0.92;
pub(super) const POLAR_BEAR_SCALE: f32 = 1.2;

pub(in crate::entity_models) fn entity_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_rotation_y((180.0 - instance.render_state.body_rot).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

fn living_entity_model_root_transform_with_renderer_transform(
    instance: EntityModelInstance,
    renderer_transform: Mat4,
) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_rotation_y((180.0 - instance.render_state.body_rot).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * renderer_transform
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

pub(in crate::entity_models) fn boat_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
        * Mat4::from_rotation_y((180.0 - instance.render_state.body_rot).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2)
}

pub(in crate::entity_models) fn slime_model_root_transform(
    instance: EntityModelInstance,
    size: i32,
) -> Mat4 {
    living_entity_model_root_transform_with_renderer_transform(
        instance,
        Mat4::from_scale(Vec3::splat(0.999))
            * Mat4::from_translation(Vec3::new(0.0, 0.001, 0.0))
            * Mat4::from_scale(Vec3::splat(size as f32)),
    )
}

pub(in crate::entity_models) fn magma_cube_model_root_transform(
    instance: EntityModelInstance,
    size: i32,
) -> Mat4 {
    living_entity_model_root_transform_with_renderer_transform(
        instance,
        Mat4::from_scale(Vec3::splat(size as f32)),
    )
}

pub(in crate::entity_models) fn player_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    living_entity_model_root_transform_with_renderer_transform(
        instance,
        Mat4::from_scale(Vec3::splat(AVATAR_RENDERER_SCALE)),
    )
}

pub(in crate::entity_models) fn wither_skeleton_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    mesh_transformer_scaled_model_root_transform(instance, WITHER_SKELETON_SCALE)
}

pub(in crate::entity_models) fn cave_spider_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    mesh_transformer_scaled_model_root_transform(instance, CAVE_SPIDER_SCALE)
}

pub(in crate::entity_models) fn polar_bear_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    mesh_transformer_scaled_model_root_transform(instance, POLAR_BEAR_SCALE)
}

pub(super) fn scaled_model_root_transform(instance: EntityModelInstance, scale: f32) -> Mat4 {
    entity_model_root_transform(instance) * Mat4::from_scale(Vec3::splat(scale))
}

pub(super) fn mesh_transformer_scaled_model_root_transform(
    instance: EntityModelInstance,
    scale: f32,
) -> Mat4 {
    entity_model_root_transform(instance)
        * part_pose_transform(PartPose {
            offset: [
                0.0,
                MESH_TRANSFORMER_ROOT_Y_OFFSET_PIXELS * (1.0 - scale),
                0.0,
            ],
            rotation: [0.0, 0.0, 0.0],
        })
        * Mat4::from_scale(Vec3::splat(scale))
}

pub(in crate::entity_models) fn villager_adult_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    mesh_transformer_scaled_model_root_transform(instance, VILLAGER_LIKE_SCALE)
}
