use super::*;
use crate::entity_models::model_layers::skeleton_clothing_model;
use crate::entity_models::model_layers::skeleton_model;

#[test]
fn skeleton_textured_layer_passes_match_vanilla_renderer_model_layers() {
    // Both the base body and the clothing overlay come from unified model trees (`SkeletonModel` /
    // `SkeletonClothingModel`), so the layer pass carries only the model layer key + texture.
    let base = skeleton_textured_layer_passes(None);
    assert_eq!(base.len(), 1);
    assert_eq!(
        base[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(base[0].texture, SKELETON_TEXTURE_REF);
    assert_eq!(base[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(base[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(base[0].order, 0);

    let stray = skeleton_textured_layer_passes(Some(SkeletonModelFamily::Stray));
    assert_eq!(stray.len(), 2);
    assert_eq!(stray[0].texture, STRAY_TEXTURE_REF);
    assert_eq!(
        stray[1].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(stray[1].texture, STRAY_OVERLAY_TEXTURE_REF);
    assert_eq!(stray[1].visibility, EntityModelLayerVisibility::All);
    assert_eq!(stray[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(stray[1].order, 1);

    let parched = skeleton_textured_layer_passes(Some(SkeletonModelFamily::Parched));
    assert_eq!(parched.len(), 1);
    assert_eq!(parched[0].texture, PARCHED_TEXTURE_REF);

    let wither = skeleton_textured_layer_passes(Some(SkeletonModelFamily::WitherSkeleton));
    assert_eq!(wither.len(), 1);
    assert_eq!(wither[0].texture, WITHER_SKELETON_TEXTURE_REF);

    let bogged =
        skeleton_textured_layer_passes(Some(SkeletonModelFamily::Bogged { sheared: false }));
    assert_eq!(bogged.len(), 2);
    assert_eq!(bogged[0].texture, BOGGED_TEXTURE_REF);
    assert_eq!(bogged[1].texture, BOGGED_OVERLAY_TEXTURE_REF);
    assert_eq!(bogged[1].order, 1);

    let sheared_bogged =
        skeleton_textured_layer_passes(Some(SkeletonModelFamily::Bogged { sheared: true }));
    assert_eq!(sheared_bogged.len(), 2);
}

#[test]
fn entity_texture_atlas_stitches_official_skeleton_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 256);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/skeleton/skeleton.png",
            "textures/entity/skeleton/stray.png",
            "textures/entity/skeleton/stray_overlay.png",
            "textures/entity/skeleton/parched.png",
            "textures/entity/skeleton/wither_skeleton.png",
            "textures/entity/skeleton/bogged.png",
            "textures/entity/skeleton/bogged_overlay.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 32.0 / 256.0]);
    assert_close2(layout.entries[2].uv.min, [0.0, 64.0 / 256.0]);
    assert_close2(layout.entries[2].uv.max, [1.0, 96.0 / 256.0]);
    assert_close2(layout.entries[3].uv.min, [0.0, 96.0 / 256.0]);
    assert_close2(layout.entries[3].uv.max, [1.0, 160.0 / 256.0]);
    assert_close2(layout.entries[6].uv.min, [0.0, 224.0 / 256.0]);
    assert_close2(layout.entries[6].uv.max, [1.0, 1.0]);
    assert!(entity_model_texture_refs().contains(&SKELETON_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&STRAY_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&STRAY_OVERLAY_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&PARCHED_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&WITHER_SKELETON_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&BOGGED_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&BOGGED_OVERLAY_TEXTURE_REF));
    let stray_overlay_first_pixel =
        rgba_offset(layout.width, 64, 0, "stray overlay atlas row").unwrap();
    assert_eq!(
        &rgba[stray_overlay_first_pixel..stray_overlay_first_pixel + 4],
        &[2; 4]
    );
    let parched_first_pixel = rgba_offset(layout.width, 96, 0, "parched atlas row").unwrap();
    assert_eq!(&rgba[parched_first_pixel..parched_first_pixel + 4], &[3; 4]);
}

#[test]
fn skeleton_textured_mesh_uses_vanilla_uvs_tints_and_variant_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();

    let skeleton_instance =
        skeleton_submission_probe(EntityModelInstance::skeleton(51, [0.0, 64.0, 0.0], 0.0));
    let skeleton_meshes = entity_model_textured_meshes(&[skeleton_instance], &atlas);
    assert_skeleton_textured_output(&skeleton_meshes, skeleton_instance);
    let skeleton = &skeleton_meshes.cutout;
    assert_eq!(skeleton.cutout_faces, 42);
    assert_eq!(skeleton.vertices.len(), 168);
    assert_eq!(skeleton.indices.len(), 252);
    assert_close2(skeleton.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(skeleton
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    // Same idle-bob X widening as the base skeleton above (±0.375 -> ±0.43708366).
    let (min, max) = textured_mesh_extents(skeleton);
    assert_close3(min, [-0.43708366, 64.001, -0.28125]);
    assert_close3(max, [0.43708366, 66.03225, 0.28125]);

    let stray_instance = skeleton_submission_probe(EntityModelInstance::skeleton_variant(
        128,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Stray,
    ));
    let stray_meshes = entity_model_textured_meshes(&[stray_instance], &atlas);
    assert_skeleton_textured_output(&stray_meshes, stray_instance);
    let stray = &stray_meshes.cutout;
    assert_eq!(stray.cutout_faces, 84);
    assert_eq!(stray.vertices.len(), 336);
    assert_close2(stray.vertices[0].uv, [16.0 / 64.0, 32.0 / 256.0]);
    assert_close2(stray.vertices[168].uv, [16.0 / 64.0, 64.0 / 256.0]);
    let (stray_min, stray_max) = textured_mesh_extents(stray);
    assert_close3(stray_min, [-0.578566, 63.985374, -0.296875]);
    assert_close3(stray_max, [0.578566, 66.047875, 0.296875]);

    let wither_instance = skeleton_submission_probe(EntityModelInstance::skeleton_variant(
        146,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::WitherSkeleton,
    ));
    let wither_meshes = entity_model_textured_meshes(&[wither_instance], &atlas);
    assert_skeleton_textured_output(&wither_meshes, wither_instance);
    let wither = &wither_meshes.cutout;
    assert_eq!(wither.cutout_faces, 42);
    assert_close2(wither.vertices[0].uv, [16.0 / 64.0, 160.0 / 256.0]);
    let (wither_min, wither_max) = textured_mesh_extents(wither);
    assert_close3(wither_min, [-0.52450037, 64.0012, -0.33750004]);
    assert_close3(wither_max, [0.52450037, 66.4387, 0.33750004]);

    let parched_instance = skeleton_submission_probe(EntityModelInstance::skeleton_variant(
        97,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Parched,
    ));
    let parched_meshes = entity_model_textured_meshes(&[parched_instance], &atlas);
    assert_skeleton_textured_output(&parched_meshes, parched_instance);
    let parched = &parched_meshes.cutout;
    assert_eq!(parched.cutout_faces, 78);
    assert_eq!(parched.vertices.len(), 312);
    assert_close2(parched.vertices[0].uv, [16.0 / 64.0, 96.0 / 256.0]);
    let (parched_min, parched_max) = textured_mesh_extents(parched);
    assert_close3(parched_min, [-0.50238097, 64.001, -0.26250002]);
    assert_close3(parched_max, [0.50238097, 66.0135, 0.26250002]);

    let bogged_instance = skeleton_submission_probe(EntityModelInstance::skeleton_variant(
        16,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Bogged { sheared: false },
    ));
    let bogged_meshes = entity_model_textured_meshes(&[bogged_instance], &atlas);
    assert_skeleton_textured_output(&bogged_meshes, bogged_instance);
    let bogged = &bogged_meshes.cutout;
    assert_eq!(bogged.cutout_faces, 120);
    assert_eq!(bogged.vertices.len(), 480);
    let bogged_mushroom =
        crate::entity_models::textured::skeleton_model_layer(Some(SkeletonModelFamily::Bogged {
            sheared: false,
        }))
        .bake()
        .part_vertex_range("red_mushroom_1")
        .unwrap()
        .start;
    assert_close2(
        bogged.vertices[bogged_mushroom].uv,
        [56.0 / 64.0, 208.0 / 256.0],
    );
    assert_close2(bogged.vertices[312].uv, [16.0 / 64.0, 224.0 / 256.0]);
    let (bogged_min, bogged_max) = textured_mesh_extents(bogged);
    assert_close3(bogged_min, [-0.57514465, 63.9885, -0.5]);
    assert_close3(bogged_max, [0.57514465, 66.1885, 0.32008255]);

    let sheared_bogged_instance = skeleton_submission_probe(EntityModelInstance::skeleton_variant(
        17,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Bogged { sheared: true },
    ));
    let sheared_bogged_meshes = entity_model_textured_meshes(&[sheared_bogged_instance], &atlas);
    assert_skeleton_textured_output(&sheared_bogged_meshes, sheared_bogged_instance);
    let sheared_bogged = &sheared_bogged_meshes.cutout;
    assert_eq!(sheared_bogged.cutout_faces, 84);
    assert_eq!(sheared_bogged.vertices.len(), 336);
    assert_close2(
        sheared_bogged.vertices[168].uv,
        [16.0 / 64.0, 224.0 / 256.0],
    );
    let (sheared_bogged_min, sheared_bogged_max) = textured_mesh_extents(sheared_bogged);
    assert_close3(sheared_bogged_min, [-0.57514465, 63.9885, -0.29375]);
    assert_close3(sheared_bogged_max, [0.57514465, 66.04475, 0.29375]);
}

#[test]
fn skeleton_clothing_missing_textures_preserve_other_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&skeleton_texture_images_for(&[
        STRAY_TEXTURE_REF,
        BOGGED_TEXTURE_REF,
    ]))
    .unwrap();

    for (instance, family, expected_base_vertices) in [
        (
            skeleton_submission_probe(EntityModelInstance::skeleton_variant(
                129,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Stray,
            )),
            SkeletonModelFamily::Stray,
            168,
        ),
        (
            skeleton_submission_probe(EntityModelInstance::skeleton_variant(
                18,
                [2.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Bogged { sheared: false },
            )),
            SkeletonModelFamily::Bogged { sheared: false },
            312,
        ),
    ] {
        let meshes = entity_model_textured_meshes(&[instance], &atlas);
        assert_skeleton_folded_meshes_are_cutout_only(&meshes);
        assert_eq!(meshes.cutout.vertices.len(), expected_base_vertices);
        let passes = skeleton_textured_layer_passes(Some(family));
        assert_eq!(passes.len(), 2);
        assert!(matches!(
            passes[1].kind,
            EntityModelLayerKind::SkeletonClothing
        ));

        let base_overlay = instance.render_state.overlay_coords();
        let clothing_overlay = [0.0, base_overlay[1]];

        assert!(meshes.cutout.vertices.iter().all(|vertex| {
            vertex.tint == [1.0, 1.0, 1.0, 1.0]
                && vertex.light == instance.render_state.shader_light()
                && vertex.overlay == base_overlay
        }));
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .all(|vertex| vertex.overlay != clothing_overlay));
    }
}

#[test]
fn skeleton_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();

    // Skeleton head is part 0: head look turns it without changing vertex count.
    let base = EntityModelInstance::skeleton(714, [0.0, 64.0, 0.0], 0.0);
    let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (45.0, 0.0));
    let pitched_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0));
    let resting_meshes = entity_model_textured_meshes(&[base], &atlas);
    let yawed_meshes = entity_model_textured_meshes(&[yawed_instance], &atlas);
    let pitched_meshes = entity_model_textured_meshes(&[pitched_instance], &atlas);
    assert_skeleton_textured_output(&resting_meshes, base);
    assert_skeleton_textured_output(&yawed_meshes, yawed_instance);
    assert_skeleton_textured_output(&pitched_meshes, pitched_instance);
    let resting = &resting_meshes.cutout;
    let yawed = &yawed_meshes.cutout;
    let pitched = &pitched_meshes.cutout;
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_ne!(resting.vertices, yawed.vertices);
    assert_ne!(yawed.vertices, pitched.vertices);

    // Head look turns the parched head and leaves its body untouched.
    let parched = EntityModelInstance::skeleton_variant(
        715,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Parched,
    );
    let parched_looking_instance = parched.with(|s| (s.head_yaw, s.head_pitch) = (45.0, -20.0));
    let parched_resting_meshes = entity_model_textured_meshes(&[parched], &atlas);
    let parched_looking_meshes = entity_model_textured_meshes(&[parched_looking_instance], &atlas);
    assert_skeleton_textured_output(&parched_resting_meshes, parched);
    assert_skeleton_textured_output(&parched_looking_meshes, parched_looking_instance);
    let parched_resting = &parched_resting_meshes.cutout;
    let parched_looking = &parched_looking_meshes.cutout;
    assert_ne!(parched_resting.vertices, parched_looking.vertices);
    let parched_model =
        crate::entity_models::textured::skeleton_model_layer(Some(SkeletonModelFamily::Parched))
            .bake();
    let parched_root = &parched_model;
    assert_eq!(
        parts_vertices(&parched_resting.vertices, parched_root, &["body"]),
        parts_vertices(&parched_looking.vertices, parched_root, &["body"])
    );
}

#[test]
fn skeleton_textured_mesh_swings_legs_when_walking() {
    // The real skeleton render path (texture-backed) consumes the projected limb
    // swing via the inherited HumanoidModel.setupAnim leg rotation, on both the
    // body layer and the Stray/Bogged clothing overlay (its layer SkeletonModel
    // runs the same setupAnim). A standing skeleton is byte-identical however far
    // the swing position has advanced; a walking one lifts its feet.
    let (atlas, _) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();
    let instances: [(&str, EntityModelInstance); 6] = [
        (
            "skeleton",
            EntityModelInstance::skeleton(80, [0.0, 64.0, 0.0], 0.0),
        ),
        (
            "stray",
            EntityModelInstance::skeleton_variant(
                81,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Stray,
            ),
        ),
        (
            "wither_skeleton",
            EntityModelInstance::skeleton_variant(
                82,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::WitherSkeleton,
            ),
        ),
        (
            "parched",
            EntityModelInstance::skeleton_variant(
                83,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Parched,
            ),
        ),
        (
            "bogged",
            EntityModelInstance::skeleton_variant(
                84,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Bogged { sheared: false },
            ),
        ),
        (
            "bogged_sheared",
            EntityModelInstance::skeleton_variant(
                85,
                [0.0, 64.0, 0.0],
                0.0,
                SkeletonModelFamily::Bogged { sheared: true },
            ),
        ),
    ];
    for (name, base) in instances {
        let still_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
        let walking_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
        let resting_meshes = entity_model_textured_meshes(&[base], &atlas);
        let still_meshes = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking_meshes = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_skeleton_textured_output(&resting_meshes, base);
        assert_skeleton_textured_output(&still_meshes, still_instance);
        assert_skeleton_textured_output(&walking_meshes, walking_instance);
        let resting = &resting_meshes.cutout;
        let still = &still_meshes.cutout;
        let walking = &walking_meshes.cutout;

        assert_eq!(
            resting.vertices, still.vertices,
            "{name}: a standing skeleton is inert"
        );
        assert_eq!(
            resting.vertices.len(),
            walking.vertices.len(),
            "{name}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.vertices, walking.vertices,
            "{name}: a walking skeleton differs"
        );

        let (rest_min, rest_max) = textured_mesh_extents(resting);
        let (walk_min, walk_max) = textured_mesh_extents(walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.1,
            "{name}: a walking skeleton's feet should lift off the ground"
        );
    }
}

#[test]
fn skeleton_textured_mesh_swings_arms_when_walking() {
    // The texture-backed skeleton render path runs the inherited HumanoidModel arm swing. A
    // standing skeleton is byte-identical however far the swing has advanced; a walking one
    // swings its arms.
    let z_extent = |verts: &[EntityModelTexturedVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[2]);
            hi = hi.max(vertex.position[2]);
        }
        hi - lo
    };
    let (atlas, _) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();
    let base = EntityModelInstance::skeleton(96, [0.0, 64.0, 0.0], 0.0);
    let still_instance = base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
    let walking_instance =
        base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let resting_meshes = entity_model_textured_meshes(&[base], &atlas);
    let still_meshes = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking_meshes = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_skeleton_textured_output(&resting_meshes, base);
    assert_skeleton_textured_output(&still_meshes, still_instance);
    assert_skeleton_textured_output(&walking_meshes, walking_instance);
    let resting = &resting_meshes.cutout;
    let still = &still_meshes.cutout;
    let walking = &walking_meshes.cutout;

    assert_eq!(
        resting.vertices, still.vertices,
        "a standing skeleton is inert"
    );
    assert_eq!(
        skeleton_parts(&resting.vertices, &["head", "body"]),
        skeleton_parts(&walking.vertices, &["head", "body"]),
        "head and body never swing"
    );
    assert_ne!(
        skeleton_parts(&resting.vertices, &["right_arm", "left_arm"]),
        skeleton_parts(&walking.vertices, &["right_arm", "left_arm"]),
        "arms swing"
    );
    assert_ne!(
        skeleton_parts(&resting.vertices, &["right_leg", "left_leg"]),
        skeleton_parts(&walking.vertices, &["right_leg", "left_leg"]),
        "legs swing"
    );
    let rest_arm_z = z_extent(&skeleton_parts(
        &resting.vertices,
        &["right_arm", "left_arm"],
    ));
    let walk_arm_z = z_extent(&skeleton_parts(
        &walking.vertices,
        &["right_arm", "left_arm"],
    ));
    assert!(
        walk_arm_z > rest_arm_z + 0.1,
        "the textured arms splay along Z when walking: {rest_arm_z} -> {walk_arm_z}"
    );
}

