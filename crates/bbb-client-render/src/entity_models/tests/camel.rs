use super::*;
use crate::entity_models::model_layers::camel_model;

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
fn camel_texture_refs_match_vanilla_renderer() {
    assert_eq!(
        CAMEL_SADDLE_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/equipment/camel_saddle/saddle.png",
            size: [128, 128],
        }
    );
    assert_eq!(
        CAMEL_HUSK_SADDLE_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/equipment/camel_husk_saddle/saddle.png",
            size: [128, 128],
        }
    );
    assert!(entity_model_texture_refs().contains(&CAMEL_SADDLE_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&CAMEL_HUSK_SADDLE_TEXTURE_REF));
}

#[test]
fn camel_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let adult = camel_textured_layer_passes(CamelModelFamily::Camel, false);
    assert_eq!(adult.len(), 1);
    assert_eq!(
        adult[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(adult[0].texture, CAMEL_TEXTURE_REF);
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(adult[0].order, 0);

    let baby = camel_textured_layer_passes(CamelModelFamily::Camel, true);
    assert_eq!(
        baby[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(baby[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(baby[0].texture, CAMEL_BABY_TEXTURE_REF);

    // The camel husk shares the adult mesh/layer; only the texture differs, and it is
    // never a baby (the husk renderer is adult-only), so the age flag must not change it.
    let husk = camel_textured_layer_passes(CamelModelFamily::CamelHusk, false);
    assert_eq!(
        husk[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(husk[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(husk[0].texture, CAMEL_HUSK_TEXTURE_REF);
    let husk_baby = camel_textured_layer_passes(CamelModelFamily::CamelHusk, true);
    assert_eq!(husk_baby[0].texture, CAMEL_HUSK_TEXTURE_REF);
}

#[test]
fn camel_textured_mesh_matches_static_vanilla_pose() {
    // Vanilla `CamelModel.setupAnim` drives the limbs via baked `KeyframeAnimation`s plus a
    // direct head clamp. The textured meshes carry the full body-layer geometry (12 adult cubes /
    // 11 baby cubes, 24 vertices each); the adult/husk walk is reproduced (exercised below), and the
    // head look is exercised separately.
    let (atlas, _) = build_entity_model_texture_atlas(&camel_texture_images()).unwrap();
    let adult =
        EntityModelInstance::camel(700, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false);
    let baby =
        EntityModelInstance::camel(701, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, true);
    let husk = EntityModelInstance::camel(
        702,
        [0.0, 64.0, 0.0],
        0.0,
        CamelModelFamily::CamelHusk,
        true,
    );

    let adult_meshes = entity_model_textured_meshes(&[adult], &atlas);
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    let husk_meshes = entity_model_textured_meshes(&[husk], &atlas);
    assert_camel_vertices_match_vanilla(&adult_meshes, adult);
    assert_camel_vertices_match_vanilla(&baby_meshes, baby);
    assert_camel_vertices_match_vanilla(&husk_meshes, husk);
    assert_eq!(adult_meshes.cutout.vertices.len(), 288);
    assert_eq!(baby_meshes.cutout.vertices.len(), 264);
    // The husk reuses the adult mesh (adult-only renderer); only its sampled texels differ.
    assert_eq!(husk_meshes.cutout.vertices.len(), 288);
    assert_eq!(
        husk_meshes
            .cutout
            .vertices
            .iter()
            .map(|v| v.position)
            .collect::<Vec<_>>(),
        adult_meshes
            .cutout
            .vertices
            .iter()
            .map(|v| v.position)
            .collect::<Vec<_>>()
    );

    // The adult/husk walk is reproduced on the textured path: a still camel (walk speed 0) matches
    // the rest pose, while a walking camel (speed > 0) differs.
    let still_instance =
        adult.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 0.0));
    let walking_instance =
        adult.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (5.0, 1.0));
    let still = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_camel_vertices_match_vanilla(&still, still_instance);
    assert_camel_vertices_match_vanilla(&walking, walking_instance);
    assert_eq!(adult_meshes.cutout.vertices, still.cutout.vertices);
    assert_eq!(
        adult_meshes.cutout.vertices.len(),
        walking.cutout.vertices.len()
    );
    assert_ne!(adult_meshes.cutout.vertices, walking.cutout.vertices);
}

#[test]
fn camel_saddle_layer_renders_for_adult_camel_and_husk_only() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        CAMEL_TEXTURE_REF,
        CAMEL_BABY_TEXTURE_REF,
        CAMEL_HUSK_TEXTURE_REF,
        CAMEL_SADDLE_TEXTURE_REF,
        CAMEL_HUSK_SADDLE_TEXTURE_REF,
    ]))
    .unwrap();

    let adult =
        EntityModelInstance::camel(760, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false)
            .with(|s| {
                s.light_coords = (5_u32 << 4) | (11_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            });
    let bare_meshes = entity_model_textured_meshes(&[adult], &atlas);
    let saddled_instance = adult.with(|s| {
        s.kind.camel_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id())
    });
    let saddled_meshes = entity_model_textured_meshes(&[saddled_instance], &atlas);
    assert_camel_vertices_match_vanilla(&bare_meshes, adult);
    assert_camel_vertices_match_vanilla(&saddled_meshes, saddled_instance);
    let bare = &bare_meshes.cutout;
    let saddled = &saddled_meshes.cutout;

    assert_ne!(saddled_instance.render_state.overlay_coords(), [0.0, 10.0]);
    assert_eq!(saddled.cutout_faces, bare.cutout_faces);
    assert_eq!(saddled.vertices.len(), bare.vertices.len());
    assert_eq!(saddled_meshes.armor_cutout.cutout_faces, 120);
    assert_eq!(saddled_meshes.armor_cutout.vertices.len(), 480);

    let ridden_instance = adult.with(|s| {
        s.kind.camel_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
        s.kind.camel_mut().saddle_ridden = true;
    });
    let ridden_meshes = entity_model_textured_meshes(&[ridden_instance], &atlas);
    assert_camel_vertices_match_vanilla(&ridden_meshes, ridden_instance);
    assert_eq!(ridden_meshes.cutout.cutout_faces, saddled.cutout_faces);
    assert_eq!(
        ridden_meshes.armor_cutout.cutout_faces - saddled_meshes.armor_cutout.cutout_faces,
        18
    );
    assert_eq!(
        ridden_meshes.armor_cutout.vertices.len() - saddled_meshes.armor_cutout.vertices.len(),
        72
    );

    let saddle_uv = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == CAMEL_SADDLE_TEXTURE_REF)
        .unwrap()
        .uv;
    let first_saddle_vertex = saddled_meshes.armor_cutout.vertices[0].uv;
    assert!(first_saddle_vertex[0] >= saddle_uv.min[0]);
    assert!(first_saddle_vertex[0] <= saddle_uv.max[0]);
    assert!(first_saddle_vertex[1] >= saddle_uv.min[1]);
    assert!(first_saddle_vertex[1] <= saddle_uv.max[1]);

    let baby =
        EntityModelInstance::camel(761, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, true).with(
            |s| {
                s.light_coords = (5_u32 << 4) | (11_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
                s.kind.camel_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
                s.kind.camel_mut().saddle_ridden = true;
            },
        );
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_camel_vertices_match_vanilla(&baby_meshes, baby);
    assert_eq!(
        baby_meshes.cutout.vertices.len(),
        264,
        "vanilla supplies no baby model for the camel saddle layer"
    );
    assert!(baby_meshes.armor_cutout.vertices.is_empty());

    let husk = EntityModelInstance::camel(
        762,
        [0.0, 64.0, 0.0],
        0.0,
        CamelModelFamily::CamelHusk,
        false,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.camel_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
    });
    let husk_meshes = entity_model_textured_meshes(&[husk], &atlas);
    assert_camel_vertices_match_vanilla(&husk_meshes, husk);
    let husk_mesh = &husk_meshes.cutout;

    assert_eq!(husk_mesh.cutout_faces, bare.cutout_faces);
    assert_eq!(husk_meshes.armor_cutout.cutout_faces, 120);
    let husk_saddle_uv = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == CAMEL_HUSK_SADDLE_TEXTURE_REF)
        .unwrap()
        .uv;
    let first_husk_saddle_vertex = husk_meshes.armor_cutout.vertices[0].uv;
    assert!(first_husk_saddle_vertex[0] >= husk_saddle_uv.min[0]);
    assert!(first_husk_saddle_vertex[0] <= husk_saddle_uv.max[0]);
    assert!(first_husk_saddle_vertex[1] >= husk_saddle_uv.min[1]);
    assert!(first_husk_saddle_vertex[1] <= husk_saddle_uv.max[1]);
}

