use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn entity_texture_atlas_stitches_official_cow_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&cow_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 384);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/cow/cow_temperate.png",
            "textures/entity/cow/cow_temperate_baby.png",
            "textures/entity/cow/cow_warm.png",
            "textures/entity/cow/cow_warm_baby.png",
            "textures/entity/cow/cow_cold.png",
            "textures/entity/cow/cow_cold_baby.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 64.0 / 384.0]);
    assert_close2(layout.entries[3].uv.min, [0.0, 192.0 / 384.0]);
    assert_close2(layout.entries[3].uv.max, [1.0, 256.0 / 384.0]);
    assert_close2(layout.entries[4].uv.min, [0.0, 256.0 / 384.0]);
    assert_close2(layout.entries[4].uv.max, [1.0, 320.0 / 384.0]);
    let warm_baby_first_pixel = rgba_offset(layout.width, 192, 0, "test").unwrap();
    assert_eq!(
        &rgba[warm_baby_first_pixel..warm_baby_first_pixel + 4],
        &[3; 4]
    );
    let cold_first_pixel = rgba_offset(layout.width, 256, 0, "test").unwrap();
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
        .with_light_coords((6_u32 << 4) | (13_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true),
        EntityModelInstance::cow_variant(602, [1.0, 64.0, 0.0], 0.0, CowModelVariant::Cold, false)
            .with_light_coords((6_u32 << 4) | (13_u32 << 20))
            .with_white_overlay_progress(0.8)
            .with_has_red_overlay(true),
        EntityModelInstance::cow_variant(603, [2.0, 64.0, 0.0], 0.0, CowModelVariant::Warm, true)
            .with_light_coords((6_u32 << 4) | (13_u32 << 20))
            .with_white_overlay_progress(0.8)
            .with_has_red_overlay(true),
    ];
    let meshes = entity_model_textured_meshes(&instances, &atlas);
    assert_cow_submissions_match_vanilla(&meshes, &instances);
    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 180);
    assert_eq!(mesh.vertices.len(), 720);
    assert_eq!(mesh.indices.len(), 1080);
    assert_close2(mesh.vertices[0].uv, [14.0 / 64.0, 0.0]);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[240].uv, [14.0 / 64.0, 256.0 / 384.0]);
    assert_eq!(mesh.vertices[240].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[504].uv, [11.0 / 64.0, 210.0 / 384.0]);
    assert_eq!(mesh.vertices[504].tint, [1.0, 1.0, 1.0, 1.0]);
    assert!(mesh.vertices.iter().all(|vertex| vertex.light
        == instances[0].render_state.shader_light()
        && vertex.overlay == instances[0].render_state.overlay_coords()));
    assert_ne!(instances[0].render_state.overlay_coords(), [0.0, 10.0]);
    let (min, max) = textured_mesh_extents(mesh);
    assert_close3(min, [-0.375, 64.001, -0.65625]);
    assert_close3(max, [2.25, 65.5635, 1.0]);
}

#[test]
fn cow_adult_model_parts_match_vanilla_26_1_body_layer() {
    // The unified cubes carry both render paths' geometry: the colored debug tint and the textured
    // `uv_size`/`texOffs`/`mirror`.
    assert_eq!(
        ADULT_COW_HEAD,
        [
            ModelCube::new(
                [-4.0, -4.0, -6.0],
                [8.0, 8.0, 6.0],
                COW_BROWN,
                [8.0, 8.0, 6.0],
                [0.0, 0.0],
                false,
            ),
            ModelCube::new(
                [-3.0, 1.0, -7.0],
                [6.0, 3.0, 1.0],
                COW_BROWN,
                [6.0, 3.0, 1.0],
                [1.0, 33.0],
                false,
            ),
            ModelCube::new(
                [-5.0, -5.0, -5.0],
                [1.0, 3.0, 1.0],
                COW_BROWN,
                [1.0, 3.0, 1.0],
                [22.0, 0.0],
                false,
            ),
            ModelCube::new(
                [4.0, -5.0, -5.0],
                [1.0, 3.0, 1.0],
                COW_BROWN,
                [1.0, 3.0, 1.0],
                [22.0, 0.0],
                false,
            ),
        ]
    );
    assert_eq!(ADULT_COW_BODY[0].size, [12.0, 18.0, 10.0]);
    assert_eq!(ADULT_COW_RIGHT_LEG[0].size, [4.0, 12.0, 4.0]);
    assert!(!ADULT_COW_RIGHT_LEG[0].mirror);
    assert!(ADULT_COW_LEFT_LEG[0].mirror);
}

