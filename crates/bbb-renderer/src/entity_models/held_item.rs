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
/// hand, or `None` if the instance is not a humanoid that holds items the standard way. Composes the
/// posed arm bone (vanilla `translateToHand` = root + arm `translateAndRotate`) with the
/// `ItemInHandLayer` hand offset (`rotX(-90°)·rotY(180°)·translate(±offsetX, offsetY, offsetZ)/16`).
/// Vanilla `useBabyOffset` selects the offsets: adult `(1, 2, -10)`, baby `(0, 1, -4.5)`. The baby
/// humanoid families (zombie, zombie variants, piglin) bake their reduced proportions straight into an
/// explicit baby mesh (no part scale), so the baby attach uses the same `root · arm` formula on the baby
/// model with only the offset swapped. The caller applies the item's third-person display transform and
/// the `0..=16`→unit model on top.
pub fn humanoid_hand_attach_transform(
    instance: &EntityModelInstance,
    left_hand: bool,
) -> Option<Mat4> {
    let arm_name = if left_hand { "left_arm" } else { "right_arm" };
    let (arm_world, baby) = humanoid_arm_world_transform(instance, arm_name)?;
    let sign = if left_hand { -1.0 } else { 1.0 };
    // Vanilla `ItemInHandLayer.submitArmWithItem`: `offsetX/Y/Z` are `(1, 2, -10)` adult, `(0, 1, -4.5)`
    // baby (so baby hands share the X=0 column — the left/right split comes only from the arm bone).
    let (offset_x, offset_y, offset_z) = if baby {
        (0.0, 1.0, -4.5)
    } else {
        (1.0, 2.0, -10.0)
    };
    Some(
        arm_world
            * Mat4::from_rotation_x(-FRAC_PI_2)
            * Mat4::from_rotation_y(PI)
            * Mat4::from_translation(Vec3::new(
                sign * offset_x / 16.0,
                offset_y / 16.0,
                offset_z / 16.0,
            )),
    )
}

/// The world transform of a named arm bone plus whether the instance is a baby (so the caller picks the
/// baby hand offset), for the humanoid families that render held items: builds and poses the same model
/// + root transform the entity scene uses, then reads `root · arm` (vanilla `translateToHand`). Returns
/// `None` for non-humanoid kinds or any model that lacks the standard arm bone (so the held-item layer
/// degrades to rendering nothing rather than panicking).
fn humanoid_arm_world_transform(
    instance: &EntityModelInstance,
    arm_name: &str,
) -> Option<(Mat4, bool)> {
    match instance.kind {
        EntityModelKind::Player { slim, .. } => {
            let mut model = PlayerModel::new(slim);
            model.prepare(instance);
            Some((
                player_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
                false,
            ))
        }
        EntityModelKind::Zombie { baby } => {
            let mut model = ZombieModel::new(baby);
            model.prepare(instance);
            Some((
                entity_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
                baby,
            ))
        }
        EntityModelKind::ZombieVariant { family, baby } => {
            let mut model = ZombieVariantModel::new(family, baby);
            model.prepare(instance);
            Some((
                zombie_variant_root_transform(*instance, family, baby)
                    * model.root().try_child_attach_transform(arm_name)?,
                baby,
            ))
        }
        EntityModelKind::Piglin { family, baby } => {
            let mut model = PiglinModel::new(family, baby);
            model.prepare(instance);
            Some((
                entity_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
                baby,
            ))
        }
        EntityModelKind::Skeleton => {
            let mut model = SkeletonModel::new(None);
            model.prepare(instance);
            Some((
                entity_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
                false,
            ))
        }
        EntityModelKind::SkeletonVariant { family } => {
            let mut model = SkeletonModel::new(Some(family));
            model.prepare(instance);
            let root = if family == SkeletonModelFamily::WitherSkeleton {
                wither_skeleton_model_root_transform(*instance)
            } else {
                entity_model_root_transform(*instance)
            };
            Some((
                root * model.root().try_child_attach_transform(arm_name)?,
                false,
            ))
        }
        EntityModelKind::Illager { family } => {
            let mut model = IllagerModel::new(instance, family);
            model.prepare(instance);
            Some((
                villager_adult_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
                false,
            ))
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
    }

    #[test]
    fn baby_humanoid_mobs_attach_lower_and_more_inward_than_adults() {
        // Baby zombies hold items too (vanilla `BabyZombieModel` is an explicit smaller mesh with the
        // baby `ItemInHandLayer` offset). The baby hand sits below and closer to the body center than the
        // adult's, since the baby arm bone is smaller and the baby offset drops X to 0.
        let adult = EntityModelInstance::new(
            5,
            EntityModelKind::Zombie { baby: false },
            [0.0, 64.0, 0.0],
            0.0,
        );
        let baby = EntityModelInstance::new(
            6,
            EntityModelKind::Zombie { baby: true },
            [0.0, 64.0, 0.0],
            0.0,
        );
        let adult_hand = humanoid_hand_attach_transform(&adult, false)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        let baby_hand = humanoid_hand_attach_transform(&baby, false)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        assert!(baby_hand.is_finite());
        // Baby is shorter, so its hand is lower than the adult's.
        assert!(
            baby_hand.y < adult_hand.y,
            "baby hand {baby_hand:?} below adult {adult_hand:?}"
        );
        // Baby right hand is closer to the X center than the adult's (smaller arm + X=0 offset).
        assert!(
            baby_hand.x.abs() < adult_hand.x.abs(),
            "baby hand {baby_hand:?} more inward than adult {adult_hand:?}"
        );
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
