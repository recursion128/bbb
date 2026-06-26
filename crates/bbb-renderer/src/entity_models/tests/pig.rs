use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn pig_adult_model_parts_match_vanilla_26_1_body_layer() {
    // The unified cubes carry both render paths' geometry: the colored debug tint and the textured
    // `uv_size`/`texOffs`/`mirror`.
    assert_eq!(
        ADULT_PIG_HEAD,
        [
            ModelCube::new(
                [-4.0, -4.0, -8.0],
                [8.0, 8.0, 8.0],
                PIG_PINK,
                [8.0, 8.0, 8.0],
                [0.0, 0.0],
                false,
            ),
            ModelCube::new(
                [-2.0, 0.0, -9.0],
                [4.0, 3.0, 1.0],
                PIG_PINK,
                [4.0, 3.0, 1.0],
                [16.0, 16.0],
                false,
            ),
        ]
    );
    assert_eq!(
        ADULT_PIG_BODY[0],
        ModelCube::new(
            [-5.0, -10.0, -7.0],
            [10.0, 16.0, 8.0],
            PIG_PINK,
            [10.0, 16.0, 8.0],
            [28.0, 8.0],
            false,
        )
    );
    assert_eq!(
        ADULT_PIG_LEG[0],
        ModelCube::new(
            [-2.0, 0.0, -2.0],
            [4.0, 6.0, 4.0],
            PIG_PINK,
            [4.0, 6.0, 4.0],
            [0.0, 16.0],
            false,
        )
    );
}

#[test]
fn pig_cold_adult_model_parts_match_vanilla_26_1_body_layer() {
    // The cold body's second cube inflates the geometry (colored 11×17×9) while the textured
    // `uv_size` keeps the base 10×16×8 (the squid body precedent).
    assert_eq!(
        COLD_PIG_BODY,
        [
            ModelCube::new(
                [-5.0, -10.0, -7.0],
                [10.0, 16.0, 8.0],
                PIG_PINK,
                [10.0, 16.0, 8.0],
                [28.0, 8.0],
                false,
            ),
            ModelCube::new(
                [-5.5, -10.5, -7.5],
                [11.0, 17.0, 9.0],
                PIG_COLD_FUR,
                [10.0, 16.0, 8.0],
                [28.0, 32.0],
                false,
            ),
        ]
    );
}

#[test]
fn pig_adult_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::pig(
        90,
        [0.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Temperate,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.3125, 64.001, -0.5625]);
    assert_close3(max, [0.3125, 65.001, 0.9375]);
}

#[test]
fn pig_cold_adult_model_mesh_uses_vanilla_cold_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::pig(
        92,
        [0.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Cold,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 48);
    assert_eq!(mesh.vertices.len(), 192);
    assert_eq!(mesh.indices.len(), 288);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.34375, 64.001, -0.5625]);
    assert_close3(max, [0.34375, 65.001, 0.9375]);
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PIG_COLD_FUR, 0.78)));
}

#[test]
fn pig_baby_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        BABY_PIG_BODY[0],
        ModelCube::new(
            [-3.5, -3.0, -4.5],
            [7.0, 6.0, 9.0],
            PIG_PINK,
            [7.0, 6.0, 9.0],
            [0.0, 0.0],
            false,
        )
    );
    // BabyPigModel bakes the deformation into the colored geometry while the UV box stays the base.
    assert_eq!(
        BABY_PIG_HEAD,
        [
            ModelCube::new(
                [-3.525, -5.025, -5.025],
                [7.05, 6.05, 6.05],
                PIG_PINK,
                [7.0, 6.0, 6.0],
                [0.0, 15.0],
                false,
            ),
            ModelCube::new(
                [-1.515, -1.99, -6.015],
                [3.03, 2.03, 1.03],
                PIG_PINK,
                [3.0, 2.0, 1.0],
                [6.0, 27.0],
                false,
            ),
        ]
    );
    // The baby legs share one geometry but distinct per-corner UV origins.
    assert_eq!(BABY_PIG_LEFT_FRONT_LEG[0].tex, [0.0, 0.0]);
    assert_eq!(BABY_PIG_RIGHT_FRONT_LEG[0].tex, [23.0, 0.0]);
    assert_eq!(BABY_PIG_LEFT_HIND_LEG[0].tex, [0.0, 4.0]);
    assert_eq!(BABY_PIG_RIGHT_HIND_LEG[0].tex, [23.0, 4.0]);
}

