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
    let fox = entity_model_mesh(&[EntityModelInstance::fox(400, [0.0, 64.0, 0.0], 0.0, false)]);
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
    let instances = [EntityModelInstance::fox(401, [0.0, 64.0, 0.0], 0.0, false)];
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
    let rest = EntityModelInstance::fox(402, [0.0, 64.0, 0.0], 0.0, false);
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
    let rest = EntityModelInstance::fox(420, [0.0, 64.0, 0.0], 0.0, false);
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
    let adult = entity_model_mesh(&[EntityModelInstance::fox(410, [0.0, 64.0, 0.0], 0.0, false)]);
    let baby = entity_model_mesh(&[EntityModelInstance::fox(411, [0.0, 64.0, 0.0], 0.0, true)]);
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
    let baby_rest = EntityModelInstance::fox(412, [0.0, 64.0, 0.0], 0.0, true);
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
    let awake = entity_model_mesh(&[EntityModelInstance::fox(430, [0.0, 64.0, 0.0], 0.0, false)]);
    assert_eq!(awake.opaque_faces, 60);
    assert_eq!(awake.vertices.len(), 240);

    let sleeping =
        entity_model_mesh(&[
            EntityModelInstance::fox(431, [0.0, 64.0, 0.0], 0.0, false).with_fox_is_sleeping(true)
        ]);
    assert_eq!(
        sleeping.opaque_faces, 36,
        "the four legs are hidden (6 cubes)"
    );
    assert_eq!(sleeping.vertices.len(), 144);

    // The baby (also 10 cubes) hides its four legs too.
    let baby_sleeping =
        entity_model_mesh(&[
            EntityModelInstance::fox(432, [0.0, 64.0, 0.0], 0.0, true).with_fox_is_sleeping(true)
        ]);
    assert_eq!(baby_sleeping.vertices.len(), 144);
}

#[test]
fn interested_fox_tilts_its_head() {
    // Vanilla `FoxModel.setWalkingPose` sets `head.zRot = headRollAngle`, the eased interest tilt. The
    // head is the first root part (vertices `[0, 96)`); the body, tail, and legs `[96, 240)` hold.
    let rest = EntityModelInstance::fox(440, [0.0, 64.0, 0.0], 0.0, false);
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
    let rest = EntityModelInstance::fox(450, [0.0, 64.0, 0.0], 0.0, false);
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
    let base =
        EntityModelInstance::fox(460, [0.0, 64.0, 0.0], 0.0, false).with_fox_is_sleeping(true);
    let at_zero = entity_model_mesh(&[base.with_age_in_ticks(0.0)]);
    let later = entity_model_mesh(&[base.with_age_in_ticks(40.0)]);
    assert_eq!(at_zero.vertices.len(), later.vertices.len());
    assert_ne!(
        at_zero.vertices, later.vertices,
        "the sleeping head roll wobbles with ageInTicks"
    );
}

#[test]
fn fox_exposes_stable_model_keys() {
    assert_eq!(EntityModelKind::Fox { baby: false }.model_key(), "fox");
    assert_eq!(EntityModelKind::Fox { baby: true }.model_key(), "fox_baby");
}

#[test]
fn fox_textured_render_matches_vanilla_renderer() {
    assert_eq!(fox_textured_layer_passes(false)[0].texture, FOX_TEXTURE_REF);
    assert_eq!(
        fox_textured_layer_passes(true)[0].texture,
        FOX_BABY_TEXTURE_REF
    );
    assert_eq!(
        fox_textured_layer_passes(false)[0].render_type,
        EntityModelLayerRenderType::Cutout
    );
    assert_eq!(
        EntityModelKind::Fox { baby: false }.vanilla_texture_ref(),
        Some(FOX_TEXTURE_REF)
    );
    assert_eq!(
        EntityModelKind::Fox { baby: true }.vanilla_texture_ref(),
        Some(FOX_BABY_TEXTURE_REF)
    );
    assert!(entity_model_texture_refs().contains(&FOX_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&FOX_BABY_TEXTURE_REF));
    assert_eq!(
        fox_entity_texture_refs(),
        &[FOX_TEXTURE_REF, FOX_BABY_TEXTURE_REF]
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
        let mesh = entity_model_textured_mesh(
            &[EntityModelInstance::fox(900, [0.0, 64.0, 0.0], 0.0, baby)],
            &atlas,
        );
        assert!(
            !mesh.vertices.is_empty(),
            "baby={baby} emits textured geometry"
        );
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    }
}
