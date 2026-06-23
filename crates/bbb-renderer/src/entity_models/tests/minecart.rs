use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn minecart_cubes_match_vanilla_26_1_body_layer() {
    // Vanilla MinecartModel.createBodyLayer(): a 20x16x2 floor panel laid flat plus four
    // 16x8x2 wall panels boxed in. No setupAnim, so the cart is static. Each unified cube carries
    // the colored tint (`MINECART_GRAY`) and the textured UV (`texOffs` / `uv_size` / `mirror`) in
    // one struct.
    //
    // bottom: texOffs(0, 10), addBox(-10, -8, -1, 20, 16, 2). The floor samples texOffs(0, 10).
    assert_eq!(
        MINECART_BOTTOM[0],
        ModelCube::new(
            [-10.0, -8.0, -1.0],
            [20.0, 16.0, 2.0],
            MINECART_GRAY,
            [20.0, 16.0, 2.0],
            [0.0, 10.0],
            false,
        )
    );
    // The four walls share one texOffs(0, 0) box(-8, -9, -1, 16x8x2), not mirrored.
    assert_eq!(
        MINECART_WALL[0],
        ModelCube::new(
            [-8.0, -9.0, -1.0],
            [16.0, 8.0, 2.0],
            MINECART_GRAY,
            [16.0, 8.0, 2.0],
            [0.0, 0.0],
            false,
        )
    );
}

#[test]
fn minecart_layer_passes_match_vanilla_renderer() {
    let passes = minecart_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::MinecartBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_MINECART);
    assert_eq!(passes[0].texture, MINECART_TEXTURE_REF);
    assert!(passes[0].parts.is_empty());
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
}

#[test]
fn minecart_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Minecart.model_key(), "minecart");
    assert_eq!(
        EntityModelKind::Minecart.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/minecart/minecart.png",
            size: [64, 32],
        })
    );
    assert!(entity_model_texture_refs().contains(&MINECART_TEXTURE_REF));
    assert_eq!(minecart_entity_texture_refs(), &[MINECART_TEXTURE_REF]);
}

#[test]
fn minecart_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::minecart(1, [0.0, 64.0, 0.0], 0.0)]);
    // Five cubes => 30 faces, 120 verts, 180 indices.
    assert_eq!(mesh.opaque_faces, 30);
    assert_eq!(mesh.vertices.len(), 120);
    assert_eq!(mesh.indices.len(), 180);
}

#[test]
fn minecart_textured_mesh_matches_colored_geometry_and_vanilla_uvs() {
    let (atlas, _) = build_entity_model_texture_atlas(&minecart_texture_images()).unwrap();
    let textured = entity_model_textured_mesh(
        &[EntityModelInstance::minecart(1, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert_eq!(textured.cutout_faces, 30);
    assert_eq!(textured.vertices.len(), 120);
    assert_eq!(textured.indices.len(), 180);
    assert!(textured
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    // The textured cart shares the colored cart's geometry exactly (same parts and transform).
    let colored = entity_model_mesh(&[EntityModelInstance::minecart(1, [0.0, 64.0, 0.0], 0.0)]);
    let (cmin, cmax) = mesh_extents(&colored);
    let (tmin, tmax) = textured_mesh_extents(&textured);
    assert_close3(tmin, cmin);
    assert_close3(tmax, cmax);
}

fn minecart_texture_images() -> Vec<EntityModelTextureImage> {
    minecart_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
