use super::*;

fn portal_faces() -> EndPortalModelFaces {
    EndPortalModelFaces::Y_AXIS
}

fn gateway_faces() -> EndPortalModelFaces {
    EndPortalModelFaces::ALL
}

fn atlas_with_portal_textures() -> EntityModelTextureAtlasLayout {
    let images = [
        blank_texture(END_SKY_TEXTURE_REF),
        blank_texture(END_PORTAL_TEXTURE_REF),
        blank_texture(END_GATEWAY_BEAM_TEXTURE_REF),
    ];
    build_entity_model_texture_atlas(&images).unwrap().0
}

fn atlas_with_portal_texture_presence(
    include_sky: bool,
    include_portal: bool,
) -> EntityModelTextureAtlasLayout {
    let mut images = vec![blank_texture(END_GATEWAY_BEAM_TEXTURE_REF)];
    if include_sky {
        images.push(blank_texture(END_SKY_TEXTURE_REF));
    }
    if include_portal {
        images.push(blank_texture(END_PORTAL_TEXTURE_REF));
    }
    build_entity_model_texture_atlas(&images).unwrap().0
}

fn blank_texture(texture: EntityModelTextureRef) -> EntityModelTextureImage {
    EntityModelTextureImage::new(
        texture,
        vec![255; (texture.size[0] * texture.size[1] * 4) as usize],
    )
}

#[test]
fn end_portal_cube_uses_vanilla_y_axis_faces_and_transform() {
    let atlas = atlas_with_portal_textures();
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
    assert_eq!(meshes.end_portal.vertices[0].sky_rect_min, [0.0, 0.0]);
    assert_eq!(meshes.end_portal.vertices[0].portal_rect_min[0], 0.0);
    assert!(meshes.end_portal.vertices[0].portal_rect_min[1] > 0.0);
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
    assert_eq!(meshes.end_portal.vertices[0].position, [2.0, 3.375, 4.0]);
    assert_eq!(meshes.end_portal.vertices[2].position, [3.0, 3.375, 5.0]);
    assert_eq!(meshes.end_portal.vertices[4].position, [2.0, 3.75, 4.0]);
}

#[test]
fn end_gateway_cube_emits_all_six_faces_while_end_portal_emits_only_y_axis() {
    let atlas = atlas_with_portal_textures();
    let gateway = EntityModelInstance::new(
        -1,
        EntityModelKind::EndPortalBlock {
            kind: EndPortalModelKind::EndGateway,
            faces: gateway_faces(),
        },
        [2.0, 3.0, 4.0],
        0.0,
    );

    let meshes = entity_model_textured_meshes(&[gateway], &atlas);

    assert_eq!(meshes.end_gateway.vertices.len(), 24);
    assert_eq!(meshes.end_gateway.indices.len(), 36);
    assert!(meshes.beacon_beam_opaque.vertices.is_empty());
    assert!(meshes.beacon_beam_translucent.vertices.is_empty());
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
    assert_eq!(meshes.end_gateway.vertices[0].position, [2.0, 3.0, 4.0]);
    assert_eq!(meshes.end_gateway.vertices[2].position, [3.0, 3.0, 5.0]);
    assert_eq!(meshes.end_gateway.vertices[4].position, [2.0, 4.0, 4.0]);

    // Verify all 6 unit-cube face normals are outward CCW normals matching Vanilla `FaceInfo`.
    let normals: Vec<[i32; 3]> = meshes
        .end_gateway
        .vertices
        .as_chunks::<4>()
        .0
        .iter()
        .map(|quad| {
            let v0 = glam::Vec3::from_array(quad[0].position);
            let v1 = glam::Vec3::from_array(quad[1].position);
            let v2 = glam::Vec3::from_array(quad[2].position);
            let normal = (v1 - v0).cross(v2 - v0).normalize();
            [
                normal.x.round() as i32,
                normal.y.round() as i32,
                normal.z.round() as i32,
            ]
        })
        .collect();
    assert_eq!(
        normals,
        vec![
            [0, -1, 0], // Down
            [0, 1, 0],  // Up
            [0, 0, -1], // North
            [0, 0, 1],  // South
            [-1, 0, 0], // West
            [1, 0, 0],  // East
        ]
    );

    // Even if all 6 faces are requested on EndPortal, it emits only Up | Down.
    let portal = EntityModelInstance::new(
        -1,
        EntityModelKind::EndPortalBlock {
            kind: EndPortalModelKind::EndPortal,
            faces: gateway_faces(),
        },
        [2.0, 3.0, 4.0],
        0.0,
    );
    let portal_meshes = entity_model_textured_meshes(&[portal], &atlas);
    assert_eq!(portal_meshes.end_portal.vertices.len(), 8);
    assert_eq!(portal_meshes.end_portal.indices.len(), 12);
}

