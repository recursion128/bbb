use super::*;
use crate::entity_models::model_layers::wither_model;

#[test]
fn wither_side_heads_follow_vanilla_target_rotation_arrays() {
    // Vanilla `WitherBossModel.setupHeadRotation`:
    //   head.yRot = (state.yHeadRots[index] - state.bodyRot) * PI / 180
    //   head.xRot = state.xHeadRots[index] * PI / 180
    let instance = EntityModelInstance::wither(1452, [0.0, 64.0, 0.0], 30.0).with(|s| {
        s.kind.wither_mut().x_head_rots = [-12.0, 25.0];
        s.kind.wither_mut().y_head_rots = [50.0, -10.0];
    });
    let mut model = wither_model(ModelLayer::WITHER, &instance);

    let right = model.child_mut("right_head").pose.rotation;
    assert!((right[0] - (-12.0_f32).to_radians()).abs() < 1.0e-6);
    assert!((right[1] - 20.0_f32.to_radians()).abs() < 1.0e-6);

    let left = model.child_mut("left_head").pose.rotation;
    assert!((left[0] - 25.0_f32.to_radians()).abs() < 1.0e-6);
    assert!((left[1] - (-40.0_f32).to_radians()).abs() < 1.0e-6);

    drop(model);
    let mut model = wither_model(
        ModelLayer::WITHER,
        &EntityModelInstance::wither(1453, [0.0, 64.0, 0.0], 0.0),
    );
    assert_eq!(
        model.child_mut("right_head").pose.rotation,
        [0.0, 0.0, 0.0],
        "the next frame must not keep the previous side-head target pose"
    );
}

#[test]
fn wither_breathing_poses_match_vanilla_setup_anim() {
    use std::f32::consts::PI;
    // Vanilla `WitherBossModel.setupAnim`:
    //   anim         = cos(ageInTicks * 0.1)
    //   ribcage.xRot = (0.065 + 0.05 * anim) * PI
    //   tail.setPos(-2, 6.9 + cos(ribcage.xRot) * 10, -0.5 + sin(ribcage.xRot) * 10)
    //   tail.xRot    = (0.265 + 0.1 * anim) * PI
    let age = 10.0_f32;
    let anim = (age * 0.1).cos();
    let ribcage_x_rot = (0.065 + 0.05 * anim) * PI;
    let (ribcage, tail) = wither_breathing_poses(age);
    assert_eq!(ribcage.offset, [-2.0, 6.9, -0.5]);
    assert!((ribcage.rotation[0] - ribcage_x_rot).abs() < 1.0e-6);
    assert_eq!([ribcage.rotation[1], ribcage.rotation[2]], [0.0, 0.0]);
    assert!((tail.offset[0] - (-2.0)).abs() < 1.0e-6);
    assert!((tail.offset[1] - (6.9 + ribcage_x_rot.cos() * 10.0)).abs() < 1.0e-5);
    assert!((tail.offset[2] - (-0.5 + ribcage_x_rot.sin() * 10.0)).abs() < 1.0e-5);
    assert!((tail.rotation[0] - (0.265 + 0.1 * anim) * PI).abs() < 1.0e-6);
    assert_eq!([tail.rotation[1], tail.rotation[2]], [0.0, 0.0]);

    // `anim == 0` (when ageInTicks * 0.1 == PI/2) collapses the sway onto the baked rest poses, so
    // the breathing oscillates symmetrically about the layer pose.
    let (rib_rest, tail_rest) = wither_breathing_poses(5.0 * PI);
    let rib_bind = layer_pose(ModelLayer::WITHER, "ribcage");
    let tail_bind = layer_pose(ModelLayer::WITHER, "tail");
    assert!((rib_rest.rotation[0] - rib_bind.rotation[0]).abs() < 1.0e-5);
    assert!((tail_rest.offset[1] - tail_bind.offset[1]).abs() < 1.0e-3);
    assert!((tail_rest.offset[2] - tail_bind.offset[2]).abs() < 1.0e-3);
    assert!((tail_rest.rotation[0] - tail_bind.rotation[0]).abs() < 1.0e-5);
}

