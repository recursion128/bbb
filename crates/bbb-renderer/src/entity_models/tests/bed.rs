use super::*;

use crate::entity_models::colored::bed_model_root_transform;
use crate::entity_models::model::{EntityModel, ModelCube};
use crate::entity_models::model_layers::{
    BedModel, BED_FOOT_LEFT_LEG_CUBE, BED_FOOT_LEFT_LEG_POSE, BED_FOOT_MAIN_CUBE,
    BED_FOOT_RIGHT_LEG_CUBE, BED_FOOT_RIGHT_LEG_POSE, BED_HEAD_LEFT_LEG_CUBE,
    BED_HEAD_LEFT_LEG_POSE, BED_HEAD_MAIN_CUBE, BED_HEAD_RIGHT_LEG_CUBE, BED_HEAD_RIGHT_LEG_POSE,
    MODEL_LAYER_BED_FOOT, MODEL_LAYER_BED_HEAD,
};
use glam::Vec3;
use std::f32::consts::{FRAC_PI_2, PI};

fn bed_cube(min: [f32; 3], size: [f32; 3], tex: [f32; 2], visible_faces: u8) -> ModelCube {
    ModelCube::new(min, size, BED_WOOL, size, tex, false).with_visible_faces(visible_faces)
}

fn bed_instance(position: [f32; 3], body_rot: f32, part: BedModelPart) -> EntityModelInstance {
    EntityModelInstance::bed(-1, position, body_rot, EntityDyeColor::Red, part)
}

#[test]
fn bed_cubes_match_vanilla_26_1_model_layers() {
    // Vanilla 26.1 `BedRenderer.createHeadLayer` (atlas 64×64): `main` texOffs(0,0) box
    // (0,0,0)+(16,16,6) with `allOfEnumExcept(Direction.UP)` (the head/foot seam face);
    // `left_leg` texOffs(50,6) box (0,6,0)+(3,3,3) at rotation(π/2, 0, π/2); `right_leg`
    // texOffs(50,18) box (-16,6,0)+(3,3,3) at rotation(π/2, 0, π), legs
    // `allOfEnumExcept(Direction.DOWN)`.
    let all_but_up = MODEL_CUBE_FACES_ALL & !MODEL_CUBE_FACE_UP;
    let all_but_down = MODEL_CUBE_FACES_ALL & !MODEL_CUBE_FACE_DOWN;
    assert_eq!(
        BED_HEAD_MAIN_CUBE,
        bed_cube([0.0, 0.0, 0.0], [16.0, 16.0, 6.0], [0.0, 0.0], all_but_up)
    );
    assert_eq!(
        BED_HEAD_LEFT_LEG_CUBE,
        bed_cube([0.0, 6.0, 0.0], [3.0, 3.0, 3.0], [50.0, 6.0], all_but_down)
    );
    assert_eq!(
        BED_HEAD_RIGHT_LEG_CUBE,
        bed_cube(
            [-16.0, 6.0, 0.0],
            [3.0, 3.0, 3.0],
            [50.0, 18.0],
            all_but_down
        )
    );
    assert_eq!(BED_HEAD_LEFT_LEG_POSE.rotation, [FRAC_PI_2, 0.0, FRAC_PI_2]);
    assert_eq!(BED_HEAD_RIGHT_LEG_POSE.rotation, [FRAC_PI_2, 0.0, PI]);
    // `createFootLayer`: `main` texOffs(0,22) with `allOfEnumExcept(Direction.DOWN)` (the foot
    // side of the same seam plane); `left_leg` texOffs(50,0) box (0,6,-16)+(3,3,3) at
    // rotation(π/2, 0, 0); `right_leg` texOffs(50,12) box (-16,6,-16)+(3,3,3) at
    // rotation(π/2, 0, 3π/2).
    assert_eq!(
        BED_FOOT_MAIN_CUBE,
        bed_cube(
            [0.0, 0.0, 0.0],
            [16.0, 16.0, 6.0],
            [0.0, 22.0],
            all_but_down
        )
    );
    assert_eq!(
        BED_FOOT_LEFT_LEG_CUBE,
        bed_cube(
            [0.0, 6.0, -16.0],
            [3.0, 3.0, 3.0],
            [50.0, 0.0],
            all_but_down
        )
    );
    assert_eq!(
        BED_FOOT_RIGHT_LEG_CUBE,
        bed_cube(
            [-16.0, 6.0, -16.0],
            [3.0, 3.0, 3.0],
            [50.0, 12.0],
            all_but_down
        )
    );
    assert_eq!(BED_FOOT_LEFT_LEG_POSE.rotation, [FRAC_PI_2, 0.0, 0.0]);
    assert_eq!(
        BED_FOOT_RIGHT_LEG_POSE.rotation,
        [FRAC_PI_2, 0.0, PI * 3.0 / 2.0]
    );
    // Every leg pose is a pure `PartPose.rotation` (zero offset).
    for pose in [
        BED_HEAD_LEFT_LEG_POSE,
        BED_HEAD_RIGHT_LEG_POSE,
        BED_FOOT_LEFT_LEG_POSE,
        BED_FOOT_RIGHT_LEG_POSE,
    ] {
        assert_eq!(pose.offset, [0.0, 0.0, 0.0]);
    }
    assert_eq!(BED_RED_TEXTURE_REF.size, [64, 64]);
}

