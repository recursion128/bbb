use super::*;

#[test]
fn fox_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultFoxModel.createBodyLayer` (atlas 48×32): six named root parts — head (with ears +
    // snout), body (with tail), and four legs.

    // `head` (offset (-1, 16.5, -3)): the 8×6×6 skull parenting the two 2×2×1 ears and the 4×2×3 snout.
    assert_eq!(FOX_HEAD_POSE.offset, [-1.0, 16.5, -3.0]);
    assert_eq!(FOX_HEAD_CUBES[0].min, [-3.0, -2.0, -5.0]);
    assert_eq!(FOX_HEAD_CUBES[0].size, [8.0, 6.0, 6.0]);
    // The ears and snout all sit at the head origin (PartPose.ZERO).
    assert_eq!(FOX_HEAD_CHILD_POSE.offset, [0.0, 0.0, 0.0]);
    assert_eq!(FOX_RIGHT_EAR_CUBES[0].min, [-3.0, -4.0, -4.0]);
    assert_eq!(FOX_LEFT_EAR_CUBES[0].min, [3.0, -4.0, -4.0]);
    assert_eq!(FOX_NOSE_CUBES[0].min, [-1.0, 2.01, -8.0]);
    assert_eq!(FOX_NOSE_CUBES[0].size, [4.0, 2.0, 3.0]);

    // `body` (offset (0, 16, -6), pitched π/2): the 6×11×6 trunk parenting the tail.
    assert_eq!(FOX_BODY_POSE.offset, [0.0, 16.0, -6.0]);
    assert_eq!(
        FOX_BODY_POSE.rotation,
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0]
    );
    assert_eq!(FOX_BODY_CUBES[0].min, [-3.0, 3.999, -3.5]);
    assert_eq!(FOX_BODY_CUBES[0].size, [6.0, 11.0, 6.0]);
    // `tail` (offset (-4, 15, -1), pitched -0.05235988): the 4×9×5 brush.
    assert_eq!(FOX_TAIL_POSE.offset, [-4.0, 15.0, -1.0]);
    assert_eq!(FOX_TAIL_POSE.rotation, [-0.05235988, 0.0, 0.0]);
    assert_eq!(FOX_TAIL_CUBES[0].min, [2.0, 0.0, -1.0]);
    assert_eq!(FOX_TAIL_CUBES[0].size, [4.0, 9.0, 5.0]);

    // The four legs share one fudge-inflated box (built off-center at +2 X); hind at z=7, front at z=0,
    // the right pair at pivot x=-5, the left pair at pivot x=-1.
    assert_eq!(FOX_RIGHT_HIND_LEG_POSE.offset, [-5.0, 17.5, 7.0]);
    assert_eq!(FOX_LEFT_HIND_LEG_POSE.offset, [-1.0, 17.5, 7.0]);
    assert_eq!(FOX_RIGHT_FRONT_LEG_POSE.offset, [-5.0, 17.5, 0.0]);
    assert_eq!(FOX_LEFT_FRONT_LEG_POSE.offset, [-1.0, 17.5, 0.0]);
    assert_eq!(FOX_LEG_CUBES[0].min, [1.999, 0.499, -1.001]);
    assert_eq!(FOX_LEG_CUBES[0].size, [2.002, 6.002, 2.002]);
}

#[test]
fn fox_mesh_uses_vanilla_body_layer_geometry() {
    // 10 cubes → 60 faces / 240 vertices / 360 indices, one orange tint.
    let fox = entity_model_mesh(&[EntityModelInstance::fox(
        400,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        FoxModelVariant::Red,
    )]);
    assert_eq!(fox.opaque_faces, 60);
    assert_eq!(fox.vertices.len(), 240);
    assert_eq!(fox.indices.len(), 360);
    assert!(fox
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(FOX_ORANGE, 1.0)));
}

#[test]
fn fox_colored_runtime_skips_the_texture_backed_fox() {
    // The fox now carries vanilla texture UVs, so it renders through the textured path. The
    // texture-skipping colored runtime path emits nothing for it, while the full path still emits the
    // colored fallback geometry.
    let instances = [EntityModelInstance::fox(
        401,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        FoxModelVariant::Red,
    )];
    assert!(!entity_model_mesh(&instances).vertices.is_empty());
    assert!(entity_model_colored_runtime_mesh(&instances)
        .vertices
        .is_empty());
}

#[test]
fn fox_head_look_turns_only_the_head() {
    // Vanilla `FoxModel.setupAnim` sets `head.xRot/yRot` from the look while standing. The head is the
    // first root part (skull + ears + snout, four cubes → vertices `[0, 96)`); the body, tail, and legs
    // `[96, 240)` hold.
    let rest = EntityModelInstance::fox(402, [0.0, 64.0, 0.0], 0.0, false, FoxModelVariant::Red);
    let looked = rest.with_head_look(35.0, -25.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_eq!(rest_mesh.vertices.len(), looked_mesh.vertices.len());
    assert_ne!(
        rest_mesh.vertices[..96],
        looked_mesh.vertices[..96],
        "the head (skull, ears, snout) turns"
    );
    assert_eq!(
        rest_mesh.vertices[96..],
        looked_mesh.vertices[96..],
        "the body, tail, and legs stay put"
    );

    // Both yaw and pitch move the head.
    let yaw_only = entity_model_mesh(&[rest.with_head_look(35.0, 0.0)]);
    let pitch_only = entity_model_mesh(&[rest.with_head_look(0.0, -25.0)]);
    assert_ne!(rest_mesh.vertices[..96], yaw_only.vertices[..96]);
    assert_ne!(rest_mesh.vertices[..96], pitch_only.vertices[..96]);
}

#[test]
fn fox_legs_swing_with_the_gait() {
    // Vanilla `AdultFoxModel.setWalkingPose` sweeps the four legs by `cos(pos·0.6662 [+π])·1.4·speed`
    // (the back-right/front-left diagonal in phase, the other half a cycle out). The head (vertices
    // `[0, 96)`) is untouched by the swing; the legs (in `[96, 240)`) move.
    let rest = EntityModelInstance::fox(420, [0.0, 64.0, 0.0], 0.0, false, FoxModelVariant::Red);
    let walking = rest.with_walk_animation(3.0, 0.8);
    let rest_mesh = entity_model_mesh(&[rest]);
    let walk_mesh = entity_model_mesh(&[walking]);
    assert_eq!(rest_mesh.vertices.len(), walk_mesh.vertices.len());
    assert_ne!(
        rest_mesh.vertices, walk_mesh.vertices,
        "the legs swing with the gait"
    );
    assert_eq!(
        rest_mesh.vertices[..96],
        walk_mesh.vertices[..96],
        "the head is untouched by the leg swing"
    );

    // The swing tracks the walk position, not just the speed.
    let walk_later = entity_model_mesh(&[rest.with_walk_animation(6.0, 0.8)]);
    assert_ne!(
        walk_mesh.vertices, walk_later.vertices,
        "the swing advances with the walk position"
    );

    // At zero speed the swing is a no-op even with a non-zero position (the limbs are at rest).
    let still = entity_model_mesh(&[rest.with_walk_animation(3.0, 0.0)]);
    assert_eq!(
        still.vertices, rest_mesh.vertices,
        "a standing fox holds the bind leg pose"
    );
}

#[test]
fn fox_baby_walk_animation_matches_vanilla_definition() {
    // Vanilla `FoxBabyAnimation.FOX_BABY_WALK`: a 0.5s LOOPING gait over seven bones (body, head, the
    // four legs, the tail), the legs carrying ROTATION + POSITION + SCALE channels.
    assert_eq!(FOX_BABY_WALK.length_seconds, 0.5);
    assert!(FOX_BABY_WALK.looping);
    assert_eq!(FOX_BABY_WALK.bones.len(), 7);

    // The right hind leg kicks to `degreeVec(35, 0, 0)` at its `0.25` keyframe (sampled at full
    // amplitude `scale = 1.0`).
    let (_, rot, scale_off) =
        sample_bone_offsets_with_scale(&FOX_BABY_WALK, "right_hind_leg", 0.25, 1.0);
    assert!((rot[0] - 35.0_f32.to_radians()).abs() < 1.0e-4);
    // Every leg stretches 1.15× on y (`scaleVec(1, 1.15, 1)` → a `0.15` offset from the `1.0` base).
    assert!((scale_off[1] - 0.15).abs() < 1.0e-4);

    // The head lifts by `posVec(0, -1.025, 0)` (y negated → `+1.025`), a constant offset scaled by the
    // walk amplitude.
    let (head_pos, _, _) = sample_bone_offsets_with_scale(&FOX_BABY_WALK, "head", 0.0, 1.0);
    assert!((head_pos[1] - 1.025).abs() < 1.0e-4);

    // At zero amplitude (a standing baby) every channel collapses to nothing.
    let (z_pos, z_rot, z_scale) =
        sample_bone_offsets_with_scale(&FOX_BABY_WALK, "right_hind_leg", 0.25, 0.0);
    assert_eq!((z_pos, z_rot, z_scale), ([0.0; 3], [0.0; 3], [0.0; 3]));
}

#[test]
fn baby_fox_scampers_with_the_keyframe_gait() {
    // Unlike the adult swing (which leaves the head alone), the baby's `FOX_BABY_WALK` lifts the head
    // and cocks the tail as well as kicking the legs, so a walking baby re-poses off the bind pose
    // across the whole mesh. The baby head is `[0, 96)`.
    let rest = EntityModelInstance::fox(430, [0.0, 64.0, 0.0], 0.0, true, FoxModelVariant::Red);
    let rest_mesh = entity_model_mesh(&[rest]);
    let walk_mesh = entity_model_mesh(&[rest.with_walk_animation(3.0, 0.8)]);
    assert_eq!(rest_mesh.vertices.len(), walk_mesh.vertices.len());
    assert_ne!(
        rest_mesh.vertices, walk_mesh.vertices,
        "the baby scampers off the bind pose"
    );
    assert_ne!(
        rest_mesh.vertices[..96],
        walk_mesh.vertices[..96],
        "the baby's head lifts with the gait (the adult's does not)"
    );

    // The gait tracks the walk position, not just the speed.
    let walk_later = entity_model_mesh(&[rest.with_walk_animation(6.0, 0.8)]);
    assert_ne!(
        walk_mesh.vertices, walk_later.vertices,
        "the baby gait advances with the walk position"
    );

    // A standing baby (zero speed) holds the bind pose, whatever the position.
    let still = entity_model_mesh(&[rest.with_walk_animation(3.0, 0.0)]);
    assert_eq!(
        still.vertices, rest_mesh.vertices,
        "a standing baby holds the bind pose"
    );
}

#[test]
fn baby_fox_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyFoxModel.createBodyLayer` (atlas 32×32): six named root parts — head (ears + snout
    // baked in as cubes), four legs, then body (with tail). Flatter than the adult, body has no pitch.

    assert_eq!(BABY_FOX_HEAD_POSE.offset, [0.0, 18.125, 0.125]);
    assert_eq!(BABY_FOX_HEAD_CUBES.len(), 4);
    assert_eq!(BABY_FOX_HEAD_CUBES[0].min, [-3.0, -2.125, -5.125]);
    assert_eq!(BABY_FOX_HEAD_CUBES[0].size, [6.0, 5.0, 5.0]);

    // Legs (right-hind / left-hind / right-front / left-front), the 2×2×2 box.
    assert_eq!(BABY_FOX_RIGHT_HIND_LEG_POSE.offset, [-1.5, 22.0, 4.0]);
    assert_eq!(BABY_FOX_LEFT_FRONT_LEG_POSE.offset, [1.5, 22.0, 0.0]);
    assert_eq!(BABY_FOX_LEG_CUBES[0].size, [2.0, 2.0, 2.0]);

    // `body` (no pitch) parenting the tail.
    assert_eq!(BABY_FOX_BODY_POSE.offset, [0.0, 20.0, 2.0]);
    assert_eq!(BABY_FOX_BODY_POSE.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(BABY_FOX_BODY_CUBES[0].size, [5.0, 4.0, 6.0]);
    assert_eq!(BABY_FOX_TAIL_POSE.offset, [0.0, -0.5, 3.0]);
    assert_eq!(BABY_FOX_TAIL_CUBES[0].size, [3.0, 3.0, 6.0]);
}

#[test]
fn baby_fox_mesh_is_more_compact_than_the_adult() {
    // The baby uses a smaller body layer, so its mesh is geometrically more compact than the adult's
    // (both 10 cubes → 240 vertices). Head is part 0 in both layouts, so the head look isolates it.
    let adult = entity_model_mesh(&[EntityModelInstance::fox(
        410,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        FoxModelVariant::Red,
    )]);
    let baby = entity_model_mesh(&[EntityModelInstance::fox(
        411,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        FoxModelVariant::Red,
    )]);
    assert_eq!(baby.vertices.len(), 240);
    assert!(baby
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(FOX_ORANGE, 1.0)));

    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    let adult_span = adult_max[2] - adult_min[2];
    let baby_span = baby_max[2] - baby_min[2];
    assert!(
        baby_span < adult_span,
        "baby z-span {baby_span} should be smaller than adult {adult_span}"
    );

    // The baby head (part 0, vertices [0, 96)) turns with the look; the rest holds.
    let baby_rest =
        EntityModelInstance::fox(412, [0.0, 64.0, 0.0], 0.0, true, FoxModelVariant::Red);
    let baby_rest_mesh = entity_model_mesh(&[baby_rest]);
    let baby_looked_mesh = entity_model_mesh(&[baby_rest.with_head_look(35.0, -25.0)]);
    assert_ne!(
        baby_rest_mesh.vertices[..96],
        baby_looked_mesh.vertices[..96]
    );
    assert_eq!(
        baby_rest_mesh.vertices[96..],
        baby_looked_mesh.vertices[96..]
    );
}

