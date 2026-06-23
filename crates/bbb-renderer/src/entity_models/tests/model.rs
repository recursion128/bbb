use glam::Mat4;

use crate::entity_models::geometry::{EntityModelMesh, ModelCubeDesc, PartPose, PART_POSE_ZERO};
use crate::entity_models::model::ModelPart;

const UNIT_CUBE: &[ModelCubeDesc] = &[ModelCubeDesc {
    min: [0.0, 0.0, 0.0],
    size: [1.0, 1.0, 1.0],
    color: [1.0, 1.0, 1.0, 1.0],
}];

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
    let mut part = ModelPart::new(bind, &[], vec![("child", ModelPart::leaf(child_bind, &[]))]);

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
fn model_part_render_skips_invisible_subtrees() {
    // Vanilla `ModelPart.render` draws nothing for a hidden part and its whole subtree. A 1×1×1 cube
    // emits 6 faces / 24 vertices when visible.
    let mut part = ModelPart::new(
        PART_POSE_ZERO,
        UNIT_CUBE,
        vec![("child", ModelPart::leaf(PART_POSE_ZERO, UNIT_CUBE))],
    );

    let mut visible_mesh = EntityModelMesh::new();
    part.render(&mut visible_mesh, Mat4::IDENTITY);
    assert_eq!(
        visible_mesh.vertices.len(),
        48,
        "the part and its child each emit one cube"
    );

    part.visible = false;
    let mut hidden_mesh = EntityModelMesh::new();
    part.render(&mut hidden_mesh, Mat4::IDENTITY);
    assert!(
        hidden_mesh.vertices.is_empty(),
        "a hidden part skips its whole subtree"
    );
}
