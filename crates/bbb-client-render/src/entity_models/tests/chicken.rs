use super::*;

#[test]
fn chicken_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let adult_temperate = chicken_textured_layer_passes(ChickenModelVariant::Temperate, false);
    assert_eq!(adult_temperate.len(), 1);
    assert_eq!(
        adult_temperate[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(
        adult_temperate[0].render_type.vanilla_name(),
        "entityCutout"
    );
    assert_eq!(adult_temperate[0].texture, CHICKEN_TEMPERATE_TEXTURE_REF);
    assert_eq!(adult_temperate[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(adult_temperate[0].order, 0);

    let adult_warm = chicken_textured_layer_passes(ChickenModelVariant::Warm, false);
    assert_eq!(adult_warm[0].texture, CHICKEN_WARM_TEXTURE_REF);

    let adult_cold = chicken_textured_layer_passes(ChickenModelVariant::Cold, false);
    assert_eq!(adult_cold[0].texture, CHICKEN_COLD_TEXTURE_REF);

    let baby_warm = chicken_textured_layer_passes(ChickenModelVariant::Warm, true);
    assert_eq!(baby_warm[0].texture, CHICKEN_WARM_BABY_TEXTURE_REF);
}

#[test]
fn entity_texture_atlas_stitches_official_chicken_png_slots() {
    let images = texture_refs_matching(|path| path.starts_with("textures/entity/chicken/"))
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect::<Vec<_>>();

    let (layout, rgba) = build_entity_model_texture_atlas(&images).unwrap();

    let textures = &texture_refs_matching(|path| path.starts_with("textures/entity/chicken/"));
    assert_texture_atlas_layout_matches_refs(&layout, textures);
    let warm_first_pixel =
        rgba_offset(layout.width, texture_atlas_entry_y(textures, 2), 0, "test").unwrap();
    assert_eq!(&rgba[warm_first_pixel..warm_first_pixel + 4], &[2; 4]);
    let cold_baby_first_pixel =
        rgba_offset(layout.width, texture_atlas_entry_y(textures, 5), 0, "test").unwrap();
    assert_eq!(
        &rgba[cold_baby_first_pixel..cold_baby_first_pixel + 4],
        &[5; 4]
    );
}

#[test]
fn chicken_textured_mesh_uses_vanilla_uvs_tints_and_variant_textures() {
    let (atlas, _) = build_entity_model_texture_atlas(&chicken_texture_images()).unwrap();
    let instances = [
        EntityModelInstance::chicken_variant(
            401,
            [0.0, 64.0, 0.0],
            0.0,
            ChickenModelVariant::Temperate,
            false,
        )
        .with(|s| {
            s.light_coords = (6_u32 << 4) | (10_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        }),
        EntityModelInstance::chicken_variant(
            402,
            [1.0, 64.0, 0.0],
            0.0,
            ChickenModelVariant::Cold,
            false,
        )
        .with(|s| {
            s.light_coords = (6_u32 << 4) | (10_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        }),
        EntityModelInstance::chicken_variant(
            403,
            [2.0, 64.0, 0.0],
            0.0,
            ChickenModelVariant::Warm,
            true,
        )
        .with(|s| {
            s.light_coords = (6_u32 << 4) | (10_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        }),
    ];
    let meshes = entity_model_textured_meshes(&instances, &atlas);
    assert_chicken_textured_output(&meshes);
    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 156);
    assert_eq!(mesh.vertices.len(), 624);
    assert_eq!(mesh.indices.len(), 936);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(mesh.vertices[192].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(mesh.vertices[432].tint, [1.0, 1.0, 1.0, 1.0]);
    assert!(mesh.vertices.iter().all(|vertex| vertex.light
        == instances[0].render_state.shader_light()
        && vertex.overlay == instances[0].render_state.overlay_coords()));
    assert_ne!(instances[0].render_state.overlay_coords(), [0.0, 10.0]);
}

#[test]
fn chicken_textured_mesh_swings_its_legs_when_walking() {
    // The real (texture-backed) chicken render path swings the same legs. A standing
    // chicken is byte-identical however far the swing has advanced; a walking one lifts
    // its feet while keeping the vertex count.
    let (atlas, _) = build_entity_model_texture_atlas(&chicken_texture_images()).unwrap();
    for base in [
        EntityModelInstance::chicken(426, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::chicken_variant(
            428,
            [0.0, 64.0, 0.0],
            0.0,
            ChickenModelVariant::Cold,
            false,
        ),
        EntityModelInstance::chicken(427, [0.0, 64.0, 0.0], 0.0, true),
    ] {
        let still_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
        let walking_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_chicken_textured_output(&resting);
        assert_chicken_textured_output(&still);
        assert_chicken_textured_output(&walking);

        assert_eq!(
            resting.cutout.vertices, still.cutout.vertices,
            "{:?} is inert",
            base.kind
        );
        assert_eq!(
            resting.cutout.vertices.len(),
            walking.cutout.vertices.len(),
            "{:?} leg swing keeps the vertex count",
            base.kind
        );
        let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
        let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "{:?} feet should lift off the ground (textured)",
            base.kind
        );
    }
}

fn assert_chicken_textured_output(meshes: &EntityModelTexturedMeshes) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}

#[test]
fn chicken_leg_swing_matches_vanilla_humanoid_phase() {
    // Vanilla ChickenModel.setupAnim: rightLeg.xRot = cos(pos*0.6662)*1.4*speed,
    // leftLeg.xRot = cos(pos*0.6662+π)*1.4*speed — the HumanoidModel phase. The right leg
    // (offset x < 0) is in phase, the left leg (x > 0) out of phase. Only xRot moves. The adult,
    // cold, and headless-baby layers share the same phase logic (the adult legs sit at x = ±,
    // the baby legs are swapped in x but the phase still follows the sign).
    let pos = 1.3_f32;
    let speed = 0.7_f32;
    let phase = pos * 0.6662;
    let leg_pose =
        |layer: ModelLayer, name: &str| layer.bake().try_child(name).unwrap().default_pose();
    let leg_poses = [
        leg_pose(ModelLayer::CHICKEN, "right_leg"),
        leg_pose(ModelLayer::CHICKEN, "left_leg"),
        leg_pose(ModelLayer::CHICKEN_BABY, "right_leg"),
        leg_pose(ModelLayer::CHICKEN_BABY, "left_leg"),
    ];
    for base in leg_poses {
        let posed = humanoid_leg_swing_pose(base, pos, speed);
        let expected = if base.offset[0] < 0.0 {
            phase.cos() * 1.4 * speed
        } else {
            (phase + std::f32::consts::PI).cos() * 1.4 * speed
        };
        assert!(
            (posed.rotation[0] - expected).abs() < 1e-6,
            "leg at x {} xRot",
            base.offset[0]
        );
        assert_eq!(posed.offset, base.offset, "leg offset");
        assert_eq!(posed.rotation[1], base.rotation[1], "leg yRot");
        assert_eq!(posed.rotation[2], base.rotation[2], "leg zRot");
    }
}

fn chicken_texture_images() -> Vec<EntityModelTextureImage> {
    texture_refs_matching(|path| path.starts_with("textures/entity/chicken/"))
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
