use super::*;

#[test]
fn sniffer_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `SnifferModel.createBodyLayer` (atlas 192×192): the mesh root holds one `bone` part
    // at `offset(0, 5, 0)` parenting the body and the six legs.
    assert_eq!(SNIFFER_BONE_POSE.offset, [0.0, 5.0, 0.0]);

    // `body`: the 25×29×40 trunk, the inner block inflated by `CubeDeformation(0.5)`
    // (`min -= 0.5`, `size += 1`), and the 25×0×40 belly plane.
    assert_eq!(SNIFFER_BODY_POSE.offset, [0.0, 0.0, 0.0]);
    assert_eq!(SNIFFER_BODY_CUBES.len(), 3);
    assert_eq!(SNIFFER_BODY_CUBES[0].min, [-12.5, -14.0, -20.0]);
    assert_eq!(SNIFFER_BODY_CUBES[0].size, [25.0, 29.0, 40.0]);
    // The deformed inner block: base min[-12.5,-14,-20] size[25,24,40] grown by 0.5 on each face.
    assert_eq!(SNIFFER_BODY_CUBES[1].min, [-13.0, -14.5, -20.5]);
    assert_eq!(SNIFFER_BODY_CUBES[1].size, [26.0, 25.0, 41.0]);
    assert_eq!(SNIFFER_BODY_CUBES[2].size, [25.0, 0.0, 40.0]);

    // `head` (offset (0, 6.5, -19.48)) parents two ears, the nose, and the lower beak.
    assert_eq!(SNIFFER_HEAD_POSE.offset, [0.0, 6.5, -19.48]);
    assert_eq!(SNIFFER_HEAD_CUBES.len(), 2);
    assert_eq!(SNIFFER_HEAD_CUBES[0].size, [13.0, 18.0, 11.0]);
    assert_eq!(SNIFFER_LEFT_EAR_POSE.offset, [6.51, -7.5, -4.51]);
    assert_eq!(SNIFFER_LEFT_EAR_CUBES[0].size, [1.0, 19.0, 7.0]);
    // The nose pad and lower beak.
    assert_eq!(SNIFFER_NOSE_POSE.offset, [0.0, -4.5, -11.5]);
    assert_eq!(SNIFFER_NOSE_CUBES[0].size, [13.0, 2.0, 9.0]);
    assert_eq!(SNIFFER_LOWER_BEAK_CUBES[0].size, [13.0, 12.0, 9.0]);

    // The six legs share one 7×10×8 box at the standard three pairs of offsets.
    for expected in [
        SNIFFER_RIGHT_FRONT_LEG_POSE,
        SNIFFER_RIGHT_MID_LEG_POSE,
        SNIFFER_RIGHT_HIND_LEG_POSE,
        SNIFFER_LEFT_FRONT_LEG_POSE,
        SNIFFER_LEFT_MID_LEG_POSE,
        SNIFFER_LEFT_HIND_LEG_POSE,
    ] {
        assert_eq!(expected.offset[1], 10.0);
    }
    // All six legs share the 7×10×8 box geometry but carry per-leg texOffs (right column u=32,
    // left column u=0; front/mid/hind at v=87/105/123).
    assert_eq!(SNIFFER_RIGHT_FRONT_LEG_CUBES[0].size, [7.0, 10.0, 8.0]);
    assert_eq!(SNIFFER_RIGHT_FRONT_LEG_CUBES[0].tex, [32.0, 87.0]);
    assert_eq!(SNIFFER_RIGHT_HIND_LEG_CUBES[0].tex, [32.0, 123.0]);
    assert_eq!(SNIFFER_LEFT_FRONT_LEG_CUBES[0].tex, [0.0, 87.0]);
    assert_eq!(SNIFFER_LEFT_HIND_LEG_CUBES[0].tex, [0.0, 123.0]);
    assert_eq!(SNIFFER_RIGHT_FRONT_LEG_POSE.offset, [-7.5, 10.0, -15.0]);
    assert_eq!(SNIFFER_LEFT_HIND_LEG_POSE.offset, [7.5, 10.0, 15.0]);
}

