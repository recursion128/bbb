use super::*;

use crate::camera::CameraPose;

fn dragon_fireball_test_atlas() -> EntityModelTextureAtlasLayout {
    EntityModelTextureAtlasLayout {
        width: 16,
        height: 16,
        entries: vec![EntityModelTextureAtlasEntry {
            texture: DRAGON_FIREBALL_TEXTURE_REF,
            uv: EntityModelUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        }],
    }
}

#[test]
fn dragon_fireball_entity_quad_matches_vanilla_renderer() {
    let atlas = dragon_fireball_test_atlas();
    let instance =
        EntityModelInstance::new(7, EntityModelKind::DragonFireball, [10.0, 20.0, 30.0], 0.0)
            .with_light_coords((15 << 4) | (2 << 20));
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
        None,
        Some(pose),
    );

    assert_eq!(meshes.submissions.len(), 1);
    let submission = &meshes.submissions[0];
    assert_eq!(
        submission.render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(submission.tint, [1.0; 4]);

    let mesh = &meshes.cutout;
    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
    // Vanilla `DragonFireballRenderer.submit`: scale(2, 2, 2) then face the camera over corners
    // (x - 0.5, y - 0.25, 0). With billboard right -X: x = px - 2·cx.
    assert_eq!(mesh.vertices[0].position, [11.0, 19.5, 30.0]);
    assert_eq!(mesh.vertices[1].position, [9.0, 19.5, 30.0]);
    assert_eq!(mesh.vertices[2].position, [9.0, 21.5, 30.0]);
    assert_eq!(mesh.vertices[3].position, [11.0, 21.5, 30.0]);
    // The full 16×16 sheet: vertex (x, y, u, v) rows (0,0,0,1), (1,0,1,1), (1,1,1,0), (0,1,0,0).
    assert_eq!(mesh.vertices[0].uv, [0.0, 1.0]);
    assert_eq!(mesh.vertices[1].uv, [1.0, 1.0]);
    assert_eq!(mesh.vertices[2].uv, [1.0, 0.0]);
    assert_eq!(mesh.vertices[3].uv, [0.0, 0.0]);
    for vertex in &mesh.vertices {
        assert_eq!(vertex.tint, [1.0; 4]);
        assert_eq!(vertex.light, [1.0, 2.0 / 15.0]);
        assert_eq!(vertex.normal, [0.0, 1.0, 0.0]);
    }
}

#[test]
fn dragon_fireball_entity_quad_requires_camera_pose() {
    let atlas = dragon_fireball_test_atlas();
    let instance =
        EntityModelInstance::new(7, EntityModelKind::DragonFireball, [0.0, 0.0, 0.0], 0.0);

    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        std::slice::from_ref(&instance),
        &atlas,
        None,
        None,
        None,
        None,
    );

    assert!(meshes.submissions.is_empty());
    assert!(meshes.cutout.vertices.is_empty());
}