#[test]
fn camel_saddle_submission_survives_missing_saddle_texture_atlas_entry() {
    // Vanilla `CamelRenderer.createCamelSaddleLayer` records the adult camel saddle as a default-order
    // `SimpleEquipmentLayer(CAMEL_SADDLE)` submit after the base body, with no overlay.
    let images = texture_images(&[CAMEL_TEXTURE_REF]);
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance =
        EntityModelInstance::camel(763, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false)
            .with(|s| {
                s.light_coords = (5_u32 << 4) | (11_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
                s.kind.camel_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
            });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(
        meshes.cutout.vertices.len(),
        288,
        "missing camel_saddle/saddle.png suppresses only folded saddle geometry"
    );
    assert!(meshes.armor_cutout.vertices.is_empty());
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));

    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
}

#[test]
fn camel_saddle_foil_and_trim_use_family_specific_equipment_layers() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        CAMEL_TEXTURE_REF,
        CAMEL_BABY_TEXTURE_REF,
        CAMEL_HUSK_TEXTURE_REF,
        CAMEL_SADDLE_TEXTURE_REF,
        CAMEL_HUSK_SADDLE_TEXTURE_REF,
        ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
    ]))
    .unwrap();
    let trim_texture = EntityDynamicTexture { handle: 76_401 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [128, 128],
        standalone_sampling: None,
        rgba: vec![0xff; 128 * 128 * 4],
    }])
    .unwrap()
    .0;
    let trimmed_saddle = |entity_id, family, baby| {
        EntityModelInstance::camel(entity_id, [0.0, 64.0, 0.0], 0.0, family, baby).with(|s| {
            s.kind.camel_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
            s.kind.camel_mut().saddle_foil = true;
            s.kind.camel_mut().saddle_trim = Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            });
        })
    };

    for (instance, saddle_texture) in [
        (
            trimmed_saddle(764, CamelModelFamily::Camel, false),
            CAMEL_SADDLE_TEXTURE_REF,
        ),
        (
            trimmed_saddle(765, CamelModelFamily::CamelHusk, false),
            CAMEL_HUSK_SADDLE_TEXTURE_REF,
        ),
    ] {
        let meshes = entity_model_textured_meshes_with_dynamic_textures(
            &[instance],
            &atlas,
            None,
            Some(&dynamic_atlas),
        );

        let uv = atlas
            .entries
            .iter()
            .find(|entry| entry.texture == saddle_texture)
            .unwrap()
            .uv;
        assert!(meshes.armor_cutout.vertices.iter().all(|vertex| {
            vertex.uv[0] >= uv.min[0]
                && vertex.uv[0] <= uv.max[0]
                && vertex.uv[1] >= uv.min[1]
                && vertex.uv[1] <= uv.max[1]
        }));
        assert_eq!(meshes.armor_cutout.vertices.len(), 480);
        assert_eq!(meshes.armor_entity_glint.vertices.len(), 480);
        assert_eq!(meshes.dynamic_player_texture_armor_trim.vertices.len(), 480);
    }

    let baby = trimmed_saddle(766, CamelModelFamily::Camel, true);
    let baby_meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[baby],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert!(baby_meshes.armor_cutout.vertices.is_empty());
    assert!(baby_meshes.armor_entity_glint.vertices.is_empty());
    assert!(baby_meshes
        .dynamic_player_texture_armor_trim
        .vertices
        .is_empty());
}

