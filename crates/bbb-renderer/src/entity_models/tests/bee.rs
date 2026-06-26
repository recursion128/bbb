use super::*;

use crate::entity_models::model::ModelCube;
use std::f32::consts::PI;

#[test]
fn bee_geometry_matches_vanilla_26_1_body_layers() {
    // Adult `AdultBeeModel.createBodyLayer` (atlas 64×64): the empty bone parents a 7×7×10 body.
    // Each unified cube carries both the colored geometry/tint and the textured `uv_size` /
    // `texOffs` / `mirror`.
    assert_eq!(
        BEE_BODY[0],
        ModelCube::new(
            [-3.5, -4.0, -5.0],
            [7.0, 7.0, 10.0],
            BEE_YELLOW,
            [7.0, 7.0, 10.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(BEE_BONE_POSE.offset, [0.0, 19.0, 0.0]);
    // The stinger is a zero-thickness plane, the legs zero-depth planes; each carries its texOffs.
    assert_eq!(BEE_STINGER[0].size, [0.0, 1.0, 2.0]);
    assert_eq!(BEE_STINGER[0].tex, [26.0, 7.0]);
    assert_eq!(BEE_LEFT_ANTENNA[0].tex, [2.0, 0.0]);
    assert_eq!(BEE_RIGHT_ANTENNA[0].tex, [2.0, 3.0]);
    assert_eq!(BEE_FRONT_LEGS[0].size, [7.0, 2.0, 0.0]);
    assert_eq!(BEE_FRONT_LEGS[0].tex, [26.0, 1.0]);
    assert_eq!(BEE_MIDDLE_LEGS[0].tex, [26.0, 3.0]);
    assert_eq!(BEE_BACK_LEGS[0].tex, [26.0, 5.0]);
    // The wings carry the vanilla `CubeDeformation(0.001)` on the geometry but keep the BASE box
    // `uv_size`; both share `texOffs(0, 18)` and only the left wing's UV mirrors.
    assert_eq!(BEE_RIGHT_WING[0].min, [-9.001, -0.001, -0.001]);
    assert_eq!(BEE_RIGHT_WING[0].size, [9.002, 0.002, 6.002]);
    assert_eq!(BEE_RIGHT_WING[0].uv_size, [9.0, 0.0, 6.0]);
    assert_eq!(BEE_RIGHT_WING[0].tex, [0.0, 18.0]);
    assert!(!BEE_RIGHT_WING[0].mirror);
    assert_eq!(BEE_LEFT_WING[0].uv_size, [9.0, 0.0, 6.0]);
    assert_eq!(BEE_LEFT_WING[0].tex, [0.0, 18.0]);
    assert!(BEE_LEFT_WING[0].mirror);
    assert_eq!(BEE_RIGHT_WING_POSE.rotation, [0.0, -0.2618, 0.0]);
    assert_eq!(BEE_LEFT_WING_POSE.rotation, [0.0, 0.2618, 0.0]);

    // Baby `BabyBeeModel.createBodyLayer` (atlas 32×32): the bone itself carries two cubes, there
    // are no antennae, and the wings sit at a `0.2182` pitch. The left wing carries the vanilla
    // negative `texOffs(-3, 9)` with a mirrored box.
    assert_eq!(BEE_BABY_BONE.len(), 2);
    assert_eq!(BEE_BABY_BONE[0].tex, [6.0, 12.0]);
    assert_eq!(BEE_BABY_BONE[1].tex, [0.0, 12.0]);
    assert_eq!(BEE_BABY_BODY[0].size, [4.0, 4.0, 5.0]);
    assert_eq!(BEE_BABY_BODY[0].tex, [0.0, 0.0]);
    assert_eq!(BEE_BABY_BONE_POSE.offset, [0.0, 19.6667, -1.8567]);
    assert_eq!(BEE_BABY_RIGHT_WING_POSE.rotation, [0.2182, 0.3491, 0.0]);
    assert_eq!(BEE_BABY_LEFT_WING_POSE.rotation, [0.2182, -0.3491, 0.0]);
    assert_eq!(BEE_BABY_LEFT_WING[0].tex, [-3.0, 9.0]);
    assert!(BEE_BABY_LEFT_WING[0].mirror);
}

#[test]
fn bee_animation_helpers_match_vanilla_setup_anim() {
    // `rightWing.zRot = cos(ageInTicks · 120.32113°) · π · 0.15` (peaks at `t=0`).
    assert!((bee_wing_z_rot(0.0) - PI * 0.15).abs() < 1.0e-6);
    // `bobUpAndDown` reads `speed = cos(ageInTicks · 0.18)` (1 at `t=0`).
    assert!((bee_bob_speed(0.0) - 1.0).abs() < 1.0e-6);
    // `bone.xRot = 0.1 + speed · π · 0.025`.
    assert!((bee_bone_x_rot(0.0) - (0.1 + PI * 0.025)).abs() < 1.0e-6);
    // `bone.y -= cos(ageInTicks · 0.18) · 0.9`.
    assert!((bee_bone_y_delta(0.0) - -0.9).abs() < 1.0e-6);
    // `frontLeg.xRot = -speed · π · 0.1 + π/8`, `backLeg.xRot = -speed · π · 0.05 + π/4`.
    assert!((bee_front_leg_x_rot(0.0) - (-PI * 0.1 + PI / 8.0)).abs() < 1.0e-6);
    assert!((bee_back_leg_x_rot(0.0) - (-PI * 0.05 + PI / 4.0)).abs() < 1.0e-6);
    // Adult-only antenna bob: `antenna.xRot = speed · π · 0.03`.
    assert!((bee_antenna_x_rot(0.0) - PI * 0.03).abs() < 1.0e-6);
    // The airborne middle leg holds `π/4`.
    assert!((BEE_MID_LEG_FLYING_X_ROT - PI / 4.0).abs() < 1.0e-6);
}

#[test]
fn bee_mesh_uses_vanilla_body_layer_geometry() {
    // Adult: body, stinger, two antennae, two wings, three leg planes → 9 cubes / 54 faces / 216
    // vertices.
    let adult = entity_model_mesh(&[EntityModelInstance::bee(930, [0.0, 64.0, 0.0], 0.0, false)]);
    assert_eq!(adult.opaque_faces, 54);
    assert_eq!(adult.vertices.len(), 216);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(BEE_YELLOW, 1.0)));

    // Baby: bone (two cubes), body, stinger, two wings, three leg planes → also 9 cubes.
    let baby = entity_model_mesh(&[EntityModelInstance::bee(931, [0.0, 64.0, 0.0], 0.0, true)]);
    assert_eq!(baby.opaque_faces, 54);
    assert_eq!(baby.vertices.len(), 216);
}

#[test]
fn bee_flaps_while_airborne_and_rests_on_ground() {
    // Airborne (the constructor default, `on_ground = false`): the wing flap and bob re-pose the
    // mesh as the age advances.
    let flying = EntityModelInstance::bee(932, [0.0, 64.0, 0.0], 0.0, false);
    let early = entity_model_mesh(&[flying]);
    let later = entity_model_mesh(&[flying.with_age_in_ticks(3.0)]);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(early.vertices, later.vertices, "the wings flap with age");

    // On the ground the model holds its bind pose, so it is static across ages and differs from
    // the airborne flap.
    let grounded = flying.with_on_ground(true);
    let grounded_early = entity_model_mesh(&[grounded]);
    let grounded_later = entity_model_mesh(&[grounded.with_age_in_ticks(3.0)]);
    assert_eq!(
        grounded_early.vertices, grounded_later.vertices,
        "a grounded bee rests at its bind pose"
    );
    assert_ne!(early.vertices, grounded_early.vertices);
}

#[test]
fn bee_texture_ref_matches_vanilla_renderer() {
    // model_key (mesh geometry) depends on `baby` only; angry/nectar never change the mesh.
    for &(angry, has_nectar) in &[(false, false), (true, false), (false, true), (true, true)] {
        assert_eq!(
            EntityModelKind::Bee {
                baby: false,
                angry,
                has_nectar,
            }
            .model_key(),
            "bee"
        );
        assert_eq!(
            EntityModelKind::Bee {
                baby: true,
                angry,
                has_nectar,
            }
            .model_key(),
            "bee_baby"
        );
    }

    // Vanilla `BeeRenderer.getTextureLocation`: the eight angry × nectar × baby faces.
    let texture = |baby, angry, has_nectar| {
        EntityModelKind::Bee {
            baby,
            angry,
            has_nectar,
        }
        .vanilla_texture_ref()
        .unwrap()
        .path
    };
    assert_eq!(texture(false, false, false), "textures/entity/bee/bee.png");
    assert_eq!(
        texture(false, true, false),
        "textures/entity/bee/bee_angry.png"
    );
    assert_eq!(
        texture(false, false, true),
        "textures/entity/bee/bee_nectar.png"
    );
    assert_eq!(
        texture(false, true, true),
        "textures/entity/bee/bee_angry_nectar.png"
    );
    assert_eq!(
        texture(true, false, false),
        "textures/entity/bee/bee_baby.png"
    );
    assert_eq!(
        texture(true, true, false),
        "textures/entity/bee/bee_angry_baby.png"
    );
    assert_eq!(
        texture(true, false, true),
        "textures/entity/bee/bee_nectar_baby.png"
    );
    assert_eq!(
        texture(true, true, true),
        "textures/entity/bee/bee_angry_nectar_baby.png"
    );

    // The accessor lists all eight faces (adult base/baby first, then the variants).
    assert_eq!(
        bee_entity_texture_refs()
            .iter()
            .map(|texture| texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/bee/bee.png",
            "textures/entity/bee/bee_baby.png",
            "textures/entity/bee/bee_angry.png",
            "textures/entity/bee/bee_nectar.png",
            "textures/entity/bee/bee_angry_nectar.png",
            "textures/entity/bee/bee_angry_baby.png",
            "textures/entity/bee/bee_nectar_baby.png",
            "textures/entity/bee/bee_angry_nectar_baby.png",
        ]
    );
}

#[test]
fn bee_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&bee_texture_images()).unwrap();

    // Vanilla `BeeModel` calls `EntityModel(root)`, so the base submit uses the default
    // `entityCutout` render type. The backend folds it into the cutout mesh, but the submission
    // keeps the vanilla texture, render type, tint, transform, and default collector order.
    let adult = EntityModelInstance::bee(940, [0.0, 64.0, 0.0], 0.0, false);
    let meshes = entity_model_textured_meshes(&[adult], &atlas);
    assert_bee_base_submission(&meshes, adult, BEE_TEXTURE_REF);

    // Nine cubes → 54 faces / 216 vertices, nothing on the translucent or eyes passes, white tint.
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.cutout.cutout_faces, 54);
    assert_eq!(meshes.cutout.vertices.len(), 216);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    // Baby is the smaller separate model: also nine cubes → 54 faces / 216 vertices.
    let baby = EntityModelInstance::bee(941, [0.0, 64.0, 0.0], 0.0, true);
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_bee_base_submission(&baby_meshes, baby, BEE_BABY_TEXTURE_REF);
    assert_eq!(baby_meshes.cutout.vertices.len(), 216);

    // Texture variants stay a model-kind selection: angry + nectar swaps the atlas reference while
    // preserving the base render type, tint, transform, and order.
    let angry_nectar = EntityModelInstance::new(
        942,
        EntityModelKind::Bee {
            baby: false,
            angry: true,
            has_nectar: true,
        },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with_bee_angry(true);
    let angry_nectar_meshes = entity_model_textured_meshes(&[angry_nectar], &atlas);
    assert_bee_base_submission(
        &angry_nectar_meshes,
        angry_nectar,
        BEE_ANGRY_NECTAR_TEXTURE_REF,
    );
    assert_eq!(angry_nectar_meshes.cutout.vertices.len(), 216);

    // Airborne the flap re-poses the mesh with age; a grounded bee rests at its bind pose.
    let later = entity_model_textured_meshes(&[adult.with_age_in_ticks(3.0)], &atlas);
    assert_ne!(meshes.cutout.vertices, later.cutout.vertices);
    let grounded = entity_model_textured_meshes(&[adult.with_on_ground(true)], &atlas);
    assert_ne!(meshes.cutout.vertices, grounded.cutout.vertices);
}

