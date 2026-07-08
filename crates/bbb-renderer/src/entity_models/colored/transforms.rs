use glam::{Mat4, Vec3};

use super::super::catalog::{
    ConduitModelPart, EntityAttachmentFace, EntityModelKind, SalmonModelSize, SignModelAttachment,
    SkullBlockModelAttachment,
};
use super::super::geometry::{part_pose_transform, PartPose};
use super::super::instances::EntityModelInstance;

const VANILLA_MODEL_ROOT_Y_OFFSET: f32 = 1.501;
const MESH_TRANSFORMER_ROOT_Y_OFFSET_PIXELS: f32 = 24.016;
const VILLAGER_LIKE_SCALE: f32 = 0.9375;
const WITHER_SKELETON_SCALE: f32 = 1.2;
const CAVE_SPIDER_SCALE: f32 = 0.7;
const AVATAR_RENDERER_SCALE: f32 = 0.9375;
const ENDERMAN_CREEPY_RENDER_JITTER: f64 = 0.02;
const RANDOM_MULTIPLIER: u64 = 25_214_903_917;
const RANDOM_INCREMENT: u64 = 11;
const RANDOM_MASK: u64 = (1_u64 << 48) - 1;

pub(in crate::entity_models) const HUSK_SCALE: f32 = 1.0625;
pub(in crate::entity_models) const GIANT_SCALE: f32 = 6.0;
pub(in crate::entity_models) const HORSE_SCALE: f32 = 1.1;
pub(super) const DONKEY_SCALE: f32 = 0.87;
pub(super) const MULE_SCALE: f32 = 0.92;
pub(super) const POLAR_BEAR_SCALE: f32 = 1.2;
const ARMOR_STAND_WOBBLE_TIME: f32 = 5.0;
const GHAST_SCALE: f32 = 4.5;
const HAPPY_GHAST_SCALE: f32 = 4.0;

fn entity_root_position_transform(instance: EntityModelInstance) -> Mat4 {
    let position = Mat4::from_translation(Vec3::from_array(instance.position));
    if let Some(spawner) = instance.render_state.spawner_display {
        position
            * Mat4::from_translation(Vec3::new(0.5, 0.4, 0.5))
            * Mat4::from_rotation_y(spawner.spin_degrees.to_radians())
            * Mat4::from_translation(Vec3::new(0.0, -0.2, 0.0))
            * Mat4::from_rotation_x(-30.0_f32.to_radians())
            * Mat4::from_scale(Vec3::splat(spawner.scale))
    } else {
        position
    }
}

pub(in crate::entity_models) fn entity_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    let setup_rotation_tail = match instance.kind {
        EntityModelKind::ArmorStand { .. } => {
            armor_stand_setup_rotation_tail(instance.render_state.armor_stand_wiggle)
        }
        _ => Mat4::IDENTITY,
    };
    living_entity_model_root_transform_with_extra_setup_rotation(instance, setup_rotation_tail)
}

/// Vanilla `ShulkerRenderer.setupRotations`: call the living setup with `bodyRot + 180`, then rotate
/// around `(0, 0.5, 0)` by `attachFace.getOpposite().getRotation()`. Passing `bodyRot + 180` changes
/// the normal non-sleeping yaw stage from `180 - bodyRot` to `-bodyRot`.
pub(in crate::entity_models) fn shulker_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    entity_root_position_transform(instance)
        * entity_pre_scale_translation(instance)
        * Mat4::from_scale(Vec3::splat(instance.render_state.scale))
        * shulker_setup_rotations_transform(instance)
        * shulker_attach_face_transform(instance.render_state.shulker_attach_face)
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

fn living_entity_model_root_transform_with_extra_setup_rotation(
    instance: EntityModelInstance,
    setup_rotation_tail: Mat4,
) -> Mat4 {
    entity_root_position_transform(instance)
        * entity_render_offset_transform(instance)
        * entity_pre_scale_translation(instance)
        * Mat4::from_scale(Vec3::splat(instance.render_state.scale))
        * entity_setup_rotations_transform(instance)
        * setup_rotation_tail
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

/// Vanilla `IllusionerRenderer.submit` applies each illusion offset after the
/// dispatcher's entity-position frame and before `LivingEntityRenderer.submit`
/// runs the scale / setup-rotations / model flip chain.
pub(in crate::entity_models) fn illusioner_model_root_transform(
    instance: EntityModelInstance,
    clone_offset: [f32; 3],
) -> Mat4 {
    entity_root_position_transform(instance)
        * entity_render_offset_transform(instance)
        * Mat4::from_translation(Vec3::from_array(clone_offset))
        * entity_pre_scale_translation(instance)
        * Mat4::from_scale(Vec3::splat(instance.render_state.scale))
        * entity_setup_rotations_transform(instance)
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
        * mesh_transformer_scale_transform(VILLAGER_LIKE_SCALE)
}

fn living_entity_model_root_transform_with_renderer_transform(
    instance: EntityModelInstance,
    renderer_transform: Mat4,
) -> Mat4 {
    entity_root_position_transform(instance)
        * entity_render_offset_transform(instance)
        * entity_pre_scale_translation(instance)
        * Mat4::from_scale(Vec3::splat(instance.render_state.scale))
        * entity_setup_rotations_transform(instance)
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * renderer_transform
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

fn entity_render_offset_transform(instance: EntityModelInstance) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(enderman_creepy_render_offset(instance)))
}

pub(in crate::entity_models) fn enderman_creepy_render_offset(
    instance: EntityModelInstance,
) -> [f32; 3] {
    if !matches!(instance.kind, EntityModelKind::Enderman) || !instance.render_state.enderman_creepy
    {
        return [0.0; 3];
    }

    let mut random = LegacyRandom::new(enderman_creepy_render_offset_seed(instance));
    let amplitude = ENDERMAN_CREEPY_RENDER_JITTER * f64::from(instance.render_state.scale);
    [
        (random.next_gaussian() * amplitude) as f32,
        0.0,
        (random.next_gaussian() * amplitude) as f32,
    ]
}

fn enderman_creepy_render_offset_seed(instance: EntityModelInstance) -> i64 {
    let entity_id = instance.entity_id as u32 as u64;
    let age_bits = u64::from(instance.render_state.age_in_ticks.to_bits());
    let seed = 0x9E37_79B9_7F4A_7C15_u64
        ^ entity_id.wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ age_bits.rotate_left(17);
    seed as i64
}

struct LegacyRandom {
    seed: u64,
    next_gaussian: Option<f64>,
}

impl LegacyRandom {
    fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ RANDOM_MULTIPLIER) & RANDOM_MASK,
            next_gaussian: None,
        }
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(RANDOM_MULTIPLIER)
            .wrapping_add(RANDOM_INCREMENT)
            & RANDOM_MASK;
        (self.seed >> (48 - bits)) as u32
    }

    fn next_f64(&mut self) -> f64 {
        let high = u64::from(self.next_bits(26)) << 27;
        let low = u64::from(self.next_bits(27));
        (high + low) as f64 / ((1_u64 << 53) as f64)
    }

    fn next_gaussian(&mut self) -> f64 {
        if let Some(value) = self.next_gaussian.take() {
            return value;
        }

        loop {
            let x = 2.0 * self.next_f64() - 1.0;
            let y = 2.0 * self.next_f64() - 1.0;
            let radius = x * x + y * y;
            if radius < 1.0 && radius != 0.0 {
                let multiplier = (-2.0 * radius.ln() / radius).sqrt();
                self.next_gaussian = Some(y * multiplier);
                return x * multiplier;
            }
        }
    }
}

/// Vanilla `CreeperRenderer.scale`: the non-uniform swell scale applied at the per-renderer
/// `this.scale()` hook (after the `(-1, -1, 1)` flip, before the `-1.501` y-offset) while a
/// creeper primes to explode. `wobble = 1 + sin(swelling * 100) * swelling * 0.01` flickers
/// the size; `g = clamp(swelling, 0, 1)^4` drives the steady inflation `s = (1 + g * 0.4) *
/// wobble` on X/Z and `hs = (1 + g * 0.1) / wobble` on Y. At `swelling = 0` it is the
/// identity (`s = hs = 1`), so a calm creeper is unscaled.
fn creeper_swell_scale(swelling: f32) -> [f32; 3] {
    let wobble = 1.0 + (swelling * 100.0).sin() * swelling * 0.01;
    let g = swelling.clamp(0.0, 1.0);
    let g = g * g;
    let g = g * g;
    let s = (1.0 + g * 0.4) * wobble;
    let hs = (1.0 + g * 0.1) / wobble;
    [s, hs, s]
}

