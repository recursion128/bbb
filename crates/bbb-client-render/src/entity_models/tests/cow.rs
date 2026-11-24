use super::*;

#[test]
fn entity_texture_atlas_stitches_official_cow_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&cow_texture_images()).unwrap();

    let textures = &texture_refs_matching(|path| path.starts_with("textures/entity/cow/cow_"));
    assert_texture_atlas_layout_matches_refs(&layout, textures);
    let warm_baby_first_pixel =
        rgba_offset(layout.width, texture_atlas_entry_y(textures, 3), 0, "test").unwrap();
    assert_eq!(
        &rgba[warm_baby_first_pixel..warm_baby_first_pixel + 4],
        &[3; 4]
    );
    let cold_first_pixel =
        rgba_offset(layout.width, texture_atlas_entry_y(textures, 4), 0, "test").unwrap();
    assert_eq!(&rgba[cold_first_pixel..cold_first_pixel + 4], &[4; 4]);
}

#[test]
fn cow_textured_mesh_uses_vanilla_uvs_tints_and_variant_textures() {
    let (atlas, _) = build_entity_model_texture_atlas(&cow_texture_images()).unwrap();
    let instances = [
        EntityModelInstance::cow_variant(
            601,
            [0.0, 64.0, 0.0],
            0.0,
            CowModelVariant::Temperate,
            false,
        )
        .with(|s| {
            s.light_coords = (6_u32 << 4) | (13_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        }),
        EntityModelInstance::cow_variant(602, [1.0, 64.0, 0.0], 0.0, CowModelVariant::Cold, false)
            .with(|s| {
                s.light_coords = (6_u32 << 4) | (13_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            }),
        EntityModelInstance::cow_variant(603, [2.0, 64.0, 0.0], 0.0, CowModelVariant::Warm, true)
            .with(|s| {
                s.light_coords = (6_u32 << 4) | (13_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            }),
    ];
    let meshes = entity_model_textured_meshes(&instances, &atlas);
    assert_cow_textured_output(&meshes);
    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 180);
    assert_eq!(mesh.vertices.len(), 720);
    assert_eq!(mesh.indices.len(), 1080);
    assert_close2(mesh.vertices[0].uv, [14.0 / 64.0, 0.0]);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close3(mesh.vertices[0].normal, [0.0, 1.0, 0.0]);
    assert_close2(mesh.vertices[240].uv, [14.0 / 64.0, 256.0 / 384.0]);
    assert_eq!(mesh.vertices[240].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[504].uv, [11.0 / 64.0, 210.0 / 384.0]);
    assert_eq!(mesh.vertices[504].tint, [1.0, 1.0, 1.0, 1.0]);
    assert!(mesh.vertices.iter().all(|vertex| {
        let [x, y, z] = vertex.normal;
        ((x * x + y * y + z * z) - 1.0).abs() < 1.0e-4
    }));
    assert!(mesh.vertices.iter().any(|vertex| vertex.normal[1] > 0.99));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.normal[0].abs() > 0.99));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.normal[2].abs() > 0.99));
    assert!(mesh.vertices.iter().all(|vertex| vertex.light
        == instances[0].render_state.shader_light()
        && vertex.overlay == instances[0].render_state.overlay_coords()));
    assert_ne!(instances[0].render_state.overlay_coords(), [0.0, 10.0]);
    let (min, max) = textured_mesh_extents(mesh);
    assert_close3(min, [-0.375, 64.001, -0.65625]);
    assert_close3(max, [2.25, 65.5635, 1.0]);
}

