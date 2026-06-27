use super::*;

#[test]
fn creaking_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `CreakingModel.createBodyLayer` (atlas 64×64): the mesh root holds one `root` part
    // at `offset(0, 24, 0)` parenting the `upper_body` pivot and the two legs.
    assert_eq!(CREAKING_ROOT_POSE.offset, [0.0, 24.0, 0.0]);

    // `upper_body` (empty pivot at (-1, -19, 0)) parents head / body / right_arm / left_arm.
    assert_eq!(CREAKING_UPPER_BODY_POSE.offset, [-1.0, -19.0, 0.0]);

    // `head`: the 6×10×6 skull, the 6×3×6 brow, and two 9×14×0 antler/branch planes.
    assert_eq!(CREAKING_HEAD_POSE.offset, [-3.0, -11.0, 0.0]);
    assert_eq!(CREAKING_HEAD_CUBES.len(), 4);
    assert_eq!(CREAKING_HEAD_CUBES[0].min, [-3.0, -10.0, -3.0]);
    assert_eq!(CREAKING_HEAD_CUBES[0].size, [6.0, 10.0, 6.0]);
    assert_eq!(CREAKING_HEAD_CUBES[2].size, [9.0, 14.0, 0.0]);
    assert_eq!(CREAKING_HEAD_CUBES[3].min, [-12.0, -14.0, 0.0]);

    // `body`: the 6×13×5 trunk plus the 6×7×5 block.
    assert_eq!(CREAKING_BODY_POSE.offset, [0.0, -7.0, 1.0]);
    assert_eq!(CREAKING_BODY_CUBES[0].size, [6.0, 13.0, 5.0]);
    assert_eq!(CREAKING_BODY_CUBES[1].min, [-6.0, -4.0, -3.0]);

    // The asymmetric arms: right is a 3×21×3 limb + hand, left a 3×16×3 limb + two blocks.
    assert_eq!(CREAKING_RIGHT_ARM_POSE.offset, [-7.0, -9.5, 1.5]);
    assert_eq!(CREAKING_RIGHT_ARM_CUBES[0].size, [3.0, 21.0, 3.0]);
    assert_eq!(CREAKING_LEFT_ARM_POSE.offset, [6.0, -9.0, 0.5]);
    assert_eq!(CREAKING_LEFT_ARM_CUBES.len(), 3);

    // The legs (each with a 5×0×9 foot plane); the right leg has an extra 3×3×3 hip block.
    assert_eq!(CREAKING_LEFT_LEG_POSE.offset, [1.5, -16.0, 0.5]);
    assert_eq!(CREAKING_LEFT_LEG_CUBES[1].size, [5.0, 0.0, 9.0]);
    assert_eq!(CREAKING_RIGHT_LEG_POSE.offset, [-1.0, -17.5, 0.5]);
    assert_eq!(CREAKING_RIGHT_LEG_CUBES.len(), 3);
    assert_eq!(CREAKING_RIGHT_LEG_CUBES[2].size, [3.0, 3.0, 3.0]);
}

#[test]
fn creaking_mesh_uses_vanilla_body_layer_geometry() {
    // 16 cubes → 96 faces / 384 vertices / 576 indices, all in the bark tint.
    let creaking = entity_model_mesh(&[EntityModelInstance::creaking(
        940,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_eq!(creaking.opaque_faces, 96);
    assert_eq!(creaking.vertices.len(), 384);
    assert_eq!(creaking.indices.len(), 576);
    assert!(creaking
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(CREAKING_BARK, 1.0)));
}

#[test]
fn creaking_head_follows_look_angles() {
    // Vanilla `CreakingModel.setupAnim` sets `head.xRot/yRot` from the plain look. The head is
    // nested root → upper_body → head (emitted first) — its four cubes (skull, brow, two antler
    // planes) are vertices [0, 96). With the walk at rest (speed 0 ⇒ amplitude 0), a non-zero look
    // re-poses only the head subtree; the body, the two arms, and the two legs stay at bind.
    let base = EntityModelInstance::creaking(941, [0.0, 64.0, 0.0], 0.0, false);
    let rest = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(35.0, -20.0)]);
    assert_eq!(rest.vertices.len(), looking.vertices.len());
    assert_ne!(
        rest.vertices[..96],
        looking.vertices[..96],
        "the head turns with the look"
    );
    assert_eq!(
        rest.vertices[96..],
        looking.vertices[96..],
        "the body, arms, and legs stay at bind"
    );
}

