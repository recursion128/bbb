use super::*;
use crate::entity_models::model::EntityModel;

fn blank_texture(texture: EntityModelTextureRef) -> EntityModelTextureImage {
    let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
    EntityModelTextureImage::new(texture, vec![0u8; len])
}

#[test]
fn ender_dragon_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `EnderDragonModel.createBodyLayer` (atlas 256×256): head (+jaw), five neck and twelve
    // tail spine segments, and the body (+wings +legs) — 19 root parts hung off the mesh root. The
    // tree is hand-built by `EnderDragonModel::new()`; the structural assertions walk it by the
    // index child names `StaticModel` used.
    let mut model = EnderDragonModel::new();

    // `head` (offset (0, 20, -62)): six cubes, parenting the jaw at (0, 4, -8).
    let head = model.root_mut().child_mut("0");
    assert_eq!(head.pose.offset, [0.0, 20.0, -62.0]);
    assert_eq!(DRAGON_HEAD_CUBES.len(), 6);
    assert_eq!(DRAGON_HEAD_CUBES[1].size, [16.0, 16.0, 16.0]);
    let jaw = head.child_mut("0");
    assert_eq!(jaw.pose.offset, [0.0, 4.0, -8.0]);
    assert_eq!(DRAGON_JAW_CUBES[0].size, [12.0, 4.0, 16.0]);

    // The five neck segments at `offset(0, 20, -12 - i·10)`, each the 2-cube spine.
    for i in 0..5 {
        let neck = model.root_mut().child_mut(&(1 + i).to_string());
        assert_eq!(neck.pose.offset, [0.0, 20.0, -12.0 - i as f32 * 10.0]);
    }
    assert_eq!(DRAGON_SPINE_CUBES.len(), 2);
    assert_eq!(DRAGON_SPINE_CUBES[0].size, [10.0, 10.0, 10.0]);

    // The twelve tail segments at `offset(0, 10, 60 + i·10)`, each the 2-cube spine.
    for i in 0..12 {
        let tail = model.root_mut().child_mut(&(6 + i).to_string());
        assert_eq!(tail.pose.offset, [0.0, 10.0, 60.0 + i as f32 * 10.0]);
    }

    // `body` (offset (0, 3, 8)): four cubes, parenting two wings and four legs.
    let body = model.root_mut().child_mut("18");
    assert_eq!(body.pose.offset, [0.0, 3.0, 8.0]);
    assert_eq!(DRAGON_BODY_CUBES[0].size, [24.0, 24.0, 64.0]);

    // `left_wing` (offset (12, 2, -6)): the bone plus the membrane, parenting the wing tip.
    let left_wing = body.child_mut("0");
    assert_eq!(left_wing.pose.offset, [12.0, 2.0, -6.0]);
    assert_eq!(DRAGON_LEFT_WING_CUBES[0].size, [56.0, 8.0, 8.0]);
    assert_eq!(DRAGON_LEFT_WING_CUBES[1].size, [56.0, 0.0, 56.0]);
    assert_eq!(left_wing.child_mut("0").pose.offset, [56.0, 0.0, 0.0]);

    // A front leg is a three-segment chain (leg → tip → foot) with the vanilla bind rotations.
    let left_front_leg = model.root_mut().child_mut("18").child_mut("1");
    assert_eq!(left_front_leg.pose.offset, [12.0, 17.0, -6.0]);
    assert_eq!(left_front_leg.pose.rotation, [1.3, 0.0, 0.0]);
    let leg_tip = left_front_leg.child_mut("0");
    assert_eq!(leg_tip.pose.rotation, [-0.5, 0.0, 0.0]);
    let foot = leg_tip.child_mut("0");
    assert_eq!(foot.pose.rotation, [0.75, 0.0, 0.0]);
    assert_eq!(DRAGON_FRONT_FOOT_CUBES[0].size, [8.0, 4.0, 16.0]);

    // The right wing extends -X (vanilla's mirror is true geometry).
    let right_wing = model.root_mut().child_mut("18").child_mut("3");
    assert_eq!(right_wing.pose.offset, [-12.0, 2.0, -6.0]);
    assert_eq!(DRAGON_RIGHT_WING_CUBES[0].min, [-56.0, -4.0, -4.0]);
}

#[test]
fn ender_dragon_mesh_uses_vanilla_body_layer_geometry() {
    // 65 cubes → 390 faces / 1560 vertices / 2340 indices; the body is dark and the wing membranes
    // carry their lighter tint.
    let dragon = entity_model_mesh(&[EntityModelInstance::ender_dragon(
        430,
        [0.0, 64.0, 0.0],
        0.0,
    )]);
    assert_eq!(dragon.opaque_faces, 390);
    assert_eq!(dragon.vertices.len(), 1560);
    assert_eq!(dragon.indices.len(), 2340);
    assert!(dragon
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(DRAGON_BODY, 1.0)));
    assert!(dragon
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(DRAGON_MEMBRANE, 1.0)));
}