/// Vanilla `CreeperRenderer` root transform: the shared living-entity transform with the
/// [`creeper_swell_scale`] inserted at the `this.scale()` hook, so a priming creeper inflates
/// and flickers. Reduces to [`entity_model_root_transform`] for a calm creeper (swell scale
/// `1`).
pub(in crate::entity_models) fn creeper_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    let [sx, sy, sz] = creeper_swell_scale(instance.render_state.creeper_swelling);
    living_entity_model_root_transform_with_renderer_transform(
        instance,
        Mat4::from_scale(Vec3::new(sx, sy, sz)),
    )
}

/// Vanilla `IronGolemRenderer.setupRotations`: after the standard living-entity setup rotation and
/// before the model flip / `-1.501` translate, a walking golem applies a whole-body Z wobble of
/// `6.5 * triangleWave(walkAnimationPos + 6, 13)` degrees. The wobble is skipped while the walk
/// animation speed is below `0.01`.
pub(in crate::entity_models) fn iron_golem_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    living_entity_model_root_transform_with_extra_setup_rotation(
        instance,
        iron_golem_setup_rotation_tail(
            instance.render_state.walk_animation_pos,
            instance.render_state.walk_animation_speed,
        ),
    )
}

fn iron_golem_setup_rotation_tail(walk_animation_pos: f32, walk_animation_speed: f32) -> Mat4 {
    if walk_animation_speed < 0.01 {
        return Mat4::IDENTITY;
    }

    let wave_pos = walk_animation_pos + 6.0;
    let triangle_wave = ((wave_pos % 13.0 - 6.5).abs() - 3.25) / 3.25;
    Mat4::from_rotation_z((6.5 * triangle_wave).to_radians())
}

/// Vanilla `ArmorStandRenderer.setupRotations`: after the armor stand's own `180 - yRot` yaw and before
/// the model flip / `-1.501` translate, a recent hit adds
/// `sin(wiggle / 1.5 * PI) * 3` degrees around Y. `wiggle >= ArmorStand.WOBBLE_TIME` is rest.
fn armor_stand_setup_rotation_tail(wiggle: f32) -> Mat4 {
    if wiggle >= ARMOR_STAND_WOBBLE_TIME {
        return Mat4::IDENTITY;
    }
    Mat4::from_rotation_y(((wiggle / 1.5 * std::f32::consts::PI).sin() * 3.0).to_radians())
}

/// Vanilla `FoxRenderer.setupRotations`: after the standard living-entity setup rotation
/// (`super.setupRotations`) and before the model flip / `-1.501` translate, a pouncing or faceplanted
/// fox applies `Axis.XP.rotationDegrees(-state.xRot)`. This pitches the whole model and all layers by
/// the render-state head pitch; every other fox uses the standard root transform.
pub(in crate::entity_models) fn fox_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    let setup_rotation_tail =
        if instance.render_state.fox_is_pouncing || instance.render_state.fox_is_faceplanted {
            Mat4::from_rotation_x((-instance.render_state.head_pitch).to_radians())
        } else {
            Mat4::IDENTITY
        };
    living_entity_model_root_transform_with_extra_setup_rotation(instance, setup_rotation_tail)
}

/// Vanilla `CatRenderer.setupRotations`: after the standard living setup rotation and before the
/// model flip / `-1.501` translate, a lying cat shifts and rolls the whole model by
/// `lieDownAmount`. The sleeping-player extra x-translate remains a later entity-neighborhood
/// projection; this helper applies the base lie-down transform shared by the base and collar layers.
pub(in crate::entity_models) fn feline_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    living_entity_model_root_transform_with_extra_setup_rotation(
        instance,
        feline_setup_rotation_tail(
            instance.render_state.feline_lie_down_amount,
            instance
                .render_state
                .feline_is_lying_on_top_of_sleeping_player,
        ),
    )
}

fn feline_setup_rotation_tail(
    lie_down_amount: f32,
    is_lying_on_top_of_sleeping_player: bool,
) -> Mat4 {
    if lie_down_amount <= 0.0 {
        return Mat4::IDENTITY;
    }
    let mut transform = Mat4::from_translation(Vec3::new(
        0.4 * lie_down_amount,
        0.15 * lie_down_amount,
        0.1 * lie_down_amount,
    )) * Mat4::from_rotation_z((90.0 * lie_down_amount).to_radians());
    if is_lying_on_top_of_sleeping_player {
        transform = transform * Mat4::from_translation(Vec3::new(0.15 * lie_down_amount, 0.0, 0.0));
    }
    transform
}

/// Vanilla `PandaRenderer.setupRotations`: after the standard living setup rotation and before the
/// model flip / `-1.501` translate, a panda applies the whole-body roll tumble, sitting tilt, scared
/// shake, and lie-on-back tilt from `PandaRenderState.rollTime` / `sitAmount` / `lieOnBackAmount`.
pub(in crate::entity_models) fn panda_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    let setup_rotation_tail = panda_setup_rotation_tail(instance);
    living_entity_model_root_transform_with_extra_setup_rotation(instance, setup_rotation_tail)
}

fn panda_setup_rotation_tail(instance: EntityModelInstance) -> Mat4 {
    let is_baby = matches!(instance.kind, EntityModelKind::Panda { baby: true, .. });
    let render_state = instance.render_state;
    let mut transform = Mat4::IDENTITY;

    if render_state.panda_roll_time > 0.0 {
        transform = transform * panda_roll_transform(render_state.panda_roll_time, is_baby);
    }

    let sit_amount = render_state.panda_sit_amount;
    if sit_amount > 0.0 {
        transform = transform
            * Mat4::from_translation(Vec3::new(0.0, 0.8 * sit_amount, 0.0))
            * Mat4::from_rotation_x((render_state.head_pitch + 90.0 * sit_amount).to_radians())
            * Mat4::from_translation(Vec3::new(0.0, -sit_amount, 0.0));
        if render_state.panda_scared {
            let shake_rot = (render_state.age_in_ticks * 1.25).cos() * std::f32::consts::PI * 0.05;
            transform = transform * Mat4::from_rotation_y(shake_rot.to_radians());
            if is_baby {
                transform = transform * Mat4::from_translation(Vec3::new(0.0, 0.8, 0.55));
            }
        }
    }

    let lie_on_back_amount = render_state.panda_lie_on_back_amount;
    if lie_on_back_amount > 0.0 {
        let y = if is_baby { 0.5 } else { 1.3 };
        transform = transform
            * Mat4::from_translation(Vec3::new(0.0, y * lie_on_back_amount, 0.0))
            * Mat4::from_rotation_x(
                (render_state.head_pitch + 180.0 * lie_on_back_amount).to_radians(),
            );
    }

    transform
}

