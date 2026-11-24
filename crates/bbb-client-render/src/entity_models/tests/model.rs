use glam::{Mat4, Vec3};

use super::assert_close3;

use crate::entity_models::geometry::{EntityModelTexturedMesh, PartPose};
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{cube, ModelCube, G0};
use crate::entity_models::EntityModelUvRect;

const PART_POSE_ZERO: PartPose = PartPose {
    offset: [0.0; 3],
    rotation: [0.0; 3],
};

static UNIT_CUBE: [ModelCube; 1] = [cube([0.0; 3], [1.0; 3], G0, [0.0, 0.0], false, 63)];

fn part(
    pose: PartPose,
    cubes: &'static [ModelCube],
    children: Vec<(&'static str, ModelPart)>,
) -> ModelPart {
    ModelPart::new(pose, cubes, children, [16.0, 16.0])
}

#[test]
fn model_part_reset_pose_restores_bind_and_visibility() {
    // Vanilla `ModelPart.resetPose` restores the captured bind pose (and visibility) over the whole
    // subtree, so each frame's `setup_anim` starts clean. A part captures its construction pose as
    // the bind pose.
    let bind = PartPose {
        offset: [1.0, 2.0, 3.0],
        rotation: [0.1, 0.2, 0.3],
    };
    let child_bind = PartPose {
        offset: [4.0, 5.0, 6.0],
        rotation: [0.4, 0.5, 0.6],
    };
    let mut part = part(
        bind,
        &[],
        vec![("child", part(child_bind, &[], Vec::new()))],
    );

    // Mutate the part and its child away from bind, then hide the part.
    part.pose = PART_POSE_ZERO;
    part.visible = false;
    part.child_mut("child").pose = PART_POSE_ZERO;

    part.reset_pose();
    assert_eq!(part.pose, bind);
    assert!(part.visible);
    assert_eq!(part.child_mut("child").pose, child_bind);
}

#[test]
fn model_part_render_textured_drives_the_same_posed_tree() {
    // The textured render path walks the same `ModelPart` tree (one `setup_anim`, two paths). A
    // 1×1×1 cube emits 6 faces / 24 vertices into the textured mesh; a hidden part skips its subtree.
    let uv_rect = EntityModelUvRect {
        min: [0.0, 0.0],
        max: [1.0, 1.0],
    };
    let tint = [1.0, 1.0, 1.0, 1.0];

    let mut part = part(
        PART_POSE_ZERO,
        &UNIT_CUBE,
        vec![("child", part(PART_POSE_ZERO, &UNIT_CUBE, Vec::new()))],
    );

    let mut visible_mesh = EntityModelTexturedMesh::new();
    part.render_textured(&mut visible_mesh, Mat4::IDENTITY, uv_rect, tint);
    assert_eq!(visible_mesh.vertices.len(), 48);

    part.visible = false;
    let mut hidden_mesh = EntityModelTexturedMesh::new();
    part.render_textured(&mut hidden_mesh, Mat4::IDENTITY, uv_rect, tint);
    assert!(hidden_mesh.vertices.is_empty());
}

#[test]
fn model_part_render_textured_uses_vanilla_normal_matrix_for_normals() {
    // Vanilla `PoseStack.Pose` transforms cube face normals through the normal
    // matrix, i.e. the inverse-transpose of the pose matrix, then normalizes.
    let uv_rect = EntityModelUvRect {
        min: [0.0, 0.0],
        max: [1.0, 1.0],
    };
    let transform = Mat4::from_scale(Vec3::new(2.0, 0.5, 1.0))
        * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_4);
    let part = part(PART_POSE_ZERO, &UNIT_CUBE, Vec::new());

    let mut mesh = EntityModelTexturedMesh::new();
    part.render_textured(&mut mesh, transform, uv_rect, [1.0; 4]);

    assert_close3(mesh.vertices[0].normal, [0.24253564, -0.9701425, 0.0]);
    let ordinary_vector_normal = transform
        .transform_vector3(Vec3::new(0.0, -1.0, 0.0))
        .normalize_or_zero()
        .to_array();
    assert!(
        (mesh.vertices[0].normal[0] - ordinary_vector_normal[0]).abs() > 0.5,
        "normal must use inverse-transpose, not ordinary vector transform"
    );
}
