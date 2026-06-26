use super::*;

use crate::entity_models::colored::happy_ghast_model_root_transform;
use crate::entity_models::model::ModelCube;

#[test]
fn happy_ghast_cubes_match_vanilla_26_1_body_layer() {
    // Vanilla HappyGhastModel.createBodyLayer(false, NONE): a 16x16x16 body at y 16 plus nine
    // tentacles parented under the body (body offset y 16 + tentacle offset y 7 = y 23), with
    // hard-coded lengths. Each unified cube carries the colored tint (`HAPPY_GHAST_CREAM`) and the
    // textured UV (`texOffs(0, 0)`) in one struct.
    assert_eq!(
        HAPPY_GHAST_TENTACLE_LENGTHS,
        [5.0, 7.0, 4.0, 5.0, 5.0, 7.0, 8.0, 8.0, 5.0]
    );
    assert_eq!(
        HAPPY_GHAST_BODY_CUBE[0],
        ModelCube::new(
            [-8.0, -8.0, -8.0],
            [16.0, 16.0, 16.0],
            HAPPY_GHAST_CREAM,
            [16.0, 16.0, 16.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_close3(HAPPY_GHAST_BODY_POSE.offset, [0.0, 16.0, 0.0]);
    assert_eq!(HAPPY_GHAST_BODY_POSE.rotation, [0.0, 0.0, 0.0]);
}

#[test]
fn happy_ghast_tentacle_ring_layout_matches_vanilla() {
    // The nine tentacles hang at `HAPPY_GHAST_TENTACLE_OFFSETS[i]` (world y 23) with no bind
    // rotation, each a `box(-1, 0, -1, 2, len, 2)` at `texOffs(0, 0)` (reused for the body and
    // every tentacle). `uv_size == size` (no deformation).
    for index in 0..9 {
        let pose = happy_ghast_tentacle_pose(index);
        assert_eq!(pose.offset, HAPPY_GHAST_TENTACLE_OFFSETS[index]);
        assert_eq!(pose.offset[1], 23.0, "tentacle {index} world y");
        assert_eq!(pose.rotation, [0.0, 0.0, 0.0]);
        let cube = happy_ghast_tentacle_cube(index);
        assert_eq!(cube.min, [-1.0, 0.0, -1.0]);
        assert_eq!(cube.size, [2.0, HAPPY_GHAST_TENTACLE_LENGTHS[index], 2.0]);
        assert_eq!(
            cube.uv_size,
            [2.0, HAPPY_GHAST_TENTACLE_LENGTHS[index], 2.0]
        );
        assert_eq!(cube.tex, [0.0, 0.0]);
        assert_eq!(cube.color, HAPPY_GHAST_CREAM);
    }
}

#[test]
fn happy_ghast_layer_passes_match_vanilla_renderer() {
    let passes = happy_ghast_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::HappyGhastBase);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].model_layer, MODEL_LAYER_HAPPY_GHAST);
    assert_eq!(passes[0].texture, HAPPY_GHAST_TEXTURE_REF);
    // The vestigial `parts` slice is nulled; emit builds `HappyGhastModel::new()` and renders it.
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
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
    assert_eq!(HAPPY_GHAST_TEXTURE_REF.size, [64, 64]);
    assert_eq!(MODEL_LAYER_HAPPY_GHAST, "minecraft:happy_ghast#main");
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
    let instance = EntityModelInstance::happy_ghast(58, [0.0, 64.0, 0.0], 0.0);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert_happy_ghast_base_submission(&meshes, instance);
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
    // The body's first vertex samples u = 2*depth/width = 32/64 = 0.5 at the texOffs(0, 0)
    // top edge; the textured mesh shares the colored geometry's bounds.
    assert_close2(meshes.cutout.vertices[0].uv, [0.5, 0.0]);
    // Same geometry, scale, and transform as the colored path, so the extents match.
    let (min, max) = textured_mesh_extents(&meshes.cutout);
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
    let early_t = entity_model_textured_meshes(&[base], &atlas);
    assert_happy_ghast_base_submission(&early_t, base);
    let later_instance = base.with_age_in_ticks(31.4);
    let later_t = entity_model_textured_meshes(&[later_instance], &atlas);
    assert_happy_ghast_base_submission(&later_t, later_instance);
    assert_ne!(
        early_t.cutout.vertices, later_t.cutout.vertices,
        "the textured tentacles wave too"
    );
    assert_eq!(
        early_t.cutout.vertices[..24],
        later_t.cutout.vertices[..24],
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

fn assert_happy_ghast_base_submission(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.texture, HAPPY_GHAST_TEXTURE_REF);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.transform, happy_ghast_model_root_transform(instance));
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
}