#[test]
fn skeleton_textured_arms_idle_bob_as_age_advances() {
    // The restructured textured humanoid path applies the inherited HumanoidModel idle arm
    // bob every frame, so a standing skeleton's arms move with ageInTicks while
    // the head and body and the legs stay byte-identical across ages.
    let (atlas, _) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();
    let base = EntityModelInstance::skeleton(97, [0.0, 64.0, 0.0], 0.0);
    let later_instance = base.with(|s| s.age_in_ticks = 27.3);
    let early_meshes = entity_model_textured_meshes(&[base], &atlas);
    let later_meshes = entity_model_textured_meshes(&[later_instance], &atlas);
    assert_skeleton_textured_output(&early_meshes, base);
    assert_skeleton_textured_output(&later_meshes, later_instance);
    let early = &early_meshes.cutout;
    let later = &later_meshes.cutout;
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_eq!(
        skeleton_parts(&early.vertices, &["head", "body"]),
        skeleton_parts(&later.vertices, &["head", "body"]),
        "head and body do not bob"
    );
    assert_ne!(
        skeleton_parts(&early.vertices, &["right_arm", "left_arm"]),
        skeleton_parts(&later.vertices, &["right_arm", "left_arm"]),
        "the arms idle-bob with ageInTicks"
    );
    assert_eq!(
        skeleton_parts(&early.vertices, &["right_leg", "left_leg"]),
        skeleton_parts(&later.vertices, &["right_leg", "left_leg"]),
        "the legs do not bob"
    );
}

