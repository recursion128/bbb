use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn squid_cubes_match_vanilla_26_1_body_layer() {
    // Body cube carries `CubeDeformation(0.02)`: box(-6, -8, -6, 12×16×12) inflated 0.02 on the
    // geometry, while the textured `uv_size` keeps the base 12×16×12 box. The unified cube carries the
    // colored tint (`SQUID_BLUE`) and the textured UV (`texOffs(0, 0)`) in one struct.
    assert_eq!(
        SQUID_BODY[0],
        ModelCube::new(
            [-6.02, -8.02, -6.02],
            [12.04, 16.04, 12.04],
            SQUID_BLUE,
            [12.0, 16.0, 12.0],
            [0.0, 0.0],
            false,
        )
    );
    // Tentacle cube: `texOffs(48, 0)` box(-1, 0, -1, 2×18×2), no deformation (so `uv_size == size`).
    assert_eq!(
        SQUID_TENTACLE[0],
        ModelCube::new(
            [-1.0, 0.0, -1.0],
            [2.0, 18.0, 2.0],
            SQUID_BLUE,
            [2.0, 18.0, 2.0],
            [48.0, 0.0],
            false,
        )
    );
}

#[test]
fn squid_tentacle_ring_layout_matches_vanilla() {
    // Vanilla `SquidModel.createBodyLayer`: the body sits at `offset(0, 8, 0)`, and the eight tentacles
    // ring at `(cos(i·2π/8)·5, 15, sin(i·2π/8)·5)`, yawed `-i·2π/8 + π/2` so each flat face points
    // outward. Spot-check the body offset and the four cardinal tentacles (i = 0/2/4/6).
    assert_close3(SQUID_BODY_POSE.offset, [0.0, 8.0, 0.0]);
    assert_eq!(SQUID_BODY_POSE.rotation, [0.0, 0.0, 0.0]);

    let half_pi = std::f32::consts::FRAC_PI_2;
    for (i, offset, y_rot) in [
        (0usize, [5.0, 15.0, 0.0], half_pi),
        (2, [0.0, 15.0, 5.0], 0.0),
        (4, [-5.0, 15.0, 0.0], -half_pi),
        (6, [0.0, 15.0, -5.0], -std::f32::consts::PI),
    ] {
        let pose = squid_tentacle_pose(i);
        assert_close3(pose.offset, offset);
        // The bind `xRot` is `0`; `setup_anim` overwrites it with `tentacleAngle` each frame.
        assert_close3(pose.rotation, [0.0, y_rot, 0.0]);
    }
}

#[test]
fn squid_mesh_uses_vanilla_body_layer_geometry_and_glow_variant() {
    // Nine cubes (body + eight tentacles) → 54 faces / 216 vertices.
    let squid = entity_model_mesh(&[EntityModelInstance::squid(
        800,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )]);
    assert_eq!(squid.opaque_faces, 54);
    assert_eq!(squid.vertices.len(), 216);
    assert_eq!(squid.indices.len(), 324);
    assert!(squid
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(SQUID_BLUE, 1.0)));

    // The glow squid reuses the geometry with the glow tint.
    let glow = entity_model_mesh(&[EntityModelInstance::squid(
        801,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
    )]);
    assert_same_geometry(&glow, &squid);
    assert!(glow
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GLOW_SQUID_TEAL, 1.0)));

    // The baby uses the `BABY_TRANSFORMER` 0.5-scaled body layer: same topology, smaller.
    let baby = entity_model_mesh(&[EntityModelInstance::squid(
        802,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
    )]);
    assert_eq!(baby.vertices.len(), 216);
    let (adult_min, adult_max) = mesh_extents(&squid);
    let (baby_min, baby_max) = mesh_extents(&baby);
    let adult_height = adult_max[1] - adult_min[1];
    let baby_height = baby_max[1] - baby_min[1];
    assert!(
        (baby_height - adult_height * 0.5).abs() < 1.0e-3,
        "baby squid is half the adult height ({baby_height} vs {adult_height})"
    );
}

