use super::*;
use crate::entity_models::model_layers::iron_golem_model;

#[test]
fn iron_golem_crackiness_from_health_fraction_matches_vanilla() {
    // Vanilla `Crackiness.GOLEM` = (0.75, 0.5, 0.25): below 0.25 high, below 0.5 medium, below 0.75
    // low, otherwise none. Full health (100/100 = 1.0) is uncracked.
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(1.0),
        IronGolemCrackiness::None
    );
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(0.75),
        IronGolemCrackiness::None
    );
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(0.74),
        IronGolemCrackiness::Low
    );
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(0.5),
        IronGolemCrackiness::Low
    );
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(0.49),
        IronGolemCrackiness::Medium
    );
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(0.25),
        IronGolemCrackiness::Medium
    );
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(0.24),
        IronGolemCrackiness::High
    );
    assert_eq!(
        IronGolemCrackiness::from_health_fraction(0.0),
        IronGolemCrackiness::High
    );
}

#[test]
fn iron_golem_textured_layer_pass_matches_vanilla_renderer_model_layer() {
    // An uncracked golem is a single base pass.
    let passes = iron_golem_textured_layer_passes(IronGolemCrackiness::None);
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].texture, IRON_GOLEM_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);

    // A cracked golem appends the matching crack overlay (Cutout, white, same model layer) — vanilla
    // `IronGolemCrackinessLayer.renderColoredCutoutModel`.
    for (crackiness, texture) in [
        (
            IronGolemCrackiness::Low,
            IRON_GOLEM_CRACKINESS_LOW_TEXTURE_REF,
        ),
        (
            IronGolemCrackiness::Medium,
            IRON_GOLEM_CRACKINESS_MEDIUM_TEXTURE_REF,
        ),
        (
            IronGolemCrackiness::High,
            IRON_GOLEM_CRACKINESS_HIGH_TEXTURE_REF,
        ),
    ] {
        let passes = iron_golem_textured_layer_passes(crackiness);
        assert_eq!(passes.len(), 2);
        assert_eq!(
            passes[1].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[1].texture, texture);
        assert_eq!(passes[1].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(passes[1].order, 1);
    }
}

#[test]
fn iron_golem_renderer_body_wobble_matches_vanilla_setup_rotations() {
    // Vanilla `IronGolemRenderer.setupRotations` appends the renderer-level Z wobble after
    // `super.setupRotations` and before the model flip. At walk pos 0:
    //   triangleWave(0 + 6, 13) = (|6 - 6.5| - 3.25) / 3.25 = -11/13.
    let position = [1.0, 64.0, -2.0];
    let base = EntityModelInstance::iron_golem(75, position, 0.0);

    let still = base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (3.0, 0.009));
    assert_eq!(
        iron_golem_model_root_transform(&still),
        entity_model_root_transform(&still),
        "speed below 0.01 skips the renderer wobble"
    );

    let walking = base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let triangle_wave = ((6.0_f32 % 13.0 - 6.5).abs() - 3.25) / 3.25;
    let expected = Mat4::from_translation(Vec3::from_array(position))
        * Mat4::from_rotation_y(std::f32::consts::PI)
        * Mat4::from_rotation_z((6.5 * triangle_wave).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.501, 0.0));
    let actual = iron_golem_model_root_transform(&walking);

    assert_ne!(
        actual,
        entity_model_root_transform(&walking),
        "walking golems use a renderer root distinct from generic living entities"
    );
    assert_close_transform(actual, expected);
}

#[test]
fn iron_golem_textured_submissions_apply_body_wobble_to_base_and_cracks() {
    let (atlas, _) = build_entity_model_texture_atlas(&iron_golem_submission_texture_images())
        .expect("iron golem submission atlas");
    let walking = EntityModelInstance::new(
        76,
        EntityModelKind::IronGolem {
            crackiness: IronGolemCrackiness::High,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    })
    .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));

    let meshes = entity_model_textured_meshes(&[walking], &atlas);
    assert_golem_textured_output(&meshes);

    assert_eq!(meshes.cutout.vertices.len(), 384);

    let expected_transform = iron_golem_model_root_transform(&walking);
    assert_ne!(
        expected_transform,
        entity_model_root_transform(&walking),
        "the submission transform includes the renderer body wobble"
    );

    assert!(meshes.cutout.vertices[..192]
        .iter()
        .all(|vertex| vertex.light == walking.render_state.shader_light()
            && vertex.overlay == walking.render_state.overlay_coords()));
    assert!(meshes.cutout.vertices[192..]
        .iter()
        .all(|vertex| vertex.light == walking.render_state.shader_light()
            && vertex.overlay == [0.0, walking.render_state.overlay_coords()[1]]));
}