#[test]
fn bed_transform_matches_vanilla_model_transform() {
    // Vanilla `BedRenderer.createModelTransform`: translation(0, 0.5625, 0) · Rx(90°) ·
    // rotateAround(Rz(180 + facing.toYRot()), 0.5, 0.5, 0.5). For SOUTH (toYRot = 0,
    // body_rot = 180) a model point (x, y, z) lands at (1-x, 0.5625-z, 1-y).
    let south = bed_model_root_transform(bed_instance([2.0, 3.0, 4.0], 180.0, BedModelPart::Head));
    let corner = south.transform_point3(Vec3::ZERO);
    assert!((corner - Vec3::new(3.0, 3.5625, 5.0)).length() < 1e-5);
    // The mattress underside corner (0, 0, 0.375) rests at world y = 0.1875 over the block.
    let underside = south.transform_point3(Vec3::new(0.0, 0.0, 0.375));
    assert!((underside - Vec3::new(3.0, 3.1875, 5.0)).length() < 1e-5);
    // NORTH: toYRot = 180 -> body_rot = 360 (identity spin): (x, y, z) -> (x, 0.5625-z, y).
    let north = bed_model_root_transform(bed_instance([0.0, 0.0, 0.0], 360.0, BedModelPart::Foot));
    assert!((north.transform_point3(Vec3::ZERO) - Vec3::new(0.0, 0.5625, 0.0)).length() < 1e-5);
    assert!(
        (north.transform_point3(Vec3::new(1.0, 1.0, 0.375)) - Vec3::new(1.0, 0.1875, 1.0)).length()
            < 1e-5
    );
    // WEST: toYRot = 90 -> body_rot = 270: (x, y, z) -> (y, 0.5625-z, 1-x).
    let west = bed_model_root_transform(bed_instance([0.0, 0.0, 0.0], 270.0, BedModelPart::Head));
    assert!((west.transform_point3(Vec3::ZERO) - Vec3::new(0.0, 0.5625, 1.0)).length() < 1e-5);
    // EAST: toYRot = 270 -> body_rot = 450: (x, y, z) -> (1-y, 0.5625-z, x).
    let east = bed_model_root_transform(bed_instance([0.0, 0.0, 0.0], 450.0, BedModelPart::Head));
    assert!((east.transform_point3(Vec3::ZERO) - Vec3::new(1.0, 0.5625, 0.0)).length() < 1e-5);
    // No `scale(-1, -1, 1)` entity flip: the transform is orientation-preserving.
    assert!(south.determinant() > 0.0);
}

