//! Block-model attachments for entity render layers. The renderer owns these transforms because they
//! depend on the same posed model bones as the entity mesh; native resolves the block model to quads and
//! bakes it through the returned matrix into the shared item-model pass.

use std::f32::consts::PI;

use glam::{Mat4, Vec3};

use super::colored::entity_model_root_transform;
use super::model::EntityModel;
use super::model_layers::SnowGolemModel;
use super::{EntityModelInstance, EntityModelKind};

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
    use glam::Vec3;

    fn snow_golem() -> EntityModelInstance {
        EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0).with_snow_golem_pumpkin(true)
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
