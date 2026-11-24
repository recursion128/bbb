use super::*;

#[test]
fn bat_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&bat_texture_images()).unwrap();

    // Bat renders into the cutout mesh (vanilla `RenderTypes::entityCutoutCull`). Nine cubes →
    // 54 faces / 216 vertices, with nothing on the translucent or eyes passes and a white tint.
    let base = EntityModelInstance::bat(920, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.age_in_ticks = 37.0;
        s.kind.bat_mut().flying_seconds = 0.0;
    });
    let meshes = entity_model_textured_meshes(&[base], &atlas);
    assert_bat_base_submission(&meshes, base);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.cutout.vertices.is_empty());
    assert_eq!(meshes.cutout_cull.cutout_faces, 54);
    assert_eq!(meshes.cutout_cull.vertices.len(), 216);
    assert!(meshes
        .cutout_cull
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    // The looping flap re-poses from relative AnimationState time and repeats every 10 ticks.
    let later_instance = base.with(|s| s.kind.bat_mut().flying_seconds = 0.15);
    let later = entity_model_textured_meshes(&[later_instance], &atlas);
    assert_bat_base_submission(&later, later_instance);
    assert_ne!(meshes.cutout_cull.vertices, later.cutout_cull.vertices);
    let cycle_instance = base.with(|s| s.kind.bat_mut().flying_seconds = 0.5);
    let one_cycle = entity_model_textured_meshes(&[cycle_instance], &atlas);
    assert_bat_base_submission(&one_cycle, cycle_instance);
    assert_eq!(meshes.cutout_cull.vertices, one_cycle.cutout_cull.vertices);
}

#[test]
fn bat_hangs_upside_down_when_resting() {
    // While `isResting`, `BatModel.setupAnim` swaps to the static `BAT_RESTING` hanging pose
    // and `applyHeadRotation` turns the head by the look yaw.
    let base = EntityModelInstance::bat(930, [0.0, 64.0, 0.0], 0.0)
        .with(|s| s.kind.bat_mut().flying_seconds = 0.0);
    let flying = entity_model_mesh(&[base]);
    let resting = entity_model_mesh(&[base.with(|s| s.kind.bat_mut().resting = true)]);
    assert_eq!(flying.vertices.len(), resting.vertices.len());
    assert_ne!(
        flying.vertices, resting.vertices,
        "a resting bat hangs in a different pose"
    );

    // The resting pose is a single keyframe, so it holds still as the age advances, whereas a
    // flying bat keeps flapping.
    let resting_later = entity_model_mesh(&[base.with(|s| {
        s.kind.bat_mut().resting = true;
        s.age_in_ticks = 3.0;
    })]);
    assert_eq!(
        resting.vertices, resting_later.vertices,
        "the resting pose holds still"
    );
    let flying_later = entity_model_mesh(&[base.with(|s| s.kind.bat_mut().flying_seconds = 0.15)]);
    assert_ne!(
        flying.vertices, flying_later.vertices,
        "a flying bat keeps flapping"
    );

    // Only a resting bat applies the head look (`applyHeadRotation`); a flying bat ignores it.
    let resting_look = entity_model_mesh(&[base
        .with(|s| s.kind.bat_mut().resting = true)
        .with(|s| (s.head_yaw, s.head_pitch) = (60.0, 0.0))]);
    assert_ne!(
        resting.vertices, resting_look.vertices,
        "a resting bat turns its head to look"
    );
    let flying_look = entity_model_mesh(&[base.with(|s| (s.head_yaw, s.head_pitch) = (60.0, 0.0))]);
    assert_eq!(
        flying.vertices, flying_look.vertices,
        "a flying bat ignores the head look"
    );
}

#[test]
fn bat_textured_mesh_hangs_upside_down_when_resting() {
    let (atlas, _) = build_entity_model_texture_atlas(&bat_texture_images()).unwrap();
    let base = EntityModelInstance::bat(931, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.bat_mut().flying_seconds = 0.0;
    });
    let flying = entity_model_textured_meshes(&[base], &atlas);
    assert_bat_base_submission(&flying, base);
    let resting_instance = base.with(|s| s.kind.bat_mut().resting = true);
    let resting = entity_model_textured_meshes(&[resting_instance], &atlas);
    assert_bat_base_submission(&resting, resting_instance);
    assert_eq!(
        flying.cutout_cull.vertices.len(),
        resting.cutout_cull.vertices.len()
    );
    assert_ne!(
        flying.cutout_cull.vertices, resting.cutout_cull.vertices,
        "a resting bat hangs in a different pose"
    );

    let resting_later = entity_model_textured_meshes(
        &[base.with(|s| {
            s.kind.bat_mut().resting = true;
            s.age_in_ticks = 3.0;
        })],
        &atlas,
    );
    assert_bat_base_submission(
        &resting_later,
        resting_instance.with(|s| s.age_in_ticks = 3.0),
    );
    assert_eq!(
        resting.cutout_cull.vertices, resting_later.cutout_cull.vertices,
        "the resting pose holds still"
    );

    let resting_look = entity_model_textured_meshes(
        &[base
            .with(|s| s.kind.bat_mut().resting = true)
            .with(|s| (s.head_yaw, s.head_pitch) = (60.0, 0.0))],
        &atlas,
    );
    assert_bat_base_submission(
        &resting_look,
        resting_instance.with(|s| (s.head_yaw, s.head_pitch) = (60.0, 0.0)),
    );
    assert_ne!(
        resting.cutout_cull.vertices, resting_look.cutout_cull.vertices,
        "a resting bat turns its head to look"
    );
}

fn bat_texture_images() -> Vec<EntityModelTextureImage> {
    [BAT_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn assert_bat_base_submission(meshes: &EntityModelTexturedMeshes, instance: EntityModelInstance) {
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
}