/// The vertex range of the camel's nested head subtree (head cubes and ears).
fn camel_head_vertex_range(baby: bool) -> std::ops::Range<usize> {
    (if baby {
        ModelLayer::CAMEL_BABY
    } else {
        ModelLayer::CAMEL
    })
    .bake()
    .part_vertex_range("head")
    .unwrap()
}

#[test]
fn camel_head_look_turns_only_the_nested_head_subtree() {
    // Vanilla `CamelModel.applyHeadRotation` drives `head.yRot/xRot` from the clamped look. The
    // head is `body.getChild("head")`, so the body, hump, tail, and legs stay put while the head
    // cubes and their ear children turn.
    let head = camel_head_vertex_range(false);
    let rest =
        EntityModelInstance::camel(710, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false);
    let looked = rest.with(|s| (s.head_yaw, s.head_pitch) = (40.0, -20.0));

    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_eq!(rest_mesh.vertices.len(), looked_mesh.vertices.len());
    assert_eq!(
        rest_mesh.vertices[..head.start],
        looked_mesh.vertices[..head.start],
        "the body/hump/tail stay put"
    );
    assert_ne!(
        rest_mesh.vertices[head.clone()],
        looked_mesh.vertices[head.clone()],
        "the nested head subtree turns"
    );
    assert_eq!(
        rest_mesh.vertices[head.end..],
        looked_mesh.vertices[head.end..],
        "the legs stay put"
    );

    let (atlas, _) = build_entity_model_texture_atlas(&camel_texture_images()).unwrap();
    let rest_textured = entity_model_textured_meshes(&[rest], &atlas);
    let looked_textured = entity_model_textured_meshes(&[looked], &atlas);
    assert_camel_vertices_match_vanilla(&rest_textured, rest);
    assert_camel_vertices_match_vanilla(&looked_textured, looked);
    assert_eq!(
        rest_textured.cutout.vertices.len(),
        looked_textured.cutout.vertices.len()
    );
    assert_eq!(
        rest_textured.cutout.vertices[..head.start],
        looked_textured.cutout.vertices[..head.start],
        "the body/hump/tail stay put"
    );
    assert_ne!(
        rest_textured.cutout.vertices[head.clone()],
        looked_textured.cutout.vertices[head.clone()],
        "the nested head subtree turns"
    );
    assert_eq!(
        rest_textured.cutout.vertices[head.end..],
        looked_textured.cutout.vertices[head.end..],
        "the legs stay put"
    );
}

