use super::*;

use crate::entity_models::model_layers::{chest_lid_eased_openness, chest_model};
use crate::entity_models::transforms::chest_model_root_transform;
use glam::{Mat4, Vec3};
use std::f32::consts::FRAC_PI_2;

#[test]
fn chest_lid_angle_matches_vanilla_easing_and_rotation() {
    // Vanilla `ChestRenderer.submit`: `o = 1 - o; o = 1 - o^3`, then
    // `ChestModel.setupAnim`: `lid.xRot = -(o * π/2)`, `lock.xRot = lid.xRot`.
    assert_eq!(chest_lid_eased_openness(0.0), 0.0);
    assert!((chest_lid_eased_openness(0.5) - 0.875).abs() < 1e-6);
    assert_eq!(chest_lid_eased_openness(1.0), 1.0);

    let instance = EntityModelInstance::chest(
        -1,
        [0.0, 64.0, 0.0],
        0.0,
        ChestModelTexture::Normal,
        ChestModelHalf::Single,
    )
    .with_block_entity(|s| s.chest_openness = 0.5);
    let model = chest_model(
        ChestModelHalf::Single,
        chest_lid_eased_openness(instance.block_entity.chest_openness),
    );
    let expected = -(0.875 * FRAC_PI_2);
    let lid_rot = model.try_child("lid").unwrap().pose.rotation;
    let lock_rot = model.try_child("lock").unwrap().pose.rotation;
    assert!((lid_rot[0] - expected).abs() < 1e-6);
    assert_eq!(lock_rot[0], lid_rot[0]);
    assert_eq!(lid_rot[1], 0.0);
    assert_eq!(lid_rot[2], 0.0);
    // The bottom never rotates, and a closed chest rests at zero.
    assert_eq!(
        model.try_child("bottom").unwrap().pose.rotation,
        [0.0, 0.0, 0.0]
    );

    let instance = instance.with_block_entity(|s| s.chest_openness = 0.0);
    let closed = chest_model(
        ChestModelHalf::Single,
        chest_lid_eased_openness(instance.block_entity.chest_openness),
    );
    assert_eq!(
        closed.try_child("lid").unwrap().pose.rotation,
        [0.0, 0.0, 0.0]
    );

    // `ChestSpecialRenderer` passes its JSON openness directly to
    // `ChestModel.setupAnim` instead of using the world-renderer easing.
    let special = chest_model(ChestModelHalf::Single, 0.5);
    let special_lid = special.try_child("lid").unwrap().pose.rotation[0];
    assert!((special_lid + 0.5 * FRAC_PI_2).abs() < 1e-6);
    assert_ne!(special_lid, lid_rot[0]);
}

