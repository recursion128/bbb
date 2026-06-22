use super::*;

#[test]
fn endermite_parts_match_vanilla_26_1_body_layer() {
    // Vanilla EndermiteModel.createBodyLayer: four chitin segments, each a box of
    // BODY_SIZES[i] = (sx, sy, sz) at addBox(-sx/2, 0, -sz/2, sx, sy, sz) posed at
    // (0, 24 - sy, placement), where placement walks -3.5 -> 0 -> 3 -> 4 by half the summed
    // depths of adjacent segments. No MeshTransformer scaling.
    assert_eq!(ENDERMITE_PARTS.len(), 4);
    assert_eq!(ENDERMITE_SEGMENT_COUNT, 4);

    let expected: [([f32; 3], [f32; 3], [f32; 3]); 4] = [
        // (offset, cube min, cube size)
        ([0.0, 21.0, -3.5], [-2.0, 0.0, -1.0], [4.0, 3.0, 2.0]),
        ([0.0, 20.0, 0.0], [-3.0, 0.0, -2.5], [6.0, 4.0, 5.0]),
        ([0.0, 21.0, 3.0], [-1.5, 0.0, -0.5], [3.0, 3.0, 1.0]),
        ([0.0, 22.0, 4.0], [-0.5, 0.0, -0.5], [1.0, 2.0, 1.0]),
    ];
    for (index, (offset, min, size)) in expected.iter().enumerate() {
        let part = &ENDERMITE_PARTS[index];
        assert_eq!(part.pose.offset, *offset);
        assert_eq!(part.pose.rotation, [0.0, 0.0, 0.0]);
        assert_eq!(part.cubes.len(), 1);
        assert_eq!(part.cubes[0].min, *min);
        assert_eq!(part.cubes[0].size, *size);
    }
}

#[test]
fn endermite_textured_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_ENDERMITE, "minecraft:endermite#main");
    assert_eq!(ENDERMITE_TEXTURE_REF.size, [64, 32]);
    assert_eq!(ENDERMITE_TEXTURED_PARTS.len(), 4);

    // Vanilla BODY_TEXS: segment i samples texOffs(0, [0, 5, 14, 18][i]) at its full size.
    let expected: [([f32; 3], [f32; 2], [f32; 3]); 4] = [
        // (cube min, texOffs, uv_size)
        ([-2.0, 0.0, -1.0], [0.0, 0.0], [4.0, 3.0, 2.0]),
        ([-3.0, 0.0, -2.5], [0.0, 5.0], [6.0, 4.0, 5.0]),
        ([-1.5, 0.0, -0.5], [0.0, 14.0], [3.0, 3.0, 1.0]),
        ([-0.5, 0.0, -0.5], [0.0, 18.0], [1.0, 2.0, 1.0]),
    ];
    for (index, (min, tex, uv_size)) in expected.iter().enumerate() {
        let part = &ENDERMITE_TEXTURED_PARTS[index];
        assert_eq!(part.pose.offset, ENDERMITE_PARTS[index].pose.offset);
        assert_eq!(
            part.cubes[0],
            TexturedModelCubeDesc {
                min: *min,
                size: *uv_size,
                uv_size: *uv_size,
                tex: *tex,
                mirror: false,
            }
        );
    }
}

#[test]
fn endermite_layer_passes_match_vanilla_renderer() {
    let passes = endermite_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::EndermiteBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_ENDERMITE);
    assert_eq!(passes[0].texture, ENDERMITE_TEXTURE_REF);
    assert_eq!(passes[0].parts, ENDERMITE_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
}

#[test]
fn endermite_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Endermite.model_key(), "endermite");
    assert_eq!(
        EntityModelKind::Endermite.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/endermite/endermite.png",
            size: [64, 32],
        })
    );
    assert!(entity_model_texture_refs().contains(&ENDERMITE_TEXTURE_REF));
    assert_eq!(endermite_entity_texture_refs(), &[ENDERMITE_TEXTURE_REF]);
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
            let base = ENDERMITE_PARTS[index].pose;
            let got = endermite_segment_pose(base, index, age);
            let want = vanilla(base, index, age);
            assert_eq!(got, want, "segment {index} at age {age}");
        }
    }
    // The middle segment (index 2, |i-2| = 0) never shifts in x.
    for age in [0.0f32, 6.3, 21.0] {
        let base = ENDERMITE_PARTS[2].pose;
        assert_eq!(endermite_segment_pose(base, 2, age).offset[0], 0.0);
    }
    // Segment 0's phase is 0 at age 0, so its x is 0 but it still carries a nonzero yRot tilt
    // (cos(0) term) — the endermite never sits flat at its layer pose.
    let seg0 = endermite_segment_pose(ENDERMITE_PARTS[0].pose, 0, 0.0);
    assert_eq!(seg0.offset[0], 0.0);
    assert_ne!(seg0.rotation[1], 0.0);
    // Segment 1 (phase = 0.15*pi at age 0) already shifts in x.
    assert_ne!(
        endermite_segment_pose(ENDERMITE_PARTS[1].pose, 1, 0.0).offset[0],
        0.0
    );
}

#[test]
fn endermite_mesh_uses_vanilla_body_layer_geometry() {
    // Four segments = 4 cubes => 4*24 = 96 verts, 4*6 = 24 faces, 4*36 = 144 indices.
    let mesh = entity_model_mesh(&[EntityModelInstance::endermite(55, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(mesh.opaque_faces, 24);
    assert_eq!(mesh.vertices.len(), 96);
    assert_eq!(mesh.indices.len(), 144);
    // The widest segment is the 6px-wide middle (segment 1: model x +-3px => +-0.1875 block,
    // plus the age-0 x wiggle and yRot tilt), and the segments span the body length front to
    // back (model z +-4.5px, the 180-degree body yaw flips z).
    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.18703502, 64.001, -0.28155565]);
    assert_close3(max, [0.20486319, 64.251, 0.29273614]);
}

#[test]
fn endermite_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&endermite_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::endermite(55, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert_eq!(mesh.cutout_faces, 24);
    assert_eq!(mesh.vertices.len(), 96);
    assert_eq!(mesh.indices.len(), 144);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-0.18703502, 64.001, -0.28155565]);
    assert_close3(max, [0.20486319, 64.251, 0.29273614]);
}

#[test]
fn endermite_segments_wiggle_as_age_in_ticks_advances() {
    // Vanilla runs setupAnim every frame: all four segments wiggle by ageInTicks. There is no
    // rest pose (the age-0 phase already carries nonzero cos/sin terms).
    let base = EntityModelInstance::endermite(55, [0.0, 64.0, 0.0], 0.0);
    let early = entity_model_mesh(&[base]);
    let later = entity_model_mesh(&[base.with_age_in_ticks(20.0)]);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(
        early.vertices, later.vertices,
        "the segments wiggle as ageInTicks advances"
    );

    let (atlas, _) = build_entity_model_texture_atlas(&endermite_texture_images()).unwrap();
    let early_t = entity_model_textured_mesh(&[base], &atlas);
    let later_t = entity_model_textured_mesh(&[base.with_age_in_ticks(20.0)], &atlas);
    assert_ne!(
        early_t.vertices, later_t.vertices,
        "the textured segments wiggle too"
    );
}

fn endermite_texture_images() -> Vec<EntityModelTextureImage> {
    endermite_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