#[test]
fn camel_textured_walk_moves_the_whole_model_and_composes_with_the_look() {
    // CAMEL_WALK: the `root` roll turns the whole model and the head walk pitch ADDS onto the clamped look (only the nested head subtree
    // tracks the look; the body and legs share the walk).
    let (atlas, _) = build_entity_model_texture_atlas(&camel_texture_images()).unwrap();
    let head = camel_head_vertex_range(false);
    let still =
        EntityModelInstance::camel(730, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false);
    let walking = still.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (5.0, 1.0));
    let walking_looking = still
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (5.0, 1.0))
        .with(|s| (s.head_yaw, s.head_pitch) = (40.0, -20.0));

    let still_mesh = entity_model_textured_meshes(&[still], &atlas);
    let walking_mesh = entity_model_textured_meshes(&[walking], &atlas);
    assert_camel_vertices_match_vanilla(&still_mesh, still);
    assert_camel_vertices_match_vanilla(&walking_mesh, walking);
    assert_eq!(
        still_mesh.cutout.vertices.len(),
        walking_mesh.cutout.vertices.len()
    );
    assert_ne!(
        still_mesh.cutout.vertices, walking_mesh.cutout.vertices,
        "the walking camel rolls its whole body and swings its legs"
    );

    let walking_looking_mesh = entity_model_textured_meshes(&[walking_looking], &atlas);
    assert_camel_vertices_match_vanilla(&walking_looking_mesh, walking_looking);
    assert_ne!(
        walking_mesh.cutout.vertices[head.clone()],
        walking_looking_mesh.cutout.vertices[head.clone()],
        "the look composes onto the walking head"
    );
    assert_eq!(
        walking_mesh.cutout.vertices[..head.start],
        walking_looking_mesh.cutout.vertices[..head.start],
        "the body/hump/tail share the same walk regardless of the look"
    );
    assert_eq!(
        walking_mesh.cutout.vertices[head.end..],
        walking_looking_mesh.cutout.vertices[head.end..],
        "the legs share the same walk regardless of the look"
    );
}

