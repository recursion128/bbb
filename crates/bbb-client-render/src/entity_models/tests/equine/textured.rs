use super::*;

#[test]
fn horse_textured_mesh_matches_vanilla_horse_geometry() {
    // Vanilla `HorseRenderer` renders the shared `HorseModel` / `BabyHorseModel` geometry on the
    // textured path with a per-coat texture: 12 adult cubes (72 faces / 288 vertices, at the 1.1
    // `livingHorseScale`) and 10 baby cubes (60 faces / 240 vertices, unscaled). The
    // coat variant only changes the sampled atlas region — positions stay identical.
    let (atlas, _) = build_entity_model_texture_atlas(&horse_texture_images()).unwrap();

    let white_adult_instance =
        EntityModelInstance::horse(160, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
            s.light_coords = (4_u32 << 4) | (12_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let white_adult_meshes = entity_model_textured_meshes(&[white_adult_instance], &atlas);
    assert_equine_buckets(&white_adult_meshes, white_adult_instance);
    let white_adult = &white_adult_meshes.cutout;
    assert_eq!(white_adult.cutout_faces, 72);
    assert_eq!(white_adult.vertices.len(), 288);
    assert_textured_vertex_state(
        &white_adult.vertices,
        white_adult_instance.render_state.shader_light(),
        white_adult_instance.render_state.overlay_coords(),
    );

    // Coat variant → same geometry, different atlas sub-rect.
    let black_adult_instance = EntityModelInstance::horse_with_variant(
        162,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        HorseColorVariant::Black,
    )
    .with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let black_adult_meshes = entity_model_textured_meshes(&[black_adult_instance], &atlas);
    assert_equine_buckets(&black_adult_meshes, black_adult_instance);
    let black_adult = &black_adult_meshes.cutout;
    assert_textured_vertex_state(
        &black_adult.vertices,
        black_adult_instance.render_state.shader_light(),
        black_adult_instance.render_state.overlay_coords(),
    );
    let white_positions: Vec<_> = white_adult.vertices.iter().map(|v| v.position).collect();
    let black_positions: Vec<_> = black_adult.vertices.iter().map(|v| v.position).collect();
    assert_eq!(white_positions, black_positions);
    let white_uvs: Vec<_> = white_adult.vertices.iter().map(|v| v.uv).collect();
    let black_uvs: Vec<_> = black_adult.vertices.iter().map(|v| v.uv).collect();
    assert_ne!(white_uvs, black_uvs);

    let baby_instance = EntityModelInstance::horse_with_variant(
        163,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        HorseColorVariant::Gray,
    )
    .with(|s| {
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let baby_meshes = entity_model_textured_meshes(&[baby_instance], &atlas);
    assert_equine_buckets(&baby_meshes, baby_instance);
    let baby = &baby_meshes.cutout;
    assert_eq!(baby.cutout_faces, 60);
    assert_eq!(baby.vertices.len(), 240);
    assert_textured_vertex_state(
        &baby.vertices,
        baby_instance.render_state.shader_light(),
        baby_instance.render_state.overlay_coords(),
    );
}

#[test]
fn horse_textured_swings_legs_when_walking() {
    let (atlas, _) = build_entity_model_texture_atlas(&horse_texture_images()).unwrap();
    let base = EntityModelInstance::horse(167, [0.0, 64.0, 0.0], 0.0, false);
    let still_instance = base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 0.0));
    let walking_instance =
        base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (5.0, 1.0));
    let still_meshes = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking_meshes = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_equine_buckets(&still_meshes, still_instance);
    assert_equine_buckets(&walking_meshes, walking_instance);
    let still = &still_meshes.cutout;
    let walking = &walking_meshes.cutout;
    assert_eq!(still.vertices.len(), walking.vertices.len());
    assert_ne!(
        still.vertices, walking.vertices,
        "the walking horse re-poses on the textured path"
    );
}

#[test]
fn horse_textured_tail_wag_changes_tail_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&horse_texture_images()).unwrap();
    let rest_instance = EntityModelInstance::horse_with_variant(
        168,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        HorseColorVariant::White,
    )
    .with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let wagging_instance = rest_instance.with(|s| {
        s.kind.equine_mut().animate_tail = true;
        s.age_in_ticks = 0.0;
    });
    let rest = entity_model_textured_meshes(&[rest_instance], &atlas);
    let wagging = entity_model_textured_meshes(&[wagging_instance], &atlas);

    assert_equine_buckets(&rest, rest_instance);
    assert_equine_buckets(&wagging, wagging_instance);

    assert_textured_vertex_state(
        &wagging.cutout.vertices,
        wagging_instance.render_state.shader_light(),
        wagging_instance.render_state.overlay_coords(),
    );

    let tail = ModelLayer::HORSE.bake().part_vertex_range("tail").unwrap();
    assert_ne!(
        rest.cutout.vertices[tail.clone()],
        wagging.cutout.vertices[tail.clone()]
    );
    assert_eq!(
        rest.cutout.vertices[..tail.start],
        wagging.cutout.vertices[..tail.start]
    );
    assert_eq!(
        rest.cutout.vertices[tail.end..],
        wagging.cutout.vertices[tail.end..]
    );
}

