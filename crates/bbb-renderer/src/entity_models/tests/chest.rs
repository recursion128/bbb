use super::*;

use crate::entity_models::colored::chest_model_root_transform;
use crate::entity_models::model::{EntityModel, ModelCube};
use crate::entity_models::model_layers::{
    chest_lid_eased_openness, ChestModel, CHEST_LEFT_BOTTOM_CUBE, CHEST_LEFT_LID_CUBE,
    CHEST_LEFT_LOCK_CUBE, CHEST_LID_POSE, CHEST_RIGHT_BOTTOM_CUBE, CHEST_RIGHT_LID_CUBE,
    CHEST_RIGHT_LOCK_CUBE, CHEST_SINGLE_BOTTOM_CUBE, CHEST_SINGLE_LID_CUBE, CHEST_SINGLE_LOCK_CUBE,
};
use glam::{Mat4, Vec3};
use std::f32::consts::FRAC_PI_2;

fn chest_cube(min: [f32; 3], size: [f32; 3], tex: [f32; 2]) -> ModelCube {
    ModelCube::new(min, size, CHEST_WOOD, size, tex, false)
}

#[test]
fn chest_cubes_match_vanilla_26_1_model_layers() {
    // Vanilla 26.1 `ChestModel` (atlas 64×64). Single (`createSingleBodyLayer`):
    // bottom texOffs(0,19) box (1,0,1)+(14,10,14) at ZERO; lid texOffs(0,0) box
    // (1,0,0)+(14,5,14) and lock texOffs(0,0) box (7,-2,14)+(2,4,1), both at
    // PartPose.offset(0, 9, 1).
    assert_eq!(
        CHEST_SINGLE_BOTTOM_CUBE,
        chest_cube([1.0, 0.0, 1.0], [14.0, 10.0, 14.0], [0.0, 19.0])
    );
    assert_eq!(
        CHEST_SINGLE_LID_CUBE,
        chest_cube([1.0, 0.0, 0.0], [14.0, 5.0, 14.0], [0.0, 0.0])
    );
    assert_eq!(
        CHEST_SINGLE_LOCK_CUBE,
        chest_cube([7.0, -2.0, 14.0], [2.0, 4.0, 1.0], [0.0, 0.0])
    );
    // Right half (`createDoubleBodyRightLayer`): 15-wide boxes spanning x = 1..16,
    // lock box (15,-2,14)+(1,4,1).
    assert_eq!(
        CHEST_RIGHT_BOTTOM_CUBE,
        chest_cube([1.0, 0.0, 1.0], [15.0, 10.0, 14.0], [0.0, 19.0])
    );
    assert_eq!(
        CHEST_RIGHT_LID_CUBE,
        chest_cube([1.0, 0.0, 0.0], [15.0, 5.0, 14.0], [0.0, 0.0])
    );
    assert_eq!(
        CHEST_RIGHT_LOCK_CUBE,
        chest_cube([15.0, -2.0, 14.0], [1.0, 4.0, 1.0], [0.0, 0.0])
    );
    // Left half (`createDoubleBodyLeftLayer`): 15-wide boxes spanning x = 0..15,
    // lock box (0,-2,14)+(1,4,1).
    assert_eq!(
        CHEST_LEFT_BOTTOM_CUBE,
        chest_cube([0.0, 0.0, 1.0], [15.0, 10.0, 14.0], [0.0, 19.0])
    );
    assert_eq!(
        CHEST_LEFT_LID_CUBE,
        chest_cube([0.0, 0.0, 0.0], [15.0, 5.0, 14.0], [0.0, 0.0])
    );
    assert_eq!(
        CHEST_LEFT_LOCK_CUBE,
        chest_cube([0.0, -2.0, 14.0], [1.0, 4.0, 1.0], [0.0, 0.0])
    );
    assert_eq!(CHEST_LID_POSE.offset, [0.0, 9.0, 1.0]);
    assert_eq!(CHEST_LID_POSE.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(CHEST_NORMAL_TEXTURE_REF.size, [64, 64]);
}

#[test]
fn chest_lid_angle_matches_vanilla_easing_and_rotation() {
    // Vanilla `ChestRenderer.submit`: `o = 1 - o; o = 1 - o^3`, then
    // `ChestModel.setupAnim`: `lid.xRot = -(o * π/2)`, `lock.xRot = lid.xRot`.
    assert_eq!(chest_lid_eased_openness(0.0), 0.0);
    assert!((chest_lid_eased_openness(0.5) - 0.875).abs() < 1e-6);
    assert_eq!(chest_lid_eased_openness(1.0), 1.0);

    let mut model = ChestModel::new(ChestModelHalf::Single);
    let instance = EntityModelInstance::chest(
        -1,
        [0.0, 64.0, 0.0],
        0.0,
        ChestModelTexture::Normal,
        ChestModelHalf::Single,
    )
    .with_chest_openness(0.5);
    model.prepare(&instance);
    let expected = -(0.875 * FRAC_PI_2);
    let lid_rot = model.root().try_child("lid").unwrap().pose.rotation;
    let lock_rot = model.root().try_child("lock").unwrap().pose.rotation;
    assert!((lid_rot[0] - expected).abs() < 1e-6);
    assert_eq!(lock_rot[0], lid_rot[0]);
    assert_eq!(lid_rot[1], 0.0);
    assert_eq!(lid_rot[2], 0.0);
    // The bottom never rotates, and a closed chest rests at zero.
    assert_eq!(
        model.root().try_child("bottom").unwrap().pose.rotation,
        [0.0, 0.0, 0.0]
    );
    let mut closed = ChestModel::new(ChestModelHalf::Single);
    closed.prepare(&instance.with_chest_openness(0.0));
    assert_eq!(
        closed.root().try_child("lid").unwrap().pose.rotation,
        [0.0, 0.0, 0.0]
    );
}

#[test]
fn chest_facing_rotation_matches_vanilla_rotation_around_block_centre() {
    // Vanilla `ChestRenderer.createModelTransformation`: rotationAround(
    // Axis.YP.rotationDegrees(-facing.toYRot()), 0.5, 0.0, 0.5), no entity flip.
    // NORTH: toYRot = 180 -> body_rot = -180; the model-space min corner (0,0,0)
    // maps to the opposite corner (1,0,1) of the block.
    let north = chest_model_root_transform(EntityModelInstance::chest(
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
    let east = chest_model_root_transform(EntityModelInstance::chest(
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
    let south = chest_model_root_transform(EntityModelInstance::chest(
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
fn chest_model_keys_and_texture_refs_match_vanilla_selection() {
    let kind = |texture, half| EntityModelKind::Chest { texture, half };
    assert_eq!(
        kind(ChestModelTexture::Normal, ChestModelHalf::Single).model_key(),
        "chest"
    );
    assert_eq!(
        kind(ChestModelTexture::Trapped, ChestModelHalf::Left).model_key(),
        "chest_left"
    );
    assert_eq!(
        kind(ChestModelTexture::Normal, ChestModelHalf::Right).model_key(),
        "chest_right"
    );
    // Sheets.chooseSprite: `<prefix>` / `<prefix>_left` / `<prefix>_right`, ender
    // always the single `ender` sprite.
    assert_eq!(
        kind(ChestModelTexture::Normal, ChestModelHalf::Single).vanilla_texture_ref(),
        Some(CHEST_NORMAL_TEXTURE_REF)
    );
    assert_eq!(
        kind(ChestModelTexture::Normal, ChestModelHalf::Left).vanilla_texture_ref(),
        Some(CHEST_NORMAL_LEFT_TEXTURE_REF)
    );
    assert_eq!(
        kind(ChestModelTexture::Normal, ChestModelHalf::Right).vanilla_texture_ref(),
        Some(CHEST_NORMAL_RIGHT_TEXTURE_REF)
    );
    assert_eq!(
        kind(ChestModelTexture::Trapped, ChestModelHalf::Single).vanilla_texture_ref(),
        Some(CHEST_TRAPPED_TEXTURE_REF)
    );
    assert_eq!(
        kind(ChestModelTexture::Ender, ChestModelHalf::Single).vanilla_texture_ref(),
        Some(CHEST_ENDER_TEXTURE_REF)
    );
    assert_eq!(
        kind(ChestModelTexture::Ender, ChestModelHalf::Left).vanilla_texture_ref(),
        Some(CHEST_ENDER_TEXTURE_REF)
    );
    assert_eq!(
        kind(ChestModelTexture::Copper, ChestModelHalf::Single).vanilla_texture_ref(),
        Some(CHEST_COPPER_TEXTURE_REF)
    );
    assert_eq!(
        kind(ChestModelTexture::CopperExposed, ChestModelHalf::Left).vanilla_texture_ref(),
        Some(CHEST_COPPER_EXPOSED_LEFT_TEXTURE_REF)
    );
    assert_eq!(
        kind(ChestModelTexture::CopperWeathered, ChestModelHalf::Right).vanilla_texture_ref(),
        Some(CHEST_COPPER_WEATHERED_RIGHT_TEXTURE_REF)
    );
    assert_eq!(
        kind(ChestModelTexture::CopperOxidized, ChestModelHalf::Single).vanilla_texture_ref(),
        Some(CHEST_COPPER_OXIDIZED_TEXTURE_REF)
    );
    assert_eq!(chest_entity_texture_refs().len(), 19);
    for texture in chest_entity_texture_refs() {
        assert!(entity_model_texture_refs().contains(texture));
        assert_eq!(texture.size, [64, 64]);
    }
}

#[test]
fn chest_layer_passes_match_vanilla_renderer() {
    let passes = chest_textured_layer_passes(ChestModelTexture::Trapped, ChestModelHalf::Left);
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::ChestBase);
    // Vanilla `ChestModel`'s constructor picks `RenderTypes::entityCutoutCull`.
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutoutCull
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutoutCull");
    assert_eq!(passes[0].model_layer, MODEL_LAYER_CHEST_LEFT);
    assert_eq!(passes[0].texture, CHEST_TRAPPED_LEFT_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
    assert_eq!(
        chest_textured_layer_passes(ChestModelTexture::Normal, ChestModelHalf::Single)[0]
            .model_layer,
        MODEL_LAYER_CHEST
    );
    assert_eq!(
        chest_textured_layer_passes(ChestModelTexture::Normal, ChestModelHalf::Right)[0]
            .model_layer,
        MODEL_LAYER_CHEST_RIGHT
    );
}

#[test]
fn chest_textured_mesh_bakes_three_boxes_into_the_cutout_bucket() {
    let images: Vec<EntityModelTextureImage> = chest_entity_texture_refs()
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
    .with_chest_openness(0.5)
    .with_light_coords((4_u32 << 4) | (12_u32 << 20));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.texture, CHEST_NORMAL_TEXTURE_REF);
    assert_eq!(
        submit.render_type,
        EntityModelLayerRenderType::EntityCutoutCull
    );
    assert_eq!(submit.transform, chest_model_root_transform(instance));
    assert_eq!(submit.light, instance.render_state.shader_light());
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
        .all(|vertex| vertex.light == submit.light));
}
