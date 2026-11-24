use super::*;
use crate::entity_models::model_layers::feline_model;

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "expected {actual} to be close to {expected}"
    );
}

fn assert_close_transform(actual: Mat4, expected: Mat4) {
    for (actual, expected) in actual
        .to_cols_array()
        .into_iter()
        .zip(expected.to_cols_array())
    {
        assert!(
            (actual - expected).abs() < 1.0e-5,
            "expected {expected}, got {actual}"
        );
    }
}

fn vanilla_rot_lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}

#[test]
fn feline_standing_drops_the_lower_tail() {
    // Vanilla `AdultFelineModel.setupAnim` sets `tail2.xRot = 1.7278761` while not sitting (the base the
    // projected walk wobble adds onto), a real change from the `0` bind rotation; the bind-0.9 `tail1`
    // is left alone at rest.
    let mut model = feline_model(
        ModelLayer::OCELOT,
        false,
        &EntityModelInstance::feline(
            503,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            CatModelVariant::Black,
            None,
        ),
    );
    assert!((model.child_mut("tail2").pose.rotation[0] - 1.7278761).abs() < 1.0e-6);
    assert_eq!(model.child_mut("tail1").pose.rotation[0], 0.9);
}

#[test]
fn feline_lower_tail_wobbles_with_the_gait() {
    // Vanilla `AdultFelineModel.setupAnim` (not-sitting, not-sprinting, not-crouching) sets
    // `tail2.xRot = 1.7278761 + (π/4)·cos(walkAnimationPos)·walkAnimationSpeed`. At rest the wobble
    // term is zero, so `tail2.xRot` holds at the standing droop; walking shifts it off the droop.
    // At rest (zero speed) the wobble collapses to the bare droop.
    let mut model = feline_model(
        ModelLayer::OCELOT,
        false,
        &EntityModelInstance::feline(
            530,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            CatModelVariant::Black,
            None,
        ),
    );
    let rest_tail2 = model.child_mut("tail2").pose.rotation[0];
    assert!((rest_tail2 - 1.7278761).abs() < 1.0e-6);

    // Walking adds `(π/4)·cos(pos)·speed` onto the droop.
    let pos = 3.0_f32;
    let speed = 0.8_f32;
    drop(model);
    let mut model = feline_model(
        ModelLayer::OCELOT,
        false,
        &EntityModelInstance::feline(
            531,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            CatModelVariant::Black,
            None,
        )
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (pos, speed)),
    );
    let walk_tail2 = model.child_mut("tail2").pose.rotation[0];
    let expected = 1.7278761 + std::f32::consts::FRAC_PI_4 * pos.cos() * speed;
    assert!((walk_tail2 - expected).abs() < 1.0e-6);
    assert_ne!(
        rest_tail2, walk_tail2,
        "the lower tail wobbles while moving"
    );

    // The wobble advances with the walk position.
    drop(model);
    let mut model = feline_model(
        ModelLayer::OCELOT,
        false,
        &EntityModelInstance::feline(
            532,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            CatModelVariant::Black,
            None,
        )
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (6.0, speed)),
    );
    let walk_later_tail2 = model.child_mut("tail2").pose.rotation[0];
    assert_ne!(walk_tail2, walk_later_tail2);

    // Zero speed at a nonzero position is still a no-op (back at the droop).
    drop(model);
    let mut model = feline_model(
        ModelLayer::OCELOT,
        false,
        &EntityModelInstance::feline(
            533,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            CatModelVariant::Black,
            None,
        )
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (3.0, 0.0)),
    );
    let still_tail2 = model.child_mut("tail2").pose.rotation[0];
    assert!((still_tail2 - 1.7278761).abs() < 1.0e-6);
}