#[test]
fn horse_textured_event_pose_changes_body_and_leg_geometry() {
    let (atlas, _) = build_entity_model_texture_atlas(&horse_texture_images()).unwrap();
    let rest_instance = EntityModelInstance::horse_with_variant(
        169,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        HorseColorVariant::White,
    )
    .with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let eating_instance = rest_instance.with(|s| {
        s.kind.equine_mut().eat_animation = 0.8;
        s.kind.equine_mut().feeding_animation = 0.5;
        s.age_in_ticks = 2.0;
    });
    let standing_instance = rest_instance.with(|s| {
        s.kind.equine_mut().stand_animation = 0.8;
        s.age_in_ticks = 2.0;
    });

    let rest = entity_model_textured_meshes(&[rest_instance], &atlas);
    let eating = entity_model_textured_meshes(&[eating_instance], &atlas);
    let standing = entity_model_textured_meshes(&[standing_instance], &atlas);
    assert_equine_buckets(&rest, rest_instance);
    assert_equine_buckets(&eating, eating_instance);
    assert_equine_buckets(&standing, standing_instance);

    for meshes in [&eating, &standing] {
        assert_textured_vertex_state(
            &meshes.cutout.vertices,
            rest_instance.render_state.shader_light(),
            rest_instance.render_state.overlay_coords(),
        );
    }

    assert_eq!(rest.cutout.vertices[0..48], eating.cutout.vertices[0..48]);
    assert_ne!(
        rest.cutout.vertices[48..192],
        eating.cutout.vertices[48..192],
        "eating/feeding changes the head-parts transform"
    );
    assert_eq!(rest.cutout.vertices[192..], eating.cutout.vertices[192..]);

    assert_ne!(
        rest.cutout.vertices[0..24],
        standing.cutout.vertices[0..24],
        "standing rears the body"
    );
    assert_ne!(
        rest.cutout.vertices[48..192],
        standing.cutout.vertices[48..192],
        "standing changes the head-parts transform"
    );
    assert_ne!(
        rest.cutout.vertices[192..],
        standing.cutout.vertices[192..],
        "standing changes the leg transforms"
    );
}

