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
        .with_light_coords((3 << 4) | (10 << 20)),
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
        .with_light_coords((12 << 4) | (5 << 20)),
    ];

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &instances,
        &atlas,
        None,
        Some(&dynamic_atlas),
    );
    let mesh = &meshes.dynamic_player_texture_solid_z_offset_forward;
    assert_eq!(meshes.submissions.len(), 2);
    assert!(meshes.submissions.iter().all(|submission| {
        submission.render_type == EntityModelLayerRenderType::EntitySolidZOffsetForward
    }));
    assert_eq!(mesh.cutout_faces, 10);
    assert_eq!(mesh.vertices.len(), 40);
    assert_eq!(mesh.indices.len(), 60);
    for (triangle_index, triangle) in mesh.indices.chunks_exact(3).enumerate() {
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
