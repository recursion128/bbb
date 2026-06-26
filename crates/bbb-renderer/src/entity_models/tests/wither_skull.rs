use super::*;

use crate::entity_models::colored::wither_skull_model_root_transform;

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
    // colored fallback geometry.
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
    let normal_pass = wither_skull_textured_layer_passes(false)[0];
    assert_eq!(normal_pass.texture, WITHER_TEXTURE_REF);
    assert_eq!(
        normal_pass.render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(normal_pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((normal_pass.order, normal_pass.submit_sequence), (0, 0));
    let dangerous_pass = wither_skull_textured_layer_passes(true)[0];
    assert_eq!(dangerous_pass.texture, WITHER_INVULNERABLE_TEXTURE_REF);
    assert_eq!(
        dangerous_pass.render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(dangerous_pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (dangerous_pass.order, dangerous_pass.submit_sequence),
        (0, 0)
    );
    assert_eq!(
        EntityModelKind::WitherSkull { dangerous: false }.vanilla_texture_ref(),
        Some(WITHER_TEXTURE_REF)
    );
    assert_eq!(
        EntityModelKind::WitherSkull { dangerous: true }.vanilla_texture_ref(),
        Some(WITHER_INVULNERABLE_TEXTURE_REF)
    );
    assert!(entity_model_texture_refs().contains(&WITHER_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&WITHER_INVULNERABLE_TEXTURE_REF));
    assert_eq!(
        wither_skull_entity_texture_refs(),
        &[WITHER_TEXTURE_REF, WITHER_INVULNERABLE_TEXTURE_REF]
    );

    let images: Vec<EntityModelTextureImage> = wither_skull_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let normal =
        EntityModelInstance::wither_skull(820, [1.0, 64.0, -2.0], 37.0).with_head_look(0.0, -18.0);
    let dangerous =
        EntityModelInstance::wither_skull_with_dangerous(820, [1.0, 64.0, -2.0], 37.0, true)
            .with_head_look(0.0, -18.0);
    let normal_meshes = entity_model_textured_meshes(&[normal], &atlas);
    let dangerous_meshes = entity_model_textured_meshes(&[dangerous], &atlas);
    assert!(normal_meshes.cutout.vertices.is_empty());
    assert_eq!(normal_meshes.submissions.len(), 1);
    let normal_submit = normal_meshes.submissions[0];
    assert_eq!(
        normal_submit.render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(
        normal_submit.render_type.vanilla_name(),
        "entityTranslucent"
    );
    assert_eq!(normal_submit.texture, WITHER_TEXTURE_REF);
    assert_eq!(normal_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((normal_submit.order, normal_submit.submit_sequence), (0, 0));
    assert_eq!(
        normal_submit.transform,
        wither_skull_model_root_transform(normal)
    );
    assert_eq!(dangerous_meshes.submissions.len(), 1);
    let dangerous_submit = dangerous_meshes.submissions[0];
    assert_eq!(
        dangerous_submit.render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(dangerous_submit.texture, WITHER_INVULNERABLE_TEXTURE_REF);
    assert_eq!(dangerous_submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (dangerous_submit.order, dangerous_submit.submit_sequence),
        (0, 0)
    );
    assert_eq!(
        dangerous_submit.transform,
        wither_skull_model_root_transform(dangerous)
    );
    let normal_mesh = &normal_meshes.translucent;
    let dangerous_mesh = &dangerous_meshes.translucent;
    assert!(!normal_mesh.vertices.is_empty());
    assert_eq!(normal_mesh.vertices.len(), dangerous_mesh.vertices.len());
    assert_ne!(
        normal_mesh.vertices, dangerous_mesh.vertices,
        "dangerous skulls select the invulnerable texture atlas entry"
    );
    assert!(normal_mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(dangerous_mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn wither_skull_exposes_stable_model_key() {
    assert_eq!(
        EntityModelKind::WitherSkull { dangerous: false }.model_key(),
        "wither_skull"
    );
    assert_eq!(
        EntityModelKind::WitherSkull { dangerous: true }.model_key(),
        "wither_skull"
    );
}
