use super::super::catalog::ArmorStandModelPose;
use super::super::geometry::EntityModelMesh;
use super::super::instances::EntityModelInstance;
use super::super::model::EntityModel;
use super::super::model_layers::ArmorStandModel;
use super::transforms::entity_model_root_transform;

pub(super) fn emit_armor_stand_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    small: bool,
    show_arms: bool,
    show_base_plate: bool,
    pose: ArmorStandModelPose,
) {
    // The unified `ArmorStandModel` tree drives both render paths; `new` selects the small / full layer
    // and `setup_anim` poses each part from the synced pose (degrees), hides the arms / base plate by
    // visibility, and yaws the base plate by `-bodyRot`.
    ArmorStandModel::new(small, show_arms, show_base_plate, pose).prepare_and_render(
        mesh,
        &instance,
        entity_model_root_transform(instance),
    );
}