#[test]
fn feline_crouch_pose_matches_vanilla_branch() {
    // Vanilla `AdultFelineModel.setupAnim` crouch branch moves body/head/tail pivots first, then the
    // non-sprinting not-sitting branch overwrites `tail2.xRot` with the crouch wobble amplitude.
    let pos = 3.0_f32;
    let speed = 0.8_f32;
    let mut model = feline_model(
        ModelLayer::OCELOT,
        false,
        &EntityModelInstance::feline(
            534,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            CatModelVariant::Black,
            None,
        )
        .with(|s| s.kind.feline_mut().is_crouching = true)
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (pos, speed)),
    );

    assert_close3(model.child_mut("body").pose.offset, [0.0, 13.0, -10.0]);
    assert_close3(model.child_mut("head").pose.offset, [0.0, 17.0, -9.0]);
    assert_close3(model.child_mut("tail1").pose.offset, [0.0, 16.0, 8.0]);
    assert_close(
        model.child_mut("tail1").pose.rotation[0],
        std::f32::consts::FRAC_PI_2,
    );
    assert_close3(model.child_mut("tail2").pose.offset, [0.0, 16.0, 16.0]);
    assert_close(
        model.child_mut("tail2").pose.rotation[0],
        1.7278761 + 0.47123894 * pos.cos() * speed,
    );

    let phase = pos * 0.6662;
    assert_close(
        model.child_mut("left_hind_leg").pose.rotation[0],
        phase.cos() * speed,
    );
    assert_close(
        model.child_mut("right_front_leg").pose.rotation[0],
        phase.cos() * speed,
    );
    assert_close(
        model.child_mut("right_hind_leg").pose.rotation[0],
        (phase + std::f32::consts::PI).cos() * speed,
    );
    assert_close(
        model.child_mut("left_front_leg").pose.rotation[0],
        (phase + std::f32::consts::PI).cos() * speed,
    );
}

#[test]
fn feline_sprint_pose_matches_vanilla_branch() {
    // Vanilla sprinting puts `tail2` on `tail1.y`, advances the lower tail forward, uses the sprint
    // tail amplitude (`π/10`), and offsets the right-hind/left-front leg phases by `0.3`.
    let pos = 3.0_f32;
    let speed = 0.8_f32;
    let mut model = feline_model(
        ModelLayer::OCELOT,
        false,
        &EntityModelInstance::feline(
            535,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            CatModelVariant::Black,
            None,
        )
        .with(|s| s.kind.feline_mut().is_sprinting = true)
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (pos, speed)),
    );

    assert_close3(model.child_mut("tail1").pose.offset, [0.0, 15.0, 8.0]);
    assert_close(
        model.child_mut("tail1").pose.rotation[0],
        std::f32::consts::FRAC_PI_2,
    );
    assert_close3(model.child_mut("tail2").pose.offset, [0.0, 15.0, 16.0]);
    assert_close(
        model.child_mut("tail2").pose.rotation[0],
        1.7278761 + (std::f32::consts::PI / 10.0) * pos.cos() * speed,
    );

    let phase = pos * 0.6662;
    assert_close(
        model.child_mut("left_hind_leg").pose.rotation[0],
        phase.cos() * speed,
    );
    assert_close(
        model.child_mut("right_hind_leg").pose.rotation[0],
        (phase + 0.3).cos() * speed,
    );
    assert_close(
        model.child_mut("left_front_leg").pose.rotation[0],
        (phase + std::f32::consts::PI + 0.3).cos() * speed,
    );
    assert_close(
        model.child_mut("right_front_leg").pose.rotation[0],
        (phase + std::f32::consts::PI).cos() * speed,
    );
}

#[test]
fn adult_feline_sitting_pose_matches_vanilla_branch() {
    // Vanilla `AdultFelineModel.setupAnim`: after head look and crouch/sprint setup, `isSitting` skips
    // the not-sitting leg swing/tail wobble branch and folds the adult body, tail, and legs.
    let mut model = feline_model(
        ModelLayer::OCELOT,
        false,
        &EntityModelInstance::feline(
            538,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            false,
            CatModelVariant::Black,
            None,
        )
        .with(|s| s.kind.feline_mut().is_sitting = true)
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (3.0, 0.8)),
    );

    assert_close3(model.child_mut("body").pose.offset, [0.0, 8.0, -5.0]);
    assert_close(
        model.child_mut("body").pose.rotation[0],
        std::f32::consts::FRAC_PI_4,
    );
    assert_close3(model.child_mut("head").pose.offset, [0.0, 11.7, -8.0]);
    assert_close3(model.child_mut("tail1").pose.offset, [0.0, 23.0, 6.0]);
    assert_close(
        model.child_mut("tail1").pose.rotation[0],
        FELINE_TAIL2_REST_X_ROT,
    );
    assert_close3(model.child_mut("tail2").pose.offset, [0.0, 22.0, 13.2]);
    assert_close(
        model.child_mut("tail2").pose.rotation[0],
        FELINE_SITTING_TAIL2_X_ROT,
    );

    for name in ["left_front_leg", "right_front_leg"] {
        let part = model.child_mut(name);
        assert_close(part.pose.rotation[0], -std::f32::consts::PI / 20.0);
        assert_close(part.pose.offset[1], 16.1);
        assert_close(part.pose.offset[2], -7.0);
    }
    for name in ["left_hind_leg", "right_hind_leg"] {
        let part = model.child_mut(name);
        assert_close(part.pose.rotation[0], -std::f32::consts::FRAC_PI_2);
        assert_close(part.pose.offset[1], 21.0);
        assert_close(part.pose.offset[2], 1.0);
    }
}

