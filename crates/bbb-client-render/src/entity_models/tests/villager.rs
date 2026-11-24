use super::*;
use crate::entity_models::model_layers::villager_model;

#[test]
fn villager_and_wandering_trader_texture_refs_match_vanilla_renderers() {
    assert!(entity_model_texture_refs().contains(&VILLAGER_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&VILLAGER_BABY_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&WANDERING_TRADER_TEXTURE_REF));
}

#[test]
fn villager_textured_layer_passes_match_vanilla_renderer_model_layers() {
    let adult = villager_textured_layer_passes(false);
    let baby = villager_textured_layer_passes(true);
    let trader = wandering_trader_textured_layer_passes();

    assert_eq!(adult.len(), 1);
    assert_eq!(
        adult[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(adult[0].texture, VILLAGER_TEXTURE_REF);
    assert_eq!(adult[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(adult[0].order, 0);

    assert_eq!(baby.len(), 1);
    assert_eq!(
        baby[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(baby[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(baby[0].texture, VILLAGER_BABY_TEXTURE_REF);
    assert_eq!(baby[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(baby[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(baby[0].order, 0);

    let farmer =
        VillagerModelData::new(VillagerModelType::Snow, VillagerModelProfession::Farmer, 4);
    let overlays = villager_data_textured_layer_passes(false, farmer);
    assert_eq!(overlays.len(), 3);
    assert_eq!(overlays[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(
        overlays[0].texture,
        villager_type_texture_ref(VillagerModelType::Snow, false)
    );
    assert_eq!(overlays[0].order, 1);
    assert_eq!(
        overlays[1].texture,
        villager_profession_texture_ref(VillagerModelProfession::Farmer).unwrap()
    );
    assert_eq!(overlays[1].order, 2);
    assert_eq!(overlays[2].texture, villager_level_texture_ref(4));
    assert_eq!(overlays[2].order, 3);

    let baby_overlays = villager_data_textured_layer_passes(true, farmer);
    assert_eq!(baby_overlays.len(), 1);
    assert_eq!(
        baby_overlays[0].texture,
        villager_type_texture_ref(VillagerModelType::Snow, true)
    );
    assert_eq!(baby_overlays[0].order, 1);

    assert_eq!(trader.len(), 1);
    assert_eq!(
        trader[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    // Vanilla `WanderingTraderRenderer` bakes only `ModelLayers.WANDERING_TRADER`
    // and does not register a baby model/texture pair.
    assert_eq!(trader[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(trader[0].texture, WANDERING_TRADER_TEXTURE_REF);
    assert_eq!(trader[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(trader[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(trader[0].order, 0);
}

#[test]
fn entity_texture_atlas_stitches_official_villager_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&villager_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 192);
    assert_eq!(layout.entries.len(), 3);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/villager/villager.png",
            "textures/entity/villager/villager_baby.png",
            "textures/entity/wandering_trader/wandering_trader.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 1.0 / 3.0]);
    assert_close2(layout.entries[1].uv.min, [0.0, 1.0 / 3.0]);
    assert_close2(layout.entries[1].uv.max, [1.0, 2.0 / 3.0]);
    assert_close2(layout.entries[2].uv.min, [0.0, 2.0 / 3.0]);
    assert_close2(layout.entries[2].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let third_slot = rgba_offset(layout.width, 128, 0, "villager test atlas").unwrap();
    assert_eq!(&rgba[third_slot..third_slot + 4], &[2; 4]);
}

#[test]
fn villager_textured_mesh_uses_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&villager_texture_images()).unwrap();
    let adult = villager_submission_probe(EntityModelInstance::villager(
        139,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    ));
    let baby = villager_submission_probe(EntityModelInstance::villager(
        140,
        [2.0, 64.0, 0.0],
        0.0,
        true,
    ));
    let trader = villager_submission_probe(EntityModelInstance::wandering_trader(
        141,
        [4.0, 64.0, 0.0],
        0.0,
    ));

    let adult_meshes = entity_model_textured_meshes(&[adult], &atlas);
    assert_villager_textured_output(&adult_meshes, adult);
    assert_eq!(adult_meshes.cutout.cutout_faces, 66);
    assert_eq!(adult_meshes.cutout.vertices.len(), 264);
    assert_eq!(adult_meshes.cutout.indices.len(), 396);
    assert_close2(adult_meshes.cutout.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(adult_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_villager_textured_output(&baby_meshes, baby);
    assert_eq!(baby_meshes.cutout.cutout_faces, 66);
    assert_eq!(baby_meshes.cutout.vertices.len(), 264);
    assert_eq!(baby_meshes.cutout.indices.len(), 396);
    assert!(baby_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    let trader_meshes = entity_model_textured_meshes(&[trader], &atlas);
    assert_villager_textured_output(&trader_meshes, trader);
    assert_eq!(
        trader_meshes.cutout.cutout_faces,
        adult_meshes.cutout.cutout_faces
    );
    assert_eq!(
        trader_meshes.cutout.vertices.len(),
        adult_meshes.cutout.vertices.len()
    );
    assert_eq!(
        trader_meshes.cutout.indices.len(),
        adult_meshes.cutout.indices.len()
    );
    assert_close2(
        trader_meshes.cutout.vertices[0].uv,
        [16.0 / 64.0, 2.0 / 3.0],
    );
}

#[test]
fn villager_profession_layers_render_type_profession_and_level_overlays() {
    let textures = [
        VILLAGER_TEXTURE_REF,
        VILLAGER_TYPE_TEXTURE_REFS[3],
        VILLAGER_PROFESSION_TEXTURE_REFS[9],
        VILLAGER_LEVEL_TEXTURE_REFS[4],
    ];
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&textures)).unwrap();
    let mason = villager_submission_probe(
        EntityModelInstance::villager(401, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
            s.kind.villager_mut().model_data = VillagerModelData::new(
                VillagerModelType::Savanna,
                VillagerModelProfession::Mason,
                9,
            )
        }),
    );
    let meshes = entity_model_textured_meshes(&[mason], &atlas);
    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 264);
    assert_eq!(mesh.vertices.len(), 1056);
    assert_close2(mesh.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert_close2(mesh.vertices[264].uv, [16.0 / 64.0, 1.0 / 4.0]);
    assert_close2(mesh.vertices[528].uv, [16.0 / 64.0, 2.0 / 4.0]);
    assert_close2(mesh.vertices[792].uv, [16.0 / 64.0, 3.0 / 4.0]);

    assert_villager_textured_output(&meshes, mason);
    let body_overlay = mason.render_state.overlay_coords();
    let layer_overlay = [0.0, body_overlay[1]];
    assert_ne!(body_overlay, layer_overlay);
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == body_overlay));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == layer_overlay));
}

#[test]
fn villager_profession_missing_textures_preserve_other_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        VILLAGER_TEXTURE_REF,
        VILLAGER_BABY_TEXTURE_REF,
    ]))
    .unwrap();

    let adult = villager_submission_probe(
        EntityModelInstance::villager(406, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
            s.kind.villager_mut().model_data = VillagerModelData::new(
                VillagerModelType::Savanna,
                VillagerModelProfession::Mason,
                9,
            )
        }),
    );
    let adult_meshes = entity_model_textured_meshes(&[adult], &atlas);
    assert_villager_folded_meshes_are_cutout_only(&adult_meshes);

    assert_eq!(adult_meshes.cutout.vertices.len(), 264);
    let adult_base_overlay = adult.render_state.overlay_coords();
    let adult_layer_overlay = [0.0, adult_base_overlay[1]];

    assert!(adult_meshes.cutout.vertices.iter().all(|vertex| {
        vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == adult.render_state.shader_light()
            && vertex.overlay == adult_base_overlay
    }));
    assert!(adult_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay != adult_layer_overlay));

    let baby = villager_submission_probe(
        EntityModelInstance::villager(407, [2.0, 64.0, 0.0], 0.0, true).with(|s| {
            s.kind.villager_mut().model_data =
                VillagerModelData::new(VillagerModelType::Plains, VillagerModelProfession::Mason, 5)
        }),
    );
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_villager_folded_meshes_are_cutout_only(&baby_meshes);

    assert_eq!(baby_meshes.cutout.vertices.len(), 264);
    let baby_base_overlay = baby.render_state.overlay_coords();
    let baby_layer_overlay = [0.0, baby_base_overlay[1]];

    assert!(baby_meshes.cutout.vertices.iter().all(|vertex| {
        vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == baby.render_state.shader_light()
            && vertex.overlay == baby_base_overlay
    }));
    assert!(baby_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay != baby_layer_overlay));
}

#[test]
fn villager_profession_layers_apply_no_hat_and_baby_rules() {
    let textures = [
        VILLAGER_TEXTURE_REF,
        VILLAGER_BABY_TEXTURE_REF,
        VILLAGER_TYPE_TEXTURE_REFS[0],
        VILLAGER_BABY_TYPE_TEXTURE_REFS[2],
        VILLAGER_PROFESSION_TEXTURE_REFS[4],
        VILLAGER_LEVEL_TEXTURE_REFS[0],
    ];
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&textures)).unwrap();

    let desert_farmer =
        EntityModelInstance::villager(402, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
            s.kind.villager_mut().model_data = VillagerModelData::new(
                VillagerModelType::Desert,
                VillagerModelProfession::Farmer,
                1,
            )
        });
    let meshes = entity_model_textured_meshes(&[desert_farmer], &atlas);
    assert_villager_textured_output(&meshes, desert_farmer);
    let mesh = &meshes.cutout;
    // Base 66 faces + type layer without the head subtree 42 + profession 66 + level 66.
    assert_eq!(mesh.cutout_faces, 240);
    assert_eq!(mesh.vertices.len(), 960);

    let baby = EntityModelInstance::villager(403, [2.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.kind.villager_mut().model_data =
            VillagerModelData::new(VillagerModelType::Plains, VillagerModelProfession::Mason, 5)
    });
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_villager_textured_output(&baby_meshes, baby);
    // Vanilla skips profession and level overlays for babies even when data carries a profession.
    assert_eq!(baby_meshes.cutout.cutout_faces, 132);
    assert_eq!(baby_meshes.cutout.vertices.len(), 528);
}

#[test]
fn villager_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&villager_texture_images()).unwrap();

    // Adult villager/wandering trader head is part 0: head look turns it.
    let adult = EntityModelInstance::villager(142, [0.0, 64.0, 0.0], 0.0, false);
    let adult_yawed_instance = adult.with(|s| (s.head_yaw, s.head_pitch) = (45.0, 0.0));
    let adult_pitched_instance = adult.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0));
    let adult_resting = entity_model_textured_meshes(&[adult], &atlas);
    let adult_yawed = entity_model_textured_meshes(&[adult_yawed_instance], &atlas);
    let adult_pitched = entity_model_textured_meshes(&[adult_pitched_instance], &atlas);
    assert_villager_textured_output(&adult_resting, adult);
    assert_villager_textured_output(&adult_yawed, adult_yawed_instance);
    assert_villager_textured_output(&adult_pitched, adult_pitched_instance);
    assert_eq!(
        adult_resting.cutout.vertices.len(),
        adult_yawed.cutout.vertices.len()
    );
    assert_ne!(adult_resting.cutout.vertices, adult_yawed.cutout.vertices);
    assert_ne!(adult_yawed.cutout.vertices, adult_pitched.cutout.vertices);

    let trader = EntityModelInstance::wandering_trader(143, [0.0, 64.0, 0.0], 0.0);
    let trader_looking_instance = trader.with(|s| (s.head_yaw, s.head_pitch) = (45.0, -20.0));
    let trader_resting = entity_model_textured_meshes(&[trader], &atlas);
    let trader_looking = entity_model_textured_meshes(&[trader_looking_instance], &atlas);
    assert_villager_textured_output(&trader_resting, trader);
    assert_villager_textured_output(&trader_looking, trader_looking_instance);
    assert_ne!(
        trader_resting.cutout.vertices,
        trader_looking.cutout.vertices
    );

    // Head look turns the baby head and leaves the legs untouched.
    let baby = EntityModelInstance::villager(144, [0.0, 64.0, 0.0], 0.0, true);
    let baby_looking_instance = baby.with(|s| (s.head_yaw, s.head_pitch) = (45.0, -20.0));
    let baby_resting = entity_model_textured_meshes(&[baby], &atlas);
    let baby_looking = entity_model_textured_meshes(&[baby_looking_instance], &atlas);
    assert_villager_textured_output(&baby_resting, baby);
    assert_villager_textured_output(&baby_looking, baby_looking_instance);
    assert_ne!(baby_resting.cutout.vertices, baby_looking.cutout.vertices);
    let baby_model = ModelLayer::VILLAGER_BABY.bake();
    let legs = ["left_leg", "right_leg"];
    assert_eq!(
        parts_vertices(&baby_resting.cutout.vertices, &baby_model, &legs),
        parts_vertices(&baby_looking.cutout.vertices, &baby_model, &legs)
    );
}

#[test]
fn villager_unhappy_head_pose_matches_vanilla_setup_anim() {
    // Vanilla `VillagerModel.setupAnim`: head look sets yRot/xRot first; the unhappy branch preserves
    // yRot, sets `head.xRot = 0.4`, and rolls by `0.3 * sin(0.45 * ageInTicks)`.
    let age = 7.0_f32;
    let base = EntityModelInstance::villager(145, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (35.0, -20.0))
        .with(|s| s.age_in_ticks = age);
    let mut content = villager_model(false, &base);
    let content_head = content.child_mut("head").pose.rotation;
    assert!(
        (content_head[0] - (-20.0_f32).to_radians()).abs() < 1.0e-6,
        "content villager keeps the look pitch: {}",
        content_head[0]
    );
    assert!(
        (content_head[1] - 35.0_f32.to_radians()).abs() < 1.0e-6,
        "content villager keeps the look yaw: {}",
        content_head[1]
    );
    assert_eq!(content_head[2], 0.0);
    let mut unhappy = villager_model(false, &base.with(|s| s.kind.villager_mut().unhappy = true));
    let unhappy_head = unhappy.child_mut("head").pose.rotation;
    let shake = 0.3 * (0.45 * age).sin();
    assert!(
        (unhappy_head[0] - 0.4).abs() < 1.0e-6,
        "unhappy branch fixes the head pitch: {}",
        unhappy_head[0]
    );
    assert!(
        (unhappy_head[1] - 35.0_f32.to_radians()).abs() < 1.0e-6,
        "unhappy branch preserves the look yaw: {}",
        unhappy_head[1]
    );
    assert!(
        (unhappy_head[2] - shake).abs() < 1.0e-6,
        "unhappy branch rolls the head: {} vs {shake}",
        unhappy_head[2]
    );
    let mut baby = villager_model(true, &base.with(|s| s.kind.villager_mut().unhappy = true));
    let baby_head = baby.child_mut("head").pose.rotation;
    assert!(
        (baby_head[0] - 0.4).abs() < 1.0e-6,
        "baby villagers use the same unhappy pitch: {}",
        baby_head[0]
    );
    assert!(
        (baby_head[2] - shake).abs() < 1.0e-6,
        "baby villagers use the same unhappy roll: {}",
        baby_head[2]
    );
}

#[test]
fn villager_family_textured_mesh_swings_legs_when_walking() {
    // The real villager render path (texture-backed) swings the same half-amplitude
    // legs. A standing villager is byte-identical however far the swing position has
    // advanced; a walking one differs, for the adult/baby layers and trader. The
    // adult-size models also lift their feet (the baby's short legs stay inside the
    // head/body box).
    let (atlas, _) = build_entity_model_texture_atlas(&villager_texture_images()).unwrap();
    let instances: [(&str, EntityModelInstance, bool); 3] = [
        (
            "villager_adult",
            EntityModelInstance::villager(303, [0.0, 64.0, 0.0], 0.0, false),
            true,
        ),
        (
            "villager_baby",
            EntityModelInstance::villager(304, [0.0, 64.0, 0.0], 0.0, true),
            false,
        ),
        (
            "wandering_trader",
            EntityModelInstance::wandering_trader(305, [0.0, 64.0, 0.0], 0.0),
            true,
        ),
    ];
    for (name, base, adult_size) in instances {
        let still_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
        let walking_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_villager_textured_output(&resting, base);
        assert_villager_textured_output(&still, still_instance);
        assert_villager_textured_output(&walking, walking_instance);

        assert_eq!(
            resting.cutout.vertices, still.cutout.vertices,
            "{name}: a standing villager is inert"
        );
        assert_eq!(
            resting.cutout.vertices.len(),
            walking.cutout.vertices.len(),
            "{name}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.cutout.vertices, walking.cutout.vertices,
            "{name}: a walking villager differs"
        );

        if adult_size {
            let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
            let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "{name}: a walking villager's feet should lift off the ground"
            );
        }
    }
}

