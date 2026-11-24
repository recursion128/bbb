use super::*;

#[test]
fn phantom_layer_passes_match_vanilla_renderer() {
    // Vanilla PhantomRenderer adds a PhantomEyesLayer, so the model emits a cutout base pass
    // plus an emissive eyes pass over the same geometry.
    let passes = phantom_textured_layer_passes();
    assert_eq!(passes.len(), 2);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].texture, PHANTOM_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);

    assert_eq!(passes[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(passes[1].texture, PHANTOM_EYES_TEXTURE_REF);
    assert_eq!(passes[1].order, 1);
}

#[test]
fn phantom_texture_ref_matches_vanilla_renderer() {
    assert!(entity_model_texture_refs().contains(&PHANTOM_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&PHANTOM_EYES_TEXTURE_REF));
}

#[test]
fn phantom_flap_time_matches_vanilla_unique_offset() {
    // Vanilla flapTime = getUniqueFlapTickOffset() + ageInTicks, getUniqueFlapTickOffset() =
    // id * 3 (Java int multiply).
    assert_eq!(phantom_flap_time(0, 0.0), 0.0);
    assert_eq!(phantom_flap_time(7, 0.0), 21.0);
    assert_eq!(phantom_flap_time(7, 4.5), 25.5);
    assert_eq!(phantom_flap_time(100, 1.0), 301.0);
}

#[test]
fn phantom_wing_and_tail_match_vanilla_setup_anim() {
    // Vanilla: anim = flapTime * 7.448451 * pi/180.
    //   wing zRot = cos(anim) * 16 deg ; right wing = -that.
    //   tail xRot = -(5 deg + cos(2*anim) * 5 deg).
    let deg = |d: f32| d.to_radians();
    for flap in [0.0f32, 21.0, 25.5, 137.0] {
        let anim = (flap * 7.448451).to_radians();
        let want_wing = anim.cos() * deg(16.0);
        let want_tail = -(deg(5.0) + (anim * 2.0).cos() * deg(5.0));
        assert!(
            (phantom_wing_z_rot(flap) - want_wing).abs() < 1e-7,
            "wing at flap {flap}"
        );
        assert!(
            (phantom_tail_x_rot(flap) - want_tail).abs() < 1e-7,
            "tail at flap {flap}"
        );
    }
    // The wing/tail never sit at the layer rest: at flap 0 the wing is cos(0)*16deg and the
    // tail carries the -10deg baseline (cos(0) term).
    assert!((phantom_wing_z_rot(0.0) - 16.0_f32.to_radians()).abs() < 1e-7);
    assert!((phantom_tail_x_rot(0.0) - (-10.0_f32.to_radians())).abs() < 1e-7);
}

#[test]
fn phantom_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&phantom_texture_images()).unwrap();
    let instance = EntityModelInstance::phantom(99, [0.0, 64.0, 0.0], 0.0, 0).with(|s| {
        s.light_coords = (6_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.translucent.vertices.is_empty());
    assert_eq!(meshes.cutout.cutout_faces, 48);
    assert_eq!(meshes.cutout.vertices.len(), 192);
    assert_eq!(meshes.cutout.indices.len(), 288);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
    assert!(meshes.eyes.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == [0.0, 10.0]));
    assert_ne!(instance.render_state.overlay_coords(), [0.0, 10.0]);
    let (min, max) = textured_mesh_extents(&meshes.cutout);
    assert_close3(min, [-1.3223567, 63.913067, -0.991909]);
    assert_close3(max, [1.2598567, 64.36279, 0.577472]);
}

#[test]
fn phantom_eyes_textured_mesh_uses_parent_model_geometry_and_eyes_render_type() {
    // Vanilla PhantomEyesLayer re-renders the whole model with phantom_eyes.png in the eyes
    // render type, so the eyes mesh shares the base geometry (8 cubes) and lands in the eyes
    // pass.
    let (atlas, _) = build_entity_model_texture_atlas(&phantom_texture_images()).unwrap();
    let meshes = entity_model_textured_meshes(
        &[EntityModelInstance::phantom(99, [0.0, 64.0, 0.0], 0.0, 0)],
        &atlas,
    );

    assert_eq!(meshes.cutout.cutout_faces, 48);
    assert_eq!(meshes.eyes.cutout_faces, 48);
    assert_eq!(meshes.eyes.vertices.len(), 192);
    assert!(meshes
        .eyes
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert_eq!(
        textured_mesh_extents(&meshes.eyes),
        textured_mesh_extents(&meshes.cutout)
    );
}

#[test]
fn phantom_eyes_missing_texture_preserves_other_geometry() {
    let images: Vec<_> = phantom_texture_images()
        .into_iter()
        .filter(|image| image.texture != PHANTOM_EYES_TEXTURE_REF)
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::phantom(311, [0.0, 64.0, 0.0], 0.0, 2).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.65;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(!meshes.cutout.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.eyes.indices.is_empty());
}

#[test]
fn phantom_wings_flap_as_age_in_ticks_advances() {
    // Vanilla runs setupAnim every frame: the wings and tail flap by flapTime while the body
    // and head hold still. The body is the first cube (vertices [0, 24)).
    let base = EntityModelInstance::phantom(99, [0.0, 64.0, 0.0], 0.0, 0);
    let early = entity_model_mesh(&[base]);
    let later = entity_model_mesh(&[base.with(|s| s.age_in_ticks = 11.0)]);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(
        early.vertices, later.vertices,
        "the wings flap as ageInTicks advances"
    );
    assert_eq!(
        early.vertices[..24],
        later.vertices[..24],
        "the body does not depend on ageInTicks"
    );

    let (atlas, _) = build_entity_model_texture_atlas(&phantom_texture_images()).unwrap();
    let early_t = entity_model_textured_meshes(&[base], &atlas);

    let later_instance = base.with(|s| s.age_in_ticks = 11.0);
    let later_t = entity_model_textured_meshes(&[later_instance], &atlas);

    assert_ne!(
        early_t.cutout.vertices, later_t.cutout.vertices,
        "the textured wings flap too"
    );
    assert_ne!(
        early_t.eyes.vertices, later_t.eyes.vertices,
        "the textured eyes overlay flaps too"
    );
}

fn phantom_texture_images() -> Vec<EntityModelTextureImage> {
    [PHANTOM_TEXTURE_REF, PHANTOM_EYES_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