#[test]
fn baby_feline_sitting_pose_matches_vanilla_branch() {
    // Vanilla `BabyFelineModel.setupAnim` sitting branch applies a smaller unscaled delta and still
    // skips the walk leg swing.
    let mut model = feline_model(
        ModelLayer::OCELOT_BABY,
        true,
        &EntityModelInstance::feline(
            539,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            true,
            CatModelVariant::Black,
            None,
        )
        .with(|s| s.kind.feline_mut().is_sitting = true)
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (3.0, 0.8)),
    );

    assert_close3(model.child_mut("body").pose.offset, [0.0, 21.75, 0.5]);
    assert_close(
        model.child_mut("body").pose.rotation[0],
        BABY_FELINE_SITTING_BODY_X_ROT_DELTA,
    );
    assert_close3(model.child_mut("head").pose.offset, [0.0, 20.0, -2.375]);
    assert_close3(model.child_mut("tail1").pose.offset, [0.0, 23.107, 3.0151]);
    assert_close(
        model.child_mut("tail1").pose.rotation[0],
        -0.567232 + BABY_FELINE_SITTING_TAIL1_X_ROT_DELTA,
    );
    assert_close(model.child_mut("left_hind_leg").pose.offset[2], 1.6);
    assert_close(model.child_mut("right_hind_leg").pose.offset[2], 1.6);
    for name in [
        "left_front_leg",
        "right_front_leg",
        "left_hind_leg",
        "right_hind_leg",
    ] {
        assert_close(model.child_mut(name).pose.rotation[0], 0.0);
    }
}

#[test]
fn adult_feline_lie_down_and_relax_pose_matches_vanilla_branch() {
    // Vanilla applies the sitting branch first when set, then still layers the cat-only lie-down and
    // relax branches. This pins that order so the model does not return early after sitting.
    let lie = 0.5;
    let tail = 0.25;
    let relax = 0.5;
    let mut model = feline_model(
        ModelLayer::OCELOT,
        false,
        &EntityModelInstance::feline(
            541,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            false,
            CatModelVariant::Black,
            None,
        )
        .with(|s| {
            s.kind.feline_mut().is_sitting = true;
            s.kind.feline_mut().lie_down_amount = lie;
            s.kind.feline_mut().lie_down_amount_tail = tail;
            s.kind.feline_mut().relax_state_one_amount = relax;
        }),
    );

    assert_close3(model.child_mut("head").pose.offset, [0.0, 11.7, -8.0]);
    assert_close(
        model.child_mut("head").pose.rotation[0],
        vanilla_rot_lerp(relax, 0.0, FELINE_RELAX_HEAD_X_ROT),
    );
    assert_close(
        model.child_mut("head").pose.rotation[1],
        vanilla_rot_lerp(lie, 0.0, ADULT_FELINE_LIE_HEAD_Y_ROT),
    );
    assert_close(
        model.child_mut("head").pose.rotation[2],
        vanilla_rot_lerp(lie, 0.0, ADULT_FELINE_LIE_HEAD_Z_ROT),
    );

    assert_close(
        model.child_mut("left_front_leg").pose.rotation[0],
        ADULT_FELINE_LIE_LEFT_FRONT_LEG_X_ROT,
    );
    {
        let part = model.child_mut("right_front_leg");
        assert_close3(part.pose.offset, [-0.2, 16.1, -7.0]);
        assert_close(
            part.pose.rotation[0],
            ADULT_FELINE_LIE_RIGHT_FRONT_LEG_X_ROT,
        );
        assert_close(part.pose.rotation[2], -0.2);
    }
    {
        let part = model.child_mut("left_hind_leg");
        assert_close3(part.pose.offset, [1.1, 21.0, 1.0]);
        assert_close(part.pose.rotation[0], -0.4);
    }
    {
        let part = model.child_mut("right_hind_leg");
        assert_close3(part.pose.offset, [-0.3, 23.0, 1.0]);
        assert_close(part.pose.rotation[0], 0.5);
        assert_close(part.pose.rotation[2], -0.5);
    }
    assert_close(
        model.child_mut("tail1").pose.rotation[0],
        vanilla_rot_lerp(tail, FELINE_TAIL2_REST_X_ROT, 0.8),
    );
    assert_close(
        model.child_mut("tail2").pose.rotation[0],
        vanilla_rot_lerp(tail, FELINE_SITTING_TAIL2_X_ROT, -0.4),
    );
}

