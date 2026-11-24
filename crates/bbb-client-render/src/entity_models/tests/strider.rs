use super::*;

fn entity_model_textured_meshes(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    super::entity_model_textured_meshes_with_static_equipment_fixture(instances, atlas)
}

fn entity_model_textured_meshes_with_dynamic_textures(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    skins: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    textures: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> EntityModelTexturedMeshes {
    super::entity_model_textured_meshes_with_static_equipment_and_dynamic_textures_fixture(
        instances, atlas, skins, textures,
    )
}

use crate::DynamicEntityTextureImage;

#[test]
fn strider_setup_anim_curves_match_vanilla() {
    // `animationSpeed = min(walkAnimationSpeed, 0.25)`.
    assert!((strider_animation_speed(0.1) - 0.1).abs() < 1.0e-6);
    assert!(
        (strider_animation_speed(0.8) - 0.25).abs() < 1.0e-6,
        "clamped"
    );

    let (pos, speed, age) = (3.0_f32, 0.2_f32, 11.0_f32);

    // `body.zRot = 0.1·sin(pos·1.5)·4·speed`.
    let expected_body_z = 0.1 * (pos * 1.5).sin() * 4.0 * speed;
    assert!((strider_body_z_rot(pos, speed) - expected_body_z).abs() < 1.0e-6);

    // Leg swing/roll: left uses phase 0, right uses phase π.
    let expected_left_x = (pos * 0.75).sin() * 2.0 * speed;
    let expected_right_x = (pos * 0.75 + std::f32::consts::PI).sin() * 2.0 * speed;
    assert!((strider_leg_x_rot(pos, speed, false) - expected_left_x).abs() < 1.0e-6);
    assert!((strider_leg_x_rot(pos, speed, true) - expected_right_x).abs() < 1.0e-6);
    let expected_left_z = (std::f32::consts::PI / 18.0) * (pos * 0.75).cos() * speed;
    assert!((strider_leg_z_rot(pos, speed, false) - expected_left_z).abs() < 1.0e-6);

    // Body bob (adult mul 2, baby mul 1).
    let expected_adult_body_y = 2.0 - 2.0 * (pos * 1.5).cos() * 2.0 * speed;
    assert!((strider_body_y(2.0, 2.0, pos, speed) - expected_adult_body_y).abs() < 1.0e-6);
    let expected_baby_body_y = 17.25 - 1.0 * (pos * 1.5).cos() * 2.0 * speed;
    assert!((strider_body_y(17.25, 1.0, pos, speed) - expected_baby_body_y).abs() < 1.0e-6);

    // Leg lift: right uses phase 0, left uses phase π (opposite of the swing).
    let expected_right_y = 8.0 + 2.0 * (pos * 0.75).sin() * 2.0 * speed;
    let expected_left_y = 8.0 + 2.0 * (pos * 0.75 + std::f32::consts::PI).sin() * 2.0 * speed;
    assert!((strider_leg_y(8.0, pos, speed, true) - expected_right_y).abs() < 1.0e-6);
    assert!((strider_leg_y(8.0, pos, speed, false) - expected_left_y).abs() < 1.0e-6);

    // Bristle flow + per-bristle ripple.
    let flow = (pos * 1.5 + std::f32::consts::PI).cos() * speed;
    assert!((strider_bristle_flow(pos, speed) - flow).abs() < 1.0e-6);
    assert!(
        (strider_bristle_top_flow(flow, age) - (flow * 0.6 + 0.1 * (age * 0.4).sin())).abs()
            < 1.0e-6
    );
    assert!(
        (strider_bristle_middle_flow(flow, age) - (flow * 1.2 + 0.1 * (age * 0.2).sin())).abs()
            < 1.0e-6
    );
    assert!(
        (strider_bristle_bottom_flow(flow, age) - (flow * 1.3 + 0.05 * (age * -0.4).sin())).abs()
            < 1.0e-6
    );
}

#[test]
fn strider_texture_refs_match_vanilla_renderer() {
    // A suffocating (cold) strider swaps to the `strider_cold` texture × age
    // (`StriderRenderer.getTextureLocation`).
    assert_eq!(strider_texture_ref(false, true), STRIDER_COLD_TEXTURE_REF);
    assert_eq!(
        strider_texture_ref(true, true),
        STRIDER_COLD_BABY_TEXTURE_REF
    );
    assert_eq!(
        STRIDER_SADDLE_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/equipment/strider_saddle/saddle.png",
            size: [64, 128],
        }
    );
    assert!(entity_model_texture_refs().contains(&STRIDER_COLD_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&STRIDER_COLD_BABY_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&STRIDER_SADDLE_TEXTURE_REF));
}

