use super::*;

#[test]
fn polar_bear_texture_refs_match_vanilla_renderer() {
    assert!(entity_model_texture_refs().contains(&POLAR_BEAR_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&POLAR_BEAR_BABY_TEXTURE_REF));
}

#[test]
fn polar_bear_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let adult = polar_bear_textured_layer_passes(false);
    assert_eq!(adult.len(), 1);
    assert_eq!(
        adult[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(adult[0].texture, POLAR_BEAR_TEXTURE_REF);
    assert_eq!(adult[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(adult[0].order, 0);

    let baby = polar_bear_textured_layer_passes(true);
    assert_eq!(baby.len(), 1);
    assert_eq!(
        baby[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(baby[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(baby[0].texture, POLAR_BEAR_BABY_TEXTURE_REF);
    assert_eq!(baby[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(baby[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(baby[0].order, 0);
}

#[test]
fn entity_texture_atlas_stitches_official_polar_bear_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&polar_bear_texture_images()).unwrap();

    assert_eq!(layout.width, 128);
    assert_eq!(layout.height, 128);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/bear/polarbear.png",
            "textures/entity/bear/polarbear_baby.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 0.5]);
    assert_close2(layout.entries[1].uv.min, [0.0, 0.5]);
    assert_close2(layout.entries[1].uv.max, [0.5, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let baby_first_pixel = rgba_offset(layout.width, 64, 0, "polar bear baby atlas row").unwrap();
    assert_eq!(&rgba[baby_first_pixel..baby_first_pixel + 4], &[1; 4]);
}

#[test]
fn polar_bear_textured_mesh_uses_vanilla_uvs_tints_and_scale() {
    let (atlas, _) = build_entity_model_texture_atlas(&polar_bear_texture_images()).unwrap();
    let adult = EntityModelInstance::polar_bear(212, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (6_u32 << 4) | (13_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let adult_meshes = entity_model_textured_meshes(&[adult], &atlas);
    assert_polar_bear_textured_output(&adult_meshes, adult);
    assert_eq!(adult_meshes.cutout.cutout_faces, 60);
    assert_eq!(adult_meshes.cutout.vertices.len(), 240);
    assert_eq!(adult_meshes.cutout.indices.len(), 360);
    assert_eq!(adult_meshes.cutout.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);

    let baby = EntityModelInstance::polar_bear(213, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.light_coords = (6_u32 << 4) | (13_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_polar_bear_textured_output(&baby_meshes, baby);
    assert_eq!(baby_meshes.cutout.cutout_faces, 54);
    assert_eq!(baby_meshes.cutout.vertices.len(), 216);
    assert_eq!(baby_meshes.cutout.indices.len(), 324);
    assert!(baby_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn polar_bear_textured_meshes_apply_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&polar_bear_texture_images()).unwrap();

    // Adult head is part 0 (4 cubes = first 96 vertices); body and legs follow and
    // must stay put under a head look.
    let adult = EntityModelInstance::polar_bear(214, [0.0, 64.0, 0.0], 0.0, false);
    let yawed_adult = adult.with(|s| (s.head_yaw, s.head_pitch) = (50.0, 0.0));
    let pitched_adult = adult.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0));
    let resting = entity_model_textured_meshes(&[adult], &atlas);
    let yawed = entity_model_textured_meshes(&[yawed_adult], &atlas);
    let pitched = entity_model_textured_meshes(&[pitched_adult], &atlas);
    assert_polar_bear_textured_output(&resting, adult);
    assert_polar_bear_textured_output(&yawed, yawed_adult);
    assert_polar_bear_textured_output(&pitched, pitched_adult);
    assert_eq!(resting.cutout.vertices.len(), yawed.cutout.vertices.len());
    let head = ModelLayer::POLAR_BEAR
        .bake()
        .part_vertex_range("head")
        .unwrap();
    let body = ModelLayer::POLAR_BEAR
        .bake()
        .part_vertex_range("body")
        .unwrap();
    assert_ne!(
        resting.cutout.vertices[head.clone()],
        yawed.cutout.vertices[head.clone()]
    );
    assert_eq!(
        resting.cutout.vertices[body.clone()],
        yawed.cutout.vertices[body]
    );
    assert_ne!(
        yawed.cutout.vertices[head.clone()],
        pitched.cutout.vertices[head]
    );

    let baby = EntityModelInstance::polar_bear(215, [0.0, 64.0, 0.0], 0.0, true);
    let baby_looking_instance = baby.with(|s| (s.head_yaw, s.head_pitch) = (50.0, -20.0));
    let baby_resting = entity_model_textured_meshes(&[baby], &atlas);
    let baby_looking = entity_model_textured_meshes(&[baby_looking_instance], &atlas);
    assert_polar_bear_textured_output(&baby_resting, baby);
    assert_polar_bear_textured_output(&baby_looking, baby_looking_instance);
    assert_ne!(baby_resting.cutout.vertices, baby_looking.cutout.vertices);
    let baby_body = ModelLayer::POLAR_BEAR_BABY
        .bake()
        .part_vertex_range("body")
        .unwrap();
    assert_eq!(
        baby_resting.cutout.vertices[baby_body.clone()],
        baby_looking.cutout.vertices[baby_body]
    );

    // While rearing, the head look still applies, composed on top of the standing
    // pose and distinct from a flat (non-standing) look.
    let standing = EntityModelInstance::polar_bear_standing(216, [0.0, 64.0, 0.0], 0.0, false, 1.0);
    let standing_looking_instance = standing.with(|s| (s.head_yaw, s.head_pitch) = (50.0, -20.0));
    let flat_looking_instance = adult.with(|s| (s.head_yaw, s.head_pitch) = (50.0, -20.0));
    let standing_resting = entity_model_textured_meshes(&[standing], &atlas);
    let standing_looking = entity_model_textured_meshes(&[standing_looking_instance], &atlas);
    let flat_looking = entity_model_textured_meshes(&[flat_looking_instance], &atlas);
    assert_polar_bear_textured_output(&standing_resting, standing);
    assert_polar_bear_textured_output(&standing_looking, standing_looking_instance);
    assert_polar_bear_textured_output(&flat_looking, flat_looking_instance);
    assert_ne!(
        standing_resting.cutout.vertices[0..96],
        standing_looking.cutout.vertices[0..96]
    );
    assert_eq!(
        standing_resting.cutout.vertices[96..],
        standing_looking.cutout.vertices[96..]
    );
    assert_ne!(
        standing_looking.cutout.vertices[0..96],
        flat_looking.cutout.vertices[0..96]
    );
}

#[test]
fn polar_bear_standing_part_roles_cover_the_reared_parts() {
    // The rear-up moves the head, body, and both front legs (by name); the swing resolves these the
    // same for the adult and baby layouts.
    assert_eq!(
        polar_bear_standing_part_roles(),
        [
            ("head", PolarBearStandPart::Head),
            ("body", PolarBearStandPart::Body),
            ("right_front_leg", PolarBearStandPart::FrontLeg),
            ("left_front_leg", PolarBearStandPart::FrontLeg),
        ]
    );
}

#[test]
fn apply_polar_bear_standing_pose_matches_vanilla_setup_anim() {
    let pi = std::f32::consts::PI;

    // Adult (ageScale 1.0) at standScale 1.0 (squared = 1.0). The bind poses are the vanilla
    // `PolarBearModel.createBodyLayer` head/body/front-leg offsets.
    let adult_head_pose = PartPose {
        offset: [0.0, 10.0, -16.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let mut head = adult_head_pose;
    apply_polar_bear_standing_pose(&mut head, PolarBearStandPart::Head, 1.0, 1.0);
    assert_eq!(head.offset, [0.0, 10.0 - 24.0, -16.0 + 13.0]);
    assert!((head.rotation[0] - pi * 0.15).abs() < 1e-6);
    assert_eq!([head.rotation[1], head.rotation[2]], [0.0, 0.0]);

    let mut body = PartPose {
        offset: [-2.0, 9.0, 12.0],
        rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
    };
    apply_polar_bear_standing_pose(&mut body, PolarBearStandPart::Body, 1.0, 1.0);
    assert_eq!(body.offset, [-2.0, 9.0 + 2.0, 12.0]);
    assert!((body.rotation[0] - (std::f32::consts::FRAC_PI_2 - pi * 0.35)).abs() < 1e-6);

    let mut front_leg = PartPose {
        offset: [-3.5, 14.0, -8.0],
        rotation: [0.0, 0.0, 0.0],
    };
    apply_polar_bear_standing_pose(&mut front_leg, PolarBearStandPart::FrontLeg, 1.0, 1.0);
    assert_eq!(front_leg.offset, [-3.5, 14.0 - 20.0, -8.0 + 4.0]);
    assert!((front_leg.rotation[0] - (-pi * 0.45)).abs() < 1e-6);

    // standScale is squared: 0.5 -> 0.25 of the full delta.
    let mut quarter_head = adult_head_pose;
    apply_polar_bear_standing_pose(&mut quarter_head, PolarBearStandPart::Head, 1.0, 0.5);
    assert_eq!(quarter_head.offset[1], 10.0 - 0.25 * 24.0);

    // Baby (ageScale 0.5) scales only the body/front-leg translation terms.
    let mut baby_body = PartPose {
        offset: [0.0, 17.5, 0.0],
        rotation: [0.0, 0.0, 0.0],
    };
    apply_polar_bear_standing_pose(&mut baby_body, PolarBearStandPart::Body, 0.5, 1.0);
    assert_eq!(baby_body.offset[1], 17.5 + 0.5 * 2.0);

    let mut baby_front_leg = PartPose {
        offset: [-2.5, 21.5, -4.5],
        rotation: [0.0, 0.0, 0.0],
    };
    apply_polar_bear_standing_pose(&mut baby_front_leg, PolarBearStandPart::FrontLeg, 0.5, 1.0);
    assert_eq!(baby_front_leg.offset[1], 21.5 - 0.5 * 20.0);
    assert_eq!(baby_front_leg.offset[2], -4.5 + 0.5 * 4.0);

    // The head translation does not use ageScale, so the baby head moves the
    // same absolute amount as the adult head.
    let mut baby_head = PartPose {
        offset: [0.0, 18.625, -5.75],
        rotation: [0.0, 0.0, 0.0],
    };
    apply_polar_bear_standing_pose(&mut baby_head, PolarBearStandPart::Head, 0.5, 1.0);
    assert_eq!(baby_head.offset, [0.0, 18.625 - 24.0, -5.75 + 13.0]);
}

#[test]
fn polar_bear_textured_mesh_swings_legs_when_walking() {
    // The real polar bear render path (texture-backed) swings the same
    // `QuadrupedModel` legs. A standing bear is byte-identical however far the swing
    // has advanced; a walking adult lifts its feet. Baby legs swing too (asserted via
    // the vertex difference, as the short legs stay inside the bounding box).
    let (atlas, _) = build_entity_model_texture_atlas(&polar_bear_texture_images()).unwrap();
    for (name, base, adult_size) in [
        (
            "adult",
            EntityModelInstance::polar_bear(232, [0.0, 64.0, 0.0], 0.0, false),
            true,
        ),
        (
            "baby",
            EntityModelInstance::polar_bear(233, [0.0, 64.0, 0.0], 0.0, true),
            false,
        ),
    ] {
        let still_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
        let walking_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_polar_bear_textured_output(&resting, base);
        assert_polar_bear_textured_output(&still, still_instance);
        assert_polar_bear_textured_output(&walking, walking_instance);

        assert_eq!(
            resting.cutout.vertices, still.cutout.vertices,
            "{name}: a standing polar bear is inert"
        );
        assert_eq!(
            resting.cutout.vertices.len(),
            walking.cutout.vertices.len(),
            "{name}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.cutout.vertices, walking.cutout.vertices,
            "{name}: a walking polar bear differs"
        );

        if adult_size {
            let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
            let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "{name}: a walking polar bear's feet should lift off the ground"
            );
        }
    }
}

fn assert_polar_bear_textured_output(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert_polar_bear_folded_meshes_are_cutout_only(meshes);

    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
}

fn assert_polar_bear_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
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

fn polar_bear_texture_images() -> Vec<EntityModelTextureImage> {
    [POLAR_BEAR_TEXTURE_REF, POLAR_BEAR_BABY_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