#[test]
fn sniffer_mesh_uses_vanilla_body_layer_geometry() {
    // 15 cubes → 90 faces / 360 vertices / 540 indices; the nose carries its own pink tint.
    let sniffer = entity_model_mesh(&[EntityModelInstance::sniffer(930, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(sniffer.opaque_faces, 90);
    assert_eq!(sniffer.vertices.len(), 360);
    assert_eq!(sniffer.indices.len(), 540);
    assert!(sniffer
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(SNIFFER_BROWN, 1.0)));
    assert!(sniffer
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(SNIFFER_NOSE, 1.0)));
}

#[test]
fn sniffer_head_follows_look_angles() {
    // Vanilla `SnifferModel.setupAnim` sets `head.xRot/yRot` from the plain look. The head is nested
    // bone → body → head; the emit order is body (3 cubes → [0, 72)), then the head subtree (head's 2
    // cubes + the ear/nose/beak children's 4 = 6 cubes → [72, 216)), then the six legs ([216, 360)). A
    // non-zero look (with the walk at rest) turns only the head subtree (the ears, nose, and beak ride
    // with it); the body and legs stay at bind.
    let base = EntityModelInstance::sniffer(931, [0.0, 64.0, 0.0], 0.0);
    let rest = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(35.0, -20.0)]);
    assert_eq!(rest.vertices.len(), looking.vertices.len());
    assert_eq!(
        rest.vertices[..72],
        looking.vertices[..72],
        "the body stays at bind"
    );
    assert_ne!(
        rest.vertices[72..216],
        looking.vertices[72..216],
        "the head, ears, nose, and beak turn with the look"
    );
    assert_eq!(
        rest.vertices[216..],
        looking.vertices[216..],
        "the six legs stay at bind"
    );
}

#[test]
fn sniffer_walk_animation_matches_vanilla_definition() {
    // Vanilla `SnifferAnimation.SNIFFER_WALK`: 2.0 s looping, animating the six legs (rotation +
    // position), the body (rotation + position), the head (rotation), and the two ears (rotation) —
    // ten bones, 102 keyframes total.
    assert_eq!(SNIFFER_WALK.length_seconds, 2.0);
    assert!(SNIFFER_WALK.looping);
    assert_eq!(SNIFFER_WALK.bones.len(), 10);
    let keyframes: usize = SNIFFER_WALK
        .bones
        .iter()
        .flat_map(|bone| bone.channels.iter())
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(keyframes, 102);

    // The right front leg pitches to `degreeVec(35, 0, 0)` at its t=0.5833 keyframe.
    let (_, rfl_rot) = sample_bone_offsets(&SNIFFER_WALK, "right_front_leg", 0.5833, 1.0);
    assert!((rfl_rot[0] - 35.0_f32.to_radians()).abs() < 1.0e-5);

    // The two ears roll oppositely: at t=0 the left ear is `-2.5°` and the right ear `+2.5°` in z
    // (CatmullRom at the first keyframe returns that keyframe's value).
    let (_, left_ear) = sample_bone_offsets(&SNIFFER_WALK, "left_ear", 0.0, 1.0);
    let (_, right_ear) = sample_bone_offsets(&SNIFFER_WALK, "right_ear", 0.0, 1.0);
    assert!((left_ear[2] - (-2.5_f32).to_radians()).abs() < 1.0e-4);
    assert!((right_ear[2] - 2.5_f32.to_radians()).abs() < 1.0e-4);
}

#[test]
fn sniffer_walk_moves_the_limbs_and_composes_with_the_look() {
    // A still sniffer (walk speed 0) samples the cycle at amplitude 0, collapsing to the bind pose; a
    // walking sniffer samples SNIFFER_WALK across the body, head, ears, and six legs. The vertex count
    // is preserved.
    let still = entity_model_mesh(&[EntityModelInstance::sniffer(932, [0.0, 64.0, 0.0], 0.0)]);
    let walking = entity_model_mesh(&[
        EntityModelInstance::sniffer(933, [0.0, 64.0, 0.0], 0.0).with_walk_animation(5.0, 1.0)
    ]);
    assert_eq!(still.vertices.len(), walking.vertices.len());
    assert_ne!(
        still.vertices, walking.vertices,
        "the walking sniffer rocks its body, ears, and legs"
    );

    // The head walk pitch ADDS onto the look, so a walking + looking sniffer differs from one that
    // only walks across the head subtree [72, 216); the body and legs share the same walk.
    let walking_looking =
        entity_model_mesh(&[EntityModelInstance::sniffer(934, [0.0, 64.0, 0.0], 0.0)
            .with_walk_animation(5.0, 1.0)
            .with_head_look(30.0, -15.0)]);
    assert_ne!(
        walking.vertices[72..216],
        walking_looking.vertices[72..216],
        "the look composes onto the walking head"
    );
    assert_eq!(
        walking.vertices[..72],
        walking_looking.vertices[..72],
        "the body shares the same walk regardless of the look"
    );
    assert_eq!(
        walking.vertices[216..],
        walking_looking.vertices[216..],
        "the six legs share the same walk regardless of the look"
    );
}

