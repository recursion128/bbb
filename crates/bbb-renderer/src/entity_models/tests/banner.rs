use super::*;

use crate::entity_models::colored::banner_model_root_transform;
use crate::entity_models::model::EntityModel;
use crate::entity_models::model_layers::{
    banner_entity_texture_refs, banner_flag_swing_x_rot, BannerModel, BANNER_BASE_TEXTURE_REF,
    BANNER_FLAG_CUBE, BANNER_PATTERN_BASE_TEXTURE_REF, BANNER_POLE_CUBE, BANNER_STANDING_BAR_CUBE,
    BANNER_WALL_BAR_CUBE, MODEL_LAYER_STANDING_BANNER, MODEL_LAYER_STANDING_BANNER_FLAG,
    MODEL_LAYER_WALL_BANNER, MODEL_LAYER_WALL_BANNER_FLAG, STANDING_BANNER_FLAG_POSE,
    WALL_BANNER_FLAG_POSE,
};
use crate::DynamicEntityTextureImage;
use glam::Vec3;
use std::f32::consts::PI;

fn banner_instance(
    position: [f32; 3],
    y_rot: f32,
    wall: bool,
    base_color: EntityDyeColor,
    layers: [Option<BannerPatternLayer>; 16],
) -> EntityModelInstance {
    EntityModelInstance::banner(-1, position, y_rot, wall, base_color, layers)
}

fn two_layers() -> [Option<BannerPatternLayer>; 16] {
    let mut layers = [None; 16];
    layers[0] = Some(BannerPatternLayer {
        texture: BannerPatternTexture::Builtin(BannerPatternKind::StripeTop),
        color: EntityDyeColor::Purple,
    });
    layers[1] = Some(BannerPatternLayer {
        texture: BannerPatternTexture::Builtin(BannerPatternKind::Creeper),
        color: EntityDyeColor::White,
    });
    layers
}

#[test]
fn banner_cubes_match_vanilla_26_1_model_layers() {
    // Vanilla `BannerModel.createBodyLayer` (atlas 64×64): the standing pole is a 2×42×2 box at
    // (-1, -42, -1) texOffs(44, 0); the bar is a 20×2×2 box texOffs(0, 42) at (-10, -44, -1)
    // standing / (-10, -20.5, 9.5) wall.
    assert_eq!(BANNER_POLE_CUBE.min, [-1.0, -42.0, -1.0]);
    assert_eq!(BANNER_POLE_CUBE.size, [2.0, 42.0, 2.0]);
    assert_eq!(BANNER_POLE_CUBE.uv_size, [2.0, 42.0, 2.0]);
    assert_eq!(BANNER_POLE_CUBE.tex, [44.0, 0.0]);
    assert_eq!(BANNER_STANDING_BAR_CUBE.min, [-10.0, -44.0, -1.0]);
    assert_eq!(BANNER_STANDING_BAR_CUBE.size, [20.0, 2.0, 2.0]);
    assert_eq!(BANNER_STANDING_BAR_CUBE.tex, [0.0, 42.0]);
    assert_eq!(BANNER_WALL_BAR_CUBE.min, [-10.0, -20.5, 9.5]);
    assert_eq!(BANNER_WALL_BAR_CUBE.size, [20.0, 2.0, 2.0]);
    assert_eq!(BANNER_WALL_BAR_CUBE.tex, [0.0, 42.0]);
    // `BannerFlagModel.createFlagLayer`: the flag is a 20×40×1 box at (-10, 0, -2) texOffs(0, 0),
    // pivoted at offset(0, -44, 0) standing / (0, -20.5, 10.5) wall.
    assert_eq!(BANNER_FLAG_CUBE.min, [-10.0, 0.0, -2.0]);
    assert_eq!(BANNER_FLAG_CUBE.size, [20.0, 40.0, 1.0]);
    assert_eq!(BANNER_FLAG_CUBE.uv_size, [20.0, 40.0, 1.0]);
    assert_eq!(BANNER_FLAG_CUBE.tex, [0.0, 0.0]);
    assert_eq!(STANDING_BANNER_FLAG_POSE.offset, [0.0, -44.0, 0.0]);
    assert_eq!(WALL_BANNER_FLAG_POSE.offset, [0.0, -20.5, 10.5]);
    // The standing tree carries pole/bar/flag; the wall form has no pole
    // (`BannerModel.createBodyLayer(standing)` only adds it when standing).
    let standing = BannerModel::new(false);
    for name in ["pole", "bar", "flag"] {
        assert!(standing.root().try_child(name).is_some(), "part {name}");
    }
    let wall = BannerModel::new(true);
    assert!(wall.root().try_child("pole").is_none());
    assert!(wall.root().try_child("bar").is_some());
    assert!(wall.root().try_child("flag").is_some());
}

