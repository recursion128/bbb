use super::*;

use crate::entity_models::model::EntityModel;

#[test]
fn wither_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `WitherBossModel.createBodyLayer(CubeDeformation.NONE)` (atlas 64×64): six sibling
    // root parts — shoulders, ribcage (spine + three ribs), tail, center head, two side heads.

    // `shoulders` (20×3×3) at ZERO, texOffs(0,16).
    assert_eq!(WITHER_SHOULDERS_POSE.offset, [0.0, 0.0, 0.0]);
    assert_eq!(WITHER_SHOULDERS_CUBES[0].min, [-10.0, 3.9, -0.5]);
    assert_eq!(WITHER_SHOULDERS_CUBES[0].size, [20.0, 3.0, 3.0]);
    assert_eq!(WITHER_SHOULDERS_CUBES[0].tex, [0.0, 16.0]);

    // `ribcage` (offset (-2, 6.9, -0.5), pitched 0.20420352 rad): the spine plus three rib bars.
    assert_eq!(WITHER_RIBCAGE_POSE.offset, [-2.0, 6.9, -0.5]);
    assert_eq!(WITHER_RIBCAGE_POSE.rotation, [0.204_203_52, 0.0, 0.0]);
    assert_eq!(WITHER_RIBCAGE_CUBES.len(), 4);
    assert_eq!(WITHER_RIBCAGE_CUBES[0].size, [3.0, 10.0, 3.0]);
    assert_eq!(WITHER_RIBCAGE_CUBES[0].tex, [0.0, 22.0]);
    assert_eq!(WITHER_RIBCAGE_CUBES[1].min, [-4.0, 1.5, 0.5]);
    assert_eq!(WITHER_RIBCAGE_CUBES[2].min, [-4.0, 4.0, 0.5]);
    assert_eq!(WITHER_RIBCAGE_CUBES[3].min, [-4.0, 6.5, 0.5]);
    assert_eq!(WITHER_RIBCAGE_CUBES[1].size, [11.0, 2.0, 2.0]);
    assert!(WITHER_RIBCAGE_CUBES[1..]
        .iter()
        .all(|cube| cube.tex == [24.0, 22.0]));

    // `tail` (3×6×3) at the bind position derived from the ribcage bind pitch, texOffs(12,22).
    let ribcage_bind_xrot = 0.20420352_f32;
    let expected_tail_y = 6.9 + ribcage_bind_xrot.cos() * 10.0;
    let expected_tail_z = -0.5 + ribcage_bind_xrot.sin() * 10.0;
    assert!((WITHER_TAIL_POSE.offset[1] - expected_tail_y).abs() < 1.0e-4);
    assert!((WITHER_TAIL_POSE.offset[2] - expected_tail_z).abs() < 1.0e-4);
    assert_eq!(WITHER_TAIL_POSE.rotation, [0.832_522_03, 0.0, 0.0]);
    assert_eq!(WITHER_TAIL_CUBES[0].size, [3.0, 6.0, 3.0]);
    assert_eq!(WITHER_TAIL_CUBES[0].tex, [12.0, 22.0]);

    // `center_head` (8×8×8) at ZERO; the two 6×6×6 side heads at their pivots.
    assert_eq!(WITHER_CENTER_HEAD_POSE.offset, [0.0, 0.0, 0.0]);
    assert_eq!(WITHER_CENTER_HEAD_CUBES[0].size, [8.0, 8.0, 8.0]);
    assert_eq!(WITHER_CENTER_HEAD_CUBES[0].tex, [0.0, 0.0]);
    assert_eq!(WITHER_RIGHT_HEAD_POSE.offset, [-8.0, 4.0, 0.0]);
    assert_eq!(WITHER_LEFT_HEAD_POSE.offset, [10.0, 4.0, 0.0]);
    assert_eq!(WITHER_SIDE_HEAD_CUBES[0].size, [6.0, 6.0, 6.0]);
    assert_eq!(WITHER_SIDE_HEAD_CUBES[0].tex, [32.0, 0.0]);
}

#[test]
fn wither_mesh_uses_vanilla_body_layer_geometry() {
    // 9 cubes → 54 faces / 216 vertices / 324 indices; the body carries the body tint and the three
    // heads carry the head tint.
    let wither = entity_model_mesh(&[EntityModelInstance::wither(1450, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(wither.opaque_faces, 54);
    assert_eq!(wither.vertices.len(), 216);
    assert_eq!(wither.indices.len(), 324);
    assert!(wither
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WITHER_BODY, 1.0)));
    assert!(wither
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WITHER_HEAD, 1.0)));
}

