use super::super::catalog::{DonkeyModelFamily, UndeadHorseModelFamily};
use glam::Mat4;

use super::super::geometry::{
    emit_model_cube, emit_model_cube_with_color, emit_model_parts, emit_model_parts_with_color,
    part_pose_transform, EntityModelMesh, ModelPartDesc,
};
use super::super::instances::EntityModelInstance;
use super::super::model_layers::{
    baby_donkey_head_pose, equine_head_look_pose, equine_leg_swing_pose, equine_tail_pose,
    head_look_at_rest, limb_swing_at_rest, ADULT_DONKEY_PARTS, ADULT_DONKEY_PARTS_WITH_CHEST,
    ADULT_HORSE_PARTS, BABY_DONKEY_PARTS, BABY_HORSE_PARTS,
};

/// The four leg part indices in the adult equine body layers: body and neck at `0`/`1`,
/// then left-hind, right-hind, left-front, right-front.
const ADULT_EQUINE_LEG_PART_INDICES: [usize; 4] = [2, 3, 4, 5];

/// `head_parts` (neck) index in the adult equine body layers: the body is at `0`, the
/// neck at `1`.
const ADULT_EQUINE_HEAD_PART_INDEX: usize = 1;

/// The four leg part indices in the baby horse body layer. `BabyHorseModel.createBabyLayer`
/// re-parents the parts so the body is at `0`, the legs at `[1, 2, 3, 4]`, and the
/// neck/head last at `5`. (The baby donkey/mule layer nests its legs under the body and
/// is handled separately.)
const BABY_HORSE_LEG_PART_INDICES: [usize; 4] = [1, 2, 3, 4];

/// `head_parts` (neck) index in the baby horse body layer: `BabyHorseModel.createBabyLayer`
/// lists the body and four legs first, so the neck/head is last at `5`.
const BABY_HORSE_HEAD_PART_INDEX: usize = 5;

/// The baby donkey/mule layer nests legs, head_parts, and tail under the body. These child
/// indices mirror `BabyDonkeyModel.createBabyLayer`: tail at `0`, legs at `[1..=4]`, and
/// `head_parts` at `5`.
const BABY_DONKEY_BODY_CHILD_LEG_INDICES: [usize; 4] = [1, 2, 3, 4];
const BABY_DONKEY_BODY_CHILD_HEAD_PARTS_INDEX: usize = 5;

/// The body part index in every equine body layer (adult/baby horse, donkey/mule with or
/// without chest): the body is always listed first.
const EQUINE_BODY_PART_INDEX: usize = 0;

/// The tail's child index under the body part in every equine layout: the tail is the
/// body's first child (chests, when present, follow it).
const EQUINE_TAIL_CHILD_INDEX: usize = 0;

/// `AbstractEquineModel.getTailXRotOffset()` for the adult horse/donkey/mule (`0`), the
/// baby horse (`−π/2`), and the baby donkey/mule (`−π/4`).
const ADULT_EQUINE_TAIL_X_ROT_OFFSET: f32 = 0.0;
const BABY_HORSE_TAIL_X_ROT_OFFSET: f32 = -std::f32::consts::FRAC_PI_2;
const BABY_DONKEY_TAIL_X_ROT_OFFSET: f32 = -std::f32::consts::FRAC_PI_4;

/// `LivingEntity.getAgeScale()`: `1.0` for adults, `0.5` for babies. The equine tail's
/// walk translation scales by this.
const ADULT_AGE_SCALE: f32 = 1.0;
const BABY_AGE_SCALE: f32 = 0.5;
use super::selection::{donkey_model_color, donkey_model_scale, undead_horse_model_color};
use super::transforms::{
    entity_model_root_transform, mesh_transformer_scaled_model_root_transform, HORSE_SCALE,
};