#[test]
fn cow_warm_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(WARM_COW_HEAD.len(), 6);
    assert_eq!(WARM_COW_HEAD[2].min, [-8.0, -3.0, -5.0]);
    assert_eq!(WARM_COW_HEAD[2].size, [4.0, 2.0, 2.0]);
    assert_eq!(WARM_COW_HEAD[2].color, COW_BROWN);
    assert!(!WARM_COW_HEAD[2].mirror);
    assert_eq!(WARM_COW_HEAD[4].min, [4.0, -3.0, -5.0]);
    assert!(WARM_COW_HEAD[4].mirror);
}

#[test]
fn cow_cold_adult_model_parts_match_vanilla_26_1_body_layer() {
    // The cold body's first cube inflates the geometry (colored 13×19×11) while the textured
    // `uv_size` keeps the base 12×18×10 (the squid body precedent).
    assert_eq!(
        COLD_COW_BODY,
        [
            ModelCube::new(
                [-6.5, -10.5, -7.5],
                [13.0, 19.0, 11.0],
                COW_COLD_FUR,
                [12.0, 18.0, 10.0],
                [20.0, 32.0],
                false,
            ),
            ModelCube::new(
                [-6.0, -10.0, -7.0],
                [12.0, 18.0, 10.0],
                COW_BROWN,
                [12.0, 18.0, 10.0],
                [18.0, 4.0],
                false,
            ),
            ModelCube::new(
                [-2.0, 2.0, -8.0],
                [4.0, 6.0, 1.0],
                COW_BROWN,
                [4.0, 6.0, 1.0],
                [52.0, 0.0],
                false,
            ),
        ]
    );
    assert_eq!(COLD_COW_RIGHT_HORN_POSE.offset, [-4.5, -2.5, -3.5]);
    assert_eq!(COLD_COW_RIGHT_HORN_POSE.rotation, [1.5708, 0.0, 0.0]);
    assert_eq!(COLD_COW_LEFT_HORN_POSE.offset, [5.5, -2.5, -5.0]);
    assert_eq!(COLD_COW_RIGHT_HORN[0].color, COW_COLD_FUR);
    assert_eq!(COLD_COW_LEFT_HORN[0].color, COW_COLD_FUR);
}

#[test]
fn cow_adult_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::cow(92, [0.0, 64.0, 0.0], 0.0, false)]);

    assert_eq!(mesh.opaque_faces, 60);
    assert_eq!(mesh.vertices.len(), 240);
    assert_eq!(mesh.indices.len(), 360);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.375, 64.001, -0.625]);
    assert_close3(max, [0.375, 65.5635, 0.9375]);
}

#[test]
fn cow_warm_adult_model_mesh_uses_vanilla_warm_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::cow_variant(
        94,
        [0.0, 64.0, 0.0],
        0.0,
        CowModelVariant::Warm,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 72);
    assert_eq!(mesh.vertices.len(), 288);
    assert_eq!(mesh.indices.len(), 432);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.5, 64.001, -0.625]);
    assert_close3(max, [0.5, 65.5635, 0.9375]);
}

#[test]
fn cow_cold_adult_model_mesh_uses_vanilla_cold_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::cow_variant(
        95,
        [0.0, 64.0, 0.0],
        0.0,
        CowModelVariant::Cold,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.40625, 64.001, -0.65625]);
    assert_close3(max, [0.40625, 65.501, 1.0]);
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(COW_COLD_FUR, 0.78)));
}