fn panda_roll_transform(roll_time: f32, is_baby: bool) -> Mat4 {
    let roll_transition_time = roll_time.fract();
    let roll_pos = roll_time.floor() as i32;
    let next_roll_pos = roll_pos + 1;
    let y = if is_baby { 0.3 } else { 0.8 };

    let (angle, translate_y) = if roll_pos < 8 {
        let this_angle = 90.0 * roll_pos as f32 / 7.0;
        let next_angle = 90.0 * next_roll_pos as f32 / 7.0;
        let angle = panda_roll_angle(
            this_angle,
            next_angle,
            next_roll_pos,
            roll_transition_time,
            8,
        );
        (angle, (y + 0.2) * (angle / 90.0))
    } else if roll_pos < 16 {
        let internal_roll_counter = (roll_pos - 8) as f32 / 7.0;
        let this_angle = 90.0 + 90.0 * internal_roll_counter;
        let next_angle = 90.0 + 90.0 * (next_roll_pos - 8) as f32 / 7.0;
        let angle = panda_roll_angle(
            this_angle,
            next_angle,
            next_roll_pos,
            roll_transition_time,
            16,
        );
        (angle, y + 0.2 + (y - 0.2) * (angle - 90.0) / 90.0)
    } else if roll_pos < 24 {
        let internal_roll_counter = (roll_pos - 16) as f32 / 7.0;
        let this_angle = 180.0 + 90.0 * internal_roll_counter;
        let next_angle = 180.0 + 90.0 * (next_roll_pos - 16) as f32 / 7.0;
        let angle = panda_roll_angle(
            this_angle,
            next_angle,
            next_roll_pos,
            roll_transition_time,
            24,
        );
        (angle, y + y * (270.0 - angle) / 90.0)
    } else if roll_pos < 32 {
        let internal_roll_counter = (roll_pos - 24) as f32 / 7.0;
        let this_angle = 270.0 + 90.0 * internal_roll_counter;
        let next_angle = 270.0 + 90.0 * (next_roll_pos - 24) as f32 / 7.0;
        let angle = panda_roll_angle(
            this_angle,
            next_angle,
            next_roll_pos,
            roll_transition_time,
            32,
        );
        (angle, y * ((360.0 - angle) / 90.0))
    } else {
        return Mat4::IDENTITY;
    };

    Mat4::from_translation(Vec3::new(0.0, translate_y, 0.0))
        * Mat4::from_rotation_x((-angle).to_radians())
}

fn panda_roll_angle(
    this_angle: f32,
    next_angle: f32,
    next_roll_pos: i32,
    roll_transition_time: f32,
    threshold: i32,
) -> f32 {
    if next_roll_pos < threshold {
        this_angle + roll_transition_time * (next_angle - this_angle)
    } else {
        this_angle
    }
}

/// Vanilla `DrownedRenderer.setupRotations`: after the standard living setup, a visually swimming
/// drowned rotates around `y = boundingBoxHeight / 2 / entityScale` by
/// `lerp(swimAmount, 0, -10 - xRot)` degrees. The rotation is identity when `swimAmount == 0`.
pub(in crate::entity_models) fn drowned_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    let swim_amount = instance.render_state.swim_amount;
    let setup_rotation_tail = if swim_amount > 0.0 {
        let rotation_x = swim_amount * (-10.0 - instance.render_state.head_pitch);
        let pivot_y = instance.render_state.bounding_box_height / 2.0 / instance.render_state.scale;
        Mat4::from_translation(Vec3::new(0.0, pivot_y, 0.0))
            * Mat4::from_rotation_x(rotation_x.to_radians())
            * Mat4::from_translation(Vec3::new(0.0, -pivot_y, 0.0))
    } else {
        Mat4::IDENTITY
    };
    living_entity_model_root_transform_with_extra_setup_rotation(instance, setup_rotation_tail)
}

/// Vanilla `WitherBossRenderer.scale` uniform model scale, applied at the per-renderer `this.scale()`
/// hook (after the `(-1, -1, 1)` flip, before the `-1.501` y-offset): `scale = 2.0`, minus
/// `invulnerableTicks / 220 * 0.5` while the wither is mid-spawn (`invulnerableTicks > 0`). So a
/// fully-spawned wither is a flat `2.0×`, and a freshly-summoned one starts at `1.5×` and grows to
/// full over its 220-tick spawn charge.
pub(in crate::entity_models) fn wither_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    let invulnerable_ticks = instance.render_state.wither_invulnerable_ticks;
    let mut scale = 2.0;
    if invulnerable_ticks > 0.0 {
        scale -= invulnerable_ticks / 220.0 * 0.5;
    }
    living_entity_model_root_transform_with_renderer_transform(
        instance,
        Mat4::from_scale(Vec3::splat(scale)),
    )
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

fn shulker_setup_rotations_transform(instance: EntityModelInstance) -> Mat4 {
    let initial_yaw = if instance.render_state.sleeping.is_some() {
        Mat4::IDENTITY
    } else {
        Mat4::from_rotation_y((-instance.render_state.body_rot).to_radians())
    };
    initial_yaw * entity_post_yaw_transform(instance)
}

fn shulker_attach_face_transform(face: EntityAttachmentFace) -> Mat4 {
    use std::f32::consts::{FRAC_PI_2, PI};

    let rotation = match face {
        // `attachFace.getOpposite()` is UP, whose `Direction.getRotation()` is identity.
        EntityAttachmentFace::Down => Mat4::IDENTITY,
        EntityAttachmentFace::Up => Mat4::from_rotation_x(PI),
        EntityAttachmentFace::North => Mat4::from_rotation_x(FRAC_PI_2),
        EntityAttachmentFace::South => Mat4::from_rotation_x(FRAC_PI_2) * Mat4::from_rotation_z(PI),
        EntityAttachmentFace::West => {
            Mat4::from_rotation_x(FRAC_PI_2) * Mat4::from_rotation_z(-FRAC_PI_2)
        }
        EntityAttachmentFace::East => {
            Mat4::from_rotation_x(FRAC_PI_2) * Mat4::from_rotation_z(FRAC_PI_2)
        }
    };
    let pivot = Vec3::new(0.0, 0.5, 0.0);
    Mat4::from_translation(pivot) * rotation * Mat4::from_translation(-pivot)
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
/// base living renderer flips `90` degrees (onto its side); `SpiderRenderer`,
/// `EndermiteRenderer`, and `SilverfishRenderer` flip `180` degrees.
pub(in crate::entity_models) fn entity_flip_degrees(kind: EntityModelKind) -> f32 {
    match kind {
        EntityModelKind::Endermite
        | EntityModelKind::Silverfish
        | EntityModelKind::Spider
        | EntityModelKind::CaveSpider => 180.0,
        _ => 90.0,
    }
}

/// Vanilla `EnderDragonRenderer.submit`: a plain `EntityRenderer` that applies the flight-history
/// yaw `Axis.YP.rotationDegrees(-yr)`, a flight-history pitch `Axis.XP.rotationDegrees(rot2 * 10)`, a
/// fixed `translate(0, 0, 1)`, then the standard flip and `-1.501` y-offset. The whole
/// `EnderDragonModel.setupAnim` is procedural (the neck/tail segments are re-placed from the flight
/// history, the wings flap, the jaw opens, plus the `root.y/z/xRot` bounce), so it is deferred and
/// the model renders at its `createBodyLayer` bind layout. The pitch (`rot2`) and the bounce are
/// deferred (identity at rest); the yaw is projected through `body_rot`.
pub(in crate::entity_models) fn ender_dragon_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    entity_root_position_transform(instance)
        * Mat4::from_rotation_y((-instance.render_state.body_rot).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

/// Vanilla `EndCrystalRenderer.submit`: a plain `EntityRenderer` (not `LivingEntityRenderer`), so
/// there is no body-yaw / `setupRotations` flip. The model is authored right-side-up (the base at
/// model-y `0..4`, the glass orbiting at model-y `24`), and the renderer applies only
/// `poseStack.scale(2.0)` then `poseStack.translate(0, -0.5, 0)` before submitting the model. The
/// procedural spin and the `getY` vertical bob are deferred, so this is the static transform.
pub(in crate::entity_models) fn end_crystal_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    entity_root_position_transform(instance)
        * Mat4::from_scale(Vec3::splat(2.0))
        * Mat4::from_translation(Vec3::new(0.0, -0.5, 0.0))
}

/// Vanilla `EvokerFangsRenderer.submit`: a plain `EntityRenderer` that applies the standard model
/// flip and `-1.501` y-offset, but a distinct yaw `Axis.YP.rotationDegrees(90 - yRot)` (the entity's
/// own yaw, not a `LivingEntityRenderer` body rotation) and no render scale / setup-rotations chain.
/// The bite animation, the base drop, and the emerge scale are deferred, so this is the static
/// transform. The fang yaw is projected through `body_rot` (the instance's `y_rot`).
pub(in crate::entity_models) fn evoker_fangs_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    entity_root_position_transform(instance)
        * Mat4::from_rotation_y((90.0 - instance.render_state.body_rot).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

/// Vanilla `ArrowRenderer.submit`: a plain `EntityRenderer` that orients the arrow along its flight
/// direction with `Axis.YP.rotationDegrees(yRot - 90)` then `Axis.ZP.rotationDegrees(xRot)` (no flip
/// / y-offset). `ArrowModel.createBodyLayer` bakes the whole mesh through `mesh.transformed(pose ->
/// pose.scaled(0.9))`, captured here as the trailing `scale(0.9)`. The arrow yaw/pitch are projected
/// through `body_rot` / `head_pitch` (the instance's `y_rot` and head pitch); `ArrowModel.setupAnim`
/// applies the impact-shake root zRot inside the model tree.
pub(in crate::entity_models) fn arrow_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    entity_root_position_transform(instance)
        * Mat4::from_rotation_y((instance.render_state.body_rot - 90.0).to_radians())
        * Mat4::from_rotation_z(instance.render_state.head_pitch.to_radians())
        * Mat4::from_scale(Vec3::splat(0.9))
}

/// Vanilla `ThrownTridentRenderer.submit`: a plain `EntityRenderer` that orients the trident along
/// its flight with `Axis.YP.rotationDegrees(yRot - 90)` then `Axis.ZP.rotationDegrees(xRot + 90)`
/// (no flip / scale / y-offset; the `+90` points the upright pole along the flight axis).
/// `TridentModel` is a `Model<Unit>` with no animation, so this is the complete transform. The
/// textured base submit is wired, and foil reuses this same transform for its `entityGlint`
/// submission. The yaw/pitch are projected through `body_rot` / `head_pitch`.
pub(in crate::entity_models) fn trident_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    entity_root_position_transform(instance)
        * Mat4::from_rotation_y((instance.render_state.body_rot - 90.0).to_radians())
        * Mat4::from_rotation_z((instance.render_state.head_pitch + 90.0).to_radians())
}

