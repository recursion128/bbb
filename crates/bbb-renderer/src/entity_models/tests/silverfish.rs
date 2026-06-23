use super::*;

#[test]
fn silverfish_cubes_and_poses_match_vanilla_26_1_body_layer() {
    // Vanilla SilverfishModel.createBodyLayer: seven body segments (BODY_SIZES[i], each
    // addBox(-sx/2, 0, -sz/2, sx, sy, sz) at (0, 24 - sy, placement)) plus three overlay
    // layers riding segments 2/4/1. placement walks -3.5 -> -1.5 -> 1 -> 4 -> 7 -> 9.5 -> 11.5.
    // No MeshTransformer scaling. Each unified cube carries the colored tint (`SILVERFISH_GRAY`)
    // and the textured UV (`texOffs` / `uv_size`); `uv_size == size` and none are mirrored.
    assert_eq!(SILVERFISH_SEGMENT_COUNT, 7);
    assert_eq!(SILVERFISH_LAYER_COUNT, 3);
    assert_eq!(SILVERFISH_SEGMENT_CUBES.len(), 7);
    assert_eq!(SILVERFISH_LAYER_CUBES.len(), 3);

    // (offset, cube min, cube size, texOffs) for the seven segments then the three overlay
    // layers (Vanilla BODY_TEXS, then texOffs(20, 0/11/18) for the layers).
    let segments: [([f32; 3], [f32; 3], [f32; 3], [f32; 2]); 7] = [
        (
            [0.0, 22.0, -3.5],
            [-1.5, 0.0, -1.0],
            [3.0, 2.0, 2.0],
            [0.0, 0.0],
        ),
        (
            [0.0, 21.0, -1.5],
            [-2.0, 0.0, -1.0],
            [4.0, 3.0, 2.0],
            [0.0, 4.0],
        ),
        (
            [0.0, 20.0, 1.0],
            [-3.0, 0.0, -1.5],
            [6.0, 4.0, 3.0],
            [0.0, 9.0],
        ),
        (
            [0.0, 21.0, 4.0],
            [-1.5, 0.0, -1.5],
            [3.0, 3.0, 3.0],
            [0.0, 16.0],
        ),
        (
            [0.0, 22.0, 7.0],
            [-1.0, 0.0, -1.5],
            [2.0, 2.0, 3.0],
            [0.0, 22.0],
        ),
        (
            [0.0, 23.0, 9.5],
            [-1.0, 0.0, -1.0],
            [2.0, 1.0, 2.0],
            [11.0, 0.0],
        ),
        (
            [0.0, 23.0, 11.5],
            [-0.5, 0.0, -1.0],
            [1.0, 1.0, 2.0],
            [13.0, 4.0],
        ),
    ];
    for (index, (offset, min, size, tex)) in segments.iter().enumerate() {
        assert_eq!(
            SILVERFISH_SEGMENT_POSES[index].offset, *offset,
            "seg {index} offset"
        );
        assert_eq!(
            SILVERFISH_SEGMENT_POSES[index].rotation,
            [0.0, 0.0, 0.0],
            "seg {index} rotation"
        );
        let cube = SILVERFISH_SEGMENT_CUBES[index];
        assert_eq!(cube.min, *min, "seg {index} cube min");
        assert_eq!(cube.size, *size, "seg {index} cube size");
        assert_eq!(cube.uv_size, *size, "seg {index} uv_size");
        assert_eq!(cube.tex, *tex, "seg {index} texOffs");
        assert!(!cube.mirror, "seg {index} mirror");
    }

    // Overlay layers (layer0 rides seg2 z, layer1 rides seg4 z, layer2 rides seg1 z).
    let layers: [([f32; 3], [f32; 3], [f32; 3], [f32; 2]); 3] = [
        (
            [0.0, 16.0, 1.0],
            [-5.0, 0.0, -1.5],
            [10.0, 8.0, 3.0],
            [20.0, 0.0],
        ),
        (
            [0.0, 20.0, 7.0],
            [-3.0, 0.0, -1.5],
            [6.0, 4.0, 3.0],
            [20.0, 11.0],
        ),
        (
            [0.0, 19.0, -1.5],
            [-3.0, 0.0, -1.5],
            [6.0, 5.0, 2.0],
            [20.0, 18.0],
        ),
    ];
    for (index, (offset, min, size, tex)) in layers.iter().enumerate() {
        assert_eq!(
            SILVERFISH_LAYER_POSES[index].offset, *offset,
            "layer {index} offset"
        );
        let cube = SILVERFISH_LAYER_CUBES[index];
        assert_eq!(cube.min, *min, "layer {index} cube min");
        assert_eq!(cube.size, *size, "layer {index} cube size");
        assert_eq!(cube.uv_size, *size, "layer {index} uv_size");
        assert_eq!(cube.tex, *tex, "layer {index} texOffs");
        assert!(!cube.mirror, "layer {index} mirror");
    }
    assert_eq!(MODEL_LAYER_SILVERFISH, "minecraft:silverfish#main");
    assert_eq!(SILVERFISH_TEXTURE_REF.size, [64, 32]);
}