#[test]
fn strider_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&strider_texture_images()).unwrap();

    let adult = EntityModelInstance::strider(750, [0.0, 64.0, 0.0], 0.0, false, false).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[adult], &atlas);

    // Nine cubes → 54 faces / 216 vertices, with nothing on the translucent or eyes passes.
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.cutout.cutout_faces, 54);
    assert_eq!(meshes.cutout.vertices.len(), 216);
    assert_eq!(meshes.cutout.indices.len(), 324);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == adult.render_state.shader_light()
            && vertex.overlay == adult.render_state.overlay_coords()));

    // Baby is the smaller model: six cubes → 36 faces / 144 vertices.
    let baby = EntityModelInstance::strider(751, [0.0, 64.0, 0.0], 0.0, true, false).with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);

    assert_eq!(baby_meshes.cutout.vertices.len(), 144);
    assert!(baby_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == baby.render_state.shader_light()
            && vertex.overlay == baby.render_state.overlay_coords()));
    assert!(baby_meshes.armor_cutout.vertices.is_empty());

    // The body tracks the look, and the walk + age animate the legs/body/bristles.
    let looking = entity_model_textured_meshes(
        &[adult.with(|s| (s.head_yaw, s.head_pitch) = (40.0, -25.0))],
        &atlas,
    );
    assert_ne!(meshes.cutout.vertices, looking.cutout.vertices);
    let ridden = entity_model_textured_meshes(
        &[adult
            .with(|s| (s.head_yaw, s.head_pitch) = (40.0, -25.0))
            .with(|s| s.kind.strider_mut().ridden = true)],
        &atlas,
    );
    assert_eq!(
        meshes.cutout.vertices, ridden.cutout.vertices,
        "ridden striders zero the body pitch/yaw in the textured model"
    );
    let walking = entity_model_textured_meshes(
        &[adult.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (3.0, 0.2))],
        &atlas,
    );
    assert_ne!(meshes.cutout.vertices, walking.cutout.vertices);
    let rippled = entity_model_textured_meshes(
        &[adult
            .with(|s| s.age_in_ticks = 13.0)
            .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (3.0, 0.2))],
        &atlas,
    );
    assert_ne!(walking.cutout.vertices, rippled.cutout.vertices);
}

