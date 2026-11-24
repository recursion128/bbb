use super::*;

use crate::entity_models::model_layers::{book_model, book_state_openness};
use crate::entity_models::transforms::{
    enchanting_table_book_model_root_transform, lectern_book_model_root_transform,
};
use glam::{Mat4, Vec3, Vec4};
use std::f32::consts::{FRAC_PI_2, PI};

fn enchanting_book_instance(
    position: [f32; 3],
    y_rot: f32,
    progress: f32,
    open: f32,
    page_flip_1: f32,
    page_flip_2: f32,
    float_y: f32,
) -> EntityModelInstance {
    EntityModelInstance::enchanting_book(-1, position, y_rot).with_block_entity(|s| {
        s.book_progress = progress;
        s.book_open = open;
        s.book_page_flip_1 = page_flip_1;
        s.book_page_flip_2 = page_flip_2;
        s.book_float_y = float_y;
    })
}

#[test]
fn book_state_openness_matches_vanilla_for_animation() {
    // Vanilla `BookModel.State.forAnimation`: (sin(progress·0.02)·0.1 + 1.25)·openness.
    // progress 0 -> 1.25·open.
    assert!((book_state_openness(0.0, 1.0) - 1.25).abs() < 1e-6);
    // The lectern's fixed state: forAnimation(0, .., .., 1.2) -> 1.25·1.2 = 1.5.
    assert!((book_state_openness(0.0, 1.2) - 1.5).abs() < 1e-6);
    // At progress·0.02 = π/2 the sin term peaks: (1·0.1 + 1.25)·1 = 1.35.
    let progress = FRAC_PI_2 / 0.02;
    assert!((book_state_openness(progress, 1.0) - 1.35).abs() < 1e-5);
}

#[test]
fn book_setup_anim_poses_covers_and_pages_like_vanilla() {
    // progress 0, open 1 -> openness 1.25; pageFlip1 0.3, pageFlip2 0.7.
    let instance = enchanting_book_instance([0.0, 0.0, 0.0], 0.0, 0.0, 1.0, 0.3, 0.7, 0.0);
    let model = book_model(
        book_state_openness(
            instance.block_entity.book_progress,
            instance.block_entity.book_open,
        ),
        instance.block_entity.book_page_flip_1,
        instance.block_entity.book_page_flip_2,
    );
    let openness = 1.25_f32;
    let page_x = openness.sin();
    let root = &model;
    // leftLid.yRot = π + openness; rightLid.yRot = -openness.
    assert!((root.try_child("left_lid").unwrap().pose.rotation[1] - (PI + openness)).abs() < 1e-6);
    assert!((root.try_child("right_lid").unwrap().pose.rotation[1] - (-openness)).abs() < 1e-6);
    // Pages: yRot = ±openness, x = sin(openness).
    let left_pages = root.try_child("left_pages").unwrap();
    assert!((left_pages.pose.rotation[1] - openness).abs() < 1e-6);
    assert!((left_pages.pose.offset[0] - page_x).abs() < 1e-6);
    let right_pages = root.try_child("right_pages").unwrap();
    assert!((right_pages.pose.rotation[1] - (-openness)).abs() < 1e-6);
    assert!((right_pages.pose.offset[0] - page_x).abs() < 1e-6);
    // flipPage.yRot = openness − openness·2·pageFlip.
    let flip1 = root.try_child("flip_page1").unwrap();
    assert!((flip1.pose.rotation[1] - (openness - openness * 2.0 * 0.3)).abs() < 1e-6);
    assert!((flip1.pose.offset[0] - page_x).abs() < 1e-6);
    let flip2 = root.try_child("flip_page2").unwrap();
    assert!((flip2.pose.rotation[1] - (openness - openness * 2.0 * 0.7)).abs() < 1e-6);
    // The static spine seam keeps its bind pose.
    assert_eq!(
        root.try_child("seam").unwrap().pose.rotation,
        [0.0, FRAC_PI_2, 0.0]
    );
}

#[test]
fn lectern_fixed_state_poses_the_book_fully_open() {
    // The lectern's BOOK_STATE: forAnimation(0, 0.1, 0.9, 1.2) -> openness 1.5.
    let instance =
        EntityModelInstance::lectern_book(-1, [0.0, 0.0, 0.0], 0.0).with_block_entity(|s| {
            s.book_progress = 0.0;
            s.book_open = 1.2;
            s.book_page_flip_1 = 0.1;
            s.book_page_flip_2 = 0.9;
        });
    let model = book_model(
        book_state_openness(
            instance.block_entity.book_progress,
            instance.block_entity.book_open,
        ),
        instance.block_entity.book_page_flip_1,
        instance.block_entity.book_page_flip_2,
    );
    let openness = 1.5_f32;
    let root = &model;
    assert!((root.try_child("left_lid").unwrap().pose.rotation[1] - (PI + openness)).abs() < 1e-6);
    // flipPage1.yRot = 1.5 − 1.5·2·0.1 = 1.2; flipPage2.yRot = 1.5 − 1.5·2·0.9 = −1.2.
    assert!((root.try_child("flip_page1").unwrap().pose.rotation[1] - 1.2).abs() < 1e-6);
    assert!((root.try_child("flip_page2").unwrap().pose.rotation[1] - (-1.2)).abs() < 1e-6);
}