#[test]
fn bed_model_keys_and_texture_refs_match_vanilla_selection() {
    let kind = |color, part| EntityModelKind::Bed { color, part };
    assert_eq!(
        kind(EntityDyeColor::Red, BedModelPart::Head).model_key(),
        "bed_head"
    );
    assert_eq!(
        kind(EntityDyeColor::Lime, BedModelPart::Foot).model_key(),
        "bed_foot"
    );
    // Sheets.getBedSprite(color) = `entity/bed/<DyeColor.getName()>`, in DyeColor id order.
    let expected_paths = [
        "textures/entity/bed/white.png",
        "textures/entity/bed/orange.png",
        "textures/entity/bed/magenta.png",
        "textures/entity/bed/light_blue.png",
        "textures/entity/bed/yellow.png",
        "textures/entity/bed/lime.png",
        "textures/entity/bed/pink.png",
        "textures/entity/bed/gray.png",
        "textures/entity/bed/light_gray.png",
        "textures/entity/bed/cyan.png",
        "textures/entity/bed/purple.png",
        "textures/entity/bed/blue.png",
        "textures/entity/bed/brown.png",
        "textures/entity/bed/green.png",
        "textures/entity/bed/red.png",
        "textures/entity/bed/black.png",
    ];
    for id in 0..16 {
        let color = EntityDyeColor::from_vanilla_id(id);
        let texture = kind(color, BedModelPart::Head)
            .vanilla_texture_ref()
            .unwrap();
        assert_eq!(texture.path, expected_paths[id as usize]);
        assert_eq!(texture.size, [64, 64]);
        // Both halves share the one bed sprite.
        assert_eq!(
            kind(color, BedModelPart::Foot).vanilla_texture_ref(),
            Some(texture)
        );
    }
    assert_eq!(bed_entity_texture_refs().len(), 16);
    for texture in bed_entity_texture_refs() {
        assert!(entity_model_texture_refs().contains(texture));
        assert_eq!(texture.size, [64, 64]);
    }
}

#[test]
fn bed_layer_passes_match_vanilla_renderer() {
    let passes = bed_textured_layer_passes(EntityDyeColor::Cyan, BedModelPart::Head);
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::BedBase);
    // Vanilla `BedRenderer` builds both piece models with `RenderTypes::entitySolid`.
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntitySolid
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entitySolid");
    assert_eq!(passes[0].model_layer, MODEL_LAYER_BED_HEAD);
    assert_eq!(passes[0].texture, BED_CYAN_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
    assert_eq!(
        bed_textured_layer_passes(EntityDyeColor::Cyan, BedModelPart::Foot)[0].model_layer,
        MODEL_LAYER_BED_FOOT
    );
}

#[test]
fn bed_textured_mesh_bakes_visible_faces_into_the_cutout_cull_bucket() {
    let images: Vec<EntityModelTextureImage> = bed_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = bed_instance([3.0, 4.0, 5.0], 180.0, BedModelPart::Head)
        .with_light_coords((4_u32 << 4) | (12_u32 << 20));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.texture, BED_RED_TEXTURE_REF);
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntitySolid);
    assert_eq!(submit.transform, bed_model_root_transform(instance));
    assert_eq!(submit.light, instance.render_state.shader_light());
    // main + 2 legs, each with one vanilla-hidden face: 3 boxes × 5 faces = 15 faces /
    // 60 vertices / 90 indices, all in the backface-culled cutout bucket (`entitySolid`).
    assert_eq!(meshes.cutout_cull.cutout_faces, 15);
    assert_eq!(meshes.cutout_cull.vertices.len(), 60);
    assert_eq!(meshes.cutout_cull.indices.len(), 90);
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes
        .cutout_cull
        .vertices
        .iter()
        .all(|vertex| vertex.light == submit.light));
}

#[test]
fn bed_model_has_no_animation() {
    let mut model = BedModel::new(BedModelPart::Head);
    model.prepare(&bed_instance([0.0, 64.0, 0.0], 180.0, BedModelPart::Head));
    assert_eq!(
        model.root().try_child("main").unwrap().pose.rotation,
        [0.0; 3]
    );
    assert_eq!(
        model.root().try_child("left_leg").unwrap().pose.rotation,
        BED_HEAD_LEFT_LEG_POSE.rotation
    );
    assert_eq!(
        model.root().try_child("right_leg").unwrap().pose.rotation,
        BED_HEAD_RIGHT_LEG_POSE.rotation
    );
}