#[test]
fn sniffer_search_walk_animation_matches_vanilla_definition() {
    // Vanilla `SnifferAnimation.SNIFFER_SNIFF_SEARCH`: 2.0 s looping, the same ten walk bones plus a
    // `nose` SCALE channel — eleven bones, 120 keyframes total.
    assert_eq!(SNIFFER_SNIFF_SEARCH.length_seconds, 2.0);
    assert!(SNIFFER_SNIFF_SEARCH.looping);
    assert_eq!(SNIFFER_SNIFF_SEARCH.bones.len(), 11);
    assert_eq!(SNIFFER_SNIFF_SEARCH.bones[10].bone, "nose");
    let keyframes: usize = SNIFFER_SNIFF_SEARCH
        .bones
        .iter()
        .flat_map(|bone| bone.channels.iter())
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(keyframes, 120);

    // The body pitches to `degreeVec(2.5, 0, 0)` at t=0.
    let (_, body_rot) = sample_bone_offsets(&SNIFFER_SNIFF_SEARCH, "body", 0.0, 1.0);
    assert!((body_rot[0] - 2.5_f32.to_radians()).abs() < 1.0e-5);

    // The nose puffs: at its t=0.4583 keyframe `scaleVec(1, 2.5, 1)` ⇒ y-scale 2.5.
    let (_, _, nose_scale) =
        sample_bone_offsets_with_scale(&SNIFFER_SNIFF_SEARCH, "nose", 0.4583, 1.0);
    let puffed = keyframe_animated_scale(nose_scale);
    assert!(
        (puffed[1] - 2.5).abs() < 1.0e-3,
        "the nose puffs in y: {puffed:?}"
    );
}

#[test]
fn sniffer_searching_swaps_in_the_search_walk() {
    // A searching sniffer (DATA_STATE == SEARCHING) samples SNIFFER_SNIFF_SEARCH in place of the base
    // walk, so it poses differently from a non-searching walker at the same walk phase, and the nose
    // puffs. Same vertex count (no cubes added/removed).
    let walking = entity_model_mesh(&[
        EntityModelInstance::sniffer(935, [0.0, 64.0, 0.0], 0.0).with_walk_animation(5.0, 1.0)
    ]);
    let searching = entity_model_mesh(&[EntityModelInstance::sniffer(936, [0.0, 64.0, 0.0], 0.0)
        .with_walk_animation(5.0, 1.0)
        .with_sniffer_is_searching(true)]);
    assert_eq!(walking.vertices.len(), searching.vertices.len());
    assert_ne!(
        walking.vertices, searching.vertices,
        "the search-walk poses the body/legs/nose differently from the base walk"
    );
}