/// Emits an equine body layer, applying the vanilla `AbstractEquineModel.setupAnim`
/// default-branch poses: the walking leg swing ([`equine_leg_swing_pose`]) on the four
/// parts at `leg_indices`, the head look/bob ([`equine_head_look_pose`]) on the
/// `head_parts` (neck) at `head_parts_index`, and the tail walk lift / optional wag
/// ([`equine_tail_pose`], with `tail_x_rot_offset` = `getTailXRotOffset()` and
/// `age_scale` = `getAgeScale()`) on the body's tail child. `color` picks the uniform-color
/// path (donkey/mule/undead horse) or the per-cube colored path (horse). The static parts
/// are reused unchanged only when the gait, head look, and tail are all at rest; otherwise
/// the body subtree is hand-emitted so the `&'static` tail child can be re-posed.
#[allow(clippy::too_many_arguments)]
fn emit_equine_posed(
    mesh: &mut EntityModelMesh,
    parts: &[ModelPartDesc],
    leg_indices: [usize; 4],
    head_parts_index: usize,
    tail_x_rot_offset: f32,
    age_scale: f32,
    transform: Mat4,
    color: Option<[f32; 4]>,
    instance: EntityModelInstance,
) {
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let in_water = instance.render_state.in_water;
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let legs_resting = limb_swing_at_rest(limb_swing_amount);

    // The tail is the body's first child. Vanilla `setupAnim` rewrites its pose every
    // frame; for a baby horse the rest angle is even overridden, so the tail must be
    // re-posed whenever the result differs from the static pose.
    let tail_rest = parts[EQUINE_BODY_PART_INDEX].children[EQUINE_TAIL_CHILD_INDEX].pose;
    let posed_tail = equine_tail_pose(
        tail_rest,
        tail_x_rot_offset,
        limb_swing_amount,
        age_scale,
        instance.render_state.equine_animate_tail,
        instance.render_state.age_in_ticks,
    );
    let tail_resting = posed_tail == tail_rest;

    if legs_resting && head_look_at_rest(head_yaw, head_pitch) && tail_resting {
        match color {
            Some(color) => emit_model_parts_with_color(mesh, parts, transform, color),
            None => emit_model_parts(mesh, parts, transform),
        }
        return;
    }

    let mut posed = parts.to_vec();
    if !legs_resting {
        for index in leg_indices {
            posed[index].pose =
                equine_leg_swing_pose(posed[index].pose, limb_swing, limb_swing_amount, in_water);
        }
    }
    posed[head_parts_index].pose = equine_head_look_pose(
        posed[head_parts_index].pose,
        head_yaw,
        head_pitch,
        limb_swing,
        limb_swing_amount,
    );

    // Hand-emit the body subtree so the tail (a `&'static` child) can take the swung pose:
    // the body's own cubes at the body transform, then its children with the tail re-posed.
    // The remaining parts (neck + legs) keep their depth-first order via the `[1..]` slice.
    let body = &posed[EQUINE_BODY_PART_INDEX];
    let body_transform = transform * part_pose_transform(body.pose);
    let mut body_children = body.children.to_vec();
    body_children[EQUINE_TAIL_CHILD_INDEX].pose = posed_tail;
    match color {
        Some(color) => {
            for &cube in body.cubes {
                emit_model_cube_with_color(mesh, body_transform, cube, color);
            }
            emit_model_parts_with_color(mesh, &body_children, body_transform, color);
            emit_model_parts_with_color(
                mesh,
                &posed[EQUINE_BODY_PART_INDEX + 1..],
                transform,
                color,
            );
        }
        None => {
            for &cube in body.cubes {
                emit_model_cube(mesh, body_transform, cube);
            }
            emit_model_parts(mesh, &body_children, body_transform);
            emit_model_parts(mesh, &posed[EQUINE_BODY_PART_INDEX + 1..], transform);
        }
    }
}

fn emit_baby_donkey_posed(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    color: [f32; 4],
    instance: EntityModelInstance,
) {
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let in_water = instance.render_state.in_water;
    let head_yaw = instance.render_state.head_yaw;
    let legs_resting = limb_swing_at_rest(limb_swing_amount);

    let body = &BABY_DONKEY_PARTS[EQUINE_BODY_PART_INDEX];
    let mut body_children = body.children.to_vec();
    if !legs_resting {
        for index in BABY_DONKEY_BODY_CHILD_LEG_INDICES {
            body_children[index].pose = equine_leg_swing_pose(
                body_children[index].pose,
                limb_swing,
                limb_swing_amount,
                in_water,
            );
        }
    }
    body_children[BABY_DONKEY_BODY_CHILD_HEAD_PARTS_INDEX].pose = baby_donkey_head_pose(
        body_children[BABY_DONKEY_BODY_CHILD_HEAD_PARTS_INDEX].pose,
        head_yaw,
    );
    body_children[EQUINE_TAIL_CHILD_INDEX].pose = equine_tail_pose(
        body_children[EQUINE_TAIL_CHILD_INDEX].pose,
        BABY_DONKEY_TAIL_X_ROT_OFFSET,
        limb_swing_amount,
        BABY_AGE_SCALE,
        instance.render_state.equine_animate_tail,
        instance.render_state.age_in_ticks,
    );

    let body_transform = transform * part_pose_transform(body.pose);
    for &cube in body.cubes {
        emit_model_cube_with_color(mesh, body_transform, cube, color);
    }
    emit_model_parts_with_color(mesh, &body_children, body_transform, color);
}