#[test]
fn horse_markings_overlay_layers_a_translucent_white_copy() {
    // Vanilla `HorseMarkingLayer` draws the white markings as a translucent overlay of the SAME posed
    // `HorseModel` on top of the base coat, but only when the coat carries markings (`Markings.NONE` →
    // `INVISIBLE_TEXTURE`, no overlay). The overlay rides the identical pose, so the body's cutout base
    // and the markings' translucent overlay have the same vertex count and positions.
    let (atlas, _) = build_entity_model_texture_atlas(&horse_texture_images()).unwrap();

    // A plain (no-markings) horse: only the cutout base, no translucent overlay.
    let plain_instance = EntityModelInstance::horse_with_variant(
        170,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        HorseColorVariant::White,
    )
    .with(|s| {
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let plain = entity_model_textured_meshes(&[plain_instance], &atlas);
    assert_equine_buckets(&plain, plain_instance);
    assert_eq!(plain.cutout.vertices.len(), 288);
    assert!(plain.translucent.vertices.is_empty());

    assert_ne!(plain_instance.render_state.overlay_coords(), [0.0, 10.0]);
    assert_textured_vertex_state(
        &plain.cutout.vertices,
        plain_instance.render_state.shader_light(),
        plain_instance.render_state.overlay_coords(),
    );

    // A marked horse: the same cutout base PLUS a translucent overlay of identical geometry.
    let marked_instance = EntityModelInstance::horse_full(
        171,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        HorseColorVariant::White,
        HorseMarkings::WhiteDots,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let marked = entity_model_textured_meshes(&[marked_instance], &atlas);
    assert_equine_buckets(&marked, marked_instance);
    assert_eq!(marked.cutout.vertices.len(), 288);
    assert_eq!(marked.translucent.cutout_faces, 72);
    assert_eq!(marked.translucent.vertices.len(), 288);

    assert_textured_vertex_state(
        &marked.cutout.vertices,
        marked_instance.render_state.shader_light(),
        marked_instance.render_state.overlay_coords(),
    );
    assert_textured_vertex_state(
        &marked.translucent.vertices,
        marked_instance.render_state.shader_light(),
        [0.0, marked_instance.render_state.overlay_coords()[1]],
    );
    let base_positions: Vec<_> = marked.cutout.vertices.iter().map(|v| v.position).collect();
    let overlay_positions: Vec<_> = marked
        .translucent
        .vertices
        .iter()
        .map(|v| v.position)
        .collect();
    assert_eq!(base_positions, overlay_positions);
    // The overlay samples the markings atlas region, not the coat's.
    let base_uvs: Vec<_> = marked.cutout.vertices.iter().map(|v| v.uv).collect();
    let overlay_uvs: Vec<_> = marked.translucent.vertices.iter().map(|v| v.uv).collect();
    assert_ne!(base_uvs, overlay_uvs);
}

#[test]
fn horse_markings_missing_markings_texture_texture_preserves_other_geometry() {
    let images = texture_images(&[HORSE_WHITE_TEXTURE_REF]);
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::horse_full(
        172,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        HorseColorVariant::White,
        HorseMarkings::WhiteDots,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(!meshes.cutout.vertices.is_empty());

    assert!(
        meshes.translucent.vertices.is_empty(),
        "missing horse_markings_whitedots.png suppresses only folded markings geometry"
    );
    assert!(meshes.translucent.indices.is_empty());
}

#[test]
fn self_visible_invisible_horse_uses_translucent_base_and_skips_markings_layer() {
    // Vanilla `LivingEntityRenderer.getRenderType`: an invisible living entity that is still visible
    // to this client uses `entityTranslucentCullItemTarget` for the base model. `HorseMarkingLayer`
    // additionally checks `!state.isInvisible`, so markings do not submit in this branch.
    let (atlas, _) = build_entity_model_texture_atlas(&horse_texture_images()).unwrap();
    let instance = EntityModelInstance::horse_full(
        173,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        HorseColorVariant::White,
        HorseMarkings::WhiteDots,
    )
    .with_invisible(true)
    .with(|s| {
        s.invisible_to_player = false;
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert_eq!(meshes.item_entity_translucent_cull.vertices.len(), 288);
    assert_textured_vertex_state(
        &meshes.item_entity_translucent_cull.vertices,
        instance.render_state.shader_light(),
        instance.render_state.overlay_coords(),
    );
}

#[test]
fn equine_saddle_layer_renders_for_adult_horses_only() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        HORSE_WHITE_TEXTURE_REF,
        HORSE_SADDLE_TEXTURE_REF,
        HORSE_WHITE_BABY_TEXTURE_REF,
    ]))
    .unwrap();

    let base = EntityModelInstance::horse(172, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let bare_meshes = entity_model_textured_meshes(&[base], &atlas);
    assert_equine_buckets(&bare_meshes, base);
    let bare = &bare_meshes.cutout;
    assert_textured_vertex_state(
        &bare.vertices,
        base.render_state.shader_light(),
        base.render_state.overlay_coords(),
    );
    let saddled_instance = base.with(|s| {
        s.kind.equine_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id())
    });
    let saddled_meshes = entity_model_textured_meshes(&[saddled_instance], &atlas);
    assert_equine_buckets(&saddled_meshes, saddled_instance);
    let saddled = &saddled_meshes.cutout;
    let saddle = &saddled_meshes.armor_cutout;
    assert_eq!(saddled.cutout_faces, bare.cutout_faces);
    assert_eq!(saddled.vertices.len(), bare.vertices.len());
    assert_eq!(saddle.cutout_faces, 102);
    assert_eq!(saddle.vertices.len(), 408);

    assert_textured_vertex_state(
        &saddled.vertices,
        saddled_instance.render_state.shader_light(),
        saddled_instance.render_state.overlay_coords(),
    );
    assert_textured_vertex_state(
        &saddle.vertices,
        saddled_instance.render_state.shader_light(),
        [0.0, 10.0],
    );
    assert!(
        saddle.vertices[0].uv[1] >= 64.0 / 192.0,
        "the overlay samples the horse_saddle atlas region"
    );
    let (bare_min, bare_max) = textured_mesh_extents(bare);
    let (saddle_min, saddle_max) = textured_mesh_extents(saddle);
    assert!(saddle_min[0] < bare_min[0]);
    assert!(saddle_max[0] > bare_max[0]);

    let ridden_instance = base.with(|s| {
        s.kind.equine_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
        s.kind.equine_mut().saddle_ridden = true;
    });
    let ridden_meshes = entity_model_textured_meshes(&[ridden_instance], &atlas);
    assert_equine_buckets(&ridden_meshes, ridden_instance);
    let ridden = &ridden_meshes.cutout;
    let ridden_saddle = &ridden_meshes.armor_cutout;
    assert_eq!(ridden.cutout_faces, saddled.cutout_faces);
    assert_eq!(ridden.vertices.len(), saddled.vertices.len());
    assert_eq!(ridden_saddle.cutout_faces - saddle.cutout_faces, 12);
    assert_eq!(ridden_saddle.vertices.len() - saddle.vertices.len(), 48);
    assert_textured_vertex_state(
        &ridden.vertices,
        ridden_instance.render_state.shader_light(),
        ridden_instance.render_state.overlay_coords(),
    );
    assert_textured_vertex_state(
        &ridden_saddle.vertices,
        ridden_instance.render_state.shader_light(),
        [0.0, 10.0],
    );

    let baby_instance = EntityModelInstance::horse(173, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.equine_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
        s.kind.equine_mut().saddle_ridden = true;
    });
    let baby_meshes = entity_model_textured_meshes(&[baby_instance], &atlas);
    assert_equine_buckets(&baby_meshes, baby_instance);
    let baby = &baby_meshes.cutout;
    assert_eq!(baby.cutout_faces, 60);
    assert_eq!(baby.vertices.len(), 240);
    assert_textured_vertex_state(
        &baby.vertices,
        baby_instance.render_state.shader_light(),
        baby_instance.render_state.overlay_coords(),
    );
}

#[test]
fn horse_saddle_missing_saddle_texture_texture_preserves_other_geometry() {
    // Vanilla `HorseRenderer` adds `SimpleEquipmentLayer(HORSE_SADDLE)` after markings and body
    // armor; a present saddle stack records an `armorCutoutNoCull` equipment submission at order 2.
    let images = texture_images(&[HORSE_WHITE_TEXTURE_REF]);
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::horse(174, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.equine_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(
        meshes.cutout.vertices.len(),
        288,
        "missing horse_saddle/saddle.png suppresses only folded saddle geometry"
    );
    assert!(meshes.armor_cutout.vertices.is_empty());
    assert_textured_vertex_state(
        &meshes.cutout.vertices,
        instance.render_state.shader_light(),
        instance.render_state.overlay_coords(),
    );

    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.translucent.indices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
}

#[test]
fn horse_body_armor_layer_renders_for_adult_horse_and_zombie_horse_only() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        HORSE_WHITE_TEXTURE_REF,
        HORSE_WHITE_BABY_TEXTURE_REF,
        ZOMBIE_HORSE_TEXTURE_REF,
        SKELETON_HORSE_TEXTURE_REF,
        HORSE_SADDLE_TEXTURE_REF,
        HORSE_BODY_IRON_TEXTURE_REF,
        HORSE_BODY_LEATHER_TEXTURE_REF,
        HORSE_BODY_LEATHER_OVERLAY_TEXTURE_REF,
        HORSE_BODY_NETHERITE_TEXTURE_REF,
    ]))
    .unwrap();

    let horse = EntityModelInstance::horse(182, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let bare_meshes = entity_model_textured_meshes(&[horse], &atlas);
    assert_equine_buckets(&bare_meshes, horse);
    let bare = &bare_meshes.cutout;
    assert_textured_vertex_state(
        &bare.vertices,
        horse.render_state.shader_light(),
        horse.render_state.overlay_coords(),
    );
    let iron_meshes = entity_model_textured_meshes(
        &[horse.with(|s| {
            s.kind.equine_mut().body_equipment_asset = Some(test_iron_equipment_asset_id())
        })],
        &atlas,
    );
    assert_equine_buckets(
        &iron_meshes,
        horse.with(|s| {
            s.kind.equine_mut().body_equipment_asset = Some(test_iron_equipment_asset_id())
        }),
    );
    let iron = &iron_meshes.cutout;
    let iron_armor = &iron_meshes.armor_cutout;
    assert_eq!(iron.cutout_faces, bare.cutout_faces);
    assert_eq!(iron.vertices.len(), bare.vertices.len());
    assert_eq!(iron_armor.cutout_faces, 72);
    assert_eq!(iron_armor.vertices.len(), 288);

    assert_textured_vertex_state(
        &iron.vertices,
        horse.render_state.shader_light(),
        horse.render_state.overlay_coords(),
    );
    assert_textured_vertex_state(
        &iron_armor.vertices,
        horse.render_state.shader_light(),
        [0.0, 10.0],
    );
    let iron_uv = atlas
        .entries
        .iter()
        .find(|entry| entry.texture == HORSE_BODY_IRON_TEXTURE_REF)
        .unwrap()
        .uv;
    let first_armor_vertex = iron_armor.vertices[0].uv;
    assert!(first_armor_vertex[0] >= iron_uv.min[0]);
    assert!(first_armor_vertex[0] <= iron_uv.max[0]);
    assert!(first_armor_vertex[1] >= iron_uv.min[1]);
    assert!(first_armor_vertex[1] <= iron_uv.max[1]);

    let armored_saddled = horse.with(|s| {
        s.kind.equine_mut().body_equipment_asset = Some(test_iron_equipment_asset_id());
        s.kind.equine_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
    });
    let armored_saddled_meshes = entity_model_textured_meshes(&[armored_saddled], &atlas);
    assert_equine_buckets(&armored_saddled_meshes, armored_saddled);

    assert_eq!(
        armored_saddled_meshes.armor_cutout.vertices.len() - iron_armor.vertices.len(),
        408
    );
    assert_textured_vertex_state(
        &armored_saddled_meshes.cutout.vertices,
        armored_saddled.render_state.shader_light(),
        armored_saddled.render_state.overlay_coords(),
    );
    assert_textured_vertex_state(
        &armored_saddled_meshes.armor_cutout.vertices[..288],
        armored_saddled.render_state.shader_light(),
        [0.0, 10.0],
    );
    assert_textured_vertex_state(
        &armored_saddled_meshes.armor_cutout.vertices[288..],
        armored_saddled.render_state.shader_light(),
        [0.0, 10.0],
    );

    let dye = 0x0033_66CC;
    let dyed_tint = [
        0x33 as f32 / 255.0,
        0x66 as f32 / 255.0,
        0xCC as f32 / 255.0,
        1.0,
    ];
    let leather_meshes = entity_model_textured_meshes(
        &[horse.with(|s| {
            s.kind.equine_mut().body_equipment_asset = Some(test_leather_equipment_asset_id());
            s.kind.equine_mut().body_armor_dye = Some(dye);
        })],
        &atlas,
    );
    assert_equine_buckets(
        &leather_meshes,
        horse.with(|s| {
            s.kind.equine_mut().body_equipment_asset = Some(test_leather_equipment_asset_id());
            s.kind.equine_mut().body_armor_dye = Some(dye);
        }),
    );
    let leather = &leather_meshes.cutout;
    assert_eq!(leather.cutout_faces, bare.cutout_faces);
    assert_eq!(leather.vertices.len(), bare.vertices.len());
    assert_eq!(leather_meshes.armor_cutout.cutout_faces, 144);
    assert_eq!(leather_meshes.armor_cutout.vertices.len(), 576);

    assert_textured_vertex_state(
        &leather.vertices,
        horse.render_state.shader_light(),
        horse.render_state.overlay_coords(),
    );
    let leather_armor = &leather_meshes.armor_cutout;
    assert_textured_vertex_state(
        &leather_armor.vertices[..288],
        horse.render_state.shader_light(),
        [0.0, 10.0],
    );
    assert_textured_vertex_state(
        &leather_armor.vertices[288..],
        horse.render_state.shader_light(),
        [0.0, 10.0],
    );
    assert!(leather_armor.vertices[..288]
        .iter()
        .all(|vertex| vertex.tint == dyed_tint));
    assert!(leather_armor.vertices[288..]
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    let baby = EntityModelInstance::horse(183, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let baby_bare_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_equine_buckets(&baby_bare_meshes, baby);
    let baby_bare = &baby_bare_meshes.cutout;
    let baby_armored_instance = baby
        .with(|s| s.kind.equine_mut().body_equipment_asset = Some(test_iron_equipment_asset_id()));
    let baby_armored_meshes = entity_model_textured_meshes(&[baby_armored_instance], &atlas);
    assert_equine_buckets(&baby_armored_meshes, baby_armored_instance);
    let baby_armored = &baby_armored_meshes.cutout;
    assert_eq!(baby_armored.vertices.len(), baby_bare.vertices.len());
    assert_textured_vertex_state(
        &baby_armored.vertices,
        baby_armored_instance.render_state.shader_light(),
        baby_armored_instance.render_state.overlay_coords(),
    );

    let zombie = EntityModelInstance::undead_horse(
        184,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Zombie,
        false,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let zombie_bare_meshes = entity_model_textured_meshes(&[zombie], &atlas);
    assert_equine_buckets(&zombie_bare_meshes, zombie);
    let zombie_bare = &zombie_bare_meshes.cutout;
    assert_textured_vertex_state(
        &zombie_bare.vertices,
        zombie.render_state.shader_light(),
        zombie.render_state.overlay_coords(),
    );
    let zombie_armored_instance = zombie.with(|s| {
        s.kind.equine_mut().body_equipment_asset = Some(test_netherite_equipment_asset_id())
    });
    let zombie_armored_meshes = entity_model_textured_meshes(&[zombie_armored_instance], &atlas);
    assert_equine_buckets(&zombie_armored_meshes, zombie_armored_instance);
    let zombie_armored = &zombie_armored_meshes.cutout;
    let zombie_armor = &zombie_armored_meshes.armor_cutout;
    assert_eq!(zombie_armored.cutout_faces, zombie_bare.cutout_faces);
    assert_eq!(zombie_armored.vertices.len(), zombie_bare.vertices.len());
    assert_eq!(zombie_armor.cutout_faces, 72);
    assert_eq!(zombie_armor.vertices.len(), 288);
    assert_textured_vertex_state(
        &zombie_armored.vertices,
        zombie_armored_instance.render_state.shader_light(),
        zombie_armored_instance.render_state.overlay_coords(),
    );
    assert_textured_vertex_state(
        &zombie_armor.vertices,
        zombie_armored_instance.render_state.shader_light(),
        [0.0, 10.0],
    );

    let skeleton = EntityModelInstance::undead_horse(
        185,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Skeleton,
        false,
    )
    .with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let skeleton_bare_meshes = entity_model_textured_meshes(&[skeleton], &atlas);
    assert_equine_buckets(&skeleton_bare_meshes, skeleton);
    let skeleton_bare = &skeleton_bare_meshes.cutout;
    assert_textured_vertex_state(
        &skeleton_bare.vertices,
        skeleton.render_state.shader_light(),
        skeleton.render_state.overlay_coords(),
    );
    let skeleton_armored_instance = skeleton.with(|s| {
        s.kind.equine_mut().body_equipment_asset = Some(test_netherite_equipment_asset_id())
    });
    let skeleton_armored_meshes =
        entity_model_textured_meshes(&[skeleton_armored_instance], &atlas);
    assert_equine_buckets(&skeleton_armored_meshes, skeleton_armored_instance);
    let skeleton_armored = &skeleton_armored_meshes.cutout;
    assert_eq!(
        skeleton_armored.vertices.len(),
        skeleton_bare.vertices.len(),
        "EntityTypeTags.CAN_WEAR_HORSE_ARMOR excludes skeleton horses"
    );
    assert!(skeleton_armored_meshes.armor_cutout.vertices.is_empty());

    let invalid_material_instance = horse.with(|s| {
        s.kind.equine_mut().body_equipment_asset = Some(test_equipment_asset_id(u32::MAX))
    });
    let invalid_material_meshes =
        entity_model_textured_meshes(&[invalid_material_instance], &atlas);
    assert_equine_buckets(&invalid_material_meshes, invalid_material_instance);
    let invalid_material = &invalid_material_meshes.cutout;
    assert_eq!(
        invalid_material.vertices.len(),
        bare.vertices.len(),
        "vanilla 26.1 has no chainmail horse_body equipment texture"
    );
    assert!(invalid_material_meshes.armor_cutout.vertices.is_empty());
}

#[test]
fn horse_body_armor_trim_follows_foil_and_precedes_saddle() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        HORSE_WHITE_TEXTURE_REF,
        HORSE_WHITE_BABY_TEXTURE_REF,
        HORSE_BODY_LEATHER_TEXTURE_REF,
        HORSE_BODY_LEATHER_OVERLAY_TEXTURE_REF,
        HORSE_SADDLE_TEXTURE_REF,
        ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
    ]))
    .unwrap();
    let trim_texture = EntityDynamicTexture { handle: 91_201 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [64, 64],
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 64 * 4],
    }])
    .unwrap()
    .0;
    let dye = 0x0033_66CC;
    let horse = EntityModelInstance::horse(186, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.kind.equine_mut().body_equipment_asset = Some(test_leather_equipment_asset_id());
        s.kind.equine_mut().body_armor_dye = Some(dye);
        s.kind.equine_mut().body_armor_foil = true;
        s.kind.equine_mut().body_armor_trim = Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        });
        s.kind.equine_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[horse],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_eq!(
        meshes.dynamic_player_texture_armor_trim.vertices.len(),
        288,
        "the trim reuses the posed adult HORSE_ARMOR model"
    );
    assert_eq!(meshes.armor_entity_glint.vertices.len(), 288);

    let baby = EntityModelInstance::horse(187, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.kind.equine_mut().body_equipment_asset = Some(test_leather_equipment_asset_id());
        s.kind.equine_mut().body_armor_foil = true;
        s.kind.equine_mut().body_armor_trim = Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        });
        s.kind.equine_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
    });
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

