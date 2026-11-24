use super::*;

use crate::entity_models::model_layers::{
    banner_flag_swing_x_rot, banner_model, BANNER_BASE_TEXTURE_REF, BANNER_PATTERN_BASE_TEXTURE_REF,
};
use crate::entity_models::transforms::banner_model_root_transform;
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
fn banner_flag_swing_matches_vanilla_setup_anim() {
    // `BannerFlagModel.setupAnim`: xRot = (-0.0125 + 0.01·cos(2π·phase))·π.
    assert!((banner_flag_swing_x_rot(0.0) - (-0.0025 * PI)).abs() < 1e-7);
    assert!((banner_flag_swing_x_rot(0.25) - (-0.0125 * PI)).abs() < 1e-7);
    assert!((banner_flag_swing_x_rot(0.5) - (-0.0225 * PI)).abs() < 1e-7);
    assert!((banner_flag_swing_x_rot(1.0) - (-0.0025 * PI)).abs() < 1e-7);
    // `prepare` writes the swing onto the flag part only.
    let instance = banner_instance(
        [0.0, 0.0, 0.0],
        0.0,
        false,
        EntityDyeColor::White,
        [None; 16],
    )
    .with_block_entity(|s| s.banner_flag_phase = 0.25);
    let model = banner_model(false, &instance);
    let flag = model.try_child("flag").unwrap();
    assert!((flag.pose.rotation[0] - (-0.0125 * PI)).abs() < 1e-7);
    assert_eq!(flag.pose.offset, [0.0, -44.0, 0.0]);
    let bar = model.try_child("bar").unwrap();
    assert_eq!(bar.pose.rotation, [0.0, 0.0, 0.0]);
}

#[test]
fn banner_transform_matches_vanilla_model_transformation() {
    // Vanilla `modelTransformation(angle)`: translate(0.5, 0, 0.5) · Ry(-angle) ·
    // scale(⅔, -⅔, -⅔), with `body_rot` carrying the pre-negated `-angle`.
    // Identity yaw (angle 0): (x, y, z) -> (0.5 + ⅔x, -⅔y, 0.5 - ⅔z).
    let identity = banner_model_root_transform(&banner_instance(
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
    let east = banner_model_root_transform(&banner_instance(
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
    assert_eq!(frame.render_type, EntityModelLayerRenderType::EntitySolid);
    assert_eq!(frame.render_type.vanilla_name(), "entitySolid");
    assert_eq!(frame.texture, BANNER_BASE_TEXTURE_REF);
    assert_eq!(
        frame.visibility,
        EntityModelLayerVisibility::RetainedParts(&["pole", "bar"])
    );
    assert_eq!(frame.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(frame.order, 0);
    let flag = &passes[1];
    assert_eq!(flag.render_type, EntityModelLayerRenderType::EntitySolid);
    assert_eq!(flag.texture, BANNER_BASE_TEXTURE_REF);
    assert_eq!(
        flag.visibility,
        EntityModelLayerVisibility::RetainedParts(&["flag"])
    );
    assert_eq!(flag.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(flag.order, 0);
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
        assert_eq!(pass.render_type, EntityModelLayerRenderType::BannerPattern);
        assert_eq!(pass.texture.path, *texture_path);
        assert_eq!(
            pass.visibility,
            EntityModelLayerVisibility::RetainedParts(&["flag"])
        );
        assert_eq!(pass.tint, *tint);
        assert_eq!(pass.order, 0);
    }
    // The wall form swaps the frame parts (no pole) and the wall layer ids; a plain banner still
    // submits the base-color mask.
    let wall_passes = banner_textured_layer_passes(true, false, EntityDyeColor::Red, &[None; 16]);
    assert_eq!(wall_passes.len(), 3);
    assert_eq!(
        wall_passes[0].visibility,
        EntityModelLayerVisibility::RetainedParts(&["bar"])
    );
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
    assert_eq!(
        passes[0].visibility,
        EntityModelLayerVisibility::RetainedParts(&["flag"])
    );
    assert_eq!(passes[1].texture, BANNER_PATTERN_BASE_TEXTURE_REF);

    let instance = EntityModelInstance::banner_flag(-1, EntityDyeColor::Red, [None; 16]);
    assert_eq!(banner_model_root_transform(&instance), glam::Mat4::IDENTITY);
    let model = banner_model(false, &instance);
    assert_eq!(model.try_child("flag").unwrap().pose.rotation[0], 0.0);
}

#[test]
fn banner_texture_refs_match_vanilla_selection() {
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
}

#[test]
fn banner_textured_mesh_bakes_frame_flag_and_pattern_layers() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/banner"))
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
        .with_block_entity(|s| {
            s.banner_flag_phase = 0.25;
            s.block_entity_crumbling = Some(test_block_entity_crumbling([3.0, 4.0, 5.0]));
        })
        .with(|s| {
            s.has_red_overlay = true;
            s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    // Four layers: frame + flag over `banner_base`, then the base-color mask and the one
    // pattern layer over the flag geometry.

    // The frame pass bakes pole + bar (12 faces) and the flag pass the flag box (6 faces) into
    // the dedicated backface-culled solid bucket; the two pattern passes re-bake the flag
    // box into the translucent bucket (vanilla `bannerPattern`'s translucent blend).
    assert_eq!(meshes.solid.cutout_faces, 18);
    assert_eq!(meshes.solid.vertices.len(), 72);
    assert_eq!(meshes.solid.indices.len(), 108);
    assert_eq!(meshes.translucent.vertices.len(), 48);
    assert_eq!(meshes.translucent.indices.len(), 72);
    assert!(meshes.cutout.vertices.is_empty());
    // `BannerRenderer` passes the overlay to frame + flag, but vanilla's dedicated
    // `bannerPattern` render type neither samples overlay nor affects crumbling. Pattern geometry
    // must therefore not be duplicated even though it shares the translucent vertex bucket.
    assert_eq!(meshes.crumbling.indices.len(), meshes.solid.indices.len());
    assert_eq!(meshes.crumbling.vertices.len(), meshes.solid.vertices.len());
}

#[test]
fn banner_runtime_pattern_uses_dynamic_atlas_without_replacing_static_layers() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/banner"))
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
        standalone_sampling: None,
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
    assert!(base_draw.sort_key.insertion_index < pattern_draw.sort_key.insertion_index);
}