#[test]
fn baby_feline_lie_down_and_relax_pose_matches_vanilla_branch() {
    let lie = 0.5;
    let tail = 0.25;
    let relax = 0.5;
    let mut model = feline_model(
        ModelLayer::OCELOT_BABY,
        true,
        &EntityModelInstance::feline(
            542,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            true,
            CatModelVariant::Black,
            None,
        )
        .with(|s| {
            s.kind.feline_mut().lie_down_amount = lie;
            s.kind.feline_mut().lie_down_amount_tail = tail;
            s.kind.feline_mut().relax_state_one_amount = relax;
        }),
    );

    let head_lie_x = vanilla_rot_lerp(lie, 0.0, std::f32::consts::PI / 18.0);
    assert_close3(model.child_mut("body").pose.offset, [1.0, 20.5, 0.5]);
    assert_close3(model.child_mut("head").pose.offset, [1.5, 20.75, -3.625]);
    assert_close(
        model.child_mut("head").pose.rotation[0],
        vanilla_rot_lerp(relax, head_lie_x, FELINE_RELAX_HEAD_X_ROT),
    );
    assert_close(
        model.child_mut("head").pose.rotation[2],
        vanilla_rot_lerp(lie, 0.0, -std::f32::consts::PI * 5.0 / 12.0),
    );
    {
        let part = model.child_mut("right_front_leg");
        assert_close3(part.pose.offset, [2.5, 21.5, -1.5]);
        assert_close(part.pose.rotation[0], -std::f32::consts::FRAC_PI_4);
    }
    {
        let part = model.child_mut("left_front_leg");
        assert_close3(part.pose.offset, [2.5, 21.0, -3.5]);
        assert_close(part.pose.rotation[0], -std::f32::consts::FRAC_PI_2);
    }
    {
        let part = model.child_mut("right_hind_leg");
        assert_close3(part.pose.offset, [1.5, 21.75, 3.0]);
        assert_close(part.pose.rotation[0], std::f32::consts::PI * 2.0 / 9.0);
        assert_close(part.pose.rotation[1], std::f32::consts::PI / 9.0);
        assert_close(part.pose.rotation[2], -std::f32::consts::PI / 9.0);
    }
    assert_close3(
        model.child_mut("left_hind_leg").pose.offset,
        [2.5, 22.0, 1.5],
    );

    let tail1_base = -0.567232;
    assert_close3(model.child_mut("tail1").pose.offset, [1.0, 19.607, 3.6651]);
    assert_close(
        model.child_mut("tail1").pose.rotation[0],
        tail1_base + vanilla_rot_lerp(tail, tail1_base, -std::f32::consts::PI / 6.0),
    );
    assert_close(
        model.child_mut("tail1").pose.rotation[2],
        vanilla_rot_lerp(tail, 0.0, -std::f32::consts::PI / 18.0),
    );
}