#[test]
fn skeleton_model_aims_both_arms_for_bow_and_arrow() {
    // Vanilla `AbstractSkeletonRenderer.getArmPose` returns `BOW_AND_ARROW` when the skeleton
    // `isAggressive() && getMainHandItem().is(Items.BOW)`, so `HumanoidModel.poseRightArm`'s
    // `BOW_AND_ARROW` case raises both arms forward along the head look (right-handed bow, the off
    // arm splayed `0.4` outward), overwriting the walk swing's xRot/yRot before the inherited idle bob.
    let yaw = 30.0_f32;
    let pitch = -12.5_f32;
    let yaw_rad = yaw.to_radians();
    let pitch_rad = pitch.to_radians();
    let aim_pitch = -std::f32::consts::FRAC_PI_2 + pitch_rad;
    let age = 11.0_f32;
    let bob_x = (age * 0.067).sin() * 0.05;
    let bob_z = (age * 0.09).cos() * 0.05 + 0.05;

    let aiming = EntityModelInstance::skeleton(810, [0.0, 64.0, 0.0], 0.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch))
        .with(|s| {
            s.age_in_ticks = age;
            s.is_aggressive = true;
            s.main_hand_holds_bow = true;
        });
    let mut model = skeleton_model(None, &aiming);

    let right = model.child_mut("right_arm");
    assert!(
        (right.pose.rotation[0] - (aim_pitch + bob_x)).abs() < 1e-6,
        "right arm aims before adding idle bob: {}",
        right.pose.rotation[0]
    );
    assert!((right.pose.rotation[2] - bob_z).abs() < 1e-6);
    assert!(
        (right.pose.rotation[1] - (-0.1 + yaw_rad)).abs() < 1e-6,
        "right arm yaws -0.1 off the head yaw: {}",
        right.pose.rotation[1]
    );
    let left = model.child_mut("left_arm");
    assert!(
        (left.pose.rotation[0] - (aim_pitch - bob_x)).abs() < 1e-6,
        "left arm aims before adding mirrored idle bob: {}",
        left.pose.rotation[0]
    );
    assert!((left.pose.rotation[2] + bob_z).abs() < 1e-6);
    assert!(
        (left.pose.rotation[1] - (0.1 + yaw_rad + 0.4)).abs() < 1e-6,
        "left arm splays 0.4 outward off the head yaw: {}",
        left.pose.rotation[1]
    );

    // The BOW aim is gated on BOTH flags: a bow held without aggression keeps the idle arms, while an
    // aggressive skeleton without a bow melees instead (raised arms — see the melee test). Neither
    // splays the off arm to the bow-aim `0.1 + yaw + 0.4`, the aim's signature.
    let bow_left_yrot = 0.1 + yaw_rad + 0.4;
    for not_aiming in [
        EntityModelInstance::skeleton(811, [0.0, 64.0, 0.0], 0.0)
            .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch))
            .with(|s| s.main_hand_holds_bow = true),
        EntityModelInstance::skeleton(812, [0.0, 64.0, 0.0], 0.0)
            .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch))
            .with(|s| s.is_aggressive = true),
    ] {
        let mut idle = skeleton_model(None, &not_aiming);
        assert!(
            (idle.child_mut("left_arm").pose.rotation[1] - bow_left_yrot).abs() > 0.1,
            "a skeleton missing either flag does not draw the bow"
        );
    }
}

