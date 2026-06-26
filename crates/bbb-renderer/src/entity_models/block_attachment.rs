//! Block-model attachments for entity render layers. The renderer owns these transforms because they
//! depend on the same posed model bones as the entity mesh; native resolves the block model to quads and
//! bakes it through the returned matrix into the shared item-model pass.

use std::f32::consts::PI;

use glam::{Mat4, Vec3};

use super::colored::entity_model_root_transform;
use super::model::EntityModel;
use super::model_layers::{CopperGolemModel, CowModel, IronGolemModel, SnowGolemModel};
use super::{CowModelVariant, EntityModelInstance, EntityModelKind, MooshroomVariant};

fn unit_cube_bottom_center_to_antenna_center() -> Mat4 {
    Mat4::from_translation(Vec3::new(-0.5, 0.0, -0.5))
        * Mat4::from_translation(Vec3::splat(0.5))
        * Mat4::from_rotation_z(PI)
        * Mat4::from_translation(Vec3::splat(-0.5))
}

/// World transform for vanilla `BlockDecorationLayer` on a copper golem's antenna block.
///
/// The returned matrix expects block quads normalized to the `0..1` unit cube. It includes
/// `CopperGolemModel.applyBlockOnAntennaTransform`: `root -> body -> head`, translate `(0, -1.75, 0)`,
/// then the layer's shared unit-cube-bottom-center-to-antenna-center transform.
pub fn copper_golem_antenna_block_transform(instance: &EntityModelInstance) -> Option<Mat4> {
    if !matches!(instance.kind, EntityModelKind::CopperGolem { .. })
        || instance.render_state.invisible
    {
        return None;
    }
    let mut model = CopperGolemModel::new();
    model.prepare(instance);
    Some(
        entity_model_root_transform(*instance)
            * model
                .root()
                .try_descendant_attach_transform(&["body", "head"])?
            * Mat4::from_translation(Vec3::new(0.0, -1.75, 0.0))
            * unit_cube_bottom_center_to_antenna_center(),
    )
}

/// World transform for vanilla `CarriedBlockLayer` on endermen.
///
/// The returned matrix expects block quads normalized to the `0..1` unit cube. Vanilla applies this
/// layer from the entity model root, not from the arms: translate `(0, 0.6875, -0.75)`, rotate X 20°,
/// rotate Y 45°, translate `(0.25, 0.1875, 0.25)`, scale `(-0.5, -0.5, 0.5)`, then rotate Y 90°.
pub fn enderman_carried_block_transform(instance: &EntityModelInstance) -> Option<Mat4> {
    if !matches!(instance.kind, EntityModelKind::Enderman)
        || !instance.render_state.enderman_carrying
        || instance.render_state.invisible
    {
        return None;
    }
    Some(
        entity_model_root_transform(*instance)
            * Mat4::from_translation(Vec3::new(0.0, 0.6875, -0.75))
            * Mat4::from_rotation_x(20.0_f32.to_radians())
            * Mat4::from_rotation_y(45.0_f32.to_radians())
            * Mat4::from_translation(Vec3::new(0.25, 0.1875, 0.25))
            * Mat4::from_scale(Vec3::new(-0.5, -0.5, 0.5))
            * Mat4::from_rotation_y(PI / 2.0),
    )
}

/// World transform for vanilla `IronGolemFlowerLayer`'s held poppy block model.
///
/// The returned matrix expects block quads normalized to the `0..1` unit cube, matching
/// [`crate::ItemModelMesh::append_quads`]. It includes the posed right-arm bone and the vanilla layer
/// transform after `getFlowerHoldingArm().translateAndRotate`: translate `(-1.1875, 1.0625,
/// -0.9375)`, translate to the block center, scale by `0.5`, rotate X by `-90°`, then translate the
/// block origin by `(-0.5, -0.5, -0.5)`.
pub fn iron_golem_flower_block_transform(instance: &EntityModelInstance) -> Option<Mat4> {
    if !matches!(instance.kind, EntityModelKind::IronGolem { .. })
        || instance.render_state.iron_golem_offer_flower_tick <= 0
        || instance.render_state.invisible
    {
        return None;
    }
    let mut model = IronGolemModel::new();
    model.prepare(instance);
    Some(
        entity_model_root_transform(*instance)
            * model.root().try_child_attach_transform("right_arm")?
            * Mat4::from_translation(Vec3::new(-1.1875, 1.0625, -0.9375))
            * Mat4::from_translation(Vec3::splat(0.5))
            * Mat4::from_scale(Vec3::splat(0.5))
            * Mat4::from_rotation_x(-PI / 2.0)
            * Mat4::from_translation(Vec3::splat(-0.5)),
    )
}