#[test]
fn bee_hides_its_stinger_after_stinging() {
    // Vanilla `BeeModel.setupAnim` toggles `stinger.visible = hasStinger`. A bee that has not
    // stung shows its stinger (the default), so the mesh is the full 9 cubes / 216 vertices;
    // once it stings (`bee_has_stinger = false`) the stinger cube is dropped, leaving 8 cubes /
    // 192 vertices. The adult emits the stinger right after the body, so the surviving vertices
    // are the body (kept) plus everything past the stinger. Colored path here, textured below.
    let adult = EntityModelInstance::bee(950, [0.0, 64.0, 0.0], 0.0, false);
    let with_stinger = entity_model_mesh(&[adult]);
    assert_eq!(
        with_stinger.vertices.len(),
        216,
        "a bee that has not stung shows its stinger by default"
    );

    let stung = entity_model_mesh(&[adult.with_bee_has_stinger(false)]);
    assert_eq!(stung.opaque_faces, 48);
    assert_eq!(
        stung.vertices.len(),
        192,
        "a stung bee drops the 24-vertex stinger cube"
    );
    // Exactly the stinger cube (vertices [24, 48), right after the body) is removed; the body
    // and every other part are byte-identical.
    assert_eq!(
        with_stinger.vertices[0..24],
        stung.vertices[0..24],
        "the body is unchanged"
    );
    assert_eq!(
        with_stinger.vertices[48..216],
        stung.vertices[24..192],
        "only the stinger cube is removed; the antennae, wings and legs are unchanged"
    );

    // Baby bees lose their stinger the same way (a fourth cube fewer from the 216).
    let baby = EntityModelInstance::bee(951, [0.0, 64.0, 0.0], 0.0, true);
    let baby_stung = entity_model_mesh(&[baby.with_bee_has_stinger(false)]);
    assert_eq!(baby_stung.vertices.len(), 192);
}