#[test]
fn skeleton_humanoid_mob_spear_pose_points_arm_along_head_look() {
    let yaw = 20.0_f32;
    let pitch = -10.0_f32;
    let yaw_rad = yaw.to_radians();
    let spear_pitch = (-std::f32::consts::FRAC_PI_2 + pitch.to_radians() + 0.8)
        .to_degrees()
        .clamp(-120.0, 30.0)
        .to_radians();
    let age = 11.0_f32;
    let bob_x = (age * 0.067).sin() * 0.05;

    // Vanilla `HumanoidMobRenderer.getArmPose`: a calm skeleton holding a spear in the main hand gets
    // `ArmPose.SPEAR`, which runs `SpearAnimations.thirdPersonHandUse` without kinetic sway.
    let main_spear = EntityModelInstance::skeleton(813, [0.0, 64.0, 0.0], 0.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch))
        .with(|s| {
            s.age_in_ticks = age;
            s.humanoid_mob_main_hand_spear_pose = true;
        });
    let mut main = skeleton_model(None, &main_spear);
    let right = main.child_mut("right_arm").pose;
    assert!((right.rotation[0] - (spear_pitch + bob_x)).abs() < 1e-6);
    assert!((right.rotation[1] - (-0.1 + yaw_rad)).abs() < 1e-6);
    assert_eq!(
        main.child_mut("left_arm").pose.rotation[1],
        0.0,
        "the off-arm EMPTY pose resets yRot before the main spear pose"
    );

    // In the right-handed, non-using dispatch, an off-hand SPEAR runs first and suppresses the main-hand
    // pose. This also suppresses an otherwise valid aggressive bow aim.
    let off_spear = EntityModelInstance::skeleton(814, [0.0, 64.0, 0.0], 0.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch))
        .with(|s| {
            s.age_in_ticks = age;
            s.is_aggressive = true;
            s.main_hand_holds_bow = true;
            s.humanoid_mob_off_hand_spear_pose = true;
        });
    let mut off = skeleton_model(None, &off_spear);
    let left = off.child_mut("left_arm").pose;
    assert!((left.rotation[0] - (spear_pitch - bob_x)).abs() < 1e-6);
    assert!((left.rotation[1] - (0.1 + yaw_rad)).abs() < 1e-6);
    let right = off.child_mut("right_arm").pose;
    let bow_right_pitch = -std::f32::consts::FRAC_PI_2 + pitch.to_radians();
    assert!(
        (right.rotation[0] - bow_right_pitch).abs() > 0.5,
        "off-hand SPEAR.affectsOffhandPose skips the main-hand BOW_AND_ARROW pose"
    );
    let mut overlay = skeleton_clothing_model(Some(SkeletonModelFamily::Stray), &off_spear);
    let overlay_left = overlay.child_mut("left_arm").pose;
    assert!(
        (overlay_left.rotation[0] - (spear_pitch - bob_x)).abs() < 1e-6,
        "stray/bogged clothing overlay tracks the same spear arm pose"
    );
}