/// World transforms for vanilla `MushroomCowMushroomLayer`'s three mushroom block models.
///
/// The returned matrices expect block quads normalized to the `0..1` unit cube. The first two
/// transforms mirror the layer's hardcoded back mushrooms in entity-model space; the third includes the
/// posed cow head bone before applying the head mushroom transform.
pub fn mooshroom_mushroom_block_transforms(
    instance: &EntityModelInstance,
) -> Option<(MooshroomVariant, [Mat4; 3])> {
    let EntityModelKind::Mooshroom {
        baby: false,
        variant,
    } = instance.kind
    else {
        return None;
    };
    if instance.render_state.invisible {
        return None;
    }

    let root = entity_model_root_transform(*instance);
    let mut model = CowModel::new(CowModelVariant::Temperate, false);
    model.prepare(instance);
    Some((
        variant,
        [
            root * Mat4::from_translation(Vec3::new(0.2, -0.35, 0.5))
                * Mat4::from_rotation_y(-48.0_f32.to_radians())
                * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
                * Mat4::from_translation(Vec3::splat(-0.5)),
            root * Mat4::from_translation(Vec3::new(0.2, -0.35, 0.5))
                * Mat4::from_rotation_y(42.0_f32.to_radians())
                * Mat4::from_translation(Vec3::new(0.1, 0.0, -0.6))
                * Mat4::from_rotation_y(-48.0_f32.to_radians())
                * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
                * Mat4::from_translation(Vec3::splat(-0.5)),
            root * model.root().try_child_attach_transform("head")?
                * Mat4::from_translation(Vec3::new(0.0, -0.7, -0.2))
                * Mat4::from_rotation_y(-78.0_f32.to_radians())
                * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
                * Mat4::from_translation(Vec3::splat(-0.5)),
        ],
    ))
}