#[test]
fn wither_textured_render_matches_vanilla_renderer() {
    // Vanilla `WitherBossRenderer.getTextureLocation`: `i = floor(invulnerableTicks)`; the
    // `wither_invulnerable.png` armor shows while `i > 0 && (i > 80 || i / 5 % 2 != 1)`. A fully-spawned
    // wither (`0`) and the flicker-off windows show `wither.png`.
    assert_eq!(
        wither_textured_layer_passes(0.0)[0].texture,
        WITHER_TEXTURE_REF,
        "a fully-spawned wither shows wither.png"
    );
    assert_eq!(
        wither_textured_layer_passes(220.0)[0].texture,
        WITHER_INVULNERABLE_TEXTURE_REF,
        "a freshly-summoned wither (> 80 ticks) shows the invulnerable armor"
    );
    assert_eq!(
        wither_textured_layer_passes(10.0)[0].texture,
        WITHER_INVULNERABLE_TEXTURE_REF,
        "i=10: 10/5%2=0 != 1, so the armor shows"
    );
    assert_eq!(
        wither_textured_layer_passes(5.0)[0].texture,
        WITHER_TEXTURE_REF,
        "i=5: 5/5%2=1, so it flickers back to wither.png"
    );
    let passes = wither_textured_layer_passes(0.0);
    assert_eq!(passes.len(), 2);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].texture, WITHER_TEXTURE_REF);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
    let grey = 128.0 / 255.0;
    assert_eq!(
        passes[1].render_type,
        EntityModelLayerRenderType::EnergySwirl
    );
    assert_eq!(passes[1].render_type.vanilla_name(), "energySwirl");
    assert_eq!(passes[1].texture, WITHER_ARMOR_TEXTURE_REF);
    assert_eq!(passes[1].tint, [grey, grey, grey, 1.0]);
    assert_eq!(passes[1].order, 1);

    // The catalog's static mapping is the base texture (the invulnerable variant is render-state
    // driven, picked per-instance in the dispatch).
    assert!(entity_model_texture_refs().contains(&WITHER_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&WITHER_INVULNERABLE_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&WITHER_ARMOR_TEXTURE_REF));

    let images: Vec<EntityModelTextureImage> = [
        WITHER_TEXTURE_REF,
        WITHER_INVULNERABLE_TEXTURE_REF,
        WITHER_ARMOR_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    // A fully-spawned and a mid-spawn (invulnerable) wither both emit base submissions and folded
    // cutout geometry tinted white; only the selected texture differs.
    for (invulnerable_ticks, expected_texture) in [
        (0.0, WITHER_TEXTURE_REF),
        (220.0, WITHER_INVULNERABLE_TEXTURE_REF),
    ] {
        let instance = EntityModelInstance::wither(1450, [0.0, 64.0, 0.0], 0.0).with(|s| {
            s.kind.wither_mut().invulnerable_ticks = invulnerable_ticks;
            s.light_coords = (5_u32 << 4) | (11_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
        let meshes = entity_model_textured_meshes(&[instance], &atlas);
        let uv = atlas
            .entries
            .iter()
            .find(|entry| entry.texture == expected_texture)
            .unwrap()
            .uv;
        assert!(meshes.cutout.vertices.iter().all(|vertex| {
            vertex.uv[0] >= uv.min[0]
                && vertex.uv[0] <= uv.max[0]
                && vertex.uv[1] >= uv.min[1]
                && vertex.uv[1] <= uv.max[1]
        }));
        assert!(meshes.translucent.vertices.is_empty());
        assert!(meshes.eyes.vertices.is_empty());

        let mesh = &meshes.cutout;

        assert!(!mesh.vertices.is_empty());
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
                && vertex.light == instance.render_state.shader_light()
                && vertex.overlay == instance.render_state.overlay_coords()));
    }
}

#[test]
fn powered_wither_emits_scrolling_energy_swirl() {
    // Build an atlas covering the wither's base, invulnerable, and armor textures.
    let images: Vec<EntityModelTextureImage> = [
        WITHER_TEXTURE_REF,
        WITHER_INVULNERABLE_TEXTURE_REF,
        WITHER_ARMOR_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    // A healthy wither has no `WitherArmorLayer`, so it emits no additive swirl geometry.
    let plain = entity_model_textured_meshes(
        &[EntityModelInstance::wither(1470, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert!(
        plain.scroll_additive.vertices.is_empty(),
        "no energy swirl on a healthy wither"
    );

    // A powered wither (≤ half health) draws the inflated `WITHER_ARMOR` model (9 cubes → 216
    // vertices) into the additive scroll mesh, every vertex tinted by the vanilla `0xFF808080`
    // half-grey.
    let grey = 128.0 / 255.0;
    let powered = EntityModelInstance::wither(1471, [0.0, 64.0, 0.0], 0.0)
        .with(|s| s.kind.wither_mut().powered = true);
    let rest = entity_model_textured_meshes(&[powered], &atlas);

    assert_eq!(rest.scroll_additive.vertices.len(), 216);
    assert!(rest
        .scroll_additive
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [grey, grey, grey, 1.0]));

    // The inflated armor (`INNER_ARMOR_DEFORMATION` = CubeDeformation 0.5) floats just outside the
    // body: under the same `2.0×` root transform, its X extent exceeds the base body's.
    let base_x_max = rest
        .cutout
        .vertices
        .iter()
        .map(|vertex| vertex.position[0])
        .fold(f32::MIN, f32::max);
    let armor_x_max = rest
        .scroll_additive
        .vertices
        .iter()
        .map(|vertex| vertex.position[0])
        .fold(f32::MIN, f32::max);
    assert!(
        armor_x_max > base_x_max,
        "the inflated armor is wider than the body ({armor_x_max} vs {base_x_max})"
    );

    // Vanilla `WitherArmorLayer.xOffset(t) = cos(t·0.02)·3` on U (oscillating, not the creeper's
    // linear scroll), `t·0.01` on V, each `% 1`. At `t = 0` the U offset is `3 % 1 = 0`, so `rest`
    // carries the base local UVs; a later age shifts both axes. The body breathes with age, but the
    // local UVs are pose-independent, so they track the offset exactly.
    let age = 50.0_f32;
    let scrolled = entity_model_textured_meshes(
        &[
            EntityModelInstance::wither(1471, [0.0, 64.0, 0.0], 0.0).with(|s| {
                s.kind.wither_mut().powered = true;
                s.age_in_ticks = age;
            }),
        ],
        &atlas,
    );
    let u_expected = ((age * 0.02).cos() * 3.0) % 1.0;
    let v_expected = (age * 0.01).rem_euclid(1.0);
    assert!(u_expected != 0.0 && v_expected > 0.0);
    for (rest_vertex, scrolled_vertex) in rest
        .scroll_additive
        .vertices
        .iter()
        .zip(&scrolled.scroll_additive.vertices)
    {
        assert!(
            (scrolled_vertex.local_uv[0] - (rest_vertex.local_uv[0] + u_expected)).abs() < 1.0e-6
        );
        assert!(
            (scrolled_vertex.local_uv[1] - (rest_vertex.local_uv[1] + v_expected)).abs() < 1.0e-6
        );
    }
}

#[test]
fn powered_wither_energy_swirl_missing_armor_texture_preserves_other_geometry() {
    let len = usize::try_from(WITHER_TEXTURE_REF.size[0] * WITHER_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        WITHER_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    let powered = EntityModelInstance::wither(1472, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.kind.wither_mut().powered = true;
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[powered], &atlas);

    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.scroll.vertices.is_empty());

    assert_eq!(meshes.cutout.vertices.len(), 216);

    assert!(
        meshes.scroll_additive.vertices.is_empty(),
        "missing wither_armor.png suppresses only folded energy-swirl geometry"
    );
}
