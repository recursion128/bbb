use super::*;

#[test]
fn witch_textured_layer_pass_matches_vanilla_renderer_model_layer() {
    let passes = witch_textured_layer_passes();

    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].texture, WITCH_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
}

#[test]
fn witch_texture_atlas_stitches_official_png_slot() {
    let (layout, rgba) = build_entity_model_texture_atlas(&witch_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 128);
    assert_eq!(layout.entries.len(), 1);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/witch/witch.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
}

#[test]
fn witch_textured_mesh_uses_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&witch_texture_images()).unwrap();
    let instance = EntityModelInstance::witch(66, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (6_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert_witch_textured_buckets(&meshes);
    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 90);
    assert_eq!(mesh.vertices.len(), 360);
    assert_eq!(mesh.indices.len(), 540);
    assert_close2(mesh.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(mesh.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
    assert_ne!(instance.render_state.overlay_coords(), [0.0, 10.0]);
    let (min, max) = textured_mesh_extents(mesh);
    assert_close3(min, [-0.76171875, 64.00094, -0.29296878]);
    assert_close3(max, [0.46875003, 66.56483, 0.7617188]);
}

#[test]
fn witch_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&witch_texture_images()).unwrap();
    let base = EntityModelInstance::witch(67, [0.0, 64.0, 0.0], 0.0);
    let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (45.0, 0.0));
    let pitched_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
    let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
    assert_witch_textured_buckets(&resting);
    assert_witch_textured_buckets(&yawed);
    assert_witch_textured_buckets(&pitched);

    // Witch head is part 0; head look turns it without changing vertex count.
    assert_eq!(resting.cutout.vertices.len(), yawed.cutout.vertices.len());
    assert_ne!(resting.cutout.vertices, yawed.cutout.vertices);
    assert_ne!(yawed.cutout.vertices, pitched.cutout.vertices);
}

#[test]
fn witch_textured_mesh_swings_legs_when_walking() {
    // The real witch render path (texture-backed) swings the same half-amplitude
    // legs. A standing witch is byte-identical however far the swing has advanced; a
    // walking one lifts its feet.
    let (atlas, _) = build_entity_model_texture_atlas(&witch_texture_images()).unwrap();
    let base = EntityModelInstance::witch(311, [0.0, 64.0, 0.0], 0.0);
    let still_instance = base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
    let walking_instance =
        base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let still = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_witch_textured_buckets(&resting);
    assert_witch_textured_buckets(&still);
    assert_witch_textured_buckets(&walking);

    assert_eq!(
        resting.cutout.vertices, still.cutout.vertices,
        "a standing witch is inert"
    );
    assert_eq!(
        resting.cutout.vertices.len(),
        walking.cutout.vertices.len(),
        "leg swing keeps the vertex count"
    );
    assert_ne!(
        resting.cutout.vertices, walking.cutout.vertices,
        "a walking witch differs"
    );

    let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
    let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
    assert!(
        (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
        "a walking witch's feet should lift off the ground"
    );
}

#[test]
fn witch_nose_bob_pose_matches_vanilla_formula() {
    // Vanilla `WitchModel.setupAnim`:
    //   speed = 0.01 * (entityId % 10);
    //   nose.xRot = sin(ageInTicks * speed) * 4.5°;
    //   nose.zRot = cos(ageInTicks * speed) * 2.5°.
    // Both are SET absolutely (the base nose pose carries rotation [0, 0, 0]); the yRot and
    // the offset are preserved. The nose is the head's `nose` child (after the hat chain).
    let base = layer_pose(ModelLayer::WITCH, "nose");

    // entityId 66 → speed 0.06; ageInTicks 40 reproduces the sin/cos amplitudes exactly.
    let (entity_id, age) = (66_i32, 40.0_f32);
    let speed = 0.01 * (entity_id % 10) as f32;
    let phase = age * speed;
    let posed = witch_nose_bob_pose(base, age, entity_id);
    assert!((posed.rotation[0] - phase.sin() * 4.5_f32.to_radians()).abs() < 1e-7);
    assert!((posed.rotation[2] - phase.cos() * 2.5_f32.to_radians()).abs() < 1e-7);
    assert_eq!(posed.rotation[1], base.rotation[1]);
    assert_eq!(posed.offset, base.offset);

    // At age 0 the bob is not neutral: zRot = cos(0) * 2.5° = 2.5°, xRot = sin(0) = 0.
    let at_zero = witch_nose_bob_pose(base, 0.0, entity_id);
    assert_eq!(at_zero.rotation[0], 0.0);
    assert!((at_zero.rotation[2] - 2.5_f32.to_radians()).abs() < 1e-7);

    // entityId % 10 == 0 freezes the bob (speed 0): the nose holds the age-0 pose forever.
    let frozen = witch_nose_bob_pose(base, age, 310);
    assert_eq!(frozen.rotation[0], 0.0);
    assert!((frozen.rotation[2] - 2.5_f32.to_radians()).abs() < 1e-7);
}

#[test]
fn witch_textured_mesh_bobs_its_nose_as_age_advances() {
    // The texture-backed render path bobs the same nose. entityId 313 → speed 0.03.
    let (atlas, _) = build_entity_model_texture_atlas(&witch_texture_images()).unwrap();
    let base = EntityModelInstance::witch(313, [0.0, 64.0, 0.0], 0.0);
    let later_instance = base.with(|s| s.age_in_ticks = 31.4);
    let early = entity_model_textured_meshes(&[base], &atlas);
    let later = entity_model_textured_meshes(&[later_instance], &atlas);
    assert_witch_textured_buckets(&early);
    assert_witch_textured_buckets(&later);
    assert_eq!(early.cutout.vertices.len(), later.cutout.vertices.len());
    assert_ne!(
        early.cutout.vertices, later.cutout.vertices,
        "the nose bobs as ageInTicks advances"
    );
    const LEGS: &[&str] = &["right_leg", "left_leg"];
    let model = ModelLayer::WITCH.bake();
    assert_eq!(
        parts_vertices(&early.cutout.vertices, &model, LEGS),
        parts_vertices(&later.cutout.vertices, &model, LEGS),
        "the legs do not depend on ageInTicks"
    );
}

fn assert_witch_textured_buckets(meshes: &EntityModelTexturedMeshes) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_cutout.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
}

fn witch_texture_images() -> Vec<EntityModelTextureImage> {
    [WITCH_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
