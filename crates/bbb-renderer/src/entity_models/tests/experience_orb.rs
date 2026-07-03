use super::*;

#[test]
fn experience_orb_texture_is_loaded_through_entity_atlas_refs() {
    assert_eq!(
        experience_orb_entity_texture_refs(),
        &[EXPERIENCE_ORB_TEXTURE_REF]
    );
    assert!(entity_model_texture_refs().contains(&EXPERIENCE_ORB_TEXTURE_REF));
}

#[test]
fn experience_orb_pickup_particle_mesh_matches_vanilla_billboard_quad() {
    let atlas = EntityModelTextureAtlasLayout {
        width: 64,
        height: 64,
        entries: vec![EntityModelTextureAtlasEntry {
            texture: EXPERIENCE_ORB_TEXTURE_REF,
            uv: EntityModelUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        }],
    };
    let mesh = experience_orb_pickup_particle_textured_mesh(
        &[ExperienceOrbPickupParticleRenderInstance {
            transform: Mat4::IDENTITY,
            icon: 5,
            age_ticks: 4.0,
            light: [0.4, 0.8],
        }],
        &atlas,
    );

    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
    assert_eq!(mesh.cutout_faces, 1);
    assert_eq!(mesh.vertices[0].position, [-0.5, -0.25, 0.0]);
    assert_eq!(mesh.vertices[1].position, [0.5, -0.25, 0.0]);
    assert_eq!(mesh.vertices[2].position, [0.5, 0.75, 0.0]);
    assert_eq!(mesh.vertices[3].position, [-0.5, 0.75, 0.0]);
    assert_eq!(mesh.vertices[0].uv, [16.0 / 64.0, 32.0 / 64.0]);
    assert_eq!(mesh.vertices[1].uv, [32.0 / 64.0, 32.0 / 64.0]);
    assert_eq!(mesh.vertices[2].uv, [32.0 / 64.0, 16.0 / 64.0]);
    assert_eq!(mesh.vertices[3].uv, [16.0 / 64.0, 16.0 / 64.0]);
    let rr = 4.0_f32 / 2.0;
    let red = (((rr).sin() + 1.0) * 0.5 * 255.0) as i32;
    let blue = (((rr + std::f32::consts::PI * 4.0 / 3.0).sin() + 1.0) * 0.1 * 255.0) as i32;
    for vertex in &mesh.vertices {
        assert_eq!(
            vertex.tint,
            [red as f32 / 255.0, 1.0, blue as f32 / 255.0, 128.0 / 255.0]
        );
        assert_eq!(vertex.light, [0.4, 0.8]);
        assert_eq!(vertex.overlay, ENTITY_VERTEX_NO_OVERLAY);
        assert_eq!(vertex.normal, [0.0, 1.0, 0.0]);
    }
}
