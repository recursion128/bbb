use super::*;

use crate::entity_models::colored::shulker_box_model_root_transform;
use crate::entity_models::model::{EntityModel, ModelCube};
use crate::entity_models::model_layers::{
    shulker_box_lid_pose, ShulkerBoxModel, MODEL_LAYER_SHULKER_BOX, SHULKER_BASE_CUBES,
    SHULKER_LID_CUBES, SHULKER_SHELL_POSE,
};
use glam::Vec3;

fn shulker_box_instance(
    position: [f32; 3],
    color: Option<EntityDyeColor>,
    facing: EntityAttachmentFace,
    progress: f32,
) -> EntityModelInstance {
    EntityModelInstance::shulker_box(-1, position, color, facing)
        .with_shulker_box_progress(progress)
}

#[test]
fn shulker_box_cubes_match_vanilla_26_1_model_layers() {
    // Vanilla 26.1 `ShulkerModel.createBoxLayer` = `createShellMesh` (atlas 64×64): `lid`
    // texOffs(0,0) box (-8,-16,-8)+(16,12,16) and `base` texOffs(0,28) box (-8,-8,-8)+(16,8,16),
    // both at PartPose.offset(0,24,0) — the mob's shell without the head.
    assert_eq!(
        SHULKER_LID_CUBES[0],
        ModelCube::new(
            [-8.0, -16.0, -8.0],
            [16.0, 12.0, 16.0],
            SHULKER_LID_CUBES[0].color,
            [16.0, 12.0, 16.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(
        SHULKER_BASE_CUBES[0],
        ModelCube::new(
            [-8.0, -8.0, -8.0],
            [16.0, 8.0, 16.0],
            SHULKER_BASE_CUBES[0].color,
            [16.0, 8.0, 16.0],
            [0.0, 28.0],
            false,
        )
    );
    assert_eq!(SHULKER_SHELL_POSE.offset, [0.0, 24.0, 0.0]);
    assert_eq!(SHULKER_SHELL_POSE.rotation, [0.0, 0.0, 0.0]);
    // The box tree carries exactly the two shell parts (no mob head).
    let model = ShulkerBoxModel::new();
    assert!(model.root().try_child("lid").is_some());
    assert!(model.root().try_child("base").is_some());
    assert!(model.root().try_child("head").is_none());
}

#[test]
fn shulker_box_lid_pose_matches_vanilla_open_formula() {
    // Vanilla `ShulkerBoxModel.setupAnim`: `lid.setPos(0, 24 - progress·0.5·16, 0)` and
    // `lid.yRot = 270°·progress`, hand-computed at progress 0 / 0.5 / 1:
    // y 24 / 20 / 16, yRot 0 / 135° (3π/4) / 270° (3π/2).
    assert_eq!(shulker_box_lid_pose(0.0), (24.0, 0.0));
    let (half_y, half_rot) = shulker_box_lid_pose(0.5);
    assert_eq!(half_y, 20.0);
    assert!((half_rot - 3.0 * std::f32::consts::FRAC_PI_4).abs() < 1e-6);
    let (full_y, full_rot) = shulker_box_lid_pose(1.0);
    assert_eq!(full_y, 16.0);
    assert!((full_rot - 3.0 * std::f32::consts::FRAC_PI_2).abs() < 1e-6);

    // The model applies the pose to the lid part; the base keeps its bind pose.
    let mut model = ShulkerBoxModel::new();
    model.prepare(&shulker_box_instance(
        [0.0, 0.0, 0.0],
        None,
        EntityAttachmentFace::Up,
        0.5,
    ));
    let lid = model.root().try_child("lid").unwrap();
    assert_eq!(lid.pose.offset, [0.0, 20.0, 0.0]);
    assert!((lid.pose.rotation[1] - half_rot).abs() < 1e-6);
    let base = model.root().try_child("base").unwrap();
    assert_eq!(base.pose.offset, [0.0, 24.0, 0.0]);
    assert_eq!(base.pose.rotation, [0.0, 0.0, 0.0]);
    // At progress 0 the lid returns to the closed bind pose.
    model.prepare(&shulker_box_instance(
        [0.0, 0.0, 0.0],
        None,
        EntityAttachmentFace::Up,
        0.0,
    ));
    let lid = model.root().try_child("lid").unwrap();
    assert_eq!(lid.pose.offset, [0.0, 24.0, 0.0]);
    assert_eq!(lid.pose.rotation, [0.0, 0.0, 0.0]);
}

#[test]
fn shulker_box_transform_matches_vanilla_six_way_model_transform() {
    // Vanilla `ShulkerBoxRenderer.createModelTransform`: T(0.5,0.5,0.5) · S(0.9995) ·
    // R(FACING.getRotation()) · S(1,-1,-1) · T(0,-1,0). The shell mesh is Y-down: its bottom
    // centre sits at model (0, 1.5, 0) (the offset(0,24,0) pivot) and the lid top centre at
    // model (0, 0.5, 0). Mapped points carry the 0.9995 anti-z-fight shrink.
    let transform = |facing| {
        shulker_box_model_root_transform(
            shulker_box_instance([0.0, 0.0, 0.0], None, facing, 0.0),
            facing,
        )
    };
    let bottom = Vec3::new(0.0, 1.5, 0.0);
    let lid_top = Vec3::new(0.0, 0.5, 0.0);
    // UP (identity rotation): bottom centre rests on the block floor, lid top at the block top.
    let up = transform(EntityAttachmentFace::Up);
    assert!((up.transform_point3(bottom) - Vec3::new(0.5, 0.000_25, 0.5)).length() < 1e-4);
    assert!((up.transform_point3(lid_top) - Vec3::new(0.5, 0.999_75, 0.5)).length() < 1e-4);
    // DOWN (Rx(180°)): the box hangs from the ceiling, opening downward.
    let down = transform(EntityAttachmentFace::Down);
    assert!((down.transform_point3(bottom) - Vec3::new(0.5, 0.999_75, 0.5)).length() < 1e-4);
    assert!((down.transform_point3(lid_top) - Vec3::new(0.5, 0.000_25, 0.5)).length() < 1e-4);
    // NORTH (rotationXYZ(90°, 0, 180°)): the base presses against the south face (z = 1).
    let north = transform(EntityAttachmentFace::North);
    assert!((north.transform_point3(bottom) - Vec3::new(0.5, 0.5, 0.999_75)).length() < 1e-4);
    assert!((north.transform_point3(lid_top) - Vec3::new(0.5, 0.5, 0.000_25)).length() < 1e-4);
    // EAST (rotationXYZ(90°, 0, -90°)): the base presses against the west face (x = 0).
    let east = transform(EntityAttachmentFace::East);
    assert!((east.transform_point3(bottom) - Vec3::new(0.000_25, 0.5, 0.5)).length() < 1e-4);
    assert!((east.transform_point3(lid_top) - Vec3::new(0.999_75, 0.5, 0.5)).length() < 1e-4);
    // The `scale(1,-1,-1)` double flip keeps the transform orientation-preserving.
    assert!(up.determinant() > 0.0);
    // The block-corner translation rides in front.
    let moved = shulker_box_model_root_transform(
        shulker_box_instance([3.0, 4.0, 5.0], None, EntityAttachmentFace::Up, 0.0),
        EntityAttachmentFace::Up,
    );
    assert!((moved.transform_point3(bottom) - Vec3::new(3.5, 4.000_25, 5.5)).length() < 1e-4);
}

#[test]
fn shulker_box_model_keys_and_texture_refs_match_vanilla_selection() {
    let kind = |color, facing| EntityModelKind::ShulkerBox { color, facing };
    assert_eq!(
        kind(None, EntityAttachmentFace::Up).model_key(),
        "shulker_box"
    );
    // `Sheets.getShulkerBoxSprite(color)` / `DEFAULT_SHULKER_TEXTURE_LOCATION`: the undyed box
    // binds `entity/shulker/shulker`, the sixteen dyed boxes `shulker_<DyeColor.getName()>` —
    // the same sheet family the shulker mob registers, so the box adds no new texture refs.
    assert_eq!(
        kind(None, EntityAttachmentFace::Up)
            .vanilla_texture_ref()
            .unwrap()
            .path,
        "textures/entity/shulker/shulker.png"
    );
    let expected_paths = [
        "textures/entity/shulker/shulker_white.png",
        "textures/entity/shulker/shulker_orange.png",
        "textures/entity/shulker/shulker_magenta.png",
        "textures/entity/shulker/shulker_light_blue.png",
        "textures/entity/shulker/shulker_yellow.png",
        "textures/entity/shulker/shulker_lime.png",
        "textures/entity/shulker/shulker_pink.png",
        "textures/entity/shulker/shulker_gray.png",
        "textures/entity/shulker/shulker_light_gray.png",
        "textures/entity/shulker/shulker_cyan.png",
        "textures/entity/shulker/shulker_purple.png",
        "textures/entity/shulker/shulker_blue.png",
        "textures/entity/shulker/shulker_brown.png",
        "textures/entity/shulker/shulker_green.png",
        "textures/entity/shulker/shulker_red.png",
        "textures/entity/shulker/shulker_black.png",
    ];
    for id in 0..16 {
        let color = EntityDyeColor::from_vanilla_id(id);
        let texture = kind(Some(color), EntityAttachmentFace::Down)
            .vanilla_texture_ref()
            .unwrap();
        assert_eq!(texture.path, expected_paths[id as usize]);
        assert_eq!(texture.size, [64, 64]);
    }
    assert_eq!(shulker_entity_texture_refs().len(), 17);
    for texture in shulker_entity_texture_refs() {
        assert!(entity_model_texture_refs().contains(texture));
    }
}

#[test]
fn shulker_box_layer_passes_match_vanilla_renderer() {
    let passes = shulker_box_textured_layer_passes(Some(EntityDyeColor::Lime));
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::ShulkerBoxBase);
    // Vanilla `ShulkerBoxRenderer.ShulkerBoxModel`'s constructor picks
    // `RenderTypes::entityCutout` (unlike the mob's `entityCutoutZOffset`).
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].model_layer, MODEL_LAYER_SHULKER_BOX);
    assert_eq!(
        passes[0].texture.path,
        "textures/entity/shulker/shulker_lime.png"
    );
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
    assert_eq!(
        shulker_box_textured_layer_passes(None)[0].texture.path,
        "textures/entity/shulker/shulker.png"
    );
}

#[test]
fn shulker_box_textured_mesh_bakes_two_boxes_into_the_cutout_bucket() {
    let images: Vec<EntityModelTextureImage> = shulker_entity_texture_refs()
        .iter()
        .map(|texture| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![7; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let facing = EntityAttachmentFace::North;
    let instance = shulker_box_instance([3.0, 4.0, 5.0], None, facing, 0.5)
        .with_light_coords((4_u32 << 4) | (12_u32 << 20));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.texture.path, "textures/entity/shulker/shulker.png");
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(
        submit.transform,
        shulker_box_model_root_transform(instance, facing)
    );
    assert_eq!(submit.light, instance.render_state.shader_light());
    // lid + base: 2 boxes -> 12 faces / 48 vertices / 72 indices, all in the unculled cutout
    // bucket (vanilla `entityCutout`).
    assert_eq!(meshes.cutout.cutout_faces, 12);
    assert_eq!(meshes.cutout.vertices.len(), 48);
    assert_eq!(meshes.cutout.indices.len(), 72);
    assert!(meshes.cutout_cull.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == submit.light));
}
