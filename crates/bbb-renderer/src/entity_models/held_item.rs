//! Held-item attachment: the world-space transform that places a held item at a humanoid's hand,
//! following vanilla `ItemInHandLayer` + `HumanoidModel.translateToHand`. The renderer owns the model
//! pose (the posed arm bone); the native layer resolves the item to quads and applies the per-item
//! third-person display transform before baking the held-item mesh into the item-model pass.

use std::f32::consts::{FRAC_PI_2, PI};

use glam::{Mat4, Vec3};

use super::colored::{
    entity_model_root_transform, player_model_root_transform, villager_adult_model_root_transform,
    wither_skeleton_model_root_transform, zombie_variant_root_transform,
};
use super::model::EntityModel;
use super::model_layers::{
    IllagerModel, PiglinModel, PlayerModel, SkeletonModel, ZombieModel, ZombieVariantModel,
};
use super::{EntityModelInstance, EntityModelKind, SkeletonModelFamily};

/// The model→world transform of the hand attach point for a humanoid's main (`right`) or off (`left`)
/// hand, or `None` if the instance is not an adult humanoid that holds items the standard way. Composes
/// the posed arm bone (vanilla `translateToHand` = root + arm `translateAndRotate`) with the
/// `ItemInHandLayer` hand offset (`rotX(-90°)·rotY(180°)·translate(±1, 2, -10)/16`, the adult offsets).
/// The caller applies the item's third-person display transform and the `0..=16`→unit model on top.
/// Baby humanoids (which use different hand offsets and a baby-scaled pose) are deferred.
pub fn humanoid_hand_attach_transform(
    instance: &EntityModelInstance,
    left_hand: bool,
) -> Option<Mat4> {
    let arm_name = if left_hand { "left_arm" } else { "right_arm" };
    let arm_world = humanoid_arm_world_transform(instance, arm_name)?;
    let sign = if left_hand { -1.0 } else { 1.0 };
    Some(
        arm_world
            * Mat4::from_rotation_x(-FRAC_PI_2)
            * Mat4::from_rotation_y(PI)
            * Mat4::from_translation(Vec3::new(sign / 16.0, 2.0 / 16.0, -10.0 / 16.0)),
    )
}

/// The world transform of a named arm bone for the humanoid families that render held items: builds and
/// poses the same model + root transform the entity scene uses, then reads `root · arm` (vanilla
/// `translateToHand`). Returns `None` for non-humanoid kinds, baby humanoids, or any model that lacks the
/// standard arm bone (so the held-item layer degrades to rendering nothing rather than panicking).
fn humanoid_arm_world_transform(instance: &EntityModelInstance, arm_name: &str) -> Option<Mat4> {
    match instance.kind {
        EntityModelKind::Player { slim, .. } => {
            let mut model = PlayerModel::new(slim);
            model.prepare(instance);
            Some(
                player_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
            )
        }
        EntityModelKind::Zombie { baby: false } => {
            let mut model = ZombieModel::new(false);
            model.prepare(instance);
            Some(
                entity_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
            )
        }
        EntityModelKind::ZombieVariant {
            family,
            baby: false,
        } => {
            let mut model = ZombieVariantModel::new(family, false);
            model.prepare(instance);
            Some(
                zombie_variant_root_transform(*instance, family, false)
                    * model.root().try_child_attach_transform(arm_name)?,
            )
        }
        EntityModelKind::Piglin {
            family,
            baby: false,
        } => {
            let mut model = PiglinModel::new(family, false);
            model.prepare(instance);
            Some(
                entity_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
            )
        }
        EntityModelKind::Skeleton => {
            let mut model = SkeletonModel::new(None);
            model.prepare(instance);
            Some(
                entity_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
            )
        }
        EntityModelKind::SkeletonVariant { family } => {
            let mut model = SkeletonModel::new(Some(family));
            model.prepare(instance);
            let root = if family == SkeletonModelFamily::WitherSkeleton {
                wither_skeleton_model_root_transform(*instance)
            } else {
                entity_model_root_transform(*instance)
            };
            Some(root * model.root().try_child_attach_transform(arm_name)?)
        }
        EntityModelKind::Illager { family } => {
            let mut model = IllagerModel::new(instance, family);
            model.prepare(instance);
            Some(
                villager_adult_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
            )
        }
        _ => None,
    }
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
    fn non_humanoid_instances_have_no_hand_attach() {
        let creeper = EntityModelInstance::new(2, EntityModelKind::Creeper, [0.0, 0.0, 0.0], 0.0);
        assert!(humanoid_hand_attach_transform(&creeper, false).is_none());
    }

    #[test]
    fn adult_humanoid_mobs_have_a_hand_attach() {
        // A zombie (and the other weapon-holding humanoid families) attaches a held item the same way a
        // player does — the generic dispatch poses the family model and reads its right-arm bone.
        let zombie = EntityModelInstance::new(
            3,
            EntityModelKind::Zombie { baby: false },
            [0.0, 64.0, 0.0],
            0.0,
        );
        let attach = humanoid_hand_attach_transform(&zombie, false).unwrap();
        assert!(attach.transform_point3(Vec3::ZERO).is_finite());
        // Baby humanoids are deferred (different offsets / baby-scaled pose).
        let baby_zombie = EntityModelInstance::new(
            4,
            EntityModelKind::Zombie { baby: true },
            [0.0, 64.0, 0.0],
            0.0,
        );
        assert!(humanoid_hand_attach_transform(&baby_zombie, false).is_none());
    }

    #[test]
    fn right_hand_attach_sits_in_front_of_and_below_the_shoulder() {
        // The attach point is to the right of the entity origin, below head height, and pushed forward
        // out of the body (the -10/16 hand offset along the arm).
        let attach = humanoid_hand_attach_transform(&player_instance(0.0), false).unwrap();
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
        let right = humanoid_hand_attach_transform(&player_instance(0.0), false)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        let left = humanoid_hand_attach_transform(&player_instance(0.0), true)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        // Mirror entities face -Z; the two hands straddle the body center on X.
        assert!(
            right.x < left.x,
            "right {right:?} should be left of left {left:?}"
        );
    }
}