#[test]
fn sniffer_state_animations_match_vanilla_definitions() {
    // Vanilla `SnifferAnimation` lengths / looping flags for the one-shots `SnifferModel.setupAnim`
    // applies from the synced state.
    assert_eq!(SNIFFER_DIG.length_seconds, 8.0);
    assert!(!SNIFFER_DIG.looping);
    assert_eq!(SNIFFER_DIG.bones.len(), 10);
    assert_eq!(SNIFFER_LONGSNIFF.length_seconds, 1.0);
    assert!(!SNIFFER_LONGSNIFF.looping);
    assert_eq!(SNIFFER_STAND_UP.length_seconds, 3.0);
    assert!(!SNIFFER_STAND_UP.looping);
    assert_eq!(SNIFFER_STAND_UP.bones.len(), 10);
    assert_eq!(SNIFFER_HAPPY.length_seconds, 2.0);
    assert!(SNIFFER_HAPPY.looping);
    assert_eq!(SNIFFER_SNIFFSNIFF.length_seconds, 8.0);
    assert!(SNIFFER_SNIFFSNIFF.looping);

    // The dig drops the `body` `-7` y at its `1.5` keyframe (`posVec` negates y, so the offset is
    // `+7`) and sinks the four corner legs `-5.5` y by the time the dig hole is reached.
    let (dig_body_pos, _, _) = sample_bone_offsets_with_scale(&SNIFFER_DIG, "body", 1.5, 1.0);
    assert!((dig_body_pos[1] - 7.0).abs() < 1.0e-4);
    let (rfl_pos, rfl_rot, _) =
        sample_bone_offsets_with_scale(&SNIFFER_DIG, "right_front_leg", 1.375, 1.0);
    assert!((rfl_pos[1] - 5.5).abs() < 1.0e-4);
    assert!((rfl_rot[2] - 90.0_f32.to_radians()).abs() < 1.0e-4);

    // The long-sniff puffs the `nose` to `scaleVec(1, 4, 1)` ⇒ scale `[1, 4, 1]` at its `0.7083`
    // keyframe.
    let (_, _, sniff_nose) =
        sample_bone_offsets_with_scale(&SNIFFER_LONGSNIFF, "nose", 0.7083, 1.0);
    let puffed = keyframe_animated_scale(sniff_nose);
    assert!((puffed[1] - 4.0).abs() < 1.0e-4);
}

#[test]
fn sniffer_state_animation_re_poses_off_the_walk_pose() {
    // A sniffer with no synced-state animation (`-1` id) renders at the walk-sampled (here still)
    // bind pose plus the look.
    let resting = entity_model_mesh(&[EntityModelInstance::sniffer(940, [0.0, 64.0, 0.0], 0.0)]);

    // A digging sniffer re-poses off the bind pose: the body sinks, the head dives, the legs fold.
    let digging = entity_model_mesh(&[EntityModelInstance::sniffer(941, [0.0, 64.0, 0.0], 0.0)
        .with_sniffer_animation_id(5)
        .with_sniffer_animation_seconds(2.0)]);
    assert_eq!(
        resting.vertices.len(),
        digging.vertices.len(),
        "the dig re-poses parts, it does not add or hide cubes"
    );
    assert_ne!(
        resting.vertices, digging.vertices,
        "the digging sniffer leaves the bind pose"
    );

    // A different state (the rising stand-up) at the same time poses differently from the dig.
    let rising = entity_model_mesh(&[EntityModelInstance::sniffer(942, [0.0, 64.0, 0.0], 0.0)
        .with_sniffer_animation_id(6)
        .with_sniffer_animation_seconds(2.0)]);
    assert_ne!(
        digging.vertices, rising.vertices,
        "the rising stand-up poses differently from the dig"
    );

    // Sampling the dig at a different time advances the pose.
    let digging_later =
        entity_model_mesh(&[EntityModelInstance::sniffer(943, [0.0, 64.0, 0.0], 0.0)
            .with_sniffer_animation_id(5)
            .with_sniffer_animation_seconds(4.0)]);
    assert_ne!(
        digging.vertices, digging_later.vertices,
        "the dig animation advances as its elapsed seconds climb"
    );

    // The `-1` no-animation sentinel leaves the sniffer at the walk/bind pose.
    let cleared = entity_model_mesh(&[EntityModelInstance::sniffer(944, [0.0, 64.0, 0.0], 0.0)
        .with_sniffer_animation_id(-1)
        .with_sniffer_animation_seconds(-1.0)]);
    assert_eq!(cleared.vertices, resting.vertices);
}

#[test]
fn sniffer_textured_render_matches_vanilla_renderer() {
    let passes = sniffer_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].texture, SNIFFER_TEXTURE_REF);
    assert_eq!(
        EntityModelKind::Sniffer.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/sniffer/sniffer.png",
            size: [192, 192],
        })
    );
    assert!(entity_model_texture_refs().contains(&SNIFFER_TEXTURE_REF));
    assert_eq!(sniffer_entity_texture_refs(), &[SNIFFER_TEXTURE_REF]);

    let images: Vec<EntityModelTextureImage> = sniffer_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::sniffer(900, [0.0, 64.0, 0.0], 0.0);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.texture, SNIFFER_TEXTURE_REF);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.transform, entity_model_root_transform(instance));
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    let mesh = &meshes.cutout;

    assert!(!mesh.vertices.is_empty());
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}