#[test]
fn snow_golem_textured_layer_pass_matches_vanilla_renderer_model_layer() {
    let passes = snow_golem_textured_layer_passes();

    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].texture, SNOW_GOLEM_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
}

#[test]
fn golem_texture_atlas_stitches_official_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&golem_texture_images()).unwrap();

    assert_eq!(layout.width, 128);
    assert_eq!(layout.height, 192);
    assert_eq!(layout.entries.len(), 2);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/iron_golem/iron_golem.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 128.0 / 192.0]);
    assert_eq!(
        layout.entries[1].texture.path,
        "textures/entity/snow_golem/snow_golem.png"
    );
    assert_close2(layout.entries[1].uv.min, [0.0, 128.0 / 192.0]);
    assert_close2(layout.entries[1].uv.max, [0.5, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let snow_start = rgba_offset(layout.width, 128, 0, "snow golem atlas row").unwrap();
    assert_eq!(&rgba[snow_start..snow_start + 4], &[1; 4]);
}

#[test]
fn golem_textured_meshes_use_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&golem_texture_images()).unwrap();

    let iron_instance = EntityModelInstance::iron_golem(70, [0.0, 64.0, 0.0], 0.0);
    let iron_meshes = entity_model_textured_meshes(&[iron_instance], &atlas);
    assert_golem_textured_output(&iron_meshes);
    let iron = &iron_meshes.cutout;
    assert_eq!(iron.cutout_faces, 48);
    assert_eq!(iron.vertices.len(), 192);
    assert_eq!(iron.indices.len(), 288);
    assert_close2(iron.vertices[0].uv, [16.0 / 128.0, 0.0]);
    assert!(iron
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (iron_min, iron_max) = textured_mesh_extents(iron);
    assert_close3(iron_min, [-0.8125, 64.001, -0.3125]);
    assert_close3(iron_max, [0.8125, 66.6885, 0.59375]);

    let snow_instance = EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0);
    let snow_meshes = entity_model_textured_meshes(&[snow_instance], &atlas);
    assert_golem_textured_output(&snow_meshes);
    let snow = &snow_meshes.cutout;
    assert_eq!(snow.cutout_faces, 30);
    assert_eq!(snow.vertices.len(), 120);
    assert_eq!(snow.indices.len(), 180);
    assert_close2(snow.vertices[0].uv, [16.0 / 128.0, 128.0 / 192.0]);
    assert!(snow
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (snow_min, snow_max) = textured_mesh_extents(snow);
    assert_close3(snow_min, [-0.6407774, 64.03225, -0.34375]);
    assert_close3(snow_max, [0.6407774, 65.71975, 0.34375]);
}

#[test]
fn snow_golem_textured_mesh_twists_upper_body_and_orbits_arms() {
    // The real (texture-backed) snow golem render path applies the same twist and orbit.
    let (atlas, _) = build_entity_model_texture_atlas(&golem_texture_images()).unwrap();
    let base = EntityModelInstance::snow_golem(123, [0.0, 64.0, 0.0], 0.0);
    let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (60.0, 0.0));
    let rest = entity_model_textured_meshes(&[base], &atlas);
    let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
    assert_golem_textured_output(&rest);
    assert_golem_textured_output(&yawed);
    assert_eq!(rest.cutout.vertices.len(), 120);
    assert_eq!(rest.cutout.vertices.len(), yawed.cutout.vertices.len());
    assert_ne!(
        rest.cutout.vertices[24..72],
        yawed.cutout.vertices[24..72],
        "arms orbit"
    );
    assert_ne!(
        rest.cutout.vertices[72..96],
        yawed.cutout.vertices[72..96],
        "upper body twists"
    );
    assert_eq!(
        rest.cutout.vertices[96..120],
        yawed.cutout.vertices[96..120],
        "lower body stays put"
    );
}