#[test]
fn creaking_walk_animation_matches_vanilla_definition() {
    // Vanilla `CreakingAnimation.CREAKING_WALK`: 1.125 s looping, animating upper_body, head, the
    // two arms (rotation), and the two legs (rotation + position). 53 keyframes total.
    assert_eq!(CREAKING_WALK.length_seconds, 1.125);
    assert!(CREAKING_WALK.looping);
    assert_eq!(CREAKING_WALK.bones.len(), 6);
    let keyframes: usize = CREAKING_WALK
        .bones
        .iter()
        .flat_map(|bone| bone.channels.iter())
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(keyframes, 53);

    // The upper_body rotation at t=0 is `degreeVec(26.8802, -23.399, -9.0616)`.
    let (_, ub_rot) = sample_bone_offsets(&CREAKING_WALK, "upper_body", 0.0, 1.0);
    assert!((ub_rot[0] - 26.8802_f32.to_radians()).abs() < 1.0e-5);
    assert!((ub_rot[1] - (-23.399_f32).to_radians()).abs() < 1.0e-5);

    // The right_leg has a position channel: at t=0 it offsets `posVec(0, 0.9674, -3.6578)` (y
    // negated). Linear midway through [0, 0.125] is the lerp toward `posVec(0, -0.2979, -0.9411)`.
    let (rl_pos, _) = sample_bone_offsets(&CREAKING_WALK, "right_leg", 0.0, 1.0);
    assert!((rl_pos[1] - -0.9674).abs() < 1.0e-5);
    assert!((rl_pos[2] - -3.6578).abs() < 1.0e-5);
    let (mid_pos, _) = sample_bone_offsets(&CREAKING_WALK, "right_leg", 0.0625, 1.0);
    let expected_z = -3.6578 + (-0.9411 - (-3.6578)) * 0.5;
    assert!(
        (mid_pos[2] - expected_z).abs() < 1.0e-4,
        "z was {}",
        mid_pos[2]
    );
}