/// Vanilla `LeashKnotRenderer.submit`: a plain `EntityRenderer` that applies only the standard model
/// flip (`scale(-1, -1, 1)`) — no yaw, no `-1.501` y-offset, no render scale. `LeashKnotModel` has no
/// `setupAnim`, so this is the complete (not deferred) transform; only the texture is colored-first.
pub(in crate::entity_models) fn leash_knot_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    entity_root_position_transform(instance) * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

fn conduit_active_bob(anim_time: f32) -> f32 {
    let wave = (anim_time * 0.1).sin() / 2.0 + 0.5;
    wave * wave + wave
}

/// Vanilla `ConduitRenderer.submit`: inactive conduits render the shell centered in the block with
/// `Axis.YP.rotation(state.activeRotation * PI / 180)`. Active conduits submit each part separately:
/// the cage bobs and rotates around normalized `(0.5, 1, 0.5)`, the outer wind may rotate by phase,
/// the inner wind is scaled/flipped, and the eye bobs while billboarded toward the camera.
pub(in crate::entity_models) fn conduit_model_root_transform(
    instance: EntityModelInstance,
    part: ConduitModelPart,
) -> Mat4 {
    use std::f32::consts::{FRAC_PI_2, PI};

    let block = entity_root_position_transform(instance);
    let center = Mat4::from_translation(Vec3::splat(0.5));
    match part {
        ConduitModelPart::Shell => {
            block
                * center
                * Mat4::from_rotation_y(instance.render_state.conduit_active_rotation * PI / 180.0)
        }
        ConduitModelPart::Cage => {
            let bob = conduit_active_bob(instance.render_state.conduit_anim_time);
            block
                * Mat4::from_translation(Vec3::new(0.5, 0.3 + bob * 0.2, 0.5))
                * Mat4::from_axis_angle(
                    Vec3::new(0.5, 1.0, 0.5).normalize(),
                    instance.render_state.conduit_active_rotation,
                )
        }
        ConduitModelPart::OuterWind { phase } => {
            let phase_rotation = match phase {
                1 => Mat4::from_rotation_x(FRAC_PI_2),
                2 => Mat4::from_rotation_z(FRAC_PI_2),
                _ => Mat4::IDENTITY,
            };
            block * center * phase_rotation
        }
        ConduitModelPart::InnerWind { .. } => {
            block
                * center
                * Mat4::from_scale(Vec3::splat(0.875))
                * Mat4::from_rotation_x(PI)
                * Mat4::from_rotation_z(PI)
        }
        ConduitModelPart::Eye { .. } => {
            let bob = conduit_active_bob(instance.render_state.conduit_anim_time);
            block
                * Mat4::from_translation(Vec3::new(0.5, 0.3 + bob * 0.2, 0.5))
                * Mat4::from_scale(Vec3::splat(0.5))
                * Mat4::from_rotation_y(-instance.render_state.body_rot.to_radians())
                * Mat4::from_rotation_x(instance.render_state.head_pitch.to_radians())
                * Mat4::from_rotation_z(PI)
                * Mat4::from_rotation_y(PI)
                * Mat4::from_scale(Vec3::splat(4.0 / 3.0))
        }
    }
}

/// Vanilla `SkullBlockRenderer.TRANSFORMATIONS`: ground heads translate to the block
/// center, rotate by the 16-segment yaw, then `scale(-1, -1, 1)`; wall heads
/// translate a quarter block off the supporting wall and rotate by
/// `-facing.getOpposite().toYRot()`.
pub(in crate::entity_models) fn skull_block_model_root_transform(
    instance: EntityModelInstance,
    attachment: SkullBlockModelAttachment,
) -> Mat4 {
    let block = entity_root_position_transform(instance);
    match attachment {
        SkullBlockModelAttachment::Ground => {
            block
                * Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5))
                * Mat4::from_rotation_y(instance.render_state.body_rot.to_radians())
                * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        }
        SkullBlockModelAttachment::Wall { facing } => {
            let (step_x, step_z, opposite_y_rot) = match facing {
                EntityAttachmentFace::North => (0.0_f32, -1.0_f32, 0.0_f32),
                EntityAttachmentFace::South => (0.0_f32, 1.0_f32, 180.0_f32),
                EntityAttachmentFace::West => (-1.0_f32, 0.0_f32, -90.0_f32),
                EntityAttachmentFace::East => (1.0_f32, 0.0_f32, 90.0_f32),
                EntityAttachmentFace::Down | EntityAttachmentFace::Up => {
                    (0.0_f32, 0.0_f32, 0.0_f32)
                }
            };
            block
                * Mat4::from_translation(Vec3::new(0.5 - step_x * 0.25, 0.25, 0.5 - step_z * 0.25))
                * Mat4::from_rotation_y((-opposite_y_rot).to_radians())
                * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        }
    }
}

/// Vanilla `ChestRenderer.submit`: `poseStack.mulPose(modelTransformation(state.facing))`, where
/// `createModelTransformation` is `new Matrix4f().rotationAround(Axis.YP.rotationDegrees(
/// -facing.toYRot()), 0.5F, 0.0F, 0.5F)` (`ChestRenderer.java:115-121`) — a yaw about the vertical
/// axis through the block centre, with **no** entity `scale(-1, -1, 1)` flip (the chest mesh is
/// authored Y-up in block-local space). `instance.position` is the chest block's min corner and
/// `body_rot` carries `-facing.toYRot()` degrees.
pub(in crate::entity_models) fn chest_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    let pivot = Vec3::new(0.5, 0.0, 0.5);
    entity_root_position_transform(instance)
        * Mat4::from_translation(pivot)
        * Mat4::from_rotation_y(instance.render_state.body_rot.to_radians())
        * Mat4::from_translation(-pivot)
}

/// Vanilla `BedRenderer.createModelTransform` (`BedRenderer.java:157-164`):
/// `new Matrix4f().translation(0, 0.5625, 0).rotate(Axis.XP.rotationDegrees(90))
/// .rotateAround(Axis.ZP.rotationDegrees(180 + direction.toYRot()), 0.5, 0.5, 0.5)` — lay the
/// flat-authored bed mesh down (`Rx(90°)`), then spin it about the block centre in that rotated
/// frame. Like the chest there is **no** entity `scale(-1, -1, 1)` flip. `instance.position` is
/// the bed block's min corner and `body_rot` carries `180 + facing.toYRot()` degrees.
pub(in crate::entity_models) fn bed_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    let pivot = Vec3::new(0.5, 0.5, 0.5);
    entity_root_position_transform(instance)
        * Mat4::from_translation(Vec3::new(0.0, 0.5625, 0.0))
        * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2)
        * Mat4::from_translation(pivot)
        * Mat4::from_rotation_z(instance.render_state.body_rot.to_radians())
        * Mat4::from_translation(-pivot)
}