#[test]
fn snow_golem_arm_pose_matches_vanilla_orbit_formula() {
    // Vanilla SnowGolemModel.setupAnim: upperBody.yRot = headYaw * π/180 * 0.25; then
    //   leftArm.yRot = upperBodyYRot;  leftArm.x = cos(upperBodyYRot)*5;  leftArm.z =
    //     -sin(upperBodyYRot)*5;
    //   rightArm.yRot = upperBodyYRot + π;  rightArm.x = -cos*5;  rightArm.z = sin*5.
    // The arm y offset and the drooping zRot (±1.0) are preserved; x/z are overwritten
    // even at rest, so a forward-facing snow golem pulls both arms to z = 0.
    let left_base = layer_pose(ModelLayer::SNOW_GOLEM, "left_arm");
    let right_base = layer_pose(ModelLayer::SNOW_GOLEM, "right_arm");

    // Rest (yaw 0): the orbit collapses the arms to z = 0 but keeps their droop.
    assert!((snow_golem_upper_body_yrot(0.0)).abs() < 1e-6);
    let left_rest = snow_golem_arm_pose(left_base, 0.0, false);
    let right_rest = snow_golem_arm_pose(right_base, 0.0, true);
    assert_close3(left_rest.offset, [5.0, 6.0, 0.0]);
    assert_close3(right_rest.offset, [-5.0, 6.0, 0.0]);
    assert_eq!(left_rest.rotation[1], 0.0);
    assert!((right_rest.rotation[1] - std::f32::consts::PI).abs() < 1e-6);
    assert_eq!(left_rest.rotation[2], left_base.rotation[2]); // 1.0 droop preserved
    assert_eq!(right_rest.rotation[2], right_base.rotation[2]); // -1.0 droop preserved

    // A general head yaw: upper body twists by a quarter, arms orbit by cos/sin.
    let head_yaw_deg = 80.0_f32;
    let upper = snow_golem_upper_body_yrot(head_yaw_deg);
    assert!((upper - head_yaw_deg.to_radians() * 0.25).abs() < 1e-6);
    let (sin, cos) = upper.sin_cos();
    let left = snow_golem_arm_pose(left_base, upper, false);
    let right = snow_golem_arm_pose(right_base, upper, true);
    assert_close3(left.offset, [cos * 5.0, 6.0, -sin * 5.0]);
    assert!((left.rotation[1] - upper).abs() < 1e-6);
    assert_close3(right.offset, [-cos * 5.0, 6.0, sin * 5.0]);
    assert!((right.rotation[1] - (upper + std::f32::consts::PI)).abs() < 1e-6);

    // The upper-body twist sets only yRot; offset, xRot, zRot are preserved.
    let upper_base = layer_pose(ModelLayer::SNOW_GOLEM, "upper_body");
    let twisted = snow_golem_upper_body_pose(upper_base, upper);
    assert_eq!(twisted.offset, upper_base.offset);
    assert_eq!(twisted.rotation[0], upper_base.rotation[0]);
    assert!((twisted.rotation[1] - upper).abs() < 1e-6);
    assert_eq!(twisted.rotation[2], upper_base.rotation[2]);
}

#[test]
fn golem_textured_meshes_apply_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&golem_texture_images()).unwrap();
    for base in [
        EntityModelInstance::iron_golem(71, [0.0, 64.0, 0.0], 0.0),
        EntityModelInstance::snow_golem(122, [0.0, 64.0, 0.0], 0.0),
    ] {
        let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (45.0, 0.0));
        let pitched_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
        let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
        assert_golem_textured_output(&resting);
        assert_golem_textured_output(&yawed);
        assert_golem_textured_output(&pitched);
        assert_eq!(resting.cutout.vertices.len(), yawed.cutout.vertices.len());
        assert_ne!(
            resting.cutout.vertices, yawed.cutout.vertices,
            "{:?}",
            base.kind
        );
        assert_ne!(
            yawed.cutout.vertices, pitched.cutout.vertices,
            "{:?}",
            base.kind
        );
    }
}

