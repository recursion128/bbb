use super::*;

use crate::entity_models::transforms::bed_model_root_transform;
use glam::Vec3;

fn bed_instance(position: [f32; 3], body_rot: f32, part: BedModelPart) -> EntityModelInstance {
    EntityModelInstance::bed(-1, position, body_rot, EntityDyeColor::Red, part)
}

#[test]
fn bed_transform_matches_vanilla_model_transform() {
    // Vanilla `BedRenderer.createModelTransform`: translation(0, 0.5625, 0) · Rx(90°) ·
    // rotateAround(Rz(180 + facing.toYRot()), 0.5, 0.5, 0.5). For SOUTH (toYRot = 0,
    // body_rot = 180) a model point (x, y, z) lands at (1-x, 0.5625-z, 1-y).
    let south = bed_model_root_transform(&bed_instance([2.0, 3.0, 4.0], 180.0, BedModelPart::Head));
    let corner = south.transform_point3(Vec3::ZERO);
    assert!((corner - Vec3::new(3.0, 3.5625, 5.0)).length() < 1e-5);
    // The mattress underside corner (0, 0, 0.375) rests at world y = 0.1875 over the block.
    let underside = south.transform_point3(Vec3::new(0.0, 0.0, 0.375));
    assert!((underside - Vec3::new(3.0, 3.1875, 5.0)).length() < 1e-5);
    // NORTH: toYRot = 180 -> body_rot = 360 (identity spin): (x, y, z) -> (x, 0.5625-z, y).
    let north = bed_model_root_transform(&bed_instance([0.0, 0.0, 0.0], 360.0, BedModelPart::Foot));
    assert!((north.transform_point3(Vec3::ZERO) - Vec3::new(0.0, 0.5625, 0.0)).length() < 1e-5);
    assert!(
        (north.transform_point3(Vec3::new(1.0, 1.0, 0.375)) - Vec3::new(1.0, 0.1875, 1.0)).length()
            < 1e-5
    );
    // WEST: toYRot = 90 -> body_rot = 270: (x, y, z) -> (y, 0.5625-z, 1-x).
    let west = bed_model_root_transform(&bed_instance([0.0, 0.0, 0.0], 270.0, BedModelPart::Head));
    assert!((west.transform_point3(Vec3::ZERO) - Vec3::new(0.0, 0.5625, 1.0)).length() < 1e-5);
    // EAST: toYRot = 270 -> body_rot = 450: (x, y, z) -> (1-y, 0.5625-z, x).
    let east = bed_model_root_transform(&bed_instance([0.0, 0.0, 0.0], 450.0, BedModelPart::Head));
    assert!((east.transform_point3(Vec3::ZERO) - Vec3::new(1.0, 0.5625, 0.0)).length() < 1e-5);
    // No `scale(-1, -1, 1)` entity flip: the transform is orientation-preserving.
    assert!(south.determinant() > 0.0);
}

#[test]
fn bed_texture_catalog_contains_all_sheets() {
    for texture in &texture_refs_matching(|path| path.starts_with("textures/entity/bed/")) {
        assert!(entity_model_texture_refs().contains(texture));
        assert_eq!(texture.size, [64, 64]);
    }
}

#[test]
fn bed_layer_passes_match_vanilla_renderer() {
    let passes = bed_textured_layer_passes(EntityDyeColor::Cyan);
    assert_eq!(passes.len(), 1);
    // Vanilla `BedRenderer` builds both piece models with `RenderTypes::entitySolid`.
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntitySolid
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entitySolid");
    assert_eq!(passes[0].texture, BED_CYAN_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
}

#[test]
fn bed_textured_mesh_bakes_visible_faces_into_the_solid_bucket() {
    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/bed/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = bed_instance([3.0, 4.0, 5.0], 180.0, BedModelPart::Head)
        .with(|s| s.light_coords = (4_u32 << 4) | (12_u32 << 20));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    // main + 2 legs, each with one vanilla-hidden face: 3 boxes × 5 faces = 15 faces /
    // 60 vertices / 90 indices, all in the backface-culled solid bucket.
    assert_eq!(meshes.solid.cutout_faces, 15);
    assert_eq!(meshes.solid.vertices.len(), 60);
    assert_eq!(meshes.solid.indices.len(), 90);
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes
        .solid
        .vertices
        .iter()
        .all(|vertex| vertex.light == instance.render_state.shader_light()));
}