#[test]
fn bee_textured_mesh_hides_its_stinger_after_stinging() {
    let (atlas, _) = build_entity_model_texture_atlas(&bee_texture_images()).unwrap();
    let adult = EntityModelInstance::bee(952, [0.0, 64.0, 0.0], 0.0, false);
    let with_stinger = entity_model_textured_meshes(&[adult], &atlas);
    assert_eq!(with_stinger.cutout.vertices.len(), 216);

    let stung = entity_model_textured_meshes(&[adult.with_bee_has_stinger(false)], &atlas);
    assert_eq!(stung.cutout.cutout_faces, 48);
    assert_eq!(
        stung.cutout.vertices.len(),
        192,
        "the textured stinger cube is dropped once the bee has stung"
    );
    assert_eq!(
        with_stinger.cutout.vertices[48..216],
        stung.cutout.vertices[24..192],
        "only the stinger cube is removed"
    );
}

#[test]
fn bee_stops_bobbing_when_angry() {
    // Vanilla `BeeModel.setupAnim` gates `bobUpAndDown` on `!isAngry`: a calm airborne bee
    // rocks its whole body (the bob runs through the `bone` pivot), while an angry one freezes
    // everything except the wings, which keep flapping. Adult vertices: body[0,24),
    // stinger[24,48), antennae[48,96), wings[96,144), legs[144,216).
    let calm = EntityModelInstance::bee(960, [0.0, 64.0, 0.0], 0.0, false); // airborne by default
    let angry = calm.with_bee_angry(true);

    // A calm bee bobs: the body and antennae shift as the age advances.
    let calm0 = entity_model_mesh(&[calm]);
    let calm3 = entity_model_mesh(&[calm.with_age_in_ticks(3.0)]);
    assert_ne!(
        calm0.vertices[0..96],
        calm3.vertices[0..96],
        "a calm bee rocks its body and antennae with the bob"
    );

    // An angry bee freezes the body, antennae and legs; only the wings keep flapping.
    let angry0 = entity_model_mesh(&[angry]);
    let angry3 = entity_model_mesh(&[angry.with_age_in_ticks(3.0)]);
    assert_eq!(
        angry0.vertices[0..96],
        angry3.vertices[0..96],
        "an angry bee's body and antennae hold still"
    );
    assert_eq!(
        angry0.vertices[144..216],
        angry3.vertices[144..216],
        "an angry bee's legs hold still at π/4"
    );
    assert_ne!(
        angry0.vertices[96..144],
        angry3.vertices[96..144],
        "the wings keep flapping even when the bee is angry"
    );

    // At a fixed age the anger gate changes the pose (the bob is off).
    assert_ne!(
        calm0.vertices, angry0.vertices,
        "an angry bee poses differently from a calm one"
    );

    // A grounded bee rests at its bind pose regardless of anger (the bob never runs).
    let grounded_calm = entity_model_mesh(&[calm.with_on_ground(true)]);
    let grounded_angry = entity_model_mesh(&[calm.with_on_ground(true).with_bee_angry(true)]);
    assert_eq!(
        grounded_calm.vertices, grounded_angry.vertices,
        "a grounded bee ignores anger"
    );
}

