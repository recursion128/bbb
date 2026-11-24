use super::*;

fn blank_texture(texture: EntityModelTextureRef) -> EntityModelTextureImage {
    let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
    EntityModelTextureImage::new(texture, vec![0u8; len])
}

#[test]
fn end_crystal_textured_submit_matches_vanilla_renderer() {
    // Vanilla `EndCrystalRenderer.submit`: submit the `EndCrystalModel` with
    // `textures/entity/end_crystal/end_crystal.png`, the default `EntityModel` render type
    // (`entityCutout`), `order(0)`, and the renderer root transform `scale(2)·translate(0,-0.5,0)`.
    assert!(entity_model_texture_refs().contains(&END_CRYSTAL_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&END_CRYSTAL_BEAM_TEXTURE_REF));
    let passes = end_crystal_textured_layer_passes();
    assert_eq!(passes.len(), 2);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].texture, END_CRYSTAL_TEXTURE_REF);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
    assert_eq!(
        passes[1].render_type,
        EntityModelLayerRenderType::EndCrystalBeam
    );
    assert_eq!(passes[1].render_type.vanilla_name(), "end_crystal_beam");
    assert_eq!(passes[1].texture, END_CRYSTAL_BEAM_TEXTURE_REF);
    assert_eq!(passes[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[1].order, 0);

    let images = vec![
        blank_texture(END_CRYSTAL_TEXTURE_REF),
        blank_texture(END_CRYSTAL_BEAM_TEXTURE_REF),
    ];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::end_crystal(450, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.age_in_ticks = 30.0;
        s.light_coords = (9_u32 << 4) | (7_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.cutout.vertices.len(), 96);
    assert_eq!(meshes.cutout.indices.len(), 144);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == [0.0, 10.0]));
}