#[test]
fn aggressive_skeleton_without_a_bow_raises_and_chops_its_arms() {
    use std::f32::consts::FRAC_PI_2;
    // Vanilla `SkeletonModel.setupAnim` (`isAggressive && !isHoldingBow`): both arms raise to `-π/2`
    // and chop with `attackTime`, the right arm yawing in (`yRot = -(0.1 - sin(tπ)·0.6)`) and the left
    // out, then take the idle bob. At rest (`attack_anim = 0`) the arms hold the raised melee-ready pose.
    let melee =
        EntityModelInstance::skeleton(820, [0.0, 64.0, 0.0], 0.0).with(|s| s.is_aggressive = true);
    let mut model = skeleton_model(None, &melee);
    let right = model.child_mut("right_arm");
    assert!(
        (right.pose.rotation[0] - (-FRAC_PI_2)).abs() < 1e-6,
        "right arm raised to -π/2: {}",
        right.pose.rotation[0]
    );
    assert!((right.pose.rotation[1] - (-0.1)).abs() < 1e-6);
    let left = model.child_mut("left_arm");
    assert!((left.pose.rotation[0] - (-FRAC_PI_2)).abs() < 1e-6);
    assert!((left.pose.rotation[1] - 0.1).abs() < 1e-6);

    // A swing chops the arms further down (xRot more negative than the raised rest pose).
    let mut swinging = skeleton_model(None, &melee.with(|s| s.attack_anim = 0.5));
    assert!(
        swinging.child_mut("right_arm").pose.rotation[0] < -FRAC_PI_2 - 0.5,
        "the arms chop down during the swing"
    );

    // A bow-aiming skeleton (aggressive + bow) aims instead — its off arm splays 0.4 (yRot 0.5), not 0.1.
    let mut aiming = skeleton_model(None, &melee.with(|s| s.main_hand_holds_bow = true));
    assert!(
        (aiming.child_mut("left_arm").pose.rotation[1] - 0.5).abs() < 1e-6,
        "a bow skeleton aims (off arm splayed 0.4), not melees"
    );

    // A calm skeleton melees neither — its arms hang on the idle pose, not raised to -π/2.
    let mut calm = skeleton_model(
        None,
        &EntityModelInstance::skeleton(821, [0.0, 64.0, 0.0], 0.0),
    );
    assert!(
        calm.child_mut("right_arm").pose.rotation[0].abs() < 0.5,
        "a calm skeleton's arms hang"
    );
}