#[test]
fn horse_body_armor_missing_texture_preserves_other_geometry() {
    let (atlas, _) =
        build_entity_model_texture_atlas(&texture_images(&[HORSE_WHITE_TEXTURE_REF])).unwrap();
    let horse = EntityModelInstance::horse(186, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.equine_mut().body_equipment_asset = Some(test_iron_equipment_asset_id());
    });

    let meshes = entity_model_textured_meshes(&[horse], &atlas);
    assert_equine_buckets(&meshes, horse);

    assert_eq!(meshes.cutout.vertices.len(), 288);
    assert!(meshes.armor_cutout.vertices.is_empty());

    assert_textured_vertex_state(
        &meshes.cutout.vertices,
        horse.render_state.shader_light(),
        horse.render_state.overlay_coords(),
    );
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay != [0.0, 10.0]));
}

#[test]
fn equine_saddle_layer_uses_family_specific_models_and_textures() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        DONKEY_TEXTURE_REF,
        DONKEY_SADDLE_TEXTURE_REF,
        MULE_TEXTURE_REF,
        MULE_SADDLE_TEXTURE_REF,
        SKELETON_HORSE_TEXTURE_REF,
        SKELETON_HORSE_SADDLE_TEXTURE_REF,
        ZOMBIE_HORSE_TEXTURE_REF,
        ZOMBIE_HORSE_SADDLE_TEXTURE_REF,
    ]))
    .unwrap();

    let donkey_instance = EntityModelInstance::donkey(
        174,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        false,
        false,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.equine_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
    });
    let donkey_meshes = entity_model_textured_meshes(&[donkey_instance], &atlas);
    assert_equine_buckets(&donkey_meshes, donkey_instance);
    let donkey = &donkey_meshes.cutout;
    let donkey_saddle = &donkey_meshes.armor_cutout;
    assert_eq!(donkey.cutout_faces, 72);
    assert_eq!(donkey.vertices.len(), 288);
    assert_eq!(donkey_saddle.cutout_faces, 114);
    assert_eq!(donkey_saddle.vertices.len(), 456);

    assert_ne!(donkey_instance.render_state.overlay_coords(), [0.0, 10.0]);

    assert!(donkey.vertices.iter().all(|vertex| vertex.light
        == donkey_instance.render_state.shader_light()
        && vertex.overlay == donkey_instance.render_state.overlay_coords()));
    assert!(donkey_saddle.vertices.iter().all(|vertex| {
        vertex.light == donkey_instance.render_state.shader_light() && vertex.overlay == [0.0, 10.0]
    }));

    let mule_instance = EntityModelInstance::donkey(
        175,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Mule,
        false,
        false,
    )
    .with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.equine_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
    });
    let mule_meshes = entity_model_textured_meshes(&[mule_instance], &atlas);
    assert_equine_buckets(&mule_meshes, mule_instance);
    let mule = &mule_meshes.cutout;
    let mule_saddle_mesh = &mule_meshes.armor_cutout;
    assert_eq!(mule.cutout_faces, 72);
    assert_eq!(mule_saddle_mesh.cutout_faces, 114);

    assert!(mule.vertices.iter().all(|vertex| vertex.light
        == mule_instance.render_state.shader_light()
        && vertex.overlay == mule_instance.render_state.overlay_coords()));
    assert!(mule_saddle_mesh.vertices.iter().all(|vertex| vertex.light
        == mule_instance.render_state.shader_light()
        && vertex.overlay == [0.0, 10.0]));
    let (_, donkey_max) = textured_mesh_extents(donkey_saddle);
    let (_, mule_max) = textured_mesh_extents(mule_saddle_mesh);
    assert!(
        mule_max[1] > donkey_max[1],
        "the mule saddle layer uses the larger 0.92 layer scale"
    );

    let skeleton_instance = EntityModelInstance::undead_horse(
        176,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Skeleton,
        false,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.equine_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
    });
    let skeleton_meshes = entity_model_textured_meshes(&[skeleton_instance], &atlas);
    assert_equine_buckets(&skeleton_meshes, skeleton_instance);
    let skeleton = &skeleton_meshes.cutout;
    let zombie_instance = EntityModelInstance::undead_horse(
        177,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Zombie,
        false,
    )
    .with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
        s.kind.equine_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
    });
    let zombie_meshes = entity_model_textured_meshes(&[zombie_instance], &atlas);
    assert_equine_buckets(&zombie_meshes, zombie_instance);
    let zombie = &zombie_meshes.cutout;
    let skeleton_saddle = &skeleton_meshes.armor_cutout;
    let zombie_saddle = &zombie_meshes.armor_cutout;
    assert_eq!(skeleton.cutout_faces, 72);
    assert_eq!(zombie.cutout_faces, 72);
    assert_eq!(skeleton_saddle.cutout_faces, 102);
    assert_eq!(zombie_saddle.cutout_faces, 102);

    assert_textured_vertex_state(
        &skeleton.vertices,
        skeleton_instance.render_state.shader_light(),
        skeleton_instance.render_state.overlay_coords(),
    );
    assert_textured_vertex_state(
        &skeleton_saddle.vertices,
        skeleton_instance.render_state.shader_light(),
        [0.0, 10.0],
    );
    assert_textured_vertex_state(
        &zombie.vertices,
        zombie_instance.render_state.shader_light(),
        zombie_instance.render_state.overlay_coords(),
    );
    assert_textured_vertex_state(
        &zombie_saddle.vertices,
        zombie_instance.render_state.shader_light(),
        [0.0, 10.0],
    );
    let skeleton_overlay_uv = skeleton_saddle.vertices[0].uv;
    let zombie_overlay_uv = zombie_saddle.vertices[0].uv;
    assert_ne!(
        skeleton_overlay_uv, zombie_overlay_uv,
        "skeleton and zombie horse saddles sample different equipment-layer atlas regions"
    );
}

