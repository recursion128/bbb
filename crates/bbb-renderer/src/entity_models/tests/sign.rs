use super::*;

use crate::entity_models::colored::sign_model_root_transform;
use crate::entity_models::model::{EntityModel, ModelCube};
use crate::entity_models::model_layers::{
    SignModel, HANGING_SIGN_BOARD_CUBE, HANGING_SIGN_CHAIN_1_CUBE, HANGING_SIGN_CHAIN_2_CUBE,
    HANGING_SIGN_PLANK_CUBE, HANGING_SIGN_V_CHAINS_CUBE, MODEL_LAYER_HANGING_SIGN_CEILING,
    MODEL_LAYER_HANGING_SIGN_CEILING_MIDDLE, MODEL_LAYER_HANGING_SIGN_WALL,
    MODEL_LAYER_SIGN_STANDING, MODEL_LAYER_SIGN_WALL, SIGN_BOARD_CUBE, SIGN_STICK_CUBE,
};
use crate::entity_models::textured::sign_textured_layer_passes;
use glam::Vec3;
use std::f32::consts::FRAC_PI_4;

fn sign_cube(min: [f32; 3], size: [f32; 3], tex: [f32; 2]) -> ModelCube {
    ModelCube::new(min, size, SIGN_WOOD, size, tex, false)
}

fn sign_instance(
    position: [f32; 3],
    body_rot: f32,
    wood: SignModelWood,
    attachment: SignModelAttachment,
) -> EntityModelInstance {
    EntityModelInstance::sign(-1, position, body_rot, wood, attachment)
}

#[test]
fn sign_cubes_match_vanilla_26_1_model_layers() {
    // Vanilla 26.1 `StandingSignRenderer.createSignLayer` (atlas 64×32): the
    // `sign` board texOffs(0,0) box (-12,-14,-1)+(24,12,2); the standing-only
    // `stick` texOffs(0,14) box (-1,-2,-1)+(2,14,2).
    assert_eq!(
        SIGN_BOARD_CUBE,
        sign_cube([-12.0, -14.0, -1.0], [24.0, 12.0, 2.0], [0.0, 0.0])
    );
    assert_eq!(
        SIGN_STICK_CUBE,
        sign_cube([-1.0, -2.0, -1.0], [2.0, 14.0, 2.0], [0.0, 14.0])
    );
    // Vanilla `HangingSignRenderer.createHangingSignLayer`: `board`
    // texOffs(0,12) box (-7,0,-1)+(14,10,2); WALL `plank` texOffs(0,0) box
    // (-8,-6,-2)+(16,2,4); the chain planes texOffs(0,6)/(6,6) box
    // (-1.5,0,0)+(3,6,0); CEILING_MIDDLE `vChains` texOffs(14,6) box
    // (-6,-6,0)+(12,6,0).
    assert_eq!(
        HANGING_SIGN_BOARD_CUBE,
        sign_cube([-7.0, 0.0, -1.0], [14.0, 10.0, 2.0], [0.0, 12.0])
    );
    assert_eq!(
        HANGING_SIGN_PLANK_CUBE,
        sign_cube([-8.0, -6.0, -2.0], [16.0, 2.0, 4.0], [0.0, 0.0])
    );
    assert_eq!(
        HANGING_SIGN_CHAIN_1_CUBE,
        sign_cube([-1.5, 0.0, 0.0], [3.0, 6.0, 0.0], [0.0, 6.0])
    );
    assert_eq!(
        HANGING_SIGN_CHAIN_2_CUBE,
        sign_cube([-1.5, 0.0, 0.0], [3.0, 6.0, 0.0], [6.0, 6.0])
    );
    assert_eq!(
        HANGING_SIGN_V_CHAINS_CUBE,
        sign_cube([-6.0, -6.0, 0.0], [12.0, 6.0, 0.0], [14.0, 6.0])
    );
    assert_eq!(SIGN_OAK_TEXTURE_REF.size, [64, 32]);
    assert_eq!(HANGING_SIGN_OAK_TEXTURE_REF.size, [64, 32]);
}

#[test]
fn sign_model_parts_match_vanilla_attachments() {
    // GROUND: sign + stick; WALL: sign only.
    let standing = SignModel::new(SignModelAttachment::Standing);
    assert!(standing.root().try_child("sign").is_some());
    assert!(standing.root().try_child("stick").is_some());
    let wall = SignModel::new(SignModelAttachment::Wall);
    assert!(wall.root().try_child("sign").is_some());
    assert!(wall.root().try_child("stick").is_none());

    // CEILING: board + the four angled chains (offset (±5,-6,0), yRot ∓π/4).
    let ceiling = SignModel::new(SignModelAttachment::HangingCeiling);
    assert!(ceiling.root().try_child("board").is_some());
    assert!(ceiling.root().try_child("plank").is_none());
    assert!(ceiling.root().try_child("vChains").is_none());
    let chains = ceiling.root().try_child("normalChains").unwrap();
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
    let middle = SignModel::new(SignModelAttachment::HangingCeilingMiddle);
    assert!(middle.root().try_child("board").is_some());
    assert!(middle.root().try_child("vChains").is_some());
    assert!(middle.root().try_child("normalChains").is_none());

    // WALL hanging: board + plank + angled chains.
    let hanging_wall = SignModel::new(SignModelAttachment::HangingWall);
    assert!(hanging_wall.root().try_child("board").is_some());
    assert!(hanging_wall.root().try_child("plank").is_some());
    assert!(hanging_wall.root().try_child("normalChains").is_some());
    assert!(hanging_wall.root().try_child("vChains").is_none());
}