#[test]
fn wither_center_head_follows_look_angles() {
    // Vanilla `WitherBossModel.setupAnim` sets `centerHead.yRot/xRot` from the net head look. The
    // center head is part 3 (vertices [144, 168)): one cube each for shoulders (1), ribcage (4), tail
    // (1) precede it (24·6 = 144). A non-zero look re-poses only those vertices; the shoulders /
    // ribcage / tail and the two side heads (which track the separate `DATA_TARGET_*` heads) stay at
    // bind.
    let base = EntityModelInstance::wither(1451, [0.0, 64.0, 0.0], 0.0);
    let looking = base.with_head_look(35.0, -20.0);
    let rest = entity_model_mesh(&[base]);
    let turned = entity_model_mesh(&[looking]);
    assert_ne!(
        rest.vertices[144..168],
        turned.vertices[144..168],
        "the center head turns with the look angles"
    );
    assert_eq!(
        rest.vertices[..144],
        turned.vertices[..144],
        "the shoulders, ribcage, and tail are unmoved by the look (their shared age breathes both)"
    );
    assert_eq!(
        rest.vertices[168..],
        turned.vertices[168..],
        "the two side heads stay at bind without side-head target rotations"
    );
}

#[test]
fn wither_side_heads_follow_vanilla_target_rotation_arrays() {
    // Vanilla `WitherBossModel.setupHeadRotation`:
    //   head.yRot = (state.yHeadRots[index] - state.bodyRot) * PI / 180
    //   head.xRot = state.xHeadRots[index] * PI / 180
    let instance = EntityModelInstance::wither(1452, [0.0, 64.0, 0.0], 30.0)
        .with_wither_x_head_rots([-12.0, 25.0])
        .with_wither_y_head_rots([50.0, -10.0]);
    let mut model = WitherModel::new();
    model.prepare(&instance);

    let right = model.root_mut().child_mut("right_head").pose.rotation;
    assert!((right[0] - (-12.0_f32).to_radians()).abs() < 1.0e-6);
    assert!((right[1] - 20.0_f32.to_radians()).abs() < 1.0e-6);

    let left = model.root_mut().child_mut("left_head").pose.rotation;
    assert!((left[0] - 25.0_f32.to_radians()).abs() < 1.0e-6);
    assert!((left[1] - (-40.0_f32).to_radians()).abs() < 1.0e-6);

    model.prepare(&EntityModelInstance::wither(1453, [0.0, 64.0, 0.0], 0.0));
    assert_eq!(
        model.root_mut().child_mut("right_head").pose.rotation,
        [0.0, 0.0, 0.0],
        "prepare must reset the previous frame's side-head target pose"
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
    let rib_bind = WITHER_RIBCAGE_POSE;
    let tail_bind = WITHER_TAIL_POSE;
    assert!((rib_rest.rotation[0] - rib_bind.rotation[0]).abs() < 1.0e-5);
    assert!((tail_rest.offset[1] - tail_bind.offset[1]).abs() < 1.0e-4);
    assert!((tail_rest.offset[2] - tail_bind.offset[2]).abs() < 1.0e-4);
    assert!((tail_rest.rotation[0] - tail_bind.rotation[0]).abs() < 1.0e-5);
}

#[test]
fn wither_ribcage_and_tail_breathe_with_age() {
    // The ribcage (cubes [24, 120)) and tail (cubes [120, 144)) sway off `ageInTicks`; the shoulders
    // (cubes [0, 24)) and the three heads (cubes [144, 216)) carry no breathing. Two distinct ages,
    // with the look at rest, re-pose only the ribcage and tail.
    let young = EntityModelInstance::wither(1460, [0.0, 64.0, 0.0], 0.0).with_age_in_ticks(3.0);
    let old = EntityModelInstance::wither(1460, [0.0, 64.0, 0.0], 0.0).with_age_in_ticks(11.0);
    let young_mesh = entity_model_mesh(&[young]);
    let old_mesh = entity_model_mesh(&[old]);
    assert_ne!(
        young_mesh.vertices[24..120],
        old_mesh.vertices[24..120],
        "the ribcage breathes with age"
    );
    assert_ne!(
        young_mesh.vertices[120..144],
        old_mesh.vertices[120..144],
        "the tail breathes with age"
    );
    assert_eq!(
        young_mesh.vertices[..24],
        old_mesh.vertices[..24],
        "the shoulders never breathe"
    );
    assert_eq!(
        young_mesh.vertices[144..],
        old_mesh.vertices[144..],
        "the three heads never breathe"
    );
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

    // The catalog's static mapping is the base texture (the invulnerable variant is render-state
    // driven, picked per-instance in the dispatch).
    assert_eq!(
        EntityModelKind::Wither.vanilla_texture_ref(),
        Some(WITHER_TEXTURE_REF)
    );
    assert!(entity_model_texture_refs().contains(&WITHER_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&WITHER_INVULNERABLE_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&WITHER_ARMOR_TEXTURE_REF));
    assert_eq!(
        wither_entity_texture_refs(),
        &[
            WITHER_TEXTURE_REF,
            WITHER_INVULNERABLE_TEXTURE_REF,
            WITHER_ARMOR_TEXTURE_REF
        ]
    );

    let images: Vec<EntityModelTextureImage> = wither_entity_texture_refs()
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
        let instance = EntityModelInstance::wither(1450, [0.0, 64.0, 0.0], 0.0)
            .with_wither_invulnerable_ticks(invulnerable_ticks);
        let meshes = entity_model_textured_meshes(&[instance], &atlas);
        assert!(meshes.translucent.vertices.is_empty());
        assert!(meshes.eyes.vertices.is_empty());
        assert_eq!(meshes.submissions.len(), 1);
        let submit = meshes.submissions[0];
        assert_eq!(submit.texture, expected_texture);
        assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(submit.transform, wither_model_root_transform(instance));
        assert_eq!((submit.order, submit.submit_sequence), (0, 0));
        let mesh = &meshes.cutout;

        assert!(!mesh.vertices.is_empty());
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    }
}