#[test]
fn creaking_walk_moves_the_limbs_and_composes_with_the_look() {
    // A still creaking (walk speed 0) samples the cycle at amplitude 0, collapsing to the bind pose;
    // a walking creaking samples CREAKING_WALK across the upper body, arms, and legs. The vertex
    // count is preserved.
    let still = entity_model_mesh(&[EntityModelInstance::creaking(
        942,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let walking =
        entity_model_mesh(&[
            EntityModelInstance::creaking(943, [0.0, 64.0, 0.0], 0.0, false)
                .with_walk_animation(5.0, 1.0),
        ]);
    assert_eq!(still.vertices.len(), walking.vertices.len());
    assert_ne!(
        still.vertices, walking.vertices,
        "the walking creaking animates its limbs"
    );

    // The head walk channel adds onto the look, so a walking + looking creaking differs from one
    // that only walks (the head re-poses further).
    let walking_looking =
        entity_model_mesh(&[
            EntityModelInstance::creaking(944, [0.0, 64.0, 0.0], 0.0, false)
                .with_walk_animation(5.0, 1.0)
                .with_head_look(30.0, -15.0),
        ]);
    assert_ne!(
        walking.vertices[..96],
        walking_looking.vertices[..96],
        "the look composes onto the walking head"
    );
}

#[test]
fn creaking_combat_animations_match_vanilla_definitions() {
    let total_keyframes = |def: &AnimationDefinition| -> usize {
        def.bones
            .iter()
            .flat_map(|bone| bone.channels.iter())
            .map(|channel| channel.keyframes.len())
            .sum()
    };

    // Vanilla `CreakingAnimation.CREAKING_ATTACK`: 0.7083 s looping, 6 bones, 68 keyframes.
    assert_eq!(CREAKING_ATTACK.length_seconds, 0.7083);
    assert!(CREAKING_ATTACK.looping);
    assert_eq!(CREAKING_ATTACK.bones.len(), 6);
    assert_eq!(total_keyframes(&CREAKING_ATTACK), 68);
    // The head carries a SCALE channel: at the strike (t=0.5) it stretches y by `scaleVec(1, 1.3, 1)`
    // (offset 0.3 onto the `1.0` base).
    let (_, _, head_scale) = sample_bone_offsets_with_scale(&CREAKING_ATTACK, "head", 0.5, 1.0);
    assert!(
        (head_scale[1] - 0.3).abs() < 1.0e-5,
        "scale y was {head_scale:?}"
    );
    // The upper_body rotation at t=0.1667 is `degreeVec(-115, 67.5, -90)`.
    let (_, ub_rot, _) =
        sample_bone_offsets_with_scale(&CREAKING_ATTACK, "upper_body", 0.1667, 1.0);
    assert!((ub_rot[0] - (-115.0_f32).to_radians()).abs() < 1.0e-4);
    assert!((ub_rot[2] - (-90.0_f32).to_radians()).abs() < 1.0e-4);

    // Vanilla `CreakingAnimation.CREAKING_INVULNERABLE`: 0.2917 s NOT looping, 3 bones, 19 keyframes.
    assert_eq!(CREAKING_INVULNERABLE.length_seconds, 0.2917);
    assert!(!CREAKING_INVULNERABLE.looping);
    assert_eq!(CREAKING_INVULNERABLE.bones.len(), 3);
    assert_eq!(total_keyframes(&CREAKING_INVULNERABLE), 19);
    // The right_arm rotation at t=0.0833 is `degreeVec(17.5, 0, 0)`.
    let (_, ra_rot, _) =
        sample_bone_offsets_with_scale(&CREAKING_INVULNERABLE, "right_arm", 0.0833, 1.0);
    assert!((ra_rot[0] - 17.5_f32.to_radians()).abs() < 1.0e-4);

    // Vanilla `CreakingAnimation.CREAKING_DEATH`: 2.25 s NOT looping, 4 bones, 52 keyframes, with an
    // upper_body SCALE squash/stretch (`scaleVec(1, 1.1, 1)` at t=0.0833 → offset 0.1).
    assert_eq!(CREAKING_DEATH.length_seconds, 2.25);
    assert!(!CREAKING_DEATH.looping);
    assert_eq!(CREAKING_DEATH.bones.len(), 4);
    assert_eq!(total_keyframes(&CREAKING_DEATH), 52);
    let (_, _, ub_scale) =
        sample_bone_offsets_with_scale(&CREAKING_DEATH, "upper_body", 0.0833, 1.0);
    assert!(
        (ub_scale[1] - 0.1).abs() < 1.0e-5,
        "scale y was {ub_scale:?}"
    );
}

#[test]
fn creaking_can_move_false_freezes_the_walk() {
    // Vanilla `setupAnim` gates `applyWalk` on `state.canMove`. An observed (canMove=false) walking
    // creaking skips the walk and holds the bind pose, identical to a still creaking — it turns to a
    // statue. Clearing the freeze lets the walk animate the limbs again.
    let base = EntityModelInstance::creaking(945, [0.0, 64.0, 0.0], 0.0, false);
    let still = entity_model_mesh(&[base]);
    let frozen = entity_model_mesh(&[base
        .with_walk_animation(5.0, 1.0)
        .with_creaking_can_move(false)]);
    assert_eq!(
        still.vertices, frozen.vertices,
        "a frozen creaking holds the bind pose"
    );
    let walking = entity_model_mesh(&[base.with_walk_animation(5.0, 1.0)]);
    assert_ne!(
        frozen.vertices, walking.vertices,
        "clearing the freeze animates the walk"
    );
}

#[test]
fn creaking_combat_keyframes_re_pose_off_the_bind_pose() {
    // Each triggered one-shot, applied over its projected elapsed seconds, re-poses the creaking off
    // the bind pose; the `-1.0` stopped sentinel applies nothing. The upper_body rotation cascades to
    // the whole upper body, and the leg channels swing the legs.
    let base = EntityModelInstance::creaking(948, [0.0, 64.0, 0.0], 0.0, false);
    let rest = entity_model_mesh(&[base]);

    let attacking = entity_model_mesh(&[base.with_creaking_attack_seconds(0.25)]);
    assert_eq!(rest.vertices.len(), attacking.vertices.len());
    assert_ne!(
        rest.vertices[..96],
        attacking.vertices[..96],
        "the attack swings the head subtree"
    );
    assert_ne!(
        rest.vertices[264..],
        attacking.vertices[264..],
        "the attack swings the legs"
    );
    assert_eq!(
        rest.vertices,
        entity_model_mesh(&[base.with_creaking_attack_seconds(-1.0)]).vertices,
        "the -1.0 sentinel applies no attack"
    );

    // The invulnerable stagger re-poses the upper body and arms (it has no leg channels).
    let invuln = entity_model_mesh(&[base.with_creaking_invulnerable_seconds(0.1)]);
    assert_ne!(
        rest.vertices[..264],
        invuln.vertices[..264],
        "the stagger recoils the upper body and arms"
    );

    // The death collapse re-poses the upper body, head, and arms over its 2.25 s span.
    let dying = entity_model_mesh(&[base.with_creaking_death_seconds(0.5)]);
    assert_ne!(
        rest.vertices, dying.vertices,
        "the death collapse re-poses the model"
    );
}

#[test]
fn creaking_textured_render_matches_vanilla_renderer() {
    // An inactive creaking renders just the cutout base body.
    let dormant = creaking_textured_layer_passes(false);
    assert_eq!(dormant.len(), 1);
    assert_eq!(dormant[0].kind, EntityModelLayerKind::CreakingBase);
    assert_eq!(
        dormant[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(dormant[0].texture, CREAKING_TEXTURE_REF);
    // An active creaking adds the emissive `creaking_eyes.png` eyes overlay.
    let glowing = creaking_textured_layer_passes(true);
    assert_eq!(glowing.len(), 2);
    assert_eq!(glowing[0].kind, EntityModelLayerKind::CreakingBase);
    assert_eq!(glowing[1].kind, EntityModelLayerKind::CreakingEyes);
    assert_eq!(glowing[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(glowing[1].texture, CREAKING_EYES_TEXTURE_REF);
    assert_eq!((glowing[1].order, glowing[1].submit_sequence), (1, 1));
    assert_eq!(
        EntityModelKind::Creaking { eyes_glowing: true }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/creaking/creaking.png",
            size: [64, 64],
        })
    );
    assert!(entity_model_texture_refs().contains(&CREAKING_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&CREAKING_EYES_TEXTURE_REF));
    assert_eq!(
        creaking_entity_texture_refs(),
        &[CREAKING_TEXTURE_REF, CREAKING_EYES_TEXTURE_REF]
    );

    let images: Vec<EntityModelTextureImage> = creaking_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::creaking(900, [0.0, 64.0, 0.0], 0.0, true)
        .with_light_coords((5_u32 << 4) | (11_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert_eq!(meshes.submissions.len(), 2);
    let base = meshes.submissions[0];
    assert_eq!(base.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(base.render_type.vanilla_name(), "entityCutout");
    assert_eq!(base.texture, CREAKING_TEXTURE_REF);
    assert_eq!(base.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(base.transform, entity_model_root_transform(instance));
    assert_eq!(base.light, instance.render_state.shader_light());
    assert_eq!(base.overlay, instance.render_state.overlay_coords());
    assert_ne!(base.overlay, [0.0, 10.0]);
    assert_eq!((base.order, base.submit_sequence), (0, 0));
    let eyes = meshes.submissions[1];
    assert_eq!(eyes.render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(eyes.render_type.vanilla_name(), "eyes");
    assert_eq!(eyes.texture, CREAKING_EYES_TEXTURE_REF);
    assert_eq!(eyes.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(eyes.transform, base.transform);
    assert_eq!(eyes.light, instance.render_state.shader_light());
    assert_eq!(
        eyes.overlay,
        [0.0, instance.render_state.overlay_coords()[1]]
    );
    assert_ne!(eyes.overlay, base.overlay);
    assert_ne!(eyes.overlay, [0.0, 10.0]);
    assert_eq!((eyes.order, eyes.submit_sequence), (1, 1));
    let mesh = &meshes.cutout;

    assert!(!mesh.vertices.is_empty());
    assert_eq!(meshes.eyes.vertices.len(), mesh.vertices.len());
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.light == base.light && vertex.overlay == base.overlay));
    assert!(meshes
        .eyes
        .vertices
        .iter()
        .all(|vertex| vertex.light == eyes.light && vertex.overlay == eyes.overlay));
}