pub(in crate::entity_models) fn emit_horse_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    baby: bool,
) {
    // Vanilla `HorseModel extends AbstractEquineModel`: the four legs swing with the
    // equine gait (front amplitude 0.8, hind 0.5), the neck (`head_parts`) takes the head
    // look/bob (yaw clamped to ±20°, pitch onto the π/6 tilt, plus a walk bob), and the
    // tail lifts with the gait (baby horse `getTailXRotOffset = −π/2`, `ageScale = 0.5`)
    // and wags on yRot when `animateTail` is projected. The ridden/eat/stand poses are deferred.
    emit_equine_posed(
        mesh,
        if baby {
            &BABY_HORSE_PARTS
        } else {
            &ADULT_HORSE_PARTS
        },
        if baby {
            BABY_HORSE_LEG_PART_INDICES
        } else {
            ADULT_EQUINE_LEG_PART_INDICES
        },
        if baby {
            BABY_HORSE_HEAD_PART_INDEX
        } else {
            ADULT_EQUINE_HEAD_PART_INDEX
        },
        if baby {
            BABY_HORSE_TAIL_X_ROT_OFFSET
        } else {
            ADULT_EQUINE_TAIL_X_ROT_OFFSET
        },
        if baby {
            BABY_AGE_SCALE
        } else {
            ADULT_AGE_SCALE
        },
        if baby {
            entity_model_root_transform(instance)
        } else {
            mesh_transformer_scaled_model_root_transform(instance, HORSE_SCALE)
        },
        None,
        instance,
    );
}

pub(in crate::entity_models) fn emit_donkey_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: DonkeyModelFamily,
    baby: bool,
    has_chest: bool,
) {
    let parts: &[ModelPartDesc] = if baby {
        &BABY_DONKEY_PARTS
    } else if has_chest {
        &ADULT_DONKEY_PARTS_WITH_CHEST
    } else {
        &ADULT_DONKEY_PARTS
    };
    let transform = if baby {
        entity_model_root_transform(instance)
    } else {
        mesh_transformer_scaled_model_root_transform(instance, donkey_model_scale(family))
    };
    let color = donkey_model_color(family);
    // The adult donkey/mule uses the clean `AbstractEquineModel.setupAnim` (it only adds
    // chest visibility), so it takes the equine leg swing (legs at [2, 3, 4, 5]), the head
    // look/bob (neck at 1), and the tail walk lift. The baby donkey/mule re-parents its
    // legs/head/tail under the body (`BabyDonkeyModel.createBabyLayer`) and overrides
    // `setupAnim` by forcing `xRot = -30°`, so it needs a nested-body pose path: legs swing
    // as equine legs, yaw is still clamped on `head_parts`, xRot is reset to 0 in the
    // default branch, and the tail parent uses `getTailXRotOffset = -π/4`.
    if baby {
        emit_baby_donkey_posed(mesh, transform, color, instance);
    } else {
        emit_equine_posed(
            mesh,
            parts,
            ADULT_EQUINE_LEG_PART_INDICES,
            ADULT_EQUINE_HEAD_PART_INDEX,
            ADULT_EQUINE_TAIL_X_ROT_OFFSET,
            ADULT_AGE_SCALE,
            transform,
            Some(color),
            instance,
        );
    }
}

pub(in crate::entity_models) fn emit_undead_horse_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: UndeadHorseModelFamily,
    baby: bool,
) {
    // Skeleton and zombie horses reuse `HorseModel`, so they take the same equine leg
    // swing, head look/bob, and tail walk lift; only the tint differs.
    emit_equine_posed(
        mesh,
        if baby {
            &BABY_HORSE_PARTS
        } else {
            &ADULT_HORSE_PARTS
        },
        if baby {
            BABY_HORSE_LEG_PART_INDICES
        } else {
            ADULT_EQUINE_LEG_PART_INDICES
        },
        if baby {
            BABY_HORSE_HEAD_PART_INDEX
        } else {
            ADULT_EQUINE_HEAD_PART_INDEX
        },
        if baby {
            BABY_HORSE_TAIL_X_ROT_OFFSET
        } else {
            ADULT_EQUINE_TAIL_X_ROT_OFFSET
        },
        if baby {
            BABY_AGE_SCALE
        } else {
            ADULT_AGE_SCALE
        },
        entity_model_root_transform(instance),
        Some(undead_horse_model_color(family)),
        instance,
    );
}