#[test]
fn banner_flag_swing_matches_vanilla_setup_anim() {
    // `BannerFlagModel.setupAnim`: xRot = (-0.0125 + 0.01·cos(2π·phase))·π.
    assert!((banner_flag_swing_x_rot(0.0) - (-0.0025 * PI)).abs() < 1e-7);
    assert!((banner_flag_swing_x_rot(0.25) - (-0.0125 * PI)).abs() < 1e-7);
    assert!((banner_flag_swing_x_rot(0.5) - (-0.0225 * PI)).abs() < 1e-7);
    assert!((banner_flag_swing_x_rot(1.0) - (-0.0025 * PI)).abs() < 1e-7);
    // `prepare` writes the swing onto the flag part only.
    let mut model = BannerModel::new(false);
    let instance = banner_instance(
        [0.0, 0.0, 0.0],
        0.0,
        false,
        EntityDyeColor::White,
        [None; 16],
    )
    .with_banner_flag_phase(0.25);
    model.prepare(&instance);
    let flag = model.root().try_child("flag").unwrap();
    assert!((flag.pose.rotation[0] - (-0.0125 * PI)).abs() < 1e-7);
    assert_eq!(flag.pose.offset, [0.0, -44.0, 0.0]);
    let bar = model.root().try_child("bar").unwrap();
    assert_eq!(bar.pose.rotation, [0.0, 0.0, 0.0]);
}

#[test]
fn banner_transform_matches_vanilla_model_transformation() {
    // Vanilla `modelTransformation(angle)`: translate(0.5, 0, 0.5) · Ry(-angle) ·
    // scale(⅔, -⅔, -⅔), with `body_rot` carrying the pre-negated `-angle`.
    // Identity yaw (angle 0): (x, y, z) -> (0.5 + ⅔x, -⅔y, 0.5 - ⅔z).
    let identity = banner_model_root_transform(banner_instance(
        [2.0, 3.0, 4.0],
        0.0,
        false,
        EntityDyeColor::White,
        [None; 16],
    ));
    assert!(
        (identity.transform_point3(Vec3::new(1.0, 2.0, 3.0))
            - Vec3::new(2.0 + 0.5 + 2.0 / 3.0, 3.0 - 4.0 / 3.0, 4.0 + 0.5 - 2.0))
        .length()
            < 1e-5
    );
    // The pole top (0, -42, 0) lands 28 blocks up the block column centre.
    assert!(
        (identity.transform_point3(Vec3::new(0.0, -42.0, 0.0)) - Vec3::new(2.5, 31.0, 4.5))
            .length()
            < 1e-5
    );
    // ROTATION 4 (east, convertToDegrees 90 -> body_rot -90): Ry(-90°) maps the scaled
    // (⅔·1, -⅔·2, -⅔·3) onto (2, ·, ⅔) before the centre offset.
    let east = banner_model_root_transform(banner_instance(
        [0.0, 0.0, 0.0],
        -90.0,
        false,
        EntityDyeColor::White,
        [None; 16],
    ));
    assert!(
        (east.transform_point3(Vec3::new(1.0, 2.0, 3.0))
            - Vec3::new(2.5, -4.0 / 3.0, 0.5 + 2.0 / 3.0))
        .length()
            < 1e-5
    );
    // scale(⅔, -⅔, -⅔) has a positive determinant: orientation-preserving, no
    // winding flip.
    assert!(identity.determinant() > 0.0);
}