#[test]
fn bee_rolls_onto_its_back_with_roll_amount() {
    // Vanilla `BeeModel.setupAnim` applies the barrel roll last: `if rollAmount > 0` it flips the
    // `bone` pivot pitch toward `3.0915928` via `rotLerpRad`. Use a grounded bee so the roll is the
    // only animation (the flap/bob never run on the ground), isolating the flip.
    let grounded = EntityModelInstance::bee(970, [0.0, 64.0, 0.0], 0.0, false).with_on_ground(true);
    let upright = entity_model_mesh(&[grounded]);

    // A partial roll re-poses the whole bee, and a fuller roll re-poses it further.
    let half = entity_model_mesh(&[grounded.with_bee_roll_amount(0.5)]);
    let full = entity_model_mesh(&[grounded.with_bee_roll_amount(1.0)]);
    assert_eq!(upright.vertices.len(), half.vertices.len());
    assert_ne!(
        upright.vertices, half.vertices,
        "a rolling bee tips off its upright bind pose"
    );
    assert_ne!(
        half.vertices, full.vertices,
        "a fuller roll tips the bee further onto its back"
    );

    // `rollAmount = 0` is the upright bind pose (the `if rollAmount > 0` guard skips the flip).
    let zero = entity_model_mesh(&[grounded.with_bee_roll_amount(0.0)]);
    assert_eq!(
        upright.vertices, zero.vertices,
        "a zero roll leaves the bee upright"
    );
}