#[test]
fn equine_saddle_foil_and_trim_keep_family_models_and_vanilla_order() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        HORSE_WHITE_TEXTURE_REF,
        HORSE_SADDLE_TEXTURE_REF,
        DONKEY_TEXTURE_REF,
        DONKEY_SADDLE_TEXTURE_REF,
        MULE_TEXTURE_REF,
        MULE_SADDLE_TEXTURE_REF,
        SKELETON_HORSE_TEXTURE_REF,
        SKELETON_HORSE_SADDLE_TEXTURE_REF,
        ZOMBIE_HORSE_TEXTURE_REF,
        ZOMBIE_HORSE_SADDLE_TEXTURE_REF,
        ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
    ]))
    .unwrap();
    let trim_texture = EntityDynamicTexture { handle: 91_202 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [64, 64],
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 64 * 4],
    }])
    .unwrap()
    .0;
    let trim = Some(EntityArmorTrim {
        texture: trim_texture,
        decal: false,
    });
    let cases = [
        (
            EntityModelInstance::horse(190, [0.0, 64.0, 0.0], 0.0, false),
            408,
        ),
        (
            EntityModelInstance::donkey(
                191,
                [0.0, 64.0, 0.0],
                0.0,
                DonkeyModelFamily::Donkey,
                false,
                false,
            ),
            456,
        ),
        (
            EntityModelInstance::donkey(
                192,
                [0.0, 64.0, 0.0],
                0.0,
                DonkeyModelFamily::Mule,
                false,
                false,
            ),
            456,
        ),
        (
            EntityModelInstance::undead_horse(
                193,
                [0.0, 64.0, 0.0],
                0.0,
                UndeadHorseModelFamily::Skeleton,
                false,
            ),
            408,
        ),
        (
            EntityModelInstance::undead_horse(
                194,
                [0.0, 64.0, 0.0],
                0.0,
                UndeadHorseModelFamily::Zombie,
                false,
            ),
            408,
        ),
    ];

    for (instance, vertex_count) in cases {
        let instance = instance.with(|s| {
            s.kind.equine_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
            s.kind.equine_mut().saddle_foil = true;
            s.kind.equine_mut().saddle_trim = trim;
        });
        let meshes = entity_model_textured_meshes_with_dynamic_textures(
            &[instance],
            &atlas,
            None,
            Some(&dynamic_atlas),
        );

        assert_eq!(meshes.armor_cutout.vertices.len(), vertex_count);
        assert_eq!(meshes.armor_entity_glint.vertices.len(), vertex_count);
        assert_eq!(
            meshes.dynamic_player_texture_armor_trim.vertices.len(),
            vertex_count,
            "trim must reuse the selected family saddle model"
        );
        assert_textured_vertex_state(
            &meshes.armor_cutout.vertices,
            instance.render_state.shader_light(),
            [0.0, 10.0],
        );
        assert_textured_vertex_state(
            &meshes.dynamic_player_texture_armor_trim.vertices,
            instance.render_state.shader_light(),
            [0.0, 10.0],
        );
    }
}