#[test]
fn feline_lie_down_root_transform_matches_cat_renderer_setup_rotations() {
    // Vanilla `CatRenderer.setupRotations` calls the living setup first, then inserts the lie-down
    // translate and Z roll before the model flip and adult-cat mesh-transformer scale.
    let lie = 0.5;
    let cat = EntityModelInstance::feline(
        543,
        [2.0, 64.0, -3.0],
        30.0,
        true,
        false,
        CatModelVariant::Black,
        None,
    )
    .with(|s| s.kind.feline_mut().lie_down_amount = lie);
    let setup_tail = Mat4::from_translation(Vec3::new(0.4 * lie, 0.15 * lie, 0.1 * lie))
        * Mat4::from_rotation_z((90.0 * lie).to_radians());
    let expected_root = Mat4::from_translation(Vec3::from_array(cat.position))
        * Mat4::from_scale(Vec3::splat(cat.render_state.scale))
        * Mat4::from_rotation_y((180.0 - cat.render_state.body_rot).to_radians())
        * setup_tail
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.501, 0.0));

    assert_close_transform(feline_model_root_transform(&cat), expected_root);
    assert_ne!(
        feline_model_root_transform(&cat),
        entity_model_root_transform(&cat),
        "lie-down setupRotations changes the whole-model root before the cat scale"
    );

    let on_sleeping_player =
        cat.with(|s| s.kind.feline_mut().is_lying_on_top_of_sleeping_player = true);
    let expected_on_sleeping_player_root = Mat4::from_translation(Vec3::from_array(cat.position))
        * Mat4::from_scale(Vec3::splat(cat.render_state.scale))
        * Mat4::from_rotation_y((180.0 - cat.render_state.body_rot).to_radians())
        * setup_tail
        * Mat4::from_translation(Vec3::new(0.15 * lie, 0.0, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.501, 0.0));
    assert_close_transform(
        feline_model_root_transform(&on_sleeping_player),
        expected_on_sleeping_player_root,
    );
    assert_ne!(
        feline_model_root_transform(&on_sleeping_player),
        feline_model_root_transform(&cat),
        "sleeping-player cats get the extra post-roll x translate"
    );

    let standing_cat = EntityModelInstance::feline(
        544,
        [2.0, 64.0, -3.0],
        30.0,
        true,
        false,
        CatModelVariant::Black,
        None,
    );
    assert_close_transform(
        feline_model_root_transform(&standing_cat),
        entity_model_root_transform(&standing_cat),
    );

    let baby = EntityModelInstance::feline(
        545,
        [2.0, 64.0, -3.0],
        30.0,
        true,
        true,
        CatModelVariant::Black,
        None,
    )
    .with(|s| s.kind.feline_mut().lie_down_amount = lie);
    let expected_baby_root = Mat4::from_translation(Vec3::from_array(baby.position))
        * Mat4::from_scale(Vec3::splat(baby.render_state.scale))
        * Mat4::from_rotation_y((180.0 - baby.render_state.body_rot).to_radians())
        * setup_tail
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.501, 0.0));
    assert_close_transform(feline_model_root_transform(&baby), expected_baby_root);
    assert_ne!(
        feline_model_root_transform(&baby),
        entity_model_root_transform(&baby),
        "baby cats get the lie-down root transform without the adult 0.8 scale"
    );
}

#[test]
fn feline_textured_mesh_applies_sitting_pose_to_base_and_collar() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/cat/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let standing = EntityModelInstance::feline(
        540,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
        CatModelVariant::Black,
        Some(EntityDyeColor::Lime),
    )
    .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (3.0, 0.8));
    let sitting = standing.with(|s| s.kind.feline_mut().is_sitting = true);
    let lying = standing.with(|s| {
        s.kind.feline_mut().lie_down_amount = 1.0;
        s.kind.feline_mut().lie_down_amount_tail = 1.0;
        s.kind.feline_mut().relax_state_one_amount = 1.0;
        s.kind.feline_mut().is_lying_on_top_of_sleeping_player = true;
    });

    let standing_meshes = entity_model_textured_meshes(&[standing], &atlas);
    let sitting_meshes = entity_model_textured_meshes(&[sitting], &atlas);
    let lying_meshes = entity_model_textured_meshes(&[lying], &atlas);

    assert_eq!(
        sitting_meshes.cutout.vertices.len(),
        standing_meshes.cutout.vertices.len()
    );
    assert_ne!(
        sitting_meshes.cutout.vertices, standing_meshes.cutout.vertices,
        "the shared posed tree drives both base and collar textured geometry"
    );
    assert_ne!(
        lying_meshes.cutout.vertices, standing_meshes.cutout.vertices,
        "the cat lie-down pose also flows through the textured base and collar layers"
    );
}

