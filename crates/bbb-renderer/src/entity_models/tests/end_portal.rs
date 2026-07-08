use super::*;

fn portal_faces() -> [EndPortalModelFace; 2] {
    [EndPortalModelFace::Down, EndPortalModelFace::Up]
}

fn atlas_with_gateway_beam() -> EntityModelTextureAtlasLayout {
    let texture = END_GATEWAY_BEAM_TEXTURE_REF;
    let image = EntityModelTextureImage::new(
        texture,
        vec![255; (texture.size[0] * texture.size[1] * 4) as usize],
    );
    build_entity_model_texture_atlas(&[image]).unwrap().0
}

#[test]
fn end_portal_model_keys_and_texture_refs_mark_special_render_types() {
    let portal = EntityModelKind::EndPortalBlock {
        kind: EndPortalModelKind::EndPortal,
        faces: portal_faces(),
    };
    let gateway = EntityModelKind::EndPortalBlock {
        kind: EndPortalModelKind::EndGateway,
        faces: portal_faces(),
    };

    assert_eq!(portal.model_key(), "end_portal_block");
    assert_eq!(gateway.model_key(), "end_gateway_block");
    assert_eq!(portal.vanilla_texture_ref(), None);
    assert_eq!(gateway.vanilla_texture_ref(), None);
}

#[test]
fn end_portal_cube_uses_vanilla_y_axis_faces_and_transform() {
    let atlas = atlas_with_gateway_beam();
    let instance = EntityModelInstance::new(
        -1,
        EntityModelKind::EndPortalBlock {
            kind: EndPortalModelKind::EndPortal,
            faces: portal_faces(),
        },
        [2.0, 3.0, 4.0],
        0.0,
    );

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.end_portal.vertices.len(), 8);
    assert_eq!(meshes.end_portal.indices.len(), 12);
    assert!(meshes.end_gateway.vertices.is_empty());
    let min_y = meshes
        .end_portal
        .vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .fold(f32::INFINITY, f32::min);
    let max_y = meshes
        .end_portal
        .vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .fold(f32::NEG_INFINITY, f32::max);
    assert!((min_y - 3.375).abs() < 1.0e-6);
    assert!((max_y - 3.75).abs() < 1.0e-6);
    assert_eq!(
        meshes.custom_submissions[0].render_type.vanilla_name(),
        "end_portal"
    );
}

#[test]
fn end_gateway_cube_uses_unit_cube_faces_without_beam_when_inactive() {
    let atlas = atlas_with_gateway_beam();
    let instance = EntityModelInstance::new(
        -1,
        EntityModelKind::EndPortalBlock {
            kind: EndPortalModelKind::EndGateway,
            faces: portal_faces(),
        },
        [2.0, 3.0, 4.0],
        0.0,
    );

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.end_gateway.vertices.len(), 8);
    assert_eq!(meshes.end_gateway.indices.len(), 12);
    assert!(meshes.scroll.vertices.is_empty());
    let min_y = meshes
        .end_gateway
        .vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .fold(f32::INFINITY, f32::min);
    let max_y = meshes
        .end_gateway
        .vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .fold(f32::NEG_INFINITY, f32::max);
    assert!((min_y - 3.0).abs() < 1.0e-6);
    assert!((max_y - 4.0).abs() < 1.0e-6);
    assert_eq!(
        meshes.custom_submissions[0].render_type.vanilla_name(),
        "end_gateway"
    );
}

#[test]
fn end_gateway_beam_reuses_vanilla_beacon_beam_geometry() {
    let atlas = atlas_with_gateway_beam();
    let instance = EntityModelInstance::new(
        -1,
        EntityModelKind::EndPortalBlock {
            kind: EndPortalModelKind::EndGateway,
            faces: portal_faces(),
        },
        [1.0, 2.0, 3.0],
        0.0,
    )
    .with_end_gateway_beam(Some(EndGatewayBeamRenderState {
        scale: 0.5,
        height: 10,
        color_argb: 0xFFC74EBD,
        animation_time: 3.25,
    }));

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.end_gateway.vertices.len(), 8);
    assert_eq!(meshes.scroll.vertices.len(), 32);
    assert_eq!(meshes.scroll.indices.len(), 48);
    assert_eq!(
        meshes.submissions[0].render_type.vanilla_name(),
        "end_gateway_beam"
    );
    assert_eq!(
        meshes.scroll.vertices[0].tint,
        [199.0 / 255.0, 78.0 / 255.0, 189.0 / 255.0, 1.0]
    );
    assert_eq!(meshes.scroll.vertices[16].tint[3], 32.0 / 255.0);
    let min_y = meshes
        .scroll
        .vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .fold(f32::INFINITY, f32::min);
    let max_y = meshes
        .scroll
        .vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .fold(f32::NEG_INFINITY, f32::max);
    assert_eq!(min_y, -8.0);
    assert_eq!(max_y, 12.0);
}

#[test]
fn end_portal_position_color_draws_sort_with_camera() {
    let atlas = atlas_with_gateway_beam();
    let portal = EntityModelInstance::new(
        -1,
        EntityModelKind::EndPortalBlock {
            kind: EndPortalModelKind::EndPortal,
            faces: portal_faces(),
        },
        [10.0, 64.0, 0.0],
        0.0,
    );

    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[portal],
        &atlas,
        None,
        None,
        Some([0.0, 64.0, 0.0]),
    );

    assert_eq!(meshes.sorted_main_translucent_draws.len(), 1);
    let EntityModelTranslucentDrawRange::PositionColor(draw) =
        meshes.sorted_main_translucent_draws[0]
    else {
        panic!("end portal should sort as position-color custom geometry");
    };
    assert_eq!(draw.render_type, EntityModelLayerRenderType::EndPortal);
    assert_eq!(draw.index_start, 0);
    assert_eq!(draw.index_count, 12);
}