#[test]
fn iron_golem_walk_pose_matches_vanilla_triangle_wave() {
    // Vanilla IronGolemModel.setupAnim drives the limbs by Mth.triangleWave(pos, 13).
    // At pos = 0: triangleWave = (|0 - 6.5| - 3.25) / 3.25 = 1.0. With speed = 1:
    // rightLeg = -1.5, leftLeg = 1.5, rightArm = (-0.2 + 1.5) = 1.3, leftArm =
    // (-0.2 - 1.5) = -1.7.
    let right_leg = iron_golem_walk_pose(
        layer_pose(ModelLayer::IRON_GOLEM, "right_leg"),
        0.0,
        1.0,
        IronGolemWalkPart::RightLeg,
    );
    let left_leg = iron_golem_walk_pose(
        layer_pose(ModelLayer::IRON_GOLEM, "left_leg"),
        0.0,
        1.0,
        IronGolemWalkPart::LeftLeg,
    );
    let right_arm = iron_golem_walk_pose(
        layer_pose(ModelLayer::IRON_GOLEM, "right_arm"),
        0.0,
        1.0,
        IronGolemWalkPart::RightArm,
    );
    let left_arm = iron_golem_walk_pose(
        layer_pose(ModelLayer::IRON_GOLEM, "right_arm"),
        0.0,
        1.0,
        IronGolemWalkPart::LeftArm,
    );
    assert!(
        (right_leg.rotation[0] + 1.5).abs() < 1e-6,
        "{}",
        right_leg.rotation[0]
    );
    assert!(
        (left_leg.rotation[0] - 1.5).abs() < 1e-6,
        "{}",
        left_leg.rotation[0]
    );
    assert!(
        (right_arm.rotation[0] - 1.3).abs() < 1e-6,
        "{}",
        right_arm.rotation[0]
    );
    assert!(
        (left_arm.rotation[0] + 1.7).abs() < 1e-6,
        "{}",
        left_arm.rotation[0]
    );

    // At pos = 6.5: triangleWave = (|6.5 - 6.5| - 3.25) / 3.25 = -1.0, scaled by speed.
    let right_leg = iron_golem_walk_pose(
        layer_pose(ModelLayer::IRON_GOLEM, "right_leg"),
        6.5,
        0.5,
        IronGolemWalkPart::RightLeg,
    );
    assert!(
        (right_leg.rotation[0] - (-1.5 * -1.0 * 0.5)).abs() < 1e-6,
        "{}",
        right_leg.rotation[0]
    );
}