#[test]
fn ender_dragon_textured_render_matches_vanilla_renderer() {
    let passes = ender_dragon_textured_layer_passes(0.0);
    // The cutout base body, always-on emissive eyes overlay, and optional healing-beam custom geometry.
    assert_eq!(passes.len(), 3);
    assert_eq!(passes[0].kind, EntityModelLayerKind::EnderDragonBase);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].model_layer, MODEL_LAYER_ENDER_DRAGON);
    assert_eq!(passes[0].texture, ENDER_DRAGON_TEXTURE_REF);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
    assert_eq!(passes[1].kind, EntityModelLayerKind::EnderDragonEyes);
    assert_eq!(passes[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(passes[1].render_type.vanilla_name(), "eyes");
    assert_eq!(passes[1].model_layer, MODEL_LAYER_ENDER_DRAGON);
    assert_eq!(passes[1].texture, ENDER_DRAGON_EYES_TEXTURE_REF);
    assert_eq!(passes[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[1].order, passes[1].submit_sequence), (0, 1));
    assert_eq!(passes[2].kind, EntityModelLayerKind::EnderDragonBeam);
    assert_eq!(
        passes[2].render_type,
        EntityModelLayerRenderType::EndCrystalBeam
    );
    assert_eq!(passes[2].render_type.vanilla_name(), "end_crystal_beam");
    assert_eq!(passes[2].model_layer, "");
    assert_eq!(passes[2].texture, END_CRYSTAL_BEAM_TEXTURE_REF);
    assert_eq!(passes[2].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[2].order, passes[2].submit_sequence), (0, 2));
    assert_eq!(
        EntityModelKind::EnderDragon.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/enderdragon/dragon.png",
            size: [256, 256],
        })
    );
    assert!(entity_model_texture_refs().contains(&ENDER_DRAGON_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ENDER_DRAGON_EXPLODING_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ENDER_DRAGON_EYES_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&END_CRYSTAL_BEAM_TEXTURE_REF));
    assert_eq!(
        ender_dragon_entity_texture_refs(),
        &[
            ENDER_DRAGON_TEXTURE_REF,
            ENDER_DRAGON_EXPLODING_TEXTURE_REF,
            ENDER_DRAGON_EYES_TEXTURE_REF,
            END_CRYSTAL_BEAM_TEXTURE_REF
        ]
    );

    let images: Vec<EntityModelTextureImage> = ender_dragon_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::ender_dragon(900, [0.0, 64.0, 0.0], 0.0);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert_eq!(meshes.submissions.len(), 2);
    let base = meshes.submissions[0];
    assert_eq!(base.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(base.render_type.vanilla_name(), "entityCutout");
    assert_eq!(base.texture, ENDER_DRAGON_TEXTURE_REF);
    assert_eq!(base.dissolve_texture, None);
    assert_eq!(base.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(base.transform, ender_dragon_model_root_transform(instance));
    assert_eq!((base.order, base.submit_sequence), (0, 0));
    let eyes = meshes.submissions[1];
    assert_eq!(eyes.render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(eyes.render_type.vanilla_name(), "eyes");
    assert_eq!(eyes.texture, ENDER_DRAGON_EYES_TEXTURE_REF);
    assert_eq!(eyes.dissolve_texture, None);
    assert_eq!(eyes.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(eyes.transform, base.transform);
    assert_eq!((eyes.order, eyes.submit_sequence), (0, 1));
    let mesh = &meshes.cutout;

    assert!(!mesh.vertices.is_empty());
    assert_eq!(meshes.eyes.vertices.len(), mesh.vertices.len());
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn ender_dragon_dying_body_uses_vanilla_cutout_dissolve_submission() {
    // Vanilla `EnderDragonRenderer.submit`: when `deathTime > 0`, submit the body with
    // `RenderTypes.entityCutoutDissolve(dragon.png, dragon_exploding.png)`, alpha
    // `1 - deathTime / 200`, and `OverlayTexture.NO_OVERLAY`; eyes are still submitted next.
    let passes = ender_dragon_textured_layer_passes(50.0);
    assert_eq!(passes[0].kind, EntityModelLayerKind::EnderDragonBase);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutoutDissolve
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutoutDissolve");
    assert_eq!(passes[0].texture, ENDER_DRAGON_TEXTURE_REF);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 0.75]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
    assert_eq!(passes[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!((passes[1].order, passes[1].submit_sequence), (0, 1));

    let images: Vec<EntityModelTextureImage> = ender_dragon_entity_texture_refs()
        .iter()
        .map(|texture| blank_texture(*texture))
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::ender_dragon(902, [0.0, 64.0, 0.0], 0.0)
        .with_ender_dragon_death_time(50.0)
        .with_light_coords((9_u32 << 4) | (12_u32 << 20))
        .with_has_red_overlay(true);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 2);
    let base = meshes.submissions[0];
    assert_eq!(
        base.render_type,
        EntityModelLayerRenderType::EntityCutoutDissolve
    );
    assert_eq!(base.render_type.vanilla_name(), "entityCutoutDissolve");
    assert_eq!(base.texture, ENDER_DRAGON_TEXTURE_REF);
    assert_eq!(
        base.dissolve_texture,
        Some(ENDER_DRAGON_EXPLODING_TEXTURE_REF)
    );
    assert_eq!(base.tint, [1.0, 1.0, 1.0, 0.75]);
    assert_eq!(base.transform, ender_dragon_model_root_transform(instance));
    assert_eq!((base.order, base.submit_sequence), (0, 0));
    assert_eq!(base.light, instance.render_state.shader_light());
    assert_eq!(base.overlay, [0.0, 10.0]);
    assert_ne!(base.overlay, instance.render_state.overlay_coords());
    // Dying-dragon body folds into the dedicated DISSOLVE bucket (not the plain cutout bucket), so the
    // GPU dissolve pipeline owns it. It carries the submission's tint (alpha `1 - deathTime/200`, forced
    // to 1.0 per-fragment only in the shader), light, and NO_OVERLAY.
    assert!(meshes.cutout.vertices.is_empty());
    assert!(!meshes.dissolve.vertices.is_empty());
    assert!(meshes
        .dissolve
        .vertices
        .iter()
        .all(|vertex| vertex.tint == base.tint
            && vertex.light == base.light
            && vertex.overlay == base.overlay));

    let eyes = meshes.submissions[1];
    assert_eq!(eyes.render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(eyes.texture, ENDER_DRAGON_EYES_TEXTURE_REF);
    assert_eq!(eyes.dissolve_texture, None);
    assert_eq!(eyes.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(eyes.transform, base.transform);
    assert_eq!((eyes.order, eyes.submit_sequence), (0, 1));
    assert_eq!(eyes.light, base.light);
    assert_eq!(eyes.overlay, [0.0, 10.0]);
}

#[test]
fn ender_dragon_dying_records_vanilla_death_rays_custom_geometry() {
    // Vanilla `EnderDragonRenderer.submit`: after body and eyes, a dying dragon translates the
    // current model pose by `(0, -1, -2)` and calls `submitRays` twice: `dragonRays()` then
    // `dragonRaysDepth()`. The geometry is seeded with 432 and uses no texture, lightmap, or overlay.
    let images: Vec<EntityModelTextureImage> = ender_dragon_entity_texture_refs()
        .iter()
        .map(|texture| blank_texture(*texture))
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let position = [4.0, 80.0, -3.0];
    let instance = EntityModelInstance::ender_dragon(904, position, 30.0)
        .with_ender_dragon_death_time(50.0)
        .with_light_coords((9_u32 << 4) | (12_u32 << 20))
        .with_has_red_overlay(true);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 2);
    assert_eq!(meshes.custom_submissions.len(), 2);
    let rays = meshes.custom_submissions[0];
    assert_eq!(rays.render_type, EntityModelLayerRenderType::DragonRays);
    assert_eq!(rays.render_type.vanilla_name(), "dragonRays");
    assert_eq!((rays.order, rays.submit_sequence), (0, 2));
    let rays_depth = meshes.custom_submissions[1];
    assert_eq!(
        rays_depth.render_type,
        EntityModelLayerRenderType::DragonRaysDepth
    );
    assert_eq!(rays_depth.render_type.vanilla_name(), "dragonRaysDepth");
    assert_eq!((rays_depth.order, rays_depth.submit_sequence), (0, 3));

    let expected_transform = ender_dragon_model_root_transform(instance)
        * Mat4::from_translation(Vec3::new(0.0, -1.0, -2.0));
    assert_eq!(rays.transform, expected_transform);
    assert_eq!(rays_depth.transform, expected_transform);
    assert_close3(
        meshes.dragon_rays.vertices[0].position,
        expected_transform.transform_point3(Vec3::ZERO).to_array(),
    );

    let death_time = 50.0_f32 / 200.0;
    let ray_count = ((death_time + death_time * death_time) / 2.0 * 60.0).floor() as usize;
    assert_eq!(ray_count, 9);
    assert_eq!(meshes.dragon_rays.vertices.len(), ray_count * 9);
    assert_eq!(meshes.dragon_rays.indices.len(), ray_count * 9);
    assert_eq!(
        meshes.dragon_rays_depth.vertices.len(),
        meshes.dragon_rays.vertices.len()
    );
    assert_eq!(meshes.dragon_rays_depth.indices, meshes.dragon_rays.indices);
    for (color, expected) in [
        (meshes.dragon_rays.vertices[0].color, [1.0, 1.0, 1.0, 1.0]),
        (meshes.dragon_rays.vertices[1].color, [1.0, 0.0, 1.0, 1.0]),
        (meshes.dragon_rays.vertices[2].color, [1.0, 0.0, 1.0, 1.0]),
    ] {
        assert_eq!(color, expected);
    }
    assert!(meshes
        .dragon_rays_depth
        .vertices
        .iter()
        .zip(&meshes.dragon_rays.vertices)
        .all(|(depth, color)| depth.position == color.position && depth.color == color.color));
}

#[test]
fn ender_dragon_death_rays_survive_missing_dragon_textures() {
    // `dragonRays` / `dragonRaysDepth` are no-texture custom geometry, so missing body, eyes, or
    // dissolve atlas entries suppress only folded textured model geometry, not the ray submissions.
    let (atlas, _) =
        build_entity_model_texture_atlas(&[blank_texture(END_CRYSTAL_BEAM_TEXTURE_REF)]).unwrap();
    let instance = EntityModelInstance::ender_dragon(905, [0.0, 70.0, 0.0], 0.0)
        .with_ender_dragon_death_time(50.0);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 2);
    assert_eq!(meshes.custom_submissions.len(), 2);
    assert_eq!(
        meshes.custom_submissions[0].render_type,
        EntityModelLayerRenderType::DragonRays
    );
    assert_eq!(
        meshes.custom_submissions[1].render_type,
        EntityModelLayerRenderType::DragonRaysDepth
    );
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.dissolve.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(!meshes.dragon_rays.vertices.is_empty());
    assert_eq!(
        meshes.dragon_rays_depth.vertices.len(),
        meshes.dragon_rays.vertices.len()
    );
}

#[test]
fn ender_dragon_dying_submission_survives_missing_dissolve_mask_atlas_entry() {
    // The submission records the vanilla secondary mask texture even when the atlas lacks
    // `dragon_exploding.png`; only the backend's folded body geometry is suppressed.
    let images = vec![
        blank_texture(ENDER_DRAGON_TEXTURE_REF),
        blank_texture(ENDER_DRAGON_EYES_TEXTURE_REF),
    ];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::ender_dragon(903, [0.0, 64.0, 0.0], 0.0)
        .with_ender_dragon_death_time(80.0)
        .with_light_coords((7_u32 << 4) | (10_u32 << 20))
        .with_has_red_overlay(true);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 2);
    let base = meshes.submissions[0];
    assert_eq!(
        base.render_type,
        EntityModelLayerRenderType::EntityCutoutDissolve
    );
    assert_eq!(base.texture, ENDER_DRAGON_TEXTURE_REF);
    assert_eq!(
        base.dissolve_texture,
        Some(ENDER_DRAGON_EXPLODING_TEXTURE_REF)
    );
    assert_eq!(base.tint, [1.0, 1.0, 1.0, 0.6]);
    assert_eq!(base.overlay, [0.0, 10.0]);
    assert!(
        meshes.dissolve.vertices.is_empty(),
        "missing dragon_exploding.png suppresses only folded dying body geometry"
    );
    assert!(meshes.dissolve.indices.is_empty());

    let eyes = meshes.submissions[1];
    assert_eq!(eyes.render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(eyes.texture, ENDER_DRAGON_EYES_TEXTURE_REF);
    assert_eq!(eyes.dissolve_texture, None);
    assert_eq!(eyes.overlay, [0.0, 10.0]);
    assert!(!meshes.eyes.vertices.is_empty());
}

#[test]
fn ender_dragon_eyes_submission_survives_missing_eyes_texture_atlas_entry() {
    // Vanilla `EnderDragonRenderer.submit` records the always-on eyes submission immediately after
    // the body, with `RenderTypes.eyes(dragon_eyes.png)` and `OverlayTexture.NO_OVERLAY`.
    let images = vec![blank_texture(ENDER_DRAGON_TEXTURE_REF)];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::ender_dragon(901, [0.0, 64.0, 0.0], 0.0)
        .with_light_coords((9_u32 << 4) | (12_u32 << 20))
        .with_has_red_overlay(true);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.translucent.vertices.is_empty());
    assert_eq!(meshes.submissions.len(), 2);
    let base = meshes.submissions[0];
    assert_eq!(base.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(base.render_type.vanilla_name(), "entityCutout");
    assert_eq!(base.texture, ENDER_DRAGON_TEXTURE_REF);
    assert_eq!(base.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(base.transform, ender_dragon_model_root_transform(instance));
    assert_eq!((base.order, base.submit_sequence), (0, 0));
    assert_eq!(base.light, instance.render_state.shader_light());
    assert_eq!(base.overlay, instance.render_state.overlay_coords());
    assert!(!meshes.cutout.vertices.is_empty());

    let eyes = meshes.submissions[1];
    assert_eq!(eyes.render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(eyes.render_type.vanilla_name(), "eyes");
    assert_eq!(eyes.texture, ENDER_DRAGON_EYES_TEXTURE_REF);
    assert_eq!(eyes.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(eyes.transform, base.transform);
    assert_eq!((eyes.order, eyes.submit_sequence), (0, 1));
    assert_eq!(eyes.light, base.light);
    assert_eq!(eyes.overlay, [0.0, 10.0]);
    assert!(
        meshes.eyes.vertices.is_empty(),
        "missing dragon_eyes.png suppresses only folded emissive eyes geometry"
    );
    assert!(meshes.eyes.indices.is_empty());
}

#[test]
fn ender_dragon_healing_beam_records_vanilla_submission_and_geometry() {
    // Vanilla `EnderDragonRenderer.submit`: submit body, submit emissive eyes, pop the model pose,
    // then call `submitCrystalBeams` with `EnderDragonRenderState.beamOffset`. The beam uses
    // `RenderTypes.endCrystalBeam(end_crystal_beam.png)`, white submit tint, and tiled black/white
    // prism vertices.
    let images = vec![
        blank_texture(ENDER_DRAGON_TEXTURE_REF),
        blank_texture(ENDER_DRAGON_EYES_TEXTURE_REF),
        blank_texture(END_CRYSTAL_BEAM_TEXTURE_REF),
    ];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let age = 40.0;
    let position = [2.0, 70.0, -5.0];
    let beam_offset = [6.0, -0.1, 8.0];
    let instance = EntityModelInstance::ender_dragon(430, position, 0.0)
        .with_age_in_ticks(age)
        .with_ender_dragon_beam(Some(EnderDragonBeamRenderState { beam_offset }))
        .with_light_coords((11_u32 << 4) | (5_u32 << 20))
        .with_has_red_overlay(true);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 3);
    assert_eq!(
        (
            meshes.submissions[0].render_type,
            meshes.submissions[0].texture,
            meshes.submissions[0].tint,
            meshes.submissions[0].order,
            meshes.submissions[0].submit_sequence,
        ),
        (
            EntityModelLayerRenderType::EntityCutout,
            ENDER_DRAGON_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
            0,
        )
    );
    assert_eq!(
        meshes.submissions[0].light,
        instance.render_state.shader_light()
    );
    assert_eq!(
        meshes.submissions[0].overlay,
        instance.render_state.overlay_coords()
    );
    assert!(meshes.cutout.vertices.iter().all(|vertex| {
        vertex.light == meshes.submissions[0].light
            && vertex.overlay == meshes.submissions[0].overlay
    }));
    assert_eq!(
        (
            meshes.submissions[1].render_type,
            meshes.submissions[1].texture,
            meshes.submissions[1].tint,
            meshes.submissions[1].order,
            meshes.submissions[1].submit_sequence,
        ),
        (
            EntityModelLayerRenderType::Eyes,
            ENDER_DRAGON_EYES_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
            1,
        )
    );
    assert_eq!(meshes.submissions[1].light, meshes.submissions[0].light);
    assert_eq!(meshes.submissions[1].overlay, [0.0, 10.0]);
    assert!(meshes.eyes.vertices.iter().all(|vertex| {
        vertex.light == meshes.submissions[1].light
            && vertex.overlay == meshes.submissions[1].overlay
    }));

    let beam_submit = meshes.submissions[2];
    assert_eq!(
        beam_submit.render_type,
        EntityModelLayerRenderType::EndCrystalBeam
    );
    assert_eq!(beam_submit.render_type.vanilla_name(), "end_crystal_beam");
    assert_eq!(beam_submit.texture, END_CRYSTAL_BEAM_TEXTURE_REF);
    assert_eq!(beam_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((beam_submit.order, beam_submit.submit_sequence), (0, 2));
    assert_eq!(beam_submit.light, meshes.submissions[0].light);
    assert_eq!(beam_submit.overlay, [0.0, 10.0]);
    assert_ne!(beam_submit.overlay, meshes.submissions[0].overlay);

    let delta = Vec3::from_array(beam_offset);
    let origin = Vec3::from_array(position) + Vec3::Y * 2.0;
    assert_close3(
        beam_submit
            .transform
            .transform_point3(Vec3::ZERO)
            .to_array(),
        origin.to_array(),
    );
    assert_close3(
        beam_submit
            .transform
            .transform_vector3(Vec3::Z)
            .normalize()
            .to_array(),
        delta.normalize().to_array(),
    );

    assert_eq!(meshes.scroll.vertices.len(), 32);
    assert_eq!(meshes.scroll.indices.len(), 48);
    let rect = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == END_CRYSTAL_BEAM_TEXTURE_REF)
        .unwrap()
        .uv;
    assert_eq!(meshes.scroll.vertices[0].uv_rect_min, rect.min);
    assert_eq!(
        meshes.scroll.vertices[0].uv_rect_size,
        [rect.max[0] - rect.min[0], rect.max[1] - rect.min[1]]
    );
    assert_eq!(meshes.scroll.vertices[0].tint, [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(meshes.scroll.vertices[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert!(meshes
        .scroll
        .vertices
        .iter()
        .all(|vertex| vertex.light == beam_submit.light && vertex.overlay == beam_submit.overlay));
    assert_eq!(meshes.scroll.vertices[0].local_uv[0], 0.0);
    assert_eq!(meshes.scroll.vertices[3].local_uv[0], 0.125);
    assert!(
        (meshes.scroll.vertices[1].local_uv[1]
            - meshes.scroll.vertices[0].local_uv[1]
            - delta.length() / 32.0)
            .abs()
            < 1.0e-6
    );
}

#[test]
fn ender_dragon_healing_beam_submission_survives_missing_beam_texture_atlas_entry() {
    // Vanilla `EnderDragonRenderer.submit` records body, eyes, then the healing beam submission;
    // missing beam texture data suppresses only the backend's folded scroll geometry.
    let images = vec![
        blank_texture(ENDER_DRAGON_TEXTURE_REF),
        blank_texture(ENDER_DRAGON_EYES_TEXTURE_REF),
    ];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let position = [2.0, 70.0, -5.0];
    let beam_offset = [6.0, -0.1, 8.0];
    let instance = EntityModelInstance::ender_dragon(431, position, 0.0)
        .with_age_in_ticks(40.0)
        .with_ender_dragon_beam(Some(EnderDragonBeamRenderState { beam_offset }))
        .with_light_coords((11_u32 << 4) | (5_u32 << 20))
        .with_has_red_overlay(true);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.translucent.vertices.is_empty());
    assert_eq!(meshes.submissions.len(), 3);
    let base = meshes.submissions[0];
    assert_eq!(base.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(base.render_type.vanilla_name(), "entityCutout");
    assert_eq!(base.texture, ENDER_DRAGON_TEXTURE_REF);
    assert_eq!(base.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(base.transform, ender_dragon_model_root_transform(instance));
    assert_eq!((base.order, base.submit_sequence), (0, 0));
    assert_eq!(base.light, instance.render_state.shader_light());
    assert_eq!(base.overlay, instance.render_state.overlay_coords());
    assert!(!meshes.cutout.vertices.is_empty());

    let eyes = meshes.submissions[1];
    assert_eq!(eyes.render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(eyes.render_type.vanilla_name(), "eyes");
    assert_eq!(eyes.texture, ENDER_DRAGON_EYES_TEXTURE_REF);
    assert_eq!(eyes.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(eyes.transform, base.transform);
    assert_eq!((eyes.order, eyes.submit_sequence), (0, 1));
    assert_eq!(eyes.light, base.light);
    assert_eq!(eyes.overlay, [0.0, 10.0]);
    assert_eq!(meshes.eyes.vertices.len(), meshes.cutout.vertices.len());

    let beam_submit = meshes.submissions[2];
    assert_eq!(
        beam_submit.render_type,
        EntityModelLayerRenderType::EndCrystalBeam
    );
    assert_eq!(beam_submit.render_type.vanilla_name(), "end_crystal_beam");
    assert_eq!(beam_submit.texture, END_CRYSTAL_BEAM_TEXTURE_REF);
    assert_eq!(beam_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((beam_submit.order, beam_submit.submit_sequence), (0, 2));
    assert_eq!(beam_submit.light, base.light);
    assert_eq!(beam_submit.overlay, [0.0, 10.0]);
    assert_ne!(beam_submit.overlay, base.overlay);
    let delta = Vec3::from_array(beam_offset);
    let origin = Vec3::from_array(position) + Vec3::Y * 2.0;
    assert_close3(
        beam_submit
            .transform
            .transform_point3(Vec3::ZERO)
            .to_array(),
        origin.to_array(),
    );
    assert_close3(
        beam_submit
            .transform
            .transform_vector3(Vec3::Z)
            .normalize()
            .to_array(),
        delta.normalize().to_array(),
    );
    assert!(
        meshes.scroll.vertices.is_empty(),
        "missing end_crystal_beam.png suppresses only folded healing-beam geometry"
    );
    assert!(meshes.scroll.indices.is_empty());
}

#[test]
fn ender_dragon_dying_dissolve_mesh_maps_mask_uv_into_exploding_atlas_rect() {
    // Vanilla `entity.fsh` samples `DissolveMaskSampler` at the *same* `texCoord0` as the base
    // texture. Because both `dragon.png` and `dragon_exploding.png` live in the shared entity atlas,
    // our equivalent bakes each dissolve vertex's `mask_uv` by re-projecting the normalized base UV
    // into the mask's atlas sub-rect. Assert that deterministic mapping, vertex by vertex, and that
    // both UV sets stay inside their own atlas rects.
    let images: Vec<EntityModelTextureImage> = ender_dragon_entity_texture_refs()
        .iter()
        .map(|texture| blank_texture(*texture))
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::ender_dragon(910, [0.0, 64.0, 0.0], 0.0)
        .with_ender_dragon_death_time(50.0);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    let rect_of = |texture: EntityModelTextureRef| {
        atlas
            .entries
            .iter()
            .find(|entry| entry.texture == texture)
            .map(|entry| entry.uv)
            .expect("atlas rect")
    };
    let base_rect = rect_of(ENDER_DRAGON_TEXTURE_REF);
    let mask_rect = rect_of(ENDER_DRAGON_EXPLODING_TEXTURE_REF);
    let base_size = [
        base_rect.max[0] - base_rect.min[0],
        base_rect.max[1] - base_rect.min[1],
    ];
    let mask_size = [
        mask_rect.max[0] - mask_rect.min[0],
        mask_rect.max[1] - mask_rect.min[1],
    ];

    assert!(
        !meshes.dissolve.vertices.is_empty(),
        "dying dragon body folds into the dissolve bucket"
    );
    assert_eq!(
        meshes.dissolve.cutout_faces * 6,
        meshes.dissolve.indices.len()
    );
    // dragon.png and dragon_exploding.png are both 256×256 and stitched at the same width, so the two
    // atlas rects are congruent (equal size) — mask_uv is the base_uv translated by (mask.min-base.min).
    // (The vanilla dragon model legitimately carries texCoord0 values outside `[0,1]`; the sampler
    // clamps them, so we assert the mapping formula, not rect containment.)
    let mut saw_out_of_unit = false;
    for vertex in &meshes.dissolve.vertices {
        let normalized = [
            (vertex.uv[0] - base_rect.min[0]) / base_size[0],
            (vertex.uv[1] - base_rect.min[1]) / base_size[1],
        ];
        if !(0.0..=1.0).contains(&normalized[0]) || !(0.0..=1.0).contains(&normalized[1]) {
            saw_out_of_unit = true;
        }
        let expected = [
            mask_rect.min[0] + normalized[0] * mask_size[0],
            mask_rect.min[1] + normalized[1] * mask_size[1],
        ];
        assert!(
            (vertex.mask_uv[0] - expected[0]).abs() < 1e-5
                && (vertex.mask_uv[1] - expected[1]).abs() < 1e-5,
            "mask_uv {:?} != expected {:?}",
            vertex.mask_uv,
            expected
        );
    }
    assert!(
        saw_out_of_unit,
        "the dragon model carries some texCoord0 outside [0,1]; the mapping must still hold there"
    );
}

/// End-to-end GPU proof of the DISSOLVE mask erosion: a full-screen quad is rendered through the real
/// [`create_entity_model_dissolve_pipeline`] against a hand-built entity atlas — a pure-red 1×1 base
/// texture and a 2×1 mask whose left column has alpha `0.2` and right column alpha `0.8`. The quad's
/// `mask_uv.x` spans the mask rect left→right, so the left half of the screen samples mask alpha `0.2`
/// and the right half `0.8` (Nearest sampling). With vertex `tint.a = 0.5` (`1 - deathTime/200` for a
/// mid-death dragon), vanilla `entity.fsh` keeps a fragment iff `tint.a >= mask.a`: the left half
/// survives (opaque red, alpha forced to 1.0) and the right half is discarded (background). With
/// `tint.a = 1.0` (a live dragon), both halves survive. Skips when no GPU adapter is available.
#[test]
fn ender_dragon_dissolve_pipeline_erodes_pixels_by_mask_alpha() {
    use wgpu::util::DeviceExt;

    use crate::camera::{CameraUniform, LightmapEnvironment};
    use crate::entity_models::geometry::EntityModelDissolveVertex;
    use crate::entity_models::gpu::create_entity_model_dissolve_pipeline;
    use crate::gpu::{create_camera_buffer, create_depth_target, create_terrain_bind_group_layout};
    use crate::lightmap::{
        create_lightmap_bind_group_layout, create_lightmap_gpu,
        create_lightmap_sample_bind_group_layout,
    };

    const WIDTH: u32 = 256;
    const HEIGHT: u32 = 64;
    // Non-sRGB target so the readback bytes are the shader's linear output verbatim. `256 * 4 = 1024`
    // is a multiple of 256, so the texture-to-buffer copy needs no row padding.
    const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let Some(adapter) =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
    else {
        // No GPU / software adapter on this machine — skip rather than fail the suite.
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("bbb-dissolve-test-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
        },
        None,
    )) else {
        return;
    };

    // Hand-built atlas: a 1×1 pure-red base texture and a 2×1 mask with a two-step alpha gradient
    // (left col 0.2, right col 0.8). The mask's RGB is irrelevant — only its alpha drives the compare.
    let base_ref = EntityModelTextureRef {
        path: "bbb-test/dissolve-base.png",
        size: [1, 1],
    };
    let mask_ref = EntityModelTextureRef {
        path: "bbb-test/dissolve-mask.png",
        size: [2, 1],
    };
    let images = [
        EntityModelTextureImage::new(base_ref, vec![255, 0, 0, 255]),
        EntityModelTextureImage::new(mask_ref, vec![0, 0, 0, 51, 0, 0, 0, 204]),
    ];
    let (atlas_layout, atlas_rgba) =
        build_entity_model_texture_atlas(&images).expect("entity atlas");
    let rect_of = |texture: EntityModelTextureRef| {
        atlas_layout
            .entries
            .iter()
            .find(|entry| entry.texture == texture)
            .map(|entry| entry.uv)
            .expect("atlas rect")
    };
    let base_rect = rect_of(base_ref);
    let mask_rect = rect_of(mask_ref);
    // Base UV: the centre of the 1×1 red texel (constant — the base is uniform red). Mask UV.y: the
    // centre of the mask row. Mask UV.x spans the mask rect so Nearest picks col 0 on the left half of
    // the screen and col 1 on the right half.
    let base_uv = [
        (base_rect.min[0] + base_rect.max[0]) * 0.5,
        (base_rect.min[1] + base_rect.max[1]) * 0.5,
    ];
    let mask_v = (mask_rect.min[1] + mask_rect.max[1]) * 0.5;

    let camera_uniform = CameraUniform::identity();

    let bind_group_layout = create_terrain_bind_group_layout(&device);
    let camera_buffer = create_camera_buffer(&device);
    queue.write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
    let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("dissolve-test-atlas"),
        size: wgpu::Extent3d {
            width: atlas_layout.width,
            height: atlas_layout.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &atlas_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &atlas_rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(atlas_layout.width * 4),
            rows_per_image: Some(atlas_layout.height),
        },
        wgpu::Extent3d {
            width: atlas_layout.width,
            height: atlas_layout.height,
            depth_or_array_layers: 1,
        },
    );
    let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
    // Nearest, ClampToEdge — mirrors the production entity atlas sampler so the two-step mask alpha
    // reads back exactly 0.2 / 0.8 with no interpolation across the column boundary.
    let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("dissolve-test-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("dissolve-test-bind-group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&atlas_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&atlas_sampler),
            },
        ],
    });
    let lightmap_bind_group_layout = create_lightmap_bind_group_layout(&device);
    let lightmap_sample_bind_group_layout = create_lightmap_sample_bind_group_layout(&device);
    let lightmap = create_lightmap_gpu(
        &device,
        &queue,
        &lightmap_bind_group_layout,
        &lightmap_sample_bind_group_layout,
        LightmapEnvironment::default(),
    );
    let pipeline = create_entity_model_dissolve_pipeline(
        &device,
        COLOR_FORMAT,
        &bind_group_layout,
        &lightmap_sample_bind_group_layout,
    );

    // A full-screen quad at NDC z = 0.5 (identity `view_proj`): NDC x −1→+1 maps left→right, so
    // `mask_uv.x` = mask_rect.min.x on the left edge and mask_rect.max.x on the right edge.
    let quad = |alpha: f32| -> [EntityModelDissolveVertex; 4] {
        let vertex = |ndc_x: f32, ndc_y: f32, mask_x: f32| EntityModelDissolveVertex {
            position: [ndc_x, ndc_y, 0.5],
            uv: base_uv,
            mask_uv: [mask_x, mask_v],
            tint: [1.0, 0.0, 0.0, alpha],
            light: [1.0, 1.0],
            overlay: [0.0, 10.0],
            normal: [0.0, 0.0, -1.0],
        };
        [
            vertex(-1.0, 1.0, mask_rect.min[0]),
            vertex(-1.0, -1.0, mask_rect.min[0]),
            vertex(1.0, -1.0, mask_rect.max[0]),
            vertex(1.0, 1.0, mask_rect.max[0]),
        ]
    };
    let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("dissolve-test-indices"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    let bytes_per_row = WIDTH * 4;

    let render = |alpha: f32| -> Vec<u8> {
        let vertices = quad(alpha);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("dissolve-test-vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("dissolve-test-color"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = create_depth_target(&device, WIDTH, HEIGHT);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dissolve-test-readback"),
            size: (bytes_per_row * HEIGHT) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("dissolve-test-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Blue background — distinct from the red base texture.
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_bind_group(1, &lightmap.sample_bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(HEIGHT),
                },
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::Maintain::Wait);
        let bytes = slice.get_mapped_range().to_vec();
        readback.unmap();
        bytes
    };

    // Sample well clear of the centre column boundary (screen centre = mask_uv.x 0.5, the Nearest seam):
    // left quarter reads mask alpha 0.2, right quarter reads mask alpha 0.8.
    let mid_y = HEIGHT / 2;
    let left_x = WIDTH / 4;
    let right_x = WIDTH * 3 / 4;
    let pixel = |data: &[u8], x: u32, y: u32| -> [u8; 4] {
        let o = (y * bytes_per_row + x * 4) as usize;
        [data[o], data[o + 1], data[o + 2], data[o + 3]]
    };
    let is_red = |p: [u8; 4]| p[0] > 40 && p[0] > p[1] && p[0] > p[2];
    let is_background = |p: [u8; 4]| p[2] > 128 && p[2] > p[0];

    // tint.a = 0.5: left half survives (0.5 >= 0.2 → opaque red), right half is eroded (0.5 < 0.8 →
    // discarded → blue background).
    let eroded = render(0.5);
    let eroded_left = pixel(&eroded, left_x, mid_y);
    let eroded_right = pixel(&eroded, right_x, mid_y);
    assert!(
        is_red(eroded_left),
        "left half (mask 0.2, tint.a 0.5) should survive as opaque red, got {eroded_left:?}"
    );
    assert!(
        is_background(eroded_right),
        "right half (mask 0.8, tint.a 0.5) should be eroded to background, got {eroded_right:?}"
    );

    // tint.a = 1.0 (a live dragon): both halves survive (1.0 >= 0.8 and 1.0 >= 0.2).
    let full = render(1.0);
    let full_left = pixel(&full, left_x, mid_y);
    let full_right = pixel(&full, right_x, mid_y);
    assert!(
        is_red(full_left) && is_red(full_right),
        "tint.a 1.0 keeps every fragment, got left {full_left:?} right {full_right:?}"
    );
}
