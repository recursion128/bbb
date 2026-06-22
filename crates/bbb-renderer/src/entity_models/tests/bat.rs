use super::*;

#[test]
fn bat_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BatModel.createBodyLayer` (atlas 32×32).
    assert_eq!(BAT_BODY[0].min, [-1.5, 0.0, -1.0]);
    assert_eq!(BAT_BODY[0].size, [3.0, 5.0, 2.0]);
    assert_eq!(BAT_HEAD[0].min, [-2.0, -3.0, -1.0]);
    assert_eq!(BAT_HEAD[0].size, [4.0, 3.0, 2.0]);

    // Ears and wings are zero-thickness planes.
    assert_eq!(BAT_RIGHT_EAR[0].size, [3.0, 5.0, 0.0]);
    assert_eq!(BAT_RIGHT_WING[0].size, [2.0, 7.0, 0.0]);
    assert_eq!(BAT_RIGHT_WING_TIP[0].size, [6.0, 8.0, 0.0]);
    assert_eq!(BAT_FEET[0].size, [3.0, 2.0, 0.0]);

    // Bind-pose offsets: body and head at +17, the ears under the head, the wings/feet under
    // the body, and each wing tip under its wing.
    assert_eq!(BAT_BODY_POSE.offset, [0.0, 17.0, 0.0]);
    assert_eq!(BAT_HEAD_POSE.offset, [0.0, 17.0, 0.0]);
    assert_eq!(BAT_RIGHT_EAR_POSE.offset, [-1.5, -2.0, 0.0]);
    assert_eq!(BAT_LEFT_EAR_POSE.offset, [1.1, -3.0, 0.0]);
    assert_eq!(BAT_RIGHT_WING_POSE.offset, [-1.5, 0.0, 0.0]);
    assert_eq!(BAT_RIGHT_WING_TIP_POSE.offset, [-2.0, 0.0, 0.0]);
    assert_eq!(BAT_LEFT_WING_POSE.offset, [1.5, 0.0, 0.0]);
    assert_eq!(BAT_FEET_POSE.offset, [0.0, 5.0, 0.0]);
}

#[test]
fn bat_flying_animation_matches_vanilla_definition() {
    // Vanilla `BatAnimation.BAT_FLYING` is a 0.5s looping animation over seven bones.
    assert_eq!(BAT_FLYING.length_seconds, 0.5);
    assert!(BAT_FLYING.looping);
    assert_eq!(BAT_FLYING.bones.len(), 7);
}

#[test]
fn bat_mesh_uses_vanilla_body_layer_geometry() {
    // Nine cubes (body, head, two ears, two wings, two wing tips, feet) → 54 faces / 216
    // vertices.
    let bat = entity_model_mesh(&[EntityModelInstance::bat(910, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(bat.opaque_faces, 54);
    assert_eq!(bat.vertices.len(), 216);
    assert_eq!(bat.indices.len(), 324);
    assert!(bat
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(BAT_BROWN, 1.0)));
}

#[test]
fn bat_wings_flap_with_age() {
    // The looping `BAT_FLYING` flap re-poses the mesh as the age advances within the 0.5s
    // (10-tick) cycle.
    let base = EntityModelInstance::bat(911, [0.0, 64.0, 0.0], 0.0);
    let early = entity_model_mesh(&[base]);
    let later = entity_model_mesh(&[base.with_age_in_ticks(3.0)]);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(early.vertices, later.vertices, "the wings flap with age");

    // The animation loops every 0.5s = 10 ticks, so age 0 and age 10 sample the same phase.
    let one_cycle = entity_model_mesh(&[base.with_age_in_ticks(10.0)]);
    assert_eq!(
        early.vertices, one_cycle.vertices,
        "the flap loops every 10 ticks"
    );
}

#[test]
fn bat_texture_ref_matches_vanilla_renderer() {
    let kind = EntityModelKind::Bat;
    assert_eq!(kind.model_key(), "bat");
    assert_eq!(
        kind.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/bat/bat.png",
            size: [32, 32],
        })
    );
}
