use super::super::catalog::{
    CamelModelFamily, DonkeyModelFamily, LlamaModelFamily, LlamaVariant, UndeadHorseModelFamily,
};
use super::super::geometry::{
    emit_model_parts, emit_model_parts_with_color, EntityModelMesh, ModelPartDesc,
};
use super::super::instances::EntityModelInstance;
use super::super::model_layers::{
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
    emit_model_parts_with_color(
        mesh,
        parts,
        entity_model_root_transform(instance),
        llama_model_color(family, variant),
    );
}