#[test]
fn enchanting_transform_hovers_and_tips_the_book() {
    // pos (10, 5, -3), yaw 0, float_y 0.1: origin lands at the pivot
    // (10.5, 5.85, -2.5), and Rz(80°) tips a local +y unit.
    let instance = enchanting_book_instance([10.0, 5.0, -3.0], 0.0, 0.0, 1.0, 0.0, 0.0, 0.1);
    let transform = enchanting_table_book_model_root_transform(&instance);
    let origin = transform * Vec4::new(0.0, 0.0, 0.0, 1.0);
    assert!((origin.truncate() - Vec3::new(10.5, 5.85, -2.5)).length() < 1e-5);
    // Rz(80°)·[0,1,0] = [-sin80°, cos80°, 0]; then + pivot.
    let up = transform * Vec4::new(0.0, 1.0, 0.0, 1.0);
    let angle = 80.0_f32.to_radians();
    let expected = Vec3::new(10.5 - angle.sin(), 5.85 + angle.cos(), -2.5);
    assert!((up.truncate() - expected).length() < 1e-5);
    // The lerped yaw rides body_rot (degrees).
    assert_eq!(instance.render_state.body_rot, 0.0);
}

#[test]
fn lectern_transform_matches_vanilla_product() {
    // Vanilla: T(pos)·T(0.5,1.0625,0.5)·Ry(-yaw)·Rz(67.5°)·T(0,-0.125,0).
    let instance = EntityModelInstance::lectern_book(-1, [2.0, 3.0, 4.0], 90.0);
    let transform = lectern_book_model_root_transform(&instance);
    let expected = Mat4::from_translation(Vec3::new(2.0, 3.0, 4.0))
        * Mat4::from_translation(Vec3::new(0.5, 1.0625, 0.5))
        * Mat4::from_rotation_y(-90.0_f32.to_radians())
        * Mat4::from_rotation_z(67.5_f32.to_radians())
        * Mat4::from_translation(Vec3::new(0.0, -0.125, 0.0));
    assert_eq!(transform, expected);
    // The origin lands on the pivot regardless of the rotations.
    let origin = transform * Vec4::new(0.0, 0.0, 0.0, 1.0);
    // Ry(-90)·Rz(67.5)·T(0,-0.125,0)·origin = Ry(-90)·Rz(67.5)·[0,-0.125,0].
    let local = Mat4::from_rotation_y(-90.0_f32.to_radians())
        * Mat4::from_rotation_z(67.5_f32.to_radians())
        * Vec4::new(0.0, -0.125, 0.0, 1.0);
    let expected_origin = Vec3::new(2.5, 4.0625, 4.5) + local.truncate();
    assert!((origin.truncate() - expected_origin).length() < 1e-5);
}

#[test]
fn book_layer_passes_match_vanilla_renderer() {
    let passes = book_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    // Vanilla `BookModel`'s constructor picks `RenderTypes::entitySolid`.
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntitySolid
    );
    assert_eq!(passes[0].texture, BOOK_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
}

#[test]
fn book_textured_mesh_bakes_seven_boxes_into_the_solid_bucket() {
    let images: Vec<EntityModelTextureImage> = [BOOK_TEXTURE_REF]
        .iter()
        .map(|texture| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![7; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = enchanting_book_instance([3.0, 4.0, 5.0], 0.0, 0.0, 1.0, 0.1, 0.9, 0.1)
        .with(|s| s.light_coords = (4_u32 << 4) | (12_u32 << 20));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    // 7 boxes (two covers, spine seam, two page halves, two flip pages) -> 42
    // faces / 168 vertices / 252 indices, all in the backface-culled solid bucket.
    assert_eq!(meshes.solid.cutout_faces, 42);
    assert_eq!(meshes.solid.vertices.len(), 168);
    assert_eq!(meshes.solid.indices.len(), 252);
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes
        .solid
        .vertices
        .iter()
        .all(|vertex| vertex.light == instance.render_state.shader_light()));
}
