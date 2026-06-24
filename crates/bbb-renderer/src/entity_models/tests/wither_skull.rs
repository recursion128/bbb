use super::*;

#[test]
fn wither_skull_geometry_matches_vanilla_26_1_skull_layer() {
    // Vanilla `WitherSkullRenderer.createSkullLayer` (atlas 64×64): one `head` part at ZERO with a
    // single 8×8×8 box (`addBox(-4, -8, -4, 8, 8, 8)` at `texOffs(0, 35)`).
    assert_eq!(WITHER_SKULL_CUBE.min, [-4.0, -8.0, -4.0]);
    assert_eq!(WITHER_SKULL_CUBE.size, [8.0, 8.0, 8.0]);
    assert_eq!(WITHER_SKULL_CUBE.uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(WITHER_SKULL_CUBE.tex, [0.0, 35.0]);
    assert!(!WITHER_SKULL_CUBE.mirror);
}

#[test]
fn wither_skull_mesh_uses_vanilla_skull_layer_geometry() {
    // 1 cube → 6 faces / 24 vertices / 36 indices, one dark tint.
    let skull = entity_model_mesh(&[EntityModelInstance::wither_skull(
        820,
        [0.0, 64.0, 0.0],
        0.0,
    )]);
    assert_eq!(skull.opaque_faces, 6);
    assert_eq!(skull.vertices.len(), 24);
    assert_eq!(skull.indices.len(), 36);
    assert!(skull
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WITHER_SKULL_GRAY, 1.0)));
}

#[test]
fn wither_skull_colored_runtime_skips_the_texture_backed_skull() {
    // The wither skull now binds the wither texture, so it renders through the textured path. The
    // texture-skipping colored runtime path emits nothing for it, while the full path still emits the
    // colored fallback geometry. (The wither_invulnerable charged swap stays deferred.)
    let instances = [EntityModelInstance::wither_skull(
        821,
        [0.0, 64.0, 0.0],
        0.0,
    )];
    assert!(!entity_model_mesh(&instances).vertices.is_empty());
    assert!(entity_model_colored_runtime_mesh(&instances)
        .vertices
        .is_empty());
}

#[test]
fn wither_skull_orients_along_flight() {
    // `WitherSkullRenderer` flips the skull (`scale(-1, -1, 1)`) and `SkullModel.setupAnim` turns it by
    // the flight `yRot`/`xRot`, so changing either the yaw (`body_rot`) or the pitch (`head_pitch`)
    // re-poses the whole skull.
    let base = EntityModelInstance::wither_skull(822, [0.0, 64.0, 0.0], 0.0);
    let yawed = EntityModelInstance::wither_skull(822, [0.0, 64.0, 0.0], 90.0);
    let pitched = base.with_head_look(0.0, 45.0);

    let base_mesh = entity_model_mesh(&[base]);
    let yawed_mesh = entity_model_mesh(&[yawed]);
    let pitched_mesh = entity_model_mesh(&[pitched]);
    assert_eq!(base_mesh.vertices.len(), yawed_mesh.vertices.len());
    assert_ne!(
        base_mesh.vertices, yawed_mesh.vertices,
        "the yaw orients the skull"
    );
    assert_ne!(
        base_mesh.vertices, pitched_mesh.vertices,
        "the pitch orients the skull"
    );
}

#[test]
fn wither_skull_textured_render_matches_vanilla_renderer() {
    assert_eq!(
        wither_skull_textured_layer_passes()[0].texture,
        WITHER_TEXTURE_REF
    );
    assert_eq!(
        EntityModelKind::WitherSkull.vanilla_texture_ref(),
        Some(WITHER_TEXTURE_REF)
    );
    assert!(entity_model_texture_refs().contains(&WITHER_TEXTURE_REF));
    assert_eq!(wither_skull_entity_texture_refs(), &[WITHER_TEXTURE_REF]);

    let len = usize::try_from(WITHER_TEXTURE_REF.size[0] * WITHER_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        WITHER_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::wither_skull(
            820,
            [0.0, 64.0, 0.0],
            0.0,
        )],
        &atlas,
    );
    assert!(!mesh.vertices.is_empty());
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn wither_skull_exposes_stable_model_key() {
    assert_eq!(EntityModelKind::WitherSkull.model_key(), "wither_skull");
}
