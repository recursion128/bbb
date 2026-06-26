use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn trident_cubes_match_vanilla_26_1_layer() {
    // Vanilla `TridentModel.createLayer` (atlas 32×32): `pole` (texOffs 0,6) parents `base`
    // (texOffs 4,0) and three 1×4×1 spikes — left (texOffs 4,3), middle (texOffs 0,0), right
    // (texOffs 4,3 mirrored). Each unified cube carries the colored tint and the textured UV.
    assert_eq!(
        TRIDENT_POLE_CUBE,
        ModelCube::new(
            [-0.5, 2.0, -0.5],
            [1.0, 25.0, 1.0],
            TRIDENT_POLE,
            [1.0, 25.0, 1.0],
            [0.0, 6.0],
            false
        )
    );
    assert_eq!(
        TRIDENT_BASE_CUBE,
        ModelCube::new(
            [-1.5, 0.0, -0.5],
            [3.0, 2.0, 1.0],
            TRIDENT_POLE,
            [3.0, 2.0, 1.0],
            [4.0, 0.0],
            false
        )
    );
    assert_eq!(
        TRIDENT_LEFT_SPIKE_CUBE,
        ModelCube::new(
            [-2.5, -3.0, -0.5],
            [1.0, 4.0, 1.0],
            TRIDENT_SPIKE,
            [1.0, 4.0, 1.0],
            [4.0, 3.0],
            false
        )
    );
    assert_eq!(
        TRIDENT_MIDDLE_SPIKE_CUBE,
        ModelCube::new(
            [-0.5, -4.0, -0.5],
            [1.0, 4.0, 1.0],
            TRIDENT_SPIKE,
            [1.0, 4.0, 1.0],
            [0.0, 0.0],
            false
        )
    );
    // The right spike samples the left spike's atlas region, mirrored.
    assert_eq!(
        TRIDENT_RIGHT_SPIKE_CUBE,
        ModelCube::new(
            [1.5, -3.0, -0.5],
            [1.0, 4.0, 1.0],
            TRIDENT_SPIKE,
            [1.0, 4.0, 1.0],
            [4.0, 3.0],
            true
        )
    );
    assert_eq!(TRIDENT_TEXTURE_REF.size, [32, 32]);
}

#[test]
fn trident_mesh_uses_vanilla_body_layer_geometry() {
    // 5 cubes → 30 faces / 120 vertices / 180 indices; the pole and the spikes carry their tints.
    let trident = entity_model_mesh(&[EntityModelInstance::trident(1350, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(trident.opaque_faces, 30);
    assert_eq!(trident.vertices.len(), 120);
    assert_eq!(trident.indices.len(), 180);
    assert!(trident
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(TRIDENT_POLE, 1.0)));
    assert!(trident
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(TRIDENT_SPIKE, 1.0)));
}

#[test]
fn trident_layer_passes_and_texture_ref_match_vanilla_renderer() {
    let passes = trident_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].texture, TRIDENT_TEXTURE_REF);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);

    assert_eq!(
        EntityModelKind::Trident.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/trident/trident.png",
            size: [32, 32],
        })
    );
    assert!(entity_model_texture_refs().contains(&TRIDENT_TEXTURE_REF));
    assert_eq!(trident_entity_texture_refs(), &[TRIDENT_TEXTURE_REF]);
}

#[test]
fn trident_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let images: Vec<EntityModelTextureImage> = trident_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::trident(1350, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert_eq!(mesh.cutout_faces, 30);
    assert_eq!(mesh.vertices.len(), 120);
    assert_eq!(mesh.indices.len(), 180);
}