#[test]
fn pig_baby_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::pig(
        91,
        [0.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Warm,
        true,
    )]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.2203125, 64.001, -0.3125]);
    assert_close3(max, [0.2203125, 64.62756, 0.5009375]);
}

#[test]
fn pig_texture_refs_match_vanilla_variant_assets() {
    let cases = [
        (
            PigModelVariant::Temperate,
            false,
            "pig_temperate",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_temperate.png",
                size: [64, 64],
            },
        ),
        (
            PigModelVariant::Temperate,
            true,
            "pig_temperate_baby",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_temperate_baby.png",
                size: [32, 32],
            },
        ),
        (
            PigModelVariant::Warm,
            false,
            "pig_warm",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_warm.png",
                size: [64, 64],
            },
        ),
        (
            PigModelVariant::Warm,
            true,
            "pig_warm_baby",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_warm_baby.png",
                size: [32, 32],
            },
        ),
        (
            PigModelVariant::Cold,
            false,
            "pig_cold",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_cold.png",
                size: [64, 64],
            },
        ),
        (
            PigModelVariant::Cold,
            true,
            "pig_cold_baby",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_cold_baby.png",
                size: [32, 32],
            },
        ),
    ];

    for (variant, baby, model_key, texture) in cases {
        let kind = EntityModelKind::Pig { variant, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn pig_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let temperate = pig_textured_layer_passes(PigModelVariant::Temperate, false);
    assert_eq!(temperate.len(), 1);
    assert_eq!(temperate[0].kind, EntityModelLayerKind::PigBase);
    assert_eq!(temperate[0].model_layer, MODEL_LAYER_PIG);
    assert_eq!(temperate[0].texture, PIG_TEMPERATE_TEXTURE_REF);
    assert_eq!(temperate[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((temperate[0].order, temperate[0].submit_sequence), (0, 0));

    let warm_baby = pig_textured_layer_passes(PigModelVariant::Warm, true);
    assert_eq!(warm_baby[0].model_layer, MODEL_LAYER_PIG_BABY);
    assert_eq!(warm_baby[0].texture, PIG_WARM_BABY_TEXTURE_REF);

    let cold_adult = pig_textured_layer_passes(PigModelVariant::Cold, false);
    assert_eq!(cold_adult[0].model_layer, MODEL_LAYER_COLD_PIG);
    assert_eq!(cold_adult[0].texture, PIG_COLD_TEXTURE_REF);
}

#[test]
fn pig_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    // The textured UV sources now live on the unified cubes (`uv_size`/`tex`/`mirror`).
    assert_eq!(MODEL_LAYER_PIG, "minecraft:pig#main");
    assert_eq!(MODEL_LAYER_PIG_BABY, "minecraft:pig_baby#main");
    assert_eq!(MODEL_LAYER_COLD_PIG, "minecraft:cold_pig#main");
    assert_eq!(MODEL_LAYER_PIG_SADDLE, "minecraft:pig#saddle");
    assert_eq!(ADULT_PIG_HEAD[0].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(ADULT_PIG_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(ADULT_PIG_HEAD[1].tex, [16.0, 16.0]);
    assert_eq!(ADULT_PIG_BODY[0].tex, [28.0, 8.0]);
    assert_eq!(PIG_SADDLE_HEAD[0].min, [-4.5, -4.5, -8.5]);
    assert_eq!(PIG_SADDLE_HEAD[0].size, [9.0, 9.0, 9.0]);
    assert_eq!(PIG_SADDLE_HEAD[0].uv_size, [8.0, 8.0, 8.0]);
    assert_eq!(PIG_SADDLE_BODY[0].min, [-5.5, -10.5, -7.5]);
    assert_eq!(PIG_SADDLE_BODY[0].size, [11.0, 17.0, 9.0]);
    assert_eq!(PIG_SADDLE_BODY[0].uv_size, [10.0, 16.0, 8.0]);
    assert_eq!(PIG_SADDLE_LEG[0].min, [-2.5, -0.5, -2.5]);
    assert_eq!(PIG_SADDLE_LEG[0].size, [5.0, 7.0, 5.0]);
    assert_eq!(PIG_SADDLE_LEG[0].uv_size, [4.0, 6.0, 4.0]);
    // The cold body inflates the geometry while the UV box stays the base size.
    assert_eq!(COLD_PIG_BODY[1].size, [11.0, 17.0, 9.0]);
    assert_eq!(COLD_PIG_BODY[1].uv_size, [10.0, 16.0, 8.0]);
    assert_eq!(COLD_PIG_BODY[1].tex, [28.0, 32.0]);
    assert_eq!(BABY_PIG_HEAD[0].uv_size, [7.0, 6.0, 6.0]);
    assert_eq!(BABY_PIG_HEAD[0].tex, [0.0, 15.0]);
    assert_eq!(BABY_PIG_HEAD[1].tex, [6.0, 27.0]);
    assert_eq!(BABY_PIG_RIGHT_FRONT_LEG[0].tex, [23.0, 0.0]);
    assert_eq!(BABY_PIG_RIGHT_HIND_LEG[0].tex, [23.0, 4.0]);
}

#[test]
fn entity_texture_atlas_stitches_official_pig_png_slots() {
    let images = pig_texture_images();

    let (layout, rgba) = build_entity_model_texture_atlas(&images).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 288);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/pig/pig_temperate.png",
            "textures/entity/pig/pig_temperate_baby.png",
            "textures/entity/pig/pig_warm.png",
            "textures/entity/pig/pig_warm_baby.png",
            "textures/entity/pig/pig_cold.png",
            "textures/entity/pig/pig_cold_baby.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 64.0 / 288.0]);
    assert_close2(layout.entries[1].uv.min, [0.0, 64.0 / 288.0]);
    assert_close2(layout.entries[1].uv.max, [0.5, 96.0 / 288.0]);
    assert_close2(layout.entries[4].uv.min, [0.0, 192.0 / 288.0]);
    assert_close2(layout.entries[4].uv.max, [1.0, 256.0 / 288.0]);
    let warm_baby_first_pixel = rgba_offset(layout.width, 160, 0, "test").unwrap();
    assert_eq!(
        &rgba[warm_baby_first_pixel..warm_baby_first_pixel + 4],
        &[3; 4]
    );
    let cold_first_pixel = rgba_offset(layout.width, 192, 0, "test").unwrap();
    assert_eq!(&rgba[cold_first_pixel..cold_first_pixel + 4], &[4; 4]);
}

#[test]
fn pig_textured_mesh_uses_vanilla_uvs_tints_and_variant_textures() {
    let (atlas, _) = build_entity_model_texture_atlas(&pig_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[
            EntityModelInstance::pig(
                501,
                [0.0, 64.0, 0.0],
                0.0,
                PigModelVariant::Temperate,
                false,
            ),
            EntityModelInstance::pig(502, [1.0, 64.0, 0.0], 0.0, PigModelVariant::Cold, false),
            EntityModelInstance::pig(503, [2.0, 64.0, 0.0], 0.0, PigModelVariant::Warm, true),
        ],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 132);
    assert_eq!(mesh.vertices.len(), 528);
    assert_eq!(mesh.indices.len(), 792);
    assert_close2(mesh.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[168].uv, [16.0 / 64.0, 192.0 / 288.0]);
    assert_eq!(mesh.vertices[168].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[360].uv, [16.0 / 64.0, 160.0 / 288.0]);
    assert_eq!(mesh.vertices[360].tint, [1.0, 1.0, 1.0, 1.0]);
    let (min, max) = textured_mesh_extents(&mesh);
    assert!(max[0] - min[0] > 2.0);
    assert_close3([min[1], max[1], max[2] - min[2]], [64.001, 65.001, 1.5]);
}

#[test]
fn pig_saddle_layer_renders_for_adults_only() {
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        PIG_TEMPERATE_TEXTURE_REF,
        PIG_SADDLE_TEXTURE_REF,
        PIG_TEMPERATE_BABY_TEXTURE_REF,
    ]))
    .unwrap();
    let bare = entity_model_textured_mesh(
        &[EntityModelInstance::pig(
            521,
            [0.0, 64.0, 0.0],
            0.0,
            PigModelVariant::Temperate,
            false,
        )],
        &atlas,
    );
    let saddled = entity_model_textured_mesh(
        &[EntityModelInstance::pig(
            522,
            [0.0, 64.0, 0.0],
            0.0,
            PigModelVariant::Temperate,
            false,
        )
        .with_pig_saddle(true)],
        &atlas,
    );
    assert_eq!(saddled.cutout_faces - bare.cutout_faces, 42);
    assert_eq!(saddled.vertices.len() - bare.vertices.len(), 168);
    assert_close2(saddled.vertices[168].uv, [16.0 / 64.0, 64.0 / 160.0]);
    let (bare_min, bare_max) = textured_mesh_extents(&bare);
    let (saddle_min, saddle_max) = textured_mesh_extents(&saddled);
    assert!(saddle_min[0] < bare_min[0]);
    assert!(saddle_max[0] > bare_max[0]);

    let baby = entity_model_textured_mesh(
        &[
            EntityModelInstance::pig(523, [0.0, 64.0, 0.0], 0.0, PigModelVariant::Temperate, true)
                .with_pig_saddle(true),
        ],
        &atlas,
    );
    assert_eq!(baby.cutout_faces, 42);
    assert_eq!(baby.vertices.len(), 168);
}

