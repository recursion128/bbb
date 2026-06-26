use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn turtle_adult_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultTurtleModel.createBodyLayer` (atlas 128×64). Each unified cube carries both the
    // colored geometry/tint and the textured `uv_size` / `texOffs` / `mirror`.
    assert_eq!(
        TURTLE_HEAD[0],
        ModelCube::new(
            [-3.0, -1.0, -3.0],
            [6.0, 5.0, 6.0],
            TURTLE_GREEN,
            [6.0, 5.0, 6.0],
            [3.0, 0.0],
            false,
        )
    );

    // Body: the `texOffs(7, 37)` shell box plus the `texOffs(31, 1)` belly box.
    assert_eq!(TURTLE_BODY.len(), 2);
    assert_eq!(TURTLE_BODY[0].min, [-9.5, 3.0, -10.0]);
    assert_eq!(TURTLE_BODY[0].size, [19.0, 20.0, 6.0]);
    assert_eq!(TURTLE_BODY[0].tex, [7.0, 37.0]);
    assert_eq!(TURTLE_BODY[0].uv_size, [19.0, 20.0, 6.0]);
    assert_eq!(TURTLE_BODY[1].min, [-5.5, 3.0, -13.0]);
    assert_eq!(TURTLE_BODY[1].size, [11.0, 18.0, 3.0]);
    assert_eq!(TURTLE_BODY[1].tex, [31.0, 1.0]);
    assert_eq!(TURTLE_BODY[1].uv_size, [11.0, 18.0, 3.0]);

    // Legs: hind `texOffs(1, 23)` / `texOffs(1, 12)`, front `texOffs(27, 30)` / `texOffs(27, 24)`.
    assert_eq!(TURTLE_RIGHT_HIND_LEG[0].size, [4.0, 1.0, 10.0]);
    assert_eq!(TURTLE_RIGHT_HIND_LEG[0].tex, [1.0, 23.0]);
    assert_eq!(TURTLE_LEFT_HIND_LEG[0].tex, [1.0, 12.0]);
    assert_eq!(TURTLE_RIGHT_FRONT_LEG[0].min, [-13.0, 0.0, -2.0]);
    assert_eq!(TURTLE_RIGHT_FRONT_LEG[0].size, [13.0, 1.0, 5.0]);
    assert_eq!(TURTLE_RIGHT_FRONT_LEG[0].tex, [27.0, 30.0]);
    assert_eq!(TURTLE_LEFT_FRONT_LEG[0].min, [0.0, 0.0, -2.0]);
    assert_eq!(TURTLE_LEFT_FRONT_LEG[0].tex, [27.0, 24.0]);
    assert!(!TURTLE_HEAD[0].mirror);

    // Offsets; the body carries the fixed `Rx(π/2)` shell tilt.
    assert_eq!(TURTLE_HEAD_POSE.offset, [0.0, 19.0, -10.0]);
    assert_eq!(TURTLE_BODY_POSE.offset, [0.0, 11.0, -10.0]);
    assert!((TURTLE_BODY_POSE.rotation[0] - std::f32::consts::FRAC_PI_2).abs() < 1.0e-6);
    assert_eq!(TURTLE_RIGHT_HIND_LEG_POSE.offset, [-3.5, 22.0, 11.0]);
    assert_eq!(TURTLE_LEFT_HIND_LEG_POSE.offset, [3.5, 22.0, 11.0]);
    assert_eq!(TURTLE_RIGHT_FRONT_LEG_POSE.offset, [-5.0, 21.0, -4.0]);
    assert_eq!(TURTLE_LEFT_FRONT_LEG_POSE.offset, [5.0, 21.0, -4.0]);
}

#[test]
fn turtle_baby_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyTurtleModel.createBodyLayer` (atlas 16×16): body `texOffs(0, 0)`, head
    // `texOffs(0, 6)`, the hind legs use the vanilla negative `texOffs(-1, …)`, the front legs
    // `texOffs(8, …)`.
    assert_eq!(TURTLE_BABY_BODY[0].min, [-2.0, -1.0, -2.0]);
    assert_eq!(TURTLE_BABY_BODY[0].size, [4.0, 2.0, 4.0]);
    assert_eq!(TURTLE_BABY_BODY[0].tex, [0.0, 0.0]);
    assert_eq!(TURTLE_BABY_HEAD[0].size, [3.0, 3.0, 3.0]);
    assert_eq!(TURTLE_BABY_HEAD[0].tex, [0.0, 6.0]);

    // Baby legs are zero-height `2×0×1` planes.
    assert_eq!(TURTLE_BABY_RIGHT_HIND_LEG[0].size, [2.0, 0.0, 1.0]);
    assert_eq!(TURTLE_BABY_RIGHT_HIND_LEG[0].tex, [-1.0, 0.0]);
    assert_eq!(TURTLE_BABY_LEFT_HIND_LEG[0].tex, [-1.0, 1.0]);
    assert_eq!(TURTLE_BABY_RIGHT_FRONT_LEG[0].tex, [8.0, 6.0]);
    assert_eq!(TURTLE_BABY_LEFT_FRONT_LEG[0].size, [2.0, 0.0, 1.0]);
    assert_eq!(TURTLE_BABY_LEFT_FRONT_LEG[0].tex, [8.0, 7.0]);

    assert_eq!(TURTLE_BABY_HEAD_POSE.offset, [0.0, 22.9, -1.0]);
    assert_eq!(TURTLE_BABY_BODY_POSE.offset, [0.0, 22.9, 1.0]);
    assert_eq!(TURTLE_BABY_RIGHT_HIND_LEG_POSE.offset, [-2.0, 23.9, 2.5]);
    assert_eq!(TURTLE_BABY_LEFT_FRONT_LEG_POSE.offset, [2.0, 23.9, -0.5]);
}