#[test]
fn sleeping_fox_hides_its_four_legs() {
    // Vanilla `FoxModel.setSleepingPose` hides all four legs (`rightHindLeg.visible = false`, …),
    // mirroring how the bee hides its stinger. The adult is 10 cubes → 240 vertices; the four legs are
    // four cubes (96 vertices), so a sleeping fox drops to six cubes → 144 vertices.
    let awake = entity_model_mesh(&[EntityModelInstance::fox(
        430,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        FoxModelVariant::Red,
    )]);
    assert_eq!(awake.opaque_faces, 60);
    assert_eq!(awake.vertices.len(), 240);

    let sleeping = entity_model_mesh(&[EntityModelInstance::fox(
        431,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        FoxModelVariant::Red,
    )
    .with_fox_is_sleeping(true)]);
    assert_eq!(
        sleeping.opaque_faces, 36,
        "the four legs are hidden (6 cubes)"
    );
    assert_eq!(sleeping.vertices.len(), 144);

    // The baby (also 10 cubes) hides its four legs too.
    let baby_sleeping = entity_model_mesh(&[EntityModelInstance::fox(
        432,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        FoxModelVariant::Red,
    )
    .with_fox_is_sleeping(true)]);
    assert_eq!(baby_sleeping.vertices.len(), 144);
}

