use super::*;

fn entity_model_textured_meshes(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    super::entity_model_textured_meshes_with_static_equipment_fixture(instances, atlas)
}

fn entity_model_textured_meshes_with_dynamic_textures(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    skins: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    textures: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> EntityModelTexturedMeshes {
    super::entity_model_textured_meshes_with_static_equipment_and_dynamic_textures_fixture(
        instances, atlas, skins, textures,
    )
}

use crate::entity_models::colored::happy_ghast_model_root_transform;
use crate::entity_models::model::ModelCube;
use crate::DynamicEntityTextureImage;

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
}

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
    // The nine tentacles hang at local y 7 under the body at y 16 (world model y 23), preserving
    // the vanilla subtree needed by the BODY-item squeeze.
    for index in 0..9 {
        let pose = happy_ghast_tentacle_pose(index);
        assert_eq!(pose.offset, HAPPY_GHAST_TENTACLE_OFFSETS[index]);
        assert_eq!(pose.offset[1], 7.0, "tentacle {index} body-local y");
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
    let passes = happy_ghast_textured_layer_passes(false);
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
    assert_eq!(
        EntityModelKind::HappyGhast { baby: false }.model_key(),
        "happy_ghast"
    );
    assert_eq!(
        EntityModelKind::HappyGhast { baby: false }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/ghast/happy_ghast.png",
            size: [64, 64],
        })
    );
    assert_eq!(HAPPY_GHAST_TEXTURE_REF.size, [64, 64]);
    assert_eq!(MODEL_LAYER_HAPPY_GHAST, "minecraft:happy_ghast#main");
    assert!(entity_model_texture_refs().contains(&HAPPY_GHAST_TEXTURE_REF));
    assert_eq!(happy_ghast_entity_texture_refs().len(), 19);
    assert_eq!(
        happy_ghast_entity_texture_refs()[0],
        HAPPY_GHAST_TEXTURE_REF
    );
    assert_eq!(
        happy_ghast_entity_texture_refs()[1],
        HAPPY_GHAST_BABY_TEXTURE_REF
    );
    assert_eq!(
        happy_ghast_entity_texture_refs()[2],
        HAPPY_GHAST_ROPES_TEXTURE_REF
    );
    assert_eq!(HAPPY_GHAST_ROPES_TEXTURE_REF.size, [64, 64]);
    assert_eq!(
        MODEL_LAYER_HAPPY_GHAST_ROPES,
        "minecraft:happy_ghast_ropes#main"
    );
    assert_eq!(
        MODEL_LAYER_HAPPY_GHAST_BABY_ROPES,
        "minecraft:happy_ghast_baby_ropes#main"
    );
    assert!(entity_model_texture_refs().contains(&HAPPY_GHAST_HARNESS_WHITE_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&HAPPY_GHAST_HARNESS_BLACK_TEXTURE_REF));
}

#[test]
fn happy_ghast_baby_uses_baby_texture_inner_body_and_scale() {
    let baby = EntityModelInstance::new(
        59,
        EntityModelKind::HappyGhast { baby: true },
        [0.0, 64.0, 0.0],
        0.0,
    );
    assert_eq!(baby.kind.model_key(), "happy_ghast_baby");
    assert_eq!(
        baby.kind.vanilla_texture_ref(),
        Some(HAPPY_GHAST_BABY_TEXTURE_REF)
    );
    assert_eq!(
        HAPPY_GHAST_BABY_INNER_BODY_CUBE[0],
        ModelCube::new(
            [-7.5, -15.5, -7.5],
            [15.0, 15.0, 15.0],
            HAPPY_GHAST_CREAM,
            [16.0, 16.0, 16.0],
            [0.0, 32.0],
            false,
        )
    );
    assert_eq!(HAPPY_GHAST_BABY_INNER_BODY_POSE.offset, [0.0, 8.0, 0.0]);

    let passes = happy_ghast_textured_layer_passes(true);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_HAPPY_GHAST_BABY);
    assert_eq!(passes[0].texture, HAPPY_GHAST_BABY_TEXTURE_REF);
    let adult_mesh =
        entity_model_mesh(&[EntityModelInstance::happy_ghast(58, [0.0, 64.0, 0.0], 0.0)]);
    let baby_mesh = entity_model_mesh(&[baby]);
    assert_eq!(baby_mesh.opaque_faces, 66);
    assert_eq!(baby_mesh.vertices.len(), 264);
    let (adult_min, adult_max) = mesh_extents(&adult_mesh);
    let (baby_min, baby_max) = mesh_extents(&baby_mesh);
    assert_close3(
        [
            (baby_max[0] - baby_min[0]) / (adult_max[0] - adult_min[0]),
            (baby_max[1] - baby_min[1]) / (adult_max[1] - adult_min[1]),
            (baby_max[2] - baby_min[2]) / (adult_max[2] - adult_min[2]),
        ],
        [0.2375, 0.2375, 0.2375],
    );
}

