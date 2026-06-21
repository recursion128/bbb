use super::super::catalog::{
    CamelModelFamily, DonkeyModelFamily, LlamaModelFamily, LlamaVariant, UndeadHorseModelFamily,
};
use glam::Mat4;

use super::super::geometry::{
    emit_model_parts, emit_model_parts_with_color, EntityModelMesh, ModelPartDesc,
};
use super::super::instances::EntityModelInstance;
use super::super::model_layers::{
    equine_head_look_pose, equine_leg_swing_pose, head_look_at_rest, head_look_pose,
    limb_swing_at_rest, quadruped_leg_swing_pose, ADULT_CAMEL_PARTS, ADULT_DONKEY_PARTS,
    ADULT_DONKEY_PARTS_WITH_CHEST, ADULT_HORSE_PARTS, ADULT_LLAMA_PARTS,
    ADULT_LLAMA_PARTS_WITH_CHEST, BABY_CAMEL_PARTS, BABY_DONKEY_PARTS, BABY_HORSE_PARTS,
    BABY_LLAMA_PARTS,
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
use super::selection::{
    camel_model_color, donkey_model_color, donkey_model_scale, llama_model_color,
    undead_horse_model_color,
};
use super::transforms::{
    entity_model_root_transform, mesh_transformer_scaled_model_root_transform, HORSE_SCALE,
};

/// Emits an equine body layer, applying the vanilla `AbstractEquineModel.setupAnim`
/// default-branch poses: the walking leg swing ([`equine_leg_swing_pose`]) on the four
/// parts at `leg_indices` and the head look/bob ([`equine_head_look_pose`]) on the
/// `head_parts` (neck) at `head_parts_index`. `color` picks the uniform-color path
/// (donkey/mule/undead horse) or the per-cube colored path (horse). The static parts are
/// reused unchanged only when both the gait and the head look are at rest.
fn emit_equine_posed(
    mesh: &mut EntityModelMesh,
    parts: &[ModelPartDesc],
    leg_indices: [usize; 4],
    head_parts_index: usize,
    transform: Mat4,
    color: Option<[f32; 4]>,
    instance: EntityModelInstance,
) {
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let legs_resting = limb_swing_at_rest(limb_swing_amount);
    let posed_storage;
    let parts_to_emit: &[ModelPartDesc] = if legs_resting && head_look_at_rest(head_yaw, head_pitch)
    {
        parts
    } else {
        let mut posed = parts.to_vec();
        if !legs_resting {
            for index in leg_indices {
                posed[index].pose =
                    equine_leg_swing_pose(posed[index].pose, limb_swing, limb_swing_amount);
            }
        }
        posed[head_parts_index].pose = equine_head_look_pose(
            posed[head_parts_index].pose,
            head_yaw,
            head_pitch,
            limb_swing,
            limb_swing_amount,
        );
        posed_storage = posed;
        &posed_storage
    };
    match color {
        Some(color) => emit_model_parts_with_color(mesh, parts_to_emit, transform, color),
        None => emit_model_parts(mesh, parts_to_emit, transform),
    }
}

pub(super) fn emit_horse_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    baby: bool,
) {
    // Vanilla `HorseModel extends AbstractEquineModel`: the four legs swing with the
    // equine gait (front amplitude 0.8, hind 0.5) and the neck (`head_parts`) takes the
    // head look/bob (yaw clamped to ±20°, pitch onto the π/6 tilt, plus a walk bob). The
    // ridden/eat/stand poses and the tail animation are deferred.
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
            entity_model_root_transform(instance)
        } else {
            mesh_transformer_scaled_model_root_transform(instance, HORSE_SCALE)
        },
        None,
        instance,
    );
}

pub(super) fn emit_donkey_model(
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
    // chest visibility), so it takes the equine leg swing (legs at [2, 3, 4, 5]) and the
    // head look/bob (neck at 1). The baby donkey/mule overrides `setupAnim` (forcing
    // `xRot = -30°`) and re-parents its legs under the body
    // (`BabyDonkeyModel.createBabyLayer`), so its leg swing and head look are deferred.
    if baby {
        emit_model_parts_with_color(mesh, parts, transform, color);
    } else {
        emit_equine_posed(
            mesh,
            parts,
            ADULT_EQUINE_LEG_PART_INDICES,
            ADULT_EQUINE_HEAD_PART_INDEX,
            transform,
            Some(color),
            instance,
        );
    }
}

pub(super) fn emit_undead_horse_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: UndeadHorseModelFamily,
    baby: bool,
) {
    // Skeleton and zombie horses reuse `HorseModel`, so they take the same equine leg
    // swing and head look/bob; only the tint differs.
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
        entity_model_root_transform(instance),
        Some(undead_horse_model_color(family)),
        instance,
    );
}

pub(super) fn emit_camel_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: CamelModelFamily,
    baby: bool,
) {
    emit_model_parts_with_color(
        mesh,
        if family == CamelModelFamily::Camel && baby {
            &BABY_CAMEL_PARTS
        } else {
            &ADULT_CAMEL_PARTS
        },
        entity_model_root_transform(instance),
        camel_model_color(family),
    );
}

pub(super) fn emit_llama_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: LlamaModelFamily,
    variant: LlamaVariant,
    baby: bool,
    has_chest: bool,
) {
    let parts: &[ModelPartDesc] = if baby {
        &BABY_LLAMA_PARTS
    } else if has_chest {
        &ADULT_LLAMA_PARTS_WITH_CHEST
    } else {
        &ADULT_LLAMA_PARTS
    };
    let transform = entity_model_root_transform(instance);
    let color = llama_model_color(family, variant);
    let head_yaw = instance.render_state.head_yaw;
    let head_pitch = instance.render_state.head_pitch;
    let limb_swing = instance.render_state.walk_animation_pos;
    let limb_swing_amount = instance.render_state.walk_animation_speed;
    // Vanilla `LlamaModel.setupAnim`: head look (`head.xRot/yRot = pitch/yaw`) plus the
    // standard `QuadrupedModel` diagonal leg swing (`cos(pos * 0.6662 [+ π]) * 1.4 *
    // speed`, right-hind/left-front in phase). The chest visibility is already handled by
    // the part-array selection. The head is part 0 in every layout.
    if head_look_at_rest(head_yaw, head_pitch) && limb_swing_at_rest(limb_swing_amount) {
        emit_model_parts_with_color(mesh, parts, transform, color);
        return;
    }
    let mut posed = parts.to_vec();
    posed[0].pose = head_look_pose(posed[0].pose, head_yaw, head_pitch);
    for index in llama_leg_part_indices(baby, has_chest) {
        posed[index].pose =
            quadruped_leg_swing_pose(posed[index].pose, limb_swing, limb_swing_amount);
    }
    emit_model_parts_with_color(mesh, &posed, transform, color);
}

/// The four leg part indices in the llama body layers. The adult layer lists head and
/// body at `0`/`1` then the legs at `[2, 3, 4, 5]`; the chest layer inserts the two
/// chest parts at `2`/`3`, pushing the legs to `[4, 5, 6, 7]`; the baby layer (no
/// chest) lists the head at `0`, the legs at `[1, 2, 3, 4]`, and the body last.
/// [`quadruped_leg_swing_pose`] resolves each leg's phase from its offset.
fn llama_leg_part_indices(baby: bool, has_chest: bool) -> [usize; 4] {
    if baby {
        [1, 2, 3, 4]
    } else if has_chest {
        [4, 5, 6, 7]
    } else {
        [2, 3, 4, 5]
    }
}