#[test]
fn camel_baby_walk_moves_the_model_and_composes_with_the_look() {
    // The baby camel hand-walks `CAMEL_BABY_WALK` on both paths: a still baby (walk speed 0) collapses
    // to the bind pose, a walking baby rolls/swings, and the head walk pitch ADDS onto the look (only
    // the nested head subtree tracks the look).
    let head = camel_head_vertex_range(true);
    let still =
        EntityModelInstance::camel(740, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, true);
    let walking = still.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (5.0, 1.0));
    let walking_looking = still
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (5.0, 1.0))
        .with(|s| (s.head_yaw, s.head_pitch) = (40.0, -20.0));

    let still_mesh = entity_model_mesh(&[still]);
    let walking_mesh = entity_model_mesh(&[walking]);
    assert_eq!(still_mesh.vertices.len(), walking_mesh.vertices.len());
    assert_ne!(
        still_mesh.vertices, walking_mesh.vertices,
        "the walking baby camel rolls its whole body and swings its legs"
    );
    let walking_looking_mesh = entity_model_mesh(&[walking_looking]);
    assert_ne!(
        walking_mesh.vertices[head.clone()],
        walking_looking_mesh.vertices[head.clone()],
        "the look composes onto the walking baby head"
    );
    assert_eq!(
        walking_mesh.vertices[..head.start],
        walking_looking_mesh.vertices[..head.start],
        "the body and tail share the same walk regardless of the look"
    );
    assert_eq!(
        walking_mesh.vertices[head.end..],
        walking_looking_mesh.vertices[head.end..],
        "the legs share the same walk regardless of the look"
    );

    // The textured path reproduces the same baby walk.
    let (atlas, _) = build_entity_model_texture_atlas(&camel_texture_images()).unwrap();
    let still_textured = entity_model_textured_meshes(&[still], &atlas);
    let walking_textured = entity_model_textured_meshes(&[walking], &atlas);
    assert_camel_vertices_match_vanilla(&still_textured, still);
    assert_camel_vertices_match_vanilla(&walking_textured, walking);
    assert_eq!(
        still_textured.cutout.vertices.len(),
        walking_textured.cutout.vertices.len()
    );
    assert_ne!(
        still_textured.cutout.vertices, walking_textured.cutout.vertices,
        "the textured baby camel walks too"
    );
}

#[test]
fn camel_head_look_clamps_to_vanilla_range() {
    // Vanilla `CamelModel.applyHeadRotation`: `yRot = clamp(yRot, -30, 30)`,
    // `xRot = clamp(xRot, -25, 45)`, then post-dash `jumpCooldown` adds
    // `45 * cooldown / 55` and clamps pitch to 70.
    assert_eq!(camel_clamped_head_look(0.0, 0.0, 0.0), (0.0, 0.0));
    assert_eq!(camel_clamped_head_look(12.0, 20.0, 0.0), (12.0, 20.0));
    assert_eq!(camel_clamped_head_look(50.0, 60.0, 0.0), (30.0, 45.0));
    assert_eq!(camel_clamped_head_look(-50.0, -60.0, 0.0), (-30.0, -25.0));
    assert_eq!(camel_clamped_head_look(12.0, 20.0, 55.0), (12.0, 65.0));
    assert_eq!(camel_clamped_head_look(12.0, 45.0, 55.0), (12.0, 70.0));
}

#[test]
fn camel_jump_cooldown_adds_vanilla_head_pitch_boost() {
    let base =
        EntityModelInstance::camel(711, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false)
            .with(|s| (s.head_yaw, s.head_pitch) = (0.0, 20.0));
    let mut resting = camel_model(CamelModelFamily::Camel, false, &base);
    let rest_head_pitch = resting.child_mut("body").child_mut("head").pose.rotation[0];
    let mut cooling_down = camel_model(
        CamelModelFamily::Camel,
        false,
        &base.with(|s| s.kind.camel_mut().jump_cooldown = 55.0),
    );
    let boosted_head_pitch = cooling_down
        .child_mut("body")
        .child_mut("head")
        .pose
        .rotation[0];

    assert!(
        (boosted_head_pitch - rest_head_pitch - 45.0_f32.to_radians()).abs() < 1.0e-5,
        "jumpCooldown=55 adds 45 degrees of head pitch: {rest_head_pitch} -> {boosted_head_pitch}"
    );
}