#[test]
fn squid_swims_its_tentacles_when_animated() {
    // A resting squid (tentacleAngle 0) is inert; sweeping the tentacles bends them
    // without changing the vertex count.
    let base = EntityModelInstance::squid(803, [0.0, 64.0, 0.0], 0.0, false, false);
    let rest = entity_model_mesh(&[base]);
    let swept = entity_model_mesh(&[base.with_squid_tentacle_angle(0.8)]);

    assert_eq!(rest.vertices.len(), swept.vertices.len());
    assert_ne!(rest.vertices, swept.vertices, "the tentacles bend");
}

#[test]
fn squid_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            false,
            false,
            "squid",
            EntityModelTextureRef {
                path: "textures/entity/squid/squid.png",
                size: [64, 32],
            },
        ),
        (
            false,
            true,
            "squid_baby",
            EntityModelTextureRef {
                path: "textures/entity/squid/squid_baby.png",
                size: [32, 32],
            },
        ),
        (
            true,
            false,
            "glow_squid",
            EntityModelTextureRef {
                path: "textures/entity/squid/glow_squid.png",
                size: [64, 32],
            },
        ),
        (
            true,
            true,
            "glow_squid_baby",
            EntityModelTextureRef {
                path: "textures/entity/squid/glow_squid_baby.png",
                size: [32, 32],
            },
        ),
    ];

    for (glow, baby, model_key, texture) in cases {
        let kind = EntityModelKind::Squid { glow, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn squid_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let cases = [
        (false, false, MODEL_LAYER_SQUID, SQUID_TEXTURE_REF),
        (false, true, MODEL_LAYER_SQUID_BABY, SQUID_BABY_TEXTURE_REF),
        (true, false, MODEL_LAYER_GLOW_SQUID, GLOW_SQUID_TEXTURE_REF),
        (
            true,
            true,
            MODEL_LAYER_GLOW_SQUID_BABY,
            GLOW_SQUID_BABY_TEXTURE_REF,
        ),
    ];
    for (glow, baby, model_layer, texture) in cases {
        let passes = squid_textured_layer_passes(glow, baby);
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0].kind, EntityModelLayerKind::SquidBase);
        assert_eq!(passes[0].model_layer, model_layer);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
    }
}

