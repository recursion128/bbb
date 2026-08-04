use super::*;

fn atlas_with_beam_texture() -> EntityModelTextureAtlasLayout {
    let images = [EntityModelTextureImage::new(
        BEACON_BEAM_TEXTURE_REF,
        vec![255; (BEACON_BEAM_TEXTURE_REF.size[0] * BEACON_BEAM_TEXTURE_REF.size[1] * 4) as usize],
    )];
    build_entity_model_texture_atlas(&images).unwrap().0
}

fn beam_instance(state: BeaconBeamRenderState) -> EntityModelInstance {
    EntityModelInstance::beacon_beam(-1, [10.0, 64.0, 0.0]).with_beacon_beam(Some(state))
}

fn beam_state(beam_start: i32, height: i32, radius_scale: f32) -> BeaconBeamRenderState {
    BeaconBeamRenderState {
        beam_start,
        height,
        color_argb: 0xFFF9_FFFE,
        radius_scale,
        animation_time: 0.0,
    }
}

#[test]
fn beacon_beam_spans_its_section_and_carries_the_vanilla_radii() {
    let atlas = atlas_with_beam_texture();
    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[beam_instance(beam_state(3, 2_048, 1.0))],
        &atlas,
        None,
        None,
        Some([0.0, 64.0, 0.0]),
        None,
    );

    // `renderPart` emits four quads per pass, both relative to the block corner.
    assert_eq!(meshes.beacon_beam_opaque.indices.len(), 24);
    assert_eq!(meshes.beacon_beam_translucent.indices.len(), 24);
    let ys = meshes
        .beacon_beam_opaque
        .vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .collect::<Vec<_>>();
    assert_eq!(ys.iter().copied().fold(f32::INFINITY, f32::min), 67.0);
    assert_eq!(
        ys.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        64.0 + 3.0 + 2_048.0
    );

    // `SOLID_BEAM_RADIUS = 0.2` around the block centre, with the extra 45°
    // yaw of `submitBeaconBeam` leaving the corner distances unchanged.
    let solid_radius = meshes
        .beacon_beam_opaque
        .vertices
        .iter()
        .map(|vertex| {
            let dx = vertex.position[0] - 10.5;
            let dz = vertex.position[2] - 0.5;
            (dx * dx + dz * dz).sqrt()
        })
        .fold(f32::NEG_INFINITY, f32::max);
    assert!((solid_radius - 0.2).abs() < 1e-5, "{solid_radius}");
    // `BEAM_GLOW_RADIUS = 0.25` on the unrotated axis-aligned square, so each
    // corner is `0.25` away on both horizontal axes.
    let glow_radius = meshes
        .beacon_beam_translucent
        .vertices
        .iter()
        .map(|vertex| {
            (vertex.position[0] - 10.5)
                .abs()
                .max((vertex.position[2] - 0.5).abs())
        })
        .fold(f32::NEG_INFINITY, f32::max);
    assert!((glow_radius - 0.25).abs() < 1e-5, "{glow_radius}");
}

#[test]
fn beam_radius_scale_multiplies_both_vanilla_radii() {
    let atlas = atlas_with_beam_texture();
    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[beam_instance(beam_state(0, 16, 3.0))],
        &atlas,
        None,
        None,
        Some([0.0, 64.0, 0.0]),
        None,
    );

    let solid_radius = meshes
        .beacon_beam_opaque
        .vertices
        .iter()
        .map(|vertex| {
            let dx = vertex.position[0] - 10.5;
            let dz = vertex.position[2] - 0.5;
            (dx * dx + dz * dz).sqrt()
        })
        .fold(f32::NEG_INFINITY, f32::max);
    assert!((solid_radius - 0.6).abs() < 1e-5, "{solid_radius}");
    let glow_radius = meshes
        .beacon_beam_translucent
        .vertices
        .iter()
        .map(|vertex| {
            (vertex.position[0] - 10.5)
                .abs()
                .max((vertex.position[2] - 0.5).abs())
        })
        .fold(f32::NEG_INFINITY, f32::max);
    assert!((glow_radius - 0.75).abs() < 1e-5, "{glow_radius}");
}

#[test]
fn beacon_beam_glow_alpha_and_translucent_sort_match_the_end_gateway_pass() {
    let atlas = atlas_with_beam_texture();
    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[beam_instance(beam_state(0, 16, 1.0))],
        &atlas,
        None,
        None,
        Some([0.0, 64.0, 0.0]),
        None,
    );

    assert!(meshes
        .beacon_beam_opaque
        .vertices
        .iter()
        .all(|vertex| vertex.tint[3] == 1.0));
    assert!(meshes
        .beacon_beam_translucent
        .vertices
        .iter()
        .all(|vertex| (vertex.tint[3] - 32.0 / 255.0).abs() < 1e-6));

    let beam_draws = meshes
        .sorted_main_translucent_draws
        .iter()
        .filter_map(|draw| match draw {
            EntityModelTranslucentDrawRange::BeaconBeamTranslucent(draw) => Some(draw),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(beam_draws.len(), 1);
    assert_eq!(
        beam_draws[0].render_type,
        EntityModelLayerRenderType::BeaconBeamTranslucent
    );
}

#[test]
fn beacons_without_beam_state_submit_nothing() {
    let atlas = atlas_with_beam_texture();
    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[EntityModelInstance::beacon_beam(-1, [10.0, 64.0, 0.0])],
        &atlas,
        None,
        None,
        Some([0.0, 64.0, 0.0]),
        None,
    );

    assert!(meshes.beacon_beam_opaque.indices.is_empty());
    assert!(meshes.beacon_beam_translucent.indices.is_empty());
}
