use super::*;

use crate::entity_models::model_layers::sign_model_layer;
use crate::entity_models::textured::sign_textured_layer_passes;
use crate::entity_models::transforms::sign_model_root_transform;
use glam::Vec3;
use std::f32::consts::FRAC_PI_4;

fn sign_instance(
    position: [f32; 3],
    body_rot: f32,
    wood: SignModelWood,
    attachment: SignModelAttachment,
) -> EntityModelInstance {
    EntityModelInstance::sign(-1, position, body_rot, wood, attachment)
}

#[test]
fn sign_model_parts_match_vanilla_attachments() {
    // GROUND: sign + stick; WALL: sign only.
    let standing = sign_model_layer(SignModelAttachment::Standing).bake();
    assert!(standing.try_child("sign").is_some());
    assert!(standing.try_child("stick").is_some());
    let wall = sign_model_layer(SignModelAttachment::Wall).bake();
    assert!(wall.try_child("sign").is_some());
    assert!(wall.try_child("stick").is_none());

    // CEILING: board + the four angled chains (offset (±5,-6,0), yRot ∓π/4).
    let ceiling = sign_model_layer(SignModelAttachment::HangingCeiling).bake();
    assert!(ceiling.try_child("board").is_some());
    assert!(ceiling.try_child("plank").is_none());
    assert!(ceiling.try_child("vChains").is_none());
    let chains = ceiling.try_child("normalChains").unwrap();
    for (name, x, y_rot) in [
        ("chainL1", -5.0, -FRAC_PI_4),
        ("chainL2", -5.0, FRAC_PI_4),
        ("chainR1", 5.0, -FRAC_PI_4),
        ("chainR2", 5.0, FRAC_PI_4),
    ] {
        let chain = chains.try_child(name).unwrap();
        assert_eq!(chain.pose.offset, [x, -6.0, 0.0]);
        assert_eq!(chain.pose.rotation, [0.0, y_rot, 0.0]);
    }

    // CEILING_MIDDLE: board + the straight vChains, no angled chains.
    let middle = sign_model_layer(SignModelAttachment::HangingCeilingMiddle).bake();
    assert!(middle.try_child("board").is_some());
    assert!(middle.try_child("vChains").is_some());
    assert!(middle.try_child("normalChains").is_none());

    // WALL hanging: board + plank + angled chains.
    let hanging_wall = sign_model_layer(SignModelAttachment::HangingWall).bake();
    assert!(hanging_wall.try_child("board").is_some());
    assert!(hanging_wall.try_child("plank").is_some());
    assert!(hanging_wall.try_child("normalChains").is_some());
    assert!(hanging_wall.try_child("vChains").is_none());
}

#[test]
fn standing_sign_root_transform_matches_vanilla_body_transformation() {
    // Vanilla `StandingSignRenderer.bodyTransformation`: translate(0.5,0.5,0.5)
    // · Ry(-angle) · scale(2/3, -2/3, -2/3). ROTATION=0 -> angle 0.
    let transform = sign_model_root_transform(
        &sign_instance(
            [2.0, 3.0, 4.0],
            0.0,
            SignModelWood::Oak,
            SignModelAttachment::Standing,
        ),
        SignModelAttachment::Standing,
    );
    // The model origin lands at the block centre.
    let origin = transform.transform_point3(Vec3::ZERO);
    assert!((origin - Vec3::new(2.5, 3.5, 4.5)).length() < 1e-5);
    // The board top edge (model y = -14, already 1/16-scaled by the emitter)
    // maps to world y = 0.5 + 14/16 * 2/3 = 1.0833 above the block base.
    let board_top = transform.transform_point3(Vec3::new(0.0, -14.0 / 16.0, 0.0));
    assert!((board_top.y - (3.0 + 0.5 + 14.0 / 16.0 * (2.0 / 3.0))).abs() < 1e-5);
    // The stick bottom (model y = 12) maps to the ground: 0.5 - 12/16 * 2/3 = 0.
    let stick_bottom = transform.transform_point3(Vec3::new(0.0, 12.0 / 16.0, 0.0));
    assert!((stick_bottom.y - 3.0).abs() < 1e-5);
    // The y/z double flip keeps the determinant positive (no winding flip).
    assert!(transform.determinant() > 0.0);
    // ROTATION=4 -> 90° -> body_rot -90: model -z (board front) turns from
    // south (+z) to west (-x).
    let rotated = sign_model_root_transform(
        &sign_instance(
            [0.0, 0.0, 0.0],
            -90.0,
            SignModelWood::Oak,
            SignModelAttachment::Standing,
        ),
        SignModelAttachment::Standing,
    );
    let front = rotated.transform_vector3(Vec3::new(0.0, 0.0, -1.0));
    assert!((front - Vec3::new(-2.0 / 3.0, 0.0, 0.0)).length() < 1e-5);
}

