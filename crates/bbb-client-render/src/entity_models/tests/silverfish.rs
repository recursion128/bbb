use super::*;

#[test]
fn silverfish_layer_passes_match_vanilla_renderer() {
    let passes = silverfish_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].texture, SILVERFISH_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
}

#[test]
fn silverfish_texture_ref_matches_vanilla_renderer() {
    assert!(entity_model_texture_refs().contains(&SILVERFISH_TEXTURE_REF));
}

#[test]
fn silverfish_segment_pose_matches_vanilla_setup_anim() {
    // Vanilla SilverfishModel.setupAnim: phase = ageInTicks*0.9 + i*0.15*pi,
    //   segment.yRot = cos(phase) * pi * 0.05 * (1 + |i - 2|)
    //   segment.x    = sin(phase) * pi * 0.2  * |i - 2|
    use std::f32::consts::PI;
    let vanilla = |base: PartPose, index: usize, age: f32| -> PartPose {
        let phase = age * 0.9 + index as f32 * 0.15 * PI;
        let dist = (index as i32 - 2).abs() as f32;
        PartPose {
            offset: [
                phase.sin() * PI * 0.2 * dist,
                base.offset[1],
                base.offset[2],
            ],
            rotation: [
                base.rotation[0],
                phase.cos() * PI * 0.05 * (1.0 + dist),
                base.rotation[2],
            ],
        }
    };
    for age in [0.0f32, 6.3, 21.0, 100.0] {
        for index in 0..SILVERFISH_SEGMENT_COUNT {
            let base = bind_pose(&format!("segment{}", index));
            let got = silverfish_segment_pose(base, index, age);
            assert_eq!(
                got,
                vanilla(base, index, age),
                "segment {index} at age {age}"
            );
        }
    }
    // The middle segment (index 2, |i-2| = 0) never shifts in x.
    for age in [0.0f32, 6.3, 21.0] {
        assert_eq!(
            silverfish_segment_pose(bind_pose(&format!("segment{}", 2)), 2, age).offset[0],
            0.0
        );
    }
}

#[test]
fn silverfish_layer_rules_copy_vanilla_source_segments() {
    // Vanilla: layer0.yRot = segment2.yRot (x untouched); layer1 copies segment4 yRot+x;
    // layer2 copies segment1 yRot+x.
    assert_eq!(SILVERFISH_LAYER_RULES, [(2, false), (4, true), (1, true)]);

    let age = 13.0f32;
    let seg2 = silverfish_segment_pose(bind_pose(&format!("segment{}", 2)), 2, age);
    let seg4 = silverfish_segment_pose(bind_pose(&format!("segment{}", 4)), 4, age);
    let seg1 = silverfish_segment_pose(bind_pose(&format!("segment{}", 1)), 1, age);

    // layer0: yRot follows seg2, x stays at the layer rest (segment2 has x == 0 anyway).
    let layer0 = silverfish_layer_pose(bind_pose(&format!("layer{}", 0)), seg2, false);
    assert_eq!(layer0.rotation[1], seg2.rotation[1]);
    assert_eq!(
        layer0.offset[0],
        bind_pose(&format!("layer{}", 0)).offset[0]
    );
    assert_eq!(
        layer0.offset[2],
        bind_pose(&format!("layer{}", 0)).offset[2]
    );

    // layer1: yRot AND x follow seg4.
    let layer1 = silverfish_layer_pose(bind_pose(&format!("layer{}", 1)), seg4, true);
    assert_eq!(layer1.rotation[1], seg4.rotation[1]);
    assert_eq!(layer1.offset[0], seg4.offset[0]);
    assert_ne!(layer1.offset[0], 0.0, "seg4 (dist 2) shifts in x at age 13");

    // layer2: yRot AND x follow seg1.
    let layer2 = silverfish_layer_pose(bind_pose(&format!("layer{}", 2)), seg1, true);
    assert_eq!(layer2.rotation[1], seg1.rotation[1]);
    assert_eq!(layer2.offset[0], seg1.offset[0]);
}

#[test]
fn silverfish_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&silverfish_texture_images()).unwrap();
    let instance = EntityModelInstance::silverfish(114, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (5_u32 << 4) | (13_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.cutout.cutout_faces, 60);
    assert_eq!(meshes.cutout.vertices.len(), 240);
    assert_eq!(meshes.cutout.indices.len(), 360);
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
    assert_close3(min, [-0.3198125, 64.001, -0.78584146]);
    assert_close3(max, [0.3198125, 64.501, 0.3169995]);
}

#[test]
fn silverfish_segments_wiggle_as_age_in_ticks_advances() {
    // Vanilla runs setupAnim every frame: all segments (and their overlay layers) wiggle by
    // ageInTicks. There is no rest pose (the age-0 phase already carries nonzero terms).
    let base = EntityModelInstance::silverfish(114, [0.0, 64.0, 0.0], 0.0);
    let early = entity_model_mesh(&[base]);
    let later = entity_model_mesh(&[base.with(|s| s.age_in_ticks = 20.0)]);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(
        early.vertices, later.vertices,
        "the segments wiggle as ageInTicks advances"
    );

    let (atlas, _) = build_entity_model_texture_atlas(&silverfish_texture_images()).unwrap();
    let early_t = entity_model_textured_meshes(&[base], &atlas);

    let later_instance = base.with(|s| s.age_in_ticks = 20.0);
    let later_t = entity_model_textured_meshes(&[later_instance], &atlas);

    assert_ne!(
        early_t.cutout.vertices, later_t.cutout.vertices,
        "the textured segments wiggle too"
    );
}

fn silverfish_texture_images() -> Vec<EntityModelTextureImage> {
    [SILVERFISH_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn bind_pose(name: &str) -> PartPose {
    ModelLayer::SILVERFISH
        .bake()
        .try_child(name)
        .unwrap()
        .default_pose()
}
