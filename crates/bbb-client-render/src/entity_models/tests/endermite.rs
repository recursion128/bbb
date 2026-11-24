use super::*;

#[test]
fn endermite_layer_passes_match_vanilla_renderer() {
    let passes = endermite_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].texture, ENDERMITE_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
}

#[test]
fn endermite_texture_ref_matches_vanilla_renderer() {
    assert!(entity_model_texture_refs().contains(&ENDERMITE_TEXTURE_REF));
}

#[test]
fn endermite_segment_pose_matches_vanilla_setup_anim() {
    // Vanilla EndermiteModel.setupAnim: phase = ageInTicks*0.9 + i*0.15*pi,
    //   segment.yRot = cos(phase) * pi * 0.01 * (1 + |i - 2|)
    //   segment.x    = sin(phase) * pi * 0.1  * |i - 2|
    // Only x and yRot change; the rest offset.y/offset.z and zero xRot/zRot are preserved.
    use std::f32::consts::PI;
    let vanilla = |base: PartPose, index: usize, age: f32| -> PartPose {
        let phase = age * 0.9 + index as f32 * 0.15 * PI;
        let dist = (index as i32 - 2).abs() as f32;
        PartPose {
            offset: [
                phase.sin() * PI * 0.1 * dist,
                base.offset[1],
                base.offset[2],
            ],
            rotation: [
                base.rotation[0],
                phase.cos() * PI * 0.01 * (1.0 + dist),
                base.rotation[2],
            ],
        }
    };
    for age in [0.0f32, 6.3, 21.0, 100.0] {
        for index in 0..ENDERMITE_SEGMENT_COUNT {
            let base = bind_pose(&format!("segment{}", index));
            let got = endermite_segment_pose(base, index, age);
            let want = vanilla(base, index, age);
            assert_eq!(got, want, "segment {index} at age {age}");
        }
    }
    // The middle segment (index 2, |i-2| = 0) never shifts in x.
    for age in [0.0f32, 6.3, 21.0] {
        let base = bind_pose(&format!("segment{}", 2));
        assert_eq!(endermite_segment_pose(base, 2, age).offset[0], 0.0);
    }
    // Segment 0's phase is 0 at age 0, so its x is 0 but it still carries a nonzero yRot tilt
    // (cos(0) term) — the endermite never sits flat at its layer pose.
    let seg0 = endermite_segment_pose(bind_pose(&format!("segment{}", 0)), 0, 0.0);
    assert_eq!(seg0.offset[0], 0.0);
    assert_ne!(seg0.rotation[1], 0.0);
    // Segment 1 (phase = 0.15*pi at age 0) already shifts in x.
    assert_ne!(
        endermite_segment_pose(bind_pose(&format!("segment{}", 1)), 1, 0.0).offset[0],
        0.0
    );
}

#[test]
fn endermite_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&endermite_texture_images()).unwrap();
    let instance = EntityModelInstance::endermite(55, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (4_u32 << 4) | (13_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.cutout.cutout_faces, 24);
    assert_eq!(meshes.cutout.vertices.len(), 96);
    assert_eq!(meshes.cutout.indices.len(), 144);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
    assert_ne!(instance.render_state.overlay_coords(), [0.0, 10.0]);
    let (min, max) = textured_mesh_extents(&meshes.cutout);
    assert_close3(min, [-0.18703502, 64.001, -0.28155565]);
    assert_close3(max, [0.20486319, 64.251, 0.29273614]);
}

#[test]
fn endermite_segments_wiggle_as_age_in_ticks_advances() {
    // Vanilla runs setupAnim every frame: all four segments wiggle by ageInTicks. There is no
    // rest pose (the age-0 phase already carries nonzero cos/sin terms).
    let base = EntityModelInstance::endermite(55, [0.0, 64.0, 0.0], 0.0);
    let early = entity_model_mesh(&[base]);
    let later = entity_model_mesh(&[base.with(|s| s.age_in_ticks = 20.0)]);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(
        early.vertices, later.vertices,
        "the segments wiggle as ageInTicks advances"
    );

    let (atlas, _) = build_entity_model_texture_atlas(&endermite_texture_images()).unwrap();
    let early_t = entity_model_textured_meshes(&[base], &atlas);

    let later_instance = base.with(|s| s.age_in_ticks = 20.0);
    let later_t = entity_model_textured_meshes(&[later_instance], &atlas);

    assert_ne!(
        early_t.cutout.vertices, later_t.cutout.vertices,
        "the textured segments wiggle too"
    );
}

fn endermite_texture_images() -> Vec<EntityModelTextureImage> {
    [ENDERMITE_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn bind_pose(name: &str) -> PartPose {
    ModelLayer::ENDERMITE
        .bake()
        .try_child(name)
        .unwrap()
        .default_pose()
}