#[test]
fn interested_fox_tilts_its_head() {
    // Vanilla `FoxModel.setWalkingPose` sets `head.zRot = headRollAngle`, the eased interest tilt. The
    // head is the first root part (vertices `[0, 96)`); the body, tail, and legs `[96, 240)` hold.
    let rest = EntityModelInstance::fox(440, [0.0, 64.0, 0.0], 0.0, false, FoxModelVariant::Red);
    let rest_mesh = entity_model_mesh(&[rest]);
    let tilted = entity_model_mesh(&[rest.with_fox_head_roll_angle(0.3)]);
    assert_eq!(rest_mesh.vertices.len(), tilted.vertices.len());
    assert_ne!(
        rest_mesh.vertices[..96],
        tilted.vertices[..96],
        "the head rolls with the interest tilt"
    );
    assert_eq!(
        rest_mesh.vertices[96..],
        tilted.vertices[96..],
        "the body, tail, and legs hold"
    );

    // A zero roll is the bind pose (the tilt is just `head.zRot = 0`).
    let zero = entity_model_mesh(&[rest.with_fox_head_roll_angle(0.0)]);
    assert_eq!(rest_mesh.vertices, zero.vertices);
}

#[test]
fn crouching_fox_lowers_its_body_off_the_bind_pose() {
    // Vanilla `FoxModel.setCrouchingPose` pitches the body, lifts the head by `crouchAmount · ageScale`,
    // and wiggles the body/legs by `cos(ageInTicks) · 0.05`, plus the adult `body.y += crouchAmount`.
    // The whole model re-poses off the bind pose.
    let rest = EntityModelInstance::fox(450, [0.0, 64.0, 0.0], 0.0, false, FoxModelVariant::Red);
    let rest_mesh = entity_model_mesh(&[rest]);
    let crouching = rest.with_fox_is_crouching(true).with_fox_crouch_amount(5.0);
    let crouch_mesh = entity_model_mesh(&[crouching]);
    assert_eq!(rest_mesh.vertices.len(), crouch_mesh.vertices.len());
    assert_ne!(
        rest_mesh.vertices, crouch_mesh.vertices,
        "a crouching fox folds off the bind pose"
    );

    // A deeper crouch lifts the head further (the `crouchAmount · ageScale` head term), so the pose
    // tracks the eased amount, not just the flag.
    let shallow =
        entity_model_mesh(&[rest.with_fox_is_crouching(true).with_fox_crouch_amount(1.0)]);
    assert_ne!(
        shallow.vertices, crouch_mesh.vertices,
        "the crouch pose tracks the eased crouchAmount"
    );
}

