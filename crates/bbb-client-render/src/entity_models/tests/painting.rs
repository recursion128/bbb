use super::*;

use crate::DynamicEntityTextureImage;

fn uv(rect: EntityModelUvRect, u: f32, v: f32) -> [f32; 2] {
    [
        rect.min[0] + u * (rect.max[0] - rect.min[0]),
        rect.min[1] + v * (rect.max[1] - rect.min[1]),
    ]
}

#[test]
fn painting_tiles_emit_vanilla_partitioned_faces_and_per_tile_light() {
    let front = EntityDynamicTexture { handle: 71_001 };
    let back = EntityDynamicTexture { handle: 71_002 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[
        DynamicEntityTextureImage {
            handle: front.handle,
            size: [32, 16],
            standalone_sampling: None,
            rgba: vec![0xff; 32 * 16 * 4],
        },
        DynamicEntityTextureImage {
            handle: back.handle,
            size: [16, 16],
            standalone_sampling: None,
            rgba: vec![0x7f; 16 * 16 * 4],
        },
    ])
    .unwrap()
    .0;
    let atlas = EntityModelTextureAtlasLayout {
        width: 1,
        height: 1,
        entries: Vec::new(),
    };
    let instances = [
        EntityModelInstance::painting_tile(
            11,
            [10.0, 20.0, 30.0],
            2,
            1,
            0,
            0,
            EntityAttachmentFace::North,
            front,
            back,
        )
        .with(|s| s.light_coords = (3 << 4) | (10 << 20)),
        EntityModelInstance::painting_tile(
            11,
            [10.0, 20.0, 30.0],
            2,
            1,
            1,
            0,
            EntityAttachmentFace::North,
            front,
            back,
        )
        .with(|s| s.light_coords = (12 << 4) | (5 << 20)),
    ];

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &instances,
        &atlas,
        None,
        Some(&dynamic_atlas),
    );
    let mesh = &meshes.dynamic_player_texture_solid_z_offset_forward;

    assert_eq!(mesh.cutout_faces, 10);
    assert_eq!(mesh.vertices.len(), 40);
    assert_eq!(mesh.indices.len(), 60);
    for (triangle_index, triangle) in mesh.indices.as_chunks::<3>().0.iter().enumerate() {
        let a = Vec3::from_array(mesh.vertices[triangle[0] as usize].position);
        let b = Vec3::from_array(mesh.vertices[triangle[1] as usize].position);
        let c = Vec3::from_array(mesh.vertices[triangle[2] as usize].position);
        let normal = Vec3::from_array(mesh.vertices[triangle[0] as usize].normal);
        assert!(
            (b - a).cross(c - a).dot(normal) > 0.0,
            "painting solid triangle {triangle_index} must keep outward front winding"
        );
    }
    assert!(mesh.vertices[..20]
        .iter()
        .all(|vertex| vertex.light == [3.0 / 15.0, 10.0 / 15.0]));
    assert!(mesh.vertices[20..]
        .iter()
        .all(|vertex| vertex.light == [12.0 / 15.0, 5.0 / 15.0]));

    let front_rect = dynamic_atlas
        .entries
        .iter()
        .find(|entry| entry.handle == front.handle)
        .unwrap()
        .uv;
    assert_eq!(mesh.vertices[0].uv, uv(front_rect, 0.5, 1.0));
    assert_eq!(mesh.vertices[1].uv, uv(front_rect, 1.0, 1.0));
    assert_eq!(mesh.vertices[2].uv, uv(front_rect, 1.0, 0.0));
    assert_eq!(mesh.vertices[3].uv, uv(front_rect, 0.5, 0.0));
    assert_eq!(mesh.vertices[20].uv, uv(front_rect, 0.0, 1.0));
    assert_eq!(mesh.vertices[21].uv, uv(front_rect, 0.5, 1.0));

    let positions = mesh
        .vertices
        .iter()
        .map(|vertex| vertex.position)
        .collect::<Vec<_>>();
    assert!(positions.contains(&[9.0, 19.5, 29.96875]));
    assert!(positions.contains(&[11.0, 20.5, 30.03125]));

    // The force-transparent cow and painting share a dynamic atlas and one draw buffer.
    let mut cow =
        EntityModelInstance::cow(12, [0.0; 3], 0.0, false).with(|state| state.invisible = true);
    cow.dynamic_texture = Some(front);
    let mut tile = instances[0].with(|state| state.invisible = true);
    let mut cache = EntityExtractionCache::default();
    for x in [10.0, 10.0, 11.0] {
        tile.position[0] = x;
        let (batch, _) = cache.build(
            &[cow, tile],
            &atlas,
            &[],
            &ResolvedEntityEquipmentAssetCatalog::default(),
            None,
            Some(&dynamic_atlas),
            &[],
            None,
            None,
        );
        let combined = &batch.dynamic_player_texture_item_entity_translucent_cull;
        assert!(combined.deferred_vertices > 0);
        let far_tile = batch.sorted_item_entity_draws[0];
        let start = far_tile.index_start as usize;
        let end = start + far_tile.index_count as usize;
        let tile_indices: Vec<_> = combined.indices[start..end]
            .iter()
            .map(|index| index - combined.deferred_vertices as u32)
            .collect();
        assert_eq!(tile_indices, mesh.indices[..30]);
    }
}

#[test]
fn painting_tile_facing_uses_vanilla_yaw_mapping() {
    let front = EntityDynamicTexture { handle: 72_001 };
    let back = EntityDynamicTexture { handle: 72_002 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[
        DynamicEntityTextureImage {
            handle: front.handle,
            size: [16, 16],
            standalone_sampling: None,
            rgba: vec![0xff; 16 * 16 * 4],
        },
        DynamicEntityTextureImage {
            handle: back.handle,
            size: [16, 16],
            standalone_sampling: None,
            rgba: vec![0xff; 16 * 16 * 4],
        },
    ])
    .unwrap()
    .0;
    let atlas = EntityModelTextureAtlasLayout {
        width: 1,
        height: 1,
        entries: Vec::new(),
    };
    let instance = EntityModelInstance::painting_tile(
        12,
        [0.0, 0.0, 0.0],
        1,
        1,
        0,
        0,
        EntityAttachmentFace::West,
        front,
        back,
    );
    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );
    let first = meshes
        .dynamic_player_texture_solid_z_offset_forward
        .vertices[0]
        .position;
    assert_close3(first, [-0.03125, -0.5, -0.5]);
}