#[test]
fn end_crystal_body_missing_texture_preserves_other_geometry() {
    let images = vec![blank_texture(END_CRYSTAL_BEAM_TEXTURE_REF)];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::end_crystal(452, [2.0, 65.0, -4.0], 0.0).with(|s| {
        s.age_in_ticks = 30.0;
        s.light_coords = (9_u32 << 4) | (7_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.end_crystal_beam.vertices.is_empty());
}

#[test]
fn end_crystal_beam_emits_vanilla_beam_geometry() {
    // Vanilla `EndCrystalRenderer.submit`: after the `entityCutout` body submit, a crystal with
    // `beamOffset` translates by that offset and calls `EnderDragonRenderer.submitCrystalBeams`.
    // The helper uses `RenderTypes.endCrystalBeam(end_crystal_beam.png)`, `order(0)`, a later submit
    // sequence than the body, black inner vertices, white outer vertices, and tiled V coordinates.
    let images = vec![
        blank_texture(END_CRYSTAL_TEXTURE_REF),
        blank_texture(END_CRYSTAL_BEAM_TEXTURE_REF),
    ];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let age = 30.0;
    let position = [10.0, 64.0, -3.0];
    let beam_offset = [4.5, 3.5, -6.5];
    let instance = EntityModelInstance::end_crystal(451, position, 0.0).with(|s| {
        s.age_in_ticks = age;
        s.kind.end_crystal_mut().beam = Some(EndCrystalBeamRenderState { beam_offset });
        s.light_coords = (10_u32 << 4) | (6_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.cutout.vertices.iter().all(|vertex| {
        vertex.light == instance.render_state.shader_light() && vertex.overlay == [0.0, 10.0]
    }));

    let origin = Vec3::from_array(position) + Vec3::from_array(beam_offset) + Vec3::Y * 2.0;

    let delta = Vec3::new(
        -beam_offset[0],
        -beam_offset[1] + end_crystal_get_y(age),
        -beam_offset[2],
    );

    assert_eq!(meshes.end_crystal_beam.vertices.len(), 32);
    assert_eq!(meshes.end_crystal_beam.indices.len(), 48);
    assert_eq!(meshes.opaque_beam_draws.len(), 1);
    assert_eq!(
        (
            meshes.opaque_beam_draws[0].render_type,
            meshes.opaque_beam_draws[0].index_start,
            meshes.opaque_beam_draws[0].index_count
        ),
        (EntityModelLayerRenderType::EndCrystalBeam, 0, 48)
    );
    let rect = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == END_CRYSTAL_BEAM_TEXTURE_REF)
        .unwrap()
        .uv;
    assert_eq!(meshes.end_crystal_beam.vertices[0].uv_rect_min, rect.min);
    assert_eq!(
        meshes.end_crystal_beam.vertices[0].uv_rect_size,
        [rect.max[0] - rect.min[0], rect.max[1] - rect.min[1]]
    );
    assert_eq!(
        meshes.end_crystal_beam.vertices[0].tint,
        [0.0, 0.0, 0.0, 1.0]
    );
    assert_eq!(
        meshes.end_crystal_beam.vertices[1].tint,
        [1.0, 1.0, 1.0, 1.0]
    );
    assert!(meshes
        .end_crystal_beam
        .vertices
        .iter()
        .all(
            |vertex| vertex.light == instance.render_state.shader_light()
                && vertex.overlay == [0.0, 10.0]
        ));
    let start_center = meshes
        .end_crystal_beam
        .vertices
        .as_chunks::<4>()
        .0
        .iter()
        .map(|quad| Vec3::from_array(quad[0].position))
        .sum::<Vec3>()
        / 8.0;
    let end_center = meshes
        .end_crystal_beam
        .vertices
        .as_chunks::<4>()
        .0
        .iter()
        .map(|quad| Vec3::from_array(quad[1].position))
        .sum::<Vec3>()
        / 8.0;
    assert_close3(start_center.to_array(), origin.to_array());
    assert_close3(end_center.to_array(), (origin + delta).to_array());
    assert_close3(
        meshes.end_crystal_beam.vertices[0].normal,
        (origin - Vec3::from_array(meshes.end_crystal_beam.vertices[0].position))
            .normalize()
            .to_array(),
    );
    assert_eq!(meshes.end_crystal_beam.vertices[0].local_uv[0], 0.0);
    assert_eq!(meshes.end_crystal_beam.vertices[3].local_uv[0], 0.125);
    let length = delta.length();
    assert!(
        (meshes.end_crystal_beam.vertices[1].local_uv[1]
            - meshes.end_crystal_beam.vertices[0].local_uv[1]
            - length / 32.0)
            .abs()
            < 1.0e-6
    );
}

#[test]
fn end_crystal_beam_missing_beam_texture_texture_preserves_other_geometry() {
    let images = vec![blank_texture(END_CRYSTAL_TEXTURE_REF)];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let age = 30.0;
    let position = [10.0, 64.0, -3.0];
    let beam_offset = [4.5, 3.5, -6.5];
    let instance = EntityModelInstance::end_crystal(453, position, 0.0).with(|s| {
        s.age_in_ticks = age;
        s.kind.end_crystal_mut().beam = Some(EndCrystalBeamRenderState { beam_offset });
        s.light_coords = (10_u32 << 4) | (6_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.cutout.vertices.len(), 96);

    assert!(
        meshes.end_crystal_beam.vertices.is_empty(),
        "missing end_crystal_beam.png suppresses only folded beam geometry"
    );
    assert!(meshes.end_crystal_beam.indices.is_empty());
    assert!(meshes.opaque_beam_draws.is_empty());
}

#[test]
fn end_crystal_beam_ranges_preserve_multiple_submit_boundaries() {
    let images = vec![
        blank_texture(END_CRYSTAL_TEXTURE_REF),
        blank_texture(END_CRYSTAL_BEAM_TEXTURE_REF),
    ];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instances = [
        EntityModelInstance::end_crystal(454, [0.0, 64.0, 0.0], 0.0).with(|s| {
            s.kind.end_crystal_mut().beam = Some(EndCrystalBeamRenderState {
                beam_offset: [4.0, 2.0, 0.0],
            })
        }),
        EntityModelInstance::end_crystal(455, [8.0, 64.0, 0.0], 0.0).with(|s| {
            s.kind.end_crystal_mut().beam = Some(EndCrystalBeamRenderState {
                beam_offset: [-4.0, 2.0, 0.0],
            })
        }),
    ];

    let meshes = entity_model_textured_meshes(&instances, &atlas);

    assert_eq!(meshes.end_crystal_beam.indices.len(), 96);
    assert_eq!(meshes.opaque_beam_draws.len(), 2);
    assert_eq!(
        meshes
            .opaque_beam_draws
            .iter()
            .map(|draw| (draw.render_type, draw.index_start, draw.index_count))
            .collect::<Vec<_>>(),
        vec![
            (EntityModelLayerRenderType::EndCrystalBeam, 0, 48),
            (EntityModelLayerRenderType::EndCrystalBeam, 48, 48),
        ]
    );
}

#[test]
fn end_crystal_textured_submit_hides_base_when_shows_bottom_false() {
    let images = vec![blank_texture(END_CRYSTAL_TEXTURE_REF)];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let hidden = entity_model_textured_meshes(
        &[EntityModelInstance::end_crystal(450, [0.0, 64.0, 0.0], 0.0)
            .with(|s| s.kind.end_crystal_mut().shows_bottom = false)],
        &atlas,
    );

    assert_eq!(hidden.cutout.vertices.len(), 72);
    assert_eq!(hidden.cutout.indices.len(), 108);
}

#[test]
fn end_crystal_bob_matches_vanilla_get_y() {
    // Vanilla `EndCrystalRenderer.getY`: hh = sin(t·0.2)/2 + 0.5; hh = (hh² + hh)·0.4; return hh − 1.4.
    // The glass bob is `getY(age)·16/2`.
    for age in [0.0_f32, 7.5, 30.0, 100.0] {
        let hh = (age * 0.2).sin() / 2.0 + 0.5;
        let hh = (hh * hh + hh) * 0.4;
        let expected = hh - 1.4;
        assert!((end_crystal_get_y(age) - expected).abs() < 1.0e-6);
        assert!((end_crystal_bob_y(age) - expected * 8.0).abs() < 1.0e-6);
    }
    // getY is always negative — the crystal hovers above its base.
    assert!(end_crystal_get_y(0.0) < 0.0);
}

#[test]
fn end_crystal_glass_spin_matches_vanilla_setup_anim() {
    use glam::Vec3;

    // At age 0 the spin is identity, so both quaternions are the π/3 tilt about the (sin45, 0, sin45)
    // diagonal. Rotating +Y by 60° about that axis gives, by Rodrigues, (-0.61237, 0.5, 0.61237).
    let (outer0, inner0) = end_crystal_glass_quaternions(0.0);
    let up = outer0 * Vec3::Y;
    assert!((up.x - (-0.61237)).abs() < 1.0e-4, "x was {}", up.x);
    assert!((up.y - 0.5).abs() < 1.0e-4, "y was {}", up.y);
    assert!((up.z - 0.61237).abs() < 1.0e-4, "z was {}", up.z);
    // Both shells share the tilt when the spin is zero.
    assert!((outer0 * Vec3::Y).abs_diff_eq(inner0 * Vec3::Y, 1.0e-6));

    // Advancing the age spins the shells, and the outer (`Ry·TILT`) and inner (`TILT·Ry`) orders
    // diverge — the order distinction is the vanilla detail this proves.
    let (outer, inner) = end_crystal_glass_quaternions(30.0);
    assert!(
        !(outer * Vec3::Y).abs_diff_eq(outer0 * Vec3::Y, 1.0e-3),
        "the outer glass spins with age"
    );
    assert!(
        !(outer * Vec3::Y).abs_diff_eq(inner * Vec3::Y, 1.0e-3),
        "the outer and inner spin orders differ"
    );
}