#[test]
fn pig_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&pig_texture_images()).unwrap();
    let base = EntityModelInstance::pig(
        504,
        [0.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Temperate,
        false,
    );
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let yawed = entity_model_textured_mesh(&[base.with_head_look(55.0, 0.0)], &atlas);
    let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -25.0)], &atlas);

    // Head look turns the textured head part without adding or dropping vertices.
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_ne!(resting.vertices, yawed.vertices);
    assert_ne!(resting.vertices, pitched.vertices);
    assert_ne!(yawed.vertices, pitched.vertices);
}

#[test]
fn pig_colored_mesh_swings_its_legs_when_walking() {
    // PigModel extends QuadrupedModel without overriding setupAnim, so the pig legs
    // swing with the same `cos(pos * 0.6662 [+ π]) * 1.4 * speed` rotation. A
    // standing pig is byte-identical with or without a swing position, and a walking
    // pig's feet lift (its lowest point rises). The colored path is the test render;
    // pigs are texture-backed, but `entity_model_mesh` emits the colored mesh.
    for baby in [false, true] {
        let base =
            EntityModelInstance::pig(510, [0.0, 64.0, 0.0], 0.0, PigModelVariant::Cold, baby);
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(rest.vertices, still.vertices, "baby={baby}: rest is inert");

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(
            rest.vertices, walking.vertices,
            "baby={baby}: walking differs"
        );

        let (rest_min, rest_max) = mesh_extents(&rest);
        let (walk_min, walk_max) = mesh_extents(&walking);
        // The antiphase swing both lifts the feet (a shorter model) and splays the
        // legs forward/back (a deeper footprint). Babies are scaled to half size with
        // tiny legs, so the margin is kept small.
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "baby={baby}: walking pig's feet should lift off the ground"
        );
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
            "baby={baby}: walking pig's legs should splay along Z"
        );
    }
}

