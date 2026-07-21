use super::*;

use crate::camera::CameraPose;

fn experience_orb_test_atlas() -> EntityModelTextureAtlasLayout {
    EntityModelTextureAtlasLayout {
        width: 64,
        height: 64,
        entries: vec![EntityModelTextureAtlasEntry {
            texture: EXPERIENCE_ORB_TEXTURE_REF,
            uv: EntityModelUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        }],
    }
}

#[test]
fn experience_orb_entity_quad_matches_vanilla_renderer() {
    let atlas = experience_orb_test_atlas();
    let instance = EntityModelInstance::new(
        9,
        EntityModelKind::ExperienceOrb { icon: 5 },
        [10.0, 20.0, 30.0],
        0.0,
    )
    .with_age_in_ticks(4.0)
    .with_light_coords((15 << 4) | (8 << 20));
    // Vanilla yaw 0 looks south (+Z): billboard right is -X and up is +Y.
    let pose = CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };

    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        std::slice::from_ref(&instance),
        &atlas,
        None,
        None,
        Some([0.0, 64.0, 0.0]),
        Some(pose),
    );

    assert_eq!(meshes.submissions.len(), 1);
    let submission = &meshes.submissions[0];
    assert_eq!(
        submission.render_type,
        EntityModelLayerRenderType::EntityTranslucentCullItemTarget
    );
    let rr = 4.0_f32 / 2.0;
    let red = (((rr).sin() + 1.0) * 0.5 * 255.0) as i32;
    let blue = (((rr + std::f32::consts::PI * 4.0 / 3.0).sin() + 1.0) * 0.1 * 255.0) as i32;
    let tint = [red as f32 / 255.0, 1.0, blue as f32 / 255.0, 128.0 / 255.0];
    assert_eq!(submission.tint, tint);

    let mesh = &meshes.item_entity_translucent_cull;
    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
    assert_eq!(meshes.sorted_item_entity_draws.len(), 1);
    // Vanilla `ExperienceOrbRenderer.submit`: translate(0, 0.1, 0), face the camera, scale 0.3
    // over corners (x - 0.5, y - 0.25, 0). With billboard right -X: x = px - 0.3·cx.
    assert_eq!(mesh.vertices[0].position, [10.15, 20.025, 30.0]);
    assert_eq!(mesh.vertices[1].position, [9.85, 20.025, 30.0]);
    assert_eq!(mesh.vertices[2].position, [9.85, 20.325, 30.0]);
    assert_eq!(mesh.vertices[3].position, [10.15, 20.325, 30.0]);
    // Icon 5 sits at sheet cell (1, 1) on the 4×4 grid of 16px cells.
    assert_eq!(mesh.vertices[0].uv, [16.0 / 64.0, 32.0 / 64.0]);
    assert_eq!(mesh.vertices[1].uv, [32.0 / 64.0, 32.0 / 64.0]);
    assert_eq!(mesh.vertices[2].uv, [32.0 / 64.0, 16.0 / 64.0]);
    assert_eq!(mesh.vertices[3].uv, [16.0 / 64.0, 16.0 / 64.0]);
    for vertex in &mesh.vertices {
        assert_eq!(vertex.tint, tint);
        assert_eq!(vertex.light, [1.0, 8.0 / 15.0]);
        assert_eq!(vertex.normal, [0.0, 1.0, 0.0]);
    }
}

#[test]
fn experience_orb_entity_quad_requires_camera_pose() {
    let atlas = experience_orb_test_atlas();
    let instance = EntityModelInstance::new(
        9,
        EntityModelKind::ExperienceOrb { icon: 0 },
        [0.0, 0.0, 0.0],
        0.0,
    );

    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        std::slice::from_ref(&instance),
        &atlas,
        None,
        None,
        None,
        None,
    );

    assert!(meshes.submissions.is_empty());
    assert!(meshes.item_entity_translucent_cull.vertices.is_empty());
    assert!(meshes.sorted_item_entity_draws.is_empty());
}

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