#[test]
fn baby_feline_crouch_and_sprint_pose_the_visible_tail() {
    // `BabyFelineModel` shares the crouch/sprint setup with the adult, but its lower `tail2` is a
    // cubeless pivot. Pin the visible `tail1` offsets/rotations so the branch still has observable
    // coverage on baby cats and ocelots.
    let mut model = feline_model(
        ModelLayer::OCELOT_BABY,
        true,
        &EntityModelInstance::feline(
            536,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            true,
            CatModelVariant::Black,
            None,
        )
        .with(|s| s.kind.feline_mut().is_crouching = true),
    );
    assert_close3(model.child_mut("tail1").pose.offset, [0.0, 20.107, 3.9151]);
    assert_close(
        model.child_mut("tail1").pose.rotation[0],
        std::f32::consts::FRAC_PI_2,
    );

    drop(model);
    let mut model = feline_model(
        ModelLayer::OCELOT_BABY,
        true,
        &EntityModelInstance::feline(
            537,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            true,
            CatModelVariant::Black,
            None,
        )
        .with(|s| s.kind.feline_mut().is_sprinting = true),
    );
    assert_close3(model.child_mut("tail1").pose.offset, [0.0, 19.107, 3.9151]);
    assert_close(
        model.child_mut("tail1").pose.rotation[0],
        std::f32::consts::FRAC_PI_2,
    );
}

#[test]
fn feline_textured_render_matches_vanilla_renderer() {
    // The cat and ocelot share one model but bind different images: the per-breed `CatVariant`
    // texture for cats (× age), the `ocelot` texture for ocelots (the breed is then ignored).
    let cat_variants = [
        CatModelVariant::Tabby,
        CatModelVariant::Black,
        CatModelVariant::Red,
        CatModelVariant::Siamese,
        CatModelVariant::BritishShorthair,
        CatModelVariant::Calico,
        CatModelVariant::Persian,
        CatModelVariant::Ragdoll,
        CatModelVariant::White,
        CatModelVariant::Jellie,
        CatModelVariant::AllBlack,
    ];
    for cat_variant in cat_variants {
        for baby in [false, true] {
            // The cat binds its per-breed texture; the ocelot ignores the breed.
            let cat_texture = feline_texture_ref(true, baby, cat_variant);
            let cat_passes = feline_textured_layer_passes(true, baby, cat_variant, None);
            assert_eq!(cat_passes.len(), 1);
            assert_eq!(
                cat_passes[0].render_type,
                EntityModelLayerRenderType::EntityCutout
            );
            assert_eq!(cat_passes[0].render_type.vanilla_name(), "entityCutout");
            assert_eq!(cat_passes[0].texture, cat_texture);
            assert_eq!(cat_passes[0].visibility, EntityModelLayerVisibility::All);
            assert_eq!(cat_passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
            assert_eq!(cat_passes[0].order, 0);
            assert!(entity_model_texture_refs().contains(&cat_texture));

            let ocelot_texture = feline_texture_ref(false, baby, cat_variant);
            assert_eq!(
                ocelot_texture,
                if baby {
                    FELINE_OCELOT_BABY_TEXTURE_REF
                } else {
                    FELINE_OCELOT_TEXTURE_REF
                }
            );
            let ocelot_passes = feline_textured_layer_passes(false, baby, cat_variant, None);
            assert_eq!(ocelot_passes.len(), 1);
            assert_eq!(
                ocelot_passes[0].render_type,
                EntityModelLayerRenderType::EntityCutout
            );
            assert_eq!(ocelot_passes[0].render_type.vanilla_name(), "entityCutout");
            assert_eq!(ocelot_passes[0].texture, ocelot_texture);
            assert_eq!(ocelot_passes[0].visibility, EntityModelLayerVisibility::All);
            assert_eq!(ocelot_passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
            assert_eq!(ocelot_passes[0].order, 0);
        }
    }

    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/cat/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    for (cat, baby) in [(true, false), (true, true), (false, false), (false, true)] {
        let instance = EntityModelInstance::feline(
            900,
            [0.0, 64.0, 0.0],
            0.0,
            cat,
            baby,
            CatModelVariant::Black,
            None,
        )
        .with(|s| {
            s.light_coords = (4_u32 << 4) | (12_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
        let meshes = entity_model_textured_meshes(&[instance], &atlas);
        assert!(meshes.translucent.vertices.is_empty());
        assert!(meshes.eyes.vertices.is_empty());

        let mesh = &meshes.cutout;

        assert!(
            !mesh.vertices.is_empty(),
            "cat={cat} baby={baby} emits textured geometry"
        );
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
                && vertex.light == instance.render_state.shader_light()
                && vertex.overlay == instance.render_state.overlay_coords()));
    }

    let collared = EntityModelInstance::feline(
        901,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
        CatModelVariant::Black,
        Some(EntityDyeColor::Lime),
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (13_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[collared], &atlas);

    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == collared.render_state.shader_light()));
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == collared.render_state.overlay_coords()));
    let collar_overlay = [0.0, collared.render_state.overlay_coords()[1]];
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == collar_overlay));
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.overlay
        == collared.render_state.overlay_coords()
        || vertex.overlay == collar_overlay));
}