/// Vanilla `BellRenderer.submit`: no pose-stack transform at all — the bell body is submitted in
/// block-local space at the block min corner for every attachment (the facing-dependent support
/// frame is part of the `bell_*` block models the terrain path draws), and the shake rotation
/// lives on the `bell_body` part pivot inside `BellModel.setupAnim`.
pub(in crate::entity_models) fn bell_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    entity_root_position_transform(instance)
}

/// Vanilla `Direction.getRotation()` (`Direction.java:144-153`) for the shulker box `FACING`:
/// `UP` identity, `DOWN` `Rx(π)`, and the four horizontals `rotationXYZ(π/2, 0, zAngle)` — JOML's
/// `rotationXYZ(x, 0, z)` composes `Rx · Rz` — with `NORTH z = π`, `SOUTH z = 0`,
/// `WEST z = π/2`, `EAST z = -π/2`.
fn shulker_box_facing_rotation(facing: EntityAttachmentFace) -> Mat4 {
    use std::f32::consts::{FRAC_PI_2, PI};

    match facing {
        EntityAttachmentFace::Up => Mat4::IDENTITY,
        EntityAttachmentFace::Down => Mat4::from_rotation_x(PI),
        EntityAttachmentFace::North => Mat4::from_rotation_x(FRAC_PI_2) * Mat4::from_rotation_z(PI),
        EntityAttachmentFace::South => Mat4::from_rotation_x(FRAC_PI_2),
        EntityAttachmentFace::West => {
            Mat4::from_rotation_x(FRAC_PI_2) * Mat4::from_rotation_z(FRAC_PI_2)
        }
        EntityAttachmentFace::East => {
            Mat4::from_rotation_x(FRAC_PI_2) * Mat4::from_rotation_z(-FRAC_PI_2)
        }
    }
}

/// Vanilla `ShulkerBoxRenderer.createModelTransform(direction)`
/// (`ShulkerBoxRenderer.java:111-121`): `translation(0.5, 0.5, 0.5) · scale(0.9995) ·
/// rotate(FACING.getRotation()) · scale(1, -1, -1) · translate(0, -1, 0)` — center the box, shrink
/// it a hair against z-fighting with neighbouring full blocks, orient it onto its attached face,
/// then apply the Y-down entity-mesh flip (the box shares the shulker mob's Y-down-authored shell
/// mesh). `instance.position` is the box block's min corner; the facing rides the kind (a full
/// quaternion, not a yaw).
pub(in crate::entity_models) fn shulker_box_model_root_transform(
    instance: EntityModelInstance,
    facing: EntityAttachmentFace,
) -> Mat4 {
    entity_root_position_transform(instance)
        * Mat4::from_translation(Vec3::new(0.5, 0.5, 0.5))
        * Mat4::from_scale(Vec3::splat(0.9995))
        * shulker_box_facing_rotation(facing)
        * Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0))
}

/// A vanilla `PoseStack.rotateAround(rotation, x, y, z)`:
/// `translate(pivot) · rotation · translate(-pivot)`.
fn rotate_around(rotation: Mat4, pivot: Vec3) -> Mat4 {
    Mat4::from_translation(pivot) * rotation * Mat4::from_translation(-pivot)
}

/// Vanilla `DecoratedPotRenderer.submit` (`DecoratedPotRenderer.java:147-169`):
/// `mulPose(createModelTransformation(direction))` — a
/// `rotateAround(Axis.YP.rotationDegrees(180 - facing.toYRot()), 0.5, 0.5, 0.5)` yaw carried in
/// `body_rot` — then, while `0 <= wobbleProgress <= 1`, the wobble `rotateAround`s about the pot's
/// bottom centre `(0.5, 0, 0.5)`:
/// - `POSITIVE`: with `dt = progress · 2π`, `Rx(-1.5 · (cos(dt) + 0.5) · sin(dt/2) · 0.015625)`
///   then `Rz(sin(dt) · 0.015625)`;
/// - `NEGATIVE`: `Ry(sin(-progress · 3π) · 0.125 · (1 - progress))` (the `WOBBLE_AMPLITUDE`
///   0.125 twist with a linear decay).
///
/// The pot mesh is authored Y-up in block-local space (the parts' own π poses do the flipping),
/// so there is no entity `scale(-1, -1, 1)`. `instance.position` is the pot block's min corner.
pub(in crate::entity_models) fn decorated_pot_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    use std::f32::consts::PI;

    let mut transform = entity_root_position_transform(instance)
        * rotate_around(
            Mat4::from_rotation_y(instance.render_state.body_rot.to_radians()),
            Vec3::new(0.5, 0.5, 0.5),
        );
    if let Some(wobble) = instance.render_state.decorated_pot_wobble {
        if (0.0..=1.0).contains(&wobble.progress) {
            let pivot = Vec3::new(0.5, 0.0, 0.5);
            if wobble.positive {
                let delta_time = wobble.progress * (PI * 2.0);
                let tilt_x = -1.5 * (delta_time.cos() + 0.5) * (delta_time / 2.0).sin();
                transform *= rotate_around(Mat4::from_rotation_x(tilt_x * 0.015625), pivot);
                let tilt_z = delta_time.sin();
                transform *= rotate_around(Mat4::from_rotation_z(tilt_z * 0.015625), pivot);
            } else {
                let turn_angle = (-wobble.progress * 3.0 * PI).sin() * 0.125;
                let linear_decay = 1.0 - wobble.progress;
                transform *= rotate_around(Mat4::from_rotation_y(turn_angle * linear_decay), pivot);
            }
        }
    }
    transform
}

/// Vanilla `StandingSignRenderer.RENDER_SCALE` (`0.6666667F`), the plain
/// sign's body scale; hanging signs use `MODEL_RENDER_SCALE = 1.0F`.
pub(in crate::entity_models) const SIGN_RENDER_SCALE: f32 = 0.666_666_7;

/// Vanilla `BannerRenderer.SIZE` (`0.6666667F`) — the `MODEL_SCALE` magnitude
/// of `(SIZE, -SIZE, -SIZE)`.
pub(in crate::entity_models) const BANNER_RENDER_SCALE: f32 = 0.666_666_7;

/// Vanilla `BannerRenderer.modelTransformation(angle)`: `new
/// Transformation(MODEL_TRANSLATION, Axis.YP.rotationDegrees(-angle),
/// MODEL_SCALE, null)` with `MODEL_TRANSLATION = (0.5, 0, 0.5)` and
/// `MODEL_SCALE = (⅔, -⅔, -⅔)` — the same shape for the ground (`ROTATION`
/// segments) and wall (`FACING`) forms; only the mesh differs.
/// `instance.position` is the banner block's min corner and `body_rot`
/// carries the pre-negated `-angle` degrees, like the signs.
pub(in crate::entity_models) fn banner_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    entity_root_position_transform(instance)
        * Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5))
        * Mat4::from_rotation_y(instance.render_state.body_rot.to_radians())
        * Mat4::from_scale(Vec3::new(
            BANNER_RENDER_SCALE,
            -BANNER_RENDER_SCALE,
            -BANNER_RENDER_SCALE,
        ))
}

/// Vanilla `EnchantTableRenderer.submit` (`EnchantTableRenderer.java:61-73`):
/// `translate(0.5, 0.75, 0.5) · translate(0, 0.1 + sin(time·0.1)·0.01, 0) ·
/// Axis.YP.rotation(-yRot) · Axis.ZP.rotationDegrees(80)` — center the book
/// over the table, bob it, turn it to face the nearest player, then tip it
/// open. The book mesh is authored in the vanilla 1/16 pixel space (baked into
/// the cube emission), so there is no extra model scale. `instance.position` is
/// the table block's min corner, `book_float_y` carries the full
/// `0.1 + sin(time·0.1)·0.01` hover offset, and `body_rot` carries the lerped
/// book yaw in degrees (vanilla `yRot` is radians; the projection converts it).
pub(in crate::entity_models) fn enchanting_table_book_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    entity_root_position_transform(instance)
        * Mat4::from_translation(Vec3::new(0.5, 0.75, 0.5))
        * Mat4::from_translation(Vec3::new(0.0, instance.render_state.book_float_y, 0.0))
        * Mat4::from_rotation_y(-instance.render_state.body_rot.to_radians())
        * Mat4::from_rotation_z(80.0_f32.to_radians())
}