fn assert_villager_textured_output(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert_villager_folded_meshes_are_cutout_only(meshes);
    let has_zero_white_overlay = match instance.kind {
        EntityModelKind::Villager { baby } => !villager_data_textured_layer_passes(
            baby,
            instance.render_state.kind.villager().model_data,
        )
        .is_empty(),
        EntityModelKind::WanderingTrader => false,
        _ => panic!("expected villager family instance"),
    };
    let base_overlay = instance.render_state.overlay_coords();
    let zero_white_overlay = [0.0, base_overlay[1]];

    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == instance.render_state.shader_light()));
    if has_zero_white_overlay && zero_white_overlay != base_overlay {
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .any(|vertex| vertex.overlay == base_overlay));
        assert!(meshes.cutout.vertices.iter().all(|vertex| {
            vertex.overlay == base_overlay || vertex.overlay == zero_white_overlay
        }));
    } else {
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .all(|vertex| vertex.overlay == base_overlay));
    }
}

fn villager_submission_probe(instance: EntityModelInstance) -> EntityModelInstance {
    instance.with(|s| {
        s.light_coords = (8_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    })
}

fn assert_villager_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_cutout.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
}

fn villager_texture_images() -> Vec<EntityModelTextureImage> {
    texture_images(&[
        VILLAGER_TEXTURE_REF,
        VILLAGER_BABY_TEXTURE_REF,
        WANDERING_TRADER_TEXTURE_REF,
    ])
}

fn texture_images(textures: &[EntityModelTextureRef]) -> Vec<EntityModelTextureImage> {
    textures
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