#[test]
fn end_portal_geometry_requires_both_atlas_textures() {
    for (include_sky, include_portal) in [(false, true), (true, false), (false, false)] {
        let atlas = atlas_with_portal_texture_presence(include_sky, include_portal);
        for kind in [
            EndPortalModelKind::EndPortal,
            EndPortalModelKind::EndGateway,
        ] {
            let instance = EntityModelInstance::new(
                -1,
                EntityModelKind::EndPortalBlock {
                    kind,
                    faces: portal_faces(),
                },
                [2.0, 3.0, 4.0],
                0.0,
            );

            let meshes = entity_model_textured_meshes(&[instance], &atlas);

            assert!(meshes.end_portal.vertices.is_empty());
            assert!(meshes.end_portal.indices.is_empty());
            assert!(meshes.end_gateway.vertices.is_empty());
            assert!(meshes.end_gateway.indices.is_empty());
            assert!(meshes.sorted_main_translucent_draws.is_empty());
        }
    }
}

#[test]
fn end_gateway_beam_reuses_vanilla_beacon_beam_geometry() {
    let atlas = atlas_with_portal_textures();
    let instance = EntityModelInstance::new(
        -1,
        EntityModelKind::EndPortalBlock {
            kind: EndPortalModelKind::EndGateway,
            faces: gateway_faces(),
        },
        [1.0, 2.0, 3.0],
        0.0,
    )
    .with_block_entity(|s| {
        s.end_gateway_beam = Some(EndGatewayBeamRenderState {
            scale: 0.5,
            height: 10,
            color_argb: 0xFFC74EBD,
            animation_time: 3.25,
        })
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.end_gateway.vertices.len(), 24);
    assert_eq!(meshes.beacon_beam_opaque.vertices.len(), 16);
    assert_eq!(meshes.beacon_beam_opaque.indices.len(), 24);
    assert_eq!(meshes.beacon_beam_translucent.vertices.len(), 16);
    assert_eq!(meshes.beacon_beam_translucent.indices.len(), 24);
    assert_eq!(meshes.opaque_beam_draws.len(), 1);
    assert_eq!(
        (
            meshes.opaque_beam_draws[0].render_type,
            meshes.opaque_beam_draws[0].index_start,
            meshes.opaque_beam_draws[0].index_count
        ),
        (EntityModelLayerRenderType::BeaconBeamOpaque, 0, 24)
    );

    assert_eq!(
        meshes.beacon_beam_opaque.vertices[0].tint,
        [199.0 / 255.0, 78.0 / 255.0, 189.0 / 255.0, 1.0]
    );
    assert_eq!(
        meshes.beacon_beam_translucent.vertices[0].tint[3],
        32.0 / 255.0
    );
    let min_y = meshes
        .beacon_beam_opaque
        .vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .fold(f32::INFINITY, f32::min);
    let max_y = meshes
        .beacon_beam_opaque
        .vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .fold(f32::NEG_INFINITY, f32::max);
    assert_eq!(min_y, -8.0);
    assert_eq!(max_y, 12.0);
}

#[test]
fn end_gateway_beam_sorts_only_translucent_glow_with_camera() {
    let atlas = atlas_with_portal_textures();
    let instance = EntityModelInstance::new(
        -1,
        EntityModelKind::EndPortalBlock {
            kind: EndPortalModelKind::EndGateway,
            faces: portal_faces(),
        },
        [10.0, 64.0, 0.0],
        0.0,
    )
    .with_block_entity(|s| {
        s.end_gateway_beam = Some(EndGatewayBeamRenderState {
            scale: 0.5,
            height: 10,
            color_argb: 0xFFC74EBD,
            animation_time: 3.25,
        })
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[instance],
        &atlas,
        None,
        None,
        Some([0.0, 64.0, 0.0]),
        None,
    );

    assert_eq!(meshes.beacon_beam_opaque.indices.len(), 24);
    assert_eq!(meshes.beacon_beam_translucent.indices.len(), 24);
    let beam_draws = meshes
        .sorted_main_translucent_draws
        .iter()
        .filter_map(|draw| match draw {
            EntityModelTranslucentDrawRange::Geometry(draw)
                if draw.render_type == EntityModelLayerRenderType::BeaconBeamTranslucent =>
            {
                Some(draw)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(beam_draws.len(), 1);
    assert_eq!(
        beam_draws[0].render_type,
        EntityModelLayerRenderType::BeaconBeamTranslucent
    );
    assert_eq!(
        (beam_draws[0].index_start, beam_draws[0].index_count),
        (0, 24)
    );
}

#[test]
fn end_portal_portal_draws_sort_with_camera() {
    let atlas = atlas_with_portal_textures();
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
        None,
    );

    assert_eq!(meshes.sorted_main_translucent_draws.len(), 1);
    let EntityModelTranslucentDrawRange::Geometry(draw) = meshes.sorted_main_translucent_draws[0]
    else {
        panic!("end portal should sort as portal texture-backed geometry");
    };
    assert_eq!(draw.render_type, EntityModelLayerRenderType::EndPortal);
    assert_eq!(draw.index_start, 0);
    assert_eq!(draw.index_count, 12);
}
