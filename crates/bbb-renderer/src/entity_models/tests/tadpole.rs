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
    assert_eq!(passes[0].texture, TADPOLE_TEXTURE_REF);

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
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::tadpole(640, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    // body box = 6 faces; the textured path emits all 6 faces of the 0-width tail box (the 4
    // degenerate side quads have zero area) → 12 cutout faces.
    assert_eq!(mesh.cutout_faces, 12);
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