/// Vanilla `LecternRenderer.submit` (`LecternRenderer.java:46-50`):
/// `translate(0.5, 1.0625, 0.5) · Axis.YP.rotationDegrees(-yRot) ·
/// Axis.ZP.rotationDegrees(67.5) · translate(0, -0.125, 0)` — the lectern's
/// static open book on its slanted rest. Like the enchanting book there is no
/// extra model scale. `instance.position` is the lectern block's min corner and
/// `body_rot` carries the `FACING.getClockWise().toYRot()` yaw in degrees.
pub(in crate::entity_models) fn lectern_book_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    entity_root_position_transform(instance)
        * Mat4::from_translation(Vec3::new(0.5, 1.0625, 0.5))
        * Mat4::from_rotation_y(-instance.render_state.body_rot.to_radians())
        * Mat4::from_rotation_z(67.5_f32.to_radians())
        * Mat4::from_translation(Vec3::new(0.0, -0.125, 0.0))
}

/// Vanilla `StandingSignRenderer.baseTransformation` /
/// `HangingSignRenderer.baseTransformation`, with `instance.position` at the
/// sign block's min corner and `body_rot` carrying `-angle` degrees
/// (`Axis.YP.rotationDegrees(-angle)`):
/// - plain forms: `translate(0.5, 0.5, 0.5) · Ry(-angle)`, the WALL
///   attachment appending `translate(0, -0.3125, -0.4375)`;
/// - hanging forms: `translation(0.5, 0.9375, 0.5) · Ry(-angle) ·
///   translate(0, -0.3125, 0)`.
pub(crate) fn sign_base_transformation(
    position: [f32; 3],
    attachment: SignModelAttachment,
    body_rot_degrees: f32,
) -> Mat4 {
    let rotation = Mat4::from_rotation_y(body_rot_degrees.to_radians());
    let base = Mat4::from_translation(Vec3::from_array(position));
    if attachment.is_hanging() {
        base * Mat4::from_translation(Vec3::new(0.5, 0.9375, 0.5))
            * rotation
            * Mat4::from_translation(Vec3::new(0.0, -0.3125, 0.0))
    } else {
        let mut transform = base * Mat4::from_translation(Vec3::new(0.5, 0.5, 0.5)) * rotation;
        if matches!(attachment, SignModelAttachment::Wall) {
            transform *= Mat4::from_translation(Vec3::new(0.0, -0.3125, -0.4375));
        }
        transform
    }
}

/// Vanilla `StandingSignRenderer.bodyTransformation` /
/// `HangingSignRenderer.bodyTransformation`: the base transformation followed
/// by the `scale(s, -s, -s)` model flip (`s = RENDER_SCALE` for plain signs,
/// `1.0` for hanging signs) that maps the Y-down sign mesh into the block.
pub(in crate::entity_models) fn sign_model_root_transform(
    instance: EntityModelInstance,
    attachment: SignModelAttachment,
) -> Mat4 {
    let scale = if attachment.is_hanging() {
        1.0
    } else {
        SIGN_RENDER_SCALE
    };
    sign_base_transformation(
        instance.position,
        attachment,
        instance.render_state.body_rot,
    ) * Mat4::from_scale(Vec3::new(scale, -scale, -scale))
}

/// Vanilla `LlamaSpitRenderer.submit`: a plain `EntityRenderer` that lifts the spit slightly and
/// orients it along its flight with `translate(0, 0.15, 0)` then `Axis.YP.rotationDegrees(yRot - 90)`
/// then `Axis.ZP.rotationDegrees(xRot)` (no flip / scale / y-offset). `LlamaSpitModel` has no
/// `setupAnim`, so this is the complete transform; only the texture is colored-first. The yaw/pitch
/// are projected through `body_rot` / `head_pitch`.
pub(in crate::entity_models) fn llama_spit_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    entity_root_position_transform(instance)
        * Mat4::from_translation(Vec3::new(0.0, 0.15, 0.0))
        * Mat4::from_rotation_y((instance.render_state.body_rot - 90.0).to_radians())
        * Mat4::from_rotation_z(instance.render_state.head_pitch.to_radians())
}

/// Vanilla `WitherSkullRenderer.submit`: a plain `EntityRenderer` that applies `scale(-1, -1, 1)` then
/// submits the `SkullModel`, whose `setupAnim` turns the single `head` by the flight `yRot`/`xRot`
/// (`head.yRot = yRot`, `head.xRot = xRot`). Since the head sits at ZERO that facing folds into the
/// root: `scale(-1, -1, 1) · Ry(yRot) · Rx(xRot)`, the yaw/pitch projected through `body_rot` /
/// `head_pitch`. There is no `-1.501` y-offset or render scale (it is not a `LivingEntityRenderer`).
pub(in crate::entity_models) fn wither_skull_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    entity_root_position_transform(instance)
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_rotation_y(instance.render_state.body_rot.to_radians())
        * Mat4::from_rotation_x(instance.render_state.head_pitch.to_radians())
}

/// Vanilla `ShulkerBulletRenderer.submit` reduced to its non-animated parts: the constant
/// `translate(0, 0.15, 0)` lift and the `scale(-0.5, -0.5, 0.5)` (the flip + half size), followed by
/// the `ShulkerBulletModel.setupAnim` facing (`main.yRot = yRot`, `main.xRot = xRot`, here folded
/// into the root since the single part sits at ZERO). The `ageInTicks`-driven tumble (`Ry(sin(t·0.1)·
/// 180) · Rx(cos(t·0.1)·180) · Rz(sin(t·0.15)·360)`, applied before the scale) and the second
/// translucent 1.5× outer-shell pass are deferred. The yaw/pitch are projected through `body_rot` /
/// `head_pitch`.
pub(in crate::entity_models) fn shulker_bullet_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    // Vanilla `ShulkerBulletRenderer.submit`: after translating up 0.15, the bullet tumbles by an
    // `ageInTicks`-driven product `Ry(sin(t·0.1)·180°) · Rx(cos(t·0.1)·180°) · Rz(sin(t·0.15)·360°)`
    // (degrees → radians: `·π`, `·π`, `·2π`), then `scale(-0.5, -0.5, 0.5)`. `ShulkerBulletModel`
    // `setupAnim` then orients `main` by the bullet's yaw/pitch (innermost), reproduced through
    // `body_rot` / `head_pitch`.
    let t = instance.render_state.age_in_ticks;
    entity_root_position_transform(instance)
        * Mat4::from_translation(Vec3::new(0.0, 0.15, 0.0))
        * Mat4::from_rotation_y((t * 0.1).sin() * std::f32::consts::PI)
        * Mat4::from_rotation_x((t * 0.1).cos() * std::f32::consts::PI)
        * Mat4::from_rotation_z((t * 0.15).sin() * std::f32::consts::TAU)
        * Mat4::from_scale(Vec3::new(-0.5, -0.5, 0.5))
        * Mat4::from_rotation_y(instance.render_state.body_rot.to_radians())
        * Mat4::from_rotation_x(instance.render_state.head_pitch.to_radians())
}

/// Vanilla `WindChargeRenderer`: a plain `EntityRenderer` whose `submit` applies no extra transform
/// (no flip, scale, or y-offset), so the model renders at the entity position. The `bone` root sits
/// at ZERO; the `wind`/`wind_charge` counter-rotation and the scrolling `breezeWind` translucent
/// texture are deferred. Shared by the wind charge and breeze wind charge.
pub(in crate::entity_models) fn wind_charge_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    entity_root_position_transform(instance)
}

pub(in crate::entity_models) fn boat_model_root_transform(instance: EntityModelInstance) -> Mat4 {
    entity_root_position_transform(instance)
        * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
        * Mat4::from_rotation_y((180.0 - instance.render_state.body_rot).to_radians())
        * Mat4::from_rotation_x(boat_damage_roll_degrees(instance).to_radians())
        * boat_bubble_transform(instance)
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2)
}

