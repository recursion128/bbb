use glam::Mat4;

use super::super::catalog::ArmorStandModelPose;
use super::super::geometry::{emit_model_cubes_at_pose, EntityModelMesh, ModelPartDesc, PartPose};
use super::super::instances::EntityModelInstance;
use super::super::model_layers::{ARMOR_STAND_PARTS, SMALL_ARMOR_STAND_PARTS};
use super::transforms::entity_model_root_transform;

pub(super) fn emit_armor_stand_model(
    mesh: &mut EntityModelMesh,
    instance: EntityModelInstance,
    small: bool,
    show_arms: bool,
    show_base_plate: bool,
    pose: ArmorStandModelPose,
) {
    let parts = if small {
        &SMALL_ARMOR_STAND_PARTS
    } else {
        &ARMOR_STAND_PARTS
    };
    let transform = entity_model_root_transform(instance);
    emit_armor_stand_part(mesh, transform, &parts[0], degrees_to_radians3(pose.head));
    emit_armor_stand_part(mesh, transform, &parts[1], degrees_to_radians3(pose.body));
    if show_arms {
        emit_armor_stand_part(
            mesh,
            transform,
            &parts[2],
            degrees_to_radians3(pose.right_arm),
        );
        emit_armor_stand_part(
            mesh,
            transform,
            &parts[3],
            degrees_to_radians3(pose.left_arm),
        );
    }
    emit_armor_stand_part(
        mesh,
        transform,
        &parts[4],
        degrees_to_radians3(pose.right_leg),
    );
    emit_armor_stand_part(
        mesh,
        transform,
        &parts[5],
        degrees_to_radians3(pose.left_leg),
    );
    emit_armor_stand_part(mesh, transform, &parts[6], degrees_to_radians3(pose.body));
    emit_armor_stand_part(mesh, transform, &parts[7], degrees_to_radians3(pose.body));
    emit_armor_stand_part(mesh, transform, &parts[8], degrees_to_radians3(pose.body));
    if show_base_plate {
        emit_armor_stand_part(
            mesh,
            transform,
            &parts[9],
            [0.0, -instance.render_state.body_rot.to_radians(), 0.0],
        );
    }
}

fn emit_armor_stand_part(
    mesh: &mut EntityModelMesh,
    transform: Mat4,
    part: &ModelPartDesc,
    rotation: [f32; 3],
) {
    emit_model_cubes_at_pose(
        mesh,
        transform,
        PartPose {
            offset: part.pose.offset,
            rotation,
        },
        part.cubes,
    );
}

fn degrees_to_radians3(rotation: [f32; 3]) -> [f32; 3] {
    [
        rotation[0].to_radians(),
        rotation[1].to_radians(),
        rotation[2].to_radians(),
    ]
}