#[test]
fn wither_renders_at_vanilla_2x_scale_and_shrinks_during_spawn() {
    // Vanilla `WitherBossRenderer.scale`: a flat `2.0×`, minus `invulnerableTicks / 220 * 0.5` while
    // spawning. So a fully-spawned wither is twice the bare model extent, and a freshly-summoned one
    // (`220` ticks → `1.5×`) is smaller, growing to full over the spawn charge.
    let spawned = entity_model_mesh(&[EntityModelInstance::wither(1451, [0.0, 64.0, 0.0], 0.0)]);
    let charging = entity_model_mesh(&[EntityModelInstance::wither(1452, [0.0, 64.0, 0.0], 0.0)
        .with_wither_invulnerable_ticks(220.0)]);
    assert_eq!(spawned.vertices.len(), charging.vertices.len());
    assert_ne!(
        spawned.vertices, charging.vertices,
        "the charging wither is scaled down"
    );

    let (s_min, s_max) = mesh_extents(&spawned);
    let (c_min, c_max) = mesh_extents(&charging);
    let spawned_width = s_max[0] - s_min[0];
    let charging_width = c_max[0] - c_min[0];
    // 1.5/2.0 = 0.75 of the spawned width.
    assert!(
        (charging_width / spawned_width - 0.75).abs() < 1.0e-3,
        "the charging wither is 1.5/2.0 of the spawned size ({charging_width} vs {spawned_width})"
    );
}

#[test]
fn powered_wither_emits_scrolling_energy_swirl() {
    // Build an atlas covering the wither's base, invulnerable, and armor textures.
    let images: Vec<EntityModelTextureImage> = wither_entity_texture_refs()
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
    assert!(!plain
        .submissions
        .iter()
        .any(|submit| submit.render_type == EntityModelLayerRenderType::EnergySwirl));

    // A powered wither (≤ half health) draws the inflated `WITHER_ARMOR` model (9 cubes → 216
    // vertices) into the additive scroll mesh, every vertex tinted by the vanilla `0xFF808080`
    // half-grey.
    let grey = 128.0 / 255.0;
    let powered =
        EntityModelInstance::wither(1471, [0.0, 64.0, 0.0], 0.0).with_wither_powered(true);
    let rest = entity_model_textured_meshes(&[powered], &atlas);
    let swirl = rest
        .submissions
        .iter()
        .find(|submit| submit.render_type == EntityModelLayerRenderType::EnergySwirl)
        .expect("powered wither emits an energySwirl submit");
    assert_eq!(swirl.texture, WITHER_ARMOR_TEXTURE_REF);
    assert_eq!(swirl.tint, [grey, grey, grey, 1.0]);
    assert_eq!(swirl.order, 1);
    assert_eq!(swirl.submit_sequence, 1);
    assert_eq!(swirl.transform, wither_model_root_transform(powered));
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
        &[EntityModelInstance::wither(1471, [0.0, 64.0, 0.0], 0.0)
            .with_wither_powered(true)
            .with_age_in_ticks(age)],
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
