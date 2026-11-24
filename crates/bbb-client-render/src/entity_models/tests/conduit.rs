use super::*;

use crate::entity_models::model_layers::{
    CONDUIT_BASE_TEXTURE_REF, CONDUIT_CAGE_TEXTURE_REF, CONDUIT_CLOSED_EYE_TEXTURE_REF,
    CONDUIT_WIND_VERTICAL_TEXTURE_REF,
};
use crate::entity_models::transforms::conduit_model_root_transform;
use glam::Vec3;

fn conduit_instance(part: ConduitModelPart) -> EntityModelInstance {
    EntityModelInstance::conduit(-1, [2.0, 3.0, 4.0], part).with_block_entity(|s| {
        s.conduit_anim_time = 0.0;
        s.conduit_active_rotation = 0.0;
    })
}

#[test]
fn conduit_layer_passes_match_vanilla_renderer() {
    let shell = conduit_textured_layer_passes(ConduitModelPart::Shell);
    assert_eq!(shell.len(), 1);
    assert_eq!(
        shell[0].render_type,
        EntityModelLayerRenderType::EntitySolid
    );
    assert_eq!(shell[0].texture, CONDUIT_BASE_TEXTURE_REF);

    let cage = conduit_textured_layer_passes(ConduitModelPart::Cage);
    assert_eq!(
        cage[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(cage[0].texture, CONDUIT_CAGE_TEXTURE_REF);

    let vertical_wind = conduit_textured_layer_passes(ConduitModelPart::OuterWind { phase: 1 });
    assert_eq!(vertical_wind[0].texture, CONDUIT_WIND_VERTICAL_TEXTURE_REF);

    let eye = conduit_textured_layer_passes(ConduitModelPart::Eye { open: false });
    assert_eq!(eye[0].texture, CONDUIT_CLOSED_EYE_TEXTURE_REF);
}

#[test]
fn conduit_transforms_match_vanilla_submit_positions() {
    let shell = conduit_model_root_transform(
        &conduit_instance(ConduitModelPart::Shell),
        ConduitModelPart::Shell,
    );
    assert!((shell.transform_point3(Vec3::ZERO) - Vec3::new(2.5, 3.5, 4.5)).length() < 1.0e-6);

    let cage = conduit_model_root_transform(
        &conduit_instance(ConduitModelPart::Cage),
        ConduitModelPart::Cage,
    );
    // At animTime 0, vanilla bob is (0.5^2 + 0.5), so y = 0.3 + 0.75 * 0.2.
    assert!((cage.transform_point3(Vec3::ZERO) - Vec3::new(2.5, 3.45, 4.5)).length() < 1.0e-6);

    let wind = conduit_model_root_transform(
        &conduit_instance(ConduitModelPart::OuterWind { phase: 1 }),
        ConduitModelPart::OuterWind { phase: 1 },
    );
    assert!((wind.transform_point3(Vec3::Y) - Vec3::new(2.5, 3.5, 5.5)).length() < 1.0e-6);

    let inner = conduit_model_root_transform(
        &conduit_instance(ConduitModelPart::InnerWind { vertical: false }),
        ConduitModelPart::InnerWind { vertical: false },
    );
    assert!((inner.transform_point3(Vec3::X) - Vec3::new(1.625, 3.5, 4.5)).length() < 1.0e-6);
}

#[test]
fn conduit_textured_mesh_submits_shell_and_active_parts() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/conduit/"))
            .iter()
            .map(|texture| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![9; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instances = [
        conduit_instance(ConduitModelPart::Shell)
            .with_block_entity(|s| {
                s.block_entity_crumbling = Some(test_block_entity_crumbling([2.0, 3.0, 4.0]))
            })
            .with(|s| s.light_coords = (4_u32 << 4) | (12_u32 << 20)),
        conduit_instance(ConduitModelPart::Cage)
            .with_block_entity(|s| {
                s.block_entity_crumbling = Some(test_block_entity_crumbling([2.0, 3.0, 4.0]))
            })
            .with(|s| s.light_coords = (5_u32 << 4) | (11_u32 << 20)),
        conduit_instance(ConduitModelPart::OuterWind { phase: 1 }),
        conduit_instance(ConduitModelPart::InnerWind { vertical: false }),
        conduit_instance(ConduitModelPart::Eye { open: true }),
    ];
    let meshes = entity_model_textured_meshes(&instances, &atlas);

    // Shell uses culling `entitySolid`; the active parts use non-culling `entityCutout`.
    assert_eq!(meshes.solid.cutout_faces, 6);
    assert_eq!(meshes.cutout.cutout_faces, 24);
    // Vanilla attaches the overlay only to shell and cage submissions. Wind and eye use otherwise
    // eligible entity cutout layers, so this proves renderer extraction state (not bucket type)
    // selects the conduit geometry that crumbles.
    let eligible = [
        conduit_instance(ConduitModelPart::Shell),
        conduit_instance(ConduitModelPart::Cage),
    ];
    let eligible_meshes = entity_model_textured_meshes(&eligible, &atlas);
    assert_eq!(
        meshes.crumbling.indices.len(),
        eligible_meshes.solid.indices.len() + eligible_meshes.cutout.indices.len()
    );
    assert_eq!(
        meshes.crumbling.vertices.len(),
        eligible_meshes.solid.vertices.len() + eligible_meshes.cutout.vertices.len()
    );
}
