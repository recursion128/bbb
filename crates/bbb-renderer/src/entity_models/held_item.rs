//! Held-item attachment: the world-space transform that places a held item at a humanoid's hand,
//! following vanilla `ItemInHandLayer` + `HumanoidModel.translateToHand`. The renderer owns the model
//! pose (the posed arm bone); the native layer resolves the item to quads and applies the per-item
//! third-person display transform before baking the held-item mesh into the item-model pass.

use std::f32::consts::{FRAC_PI_2, PI};

use glam::{Mat4, Vec3};

use super::colored::player_model_root_transform;
use super::model::EntityModel;
use super::model_layers::PlayerModel;
use super::{EntityModelInstance, EntityModelKind};

/// The model→world transform of the hand attach point for a player's main (`right`) or off (`left`)
/// hand, or `None` if the instance is not a player. Composes the posed arm bone (vanilla
/// `translateToHand` = root + arm `translateAndRotate`) with the `ItemInHandLayer` hand offset
/// (`rotX(-90°)·rotY(180°)·translate(±1, 2, -10)/16`, the adult offsets). The caller applies the item's
/// third-person display transform and the `0..=16`→unit model on top.
pub fn player_hand_attach_transform(
    instance: &EntityModelInstance,
    left_hand: bool,
) -> Option<Mat4> {
    let EntityModelKind::Player { slim, .. } = instance.kind else {
        return None;
    };
    let mut model = PlayerModel::new(slim);
    model.prepare(instance);
    let arm_name = if left_hand { "left_arm" } else { "right_arm" };
    let arm_world =
        player_model_root_transform(*instance) * model.root().child_attach_transform(arm_name);
    let sign = if left_hand { -1.0 } else { 1.0 };
    Some(
        arm_world
            * Mat4::from_rotation_x(-FRAC_PI_2)
            * Mat4::from_rotation_y(PI)
            * Mat4::from_translation(Vec3::new(sign / 16.0, 2.0 / 16.0, -10.0 / 16.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_models::PLAYER_MODEL_PARTS_ALL_VISIBLE;

    fn player_instance(y_rot: f32) -> EntityModelInstance {
        EntityModelInstance::player_with_parts(
            1,
            [0.0, 64.0, 0.0],
            y_rot,
            false,
            PLAYER_MODEL_PARTS_ALL_VISIBLE,
        )
    }

    #[test]
    fn non_player_instances_have_no_hand_attach() {
        let creeper = EntityModelInstance::new(2, EntityModelKind::Creeper, [0.0, 0.0, 0.0], 0.0);
        assert!(player_hand_attach_transform(&creeper, false).is_none());
    }

    #[test]
    fn right_hand_attach_sits_in_front_of_and_below_the_shoulder() {
        // The attach point is to the right of the entity origin, below head height, and pushed forward
        // out of the body (the -10/16 hand offset along the arm).
        let attach = player_hand_attach_transform(&player_instance(0.0), false).unwrap();
        let origin = attach.transform_point3(Vec3::ZERO);
        // Right hand → negative model-X side; the AvatarRenderer scale keeps it within ~1 block.
        assert!(origin.x < 0.0, "right hand on the -x side, got {origin:?}");
        assert!(
            origin.y < 64.0 + 1.6,
            "hand below head height, got {origin:?}"
        );
        assert!(origin.is_finite());
    }

    #[test]
    fn left_and_right_hands_mirror_across_x() {
        let right = player_hand_attach_transform(&player_instance(0.0), false)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        let left = player_hand_attach_transform(&player_instance(0.0), true)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        // Mirror entities face -Z; the two hands straddle the body center on X.
        assert!(
            right.x < left.x,
            "right {right:?} should be left of left {left:?}"
        );
    }
}