#[test]
fn skeleton_bow_aim_moves_only_the_arms_in_the_textured_mesh() {
    // End-to-end through the dispatch path: drawing a bow re-poses the arm vertices
    // while leaving the head/body and legs byte-identical, and the pose is
    // gated on both flags so a skeleton missing either renders the resting mesh.
    let (atlas, _) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();
    let base = EntityModelInstance::skeleton(98, [0.0, 64.0, 0.0], 0.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (20.0, -10.0));
    let aiming_instance = base.with(|s| {
        s.is_aggressive = true;
        s.main_hand_holds_bow = true;
    });
    let resting_meshes = entity_model_textured_meshes(&[base], &atlas);
    let aiming_meshes = entity_model_textured_meshes(&[aiming_instance], &atlas);
    assert_skeleton_textured_output(&resting_meshes, base);
    assert_skeleton_textured_output(&aiming_meshes, aiming_instance);
    let resting = &resting_meshes.cutout;
    let aiming = &aiming_meshes.cutout;

    assert_eq!(resting.vertices.len(), aiming.vertices.len());
    assert_eq!(
        skeleton_parts(&resting.vertices, &["head", "body"]),
        skeleton_parts(&aiming.vertices, &["head", "body"]),
        "the head and body do not aim"
    );
    assert_ne!(
        skeleton_parts(&resting.vertices, &["right_arm", "left_arm"]),
        skeleton_parts(&aiming.vertices, &["right_arm", "left_arm"]),
        "both arms raise to draw the bow"
    );
    assert_eq!(
        skeleton_parts(&resting.vertices, &["right_leg", "left_leg"]),
        skeleton_parts(&aiming.vertices, &["right_leg", "left_leg"]),
        "the legs do not aim"
    );

    // Aggression without a bow melees instead — it re-poses the arms but not the
    // head/body or legs (a different pose from the bow aim; see the melee test).
    let meleeing_instance = base.with(|s| s.is_aggressive = true);
    let meleeing_meshes = entity_model_textured_meshes(&[meleeing_instance], &atlas);
    assert_skeleton_textured_output(&meleeing_meshes, meleeing_instance);
    let meleeing = &meleeing_meshes.cutout;
    assert_ne!(
        skeleton_parts(&resting.vertices, &["right_arm", "left_arm"]),
        skeleton_parts(&meleeing.vertices, &["right_arm", "left_arm"]),
        "a melee skeleton raises its arms"
    );
    assert_eq!(
        skeleton_parts(&resting.vertices, &["head", "body"]),
        skeleton_parts(&meleeing.vertices, &["head", "body"]),
        "the melee leaves the head and body"
    );
    assert_eq!(
        skeleton_parts(&resting.vertices, &["right_leg", "left_leg"]),
        skeleton_parts(&meleeing.vertices, &["right_leg", "left_leg"]),
        "the melee leaves the legs"
    );
    // A holstered bow with no aggression neither aims nor melees.
    let holstered_instance = base.with(|s| s.main_hand_holds_bow = true);
    let holstered_meshes = entity_model_textured_meshes(&[holstered_instance], &atlas);
    assert_skeleton_textured_output(&holstered_meshes, holstered_instance);
    assert_eq!(
        resting.vertices, holstered_meshes.cutout.vertices,
        "a holstered bow does not aim"
    );
}

