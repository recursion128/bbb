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

use crate::DynamicEntityTextureImage;

#[test]
fn happy_ghast_layer_passes_match_vanilla_renderer() {
    let passes = happy_ghast_textured_layer_passes(false);
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].texture, HAPPY_GHAST_TEXTURE_REF);
    // The vestigial `parts` slice is nulled; emit builds `HappyGhastModel::new()` and renders it.
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
}

#[test]
fn happy_ghast_texture_ref_matches_vanilla_renderer() {
    assert_eq!(HAPPY_GHAST_TEXTURE_REF.size, [64, 64]);
    assert_eq!(ModelLayer::HAPPY_GHAST, ModelLayer::HAPPY_GHAST);
    assert!(entity_model_texture_refs().contains(&HAPPY_GHAST_TEXTURE_REF));
    assert_eq!(HAPPY_GHAST_ROPES_TEXTURE_REF.size, [64, 64]);
    assert_eq!(ModelLayer::HAPPY_GHAST_ROPES, ModelLayer::HAPPY_GHAST_ROPES);
    assert_eq!(
        ModelLayer::HAPPY_GHAST_BABY_ROPES,
        ModelLayer::HAPPY_GHAST_BABY_ROPES
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

    let passes = happy_ghast_textured_layer_passes(true);
    assert_eq!(passes[0].texture, HAPPY_GHAST_BABY_TEXTURE_REF);
    let adult_mesh =
        entity_model_mesh(&[EntityModelInstance::happy_ghast(58, [0.0, 64.0, 0.0], 0.0)]);
    let baby_mesh = entity_model_mesh(&[baby]);
    assert_eq!(baby_mesh.cutout_faces, 66);
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
fn happy_ghast_harness_renders_for_both_ages_and_moves_ridden_goggles() {
    let (atlas, _) = build_entity_model_texture_atlas(&happy_ghast_texture_images()).unwrap();
    let adult = EntityModelInstance::happy_ghast(61, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.kind.happy_ghast_mut().body_equipment_asset =
            Some(test_happy_ghast_white_equipment_asset_id())
    });
    let unridden = entity_model_textured_meshes(&[adult], &atlas);

    assert_eq!(unridden.armor_cutout.cutout_faces, 12);
    assert_eq!(unridden.armor_cutout.vertices.len(), 48);

    let ridden = entity_model_textured_meshes(
        &[adult.with(|s| s.kind.happy_ghast_mut().ridden = true)],
        &atlas,
    );
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
    .with(|s| {
        s.kind.happy_ghast_mut().body_equipment_asset =
            Some(test_happy_ghast_black_equipment_asset_id())
    });
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_eq!(baby_meshes.armor_cutout.vertices.len(), 48);
    let rect = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == HAPPY_GHAST_HARNESS_BLACK_TEXTURE_REF)
        .unwrap()
        .uv;
    let uv = baby_meshes.armor_cutout.vertices[0].uv;
    assert!((rect.min[0]..=rect.max[0]).contains(&uv[0]));
    assert!((rect.min[1]..=rect.max[1]).contains(&uv[1]));
}

#[test]
fn happy_ghast_ropes_require_leash_holder_and_harness_tag_and_use_inflated_model() {
    let (atlas, _) = build_entity_model_texture_atlas(&happy_ghast_texture_images()).unwrap();
    let base = EntityModelInstance::happy_ghast(64, [0.0, 64.0, 0.0], 0.0);

    let no_tag = entity_model_textured_meshes(
        &[base.with(|s| s.kind.happy_ghast_mut().is_leash_holder = true)],
        &atlas,
    );
    let tagged_without_renderable_asset = base.with(|s| {
        s.kind.happy_ghast_mut().body_item = true;
        s.kind.happy_ghast_mut().body_item_is_harness = true;
        s.kind.happy_ghast_mut().is_leash_holder = true;
    });
    let tagged_meshes = entity_model_textured_meshes(&[tagged_without_renderable_asset], &atlas);
    assert_eq!(no_tag.cutout.vertices.len(), 240);
    assert_eq!(tagged_meshes.cutout.vertices.len(), 480);
    assert!(tagged_meshes.armor_cutout.vertices.is_empty());
    let harness = base.with(|s| {
        s.kind.happy_ghast_mut().body_equipment_asset =
            Some(test_happy_ghast_white_equipment_asset_id())
    });
    let no_holder = entity_model_textured_meshes(&[harness], &atlas);

    let base_cutout_vertex_count = no_holder.cutout.vertices.len();

    let ropes = entity_model_textured_meshes(
        &[harness.with(|s| {
            s.kind.happy_ghast_mut().body_item = true;
            s.kind.happy_ghast_mut().body_item_is_harness = true;
            s.kind.happy_ghast_mut().is_leash_holder = true;
        })],
        &atlas,
    );

    assert_eq!(ropes.cutout.vertices.len(), base_cutout_vertex_count * 2);
    assert_ne!(
        ropes.cutout.vertices[..24],
        ropes.cutout.vertices[base_cutout_vertex_count..base_cutout_vertex_count + 24],
        "the rope body is the base body inflated by CubeDeformation(0.2)"
    );
    let hidden = entity_model_textured_meshes(
        &[harness
            .with(|s| {
                s.kind.happy_ghast_mut().body_item = true;
                s.kind.happy_ghast_mut().body_item_is_harness = true;
                s.kind.happy_ghast_mut().is_leash_holder = true;
            })
            .with_invisible(true)
            .with(|s| s.invisible_to_player = true)],
        &atlas,
    );
    assert_eq!(hidden.armor_cutout.vertices.len(), 48);
    assert_eq!(hidden.cutout.vertices.len(), base_cutout_vertex_count);
    assert!(hidden
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay == ENTITY_VERTEX_NO_OVERLAY));
    let baby = EntityModelInstance::new(
        65,
        EntityModelKind::HappyGhast { baby: true },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| {
        s.kind.happy_ghast_mut().body_equipment_asset =
            Some(test_happy_ghast_black_equipment_asset_id());
        s.kind.happy_ghast_mut().body_item = true;
        s.kind.happy_ghast_mut().body_item_is_harness = true;
        s.kind.happy_ghast_mut().is_leash_holder = true;
    });
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_eq!(baby_meshes.armor_cutout.vertices.len(), 48);
    assert_eq!(baby_meshes.cutout.vertices.len(), 264 * 2);
}

