use glam::{Mat4, Vec3};

use super::super::catalog::EntityModelKind;
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
        * entity_post_yaw_transform(instance)
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

fn living_entity_model_root_transform_with_renderer_transform(
    instance: EntityModelInstance,
    renderer_transform: Mat4,
) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_rotation_y((180.0 - instance.render_state.body_rot).to_radians())
        * entity_post_yaw_transform(instance)
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * renderer_transform
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

/// Vanilla `LivingEntityRenderer.setupRotations` else-if chain, inserted right
/// after the `180 - bodyRot` yaw and before the `(-1, -1, 1)` flip. The death
/// tip-over takes precedence over the riptide auto-spin (mirroring vanilla's
/// `if deathTime > 0 ... else if isAutoSpinAttack ...`). Identity for a living,
/// non-spinning entity. (Sleeping and Dinnerbone/Grumm upside-down are not yet
/// projected and fall through to identity.)
fn entity_post_yaw_transform(instance: EntityModelInstance) -> Mat4 {
    let death_time = instance.render_state.death_time;
    if death_time > 0.0 {
        return Mat4::from_rotation_z(
            (death_fall_factor(death_time) * entity_flip_degrees(instance.kind)).to_radians(),
        );
    }
    // Vanilla auto-spin attack (riptide): Rx(-90 - xRot) then Ry(ageInTicks * -75),
    // about the post-yaw origin, so it is scale-agnostic like the death flip.
    if let Some(age_ticks) = instance.render_state.auto_spin_age_ticks {
        return Mat4::from_rotation_x((-90.0 - instance.render_state.head_pitch).to_radians())
            * Mat4::from_rotation_y((age_ticks * -75.0).to_radians());
    }
    Mat4::IDENTITY
}

/// Vanilla `LivingEntityRenderer.setupRotations` fall factor: `fall =
/// (deathTime - 1) / 20 * 1.6`, then `fall = sqrt(fall)`, clamped to `1.0`. The
/// vanilla `state.deathTime` is always `>= 1` when the entity is dying (it is the
/// integer `entity.deathTime >= 1` plus a partial tick), so the radicand is never
/// negative in practice; the `max(0.0)` only guards out-of-range inputs.
pub(in crate::entity_models) fn death_fall_factor(death_time: f32) -> f32 {
    (((death_time - 1.0) / 20.0 * 1.6).max(0.0)).sqrt().min(1.0)
}

/// Vanilla `LivingEntityRenderer.getFlipDegrees`: the death tip-over angle. The
/// base living renderer flips `90` degrees (onto its side); `SpiderRenderer`
/// (and the cave spider that extends it) flip `180` degrees (onto its back).
pub(in crate::entity_models) fn entity_flip_degrees(kind: EntityModelKind) -> f32 {
    match kind {
        EntityModelKind::Spider | EntityModelKind::CaveSpider => 180.0,
        _ => 90.0,
    }
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
