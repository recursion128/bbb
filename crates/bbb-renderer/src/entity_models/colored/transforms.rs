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

pub(in crate::entity_models) const HUSK_SCALE: f32 = 1.0625;
pub(super) const HORSE_SCALE: f32 = 1.1;
pub(super) const DONKEY_SCALE: f32 = 0.87;
pub(super) const MULE_SCALE: f32 = 0.92;
pub(super) const POLAR_BEAR_SCALE: f32 = 1.2;
const GHAST_SCALE: f32 = 4.5;
const HAPPY_GHAST_SCALE: f32 = 4.0;

pub(in crate::entity_models) fn entity_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(instance.position))
        * entity_pre_scale_translation(instance)
        * Mat4::from_scale(Vec3::splat(instance.render_state.scale))
        * entity_setup_rotations_transform(instance)
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

fn living_entity_model_root_transform_with_renderer_transform(
    instance: EntityModelInstance,
    renderer_transform: Mat4,
) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(instance.position))
        * entity_pre_scale_translation(instance)
        * Mat4::from_scale(Vec3::splat(instance.render_state.scale))
        * entity_setup_rotations_transform(instance)
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * renderer_transform
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

/// Vanilla `LivingEntityRenderer.submit` bed head-offset translate, applied before
/// the entity scale (so it is in world units): `translate(-stepX * headOffset, 0,
/// -stepZ * headOffset)` while sleeping in a bed. Identity otherwise. Our post-`T(pos)`
/// frame is the pre-scale world-unit frame, matching vanilla's pre-`scale(entityScale)`
/// translate.
fn entity_pre_scale_translation(instance: EntityModelInstance) -> Mat4 {
    match instance.render_state.sleeping {
        Some(sleeping) => Mat4::from_translation(Vec3::new(
            sleeping.bed_offset[0],
            0.0,
            sleeping.bed_offset[1],
        )),
        None => Mat4::IDENTITY,
    }
}

/// Vanilla `LivingEntityRenderer.setupRotations` body-yaw stage: the `180 - bodyRot`
/// yaw is skipped while sleeping (`if (!hasPose(SLEEPING))`), then the else-if chain
/// (death/auto-spin/sleeping/upside-down) runs in [`entity_post_yaw_transform`].
fn entity_setup_rotations_transform(instance: EntityModelInstance) -> Mat4 {
    let initial_yaw = if instance.render_state.sleeping.is_some() {
        Mat4::IDENTITY
    } else {
        Mat4::from_rotation_y((180.0 - instance.render_state.body_rot).to_radians())
    };
    initial_yaw * entity_post_yaw_transform(instance)
}

/// Vanilla `LivingEntityRenderer.setupRotations` else-if chain, inserted right
/// after the `180 - bodyRot` yaw and before the `(-1, -1, 1)` flip. The death
/// tip-over takes precedence over the riptide auto-spin, which takes precedence
/// over the Dinnerbone/Grumm upside-down flip (mirroring vanilla's `if deathTime >
/// 0 ... else if isAutoSpinAttack ... else if hasPose(SLEEPING) ... else if
/// isUpsideDown ...`). Identity for a living, upright, awake, non-spinning entity.
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
    // Vanilla sleeping in bed: Ry(angle) then Rz(getFlipDegrees) then Ry(270), laying
    // the model on its side along the bed direction.
    if let Some(sleeping) = instance.render_state.sleeping {
        return Mat4::from_rotation_y(sleeping.yaw_angle.to_radians())
            * Mat4::from_rotation_z(entity_flip_degrees(instance.kind).to_radians())
            * Mat4::from_rotation_y(270.0_f32.to_radians());
    }
    // Vanilla Dinnerbone/Grumm upside-down: translate up by `(bbHeight + 0.1) /
    // entityScale` then flip 180 about Z. The `/ entityScale` divisor undoes the
    // leading `scale(entityScale)` so the world-space lift is exactly `bbHeight + 0.1`
    // (the bounding box height already includes the scale attribute).
    if let Some(height) = instance.render_state.upside_down_height {
        return Mat4::from_translation(Vec3::new(
            0.0,
            (height + 0.1) / instance.render_state.scale,
            0.0,
        )) * Mat4::from_rotation_z(180.0_f32.to_radians());
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

pub(in crate::entity_models) fn mesh_transformer_scaled_model_root_transform(
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

/// Vanilla `GhastModel.createBodyLayer` bakes `MeshTransformer.scaling(4.5F)` into the layer.
pub(in crate::entity_models) fn ghast_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    mesh_transformer_scaled_model_root_transform(instance, GHAST_SCALE)
}

pub(in crate::entity_models) fn happy_ghast_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    mesh_transformer_scaled_model_root_transform(instance, HAPPY_GHAST_SCALE)
}

/// Vanilla `PhantomRenderer` transform overrides. `setupRotations` appends an extra
/// `Axis.XP.rotationDegrees(state.xRot)` body pitch right after the standard body-yaw stage
/// (before the `(-1, -1, 1)` flip); `state.xRot` is the projected `head_pitch` (the entity
/// pitch, already negated when upside down). The `scale()` override then scales by `1 + 0.15
/// * size` and translates `(0, 1.3125, 0.1875)` in the scaled frame (the `this.scale()` slot,
/// between the flip and the `-1.501` model-Y offset).
pub(in crate::entity_models) fn phantom_model_root_transform(
    instance: EntityModelInstance,
    size: i32,
) -> Mat4 {
    let scale = 1.0 + 0.15 * size as f32;
    Mat4::from_translation(Vec3::from_array(instance.position))
        * entity_pre_scale_translation(instance)
        * Mat4::from_scale(Vec3::splat(instance.render_state.scale))
        * entity_setup_rotations_transform(instance)
        * Mat4::from_rotation_x(instance.render_state.head_pitch.to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_scale(Vec3::splat(scale))
        * Mat4::from_translation(Vec3::new(0.0, 1.3125, 0.1875))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

/// Vanilla `PufferfishRenderer.setupRotations` vertical bob, applied before the standard
/// body-yaw stage: `translate(0, cos(ageInTicks * 0.05) * 0.08, 0)`. Inserted in the
/// post-scale (`entityScale`) frame, exactly where vanilla calls `poseStack.translate`
/// before `super.setupRotations`.
pub(in crate::entity_models) fn pufferfish_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    let bob = (instance.render_state.age_in_ticks * 0.05).cos() * 0.08;
    Mat4::from_translation(Vec3::from_array(instance.position))
        * entity_pre_scale_translation(instance)
        * Mat4::from_scale(Vec3::splat(instance.render_state.scale))
        * Mat4::from_translation(Vec3::new(0.0, bob, 0.0))
        * entity_setup_rotations_transform(instance)
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}