#[test]
fn donkey_textured_mesh_matches_vanilla_adult_geometry() {
    // Vanilla `DonkeyModel` / `MuleModel` reuse `AbstractEquineModel.createBodyMesh` + `modifyMesh`
    // (donkey ears + side chests), so the adult donkey/mule render on the textured path: 12 cubes
    // (72 faces / 288 vertices) without chest, 14 (84 / 336) with. The mule shares the geometry at the larger
    // 0.92 scale. The baby donkey/mule re-parented mesh is exercised separately.
    let (atlas, _) = build_entity_model_texture_atlas(&donkey_texture_images()).unwrap();

    let donkey_instance = EntityModelInstance::donkey(
        160,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        false,
        false,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let donkey_meshes = entity_model_textured_meshes(&[donkey_instance], &atlas);
    assert_equine_buckets(&donkey_meshes, donkey_instance);
    let donkey = &donkey_meshes.cutout;
    assert_eq!(donkey.cutout_faces, 72);
    assert_eq!(donkey.vertices.len(), 288);

    assert!(donkey.vertices.iter().all(|vertex| vertex.light
        == donkey_instance.render_state.shader_light()
        && vertex.overlay == donkey_instance.render_state.overlay_coords()));
    let (_t_min, t_max) = textured_mesh_extents(donkey);

    // The two side chests add 12 faces (2 boxes × 6).
    let with_chest_instance = EntityModelInstance::donkey(
        162,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        false,
        true,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let with_chest_meshes = entity_model_textured_meshes(&[with_chest_instance], &atlas);
    assert_equine_buckets(&with_chest_meshes, with_chest_instance);
    let with_chest = &with_chest_meshes.cutout;
    assert_eq!(with_chest.cutout_faces, 84);
    assert_eq!(with_chest.vertices.len(), 336);

    assert!(with_chest.vertices.iter().all(|vertex| vertex.light
        == with_chest_instance.render_state.shader_light()
        && vertex.overlay == with_chest_instance.render_state.overlay_coords()));

    // The mule shares the geometry at the larger 0.92 scale (vs donkey 0.87) and a different texture.
    let mule_instance = EntityModelInstance::donkey(
        163,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Mule,
        false,
        false,
    )
    .with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let mule_meshes = entity_model_textured_meshes(&[mule_instance], &atlas);
    assert_equine_buckets(&mule_meshes, mule_instance);
    let mule = &mule_meshes.cutout;
    assert_eq!(mule.cutout_faces, 72);

    assert!(mule.vertices.iter().all(|vertex| vertex.light
        == mule_instance.render_state.shader_light()
        && vertex.overlay == mule_instance.render_state.overlay_coords()));
    let (_, mule_max) = textured_mesh_extents(mule);
    assert!(
        mule_max[1] > t_max[1],
        "the mule renders at the larger 0.92 scale"
    );
}

#[test]
fn donkey_textured_baby_matches_vanilla_baby_geometry() {
    // Vanilla `BabyDonkeyModel.createBabyLayer()` is a distinct re-parented mesh (10 cubes, 60 faces /
    // 240 vertices). Its `setupAnim` reuses the equine nested leg/tail fields, then forces the default
    // head xRot to 0, and the empty chest children make `hasChest` immaterial.
    let (atlas, _) = build_entity_model_texture_atlas(&donkey_texture_images()).unwrap();
    let baby_instance = EntityModelInstance::donkey(
        165,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Donkey,
        true,
        false,
    );
    let baby_meshes = entity_model_textured_meshes(&[baby_instance], &atlas);
    assert_equine_buckets(&baby_meshes, baby_instance);
    let baby = &baby_meshes.cutout;
    assert_eq!(baby.cutout_faces, 60);
    assert_eq!(baby.vertices.len(), 240);

    assert_textured_vertex_state(
        &baby.vertices,
        baby_instance.render_state.shader_light(),
        baby_instance.render_state.overlay_coords(),
    );

    let walking_instance =
        baby_instance.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let walking_meshes = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_equine_buckets(&walking_meshes, walking_instance);

    assert_textured_vertex_state(
        &walking_meshes.cutout.vertices,
        walking_instance.render_state.shader_light(),
        walking_instance.render_state.overlay_coords(),
    );
    assert_eq!(walking_meshes.cutout.vertices.len(), baby.vertices.len());
    assert_eq!(
        baby.vertices[0..24],
        walking_meshes.cutout.vertices[0..24],
        "the textured baby body cube stays put"
    );
    assert_ne!(
        baby.vertices[24..144],
        walking_meshes.cutout.vertices[24..144],
        "the textured baby tail and nested legs re-pose when walking"
    );

    // `hasChest` does not change the baby (its chest children are empty).
    let baby_chest_instance = EntityModelInstance::donkey(
        167,
        [0.0, 64.0, 0.0],
        0.0,
        DonkeyModelFamily::Mule,
        true,
        true,
    );
    let baby_chest_meshes = entity_model_textured_meshes(&[baby_chest_instance], &atlas);
    assert_equine_buckets(&baby_chest_meshes, baby_chest_instance);
    let baby_chest = &baby_chest_meshes.cutout;
    assert_eq!(baby_chest.cutout_faces, 60);
}

#[test]
fn undead_horse_textured_mesh_matches_vanilla_horse_geometry() {
    // Vanilla `UndeadHorseRenderer extends HorseRenderer`, so the skeleton/zombie horses render the
    // shared `HorseModel` / `BabyHorseModel` geometry on the textured path: 12 adult cubes (72 faces /
    // 288 vertices) and 10 baby cubes (60 faces / 240 vertices).
    let (atlas, _) = build_entity_model_texture_atlas(&undead_horse_texture_images()).unwrap();

    let skeleton_adult_instance = EntityModelInstance::undead_horse(
        170,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Skeleton,
        false,
    )
    .with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let skeleton_adult_meshes = entity_model_textured_meshes(&[skeleton_adult_instance], &atlas);
    assert_equine_buckets(&skeleton_adult_meshes, skeleton_adult_instance);
    let skeleton_adult = &skeleton_adult_meshes.cutout;
    assert_eq!(skeleton_adult.cutout_faces, 72);
    assert_eq!(skeleton_adult.vertices.len(), 288);
    assert_eq!(skeleton_adult.indices.len(), 432);
    assert_textured_vertex_state(
        &skeleton_adult.vertices,
        skeleton_adult_instance.render_state.shader_light(),
        skeleton_adult_instance.render_state.overlay_coords(),
    );
    let (adult_min, adult_max) = textured_mesh_extents(skeleton_adult);
    assert_close3(adult_min, [-0.31562507, 64.001625, -1.0915062]);
    assert_close3(adult_max, [0.31562507, 66.11081, 1.4726361]);

    // Same geometry, different family → identical vertex positions but a different atlas sub-rect.
    let zombie_adult_instance = EntityModelInstance::undead_horse(
        171,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Zombie,
        false,
    )
    .with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let zombie_adult_meshes = entity_model_textured_meshes(&[zombie_adult_instance], &atlas);
    assert_equine_buckets(&zombie_adult_meshes, zombie_adult_instance);
    let zombie_adult = &zombie_adult_meshes.cutout;
    assert_textured_vertex_state(
        &zombie_adult.vertices,
        zombie_adult_instance.render_state.shader_light(),
        zombie_adult_instance.render_state.overlay_coords(),
    );
    let skeleton_positions: Vec<_> = skeleton_adult.vertices.iter().map(|v| v.position).collect();
    let zombie_positions: Vec<_> = zombie_adult.vertices.iter().map(|v| v.position).collect();
    assert_eq!(skeleton_positions, zombie_positions);
    let skeleton_uvs: Vec<_> = skeleton_adult.vertices.iter().map(|v| v.uv).collect();
    let zombie_uvs: Vec<_> = zombie_adult.vertices.iter().map(|v| v.uv).collect();
    assert_ne!(skeleton_uvs, zombie_uvs);

    // The baby re-parented layout (`BabyHorseModel.createBabyLayer`) renders with the same textured path.
    let zombie_baby_instance = EntityModelInstance::undead_horse(
        172,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Zombie,
        true,
    )
    .with(|s| {
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let zombie_baby_meshes = entity_model_textured_meshes(&[zombie_baby_instance], &atlas);
    assert_equine_buckets(&zombie_baby_meshes, zombie_baby_instance);
    let zombie_baby = &zombie_baby_meshes.cutout;
    assert_eq!(zombie_baby.cutout_faces, 60);
    assert_eq!(zombie_baby.vertices.len(), 240);
    assert_textured_vertex_state(
        &zombie_baby.vertices,
        zombie_baby_instance.render_state.shader_light(),
        zombie_baby_instance.render_state.overlay_coords(),
    );
}

#[test]
fn undead_horse_textured_swings_legs_when_walking() {
    // The undead horse reuses `HorseModel.setupAnim`, so the textured path swings the legs and lifts
    // the tail with the gait: a still horse matches the rest pose, a walking one differs.
    let (atlas, _) = build_entity_model_texture_atlas(&undead_horse_texture_images()).unwrap();
    let base = EntityModelInstance::undead_horse(
        190,
        [0.0, 64.0, 0.0],
        0.0,
        UndeadHorseModelFamily::Skeleton,
        false,
    );
    let still_instance = base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 0.0));
    let walking_instance =
        base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (5.0, 1.0));
    let still_meshes = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking_meshes = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_equine_buckets(&still_meshes, still_instance);
    assert_equine_buckets(&walking_meshes, walking_instance);
    let still = &still_meshes.cutout;
    let walking = &walking_meshes.cutout;
    assert_eq!(still.vertices.len(), walking.vertices.len());
    assert_ne!(
        still.vertices, walking.vertices,
        "the walking undead horse re-poses on the textured path"
    );
}