#[test]
fn feline_collar_missing_texture_preserves_other_geometry() {
    // Vanilla `CatCollarLayer` calls the shared colored-cutout copy helper with `order(1)`;
    // missing atlas data suppresses only the folded collar geometry.
    let images = [
        feline_texture_ref(true, false, CatModelVariant::Black),
        feline_texture_ref(true, true, CatModelVariant::Black),
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect::<Vec<_>>();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    for (id, baby, collar_texture) in [
        (902, false, FELINE_CAT_COLLAR_TEXTURE_REF),
        (903, true, FELINE_CAT_COLLAR_BABY_TEXTURE_REF),
    ] {
        let bare = EntityModelInstance::feline(
            id,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            baby,
            CatModelVariant::Black,
            None,
        )
        .with(|s| {
            s.light_coords = (5_u32 << 4) | (13_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
        let bare_meshes = entity_model_textured_meshes(&[bare], &atlas);

        let collared = EntityModelInstance::feline(
            id,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            baby,
            CatModelVariant::Black,
            Some(EntityDyeColor::Lime),
        )
        .with(|s| {
            s.light_coords = (5_u32 << 4) | (13_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
        let meshes = entity_model_textured_meshes(&[collared], &atlas);

        assert_eq!(
            meshes.cutout.vertices,
            bare_meshes.cutout.vertices,
            "missing {path} suppresses only folded collar geometry",
            path = collar_texture.path
        );
        assert_eq!(meshes.cutout.indices, bare_meshes.cutout.indices);
        assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
            == collared.render_state.shader_light()
            && vertex.overlay == collared.render_state.overlay_coords()));
        assert!(meshes.translucent.vertices.is_empty());
        assert!(meshes.eyes.vertices.is_empty());
    }
}

#[test]
fn feline_collar_layer_matches_vanilla_cat_collar_layer() {
    // Vanilla `CatCollarLayer`: a tame cat re-renders the mesh with `cat_collar.png` (adult) /
    // `cat_collar_baby.png` (baby), tinted by the dye's diffuse color. With no collar there is just
    // the base body pass.
    assert_eq!(
        feline_textured_layer_passes(true, false, CatModelVariant::Black, None).len(),
        1
    );
    for (baby, collar_texture) in [
        (false, FELINE_CAT_COLLAR_TEXTURE_REF),
        (true, FELINE_CAT_COLLAR_BABY_TEXTURE_REF),
    ] {
        let passes = feline_textured_layer_passes(
            true,
            baby,
            CatModelVariant::Black,
            Some(EntityDyeColor::Red),
        );
        assert_eq!(passes.len(), 2, "the base body plus the collar overlay");
        assert_eq!(passes[0].order, 0);
        assert_eq!(
            passes[1].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[1].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[1].texture, collar_texture);
        assert_eq!(passes[1].visibility, EntityModelLayerVisibility::All);
        assert_eq!(passes[1].tint, EntityDyeColor::Red.texture_diffuse_color());
        assert_eq!(passes[1].order, 1);
        assert!(entity_model_texture_refs().contains(&collar_texture));
    }

    // Each dye tints the collar by its diffuse color.
    let lime = feline_textured_layer_passes(
        true,
        false,
        CatModelVariant::Black,
        Some(EntityDyeColor::Lime),
    );
    assert_eq!(lime[1].tint, EntityDyeColor::Lime.texture_diffuse_color());
}