#[test]
fn squid_textured_mesh_uses_vanilla_geometry_and_variant_texture() {
    let (atlas, _) = build_entity_model_texture_atlas(&squid_texture_images()).unwrap();
    // Vanilla `SquidModel` inherits the default `EntityModel` render type (`entityCutout`).
    // `SquidRenderer` / `GlowSquidRenderer` only switch the texture, so pin the submission metadata
    // in addition to the folded cutout mesh geometry.
    let squid_instance = EntityModelInstance::squid(810, [0.0, 64.0, 0.0], 0.0, false, false)
        .with_light_coords((6_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let squid_meshes = entity_model_textured_meshes(&[squid_instance], &atlas);
    assert_squid_submission(&squid_meshes, squid_instance, SQUID_TEXTURE_REF, false);
    assert_ne!(squid_instance.render_state.overlay_coords(), [0.0, 10.0]);
    assert!(squid_meshes.translucent.vertices.is_empty());
    assert!(squid_meshes.eyes.vertices.is_empty());
    // Nine cubes (body + eight tentacles) → 216 textured vertices, all on the cutout pass.
    let squid = &squid_meshes.cutout;
    assert_eq!(squid.vertices.len(), 216);

    // The glow squid reuses the geometry at the same positions but samples a different
    // texture (glow_squid.png), so the vertex UVs differ from the plain squid.
    let glow_instance = EntityModelInstance::squid(811, [0.0, 64.0, 0.0], 0.0, true, false)
        .with_light_coords((15_u32 << 4) | (8_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let glow_meshes = entity_model_textured_meshes(&[glow_instance], &atlas);
    assert_squid_submission(&glow_meshes, glow_instance, GLOW_SQUID_TEXTURE_REF, false);
    let glow = &glow_meshes.cutout;
    assert_eq!(glow.vertices.len(), squid.vertices.len());
    assert_eq!(
        glow.vertices.iter().map(|v| v.position).collect::<Vec<_>>(),
        squid
            .vertices
            .iter()
            .map(|v| v.position)
            .collect::<Vec<_>>()
    );
    assert_ne!(
        glow.vertices.iter().map(|v| v.uv).collect::<Vec<_>>(),
        squid.vertices.iter().map(|v| v.uv).collect::<Vec<_>>()
    );

    // The baby uses the 0.5-scaled body layer.
    let baby_instance = EntityModelInstance::squid(812, [0.0, 64.0, 0.0], 0.0, false, true)
        .with_light_coords((6_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let baby_meshes = entity_model_textured_meshes(&[baby_instance], &atlas);
    assert_squid_submission(&baby_meshes, baby_instance, SQUID_BABY_TEXTURE_REF, true);
    let baby = &baby_meshes.cutout;
    assert_eq!(baby.vertices.len(), 216);
    let (adult_min, adult_max) = textured_mesh_extents(&squid);
    let (baby_min, baby_max) = textured_mesh_extents(&baby);
    assert!(
        ((baby_max[1] - baby_min[1]) - (adult_max[1] - adult_min[1]) * 0.5).abs() < 1.0e-3,
        "baby squid is half the adult height"
    );
}

#[test]
fn squid_textured_mesh_swims_its_tentacles() {
    let (atlas, _) = build_entity_model_texture_atlas(&squid_texture_images()).unwrap();
    let base = EntityModelInstance::squid(813, [0.0, 64.0, 0.0], 0.0, false, false)
        .with_light_coords((6_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let rest = entity_model_textured_meshes(&[base], &atlas);
    assert_squid_submission(&rest, base, SQUID_TEXTURE_REF, false);
    let swept_instance = base.with_squid_tentacle_angle(0.8);
    let swept = entity_model_textured_meshes(&[swept_instance], &atlas);
    assert_squid_submission(&swept, swept_instance, SQUID_TEXTURE_REF, false);
    assert_eq!(rest.cutout.vertices.len(), swept.cutout.vertices.len());
    assert_ne!(rest.cutout.vertices, swept.cutout.vertices);
}

#[test]
fn squid_applies_swim_body_tilt() {
    // `SquidRenderer.setupRotations` pitches the squid by `xBodyRot` (about X) then rolls
    // it by `zBodyRot` (about Y) after the body yaw. A resting squid is upright; a pitched
    // squid is reoriented (its tall body lays toward the horizontal) without changing the
    // vertex count.
    let base = EntityModelInstance::squid(820, [0.0, 64.0, 0.0], 0.0, false, false);
    let rest = entity_model_mesh(&[base]);
    let pitched = entity_model_mesh(&[base.with_squid_body_tilt(-90.0, 0.0)]);
    assert_eq!(rest.vertices.len(), pitched.vertices.len());
    assert_ne!(
        rest.vertices, pitched.vertices,
        "the pitch reorients the squid"
    );

    // A 90° pitch swaps the body's vertical extent into depth: the pitched squid is much
    // shorter in Y and deeper in Z than the upright one.
    let (rest_min, rest_max) = mesh_extents(&rest);
    let (pitch_min, pitch_max) = mesh_extents(&pitched);
    assert!(
        (pitch_max[1] - pitch_min[1]) < (rest_max[1] - rest_min[1]) - 0.3,
        "a pitched squid is shorter in Y"
    );
    assert!(
        (pitch_max[2] - pitch_min[2]) > (rest_max[2] - rest_min[2]) + 0.3,
        "a pitched squid is deeper in Z"
    );

    // The roll about Y also reorients the model.
    let rolled = entity_model_mesh(&[base.with_squid_body_tilt(0.0, 35.0)]);
    assert_ne!(
        rest.vertices, rolled.vertices,
        "the roll reorients the squid"
    );
}

fn squid_texture_images() -> Vec<EntityModelTextureImage> {
    squid_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn assert_squid_submission(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    texture: EntityModelTextureRef,
    baby: bool,
) {
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.texture, texture);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.order, 0);
    assert_eq!(submit.submit_sequence, 0);
    assert_eq!(submit.transform, squid_model_root_transform(instance, baby));
    assert_eq!(submit.light, instance.render_state.shader_light());
    assert_eq!(submit.overlay, instance.render_state.overlay_coords());
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
}
