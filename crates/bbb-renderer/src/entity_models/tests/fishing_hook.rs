use super::*;

use crate::camera::CameraPose;

fn fishing_hook_test_atlas() -> EntityModelTextureAtlasLayout {
    EntityModelTextureAtlasLayout {
        width: 16,
        height: 16,
        entries: vec![EntityModelTextureAtlasEntry {
            texture: FISHING_HOOK_TEXTURE_REF,
            uv: EntityModelUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        }],
    }
}

fn fishing_hook_instance() -> EntityModelInstance {
    EntityModelInstance::new(17, EntityModelKind::FishingHook, [10.0, 20.0, 30.0], 0.0)
        .with_fishing_line_origin_offset(Some([4.0, 2.0, -8.0]))
        .with_light_coords((12 << 4) | (3 << 20))
}

#[test]
fn fishing_hook_quad_and_submission_order_match_vanilla_renderer() {
    let instance = fishing_hook_instance();
    let pose = CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        std::slice::from_ref(&instance),
        &fishing_hook_test_atlas(),
        None,
        None,
        None,
        Some(pose),
    );

    assert_eq!(meshes.submissions.len(), 1);
    let quad = meshes.submissions[0];
    assert_eq!(
        quad.render_type,
        EntityModelLayerRenderType::EntityCutoutCull
    );
    assert_eq!(quad.texture, FISHING_HOOK_TEXTURE_REF);
    assert_eq!(quad.submit_sequence, 0);
    assert_eq!(quad.light, [12.0 / 15.0, 3.0 / 15.0]);
    assert_eq!(meshes.custom_submissions.len(), 1);
    let line = meshes.custom_submissions[0];
    assert_eq!(line.render_type, EntityModelLayerRenderType::Lines);
    assert_eq!(line.submit_sequence, 1);

    let mesh = &meshes.cutout_cull;
    assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.vertices[0].position, [10.25, 19.75, 30.0]);
    assert_eq!(mesh.vertices[1].position, [9.75, 19.75, 30.0]);
    assert_eq!(mesh.vertices[2].position, [9.75, 20.25, 30.0]);
    assert_eq!(mesh.vertices[3].position, [10.25, 20.25, 30.0]);
    assert_eq!(mesh.vertices[0].uv, [0.0, 1.0]);
    assert_eq!(mesh.vertices[1].uv, [1.0, 1.0]);
    assert_eq!(mesh.vertices[2].uv, [1.0, 0.0]);
    assert_eq!(mesh.vertices[3].uv, [0.0, 0.0]);
    for vertex in &mesh.vertices {
        assert_eq!(vertex.tint, [1.0; 4]);
        assert_eq!(vertex.light, [12.0 / 15.0, 3.0 / 15.0]);
        assert_eq!(vertex.overlay, ENTITY_VERTEX_NO_OVERLAY);
        assert_eq!(vertex.normal, [0.0, 1.0, 0.0]);
    }
}

#[test]
fn fishing_hook_line_uses_vanilla_sixteen_step_parabola() {
    let lines = fishing_hook_line_segments(fishing_hook_instance(), 3.75).unwrap();

    assert_eq!(lines.len(), 16);
    assert_eq!(lines[0].from, [10.0, 20.25, 30.0]);
    assert_eq!(lines[7].to, [12.0, 21.0, 26.0]);
    assert_eq!(lines[15].to, [14.0, 22.25, 22.0]);
    assert!(lines
        .iter()
        .all(|line| line.color == [0.0, 0.0, 0.0, 1.0] && line.width == 3.75));
    for pair in lines.windows(2) {
        assert_eq!(pair[0].to, pair[1].from);
    }
}

#[test]
fn fishing_hook_requires_owner_line_state_and_camera() {
    let atlas = fishing_hook_test_atlas();
    let without_owner = EntityModelInstance::new(17, EntityModelKind::FishingHook, [0.0; 3], 0.0);
    let pose = CameraPose {
        position: [0.0; 3],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[without_owner],
        &atlas,
        None,
        None,
        None,
        Some(pose),
    );
    assert!(meshes.submissions.is_empty());
    assert!(meshes.custom_submissions.is_empty());
    assert!(fishing_hook_line_segments(without_owner, 2.5).is_none());

    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[fishing_hook_instance()],
        &atlas,
        None,
        None,
        None,
        None,
    );
    assert!(meshes.submissions.is_empty());
    assert!(meshes.custom_submissions.is_empty());
}

#[test]
fn fishing_hook_texture_and_catalog_selection_are_registered() {
    assert_eq!(
        fishing_hook_entity_texture_refs(),
        &[FISHING_HOOK_TEXTURE_REF]
    );
    assert!(entity_model_texture_refs().contains(&FISHING_HOOK_TEXTURE_REF));
    assert_eq!(EntityModelKind::FishingHook.model_key(), "fishing_hook");
    assert_eq!(
        EntityModelKind::FishingHook.vanilla_texture_ref(),
        Some(FISHING_HOOK_TEXTURE_REF)
    );
}
