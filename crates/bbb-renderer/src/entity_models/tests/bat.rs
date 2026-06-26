use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn bat_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BatModel.createBodyLayer` (atlas 32×32). Each unified cube carries both the colored
    // geometry/tint and the textured `uv_size` / `texOffs` / `mirror`; no `CubeDeformation`, so each
    // `uv_size` matches its box `size`.
    assert_eq!(
        BAT_BODY[0],
        ModelCube::new(
            [-1.5, 0.0, -1.0],
            [3.0, 5.0, 2.0],
            BAT_BROWN,
            [3.0, 5.0, 2.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(
        BAT_HEAD[0],
        ModelCube::new(
            [-2.0, -3.0, -1.0],
            [4.0, 3.0, 2.0],
            BAT_BROWN,
            [4.0, 3.0, 2.0],
            [0.0, 7.0],
            false,
        )
    );

    // Ears and wings are zero-thickness planes, each with its own `texOffs`.
    assert_eq!(BAT_RIGHT_EAR[0].size, [3.0, 5.0, 0.0]);
    assert_eq!(BAT_RIGHT_EAR[0].tex, [1.0, 15.0]);
    assert_eq!(BAT_LEFT_EAR[0].tex, [8.0, 15.0]);
    assert_eq!(BAT_RIGHT_WING[0].size, [2.0, 7.0, 0.0]);
    assert_eq!(BAT_RIGHT_WING[0].tex, [12.0, 0.0]);
    assert_eq!(BAT_LEFT_WING[0].tex, [12.0, 7.0]);
    assert_eq!(BAT_RIGHT_WING_TIP[0].size, [6.0, 8.0, 0.0]);
    assert_eq!(BAT_RIGHT_WING_TIP[0].tex, [16.0, 0.0]);
    assert_eq!(BAT_LEFT_WING_TIP[0].tex, [16.0, 8.0]);
    assert_eq!(BAT_FEET[0].size, [3.0, 2.0, 0.0]);
    assert_eq!(BAT_FEET[0].tex, [16.0, 16.0]);

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
    // The texture-ref accessor lists exactly the single base texture.
    assert_eq!(
        bat_entity_texture_refs(),
        &[EntityModelTextureRef {
            path: "textures/entity/bat/bat.png",
            size: [32, 32],
        }]
    );
}

#[test]
fn bat_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&bat_texture_images()).unwrap();

    // Bat renders into the cutout mesh (vanilla `RenderTypes::entityCutoutCull`). Nine cubes →
    // 54 faces / 216 vertices, with nothing on the translucent or eyes passes and a white tint.
    let base = EntityModelInstance::bat(920, [0.0, 64.0, 0.0], 0.0);
    let meshes = entity_model_textured_meshes(&[base], &atlas);
    assert_eq!(meshes.submissions.len(), 1);
    assert_eq!(
        meshes.submissions[0].render_type,
        EntityModelLayerRenderType::EntityCutoutCull
    );
    assert_eq!(meshes.submissions[0].texture, BAT_TEXTURE_REF);
    assert_eq!(meshes.submissions[0].order, 0);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.cutout.cutout_faces, 54);
    assert_eq!(meshes.cutout.vertices.len(), 216);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    // The looping flap re-poses the wings as the age advances and repeats every 10 ticks.
    let later = entity_model_textured_meshes(&[base.with_age_in_ticks(3.0)], &atlas);
    assert_ne!(meshes.cutout.vertices, later.cutout.vertices);
    let one_cycle = entity_model_textured_meshes(&[base.with_age_in_ticks(10.0)], &atlas);
    assert_eq!(meshes.cutout.vertices, one_cycle.cutout.vertices);
}

#[test]
fn bat_resting_animation_matches_vanilla_definition() {
    // Vanilla `BatAnimation.BAT_RESTING` is a 0.5s looping animation over the same seven bones
    // as the flying flap, but every channel has a single keyframe (a static hanging pose).
    assert_eq!(BAT_RESTING.length_seconds, 0.5);
    assert!(BAT_RESTING.looping);
    assert_eq!(BAT_RESTING.bones.len(), 7);
}

#[test]
fn bat_hangs_upside_down_when_resting() {
    // While `isResting`, `BatModel.setupAnim` swaps to the static `BAT_RESTING` hanging pose
    // and `applyHeadRotation` turns the head by the look yaw. Colored path here, textured below.
    let base = EntityModelInstance::bat(930, [0.0, 64.0, 0.0], 0.0);
    let flying = entity_model_mesh(&[base]);
    let resting = entity_model_mesh(&[base.with_bat_resting(true)]);
    assert_eq!(flying.vertices.len(), resting.vertices.len());
    assert_ne!(
        flying.vertices, resting.vertices,
        "a resting bat hangs in a different pose"
    );

    // The resting pose is a single keyframe, so it holds still as the age advances, whereas a
    // flying bat keeps flapping.
    let resting_later = entity_model_mesh(&[base.with_bat_resting(true).with_age_in_ticks(3.0)]);
    assert_eq!(
        resting.vertices, resting_later.vertices,
        "the resting pose holds still"
    );
    let flying_later = entity_model_mesh(&[base.with_age_in_ticks(3.0)]);
    assert_ne!(
        flying.vertices, flying_later.vertices,
        "a flying bat keeps flapping"
    );

    // Only a resting bat applies the head look (`applyHeadRotation`); a flying bat ignores it.
    let resting_look = entity_model_mesh(&[base.with_bat_resting(true).with_head_look(60.0, 0.0)]);
    assert_ne!(
        resting.vertices, resting_look.vertices,
        "a resting bat turns its head to look"
    );
    let flying_look = entity_model_mesh(&[base.with_head_look(60.0, 0.0)]);
    assert_eq!(
        flying.vertices, flying_look.vertices,
        "a flying bat ignores the head look"
    );
}

#[test]
fn bat_textured_mesh_hangs_upside_down_when_resting() {
    let (atlas, _) = build_entity_model_texture_atlas(&bat_texture_images()).unwrap();
    let base = EntityModelInstance::bat(931, [0.0, 64.0, 0.0], 0.0);
    let flying = entity_model_textured_meshes(&[base], &atlas);
    let resting = entity_model_textured_meshes(&[base.with_bat_resting(true)], &atlas);
    assert_eq!(flying.cutout.vertices.len(), resting.cutout.vertices.len());
    assert_ne!(
        flying.cutout.vertices, resting.cutout.vertices,
        "a resting bat hangs in a different pose"
    );

    let resting_later = entity_model_textured_meshes(
        &[base.with_bat_resting(true).with_age_in_ticks(3.0)],
        &atlas,
    );
    assert_eq!(
        resting.cutout.vertices, resting_later.cutout.vertices,
        "the resting pose holds still"
    );

    let resting_look = entity_model_textured_meshes(
        &[base.with_bat_resting(true).with_head_look(60.0, 0.0)],
        &atlas,
    );
    assert_ne!(
        resting.cutout.vertices, resting_look.cutout.vertices,
        "a resting bat turns its head to look"
    );
}

fn bat_texture_images() -> Vec<EntityModelTextureImage> {
    bat_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