#[test]
fn strider_saddle_layer_renders_for_adults_only() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        STRIDER_TEXTURE_REF,
        STRIDER_BABY_TEXTURE_REF,
        STRIDER_COLD_TEXTURE_REF,
        STRIDER_COLD_BABY_TEXTURE_REF,
        STRIDER_SADDLE_TEXTURE_REF,
    ]))
    .unwrap();

    let adult = EntityModelInstance::strider(752, [0.0, 64.0, 0.0], 0.0, false, false).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let bare_meshes = entity_model_textured_meshes(&[adult], &atlas);
    let saddled_instance = adult.with(|s| {
        s.kind.strider_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id())
    });
    let saddled_meshes = entity_model_textured_meshes(&[saddled_instance], &atlas);
    let bare = &bare_meshes.cutout;
    let saddled = &saddled_meshes.cutout;
    let saddle_mesh = &saddled_meshes.armor_cutout;

    assert_eq!(saddled.cutout_faces, bare.cutout_faces);
    assert_eq!(saddled.vertices.len(), bare.vertices.len());
    assert_eq!(saddle_mesh.cutout_faces, 54);
    assert_eq!(saddle_mesh.vertices.len(), 216);
    assert!(saddled.vertices[..216]
        .iter()
        .all(|vertex| vertex.light == adult.render_state.shader_light()
            && vertex.overlay == adult.render_state.overlay_coords()));
    assert!(saddle_mesh
        .vertices
        .iter()
        .all(|vertex| vertex.light == adult.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]));

    let saddle_uv = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == STRIDER_SADDLE_TEXTURE_REF)
        .unwrap()
        .uv;
    let first_saddle_vertex = saddle_mesh.vertices[0].uv;
    assert!(first_saddle_vertex[0] >= saddle_uv.min[0]);
    assert!(first_saddle_vertex[0] <= saddle_uv.max[0]);
    assert!(first_saddle_vertex[1] >= saddle_uv.min[1]);
    assert!(first_saddle_vertex[1] <= saddle_uv.max[1]);

    let baby = EntityModelInstance::strider(753, [0.0, 64.0, 0.0], 0.0, true, false).with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.strider_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
    });
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);

    assert_eq!(
        baby_meshes.cutout.cutout_faces, 36,
        "vanilla supplies no baby model for the strider saddle layer"
    );
    assert_eq!(baby_meshes.cutout.vertices.len(), 144);
    assert!(baby_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == baby.render_state.shader_light()
            && vertex.overlay == baby.render_state.overlay_coords()));
}

#[test]
fn strider_saddle_missing_saddle_texture_texture_preserves_other_geometry() {
    // Vanilla `StriderRenderer` records `SimpleEquipmentLayer(STRIDER_SADDLE)` with the default
    // collector order, after the base body, and forces `OverlayTexture.NO_OVERLAY`.
    let images = texture_images(&[STRIDER_TEXTURE_REF]);
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance =
        EntityModelInstance::strider(754, [0.0, 64.0, 0.0], 0.0, false, false).with(|s| {
            s.light_coords = (5_u32 << 4) | (11_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
            s.kind.strider_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
        });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(
        meshes.cutout.vertices.len(),
        216,
        "missing strider_saddle/saddle.png suppresses only folded saddle geometry"
    );
    assert!(meshes.armor_cutout.vertices.is_empty());
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));

    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
}

#[test]
fn strider_saddle_foil_and_trim_follow_the_base_equipment_layer() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        STRIDER_TEXTURE_REF,
        STRIDER_BABY_TEXTURE_REF,
        STRIDER_SADDLE_TEXTURE_REF,
        ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
    ]))
    .unwrap();
    let trim_texture = EntityDynamicTexture { handle: 75_401 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [64, 128],
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 128 * 4],
    }])
    .unwrap()
    .0;
    let trimmed_saddle = |entity_id, baby| {
        EntityModelInstance::strider(entity_id, [0.0, 64.0, 0.0], 0.0, baby, false).with(|s| {
            s.kind.strider_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
            s.kind.strider_mut().saddle_foil = true;
            s.kind.strider_mut().saddle_trim = Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            });
        })
    };

    let adult = trimmed_saddle(755, false);
    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[adult],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_eq!(meshes.armor_cutout.vertices.len(), 216);
    assert_eq!(meshes.armor_entity_glint.vertices.len(), 216);
    assert_eq!(meshes.dynamic_player_texture_armor_trim.vertices.len(), 216);

    let baby = trimmed_saddle(756, true);
    let baby_meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[baby],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert!(baby_meshes.armor_cutout.vertices.is_empty());
    assert!(baby_meshes.armor_entity_glint.vertices.is_empty());
    assert!(baby_meshes
        .dynamic_player_texture_armor_trim
        .vertices
        .is_empty());
}

fn strider_texture_images() -> Vec<EntityModelTextureImage> {
    texture_images(&[
        STRIDER_TEXTURE_REF,
        STRIDER_BABY_TEXTURE_REF,
        STRIDER_COLD_TEXTURE_REF,
        STRIDER_COLD_BABY_TEXTURE_REF,
    ])
}

fn texture_images(textures: &[EntityModelTextureRef]) -> Vec<EntityModelTextureImage> {
    textures
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