#[test]
fn stray_clothing_overlay_tracks_the_aiming_arms() {
    // The stray frost overlay is posed by the same `SkeletonModel.setupAnim`, so its arms aim with
    // the body's — drawing a bow must move the overlay mesh too (vanilla poses the overlay layer
    // with the base model's part angles).
    let (atlas, _) = build_entity_model_texture_atlas(&skeleton_texture_images()).unwrap();
    let base = EntityModelInstance::skeleton_variant(
        99,
        [0.0, 64.0, 0.0],
        0.0,
        SkeletonModelFamily::Stray,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (20.0, -10.0));
    let aiming_instance = base.with(|s| {
        s.is_aggressive = true;
        s.main_hand_holds_bow = true;
    });
    let resting_meshes = entity_model_textured_meshes(&[base], &atlas);
    let aiming_meshes = entity_model_textured_meshes(&[aiming_instance], &atlas);
    assert_skeleton_textured_output(&resting_meshes, base);
    assert_skeleton_textured_output(&aiming_meshes, aiming_instance);
    let resting = &resting_meshes.cutout;
    let aiming = &aiming_meshes.cutout;
    assert_eq!(resting.vertices.len(), aiming.vertices.len());
    assert_ne!(
        resting.vertices, aiming.vertices,
        "the stray body and its frost overlay both aim the bow"
    );
}

