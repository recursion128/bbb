use super::*;

#[test]
fn turtle_textured_egg_belly_shows_when_carrying_an_egg() {
    let (atlas, _) = build_entity_model_texture_atlas(&turtle_texture_images()).unwrap();

    let plain = EntityModelInstance::turtle(752, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.on_ground = true;
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let egg = plain.with(|s| s.kind.turtle_mut().has_egg = true);
    let plain_meshes = entity_model_textured_meshes(&[plain], &atlas);
    let egg_meshes = entity_model_textured_meshes(&[egg], &atlas);

    // `AdultTurtleModel.setupAnim` shifts the whole root one model unit (`root.y--`) with an egg.
    let leg = ModelLayer::TURTLE
        .bake()
        .part_vertex_range("right_front_leg")
        .unwrap();
    let plain_leg = plain_meshes.cutout.vertices[leg.start].position;
    let egg_leg = egg_meshes.cutout.vertices[leg.start].position;
    assert_close3([egg_leg[0], egg_leg[1] - 1.0 / 16.0, egg_leg[2]], plain_leg);

    // The egg belly renders into the same cutout mesh: +6 faces / +24 vertices (eight cubes).
    assert_eq!(
        egg_meshes.cutout.cutout_faces,
        plain_meshes.cutout.cutout_faces + 6
    );
    assert_eq!(
        egg_meshes.cutout.vertices.len(),
        plain_meshes.cutout.vertices.len() + 24
    );
    assert_eq!(egg_meshes.cutout.vertices.len(), 192);
    assert!(plain_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == plain.render_state.shader_light()
            && vertex.overlay == plain.render_state.overlay_coords()));
    assert!(egg_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == egg.render_state.shader_light()
            && vertex.overlay == egg.render_state.overlay_coords()));
    assert!(egg_meshes.translucent.vertices.is_empty());
    assert!(egg_meshes.eyes.vertices.is_empty());
}

#[test]
fn turtle_setup_anim_curves_match_vanilla() {
    let (pos, speed) = (3.0_f32, 0.7_f32);

    // Quadruped base swing: diagonal phase (right-hind / left-front at phase 0, the other two π).
    let expected_phase0 = (pos * 0.6662).cos() * 1.4 * speed;
    let expected_phasepi = (pos * 0.6662 + std::f32::consts::PI).cos() * 1.4 * speed;
    assert!((turtle_quadruped_leg_x_rot(pos, speed, false) - expected_phase0).abs() < 1.0e-6);
    assert!((turtle_quadruped_leg_x_rot(pos, speed, true) - expected_phasepi).abs() < 1.0e-6);

    // Land yaw swing (not laying): front weight 8, hind weight 3, right negated.
    let swing = (pos * 5.0).cos();
    assert!(
        (turtle_land_leg_y_rot(pos, speed, true, false, false) - swing * 8.0 * speed).abs()
            < 1.0e-6
    );
    assert!(
        (turtle_land_leg_y_rot(pos, speed, false, true, false) - -(swing * 3.0 * speed)).abs()
            < 1.0e-6
    );

    // Water paddle swing.
    let water = (pos * 0.6662 * 0.6).cos() * 0.5 * speed;
    assert!((turtle_water_swing(pos, speed) - water).abs() < 1.0e-6);

    // Combined per-leg rotation: on land the quadruped xRot remains and the yRot is added; in
    // water the hind xRot is the paddle swing and the front legs add it on zRot.
    let right_hind_land = turtle_leg_rotation(pos, speed, true, false, true, false);
    assert!((right_hind_land[0] - turtle_quadruped_leg_x_rot(pos, speed, false)).abs() < 1.0e-6);
    assert!(
        (right_hind_land[1] - turtle_land_leg_y_rot(pos, speed, false, true, false)).abs() < 1.0e-6
    );
    assert_eq!(right_hind_land[2], 0.0);

    let right_hind_water = turtle_leg_rotation(pos, speed, false, false, true, false);
    assert!((right_hind_water[0] - water).abs() < 1.0e-6);
    assert_eq!(right_hind_water[1], 0.0);
    assert_eq!(right_hind_water[2], 0.0);

    let right_front_water = turtle_leg_rotation(pos, speed, false, true, true, false);
    assert!((right_front_water[0] - turtle_quadruped_leg_x_rot(pos, speed, true)).abs() < 1.0e-6);
    assert!((right_front_water[2] - -water).abs() < 1.0e-6);
}

