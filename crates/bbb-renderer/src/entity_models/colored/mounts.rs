use super::super::catalog::{
    CamelModelFamily, DonkeyModelFamily, LlamaModelFamily, LlamaVariant, UndeadHorseModelFamily,
};
use super::super::geometry::{
    emit_model_parts, emit_model_parts_with_color, EntityModelMesh, ModelPartDesc,
};
use super::super::instances::EntityModelInstance;
use super::super::model_layers::{
    head_look_at_rest, head_look_pose, limb_swing_at_rest, quadruped_leg_swing_pose,
    ADULT_CAMEL_PARTS, ADULT_DONKEY_PARTS, ADULT_DONKEY_PARTS_WITH_CHEST, ADULT_HORSE_PARTS,
    ADULT_LLAMA_PARTS, ADULT_LLAMA_PARTS_WITH_CHEST, BABY_CAMEL_PARTS, BABY_DONKEY_PARTS,
    BABY_HORSE_PARTS, BABY_LLAMA_PARTS,
};
use super::selection::{
    camel_model_color, donkey_model_color, donkey_model_scale, llama_model_color,
    undead_horse_model_color,
};
use super::transforms::{
    entity_model_root_transform, mesh_transformer_scaled_model_root_transform, HORSE_SCALE,
};

pub(super) fn emit_horse_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    baby: bool,
) {
    emit_model_parts(
        mesh,
        if baby {
            &BABY_HORSE_PARTS
        } else {
            &ADULT_HORSE_PARTS
        },
        if baby {
            entity_model_root_transform(instance)
        } else {
            mesh_transformer_scaled_model_root_transform(instance, HORSE_SCALE)
        },
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
    emit_model_parts_with_color(mesh, parts, transform, donkey_model_color(family));
}

pub(super) fn emit_undead_horse_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    family: UndeadHorseModelFamily,
    baby: bool,
) {
    emit_model_parts_with_color(
        mesh,
        if baby {
            &BABY_HORSE_PARTS
        } else {
            &ADULT_HORSE_PARTS
        },
        entity_model_root_transform(instance),
        undead_horse_model_color(family),
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
