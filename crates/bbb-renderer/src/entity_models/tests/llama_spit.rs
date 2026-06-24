use super::*;

#[test]
fn llama_spit_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `LlamaSpitModel.createBodyLayer` (atlas 64×32): one `main` part at ZERO with seven
    // 2×2×2 boxes forming a cross — a centre cube and one neighbour stepping out along each axis.
    assert_eq!(LLAMA_SPIT_CUBES.len(), 7);

    // The exact seven `addBox` origins, all 2×2×2, all at the shared `texOffs(0, 0)`, no mirror.
    let origins: Vec<[f32; 3]> = LLAMA_SPIT_CUBES.iter().map(|cube| cube.min).collect();
    assert_eq!(
        origins,
        vec![
            [-4.0, 0.0, 0.0],
            [0.0, -4.0, 0.0],
            [0.0, 0.0, -4.0],
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [0.0, 0.0, 2.0],
        ]
    );
    assert!(LLAMA_SPIT_CUBES
        .iter()
        .all(|cube| cube.size == [2.0, 2.0, 2.0]));
    assert!(LLAMA_SPIT_CUBES
        .iter()
        .all(|cube| cube.uv_size == [2.0, 2.0, 2.0]));
    assert!(LLAMA_SPIT_CUBES.iter().all(|cube| cube.tex == [0.0, 0.0]));
    assert!(LLAMA_SPIT_CUBES.iter().all(|cube| !cube.mirror));
}

#[test]
fn llama_spit_mesh_uses_vanilla_body_layer_geometry() {
    // 7 cubes → 42 faces / 168 vertices / 252 indices; the cross carries its single tint.
    let spit = entity_model_mesh(&[EntityModelInstance::llama_spit(790, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(spit.opaque_faces, 42);
    assert_eq!(spit.vertices.len(), 168);
    assert_eq!(spit.indices.len(), 252);
    assert!(spit
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(LLAMA_SPIT_COLOR, 1.0)));
}

#[test]
fn llama_spit_orients_along_flight() {
    // `LlamaSpitRenderer` orients the spit by `Ry(yRot - 90)` then `Rz(xRot)`, so changing either
    // the yaw (`body_rot`) or the pitch (`head_pitch`) re-poses the whole cross.
    let base = EntityModelInstance::llama_spit(791, [0.0, 64.0, 0.0], 0.0);
    let yawed = EntityModelInstance::llama_spit(791, [0.0, 64.0, 0.0], 90.0);
    let pitched = base.with_head_look(0.0, 45.0);

    let base_mesh = entity_model_mesh(&[base]);
    let yawed_mesh = entity_model_mesh(&[yawed]);
    let pitched_mesh = entity_model_mesh(&[pitched]);
    assert_eq!(base_mesh.vertices.len(), yawed_mesh.vertices.len());
    assert_ne!(
        base_mesh.vertices, yawed_mesh.vertices,
        "the yaw orients the spit"
    );
    assert_ne!(
        base_mesh.vertices, pitched_mesh.vertices,
        "the pitch orients the spit"
    );
}

#[test]
fn llama_spit_textured_render_matches_vanilla_renderer() {
    assert_eq!(
        llama_spit_textured_layer_passes()[0].texture,
        LLAMA_SPIT_TEXTURE_REF
    );
    assert_eq!(
        EntityModelKind::LlamaSpit.vanilla_texture_ref(),
        Some(LLAMA_SPIT_TEXTURE_REF)
    );
    assert!(entity_model_texture_refs().contains(&LLAMA_SPIT_TEXTURE_REF));
    assert_eq!(llama_spit_entity_texture_refs(), &[LLAMA_SPIT_TEXTURE_REF]);

    let len = usize::try_from(LLAMA_SPIT_TEXTURE_REF.size[0] * LLAMA_SPIT_TEXTURE_REF.size[1] * 4)
        .unwrap();
    let images = vec![EntityModelTextureImage::new(
        LLAMA_SPIT_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::llama_spit(790, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert!(!mesh.vertices.is_empty());
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}