#[test]
fn banner_layer_passes_match_vanilla_renderer() {
    // Vanilla `submitBanner`: the frame and flag submit `entitySolid` over `banner_base`, then
    // `submitPatterns` layers the `base` mask (tinted by the base color) and each pattern sheet
    // (tinted by its dye) over the same flag geometry through `RenderTypes::bannerPattern`
    // (translucent blend).
    let passes = banner_textured_layer_passes(false, false, EntityDyeColor::Lime, &two_layers());
    assert_eq!(passes.len(), 5);
    let frame = &passes[0];
    assert_eq!(frame.kind, EntityModelLayerKind::BannerBase);
    assert_eq!(frame.render_type, EntityModelLayerRenderType::EntitySolid);
    assert_eq!(frame.render_type.vanilla_name(), "entitySolid");
    assert_eq!(frame.model_layer, MODEL_LAYER_STANDING_BANNER);
    assert_eq!(frame.texture, BANNER_BASE_TEXTURE_REF);
    assert_eq!(
        frame.visibility,
        EntityModelLayerVisibility::RetainedParts(&["pole", "bar"])
    );
    assert_eq!(frame.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((frame.order, frame.submit_sequence), (0, 0));
    let flag = &passes[1];
    assert_eq!(flag.kind, EntityModelLayerKind::BannerFlag);
    assert_eq!(flag.render_type, EntityModelLayerRenderType::EntitySolid);
    assert_eq!(flag.model_layer, MODEL_LAYER_STANDING_BANNER_FLAG);
    assert_eq!(flag.texture, BANNER_BASE_TEXTURE_REF);
    assert_eq!(
        flag.visibility,
        EntityModelLayerVisibility::RetainedParts(&["flag"])
    );
    assert_eq!(flag.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((flag.order, flag.submit_sequence), (0, 1));
    // The pattern stack in submit order: base mask, then the stored layers.
    let expectations: [(&str, [f32; 4]); 3] = [
        (
            "textures/entity/banner/base.png",
            EntityDyeColor::Lime.texture_diffuse_color(),
        ),
        (
            "textures/entity/banner/stripe_top.png",
            EntityDyeColor::Purple.texture_diffuse_color(),
        ),
        (
            "textures/entity/banner/creeper.png",
            EntityDyeColor::White.texture_diffuse_color(),
        ),
    ];
    for (index, (texture_path, tint)) in expectations.iter().enumerate() {
        let pass = &passes[index + 2];
        assert_eq!(pass.kind, EntityModelLayerKind::BannerPattern);
        assert_eq!(pass.render_type, EntityModelLayerRenderType::BannerPattern);
        assert_eq!(pass.model_layer, MODEL_LAYER_STANDING_BANNER_FLAG);
        assert_eq!(pass.texture.path, *texture_path);
        assert_eq!(
            pass.visibility,
            EntityModelLayerVisibility::RetainedParts(&["flag"])
        );
        assert_eq!(pass.tint, *tint);
        assert_eq!((pass.order, pass.submit_sequence), (0, (index + 2) as u32));
    }
    // The wall form swaps the frame parts (no pole) and the wall layer ids; a plain banner still
    // submits the base-color mask.
    let wall_passes = banner_textured_layer_passes(true, false, EntityDyeColor::Red, &[None; 16]);
    assert_eq!(wall_passes.len(), 3);
    assert_eq!(wall_passes[0].model_layer, MODEL_LAYER_WALL_BANNER);
    assert_eq!(
        wall_passes[0].visibility,
        EntityModelLayerVisibility::RetainedParts(&["bar"])
    );
    assert_eq!(wall_passes[1].model_layer, MODEL_LAYER_WALL_BANNER_FLAG);
    assert_eq!(wall_passes[2].texture, BANNER_PATTERN_BASE_TEXTURE_REF);
    assert_eq!(
        wall_passes[2].tint,
        EntityDyeColor::Red.texture_diffuse_color()
    );
}

#[test]
fn banner_flag_only_passes_match_gui_result_renderer() {
    let passes = banner_textured_layer_passes(false, true, EntityDyeColor::Red, &two_layers());
    assert_eq!(passes.len(), 4);
    assert!(passes
        .iter()
        .all(|pass| pass.kind != EntityModelLayerKind::BannerBase));
    assert_eq!(passes[0].kind, EntityModelLayerKind::BannerFlag);
    assert_eq!(passes[0].submit_sequence, 0);
    assert_eq!(passes[1].kind, EntityModelLayerKind::BannerPattern);
    assert_eq!(passes[1].texture, BANNER_PATTERN_BASE_TEXTURE_REF);
    assert_eq!(passes[1].submit_sequence, 1);
    assert_eq!(passes[2].submit_sequence, 2);
    assert_eq!(passes[3].submit_sequence, 3);

    let instance = EntityModelInstance::banner_flag(-1, EntityDyeColor::Red, [None; 16]);
    assert_eq!(banner_model_root_transform(instance), glam::Mat4::IDENTITY);
    let mut model = BannerModel::new(false);
    model.prepare(&instance);
    assert_eq!(
        model.root().try_child("flag").unwrap().pose.rotation[0],
        0.0
    );
}

#[test]
fn banner_model_keys_and_texture_refs_match_vanilla_selection() {
    let kind = EntityModelKind::Banner {
        wall: false,
        flag_only: false,
        base_color: EntityDyeColor::Lime,
        layers: [None; 16],
    };
    assert_eq!(kind.model_key(), "banner");
    assert_eq!(kind.vanilla_texture_ref(), Some(BANNER_BASE_TEXTURE_REF));
    assert_eq!(
        BANNER_BASE_TEXTURE_REF.path,
        "textures/entity/banner/banner_base.png"
    );
    assert_eq!(BANNER_BASE_TEXTURE_REF.size, [64, 64]);
    assert_eq!(
        BANNER_PATTERN_BASE_TEXTURE_REF.path,
        "textures/entity/banner/base.png"
    );
    // banner_base + base + 42 patterns, all registered into the shared entity atlas.
    assert_eq!(banner_entity_texture_refs().len(), 44);
    for texture in banner_entity_texture_refs() {
        assert!(entity_model_texture_refs().contains(texture));
    }
}

#[test]
fn banner_textured_mesh_bakes_frame_flag_and_pattern_layers() {
    let images: Vec<EntityModelTextureImage> = banner_entity_texture_refs()
        .iter()
        .map(|texture| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![7; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let mut layers = [None; 16];
    layers[0] = Some(BannerPatternLayer {
        texture: BannerPatternTexture::Builtin(BannerPatternKind::Creeper),
        color: EntityDyeColor::Black,
    });
    let instance = banner_instance([3.0, 4.0, 5.0], 0.0, false, EntityDyeColor::Lime, layers)
        .with_banner_flag_phase(0.25)
        .with_block_entity_crumbling(Some(test_block_entity_crumbling([3.0, 4.0, 5.0])))
        .with_has_red_overlay(true)
        .with_light_coords((4_u32 << 4) | (12_u32 << 20));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    // Four submissions: frame + flag over `banner_base`, then the base-color mask and the one
    // pattern layer over the flag geometry.
    assert_eq!(meshes.submissions.len(), 4);
    let frame = meshes.submissions[0];
    assert_eq!(frame.render_type, EntityModelLayerRenderType::EntitySolid);
    assert_eq!(frame.texture, BANNER_BASE_TEXTURE_REF);
    assert_eq!(frame.tint, [1.0, 1.0, 1.0, 1.0]);
    let flag = meshes.submissions[1];
    assert_eq!(flag.texture, BANNER_BASE_TEXTURE_REF);
    let base_mask = meshes.submissions[2];
    assert_eq!(
        base_mask.render_type,
        EntityModelLayerRenderType::BannerPattern
    );
    assert_eq!(base_mask.texture, BANNER_PATTERN_BASE_TEXTURE_REF);
    assert_eq!(base_mask.tint, EntityDyeColor::Lime.texture_diffuse_color());
    assert_eq!(base_mask.overlay, ENTITY_VERTEX_NO_OVERLAY);
    let pattern = meshes.submissions[3];
    assert_eq!(
        pattern.render_type,
        EntityModelLayerRenderType::BannerPattern
    );
    assert_eq!(pattern.texture.path, "textures/entity/banner/creeper.png");
    assert_eq!(pattern.tint, EntityDyeColor::Black.texture_diffuse_color());
    assert_eq!(pattern.overlay, ENTITY_VERTEX_NO_OVERLAY);
    assert_ne!(flag.overlay, ENTITY_VERTEX_NO_OVERLAY);
    for submission in &meshes.submissions {
        assert_eq!(submission.transform, banner_model_root_transform(instance));
        assert_eq!(submission.light, instance.render_state.shader_light());
    }
    // The frame pass bakes pole + bar (12 faces) and the flag pass the flag box (6 faces) into
    // the backface-culled cutout bucket (`entitySolid`); the two pattern passes re-bake the flag
    // box into the translucent bucket (vanilla `bannerPattern`'s translucent blend).
    assert_eq!(meshes.cutout_cull.cutout_faces, 18);
    assert_eq!(meshes.cutout_cull.vertices.len(), 72);
    assert_eq!(meshes.cutout_cull.indices.len(), 108);
    assert_eq!(meshes.translucent.vertices.len(), 48);
    assert_eq!(meshes.translucent.indices.len(), 72);
    assert!(meshes.cutout.vertices.is_empty());
    // `BannerRenderer` passes the overlay to frame + flag, but vanilla's dedicated
    // `bannerPattern` render type neither samples overlay nor affects crumbling. Pattern geometry
    // must therefore not be duplicated even though it shares the translucent vertex bucket.
    assert!(meshes.submissions[0].affects_crumbling);
    assert!(meshes.submissions[1].affects_crumbling);
    assert!(!meshes.submissions[2].affects_crumbling);
    assert!(!meshes.submissions[3].affects_crumbling);
    assert_eq!(
        meshes.crumbling.indices.len(),
        meshes.cutout_cull.indices.len()
    );
    assert_eq!(
        meshes.crumbling.vertices.len(),
        meshes.cutout_cull.vertices.len()
    );
}

#[test]
fn banner_runtime_pattern_uses_dynamic_atlas_without_replacing_static_layers() {
    let images: Vec<EntityModelTextureImage> = banner_entity_texture_refs()
        .iter()
        .map(|texture| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![7; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let texture = EntityDynamicTexture { handle: 91_002 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: texture.handle,
        size: [64, 64],
        rgba: vec![0xff; 64 * 64 * 4],
    }])
    .unwrap()
    .0;
    let mut layers = [None; 16];
    layers[0] = Some(BannerPatternLayer {
        texture: BannerPatternTexture::Runtime(texture),
        color: EntityDyeColor::Cyan,
    });
    let instance = banner_instance([3.0, 4.0, 5.0], 0.0, false, EntityDyeColor::Lime, layers);

    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[instance],
        &atlas,
        None,
        Some(&dynamic_atlas),
        Some([0.0, 0.0, 0.0]),
        None,
    );

    assert_eq!(meshes.submissions.len(), 4);
    assert_eq!(meshes.submissions[2].dynamic_texture, None);
    assert_eq!(meshes.submissions[3].dynamic_texture, Some(texture));
    assert_eq!(
        meshes.submissions[3].tint,
        EntityDyeColor::Cyan.texture_diffuse_color()
    );
    assert_eq!(meshes.translucent.vertices.len(), 24);
    assert_eq!(meshes.dynamic_player_texture_translucent.vertices.len(), 24);
    assert_eq!(meshes.sorted_main_translucent_draws.len(), 2);
    let EntityModelTranslucentDrawRange::Textured(base_draw) =
        meshes.sorted_main_translucent_draws[0]
    else {
        panic!("expected static base-mask draw");
    };
    let EntityModelTranslucentDrawRange::Textured(pattern_draw) =
        meshes.sorted_main_translucent_draws[1]
    else {
        panic!("expected dynamic pattern draw");
    };
    assert_eq!(base_draw.atlas, EntityModelTexturedDrawAtlas::Static);
    assert_eq!(
        base_draw.render_type,
        EntityModelLayerRenderType::BannerPattern
    );
    assert_eq!(
        pattern_draw.atlas,
        EntityModelTexturedDrawAtlas::DynamicPlayerTexture
    );
    assert_eq!(
        pattern_draw.render_type,
        EntityModelLayerRenderType::BannerPattern
    );
    assert!(base_draw.insertion_index < pattern_draw.insertion_index);
}