#[test]
fn cow_baby_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        BABY_COW_HEAD,
        [
            ModelCube::new(
                [-3.0, -4.569, -4.8333],
                [6.0, 6.0, 5.0],
                COW_BROWN,
                [6.0, 6.0, 5.0],
                [0.0, 18.0],
                false,
            ),
            ModelCube::new(
                [3.0, -5.569, -3.8333],
                [1.0, 2.0, 1.0],
                COW_BROWN,
                [1.0, 2.0, 1.0],
                [8.0, 29.0],
                false,
            ),
            ModelCube::new(
                [-4.0, -5.569, -3.8333],
                [1.0, 2.0, 1.0],
                COW_BROWN,
                [1.0, 2.0, 1.0],
                [4.0, 29.0],
                true,
            ),
            ModelCube::new(
                [-2.0, -1.569, -5.8333],
                [4.0, 3.0, 1.0],
                COW_BROWN,
                [4.0, 3.0, 1.0],
                [12.0, 29.0],
                false,
            ),
        ]
    );
    assert_eq!(BABY_COW_BODY[0].size, [8.0, 6.0, 12.0]);
    // The baby legs share one geometry but distinct per-corner UV origins.
    assert_eq!(BABY_COW_RIGHT_FRONT_LEG[0].tex, [22.0, 18.0]);
    assert_eq!(BABY_COW_LEFT_FRONT_LEG[0].tex, [34.0, 18.0]);
    assert_eq!(BABY_COW_RIGHT_HIND_LEG[0].tex, [22.0, 27.0]);
    assert_eq!(BABY_COW_LEFT_HIND_LEG[0].tex, [34.0, 27.0]);
}

#[test]
fn cow_baby_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::cow(93, [0.0, 64.0, 0.0], 0.0, true)]);

    assert_eq!(mesh.opaque_faces, 54);
    assert_eq!(mesh.vertices.len(), 216);
    assert_eq!(mesh.indices.len(), 324);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.375]);
    assert_close3(max, [0.25, 65.001, 0.6875]);
}