pub(in crate::entity_models) fn boat_damage_roll_degrees(instance: EntityModelInstance) -> f32 {
    vehicle_damage_roll_degrees(
        instance.render_state.boat_hurt_time,
        instance.render_state.boat_damage_time,
        instance.render_state.boat_hurt_dir,
    )
}

fn vehicle_damage_roll_degrees(hurt: f32, damage_time: f32, hurt_dir: i32) -> f32 {
    if hurt > 0.0 {
        hurt.sin() * hurt * damage_time / 10.0 * hurt_dir as f32
    } else {
        0.0
    }
}

pub(in crate::entity_models) fn boat_bubble_transform(instance: EntityModelInstance) -> Mat4 {
    let angle = instance.render_state.boat_bubble_angle;
    if instance.render_state.boat_underwater || angle == 0.0 {
        Mat4::IDENTITY
    } else {
        Mat4::from_quat(glam::Quat::from_axis_angle(
            Vec3::new(1.0, 0.0, 1.0).normalize(),
            angle.to_radians(),
        ))
    }
}

/// Vanilla `AbstractMinecartRenderer.submit` root transform: the renderer applies deterministic
/// per-id hover jitter, then either `newRender` (`Ry(yRot)`, `Rz(-xRot)`,
/// `translate(0, 0.375, 0)`) for tracked new-behavior steps or the old-render no-rail fallback
/// (`translate(0, 0.375, 0)`, `Ry(180 - yRot)`, `Rz(-xRot)`). It then applies the hurt roll and the
/// final model flip. Old-render rail state additionally prepends the vanilla
/// `posOnRail - entity` translation and derives yaw/pitch from `backPos - frontPos`.
/// World projection already folds exact weighted `renderPos` interpolation into the
/// instance position/rotation for new-render carts.
pub(in crate::entity_models) fn minecart_model_root_transform(
    instance: EntityModelInstance,
) -> Mat4 {
    minecart_pre_model_transform(instance) * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

fn minecart_pre_model_transform(instance: EntityModelInstance) -> Mat4 {
    let jitter = Vec3::from_array(minecart_render_jitter_offset(instance.entity_id));
    let renderer_transform = if instance.render_state.minecart_new_render {
        Mat4::from_rotation_y(instance.render_state.body_rot.to_radians())
            * Mat4::from_rotation_z((-instance.render_state.head_pitch).to_radians())
            * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
    } else {
        minecart_old_render_transform(instance)
    };
    entity_root_position_transform(instance)
        * Mat4::from_translation(jitter)
        * renderer_transform
        * Mat4::from_rotation_x(minecart_damage_roll_degrees(instance).to_radians())
}

fn minecart_old_render_transform(instance: EntityModelInstance) -> Mat4 {
    let mut rotation = instance.render_state.body_rot;
    let mut x_rot = instance.render_state.head_pitch;
    let rail_transform = match (
        instance.render_state.minecart_pos_on_rail,
        instance.render_state.minecart_front_pos,
        instance.render_state.minecart_back_pos,
    ) {
        (Some(pos_on_rail), Some(front_pos), Some(back_pos)) => {
            let entity_pos = Vec3::from_array(instance.position);
            let pos_on_rail = Vec3::from_array(pos_on_rail);
            let front_pos = Vec3::from_array(front_pos);
            let back_pos = Vec3::from_array(back_pos);
            let direction = back_pos - front_pos;
            if direction.length() != 0.0 {
                let direction = direction.normalize();
                rotation = direction.z.atan2(direction.x).to_degrees();
                x_rot = direction.y.atan() * 73.0;
            }
            Mat4::from_translation(Vec3::new(
                pos_on_rail.x - entity_pos.x,
                (front_pos.y + back_pos.y) * 0.5 - entity_pos.y,
                pos_on_rail.z - entity_pos.z,
            ))
        }
        _ => Mat4::IDENTITY,
    };
    rail_transform
        * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
        * Mat4::from_rotation_y((180.0 - rotation).to_radians())
        * Mat4::from_rotation_z((-x_rot).to_radians())
}

/// Vanilla `AbstractMinecartRenderer.submit` display block transform: after jitter, old/new render,
/// and hurt roll, but before the body model's final `scale(-1, -1, 1)`, contents scale by `0.75`,
/// translate by `(-0.5, (displayOffset - 8) / 16, 0.5)`, then rotate Y by `90°`.
pub(in crate::entity_models) fn minecart_display_block_content_transform(
    instance: EntityModelInstance,
    display_offset: i32,
) -> Mat4 {
    minecart_pre_model_transform(instance)
        * Mat4::from_scale(Vec3::splat(0.75))
        * Mat4::from_translation(Vec3::new(-0.5, (display_offset as f32 - 8.0) / 16.0, 0.5))
        * Mat4::from_rotation_y(90.0_f32.to_radians())
}

/// Vanilla `AbstractMinecartRenderer.submit` damage roll:
/// `sin(hurtTime) * hurtTime * damageTime / 10 * hurtDir`, applied after the old/new render
/// transform and before display-block contents and the final model flip.
pub(in crate::entity_models) fn minecart_damage_roll_degrees(instance: EntityModelInstance) -> f32 {
    vehicle_damage_roll_degrees(
        instance.render_state.minecart_hurt_time,
        instance.render_state.minecart_damage_time,
        instance.render_state.minecart_hurt_dir,
    )
}

/// Vanilla `AbstractMinecartRenderer.extractRenderState` offsetSeed and submit-time jitter:
/// `seed = id * 493286711L; offsetSeed = seed * seed * 4392167121L + seed * 98761L`, then each axis
/// samples three shifted bits into a `[-0.00175, 0.00175]` world-unit nudge.
pub(in crate::entity_models) fn minecart_render_jitter_offset(entity_id: i32) -> [f32; 3] {
    let seed = i64::from(entity_id).wrapping_mul(493_286_711);
    let offset_seed = seed
        .wrapping_mul(seed)
        .wrapping_mul(4_392_167_121_i64)
        .wrapping_add(seed.wrapping_mul(98_761));

    let component = |shift: u32| ((((offset_seed >> shift) & 7) as f32 + 0.5) / 8.0 - 0.5) * 0.004;
    [component(16), component(20), component(24)]
}

pub(in crate::entity_models) fn slime_model_root_transform(
    instance: EntityModelInstance,
    size: i32,
) -> Mat4 {
    // Vanilla `SlimeRenderer.scale`: a `0.999` shrink + `0.001` lift to avoid
    // z-fighting between the two slime shells, then the squish stretch.
    living_entity_model_root_transform_with_renderer_transform(
        instance,
        Mat4::from_scale(Vec3::splat(0.999))
            * Mat4::from_translation(Vec3::new(0.0, 0.001, 0.0))
            * slime_squish_scale(instance.render_state.slime_squish, size),
    )
}

pub(in crate::entity_models) fn magma_cube_model_root_transform(
    instance: EntityModelInstance,
    size: i32,
) -> Mat4 {
    // Vanilla `MagmaCubeRenderer.scale`: the same squish stretch as the slime, but
    // without the slime's `0.999` two-shell z-fight nudge.
    living_entity_model_root_transform_with_renderer_transform(
        instance,
        slime_squish_scale(instance.render_state.slime_squish, size),
    )
}

/// Vanilla `SlimeRenderer.scale` / `MagmaCubeRenderer.scale` body stretch:
/// `ss = squish / (size * 0.5 + 1)`, `w = 1 / (ss + 1)`, then
/// `scale(w * size, 1/w * size, w * size)` — the body widens as it flattens and
/// narrows as it stretches, conserving silhouette. At `squish == 0` this is `w == 1`,
/// i.e. the plain `size` cube, so a resting slime is byte-for-byte unchanged.
fn slime_squish_scale(squish: f32, size: i32) -> Mat4 {
    let size = size as f32;
    let ss = squish / (size * 0.5 + 1.0);
    let w = 1.0 / (ss + 1.0);
    Mat4::from_scale(Vec3::new(w * size, size / w, w * size))
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
    entity_model_root_transform(instance) * mesh_transformer_scale_transform(scale)
}

pub(in crate::entity_models) fn mesh_transformer_scale_transform(scale: f32) -> Mat4 {
    part_pose_transform(PartPose {
        offset: [
            0.0,
            MESH_TRANSFORMER_ROOT_Y_OFFSET_PIXELS * (1.0 - scale),
            0.0,
        ],
        rotation: [0.0, 0.0, 0.0],
    }) * Mat4::from_scale(Vec3::splat(scale))
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
    entity_root_position_transform(instance)
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
    entity_root_position_transform(instance)
        * entity_pre_scale_translation(instance)
        * Mat4::from_scale(Vec3::splat(instance.render_state.scale))
        * Mat4::from_translation(Vec3::new(0.0, bob, 0.0))
        * entity_setup_rotations_transform(instance)
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

const SQUID_BABY_SCALE: f32 = 0.5;

/// Vanilla `SquidRenderer.setupRotations` fully overrides
/// `LivingEntityRenderer.setupRotations`: `translate(0, isBaby ? 0.25 : 0.5, 0)`, the
/// standard `Axis.YP.rotationDegrees(180 - bodyRot)` body yaw, the swim body tilt
/// (`Axis.XP.rotationDegrees(xBodyRot)` then `Axis.YP.rotationDegrees(zBodyRot)`), and
/// `translate(0, isBaby ? -0.6 : -1.2, 0)`. Because it overrides the base method, a
/// squid never runs the death/auto-spin/sleeping/upside-down chain, so it never tips
/// over. The swim body tilt (`xBodyRot` the movement pitch, `zBodyRot` the swim roll,
/// both in degrees, lerped into the render state) is `0` at rest — the orientation of a
/// floating squid. The baby uses the `SquidModel.BABY_TRANSFORMER`
/// (`MeshTransformer.scaling(0.5)`) body layer, composed innermost like the other
/// mesh-transformer-scaled models.
pub(in crate::entity_models) fn squid_model_root_transform(
    instance: EntityModelInstance,
    baby: bool,
) -> Mat4 {
    let (up, down) = if baby { (0.25, -0.6) } else { (0.5, -1.2) };
    let mut transform = entity_root_position_transform(instance)
        * Mat4::from_scale(Vec3::splat(instance.render_state.scale))
        * Mat4::from_translation(Vec3::new(0.0, up, 0.0))
        * Mat4::from_rotation_y((180.0 - instance.render_state.body_rot).to_radians())
        * Mat4::from_rotation_x(instance.render_state.squid_x_body_rot.to_radians())
        * Mat4::from_rotation_y(instance.render_state.squid_z_body_rot.to_radians())
        * Mat4::from_translation(Vec3::new(0.0, down, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0));
    if baby {
        transform *= part_pose_transform(PartPose {
            offset: [
                0.0,
                MESH_TRANSFORMER_ROOT_Y_OFFSET_PIXELS * (1.0 - SQUID_BABY_SCALE),
                0.0,
            ],
            rotation: [0.0, 0.0, 0.0],
        }) * Mat4::from_scale(Vec3::splat(SQUID_BABY_SCALE));
    }
    transform
}

/// Vanilla `CodRenderer.setupRotations` extends the standard `MobRenderer`
/// `super.setupRotations` (body yaw + death/auto-spin/upside-down chain) with a swim
/// wiggle `Axis.YP.rotationDegrees(4.3 * sin(0.6 * ageInTicks))`, then — when out of
/// water — a beached flop `translate(0.1, 0.1, -0.1)` + `Axis.ZP.rotationDegrees(90)`.
pub(in crate::entity_models) fn cod_model_root_transform(
    instance: EntityModelInstance,
    in_water: bool,
) -> Mat4 {
    let wiggle = 4.3 * (0.6 * instance.render_state.age_in_ticks).sin();
    let mut transform = entity_root_position_transform(instance)
        * entity_pre_scale_translation(instance)
        * Mat4::from_scale(Vec3::splat(instance.render_state.scale))
        * entity_setup_rotations_transform(instance)
        * Mat4::from_rotation_y(wiggle.to_radians());
    if !in_water {
        transform *= Mat4::from_translation(Vec3::new(0.1, 0.1, -0.1))
            * Mat4::from_rotation_z(90.0_f32.to_radians());
    }
    transform
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

/// Vanilla `SalmonRenderer.setupRotations` extends `super.setupRotations` (body yaw +
/// death chain) with a swim wiggle `Axis.YP.rotationDegrees(amplitude * 4.3 *
/// sin(angle * 0.6 * ageInTicks))`, then — out of water — a beached flop
/// `translate(0.2, 0.1, 0.0)` + `Axis.ZP.rotationDegrees(90)`. `amplitude`/`angle` are
/// `(1.0, 1.0)` in water and `(1.3, 1.7)` out. `size` selects the small/medium/large
/// `MeshTransformer` scale, composed innermost like the other scaled models (medium is
/// the unscaled base).
pub(in crate::entity_models) fn salmon_model_root_transform(
    instance: EntityModelInstance,
    in_water: bool,
    size: SalmonModelSize,
) -> Mat4 {
    let (amplitude, angle) = if in_water { (1.0, 1.0) } else { (1.3, 1.7) };
    let wiggle = amplitude * 4.3 * (angle * 0.6 * instance.render_state.age_in_ticks).sin();
    let mut transform = entity_root_position_transform(instance)
        * entity_pre_scale_translation(instance)
        * Mat4::from_scale(Vec3::splat(instance.render_state.scale))
        * entity_setup_rotations_transform(instance)
        * Mat4::from_rotation_y(wiggle.to_radians());
    if !in_water {
        transform *= Mat4::from_translation(Vec3::new(0.2, 0.1, 0.0))
            * Mat4::from_rotation_z(90.0_f32.to_radians());
    }
    transform *= Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0));
    let model_scale = size.scale();
    if model_scale != 1.0 {
        transform *= part_pose_transform(PartPose {
            offset: [
                0.0,
                MESH_TRANSFORMER_ROOT_Y_OFFSET_PIXELS * (1.0 - model_scale),
                0.0,
            ],
            rotation: [0.0, 0.0, 0.0],
        }) * Mat4::from_scale(Vec3::splat(model_scale));
    }
    transform
}

/// Vanilla `TropicalFishRenderer.setupRotations` extends `super.setupRotations` (body yaw
/// + death chain) with a swim wiggle `Axis.YP.rotationDegrees(4.3 * sin(0.6 *
/// ageInTicks))` (no amplitude multiplier, unlike salmon), then — out of water — the
/// beached flop `translate(0.2, 0.1, 0.0)` + `Axis.ZP.rotationDegrees(90)`. The
/// small/large body shapes share this transform (the geometry differs, not the rotation).
pub(in crate::entity_models) fn tropical_fish_model_root_transform(
    instance: EntityModelInstance,
    in_water: bool,
) -> Mat4 {
    let wiggle = 4.3 * (0.6 * instance.render_state.age_in_ticks).sin();
    let mut transform = entity_root_position_transform(instance)
        * entity_pre_scale_translation(instance)
        * Mat4::from_scale(Vec3::splat(instance.render_state.scale))
        * entity_setup_rotations_transform(instance)
        * Mat4::from_rotation_y(wiggle.to_radians());
    if !in_water {
        transform *= Mat4::from_translation(Vec3::new(0.2, 0.1, 0.0))
            * Mat4::from_rotation_z(90.0_f32.to_radians());
    }
    transform
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -VANILLA_MODEL_ROOT_Y_OFFSET, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_models::SpawnerDisplayRenderState;

    #[test]
    fn spawner_display_position_transform_wraps_entity_root() {
        let instance = EntityModelInstance::new(
            1,
            EntityModelKind::Zombie { baby: false },
            [2.0, 3.0, 4.0],
            0.0,
        )
        .with_spawner_display(Some(SpawnerDisplayRenderState {
            spin_degrees: 0.0,
            scale: 0.5,
        }));

        let transform = entity_root_position_transform(instance);

        assert_vec3_approx(
            transform.transform_point3(Vec3::ZERO),
            Vec3::new(2.5, 3.2, 4.5),
        );
        assert_vec3_approx(
            transform.transform_point3(Vec3::Y),
            Vec3::new(2.5, 3.633_012_8, 4.25),
        );
    }

    fn assert_vec3_approx(actual: Vec3, expected: Vec3) {
        let delta = actual - expected;
        assert!(
            delta.length() < 0.000_01,
            "actual={actual:?} expected={expected:?} delta={delta:?}"
        );
    }
}