#[test]
fn camel_sitting_and_standing_re_pose_the_body_and_legs_vs_the_bind_pose() {
    // Vanilla `CamelModel.setupAnim` applies the sit/sit-pose/stand-up keyframes ADDITIVELY onto the
    // walk pose. A standing camel (all three sentinels -1) sits at the bind pose; a sitting-down camel,
    // a seated camel, and a standing-up camel each re-pose the body and legs differently.
    let bind =
        EntityModelInstance::camel(750, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false);
    let sitting_down = bind.with(|s| s.kind.camel_mut().sit_seconds = 1.0);
    let seated = bind.with(|s| s.kind.camel_mut().sit_pose_seconds = 0.5);
    let standing_up = bind.with(|s| s.kind.camel_mut().standup_seconds = 0.5);

    let bind_mesh = entity_model_mesh(&[bind]);
    let sitting_down_mesh = entity_model_mesh(&[sitting_down]);
    let seated_mesh = entity_model_mesh(&[seated]);
    let standing_up_mesh = entity_model_mesh(&[standing_up]);

    // Every pose preserves the vertex count.
    assert_eq!(bind_mesh.vertices.len(), sitting_down_mesh.vertices.len());
    assert_eq!(bind_mesh.vertices.len(), seated_mesh.vertices.len());
    assert_eq!(bind_mesh.vertices.len(), standing_up_mesh.vertices.len());

    // Each sit/stand pose differs from the bind pose, and the three differ from each other.
    assert_ne!(
        bind_mesh.vertices, sitting_down_mesh.vertices,
        "the sitting-down camel folds down off the bind pose"
    );
    assert_ne!(
        bind_mesh.vertices, seated_mesh.vertices,
        "the seated camel holds a folded pose off the bind pose"
    );
    assert_ne!(
        bind_mesh.vertices, standing_up_mesh.vertices,
        "the standing-up camel unfolds off the bind pose"
    );
    assert_ne!(
        sitting_down_mesh.vertices, standing_up_mesh.vertices,
        "sitting down and standing up pose the camel differently"
    );

    // The textured path reproduces the same sit/stand re-pose.
    let (atlas, _) = build_entity_model_texture_atlas(&camel_texture_images()).unwrap();
    let bind_textured = entity_model_textured_meshes(&[bind], &atlas);
    let seated_textured = entity_model_textured_meshes(&[seated], &atlas);
    assert_camel_vertices_match_vanilla(&bind_textured, bind);
    assert_camel_vertices_match_vanilla(&seated_textured, seated);
    assert_eq!(
        bind_textured.cutout.vertices.len(),
        seated_textured.cutout.vertices.len()
    );
    assert_ne!(
        bind_textured.cutout.vertices, seated_textured.cutout.vertices,
        "the seated camel re-poses on the textured path too"
    );
}

fn assert_camel_vertices_match_vanilla(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert_camel_folded_meshes_are_cutout_only(meshes);
    assert!(!meshes.cutout.vertices.is_empty());
    assert!(meshes.cutout.vertices.iter().all(|vertex| {
        vertex.tint == [1.0; 4]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == instance.render_state.overlay_coords()
    }));
    assert!(meshes.armor_cutout.vertices.iter().all(|vertex| {
        vertex.light == instance.render_state.shader_light()
            && vertex.overlay == ENTITY_VERTEX_NO_OVERLAY
    }));
}

fn assert_camel_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
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