#[test]
fn turtle_laying_egg_amplifies_front_legs() {
    // Vanilla `TurtleModel.setupAnim` land branch with `isLayingEgg`:
    //   layEgg = 4, layEggAmplitude = 2;
    //   frontSwing = cos(layEgg · pos · 5); rightFront.yRot = -frontSwing · 8 · speed · 2.
    // The hind legs (cos(pos · 5) · 3 · speed) and the water branch are unaffected.
    let (pos, speed) = (3.0_f32, 0.7_f32);

    let front_swing_laying = (4.0 * pos * 5.0).cos();
    let expected_right_front = -front_swing_laying * 8.0 * speed * 2.0;
    let expected_left_front = front_swing_laying * 8.0 * speed * 2.0;
    assert!(
        (turtle_land_leg_y_rot(pos, speed, true, true, true) - expected_right_front).abs() < 1.0e-6
    );
    assert!(
        (turtle_land_leg_y_rot(pos, speed, true, false, true) - expected_left_front).abs() < 1.0e-6
    );

    // Hind legs are identical whether laying or not.
    assert_eq!(
        turtle_land_leg_y_rot(pos, speed, false, true, true),
        turtle_land_leg_y_rot(pos, speed, false, true, false)
    );
    assert_eq!(
        turtle_land_leg_y_rot(pos, speed, false, false, true),
        turtle_land_leg_y_rot(pos, speed, false, false, false)
    );

    // The laying flag only touches the land yaw slot — the quadruped xRot is unchanged, zRot 0.
    let right_front_land = turtle_leg_rotation(pos, speed, true, true, true, true);
    assert!((right_front_land[0] - turtle_quadruped_leg_x_rot(pos, speed, true)).abs() < 1.0e-6);
    assert!((right_front_land[1] - expected_right_front).abs() < 1.0e-6);
    assert_eq!(right_front_land[2], 0.0);

    // The water branch ignores laying entirely (layEgg only applies on land).
    assert_eq!(
        turtle_leg_rotation(pos, speed, false, true, true, true),
        turtle_leg_rotation(pos, speed, false, true, true, false)
    );
}

#[test]
fn turtle_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&turtle_texture_images()).unwrap();

    // Adult renders into the cutout mesh (default `RenderTypes::entityCutout`). Seven cubes →
    // 42 faces / 168 vertices, with nothing on the translucent or eyes passes.
    let adult = EntityModelInstance::turtle(750, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.on_ground = true;
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[adult], &atlas);

    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.cutout.cutout_faces, 42);
    assert_eq!(meshes.cutout.vertices.len(), 168);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == adult.render_state.shader_light()
            && vertex.overlay == adult.render_state.overlay_coords()));

    // Baby is the smaller model and uses vanilla `BabyTurtleModel`'s `entityCutoutCull` render type:
    // six cubes → 36 faces / 144 vertices.
    let baby = EntityModelInstance::turtle(751, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);

    assert_eq!(baby_meshes.cutout_cull.vertices.len(), 144);
    assert!(baby_meshes
        .cutout_cull
        .vertices
        .iter()
        .all(|vertex| vertex.light == baby.render_state.shader_light()
            && vertex.overlay == baby.render_state.overlay_coords()));

    // The head tracks the look, and the land walk differs from the water paddle.
    let looking = entity_model_textured_meshes(
        &[adult.with(|s| (s.head_yaw, s.head_pitch) = (40.0, -20.0))],
        &atlas,
    );
    assert_ne!(meshes.cutout.vertices, looking.cutout.vertices);
    let walking = entity_model_textured_meshes(
        &[adult
            .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (3.0, 0.7))
            .with(|s| {
                s.on_ground = true;
                s.in_water = false;
            })],
        &atlas,
    );
    let swimming = entity_model_textured_meshes(
        &[adult
            .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (3.0, 0.7))
            .with(|s| {
                s.on_ground = false;
                s.in_water = true;
            })],
        &atlas,
    );
    assert_ne!(walking.cutout.vertices, swimming.cutout.vertices);
}

fn turtle_texture_images() -> Vec<EntityModelTextureImage> {
    [TURTLE_TEXTURE_REF, TURTLE_BABY_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