#[test]
fn pig_textured_mesh_swings_its_legs_when_walking() {
    // The real pig render path (textured) consumes the projected limb swing through
    // the shared QuadrupedModel leg rotation: a standing pig is inert, a walking
    // pig keeps its vertex count but lifts its feet, for every variant and the baby.
    let (atlas, _) = build_entity_model_texture_atlas(&pig_texture_images()).unwrap();
    for (variant, baby) in [
        (PigModelVariant::Temperate, false),
        (PigModelVariant::Warm, false),
        (PigModelVariant::Cold, false),
        (PigModelVariant::Temperate, true),
    ] {
        let base = EntityModelInstance::pig(511, [0.0, 64.0, 0.0], 0.0, variant, baby);
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
        let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

        assert_eq!(
            resting.vertices, still.vertices,
            "{variant:?} baby={baby}: a standing pig is inert"
        );
        assert_eq!(
            resting.vertices.len(),
            walking.vertices.len(),
            "{variant:?} baby={baby}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.vertices, walking.vertices,
            "{variant:?} baby={baby}: a walking pig differs"
        );

        let (rest_min, rest_max) = textured_mesh_extents(&resting);
        let (walk_min, walk_max) = textured_mesh_extents(&walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "{variant:?} baby={baby}: a walking pig's feet should lift off the ground"
        );
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
            "{variant:?} baby={baby}: a walking pig's legs should splay along Z"
        );
    }
}

fn pig_texture_images() -> Vec<EntityModelTextureImage> {
    texture_images(pig_entity_texture_refs())
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