fn camel_texture_images() -> Vec<EntityModelTextureImage> {
    texture_images(&[
        CAMEL_TEXTURE_REF,
        CAMEL_BABY_TEXTURE_REF,
        CAMEL_HUSK_TEXTURE_REF,
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

#[test]
fn camel_dash_seconds_add_the_gallop_pose() {
    // Vanilla `CamelModel.setupAnim` applies `CAMEL_DASH` ADDITIVELY over the walk pose. With the camel
    // at rest (no walk speed), the dash offsets are the only motion: the ears flare a constant ±67.5°
    // yaw and the body pitches up a constant 5° (both Linear, so the sample at any time is exact).
    let base =
        EntityModelInstance::camel(700, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false);
    let mut resting = camel_model(CamelModelFamily::Camel, false, &base);
    let rest_left_ear_yaw = resting
        .child_mut("body")
        .child_mut("head")
        .child_mut("left_ear")
        .pose
        .rotation[1];
    let rest_body_pitch = resting.child_mut("body").pose.rotation[0];
    let mut dashing = camel_model(
        CamelModelFamily::Camel,
        false,
        &base.with(|s| s.kind.camel_mut().dash_seconds = 0.0),
    );
    let dash_left_ear_yaw = dashing
        .child_mut("body")
        .child_mut("head")
        .child_mut("left_ear")
        .pose
        .rotation[1];
    let dash_body_pitch = dashing.child_mut("body").pose.rotation[0];

    assert!(
        (dash_left_ear_yaw - rest_left_ear_yaw - (-67.5_f32).to_radians()).abs() < 1.0e-5,
        "dash flares the left ear by -67.5° yaw: {rest_left_ear_yaw} -> {dash_left_ear_yaw}"
    );
    assert!(
        (dash_body_pitch - rest_body_pitch - 5.0_f32.to_radians()).abs() < 1.0e-5,
        "dash pitches the body up 5°: {rest_body_pitch} -> {dash_body_pitch}"
    );
}

#[test]
fn camel_idle_seconds_add_the_vanilla_idle_pose_without_changing_submission_metadata() {
    // Vanilla `CamelAnimation.CAMEL_IDLE` is a non-looping 4.0 s keyframe that nudges the tail/head
    // and flaps the ears when `Camel.setupAnimationStates` restarts `idleAnimationState`.

    let base =
        EntityModelInstance::camel(701, [0.0, 64.0, 0.0], 0.0, CamelModelFamily::Camel, false);
    let mut resting = camel_model(CamelModelFamily::Camel, false, &base);
    let rest_tail_yaw = resting.child_mut("body").child_mut("tail").pose.rotation[1];
    let rest_head_pitch = resting.child_mut("body").child_mut("head").pose.rotation[0];
    let mut tail_idle = camel_model(
        CamelModelFamily::Camel,
        false,
        &base.with(|s| s.kind.camel_mut().idle_seconds = 1.0),
    );
    let idle_tail_yaw = tail_idle.child_mut("body").child_mut("tail").pose.rotation[1];
    assert!(
        (idle_tail_yaw - rest_tail_yaw - 0.43523_f32.to_radians()).abs() < 1.0e-5,
        "idle t=1.0 adds the vanilla tail yaw: {rest_tail_yaw} -> {idle_tail_yaw}"
    );
    let mut head_idle = camel_model(
        CamelModelFamily::Camel,
        false,
        &base.with(|s| s.kind.camel_mut().idle_seconds = 2.0),
    );
    let idle_head_pitch = head_idle.child_mut("body").child_mut("head").pose.rotation[0];
    assert!(
        (idle_head_pitch - rest_head_pitch - (-2.5_f32).to_radians()).abs() < 1.0e-5,
        "idle t=2.0 dips the head by -2.5°: {rest_head_pitch} -> {idle_head_pitch}"
    );

    let (atlas, _) = build_entity_model_texture_atlas(&camel_texture_images()).unwrap();
    let resting_meshes = entity_model_textured_meshes(&[base], &atlas);
    let idle_instance = base.with(|s| s.kind.camel_mut().idle_seconds = 1.0);
    let idle_meshes = entity_model_textured_meshes(&[idle_instance], &atlas);
    assert_camel_vertices_match_vanilla(&resting_meshes, base);
    assert_camel_vertices_match_vanilla(&idle_meshes, idle_instance);

    assert_ne!(
        resting_meshes.cutout.vertices, idle_meshes.cutout.vertices,
        "CAMEL_IDLE must re-pose the textured folded geometry"
    );
}