#[test]
fn visible_glowing_cow_folds_vanilla_outline_buffer_copy() {
    let (atlas, _) = build_entity_model_texture_atlas(&cow_texture_images()).unwrap();
    let cow = EntityModelInstance::cow_variant(
        604,
        [0.0, 64.0, 0.0],
        0.0,
        CowModelVariant::Temperate,
        false,
    )
    .with_outline_color(0xff33_66cc)
    .with(|s| {
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[cow], &atlas);

    assert_eq!(meshes.cutout.vertices.len(), 240);
    assert_eq!(meshes.outline.vertices.len(), meshes.cutout.vertices.len());
    assert_eq!(meshes.outline.indices.len(), meshes.cutout.indices.len());
    assert_eq!(meshes.outline.cutout_faces, meshes.cutout.cutout_faces);
    let outline_tint = [
        0x33 as f32 / 255.0,
        0x66 as f32 / 255.0,
        0xcc as f32 / 255.0,
        1.0,
    ];
    assert!(meshes.outline.vertices.iter().all(|vertex| {
        vertex.tint == outline_tint
            && vertex.light == cow.render_state.shader_light()
            && vertex.overlay == cow.render_state.overlay_coords()
    }));
}

#[test]
fn cow_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let temperate = cow_textured_layer_passes(CowModelVariant::Temperate, false);
    assert_eq!(temperate.len(), 1);
    assert_eq!(
        temperate[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(temperate[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(temperate[0].texture, COW_TEMPERATE_TEXTURE_REF);
    assert_eq!(temperate[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(temperate[0].order, 0);

    let temperate_baby = cow_textured_layer_passes(CowModelVariant::Temperate, true);
    assert_eq!(temperate_baby[0].texture, COW_TEMPERATE_BABY_TEXTURE_REF);

    let warm = cow_textured_layer_passes(CowModelVariant::Warm, false);
    assert_eq!(warm[0].texture, COW_WARM_TEXTURE_REF);

    let warm_baby = cow_textured_layer_passes(CowModelVariant::Warm, true);
    assert_eq!(warm_baby[0].texture, COW_WARM_BABY_TEXTURE_REF);

    let cold = cow_textured_layer_passes(CowModelVariant::Cold, false);
    assert_eq!(cold[0].texture, COW_COLD_TEXTURE_REF);

    let cold_baby = cow_textured_layer_passes(CowModelVariant::Cold, true);
    assert_eq!(cold_baby[0].texture, COW_COLD_BABY_TEXTURE_REF);
}

#[test]
fn cow_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&cow_texture_images()).unwrap();
    let base = EntityModelInstance::cow_variant(
        604,
        [0.0, 64.0, 0.0],
        0.0,
        CowModelVariant::Temperate,
        false,
    );
    let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (45.0, 0.0));
    let pitched_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -25.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
    let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
    assert_cow_textured_output(&resting);
    assert_cow_textured_output(&yawed);
    assert_cow_textured_output(&pitched);

    // Head look turns the textured head part without adding or dropping vertices.
    assert_eq!(resting.cutout.vertices.len(), yawed.cutout.vertices.len());
    assert_ne!(resting.cutout.vertices, yawed.cutout.vertices);
    assert_ne!(resting.cutout.vertices, pitched.cutout.vertices);
    assert_ne!(yawed.cutout.vertices, pitched.cutout.vertices);
}

#[test]
fn cow_textured_mesh_swings_legs_when_walking() {
    // The real cow render path (textured) consumes the projected limb swing via the
    // vanilla QuadrupedModel.setupAnim leg rotation. A standing cow is byte-identical
    // however far the swing position has advanced, and a walking cow's feet lift off
    // the ground (its lowest point rises), for every variant and the baby layer.
    let (atlas, _) = build_entity_model_texture_atlas(&cow_texture_images()).unwrap();
    for (variant, baby) in [
        (CowModelVariant::Temperate, false),
        (CowModelVariant::Warm, false),
        (CowModelVariant::Cold, false),
        (CowModelVariant::Temperate, true),
    ] {
        let base = EntityModelInstance::cow_variant(605, [0.0, 64.0, 0.0], 0.0, variant, baby);
        let still_instance = base.with(|s| {
            s.age_in_ticks = 37.5;
            (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0);
        });
        let walking_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_cow_textured_output(&resting);
        assert_cow_textured_output(&still);
        assert_cow_textured_output(&walking);

        assert_eq!(
            resting.cutout.vertices, still.cutout.vertices,
            "{variant:?} baby={baby}: a standing cow is inert"
        );
        assert_eq!(
            resting.cutout.vertices.len(),
            walking.cutout.vertices.len(),
            "{variant:?} baby={baby}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.cutout.vertices, walking.cutout.vertices,
            "{variant:?} baby={baby}: a walking cow differs"
        );

        let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
        let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.1,
            "{variant:?} baby={baby}: a walking cow's feet should lift off the ground"
        );
    }
}

fn assert_cow_textured_output(meshes: &EntityModelTexturedMeshes) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}

fn cow_texture_images() -> Vec<EntityModelTextureImage> {
    texture_refs_matching(|path| path.starts_with("textures/entity/cow/cow_"))
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