#[test]
fn cow_texture_refs_match_vanilla_renderers() {
    let cow_cases = [
        (
            CowModelVariant::Temperate,
            false,
            "cow_temperate",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_temperate.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Temperate,
            true,
            "cow_temperate_baby",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_temperate_baby.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Warm,
            false,
            "cow_warm",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_warm.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Warm,
            true,
            "cow_warm_baby",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_warm_baby.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Cold,
            false,
            "cow_cold",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_cold.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Cold,
            true,
            "cow_cold_baby",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_cold_baby.png",
                size: [64, 64],
            },
        ),
    ];
    for (variant, baby, model_key, texture) in cow_cases {
        let kind = EntityModelKind::Cow { variant, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn cow_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let temperate = cow_textured_layer_passes(CowModelVariant::Temperate, false);
    assert_eq!(temperate.len(), 1);
    assert_eq!(temperate[0].kind, EntityModelLayerKind::CowBase);
    assert_eq!(
        temperate[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(temperate[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(temperate[0].model_layer, MODEL_LAYER_COW);
    assert_eq!(temperate[0].texture, COW_TEMPERATE_TEXTURE_REF);
    assert_eq!(temperate[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((temperate[0].order, temperate[0].submit_sequence), (0, 0));

    let temperate_baby = cow_textured_layer_passes(CowModelVariant::Temperate, true);
    assert_eq!(temperate_baby[0].model_layer, MODEL_LAYER_COW_BABY);
    assert_eq!(temperate_baby[0].texture, COW_TEMPERATE_BABY_TEXTURE_REF);

    let warm = cow_textured_layer_passes(CowModelVariant::Warm, false);
    assert_eq!(warm[0].model_layer, MODEL_LAYER_WARM_COW);
    assert_eq!(warm[0].texture, COW_WARM_TEXTURE_REF);

    let warm_baby = cow_textured_layer_passes(CowModelVariant::Warm, true);
    assert_eq!(warm_baby[0].model_layer, MODEL_LAYER_WARM_COW_BABY);
    assert_eq!(warm_baby[0].texture, COW_WARM_BABY_TEXTURE_REF);

    let cold = cow_textured_layer_passes(CowModelVariant::Cold, false);
    assert_eq!(cold[0].model_layer, MODEL_LAYER_COLD_COW);
    assert_eq!(cold[0].texture, COW_COLD_TEXTURE_REF);

    let cold_baby = cow_textured_layer_passes(CowModelVariant::Cold, true);
    assert_eq!(cold_baby[0].model_layer, MODEL_LAYER_COLD_COW_BABY);
    assert_eq!(cold_baby[0].texture, COW_COLD_BABY_TEXTURE_REF);
}

#[test]
fn cow_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    // The textured UV sources now live on the unified cubes (`uv_size`/`tex`/`mirror`).
    assert_eq!(MODEL_LAYER_COW, "minecraft:cow#main");
    assert_eq!(MODEL_LAYER_COW_BABY, "minecraft:cow_baby#main");
    assert_eq!(MODEL_LAYER_WARM_COW, "minecraft:warm_cow#main");
    assert_eq!(MODEL_LAYER_WARM_COW_BABY, "minecraft:warm_cow_baby#main");
    assert_eq!(MODEL_LAYER_COLD_COW, "minecraft:cold_cow#main");
    assert_eq!(MODEL_LAYER_COLD_COW_BABY, "minecraft:cold_cow_baby#main");
    assert_eq!(ADULT_COW_HEAD[0].uv_size, [8.0, 8.0, 6.0]);
    assert_eq!(ADULT_COW_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(ADULT_COW_BODY[1].tex, [52.0, 0.0]);
    assert!(!ADULT_COW_BODY[1].mirror);
    assert_eq!(WARM_COW_HEAD[4].tex, [27.0, 0.0]);
    assert!(WARM_COW_HEAD[4].mirror);
    // The cold body inflates the geometry while the UV box stays the base size.
    assert_eq!(COLD_COW_BODY[0].size, [13.0, 19.0, 11.0]);
    assert_eq!(COLD_COW_BODY[0].uv_size, [12.0, 18.0, 10.0]);
    assert_eq!(COLD_COW_BODY[0].tex, [20.0, 32.0]);
    assert_eq!(COLD_COW_RIGHT_HORN[0].tex, [0.0, 40.0]);
    assert_eq!(COLD_COW_LEFT_HORN[0].tex, [0.0, 32.0]);
    assert_eq!(BABY_COW_HEAD[2].tex, [4.0, 29.0]);
    assert!(BABY_COW_HEAD[2].mirror);
    assert_eq!(BABY_COW_LEFT_HIND_LEG[0].tex, [34.0, 27.0]);
    assert!(!BABY_COW_LEFT_HIND_LEG[0].mirror);
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
    let yawed_instance = base.with_head_look(45.0, 0.0);
    let pitched_instance = base.with_head_look(0.0, -25.0);
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
    let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
    assert_cow_submissions_match_vanilla(&resting, &[base]);
    assert_cow_submissions_match_vanilla(&yawed, &[yawed_instance]);
    assert_cow_submissions_match_vanilla(&pitched, &[pitched_instance]);

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
        let still_instance = base.with_walk_animation(2.5, 0.0);
        let walking_instance = base.with_walk_animation(0.0, 1.0);
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_cow_submissions_match_vanilla(&resting, &[base]);
        assert_cow_submissions_match_vanilla(&still, &[still_instance]);
        assert_cow_submissions_match_vanilla(&walking, &[walking_instance]);

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

fn assert_cow_submissions_match_vanilla(
    meshes: &EntityModelTexturedMeshes,
    instances: &[EntityModelInstance],
) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.submissions.len(), instances.len());

    for (submit, instance) in meshes.submissions.iter().zip(instances) {
        let instance = *instance;
        assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
        assert_eq!(submit.texture, instance.kind.vanilla_texture_ref().unwrap());
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(submit.transform, entity_model_root_transform(instance));
        assert_eq!((submit.order, submit.submit_sequence), (0, 0));
        assert_eq!(submit.light, instance.render_state.shader_light());
        assert_eq!(submit.overlay, instance.render_state.overlay_coords());
    }
}

fn cow_texture_images() -> Vec<EntityModelTextureImage> {
    cow_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