#[test]
fn turtle_egg_belly_geometry_matches_vanilla_26_1() {
    // Vanilla `AdultTurtleModel.createBodyLayer`: the `egg_belly` overlay shell is
    // `texOffs(70, 33).addBox(-4.5, 3, -14, 9, 18, 1)` at the same `PartPose` as the body
    // (`offsetAndRotation(0, 11, -10, π/2, 0, 0)`).
    assert_eq!(TURTLE_EGG_BELLY.len(), 1);
    assert_eq!(TURTLE_EGG_BELLY[0].min, [-4.5, 3.0, -14.0]);
    assert_eq!(TURTLE_EGG_BELLY[0].size, [9.0, 18.0, 1.0]);
    assert_eq!(TURTLE_EGG_BELLY[0].color, TURTLE_SHELL);
    assert_eq!(TURTLE_EGG_BELLY[0].tex, [70.0, 33.0]);
    assert_eq!(TURTLE_EGG_BELLY[0].uv_size, [9.0, 18.0, 1.0]);
    assert!(!TURTLE_EGG_BELLY[0].mirror);

    // `setupAnim` does `this.root.y--` while the egg belly is visible.
    assert_eq!(TURTLE_EGG_ROOT_DROP_POSE.offset, [0.0, -1.0, 0.0]);
    assert_eq!(TURTLE_EGG_ROOT_DROP_POSE.rotation, [0.0, 0.0, 0.0]);
}