#[test]
fn iron_golem_textured_mesh_swings_its_limbs_when_walking() {
    // The real iron golem render path (texture-backed) swings the same limbs. A
    // standing golem is byte-identical however far the swing has advanced; a walking
    // one lifts its feet.
    let (atlas, _) = build_entity_model_texture_atlas(&golem_texture_images()).unwrap();
    let base = EntityModelInstance::iron_golem(271, [0.0, 64.0, 0.0], 0.0);
    let still_instance = base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
    let walking_instance =
        base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let still = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_golem_textured_output(&resting);
    assert_golem_textured_output(&still);
    assert_golem_textured_output(&walking);

    assert_eq!(
        resting.cutout.vertices, still.cutout.vertices,
        "a standing iron golem is inert"
    );
    assert_eq!(
        resting.cutout.vertices.len(),
        walking.cutout.vertices.len(),
        "limb swing keeps the vertex count"
    );
    assert_ne!(
        resting.cutout.vertices, walking.cutout.vertices,
        "a walking iron golem differs"
    );

    let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
    let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
    assert!(
        (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
        "a walking iron golem's feet should lift off the ground"
    );
}

fn assert_golem_textured_output(meshes: &EntityModelTexturedMeshes) {
    assert_golem_folded_meshes_are_cutout_only(meshes);
}

fn assert_golem_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
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

fn golem_texture_images() -> Vec<EntityModelTextureImage> {
    [IRON_GOLEM_TEXTURE_REF, SNOW_GOLEM_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

// Vanilla `Mth.triangleWave(index, period)` (replicated for the test expectations).
fn triangle_wave(index: f32, period: f32) -> f32 {
    ((index % period - period * 0.5).abs() - period * 0.25) / (period * 0.25)
}

fn iron_golem_submission_texture_images() -> Vec<EntityModelTextureImage> {
    [
        IRON_GOLEM_TEXTURE_REF,
        IRON_GOLEM_CRACKINESS_HIGH_TEXTURE_REF,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(texture, vec![index as u8; len])
    })
    .collect()
}

fn assert_close_transform(actual: Mat4, expected: Mat4) {
    for vector in [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z] {
        assert_close3(
            actual.transform_point3(vector).to_array(),
            expected.transform_point3(vector).to_array(),
        );
    }
}

#[test]
fn attacking_iron_golem_raises_both_fists() {
    // Vanilla `IronGolemModel.setupAnim`: while `attackTicksRemaining > 0` both arms raise into the
    // smash (`xRot = -2 + 1.5·triangleWave(tick, 10)`), overriding the walk swing; the legs keep it.
    let attack = 8.0_f32;
    let walking = EntityModelInstance::iron_golem(73, [0.0, 64.0, 0.0], 0.0)
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (3.0, 1.0));
    let attacking = walking.with(|s| s.kind.iron_golem_mut().attack_ticks_remaining = attack);
    let mut model = iron_golem_model(&attacking);
    let expected = -2.0 + 1.5 * triangle_wave(attack, 10.0);
    let right_arm = model.child_mut("right_arm").pose.rotation[0];
    let left_arm = model.child_mut("left_arm").pose.rotation[0];
    assert!(
        (right_arm - expected).abs() < 1.0e-6,
        "right fist raises: {right_arm}"
    );
    assert!(
        (left_arm - expected).abs() < 1.0e-6,
        "left fist raises: {left_arm}"
    );

    // The legs still walk-swing (the smash only overrides the arms): they differ from a standing golem.
    let standing = {
        let mut m = iron_golem_model(&EntityModelInstance::iron_golem(73, [0.0, 64.0, 0.0], 0.0));
        m.child_mut("right_leg").pose.rotation[0]
    };
    let attacking_leg = model.child_mut("right_leg").pose.rotation[0];
    assert_ne!(
        attacking_leg, standing,
        "the legs keep walking during the smash"
    );
}

#[test]
fn offering_iron_golem_holds_out_a_poppy() {
    // Vanilla `IronGolemModel.setupAnim`: while `offerFlowerTick > 0` (and not attacking) the right arm
    // holds the poppy out (`xRot = -0.8 + 0.025·triangleWave(tick, 70)`) and the left arm drops flat.
    let offer = 200;
    let base = EntityModelInstance::iron_golem(74, [0.0, 64.0, 0.0], 0.0)
        .with(|s| s.kind.iron_golem_mut().offer_flower_tick = offer);
    let mut model = iron_golem_model(&base);

    let right_arm = model.child_mut("right_arm").pose.rotation[0];
    let left_arm = model.child_mut("left_arm").pose.rotation[0];
    assert!(
        (right_arm - (-0.8 + 0.025 * triangle_wave(offer as f32, 70.0))).abs() < 1.0e-6,
        "the right arm holds out the poppy: {right_arm}"
    );
    assert_eq!(left_arm, 0.0, "the left arm drops flat");

    // Attack takes priority over the offer pose.
    let mut both =
        iron_golem_model(&base.with(|s| s.kind.iron_golem_mut().attack_ticks_remaining = 5.0));
    let both_left = both.child_mut("left_arm").pose.rotation[0];
    let both_right = both.child_mut("right_arm").pose.rotation[0];
    assert!(
        (both_left - both_right).abs() < 1.0e-6,
        "the attack raises both arms equally, overriding the offer"
    );

    // A resting golem with neither timer keeps its bind arms (no override).
    let mut resting = iron_golem_model(&EntityModelInstance::iron_golem(74, [0.0, 64.0, 0.0], 0.0));
    assert_eq!(resting.child_mut("right_arm").pose.rotation[0], 0.0);
}