#[test]
fn happy_ghast_harness_foil_and_trim_follow_the_base_layer() {
    let mut images = happy_ghast_texture_images();
    images.push(EntityModelTextureImage::new(
        ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
        vec![
            255;
            usize::try_from(
                ENCHANTED_GLINT_ARMOR_TEXTURE_REF.size[0]
                    * ENCHANTED_GLINT_ARMOR_TEXTURE_REF.size[1]
                    * 4
            )
            .unwrap()
        ],
    ));
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let trim_texture = EntityDynamicTexture { handle: 62_401 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [64, 64],
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 64 * 4],
    }])
    .unwrap()
    .0;
    let instance = EntityModelInstance::happy_ghast(63, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.kind.happy_ghast_mut().body_equipment_asset =
            Some(test_happy_ghast_blue_equipment_asset_id());
        s.kind.happy_ghast_mut().body_item = true;
        s.kind.happy_ghast_mut().body_item_is_harness = true;
        s.kind.happy_ghast_mut().is_leash_holder = true;
        s.kind.happy_ghast_mut().harness_foil = true;
        s.kind.happy_ghast_mut().harness_trim = Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        });
    });
    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[instance],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );
    assert_eq!(meshes.armor_cutout.vertices.len(), 48);
    assert_eq!(meshes.armor_entity_glint.vertices.len(), 48);
    assert_eq!(meshes.dynamic_player_texture_armor_trim.vertices.len(), 48);
    assert_eq!(meshes.cutout.vertices.len(), 480);
    let rect = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == HAPPY_GHAST_HARNESS_BLUE_TEXTURE_REF)
        .unwrap()
        .uv;
    let uv = meshes.armor_cutout.vertices[0].uv;
    assert!((rect.min[0]..=rect.max[0]).contains(&uv[0]));
    assert!((rect.min[1]..=rect.max[1]).contains(&uv[1]));
    assert!(meshes
        .armor_cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay == ENTITY_VERTEX_NO_OVERLAY));
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
fn happy_ghast_textured_mesh_uses_vanilla_uvs_and_scaling() {
    let (atlas, _) = build_entity_model_texture_atlas(&happy_ghast_texture_images()).unwrap();
    let instance = EntityModelInstance::happy_ghast(58, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.6;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert_happy_ghast_vertex_state(&meshes, instance);
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
    // top edge.
    assert_close2(meshes.cutout.vertices[0].uv, [0.5, 0.0]);
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
    let later = entity_model_mesh(&[base.with(|s| s.age_in_ticks = 31.4)]);
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
    assert_happy_ghast_vertex_state(&early_t, base);
    let later_instance = base.with(|s| s.age_in_ticks = 31.4);
    let later_t = entity_model_textured_meshes(&[later_instance], &atlas);
    assert_happy_ghast_vertex_state(&later_t, later_instance);
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
    let mut textures = texture_refs_matching(|path| {
        path.starts_with("textures/entity/ghast/happy_ghast")
            || path.starts_with("textures/entity/equipment/happy_ghast_body/")
    });
    textures.insert(2, HAPPY_GHAST_ROPES_TEXTURE_REF);
    textures
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn assert_happy_ghast_vertex_state(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
}
