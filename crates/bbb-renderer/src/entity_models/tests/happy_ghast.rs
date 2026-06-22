use super::*;

#[test]
fn happy_ghast_parts_match_vanilla_26_1_body_layer() {
    // Vanilla HappyGhastModel.createBodyLayer(false, NONE): a 16x16x16 body at y 16 plus nine
    // tentacles parented under the body (body offset y 16 + tentacle offset y 7 = y 23), with
    // hard-coded lengths.
    assert_eq!(HAPPY_GHAST_PARTS.len(), 10);
    assert_eq!(
        HAPPY_GHAST_TENTACLE_LENGTHS,
        [5.0, 7.0, 4.0, 5.0, 5.0, 7.0, 8.0, 8.0, 5.0]
    );

    assert_part(
        &HAPPY_GHAST_PARTS[0],
        [0.0, 16.0, 0.0],
        [0.0, 0.0, 0.0],
        HAPPY_GHAST_BODY_CUBE.as_slice(),
    );
    assert_eq!(HAPPY_GHAST_BODY_CUBE[0].min, [-8.0, -8.0, -8.0]);
    assert_eq!(HAPPY_GHAST_BODY_CUBE[0].size, [16.0, 16.0, 16.0]);

    for index in 0..9 {
        let part = &HAPPY_GHAST_PARTS[index + 1];
        assert_eq!(part.pose.offset, HAPPY_GHAST_TENTACLE_OFFSETS[index]);
        assert_eq!(part.pose.offset[1], 23.0, "tentacle {index} world y");
        assert_eq!(part.pose.rotation, [0.0, 0.0, 0.0]);
        assert_eq!(part.cubes.len(), 1);
        assert_eq!(part.cubes[0].min, [-1.0, 0.0, -1.0]);
        assert_eq!(
            part.cubes[0].size,
            [2.0, HAPPY_GHAST_TENTACLE_LENGTHS[index], 2.0]
        );
    }
}

#[test]
fn happy_ghast_textured_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_HAPPY_GHAST, "minecraft:happy_ghast#main");
    assert_eq!(HAPPY_GHAST_TEXTURE_REF.size, [64, 64]);
    assert_eq!(HAPPY_GHAST_TEXTURED_PARTS.len(), 10);
    assert_eq!(
        HAPPY_GHAST_TEXTURED_BODY_CUBE[0],
        TexturedModelCubeDesc {
            min: [-8.0, -8.0, -8.0],
            size: [16.0, 16.0, 16.0],
            uv_size: [16.0, 16.0, 16.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    // Vanilla reuses texOffs(0, 0) for every tentacle, so each samples the top-left region.
    for index in 0..9 {
        let part = &HAPPY_GHAST_TEXTURED_PARTS[index + 1];
        assert_eq!(part.pose.offset, HAPPY_GHAST_TENTACLE_OFFSETS[index]);
        assert_eq!(part.cubes[0].tex, [0.0, 0.0]);
        assert_eq!(
            part.cubes[0].uv_size,
            [2.0, HAPPY_GHAST_TENTACLE_LENGTHS[index], 2.0]
        );
    }
    assert_eq!(HAPPY_GHAST_TEXTURED_PARTS[0].pose.offset, [0.0, 16.0, 0.0]);
}

#[test]
fn happy_ghast_layer_passes_match_vanilla_renderer() {
    let passes = happy_ghast_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::HappyGhastBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_HAPPY_GHAST);
    assert_eq!(passes[0].texture, HAPPY_GHAST_TEXTURE_REF);
    assert_eq!(passes[0].parts, HAPPY_GHAST_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
}

#[test]
fn happy_ghast_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::HappyGhast.model_key(), "happy_ghast");
    assert_eq!(
        EntityModelKind::HappyGhast.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/ghast/happy_ghast.png",
            size: [64, 64],
        })
    );
    assert!(entity_model_texture_refs().contains(&HAPPY_GHAST_TEXTURE_REF));
    assert_eq!(
        happy_ghast_entity_texture_refs(),
        &[HAPPY_GHAST_TEXTURE_REF]
    );
}

#[test]
fn happy_ghast_reuses_vanilla_ghast_tentacle_wave() {
    // Vanilla HappyGhastModel.setupAnim calls GhastModel.animateTentacles verbatim:
    // tentacle[i].xRot = 0.2 * sin(ageInTicks*0.3 + i) + 0.4 (never at rest).
    for &(index, age) in &[(0usize, 0.0f32), (4, 12.5), (8, 40.0)] {
        let expected = 0.2 * (age * 0.3 + index as f32).sin() + 0.4;
        assert!((ghast_tentacle_x_rot(index, age) - expected).abs() < 1e-7);
    }
}

#[test]
fn happy_ghast_mesh_uses_vanilla_scaled_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::happy_ghast(58, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(mesh.opaque_faces, 60);
    assert_eq!(mesh.vertices.len(), 240);
    assert_eq!(mesh.indices.len(), 360);
    // The 16px body scaled 4.0x by MeshTransformer.scaling spans +-2.0 blocks in X
    // (8px * 4.0 / 16); the tentacles hang below the origin at the age-0 resting tilt.
    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-2.0000002, 62.286907, -2.479002]);
    assert_close3(max, [2.0000002, 68.004, 2.0000002]);
}

#[test]
fn happy_ghast_textured_mesh_uses_vanilla_uvs_and_scaling() {
    let (atlas, _) = build_entity_model_texture_atlas(&happy_ghast_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::happy_ghast(58, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert_eq!(mesh.cutout_faces, 60);
    assert_eq!(mesh.vertices.len(), 240);
    assert_eq!(mesh.indices.len(), 360);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    // The body's first vertex samples u = 2*depth/width = 32/64 = 0.5 at the texOffs(0, 0)
    // top edge; the textured mesh shares the colored geometry's bounds.
    assert_close2(mesh.vertices[0].uv, [0.5, 0.0]);
    // Same geometry, scale, and transform as the colored path, so the extents match.
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-2.0000002, 62.286907, -2.479002]);
    assert_close3(max, [2.0000002, 68.004, 2.0000002]);
}

#[test]
fn happy_ghast_tentacles_wave_as_age_in_ticks_advances() {
    // Vanilla runs setupAnim every frame, waving the nine tentacles by ageInTicks while the
    // body holds still. The body is part 0 (vertices [0, 24)); the tentacles follow.
    let base = EntityModelInstance::happy_ghast(58, [0.0, 64.0, 0.0], 0.0);
    let early = entity_model_mesh(&[base]);
    let later = entity_model_mesh(&[base.with_age_in_ticks(31.4)]);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(
        early.vertices, later.vertices,
        "the tentacles wave as ageInTicks advances"
    );
    assert_eq!(
        early.vertices[..24],
        later.vertices[..24],
        "the body does not depend on ageInTicks"
    );

    let (atlas, _) = build_entity_model_texture_atlas(&happy_ghast_texture_images()).unwrap();
    let early_t = entity_model_textured_mesh(&[base], &atlas);
    let later_t = entity_model_textured_mesh(&[base.with_age_in_ticks(31.4)], &atlas);
    assert_ne!(
        early_t.vertices, later_t.vertices,
        "the textured tentacles wave too"
    );
    assert_eq!(
        early_t.vertices[..24],
        later_t.vertices[..24],
        "the textured body does not depend on ageInTicks"
    );
}

fn happy_ghast_texture_images() -> Vec<EntityModelTextureImage> {
    happy_ghast_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