#[test]
fn standing_sign_root_transform_matches_vanilla_body_transformation() {
    // Vanilla `StandingSignRenderer.bodyTransformation`: translate(0.5,0.5,0.5)
    // · Ry(-angle) · scale(2/3, -2/3, -2/3). ROTATION=0 -> angle 0.
    let transform = sign_model_root_transform(
        sign_instance(
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
        sign_instance(
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
        sign_instance(
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
        sign_instance(
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
        sign_instance(
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
fn sign_model_keys_and_texture_refs_match_vanilla_selection() {
    let kind = |wood, attachment| EntityModelKind::Sign { wood, attachment };
    assert_eq!(
        kind(SignModelWood::Oak, SignModelAttachment::Standing).model_key(),
        "sign_standing"
    );
    assert_eq!(
        kind(SignModelWood::Oak, SignModelAttachment::Wall).model_key(),
        "sign_wall"
    );
    assert_eq!(
        kind(SignModelWood::Oak, SignModelAttachment::HangingCeiling).model_key(),
        "hanging_sign_ceiling"
    );
    assert_eq!(
        kind(
            SignModelWood::Oak,
            SignModelAttachment::HangingCeilingMiddle
        )
        .model_key(),
        "hanging_sign_ceiling_middle"
    );
    assert_eq!(
        kind(SignModelWood::Oak, SignModelAttachment::HangingWall).model_key(),
        "hanging_sign_wall"
    );
    // Sheets.getSignSprite / getHangingSignSprite: entity/signs/<wood> vs
    // entity/signs/hanging/<wood>, table-driven over all twelve woods.
    let woods = [
        (SignModelWood::Oak, "oak"),
        (SignModelWood::Spruce, "spruce"),
        (SignModelWood::Birch, "birch"),
        (SignModelWood::Acacia, "acacia"),
        (SignModelWood::Cherry, "cherry"),
        (SignModelWood::Jungle, "jungle"),
        (SignModelWood::DarkOak, "dark_oak"),
        (SignModelWood::PaleOak, "pale_oak"),
        (SignModelWood::Crimson, "crimson"),
        (SignModelWood::Warped, "warped"),
        (SignModelWood::Mangrove, "mangrove"),
        (SignModelWood::Bamboo, "bamboo"),
    ];
    for (wood, name) in woods {
        for attachment in [SignModelAttachment::Standing, SignModelAttachment::Wall] {
            let texture = kind(wood, attachment).vanilla_texture_ref().unwrap();
            assert_eq!(texture.path, format!("textures/entity/signs/{name}.png"));
            assert_eq!(texture.size, [64, 32]);
        }
        for attachment in [
            SignModelAttachment::HangingCeiling,
            SignModelAttachment::HangingCeilingMiddle,
            SignModelAttachment::HangingWall,
        ] {
            let texture = kind(wood, attachment).vanilla_texture_ref().unwrap();
            assert_eq!(
                texture.path,
                format!("textures/entity/signs/hanging/{name}.png")
            );
        }
    }
    assert_eq!(sign_entity_texture_refs().len(), 24);
    for texture in sign_entity_texture_refs() {
        assert!(entity_model_texture_refs().contains(texture));
        assert_eq!(texture.size, [64, 32]);
    }
}

#[test]
fn sign_layer_passes_match_vanilla_renderer() {
    let passes = sign_textured_layer_passes(SignModelWood::Cherry, SignModelAttachment::Standing);
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::SignBase);
    // Vanilla `createSignModel` builds `Model.Simple(_, RenderTypes::entityCutout)`.
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].model_layer, MODEL_LAYER_SIGN_STANDING);
    assert_eq!(passes[0].texture, SIGN_CHERRY_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
    let layer =
        |attachment| sign_textured_layer_passes(SignModelWood::Oak, attachment)[0].model_layer;
    assert_eq!(layer(SignModelAttachment::Wall), MODEL_LAYER_SIGN_WALL);
    assert_eq!(
        layer(SignModelAttachment::HangingCeiling),
        MODEL_LAYER_HANGING_SIGN_CEILING
    );
    assert_eq!(
        layer(SignModelAttachment::HangingCeilingMiddle),
        MODEL_LAYER_HANGING_SIGN_CEILING_MIDDLE
    );
    assert_eq!(
        layer(SignModelAttachment::HangingWall),
        MODEL_LAYER_HANGING_SIGN_WALL
    );
}

#[test]
fn sign_textured_mesh_bakes_boxes_into_the_cutout_bucket() {
    let images: Vec<EntityModelTextureImage> = sign_entity_texture_refs()
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
    .with_light_coords((4_u32 << 4) | (12_u32 << 20));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.texture, SIGN_OAK_TEXTURE_REF);
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(
        submit.transform,
        sign_model_root_transform(instance, SignModelAttachment::Standing)
    );
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
    assert_eq!(meshes.submissions.len(), 1);
    assert_eq!(
        meshes.submissions[0].texture,
        HANGING_SIGN_BAMBOO_TEXTURE_REF
    );
    assert_eq!(meshes.cutout.cutout_faces, 36);
}