/// World transform for vanilla `SnowGolemHeadLayer`'s carved-pumpkin block model.
///
/// The returned matrix expects block quads normalized to the `0..1` unit cube, matching
/// [`crate::ItemModelMesh::append_quads`]. It includes the posed snow-golem head bone and the vanilla
/// layer transform after `getHead().translateAndRotate`: translate `(0, -0.34375, 0)`, rotate Y 180°,
/// scale `(0.625, -0.625, -0.625)`, then translate the block origin by `(-0.5, -0.5, -0.5)`.
pub fn snow_golem_head_block_transform(instance: &EntityModelInstance) -> Option<Mat4> {
    if !matches!(instance.kind, EntityModelKind::SnowGolem)
        || !instance.render_state.snow_golem_pumpkin
        || instance.render_state.invisible
    {
        return None;
    }
    let mut model = SnowGolemModel::new();
    model.prepare(instance);
    Some(
        entity_model_root_transform(*instance)
            * model.root().try_child_attach_transform("head")?
            * Mat4::from_translation(Vec3::new(0.0, -0.34375, 0.0))
            * Mat4::from_rotation_y(PI)
            * Mat4::from_scale(Vec3::new(0.625, -0.625, -0.625))
            * Mat4::from_translation(Vec3::splat(-0.5)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_models::CopperGolemWeathering;
    use glam::Vec3;

    fn copper_golem() -> EntityModelInstance {
        EntityModelInstance::new(
            28,
            EntityModelKind::CopperGolem {
                weathering: CopperGolemWeathering::Unaffected,
            },
            [0.0, 64.0, 0.0],
            0.0,
        )
    }

    fn iron_golem() -> EntityModelInstance {
        EntityModelInstance::iron_golem(74, [0.0, 64.0, 0.0], 0.0)
            .with_iron_golem_offer_flower_tick(400)
    }

    fn snow_golem() -> EntityModelInstance {
        EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0).with_snow_golem_pumpkin(true)
    }

    fn mooshroom(baby: bool) -> EntityModelInstance {
        EntityModelInstance::new(
            86,
            EntityModelKind::Mooshroom {
                baby,
                variant: MooshroomVariant::Red,
            },
            [0.0, 64.0, 0.0],
            0.0,
        )
    }

    #[test]
    fn enderman_carried_block_is_gated_on_kind_visibility_and_carry_state() {
        let enderman =
            EntityModelInstance::enderman(41, [0.0, 64.0, 0.0], 0.0).with_enderman_carrying(true);
        assert!(enderman_carried_block_transform(&enderman).is_some());

        let empty = enderman.with_enderman_carrying(false);
        assert!(enderman_carried_block_transform(&empty).is_none());

        let invisible = enderman.with_invisible(true);
        assert!(enderman_carried_block_transform(&invisible).is_none());

        let creeper =
            EntityModelInstance::new(100, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0)
                .with_enderman_carrying(true);
        assert!(enderman_carried_block_transform(&creeper).is_none());
    }

    #[test]
    fn enderman_carried_block_follows_model_root_rotation() {
        let base = enderman_carried_block_transform(
            &EntityModelInstance::enderman(41, [0.0, 64.0, 0.0], 0.0).with_enderman_carrying(true),
        )
        .unwrap();
        let turned = enderman_carried_block_transform(
            &EntityModelInstance::enderman(41, [0.0, 64.0, 0.0], 90.0).with_enderman_carrying(true),
        )
        .unwrap();
        let base_center = base.transform_point3(Vec3::splat(0.5));
        let turned_center = turned.transform_point3(Vec3::splat(0.5));

        assert!(base_center.is_finite());
        assert!(turned_center.is_finite());
        assert_ne!(base_center, turned_center);
    }

    #[test]
    fn copper_golem_antenna_block_is_gated_on_kind_and_visibility() {
        assert!(copper_golem_antenna_block_transform(&copper_golem()).is_some());
        assert!(
            copper_golem_antenna_block_transform(&copper_golem().with_invisible(true)).is_none()
        );
        assert!(copper_golem_antenna_block_transform(&snow_golem()).is_none());
    }

    #[test]
    fn copper_golem_antenna_block_follows_head_look() {
        let base = copper_golem_antenna_block_transform(&copper_golem()).unwrap();
        let turned =
            copper_golem_antenna_block_transform(&copper_golem().with_head_look(30.0, 20.0))
                .unwrap();
        let base_center = base.transform_point3(Vec3::splat(0.5));
        let turned_center = turned.transform_point3(Vec3::splat(0.5));

        assert!(base_center.is_finite());
        assert!(turned_center.is_finite());
        assert_ne!(base_center, turned_center);
    }

    #[test]
    fn iron_golem_flower_block_is_gated_on_kind_visibility_and_offer_state() {
        assert!(iron_golem_flower_block_transform(&iron_golem()).is_some());

        let idle = iron_golem().with_iron_golem_offer_flower_tick(0);
        assert!(iron_golem_flower_block_transform(&idle).is_none());

        let invisible = iron_golem().with_invisible(true);
        assert!(iron_golem_flower_block_transform(&invisible).is_none());

        let creeper =
            EntityModelInstance::new(100, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0)
                .with_iron_golem_offer_flower_tick(400);
        assert!(iron_golem_flower_block_transform(&creeper).is_none());
    }

    #[test]
    fn iron_golem_flower_block_follows_flower_arm_pose() {
        let base = iron_golem_flower_block_transform(&iron_golem()).unwrap();
        let later =
            iron_golem_flower_block_transform(&iron_golem().with_iron_golem_offer_flower_tick(350))
                .unwrap();
        let attacking = iron_golem_flower_block_transform(
            &iron_golem().with_iron_golem_attack_ticks_remaining(5.0),
        )
        .unwrap();
        let base_center = base.transform_point3(Vec3::splat(0.5));
        let later_center = later.transform_point3(Vec3::splat(0.5));
        let attacking_center = attacking.transform_point3(Vec3::splat(0.5));

        assert!(base_center.is_finite());
        assert!(later_center.is_finite());
        assert!(attacking_center.is_finite());
        assert_ne!(base_center, later_center);
        assert_ne!(base_center, attacking_center);
    }

    #[test]
    fn mooshroom_mushroom_blocks_are_gated_on_adult_visibility_and_kind() {
        let (variant, transforms) = mooshroom_mushroom_block_transforms(&mooshroom(false)).unwrap();
        assert_eq!(variant, MooshroomVariant::Red);
        assert_eq!(transforms.len(), 3);

        assert!(mooshroom_mushroom_block_transforms(&mooshroom(true)).is_none());
        assert!(
            mooshroom_mushroom_block_transforms(&mooshroom(false).with_invisible(true)).is_none()
        );
        assert!(mooshroom_mushroom_block_transforms(&snow_golem()).is_none());
    }

    #[test]
    fn mooshroom_head_mushroom_follows_head_look() {
        let (_, base) = mooshroom_mushroom_block_transforms(&mooshroom(false)).unwrap();
        let (_, turned) =
            mooshroom_mushroom_block_transforms(&mooshroom(false).with_head_look(35.0, -15.0))
                .unwrap();
        let base_back_center = base[0].transform_point3(Vec3::splat(0.5));
        let turned_back_center = turned[0].transform_point3(Vec3::splat(0.5));
        let base_head_center = base[2].transform_point3(Vec3::splat(0.5));
        let turned_head_center = turned[2].transform_point3(Vec3::splat(0.5));

        assert_eq!(base_back_center, turned_back_center);
        assert!(base_head_center.is_finite());
        assert!(turned_head_center.is_finite());
        assert_ne!(base_head_center, turned_head_center);
    }

    #[test]
    fn snow_golem_head_block_is_gated_on_kind_visibility_and_pumpkin_state() {
        assert!(snow_golem_head_block_transform(&snow_golem()).is_some());

        let sheared = snow_golem().with_snow_golem_pumpkin(false);
        assert!(snow_golem_head_block_transform(&sheared).is_none());

        let invisible = snow_golem().with_invisible(true);
        assert!(snow_golem_head_block_transform(&invisible).is_none());

        let creeper =
            EntityModelInstance::new(100, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0)
                .with_snow_golem_pumpkin(true);
        assert!(snow_golem_head_block_transform(&creeper).is_none());
    }

    #[test]
    fn snow_golem_head_block_follows_head_look() {
        let base = snow_golem_head_block_transform(&snow_golem()).unwrap();
        let turned =
            snow_golem_head_block_transform(&snow_golem().with_head_look(45.0, 20.0)).unwrap();
        let base_center = base.transform_point3(Vec3::splat(0.5));
        let turned_center = turned.transform_point3(Vec3::splat(0.5));

        assert!(base_center.is_finite());
        assert!(turned_center.is_finite());
        assert_ne!(base_center, turned_center);
    }
}
