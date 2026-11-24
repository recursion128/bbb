use super::*;
use glam::{Mat4, Vec3};

#[test]
fn armor_stand_texture_refs_match_vanilla_renderer() {
    assert!(entity_model_texture_refs().contains(&ARMOR_STAND_TEXTURE_REF));
}

#[test]
fn armor_stand_textured_mesh_submits_one_cutout_pass() {
    let (atlas, _) = build_entity_model_texture_atlas(&armor_stand_texture_images()).unwrap();
    for (small, show_arms, show_base_plate) in [
        (false, false, true),
        (false, true, false),
        (true, true, true),
    ] {
        let mut pose = DEFAULT_ARMOR_STAND_MODEL_POSE;
        pose.head = [0.0, 45.0, 0.0];
        pose.body = [0.0, 0.0, 12.0];
        let instance = EntityModelInstance::armor_stand(
            5,
            [0.0, 64.0, 0.0],
            0.0,
            small,
            show_arms,
            show_base_plate,
            pose,
        );
        let instances = [instance];
        let textured_meshes = entity_model_textured_meshes(&instances, &atlas);
        assert!(textured_meshes.translucent.vertices.is_empty());
        assert!(textured_meshes.eyes.vertices.is_empty());

        let textured = &textured_meshes.cutout;

        assert!(textured
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    }
}

#[test]
fn armor_stand_hit_wiggle_applies_vanilla_setup_rotation_to_base_submission() {
    let wiggle = 1.25_f32;
    let instance = EntityModelInstance::armor_stand(
        5,
        [0.0, 64.0, 0.0],
        30.0,
        false,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| s.kind.armor_stand_mut().wiggle = wiggle);
    let rotation = (wiggle / 1.5 * std::f32::consts::PI).sin() * 3.0;
    let expected = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_scale(Vec3::splat(instance.render_state.scale))
        * Mat4::from_rotation_y((180.0_f32 - instance.render_state.body_rot).to_radians())
        * Mat4::from_rotation_y(rotation.to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.501, 0.0));

    assert_close_transform(entity_model_root_transform(&instance), expected);

    let settled = instance.with(|s| s.kind.armor_stand_mut().wiggle = 5.0);
    assert_eq!(
        entity_model_root_transform(&settled),
        entity_model_root_transform(&EntityModelInstance::armor_stand(
            5,
            [0.0, 64.0, 0.0],
            30.0,
            false,
            true,
            true,
            DEFAULT_ARMOR_STAND_MODEL_POSE,
        ))
    );
}

#[test]
fn armor_stand_marker_render_type_follows_vanilla_visibility_branch() {
    let (atlas, _) = build_entity_model_texture_atlas(&armor_stand_texture_images()).unwrap();
    let visible_marker = EntityModelInstance::armor_stand_with_marker(
        9,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let visible = entity_model_textured_meshes(&[visible_marker], &atlas);

    assert!(!visible.cutout.vertices.is_empty());
    assert!(visible.cutout.vertices.iter().all(|vertex| vertex.light
        == visible_marker.render_state.shader_light()
        && vertex.overlay == visible_marker.render_state.overlay_coords()));
    assert!(visible.translucent.vertices.is_empty());

    let hidden_invisible = visible_marker.with_invisible(true);
    let hidden = entity_model_textured_meshes(&[hidden_invisible], &atlas);

    assert!(hidden.cutout.vertices.is_empty());
    assert!(hidden.translucent.vertices.is_empty());

    let self_visible_invisible = hidden_invisible.with(|s| s.invisible_to_player = false);
    let self_visible = entity_model_textured_meshes(&[self_visible_invisible], &atlas);

    // Vanilla `ArmorStandRenderer.getRenderType`: marker force-transparent uses
    // `RenderTypes.entityTranslucent(texture, false)`, not the generic living
    // `entityTranslucentCullItemTarget`.

    assert!(self_visible.cutout.vertices.is_empty());
    assert!(!self_visible.translucent.vertices.is_empty());
    assert!(self_visible
        .translucent
        .vertices
        .iter()
        .all(
            |vertex| vertex.light == self_visible_invisible.render_state.shader_light()
                && vertex.overlay == self_visible_invisible.render_state.overlay_coords()
        ));

    let glowing_hidden_marker = hidden_invisible.with_outline_color(0xff44_88cc);
    let glowing_hidden_marker_meshes =
        entity_model_textured_meshes(&[glowing_hidden_marker], &atlas);

    assert!(glowing_hidden_marker_meshes.cutout.vertices.is_empty());
    assert!(glowing_hidden_marker_meshes.translucent.vertices.is_empty());
    assert!(glowing_hidden_marker_meshes.eyes.vertices.is_empty());
}

#[test]
fn non_marker_armor_stand_uses_living_invisible_outline_branch() {
    let (atlas, _) = build_entity_model_texture_atlas(&armor_stand_texture_images()).unwrap();
    let hidden_glowing = EntityModelInstance::armor_stand_with_marker(
        10,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        true,
        true,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with_invisible(true)
    .with_outline_color(0xff22_aa66)
    .with(|s| {
        s.light_coords = (3_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[hidden_glowing], &atlas);

    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}

fn armor_stand_texture_images() -> Vec<EntityModelTextureImage> {
    [ARMOR_STAND_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn assert_close_transform(actual: Mat4, expected: Mat4) {
    for (a, e) in actual
        .to_cols_array()
        .into_iter()
        .zip(expected.to_cols_array())
    {
        assert!(
            (a - e).abs() < 1.0e-5,
            "transform mismatch: {actual:?} != {expected:?}"
        );
    }
}