#[test]
fn sleeping_fox_overrides_the_head_pose() {
    // Vanilla `FoxModel.setupAnim` gives a sleeping fox a fixed head yaw with a slow `ageInTicks` roll
    // wobble, and suppresses the resting head look. The head re-poses even with no look applied, and the
    // wobble tracks `ageInTicks`.
    let base = EntityModelInstance::fox(460, [0.0, 64.0, 0.0], 0.0, false, FoxModelVariant::Red)
        .with_fox_is_sleeping(true);
    let at_zero = entity_model_mesh(&[base.with_age_in_ticks(0.0)]);
    let later = entity_model_mesh(&[base.with_age_in_ticks(40.0)]);
    assert_eq!(at_zero.vertices.len(), later.vertices.len());
    assert_ne!(
        at_zero.vertices, later.vertices,
        "the sleeping head roll wobbles with ageInTicks"
    );
}

#[test]
fn fox_pounce_and_faceplant_setup_rotations_pitch_the_whole_model() {
    // Vanilla `FoxRenderer.setupRotations`: after `super.setupRotations`, pouncing or faceplanted foxes
    // rotate the whole model by `Axis.XP.rotationDegrees(-state.xRot)`. Use a baby fox for the pounce
    // check because `BabyFoxModel` has no pounce model-pose override; any non-head movement here is the
    // renderer root pitch, not `AdultFoxModel.setPouncingPose`.
    let base = EntityModelInstance::fox(470, [0.0, 64.0, 0.0], 0.0, true, FoxModelVariant::Red);

    let standing_flat = entity_model_mesh(&[base.with_head_look(0.0, 0.0)]);
    let standing_pitched = entity_model_mesh(&[base.with_head_look(0.0, 30.0)]);
    assert_eq!(
        standing_flat.vertices[96..],
        standing_pitched.vertices[96..],
        "without the renderer pounce/faceplant branch, pitch only turns the head"
    );

    let pouncing_flat =
        entity_model_mesh(&[base.with_fox_is_pouncing(true).with_head_look(0.0, 0.0)]);
    let pouncing_pitched =
        entity_model_mesh(&[base.with_fox_is_pouncing(true).with_head_look(0.0, 30.0)]);
    assert_ne!(
        pouncing_flat.vertices[96..],
        pouncing_pitched.vertices[96..],
        "pouncing applies the pitch to the whole model"
    );

    let faceplanted_flat =
        entity_model_mesh(&[base.with_fox_is_faceplanted(true).with_head_look(0.0, 0.0)]);
    let faceplanted_pitched =
        entity_model_mesh(&[base.with_fox_is_faceplanted(true).with_head_look(0.0, 30.0)]);
    assert_ne!(
        faceplanted_flat.vertices[96..],
        faceplanted_pitched.vertices[96..],
        "faceplanted applies the pitch to the whole model"
    );
}

