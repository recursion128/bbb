use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn leash_knot_cube_matches_vanilla_26_1_body_layer() {
    // Vanilla `LeashKnotModel.createBodyLayer` (atlas 32×32): a single `knot` part at ZERO with one
    // 6×8×6 box at texOffs(0, 0), no deformation, not mirrored. The unified cube carries the colored
    // tint (`LEASH_KNOT_COLOR`) and the textured UV (`uv_size == size`).
    assert_eq!(
        LEASH_KNOT_KNOT_CUBE,
        ModelCube::new(
            [-3.0, -8.0, -3.0],
            [6.0, 8.0, 6.0],
            LEASH_KNOT_COLOR,
            [6.0, 8.0, 6.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(LEASH_KNOT_TEXTURE_REF.size, [32, 32]);
}

#[test]
fn leash_knot_mesh_uses_vanilla_body_layer_geometry() {
    // 1 cube → 6 faces / 24 vertices / 36 indices, carrying the knot tint.
    let knot = entity_model_mesh(&[EntityModelInstance::leash_knot(760, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(knot.opaque_faces, 6);
    assert_eq!(knot.vertices.len(), 24);
    assert_eq!(knot.indices.len(), 36);
    assert!(knot
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(LEASH_KNOT_COLOR, 1.0)));
}

#[test]
fn leash_knot_layer_passes_match_vanilla_renderer() {
    let passes = leash_knot_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].texture, LEASH_KNOT_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn leash_knot_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::LeashKnot.model_key(), "leash_knot");
    assert_eq!(
        EntityModelKind::LeashKnot.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/lead_knot/lead_knot.png",
            size: [32, 32],
        })
    );
    assert!(entity_model_texture_refs().contains(&LEASH_KNOT_TEXTURE_REF));
    assert_eq!(leash_knot_entity_texture_refs(), &[LEASH_KNOT_TEXTURE_REF]);
}

#[test]
fn leash_knot_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let images: Vec<EntityModelTextureImage> = leash_knot_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::leash_knot(760, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert_eq!(mesh.cutout_faces, 6);
    assert_eq!(mesh.vertices.len(), 24);
    assert_eq!(mesh.indices.len(), 36);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}