fn skeleton_texture_images() -> Vec<EntityModelTextureImage> {
    skeleton_texture_images_for(&texture_refs_matching(|path| {
        path.starts_with("textures/entity/skeleton/")
    }))
}

fn skeleton_texture_images_for(textures: &[EntityModelTextureRef]) -> Vec<EntityModelTextureImage> {
    textures
        .iter()
        .copied()
        .enumerate()
        .map(|(index, texture)| {
            EntityModelTextureImage::new(
                texture,
                vec![
                    u8::try_from(index).unwrap();
                    usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap()
                ],
            )
        })
        .collect()
}

fn skeleton_submission_probe(instance: EntityModelInstance) -> EntityModelInstance {
    instance.with(|s| {
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    })
}

fn assert_skeleton_textured_output(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    let family = match instance.kind {
        EntityModelKind::Skeleton => None,
        EntityModelKind::SkeletonVariant { family } => Some(family),
        _ => panic!("expected skeleton instance"),
    };
    assert_skeleton_folded_meshes_are_cutout_only(meshes);
    let passes = skeleton_textured_layer_passes(family);

    let base_overlay = instance.render_state.overlay_coords();
    let clothing_overlay = [0.0, base_overlay[1]];
    let has_clothing = passes
        .iter()
        .any(|pass| matches!(pass.kind, EntityModelLayerKind::SkeletonClothing));
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == instance.render_state.shader_light()));
    if has_clothing && clothing_overlay != base_overlay {
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .any(|vertex| vertex.overlay == base_overlay));
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .any(|vertex| vertex.overlay == clothing_overlay));
        assert!(meshes.cutout.vertices.iter().all(|vertex| {
            vertex.overlay == base_overlay || vertex.overlay == clothing_overlay
        }));
    } else {
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .all(|vertex| vertex.overlay == base_overlay));
    }
}

fn assert_skeleton_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
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

fn skeleton_parts<V: Copy>(vertices: &[V], names: &[&str]) -> Vec<V> {
    parts_vertices(
        vertices,
        &crate::entity_models::textured::skeleton_model_layer(None).bake(),
        names,
    )
}