#[test]
fn bee_textured_mesh_stops_bobbing_when_angry() {
    let (atlas, _) = build_entity_model_texture_atlas(&bee_texture_images()).unwrap();
    let calm = EntityModelInstance::bee(961, [0.0, 64.0, 0.0], 0.0, false);
    let angry = calm.with_bee_angry(true);

    let calm0 = entity_model_textured_meshes(&[calm], &atlas);
    let angry0 = entity_model_textured_meshes(&[angry], &atlas);
    assert_ne!(
        calm0.cutout.vertices, angry0.cutout.vertices,
        "the anger gate changes the textured pose"
    );

    // As with the colored path, an angry bee freezes everything but the wings.
    let angry3 = entity_model_textured_meshes(&[angry.with_age_in_ticks(3.0)], &atlas);
    assert_eq!(
        angry0.cutout.vertices[0..96],
        angry3.cutout.vertices[0..96],
        "the body and antennae hold still while angry"
    );
    assert_ne!(
        angry0.cutout.vertices[96..144],
        angry3.cutout.vertices[96..144],
        "the wings keep flapping while angry"
    );
}

fn bee_texture_images() -> Vec<EntityModelTextureImage> {
    bee_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn assert_bee_base_submission(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    texture: EntityModelTextureRef,
) {
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.texture, texture);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.transform, entity_model_root_transform(instance));
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
}