#[test]
fn happy_ghast_body_item_squeezes_the_complete_body_subtree() {
    let bare = EntityModelInstance::happy_ghast(60, [0.0, 64.0, 0.0], 0.0);
    let equipped = bare.with_happy_ghast_body_item(true);
    let bare_mesh = entity_model_mesh(&[bare]);
    let equipped_mesh = entity_model_mesh(&[equipped]);
    assert_eq!(bare_mesh.opaque_faces, equipped_mesh.opaque_faces);
    let (bare_min, bare_max) = mesh_extents(&bare_mesh);
    let (equipped_min, equipped_max) = mesh_extents(&equipped_mesh);
    for axis in 0..3 {
        assert_close(
            equipped_max[axis] - equipped_min[axis],
            (bare_max[axis] - bare_min[axis]) * 0.9375,
        );
    }
    assert_ne!(
        bare_mesh.vertices[24..],
        equipped_mesh.vertices[24..],
        "tentacles inherit the body squeeze"
    );
}

#[test]
fn happy_ghast_harness_renders_for_both_ages_and_moves_ridden_goggles() {
    let (atlas, _) = build_entity_model_texture_atlas(&happy_ghast_texture_images()).unwrap();
    let adult = EntityModelInstance::happy_ghast(61, [0.0, 64.0, 0.0], 0.0)
        .with_happy_ghast_body_equipment_asset(Some(test_happy_ghast_white_equipment_asset_id()));
    let unridden = entity_model_textured_meshes(&[adult], &atlas);
    assert_eq!(unridden.submissions.len(), 2);
    assert_eq!(
        unridden.submissions[1].render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!(
        unridden.submissions[1].texture,
        HAPPY_GHAST_HARNESS_WHITE_TEXTURE_REF
    );
    assert_eq!(
        (
            unridden.submissions[1].order,
            unridden.submissions[1].submit_sequence
        ),
        (0, 1)
    );
    assert_eq!(unridden.armor_cutout.cutout_faces, 12);
    assert_eq!(unridden.armor_cutout.vertices.len(), 48);

    let ridden = entity_model_textured_meshes(&[adult.with_happy_ghast_ridden(true)], &atlas);
    assert_eq!(ridden.armor_cutout.vertices.len(), 48);
    assert_ne!(
        unridden.armor_cutout.vertices[24..],
        ridden.armor_cutout.vertices[24..],
        "goggles change from the raised -45 degree pose to the lowered ridden pose"
    );

    let baby = EntityModelInstance::new(
        62,
        EntityModelKind::HappyGhast { baby: true },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with_happy_ghast_body_equipment_asset(Some(test_happy_ghast_black_equipment_asset_id()));
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_eq!(baby_meshes.submissions.len(), 2);
    assert_eq!(
        baby_meshes.submissions[0].texture,
        HAPPY_GHAST_BABY_TEXTURE_REF
    );
    assert_eq!(
        baby_meshes.submissions[1].texture,
        HAPPY_GHAST_HARNESS_BLACK_TEXTURE_REF
    );
    assert_eq!(
        baby_meshes.submissions[1].transform,
        happy_ghast_model_root_transform(baby, true)
    );
}

#[test]
fn happy_ghast_ropes_require_leash_holder_and_harness_tag_and_use_inflated_model() {
    let (atlas, _) = build_entity_model_texture_atlas(&happy_ghast_texture_images()).unwrap();
    let base = EntityModelInstance::happy_ghast(64, [0.0, 64.0, 0.0], 0.0);
    let no_tag =
        entity_model_textured_meshes(&[base.with_happy_ghast_is_leash_holder(true)], &atlas);
    assert_eq!(no_tag.submissions.len(), 1);

    let tagged_without_renderable_asset = base
        .with_happy_ghast_body_item(true)
        .with_happy_ghast_body_item_is_harness(true)
        .with_happy_ghast_is_leash_holder(true);
    let tagged_meshes = entity_model_textured_meshes(&[tagged_without_renderable_asset], &atlas);
    assert_eq!(tagged_meshes.submissions.len(), 2);
    assert_eq!(
        tagged_meshes.submissions[1].texture,
        HAPPY_GHAST_ROPES_TEXTURE_REF
    );

    let harness = base
        .with_happy_ghast_body_equipment_asset(Some(test_happy_ghast_white_equipment_asset_id()));
    let no_holder = entity_model_textured_meshes(&[harness], &atlas);
    assert_eq!(no_holder.submissions.len(), 2);
    let base_cutout_vertex_count = no_holder.cutout.vertices.len();

    let ropes = entity_model_textured_meshes(
        &[harness
            .with_happy_ghast_body_item(true)
            .with_happy_ghast_body_item_is_harness(true)
            .with_happy_ghast_is_leash_holder(true)],
        &atlas,
    );
    assert_eq!(ropes.submissions.len(), 3);
    let rope = ropes.submissions[2];
    assert_eq!(rope.texture, HAPPY_GHAST_ROPES_TEXTURE_REF);
    assert_eq!(rope.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!((rope.order, rope.submit_sequence), (0, 2));
    assert_eq!(rope.overlay, ENTITY_VERTEX_NO_OVERLAY);
    assert_eq!(ropes.cutout.vertices.len(), base_cutout_vertex_count * 2);
    assert_ne!(
        ropes.cutout.vertices[..24],
        ropes.cutout.vertices[base_cutout_vertex_count..base_cutout_vertex_count + 24],
        "the rope body is the base body inflated by CubeDeformation(0.2)"
    );

    let hidden = entity_model_textured_meshes(
        &[harness
            .with_happy_ghast_body_item(true)
            .with_happy_ghast_body_item_is_harness(true)
            .with_happy_ghast_is_leash_holder(true)
            .with_invisible(true)
            .with_invisible_to_player(true)],
        &atlas,
    );
    assert_eq!(hidden.submissions.len(), 2);
    assert_eq!(
        hidden.submissions[0].texture,
        HAPPY_GHAST_HARNESS_WHITE_TEXTURE_REF
    );
    assert_eq!(hidden.submissions[1].texture, HAPPY_GHAST_ROPES_TEXTURE_REF);
    assert_eq!(hidden.submissions[0].submit_sequence, 0);
    assert_eq!(hidden.submissions[1].submit_sequence, 1);

    let baby = EntityModelInstance::new(
        65,
        EntityModelKind::HappyGhast { baby: true },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with_happy_ghast_body_equipment_asset(Some(test_happy_ghast_black_equipment_asset_id()))
    .with_happy_ghast_body_item(true)
    .with_happy_ghast_body_item_is_harness(true)
    .with_happy_ghast_is_leash_holder(true);
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_eq!(baby_meshes.submissions.len(), 3);
    assert_eq!(
        baby_meshes.submissions[2].texture,
        HAPPY_GHAST_ROPES_TEXTURE_REF
    );
    assert_eq!(
        baby_meshes.submissions[2].transform,
        happy_ghast_model_root_transform(baby, true)
    );
}

#[test]
fn happy_ghast_harness_foil_and_trim_follow_the_base_layer() {
    let (atlas, _) = build_entity_model_texture_atlas(&happy_ghast_texture_images()).unwrap();
    let trim_texture = EntityDynamicTexture { handle: 62_401 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [64, 64],
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 64 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::happy_ghast(63, [0.0, 64.0, 0.0], 0.0)
        .with_happy_ghast_body_equipment_asset(Some(test_happy_ghast_blue_equipment_asset_id()))
        .with_happy_ghast_body_item(true)
        .with_happy_ghast_body_item_is_harness(true)
        .with_happy_ghast_is_leash_holder(true)
        .with_happy_ghast_harness_foil(true)
        .with_happy_ghast_harness_trim(Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        }));
    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );
    assert_eq!(meshes.submissions.len(), 5);
    let [base, harness, foil, trim, ropes] = meshes.submissions.as_slice() else {
        panic!("expected base, harness, foil, trim, and ropes")
    };
    assert_eq!(base.texture, HAPPY_GHAST_TEXTURE_REF);
    assert_eq!(harness.texture, HAPPY_GHAST_HARNESS_BLUE_TEXTURE_REF);
    assert_eq!(
        harness.render_type,
        EntityModelLayerRenderType::ArmorCutoutNoCull
    );
    assert_eq!((harness.order, harness.submit_sequence), (0, 1));
    assert_eq!(foil.texture, ENCHANTED_GLINT_ARMOR_TEXTURE_REF);
    assert_eq!(
        foil.render_type,
        EntityModelLayerRenderType::ArmorEntityGlint
    );
    assert_eq!((foil.order, foil.submit_sequence), (1, 2));
    assert_eq!(trim.texture, HAPPY_GHAST_HARNESS_BLUE_TEXTURE_REF);
    assert_eq!(trim.dynamic_texture, Some(trim_texture));
    assert_eq!(
        trim.render_type,
        EntityModelLayerRenderType::ArmorTrimCutoutNoCull
    );
    assert_eq!((trim.order, trim.submit_sequence), (2, 3));
    assert_eq!(ropes.texture, HAPPY_GHAST_ROPES_TEXTURE_REF);
    assert_eq!(ropes.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!((ropes.order, ropes.submit_sequence), (0, 4));
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
    let instance = EntityModelInstance::happy_ghast(58, [0.0, 64.0, 0.0], 0.0)
        .with_light_coords((4_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.6)
        .with_has_red_overlay(true);
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
    assert_eq!(
        submit.transform,
        happy_ghast_model_root_transform(instance, false)
    );
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    assert_eq!(submit.light, instance.render_state.shader_light());
    assert_eq!(submit.overlay, instance.render_state.overlay_coords());
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == submit.light && vertex.overlay == submit.overlay));
}