#[test]
fn turtle_egg_belly_shows_and_drops_root_when_carrying_an_egg() {
    let plain = entity_model_mesh(&[EntityModelInstance::turtle(
        670,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let egg = entity_model_mesh(&[
        EntityModelInstance::turtle(670, [0.0, 64.0, 0.0], 0.0, false).with_turtle_has_egg(true),
    ]);

    // The `egg_belly` overlay shell is one extra cube: +6 faces / +24 vertices (eight cubes).
    assert_eq!(egg.opaque_faces, plain.opaque_faces + 6);
    assert_eq!(egg.vertices.len(), plain.vertices.len() + 24);
    assert_eq!(egg.vertices.len(), 192);

    // The model space is y-down (legs sit at a larger model y than the head), so the
    // `scale(-1, -1, 1)` flip turns `root.y--` into a uniform world rise. The shell is the
    // topmost part (the egg belly hangs below it), so its peak climbs by exactly the drop delta.
    let top = |mesh: &EntityModelMesh| {
        mesh.vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::NEG_INFINITY, f32::max)
    };
    let delta = top(&egg) - top(&plain);
    assert!(delta > 1.0e-4, "root.y-- lifts the whole model");

    // The shift is rigid: every plain (shared) vertex reappears in the egg mesh translated up by
    // the same delta on Y, with X/Z unchanged. (The 24 leftover egg vertices are the new cube.)
    for plain_vertex in &plain.vertices {
        let [x, y, z] = plain_vertex.position;
        let found = egg.vertices.iter().any(|egg_vertex| {
            (egg_vertex.position[0] - x).abs() < 1.0e-4
                && (egg_vertex.position[2] - z).abs() < 1.0e-4
                && (egg_vertex.position[1] - (y + delta)).abs() < 1.0e-3
        });
        assert!(
            found,
            "every shared part shifts up by the same root.y-- delta"
        );
    }
}

#[test]
fn turtle_baby_ignores_has_egg() {
    // Only `AdultTurtleModel` has the egg belly; the projection clears `hasEgg` for babies, and
    // the baby model has no egg-belly part regardless of the flag.
    let baby = entity_model_mesh(&[EntityModelInstance::turtle(
        671,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    let baby_with_flag =
        entity_model_mesh(&[
            EntityModelInstance::turtle(671, [0.0, 64.0, 0.0], 0.0, true).with_turtle_has_egg(true),
        ]);
    assert_eq!(baby.vertices.len(), 144);
    assert_eq!(baby.vertices, baby_with_flag.vertices);
}

#[test]
fn turtle_textured_egg_belly_shows_when_carrying_an_egg() {
    let (atlas, _) = build_entity_model_texture_atlas(&turtle_texture_images()).unwrap();

    let plain = EntityModelInstance::turtle(752, [0.0, 64.0, 0.0], 0.0, false).with_on_ground(true);
    let egg = plain.with_turtle_has_egg(true);
    let plain_meshes = entity_model_textured_meshes(&[plain], &atlas);
    let egg_meshes = entity_model_textured_meshes(&[egg], &atlas);

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
fn turtle_laying_egg_animates_front_legs_through_the_mesh() {
    // A laying turtle walking on land poses its front legs differently (amplified yaw swing).
    let base = EntityModelInstance::turtle(680, [0.0, 64.0, 0.0], 0.0, false)
        .with_walk_animation(3.0, 0.7)
        .with_on_ground(true)
        .with_in_water(false);
    let plain = entity_model_mesh(&[base]);
    let laying = entity_model_mesh(&[base.with_turtle_laying_egg(true)]);
    assert_eq!(plain.vertices.len(), laying.vertices.len());
    assert_ne!(
        plain.vertices, laying.vertices,
        "laying amplifies the front-leg land swing"
    );

    // The amplitude lives in the shared `TurtleModel`, so baby turtles lay too.
    let baby = EntityModelInstance::turtle(681, [0.0, 64.0, 0.0], 0.0, true)
        .with_walk_animation(3.0, 0.7)
        .with_on_ground(true)
        .with_in_water(false);
    assert_ne!(
        entity_model_mesh(&[baby]).vertices,
        entity_model_mesh(&[baby.with_turtle_laying_egg(true)]).vertices,
    );

    // In water the laying flag has no effect (vanilla applies `layEgg` only on land).
    let swimming = base.with_on_ground(false).with_in_water(true);
    assert_eq!(
        entity_model_mesh(&[swimming]).vertices,
        entity_model_mesh(&[swimming.with_turtle_laying_egg(true)]).vertices,
    );
}

#[test]
fn turtle_adult_mesh_uses_vanilla_body_layer_geometry() {
    // Seven cubes (head, shell, belly, four legs) → 42 faces / 168 vertices. The egg_belly
    // shell is gated on `hasEgg` and is not emitted for a turtle without an egg.
    let turtle = entity_model_mesh(&[EntityModelInstance::turtle(
        660,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_eq!(turtle.opaque_faces, 42);
    assert_eq!(turtle.vertices.len(), 168);
    assert_eq!(turtle.indices.len(), 252);
    assert!(turtle
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(TURTLE_SHELL, 1.0)));
    assert!(turtle
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(TURTLE_GREEN, 1.0)));
}

#[test]
fn turtle_baby_mesh_uses_vanilla_body_layer_geometry() {
    // Six cubes (body, head, four zero-height leg planes) → 36 faces / 144 vertices.
    let baby = entity_model_mesh(&[EntityModelInstance::turtle(
        661,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    assert_eq!(baby.opaque_faces, 36);
    assert_eq!(baby.vertices.len(), 144);

    // The baby is a different (smaller) model than the adult.
    let adult = entity_model_mesh(&[EntityModelInstance::turtle(
        662,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_ne!(baby.vertices.len(), adult.vertices.len());
}

#[test]
fn turtle_head_tracks_look_angles() {
    let base = EntityModelInstance::turtle(663, [0.0, 64.0, 0.0], 0.0, false);
    let forward = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(40.0, -20.0)]);
    assert_eq!(forward.vertices.len(), looking.vertices.len());
    assert_ne!(
        forward.vertices, looking.vertices,
        "the head tracks the look"
    );
}

#[test]
fn turtle_land_and_water_leg_poses_differ() {
    // The land walk (legs swing on yRot) differs from the water paddle (hind xRot / front zRot).
    // `isOnLand = !in_water && on_ground`.
    let walking = EntityModelInstance::turtle(664, [0.0, 64.0, 0.0], 0.0, false)
        .with_walk_animation(3.0, 0.7);
    let on_land = walking.with_on_ground(true).with_in_water(false);
    let swimming = walking.with_on_ground(false).with_in_water(true);
    let land_mesh = entity_model_mesh(&[on_land]);
    let water_mesh = entity_model_mesh(&[swimming]);
    assert_eq!(land_mesh.vertices.len(), water_mesh.vertices.len());
    assert_ne!(
        land_mesh.vertices, water_mesh.vertices,
        "the land walk and water paddle pose the legs differently"
    );

    // A standing turtle (no walk) differs from a walking one.
    let standing =
        entity_model_mesh(&[
            EntityModelInstance::turtle(665, [0.0, 64.0, 0.0], 0.0, false).with_on_ground(true),
        ]);
    assert_ne!(
        standing.vertices, land_mesh.vertices,
        "the walk animates the legs"
    );
}

#[test]
fn turtle_texture_refs_match_vanilla_renderer() {
    assert_eq!(
        EntityModelKind::Turtle { baby: false }.model_key(),
        "turtle"
    );
    assert_eq!(
        EntityModelKind::Turtle { baby: true }.model_key(),
        "turtle_baby"
    );
    assert_eq!(
        EntityModelKind::Turtle { baby: false }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/turtle/turtle.png",
            size: [128, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Turtle { baby: true }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/turtle/turtle_baby.png",
            size: [16, 16],
        })
    );
}

#[test]
fn turtle_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&turtle_texture_images()).unwrap();

    // Adult renders into the cutout mesh (default `RenderTypes::entityCutout`). Seven cubes →
    // 42 faces / 168 vertices, with nothing on the translucent or eyes passes.
    let adult = EntityModelInstance::turtle(750, [0.0, 64.0, 0.0], 0.0, false).with_on_ground(true);
    let meshes = entity_model_textured_meshes(&[adult], &atlas);
    assert_eq!(meshes.submissions.len(), 1);
    let adult_submit = meshes.submissions[0];
    assert_eq!(
        adult_submit.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult_submit.texture, TURTLE_TEXTURE_REF);
    assert_eq!(adult_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(adult_submit.order, 0);
    assert_eq!(adult_submit.submit_sequence, 0);
    assert_eq!(adult_submit.transform, entity_model_root_transform(adult));
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.cutout.cutout_faces, 42);
    assert_eq!(meshes.cutout.vertices.len(), 168);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    // Baby is the smaller model and uses vanilla `BabyTurtleModel`'s `entityCutoutCull` render type:
    // six cubes → 36 faces / 144 vertices.
    let baby = EntityModelInstance::turtle(751, [0.0, 64.0, 0.0], 0.0, true);
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_eq!(baby_meshes.submissions.len(), 1);
    let baby_submit = baby_meshes.submissions[0];
    assert_eq!(
        baby_submit.render_type,
        EntityModelLayerRenderType::EntityCutoutCull
    );
    assert_eq!(baby_submit.texture, TURTLE_BABY_TEXTURE_REF);
    assert_eq!(baby_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(baby_submit.order, 0);
    assert_eq!(baby_submit.submit_sequence, 0);
    assert_eq!(baby_submit.transform, entity_model_root_transform(baby));
    assert_eq!(baby_meshes.cutout.vertices.len(), 144);

    // The head tracks the look, and the land walk differs from the water paddle.
    let looking = entity_model_textured_meshes(&[adult.with_head_look(40.0, -20.0)], &atlas);
    assert_ne!(meshes.cutout.vertices, looking.cutout.vertices);
    let walking = entity_model_textured_meshes(
        &[adult
            .with_walk_animation(3.0, 0.7)
            .with_on_ground(true)
            .with_in_water(false)],
        &atlas,
    );
    let swimming = entity_model_textured_meshes(
        &[adult
            .with_walk_animation(3.0, 0.7)
            .with_on_ground(false)
            .with_in_water(true)],
        &atlas,
    );
    assert_ne!(walking.cutout.vertices, swimming.cutout.vertices);
}

fn turtle_texture_images() -> Vec<EntityModelTextureImage> {
    turtle_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
