use crate::entity_models::catalog::ArmorStandModelPose;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

fn degrees_to_radians3(rotation: [f32; 3]) -> [f32; 3] {
    [
        rotation[0].to_radians(),
        rotation[1].to_radians(),
        rotation[2].to_radians(),
    ]
}

/// Vanilla `ArmorStandModel`.
pub(in crate::entity_models) fn armor_stand_model(
    small: bool,
    show_arms: bool,
    show_base_plate: bool,
    pose: ArmorStandModelPose,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let layer = if small {
        ModelLayer::ARMOR_STAND_SMALL
    } else {
        ModelLayer::ARMOR_STAND
    };
    let mut root = layer.bake();
    let body = degrees_to_radians3(pose.body);
    root.part_mut("hat").unwrap().visible = false;
    root.child_mut("head").pose.rotation = degrees_to_radians3(pose.head);
    root.child_mut("body").pose.rotation = body;
    if show_arms {
        root.child_mut("right_arm").pose.rotation = degrees_to_radians3(pose.right_arm);
        root.child_mut("left_arm").pose.rotation = degrees_to_radians3(pose.left_arm);
    } else {
        root.child_mut("right_arm").visible = false;
        root.child_mut("left_arm").visible = false;
    }
    root.child_mut("right_leg").pose.rotation = degrees_to_radians3(pose.right_leg);
    root.child_mut("left_leg").pose.rotation = degrees_to_radians3(pose.left_leg);
    // The two body sticks and the shoulder stick share the body pose.
    root.child_mut("right_body_stick").pose.rotation = body;
    root.child_mut("left_body_stick").pose.rotation = body;
    root.child_mut("shoulder_stick").pose.rotation = body;
    if show_base_plate {
        root.child_mut("base_plate").pose.rotation =
            [0.0, -instance.render_state.body_rot.to_radians(), 0.0];
    } else {
        root.child_mut("base_plate").visible = false;
    }
    root
}
