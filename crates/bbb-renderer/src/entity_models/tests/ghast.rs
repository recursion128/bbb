use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn ghast_cubes_match_vanilla_26_1_body_layer() {
    // Vanilla GhastModel.createBodyLayer: a 16x16x16 body at y 17.6 plus nine tentacles at
    // y 24.6, whose lengths come from a fixed-seed RandomSource(1660) (random.nextInt(7)+8)
    // and whose xz offsets come from the documented index formula. Each unified cube carries the
    // colored tint (`GHAST_WHITE`) and the textured UV (`texOffs(0, 0)`) in one struct.
    assert_eq!(
        GHAST_TENTACLE_LENGTHS,
        [8.0, 13.0, 9.0, 11.0, 11.0, 10.0, 12.0, 9.0, 12.0]
    );
    assert_eq!(
        GHAST_BODY_CUBE[0],
        ModelCube::new(
            [-8.0, -8.0, -8.0],
            [16.0, 16.0, 16.0],
            GHAST_WHITE,
            [16.0, 16.0, 16.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_close3(GHAST_BODY_POSE.offset, [0.0, 17.6, 0.0]);
    assert_eq!(GHAST_BODY_POSE.rotation, [0.0, 0.0, 0.0]);
}

#[test]
fn ghast_tentacle_ring_layout_matches_vanilla() {
    // The nine tentacles hang at `GHAST_TENTACLE_OFFSETS[i]` (`y = 24.6`) with no bind rotation, each
    // a `box(-1, 0, -1, 2, len, 2)` at `texOffs(0, 0)` (reused for the body and every tentacle, so
    // each samples the same top-left region). `uv_size == size` (no deformation).
    for index in 0..9 {
        let pose = ghast_tentacle_pose(index);
        assert_eq!(pose.offset, GHAST_TENTACLE_OFFSETS[index]);
        assert_eq!(pose.rotation, [0.0, 0.0, 0.0]);
        // The bind `xRot` is overwritten by `setup_anim`; the cube geometry/UV are stable.
        let cube = ghast_tentacle_cube(index);
        assert_eq!(cube.min, [-1.0, 0.0, -1.0]);
        assert_eq!(cube.size, [2.0, GHAST_TENTACLE_LENGTHS[index], 2.0]);
        assert_eq!(cube.uv_size, [2.0, GHAST_TENTACLE_LENGTHS[index], 2.0]);
        assert_eq!(cube.tex, [0.0, 0.0]);
        assert_eq!(cube.color, GHAST_WHITE);
    }
}

#[test]
fn ghast_layer_passes_match_vanilla_renderer() {
    let passes = ghast_textured_layer_passes(false);
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::GhastBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_GHAST);
    assert_eq!(passes[0].texture, GHAST_TEXTURE_REF);
    // The vestigial `parts` slice is nulled; emit builds `GhastModel::new()` and renders its tree.
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
    // Vanilla `GhastRenderer.getTextureLocation`: `isCharging` swaps to the shooting face.
    let charging = ghast_textured_layer_passes(true);
    assert_eq!(charging[0].texture, GHAST_SHOOTING_TEXTURE_REF);
}

#[test]
fn ghast_texture_ref_matches_vanilla_renderer() {
    assert_eq!(
        EntityModelKind::Ghast { charging: false }.model_key(),
        "ghast"
    );
    assert_eq!(
        EntityModelKind::Ghast { charging: false }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/ghast/ghast.png",
            size: [64, 32],
        })
    );
    // A charging ghast resolves the shooting texture (same model_key, only the texture differs).
    assert_eq!(
        EntityModelKind::Ghast { charging: true }.model_key(),
        "ghast"
    );
    assert_eq!(
        EntityModelKind::Ghast { charging: true }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/ghast/ghast_shooting.png",
            size: [64, 32],
        })
    );
    assert_eq!(GHAST_TEXTURE_REF.size, [64, 32]);
    assert_eq!(GHAST_SHOOTING_TEXTURE_REF.size, [64, 32]);
    assert_eq!(MODEL_LAYER_GHAST, "minecraft:ghast#main");
    assert!(entity_model_texture_refs().contains(&GHAST_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&GHAST_SHOOTING_TEXTURE_REF));
    assert_eq!(
        ghast_entity_texture_refs(),
        &[GHAST_TEXTURE_REF, GHAST_SHOOTING_TEXTURE_REF]
    );
}

#[test]
fn ghast_tentacle_x_rot_matches_vanilla_formula() {
    // Vanilla GhastModel.animateTentacles: tentacle[i].xRot = 0.2 * sin(ageInTicks*0.3 + i)
    // + 0.4. Driven purely by ageInTicks (never zero), so the tentacles always wave.
    for &(index, age) in &[(0usize, 0.0f32), (3, 12.5), (8, 40.0)] {
        let expected = 0.2 * (age * 0.3 + index as f32).sin() + 0.4;
        assert!((ghast_tentacle_x_rot(index, age) - expected).abs() < 1e-7);
    }
    // At ageInTicks 0 the tentacles already carry a nonzero tilt (sin(i) terms), so there is
    // no rest pose.
    assert!((ghast_tentacle_x_rot(0, 0.0) - 0.4).abs() < 1e-7);
    assert_ne!(ghast_tentacle_x_rot(1, 0.0), 0.0);
}

#[test]
fn ghast_mesh_uses_vanilla_scaled_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::ghast(57, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(mesh.opaque_faces, 60);
    assert_eq!(mesh.vertices.len(), 240);
    assert_eq!(mesh.indices.len(), 360);
    // The 16px body and the longest tentacles, scaled 4.5x by MeshTransformer.scaling: the
    // body spans +-2.25 blocks in X (8px * 4.5 / 16) and the tentacles hang well below the
    // entity origin (the model age is 0, so the resting tentacle tilt sets these bounds).
    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-2.2500002, 60.56373, -3.5384488]);
    assert_close3(max, [2.2500002, 68.054504, 2.2500002]);
}

#[test]
fn ghast_textured_mesh_uses_vanilla_uvs_and_scaling() {
    let (atlas, _) = build_entity_model_texture_atlas(&ghast_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::ghast(57, [0.0, 64.0, 0.0], 0.0)],
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
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-2.2500002, 60.56373, -3.5384488]);
    assert_close3(max, [2.2500002, 68.054504, 2.2500002]);
}

#[test]
fn ghast_tentacles_wave_as_age_in_ticks_advances() {
    // Vanilla runs setupAnim every frame, waving the nine tentacles by ageInTicks while the
    // body holds still. The body is part 0 (vertices [0, 24)); the tentacles follow.
    let base = EntityModelInstance::ghast(57, [0.0, 64.0, 0.0], 0.0);
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

    let (atlas, _) = build_entity_model_texture_atlas(&ghast_texture_images()).unwrap();
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

fn ghast_texture_images() -> Vec<EntityModelTextureImage> {
    ghast_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
