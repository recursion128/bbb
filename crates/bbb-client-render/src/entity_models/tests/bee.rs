use super::*;

use std::f32::consts::PI;

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
fn bee_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&bee_texture_images()).unwrap();

    // Vanilla `BeeModel` calls `EntityModel(root)`, so the base submit uses the default
    // `entityCutout` render type. The backend folds it into the cutout mesh, but the submission
    // keeps the vanilla texture, render type, tint, transform, and default collector order.
    let adult = EntityModelInstance::bee(940, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[adult], &atlas);

    assert_bee_cutout_vertex_state(&meshes, adult);

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
    let baby = EntityModelInstance::bee(941, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);

    assert_eq!(baby_meshes.cutout.vertices.len(), 216);
    assert_bee_cutout_vertex_state(&baby_meshes, baby);

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
    .with(|s| {
        s.kind.bee_mut().angry = true;
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let angry_nectar_meshes = entity_model_textured_meshes(&[angry_nectar], &atlas);

    assert_eq!(angry_nectar_meshes.cutout.vertices.len(), 216);
    assert_bee_cutout_vertex_state(&angry_nectar_meshes, angry_nectar);

    // Airborne the flap re-poses the mesh with age; a grounded bee rests at its bind pose.
    let later = entity_model_textured_meshes(&[adult.with(|s| s.age_in_ticks = 3.0)], &atlas);
    assert_ne!(meshes.cutout.vertices, later.cutout.vertices);
    let grounded = entity_model_textured_meshes(&[adult.with(|s| s.on_ground = true)], &atlas);
    assert_ne!(meshes.cutout.vertices, grounded.cutout.vertices);
}

#[test]
fn bee_textured_mesh_hides_its_stinger_after_stinging() {
    let (atlas, _) = build_entity_model_texture_atlas(&bee_texture_images()).unwrap();
    let adult = EntityModelInstance::bee(952, [0.0, 64.0, 0.0], 0.0, false);
    let with_stinger = entity_model_textured_meshes(&[adult], &atlas);
    assert_eq!(with_stinger.cutout.vertices.len(), 216);

    let stung = entity_model_textured_meshes(
        &[adult.with(|s| s.kind.bee_mut().has_stinger = false)],
        &atlas,
    );
    assert_eq!(stung.cutout.cutout_faces, 48);
    assert_eq!(
        stung.cutout.vertices.len(),
        192,
        "the textured stinger cube is dropped once the bee has stung"
    );

    let stinger = ModelLayer::BEE.bake().part_vertex_range("stinger").unwrap();
    let mut without_stinger = with_stinger.cutout.vertices.clone();
    without_stinger.drain(stinger);
    assert_eq!(
        without_stinger, stung.cutout.vertices,
        "only the stinger cube is removed"
    );
}

#[test]
fn bee_textured_mesh_stops_bobbing_when_angry() {
    let (atlas, _) = build_entity_model_texture_atlas(&bee_texture_images()).unwrap();
    let calm = EntityModelInstance::bee(961, [0.0, 64.0, 0.0], 0.0, false);
    let angry = calm.with(|s| s.kind.bee_mut().angry = true);

    let calm0 = entity_model_textured_meshes(&[calm], &atlas);
    let angry0 = entity_model_textured_meshes(&[angry], &atlas);
    assert_ne!(
        calm0.cutout.vertices, angry0.cutout.vertices,
        "the anger gate changes the textured pose"
    );

    // An angry bee freezes everything but the wings.
    let angry3 = entity_model_textured_meshes(&[angry.with(|s| s.age_in_ticks = 3.0)], &atlas);

    let model = ModelLayer::BEE.bake();
    let still = ["body", "front_legs", "middle_legs", "back_legs"];
    let wings = ["right_wing", "left_wing"];
    assert_eq!(
        parts_vertices(&angry0.cutout.vertices, &model, &still),
        parts_vertices(&angry3.cutout.vertices, &model, &still),
        "the body and antennae hold still while angry"
    );
    assert_ne!(
        parts_vertices(&angry0.cutout.vertices, &model, &wings),
        parts_vertices(&angry3.cutout.vertices, &model, &wings),
        "the wings keep flapping while angry"
    );
}

fn bee_texture_images() -> Vec<EntityModelTextureImage> {
    texture_refs_matching(|path| {
        (path.starts_with("textures/entity/bee/")) && !path.ends_with("bee_stinger.png")
    })
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect()
}

fn assert_bee_cutout_vertex_state(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
}