#[test]
fn silverfish_layer_passes_match_vanilla_renderer() {
    let passes = silverfish_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::SilverfishBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_SILVERFISH);
    assert_eq!(passes[0].texture, SILVERFISH_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
}

#[test]
fn silverfish_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Silverfish.model_key(), "silverfish");
    assert_eq!(
        EntityModelKind::Silverfish.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/silverfish/silverfish.png",
            size: [64, 32],
        })
    );
    assert!(entity_model_texture_refs().contains(&SILVERFISH_TEXTURE_REF));
    assert_eq!(silverfish_entity_texture_refs(), &[SILVERFISH_TEXTURE_REF]);
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
            let base = SILVERFISH_SEGMENT_POSES[index];
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
            silverfish_segment_pose(SILVERFISH_SEGMENT_POSES[2], 2, age).offset[0],
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
    let seg2 = silverfish_segment_pose(SILVERFISH_SEGMENT_POSES[2], 2, age);
    let seg4 = silverfish_segment_pose(SILVERFISH_SEGMENT_POSES[4], 4, age);
    let seg1 = silverfish_segment_pose(SILVERFISH_SEGMENT_POSES[1], 1, age);

    // layer0: yRot follows seg2, x stays at the layer rest (segment2 has x == 0 anyway).
    let layer0 = silverfish_layer_pose(SILVERFISH_LAYER_POSES[0], seg2, false);
    assert_eq!(layer0.rotation[1], seg2.rotation[1]);
    assert_eq!(layer0.offset[0], SILVERFISH_LAYER_POSES[0].offset[0]);
    assert_eq!(layer0.offset[2], SILVERFISH_LAYER_POSES[0].offset[2]);

    // layer1: yRot AND x follow seg4.
    let layer1 = silverfish_layer_pose(SILVERFISH_LAYER_POSES[1], seg4, true);
    assert_eq!(layer1.rotation[1], seg4.rotation[1]);
    assert_eq!(layer1.offset[0], seg4.offset[0]);
    assert_ne!(layer1.offset[0], 0.0, "seg4 (dist 2) shifts in x at age 13");

    // layer2: yRot AND x follow seg1.
    let layer2 = silverfish_layer_pose(SILVERFISH_LAYER_POSES[2], seg1, true);
    assert_eq!(layer2.rotation[1], seg1.rotation[1]);
    assert_eq!(layer2.offset[0], seg1.offset[0]);
}

#[test]
fn silverfish_mesh_uses_vanilla_body_layer_geometry() {
    // Seven segments + three layers = 10 cubes => 10*24 = 240 verts, 10*6 = 60 faces,
    // 10*36 = 360 indices.
    let mesh = entity_model_mesh(&[EntityModelInstance::silverfish(114, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(mesh.opaque_faces, 60);
    assert_eq!(mesh.vertices.len(), 240);
    assert_eq!(mesh.indices.len(), 360);
    // The widest part is the 10px layer0 (+-5px => +-0.3125 block, plus its age-0 yRot tilt),
    // the tall layer0 reaches up to model y 16px, and the segments span the body length front
    // to back (model z up to 12.5px, the 180-degree body yaw flips z).
    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.3198125, 64.001, -0.78584146]);
    assert_close3(max, [0.3198125, 64.501, 0.3169995]);
}

#[test]
fn silverfish_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&silverfish_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::silverfish(114, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert_eq!(mesh.cutout_faces, 60);
    assert_eq!(mesh.vertices.len(), 240);
    assert_eq!(mesh.indices.len(), 360);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-0.3198125, 64.001, -0.78584146]);
    assert_close3(max, [0.3198125, 64.501, 0.3169995]);
}

#[test]
fn silverfish_segments_wiggle_as_age_in_ticks_advances() {
    // Vanilla runs setupAnim every frame: all segments (and their overlay layers) wiggle by
    // ageInTicks. There is no rest pose (the age-0 phase already carries nonzero terms).
    let base = EntityModelInstance::silverfish(114, [0.0, 64.0, 0.0], 0.0);
    let early = entity_model_mesh(&[base]);
    let later = entity_model_mesh(&[base.with_age_in_ticks(20.0)]);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(
        early.vertices, later.vertices,
        "the segments wiggle as ageInTicks advances"
    );

    let (atlas, _) = build_entity_model_texture_atlas(&silverfish_texture_images()).unwrap();
    let early_t = entity_model_textured_mesh(&[base], &atlas);
    let later_t = entity_model_textured_mesh(&[base.with_age_in_ticks(20.0)], &atlas);
    assert_ne!(
        early_t.vertices, later_t.vertices,
        "the textured segments wiggle too"
    );
}

fn silverfish_texture_images() -> Vec<EntityModelTextureImage> {
    silverfish_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