#[test]
fn wall_sign_root_transform_applies_vanilla_wall_offset() {
    // Vanilla WALL attachment: baseTransformation appends
    // translate(0, -0.3125, -0.4375) before the scale flip. FACING=south ->
    // toYRot 0 -> body_rot 0.
    let transform = sign_model_root_transform(
        &sign_instance(
            [0.0, 0.0, 0.0],
            0.0,
            SignModelWood::Spruce,
            SignModelAttachment::Wall,
        ),
        SignModelAttachment::Wall,
    );
    let origin = transform.transform_point3(Vec3::ZERO);
    assert!((origin - Vec3::new(0.5, 0.5 - 0.3125, 0.5 - 0.4375)).length() < 1e-5);
    // FACING=north (toYRot 180, body_rot -180): the offset flips to the other
    // wall face.
    let north = sign_model_root_transform(
        &sign_instance(
            [0.0, 0.0, 0.0],
            -180.0,
            SignModelWood::Spruce,
            SignModelAttachment::Wall,
        ),
        SignModelAttachment::Wall,
    );
    let origin = north.transform_point3(Vec3::ZERO);
    assert!((origin - Vec3::new(0.5, 0.1875, 0.5 + 0.4375)).length() < 1e-5);
}

#[test]
fn hanging_sign_root_transform_matches_vanilla_body_transformation() {
    // Vanilla `HangingSignRenderer.bodyTransformation`:
    // translation(0.5, 0.9375, 0.5) · Ry(-angle) · translate(0, -0.3125, 0) ·
    // scale(1, -1, -1).
    let transform = sign_model_root_transform(
        &sign_instance(
            [1.0, 2.0, 3.0],
            0.0,
            SignModelWood::Bamboo,
            SignModelAttachment::HangingCeiling,
        ),
        SignModelAttachment::HangingCeiling,
    );
    let origin = transform.transform_point3(Vec3::ZERO);
    assert!((origin - Vec3::new(1.5, 2.0 + 0.9375 - 0.3125, 3.5)).length() < 1e-5);
    // The board bottom edge (model y = 10) hangs at the block base:
    // 0.625 - 10/16 = 0.
    let board_bottom = transform.transform_point3(Vec3::new(0.0, 10.0 / 16.0, 0.0));
    assert!((board_bottom.y - 2.0).abs() < 1e-5);
    // Full-scale flip: unit model x stays unit world x.
    let x = transform.transform_vector3(Vec3::X);
    assert!((x - Vec3::X).length() < 1e-5);
    assert!(transform.determinant() > 0.0);
}

#[test]
fn sign_texture_catalog_contains_all_sheets() {
    for texture in &texture_refs_matching(|path| path.starts_with("textures/entity/signs/")) {
        assert!(entity_model_texture_refs().contains(texture));
        assert_eq!(texture.size, [64, 32]);
    }
}

#[test]
fn sign_layer_passes_match_vanilla_renderer() {
    let passes = sign_textured_layer_passes(SignModelWood::Cherry, SignModelAttachment::Standing);
    assert_eq!(passes.len(), 1);
    // Vanilla `createSignModel` builds `Model.Simple(_, RenderTypes::entityCutout)`.
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].texture, SIGN_CHERRY_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
}

#[test]
fn sign_textured_mesh_bakes_boxes_into_the_cutout_bucket() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/signs/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = sign_instance(
        [3.0, 4.0, 5.0],
        -67.5,
        SignModelWood::Oak,
        SignModelAttachment::Standing,
    )
    .with(|s| s.light_coords = (4_u32 << 4) | (12_u32 << 20));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    // board + stick: 2 boxes -> 12 faces / 48 vertices / 72 indices, all in
    // the non-culled cutout bucket (vanilla `entityCutout`).
    assert_eq!(meshes.cutout.cutout_faces, 12);
    assert_eq!(meshes.cutout.vertices.len(), 48);
    assert_eq!(meshes.cutout.indices.len(), 72);
    assert!(meshes.cutout_cull.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    // A hanging wall sign bakes board + plank + four chain planes (6 boxes).
    let hanging = sign_instance(
        [0.0, 0.0, 0.0],
        0.0,
        SignModelWood::Bamboo,
        SignModelAttachment::HangingWall,
    );
    let meshes = entity_model_textured_meshes(&[hanging], &atlas);

    assert_eq!(meshes.cutout.cutout_faces, 36);
}