#[test]
fn fox_exposes_stable_model_keys() {
    // The model key tracks only the body layout (adult/baby); the colour variant shares geometry.
    for variant in [FoxModelVariant::Red, FoxModelVariant::Snow] {
        assert_eq!(
            EntityModelKind::Fox {
                baby: false,
                variant
            }
            .model_key(),
            "fox"
        );
        assert_eq!(
            EntityModelKind::Fox {
                baby: true,
                variant
            }
            .model_key(),
            "fox_baby"
        );
    }
}

#[test]
fn fox_textured_render_matches_vanilla_renderer() {
    // `FoxRenderer.getTextureLocation`: the {red, snow} × {adult, baby} × {idle, sleeping} matrix.
    for variant in [FoxModelVariant::Red, FoxModelVariant::Snow] {
        for baby in [false, true] {
            for sleeping in [false, true] {
                let texture = fox_texture_ref(variant, baby, sleeping);
                assert_eq!(
                    fox_textured_layer_passes(variant, baby, sleeping)[0].texture,
                    texture
                );
                assert_eq!(
                    fox_textured_layer_passes(variant, baby, sleeping)[0].render_type,
                    EntityModelLayerRenderType::EntityCutout
                );
                assert!(entity_model_texture_refs().contains(&texture));
            }
            // The kind-only `vanilla_texture_ref` returns the idle cell (sleeping is render state).
            assert_eq!(
                EntityModelKind::Fox { baby, variant }.vanilla_texture_ref(),
                Some(fox_texture_ref(variant, baby, false))
            );
        }
    }
    assert_eq!(
        fox_entity_texture_refs(),
        &[
            FOX_RED_TEXTURE_REF,
            FOX_RED_BABY_TEXTURE_REF,
            FOX_RED_SLEEP_TEXTURE_REF,
            FOX_RED_SLEEP_BABY_TEXTURE_REF,
            FOX_SNOW_TEXTURE_REF,
            FOX_SNOW_BABY_TEXTURE_REF,
            FOX_SNOW_SLEEP_TEXTURE_REF,
            FOX_SNOW_SLEEP_BABY_TEXTURE_REF,
        ]
    );

    let images: Vec<EntityModelTextureImage> = fox_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    for baby in [false, true] {
        for sleeping in [false, true] {
            let instance =
                EntityModelInstance::fox(900, [0.0, 64.0, 0.0], 0.0, baby, FoxModelVariant::Snow)
                    .with_fox_is_sleeping(sleeping);
            let meshes = entity_model_textured_meshes(&[instance], &atlas);
            assert!(meshes.translucent.vertices.is_empty());
            assert!(meshes.eyes.vertices.is_empty());
            assert_eq!(meshes.submissions.len(), 1);
            let submit = meshes.submissions[0];
            assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
            assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
            assert_eq!(
                submit.texture,
                fox_texture_ref(FoxModelVariant::Snow, baby, sleeping)
            );
            assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
            assert_eq!(submit.transform, fox_model_root_transform(instance));
            assert_eq!((submit.order, submit.submit_sequence), (0, 0));
            let mesh = &meshes.cutout;

            assert!(
                !mesh.vertices.is_empty(),
                "baby={baby} sleeping={sleeping} emits textured geometry"
            );
            assert!(mesh
                .vertices
                .iter()
                .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
        }
    }

    let pouncing =
        EntityModelInstance::fox(901, [0.0, 64.0, 0.0], 0.0, true, FoxModelVariant::Snow)
            .with_fox_is_pouncing(true)
            .with_head_look(0.0, 30.0);
    let meshes = entity_model_textured_meshes(&[pouncing], &atlas);
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(
        submit.texture,
        fox_texture_ref(FoxModelVariant::Snow, true, false)
    );
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.transform, fox_model_root_transform(pouncing));
    assert_ne!(submit.transform, entity_model_root_transform(pouncing));
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
}
