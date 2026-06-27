use super::*;

#[test]
fn tadpole_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `TadpoleModel.createBodyLayer` (atlas 16×16): two named sibling root parts — a 3×2×3
    // `body` box at offset (0, 22, -3) and a 0×2×7 `tail` fin plane at offset (0, 22, 0).
    assert_eq!(TADPOLE_BODY_POSE.offset, [0.0, 22.0, -3.0]);
    assert_eq!(TADPOLE_BODY_CUBES.len(), 1);
    assert_eq!(TADPOLE_BODY_CUBES[0].min, [-1.5, -1.0, 0.0]);
    assert_eq!(TADPOLE_BODY_CUBES[0].size, [3.0, 2.0, 3.0]);

    assert_eq!(TADPOLE_TAIL_POSE.offset, [0.0, 22.0, 0.0]);
    assert_eq!(TADPOLE_TAIL_CUBES.len(), 1);
    assert_eq!(TADPOLE_TAIL_CUBES[0].min, [0.0, -1.0, 0.0]);
    assert_eq!(TADPOLE_TAIL_CUBES[0].size, [0.0, 2.0, 7.0]);

    // Both parts sample texOffs(0, 0) (uv_size == size, not mirrored).
    assert_eq!(TADPOLE_BODY_CUBES[0].tex, [0.0, 0.0]);
    assert_eq!(TADPOLE_BODY_CUBES[0].uv_size, [3.0, 2.0, 3.0]);
    assert_eq!(TADPOLE_TAIL_CUBES[0].tex, [0.0, 0.0]);
    assert_eq!(TADPOLE_TAIL_CUBES[0].uv_size, [0.0, 2.0, 7.0]);
}

#[test]
fn tadpole_layer_passes_and_texture_ref_match_vanilla_renderer() {
    let passes = tadpole_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].kind, EntityModelLayerKind::TadpoleBase);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_TADPOLE);
    assert_eq!(passes[0].texture, TADPOLE_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));

    assert_eq!(
        EntityModelKind::Tadpole.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/tadpole/tadpole.png",
            size: [16, 16],
        })
    );
    assert!(entity_model_texture_refs().contains(&TADPOLE_TEXTURE_REF));
    assert_eq!(tadpole_entity_texture_refs(), &[TADPOLE_TEXTURE_REF]);
}

#[test]
fn tadpole_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let images: Vec<EntityModelTextureImage> = tadpole_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let base = EntityModelInstance::tadpole(640, [0.0, 64.0, 0.0], 0.0)
        .with_in_water(true)
        .with_light_coords((7_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let meshes = entity_model_textured_meshes(&[base], &atlas);
    assert_tadpole_base_submission(&meshes, base);

    // body box = 6 faces; the textured path emits all 6 faces of the 0-width tail box (the 4
    // degenerate side quads have zero area) → 12 cutout faces.
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.cutout.cutout_faces, 12);
    assert_eq!(meshes.cutout.vertices.len(), 48);

    // The textured path preserves the same animation split as the colored path: the body remains
    // stable while the tail sways with age, and the beached tail uses the larger amplitude.
    let swaying = entity_model_textured_meshes(&[base.with_age_in_ticks(5.0)], &atlas);
    assert_tadpole_base_submission(&swaying, base.with_age_in_ticks(5.0));
    assert_eq!(
        meshes.cutout.vertices[..24],
        swaying.cutout.vertices[..24],
        "the textured body stays put"
    );
    assert_ne!(
        meshes.cutout.vertices[24..],
        swaying.cutout.vertices[24..],
        "the textured tail fin sways with age"
    );

    let beached =
        entity_model_textured_meshes(&[base.with_in_water(false).with_age_in_ticks(5.0)], &atlas);
    assert_tadpole_base_submission(&beached, base.with_in_water(false).with_age_in_ticks(5.0));
    assert_ne!(beached.cutout.vertices[24..], swaying.cutout.vertices[24..]);
}

#[test]
fn tadpole_mesh_uses_vanilla_body_layer_geometry() {
    // The body box contributes 6 faces; the tail is a zero-width plane (front/back quads only). The
    // body carries the body tint and the tail carries its own fin tint.
    let tadpole = entity_model_mesh(&[EntityModelInstance::tadpole(640, [0.0, 64.0, 0.0], 0.0)]);
    assert!(tadpole
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(TADPOLE_BODY, 1.0)));
    assert!(tadpole
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(TADPOLE_TAIL, 1.0)));
}

#[test]
fn tadpole_tail_sway_matches_vanilla_setup_anim() {
    // `tail.yRot = -amplitude * 0.25 * sin(0.3 * ageInTicks)`, amplitude 1.0 in water / 1.5 out (a
    // beached tadpole thrashes harder). At age 0 the sway is zero regardless of amplitude.
    assert_eq!(tadpole_tail_yrot(0.0, true), 0.0);
    assert_eq!(tadpole_tail_yrot(0.0, false), 0.0);

    let age = 5.0_f32;
    let s = (0.3 * age).sin();
    assert!((tadpole_tail_yrot(age, true) - (-1.0 * 0.25 * s)).abs() < 1.0e-6);
    assert!((tadpole_tail_yrot(age, false) - (-1.5 * 0.25 * s)).abs() < 1.0e-6);
    assert!(tadpole_tail_yrot(age, false).abs() > tadpole_tail_yrot(age, true).abs());
}

#[test]
fn tadpole_swims_its_tail_with_age() {
    // A still tadpole (age 0) is at bind; advancing the age sways the `tail` fin (vertices [24, 48))
    // while the `body` box (vertices [0, 24)) stays put.
    let base = EntityModelInstance::tadpole(641, [0.0, 64.0, 0.0], 0.0).with_in_water(true);
    let rest = entity_model_mesh(&[base]);
    let swaying = entity_model_mesh(&[base.with_age_in_ticks(5.0)]);
    assert_eq!(rest.vertices.len(), swaying.vertices.len());
    assert_eq!(
        rest.vertices[..24],
        swaying.vertices[..24],
        "the body stays put"
    );
    assert_ne!(
        rest.vertices[24..],
        swaying.vertices[24..],
        "the tail fin sways with the age"
    );

    // A beached tadpole thrashes harder, so its tail differs from the in-water sway at the same age.
    let beached = entity_model_mesh(&[base.with_in_water(false).with_age_in_ticks(5.0)]);
    assert_ne!(beached.vertices[24..], swaying.vertices[24..]);
}

fn assert_tadpole_base_submission(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.texture, TADPOLE_TEXTURE_REF);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.transform, entity_model_root_transform(instance));
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    assert_eq!(submit.light, instance.render_state.shader_light());
    assert_eq!(submit.overlay, instance.render_state.overlay_coords());
    assert_ne!(submit.overlay, [0.0, 10.0]);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == submit.light && vertex.overlay == submit.overlay));
}