#[test]
fn chest_facing_rotation_matches_vanilla_rotation_around_block_centre() {
    // Vanilla `ChestRenderer.createModelTransformation`: rotationAround(
    // Axis.YP.rotationDegrees(-facing.toYRot()), 0.5, 0.0, 0.5), no entity flip.
    // NORTH: toYRot = 180 -> body_rot = -180; the model-space min corner (0,0,0)
    // maps to the opposite corner (1,0,1) of the block.
    let north = chest_model_root_transform(&EntityModelInstance::chest(
        -1,
        [10.0, 5.0, -3.0],
        -180.0,
        ChestModelTexture::Normal,
        ChestModelHalf::Single,
    ));
    let corner = north.transform_point3(Vec3::ZERO);
    assert!((corner - Vec3::new(11.0, 5.0, -2.0)).length() < 1e-5);
    // The pivot column (0.5, y, 0.5) is invariant.
    let pivot = north.transform_point3(Vec3::new(0.5, 0.25, 0.5));
    assert!((pivot - Vec3::new(10.5, 5.25, -2.5)).length() < 1e-5);
    // EAST: toYRot = 270 -> body_rot = -270 (≡ +90°): (0,0,0) -> (0,0,1).
    let east = chest_model_root_transform(&EntityModelInstance::chest(
        -1,
        [0.0, 0.0, 0.0],
        -270.0,
        ChestModelTexture::Normal,
        ChestModelHalf::Single,
    ));
    let corner = east.transform_point3(Vec3::ZERO);
    assert!((corner - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-5);
    // No `scale(-1, -1, 1)` flip: the transform is orientation-preserving and
    // keeps +Y up.
    assert!(north.determinant() > 0.0);
    assert!((north.transform_vector3(Vec3::Y) - Vec3::Y).length() < 1e-6);
    // SOUTH: toYRot = 0 -> identity rotation at the block corner.
    let south = chest_model_root_transform(&EntityModelInstance::chest(
        -1,
        [2.0, 3.0, 4.0],
        0.0,
        ChestModelTexture::Normal,
        ChestModelHalf::Single,
    ));
    assert!((south.transform_point3(Vec3::ZERO) - Vec3::new(2.0, 3.0, 4.0)).length() < 1e-6);
    assert_eq!(south, Mat4::from_translation(Vec3::new(2.0, 3.0, 4.0)));
}

#[test]
fn chest_texture_refs_match_vanilla_selection() {
    // Sheets.chooseSprite: `<prefix>` / `<prefix>_left` / `<prefix>_right`, ender
    // always the single `ender` sprite.
    for texture in &texture_refs_matching(|path| path.starts_with("textures/entity/chest/")) {
        assert!(entity_model_texture_refs().contains(texture));
        assert_eq!(texture.size, [64, 64]);
    }
}

#[test]
fn chest_layer_passes_match_vanilla_renderer() {
    let passes = chest_textured_layer_passes(ChestModelTexture::Trapped, ChestModelHalf::Left);
    assert_eq!(passes.len(), 1);
    // Vanilla `ChestModel`'s constructor picks `RenderTypes::entityCutoutCull`.
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutoutCull
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutoutCull");
    assert_eq!(passes[0].texture, CHEST_TRAPPED_LEFT_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
    let christmas =
        chest_textured_layer_passes(ChestModelTexture::Christmas, ChestModelHalf::Right);
    assert_eq!(christmas[0].texture, CHEST_CHRISTMAS_RIGHT_TEXTURE_REF);
    assert_eq!(
        christmas[0].render_type,
        EntityModelLayerRenderType::EntityCutoutCull
    );
}

#[test]
fn chest_textured_mesh_bakes_three_boxes_into_the_cutout_bucket() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/chest/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::chest(
        -1,
        [3.0, 4.0, 5.0],
        -180.0,
        ChestModelTexture::Normal,
        ChestModelHalf::Single,
    )
    .with_block_entity(|s| {
        s.chest_openness = 0.5;
        s.block_entity_crumbling = Some(test_block_entity_crumbling([3.0, 4.0, 5.0]));
    })
    .with(|s| s.light_coords = (4_u32 << 4) | (12_u32 << 20));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    // bottom + lid + lock: 3 boxes -> 18 faces / 72 vertices / 108 indices,
    // all in the backface-culled cutout bucket (vanilla `entityCutoutCull`).
    assert_eq!(meshes.cutout_cull.cutout_faces, 18);
    assert_eq!(meshes.cutout_cull.vertices.len(), 72);
    assert_eq!(meshes.cutout_cull.indices.len(), 108);
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes
        .cutout_cull
        .vertices
        .iter()
        .all(|vertex| vertex.light == instance.render_state.shader_light()));

    // Vanilla's `ModelFeatureRenderer` duplicates the already-posed chest model into the
    // crumbling consumer. The duplicate keeps world positions/normals and projects UVs from the
    // position local to the block entity origin.
    assert_eq!(meshes.crumbling.indices, meshes.cutout_cull.indices);
    assert_eq!(meshes.crumbling.vertices.len(), 72);
    for (source, crumbling) in meshes
        .cutout_cull
        .vertices
        .iter()
        .zip(&meshes.crumbling.vertices)
    {
        assert_eq!(crumbling.position, source.position);
        assert_eq!(crumbling.normal, source.normal);
        assert_eq!(crumbling.uv_rect_min, [0.25, 0.375]);
        assert_eq!(crumbling.uv_rect_size, [0.125, 0.125]);
        assert_eq!(
            crumbling.decal_uv,
            crate::sheeted_decal::sheeted_decal_uv(
                [
                    source.position[0] - 3.0,
                    source.position[1] - 4.0,
                    source.position[2] - 5.0,
                ],
                source.normal,
                1.0,
            )
        );
    }
}
