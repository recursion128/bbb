use super::*;

#[test]
fn spider_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        SPIDER_HEAD[0],
        ModelCubeDesc {
            min: [-4.0, -4.0, -8.0],
            size: [8.0, 8.0, 8.0],
            color: SPIDER_DARK,
        }
    );
    assert_eq!(
        SPIDER_BODY_0[0],
        ModelCubeDesc {
            min: [-3.0, -3.0, -3.0],
            size: [6.0, 6.0, 6.0],
            color: SPIDER_DARK,
        }
    );
    assert_eq!(
        SPIDER_BODY_1[0],
        ModelCubeDesc {
            min: [-5.0, -4.0, -6.0],
            size: [10.0, 8.0, 12.0],
            color: SPIDER_DARK,
        }
    );
    assert_eq!(
        SPIDER_RIGHT_LEG[0],
        ModelCubeDesc {
            min: [-15.0, -1.0, -1.0],
            size: [16.0, 2.0, 2.0],
            color: SPIDER_DARK,
        }
    );
    assert_eq!(
        SPIDER_LEFT_LEG[0],
        ModelCubeDesc {
            min: [-1.0, -1.0, -1.0],
            size: [16.0, 2.0, 2.0],
            color: SPIDER_DARK,
        }
    );

    assert_eq!(SPIDER_PARTS.len(), 11);
    assert_part(
        &SPIDER_PARTS[0],
        [0.0, 15.0, -3.0],
        [0.0, 0.0, 0.0],
        SPIDER_HEAD.as_slice(),
    );
    assert_part(
        &SPIDER_PARTS[1],
        [0.0, 15.0, 0.0],
        [0.0, 0.0, 0.0],
        SPIDER_BODY_0.as_slice(),
    );
    assert_part(
        &SPIDER_PARTS[2],
        [0.0, 15.0, 9.0],
        [0.0, 0.0, 0.0],
        SPIDER_BODY_1.as_slice(),
    );

    let leg_specs = [
        (
            [-4.0, 15.0, 2.0],
            [
                0.0,
                std::f32::consts::FRAC_PI_4,
                -std::f32::consts::FRAC_PI_4,
            ],
            SPIDER_RIGHT_LEG.as_slice(),
        ),
        (
            [4.0, 15.0, 2.0],
            [
                0.0,
                -std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_4,
            ],
            SPIDER_LEFT_LEG.as_slice(),
        ),
        (
            [-4.0, 15.0, 1.0],
            [0.0, std::f32::consts::FRAC_PI_8, -0.58119464],
            SPIDER_RIGHT_LEG.as_slice(),
        ),
        (
            [4.0, 15.0, 1.0],
            [0.0, -std::f32::consts::FRAC_PI_8, 0.58119464],
            SPIDER_LEFT_LEG.as_slice(),
        ),
        (
            [-4.0, 15.0, 0.0],
            [0.0, -std::f32::consts::FRAC_PI_8, -0.58119464],
            SPIDER_RIGHT_LEG.as_slice(),
        ),
        (
            [4.0, 15.0, 0.0],
            [0.0, std::f32::consts::FRAC_PI_8, 0.58119464],
            SPIDER_LEFT_LEG.as_slice(),
        ),
        (
            [-4.0, 15.0, -1.0],
            [
                0.0,
                -std::f32::consts::FRAC_PI_4,
                -std::f32::consts::FRAC_PI_4,
            ],
            SPIDER_RIGHT_LEG.as_slice(),
        ),
        (
            [4.0, 15.0, -1.0],
            [
                0.0,
                std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_4,
            ],
            SPIDER_LEFT_LEG.as_slice(),
        ),
    ];
    for (part, (offset, rotation, cubes)) in SPIDER_PARTS[3..].iter().zip(leg_specs) {
        assert_part(part, offset, rotation, cubes);
    }
}

#[test]
fn spider_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::spider(124, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-1.0282283, 64.0193, -0.9375]);
    assert_close3(max, [1.0282283, 64.8135, 0.7696068]);
}

#[test]
fn cave_spider_mesh_uses_vanilla_scaled_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::cave_spider(22, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.71976, 64.01351, -0.65625]);
    assert_close3(max, [0.71976, 64.56945, 0.5387248]);
}

#[test]
fn spider_texture_refs_match_vanilla_renderers() {
    assert_eq!(EntityModelKind::Spider.model_key(), "spider");
    assert_eq!(
        EntityModelKind::Spider.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/spider/spider.png",
            size: [64, 32],
        })
    );
    assert_eq!(EntityModelKind::CaveSpider.model_key(), "cave_spider");
    assert_eq!(
        EntityModelKind::CaveSpider.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/spider/cave_spider.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        spider_entity_texture_refs(),
        [
            EntityModelTextureRef {
                path: "textures/entity/spider/spider.png",
                size: [64, 32],
            },
            EntityModelTextureRef {
                path: "textures/entity/spider/cave_spider.png",
                size: [64, 32],
            },
            EntityModelTextureRef {
                path: "textures/entity/spider/spider_eyes.png",
                size: [64, 32],
            },
        ]
    );
    assert_eq!(
        EntityModelKind::Spider.vanilla_layer_texture_refs(),
        &[SPIDER_EYES_TEXTURE_REF]
    );
    assert_eq!(
        EntityModelKind::CaveSpider.vanilla_layer_texture_refs(),
        &[SPIDER_EYES_TEXTURE_REF]
    );
}

#[test]
fn spider_textured_layer_passes_match_vanilla_renderer_model_layers() {
    let spider = spider_textured_layer_passes(false);
    assert_eq!(spider.len(), 2);
    assert_eq!(spider[0].kind, EntityModelLayerKind::SpiderBase);
    assert_eq!(spider[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(spider[0].model_layer, MODEL_LAYER_SPIDER);
    assert_eq!(spider[0].texture, SPIDER_TEXTURE_REF);
    assert_eq!(spider[0].parts, SPIDER_TEXTURED_PARTS.as_slice());
    assert_eq!(spider[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (spider[0].collector_order, spider[0].submit_sequence),
        (0, 0)
    );
    assert_eq!(spider[1].kind, EntityModelLayerKind::SpiderEyes);
    assert_eq!(spider[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(spider[1].model_layer, MODEL_LAYER_SPIDER);
    assert_eq!(spider[1].texture, SPIDER_EYES_TEXTURE_REF);
    assert_eq!(spider[1].parts, SPIDER_TEXTURED_PARTS.as_slice());
    assert_eq!(spider[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (spider[1].collector_order, spider[1].submit_sequence),
        (1, 1)
    );

    let cave = spider_textured_layer_passes(true);
    assert_eq!(cave.len(), 2);
    assert_eq!(cave[0].kind, EntityModelLayerKind::SpiderBase);
    assert_eq!(cave[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(cave[0].model_layer, MODEL_LAYER_CAVE_SPIDER);
    assert_eq!(cave[0].texture, CAVE_SPIDER_TEXTURE_REF);
    assert_eq!(cave[0].parts, SPIDER_TEXTURED_PARTS.as_slice());
    assert_eq!(cave[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((cave[0].collector_order, cave[0].submit_sequence), (0, 0));
    assert_eq!(cave[1].kind, EntityModelLayerKind::SpiderEyes);
    assert_eq!(cave[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(cave[1].model_layer, MODEL_LAYER_CAVE_SPIDER);
    assert_eq!(cave[1].texture, SPIDER_EYES_TEXTURE_REF);
    assert_eq!(cave[1].parts, SPIDER_TEXTURED_PARTS.as_slice());
    assert_eq!(cave[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((cave[1].collector_order, cave[1].submit_sequence), (1, 1));
}

#[test]
fn spider_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_SPIDER, "minecraft:spider#main");
    assert_eq!(MODEL_LAYER_CAVE_SPIDER, "minecraft:cave_spider#main");
    assert_eq!(
        SPIDER_EYES_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/spider/spider_eyes.png",
            size: [64, 32],
        }
    );
    assert_eq!(SPIDER_TEXTURED_PARTS.len(), 11);
    assert_eq!(
        SPIDER_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-4.0, -4.0, -8.0],
            size: [8.0, 8.0, 8.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [32.0, 4.0],
            mirror: false,
        }
    );
    assert_eq!(
        SPIDER_TEXTURED_BODY_0[0],
        TexturedModelCubeDesc {
            min: [-3.0, -3.0, -3.0],
            size: [6.0, 6.0, 6.0],
            uv_size: [6.0, 6.0, 6.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        SPIDER_TEXTURED_BODY_1[0],
        TexturedModelCubeDesc {
            min: [-5.0, -4.0, -6.0],
            size: [10.0, 8.0, 12.0],
            uv_size: [10.0, 8.0, 12.0],
            tex: [0.0, 12.0],
            mirror: false,
        }
    );
    assert_eq!(
        SPIDER_TEXTURED_RIGHT_LEG[0],
        TexturedModelCubeDesc {
            min: [-15.0, -1.0, -1.0],
            size: [16.0, 2.0, 2.0],
            uv_size: [16.0, 2.0, 2.0],
            tex: [18.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        SPIDER_TEXTURED_LEFT_LEG[0],
        TexturedModelCubeDesc {
            min: [-1.0, -1.0, -1.0],
            size: [16.0, 2.0, 2.0],
            uv_size: [16.0, 2.0, 2.0],
            tex: [18.0, 0.0],
            mirror: true,
        }
    );
    assert_eq!(SPIDER_TEXTURED_PARTS[0].pose, SPIDER_PARTS[0].pose);
    assert_eq!(SPIDER_TEXTURED_PARTS[3].pose, SPIDER_PARTS[3].pose);
    assert_eq!(SPIDER_TEXTURED_PARTS[10].pose, SPIDER_PARTS[10].pose);
}

#[test]
fn entity_texture_atlas_stitches_official_spider_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&spider_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 96);
    assert_eq!(layout.entries.len(), 3);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/spider/spider.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 1.0 / 3.0]);
    assert_eq!(
        layout.entries[1].texture.path,
        "textures/entity/spider/cave_spider.png"
    );
    assert_close2(layout.entries[1].uv.min, [0.0, 1.0 / 3.0]);
    assert_close2(layout.entries[1].uv.max, [1.0, 2.0 / 3.0]);
    assert_eq!(
        layout.entries[2].texture.path,
        "textures/entity/spider/spider_eyes.png"
    );
    assert_close2(layout.entries[2].uv.min, [0.0, 2.0 / 3.0]);
    assert_close2(layout.entries[2].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let cave_start = rgba_offset(layout.width, 32, 0, "cave spider atlas row").unwrap();
    assert_eq!(&rgba[cave_start..cave_start + 4], &[1; 4]);
    let eyes_start = rgba_offset(layout.width, 64, 0, "spider eyes atlas row").unwrap();
    assert_eq!(&rgba[eyes_start..eyes_start + 4], &[2; 4]);
}

#[test]
fn spider_textured_mesh_uses_vanilla_uvs_tints_and_cave_scale() {
    let (atlas, _) = build_entity_model_texture_atlas(&spider_texture_images()).unwrap();

    let spider = entity_model_textured_mesh(
        &[EntityModelInstance::spider(912, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert_eq!(spider.cutout_faces, 66);
    assert_eq!(spider.vertices.len(), 264);
    assert_eq!(spider.indices.len(), 396);
    assert_close2(spider.vertices[0].uv, [48.0 / 64.0, 4.0 / 96.0]);
    assert!(spider
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(&spider);
    assert_close3(min, [-1.0282283, 64.0193, -0.9375]);
    assert_close3(max, [1.0282283, 64.8135, 0.7696068]);

    let cave = entity_model_textured_mesh(
        &[EntityModelInstance::cave_spider(913, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert_eq!(cave.cutout_faces, 66);
    assert_eq!(cave.vertices.len(), 264);
    assert_eq!(cave.indices.len(), 396);
    assert_close2(cave.vertices[0].uv, [48.0 / 64.0, 36.0 / 96.0]);
    assert!(cave
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(&cave);
    assert_close3(min, [-0.71976, 64.01351, -0.65625]);
    assert_close3(max, [0.71976, 64.56945, 0.5387248]);
}

#[test]
fn spider_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&spider_texture_images()).unwrap();
    for base in [
        EntityModelInstance::spider(914, [0.0, 64.0, 0.0], 0.0),
        EntityModelInstance::cave_spider(915, [0.0, 64.0, 0.0], 0.0),
    ] {
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let yawed = entity_model_textured_mesh(&[base.with_head_look(45.0, 0.0)], &atlas);
        let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -20.0)], &atlas);
        assert_eq!(resting.vertices.len(), yawed.vertices.len());
        assert_ne!(resting.vertices, yawed.vertices, "{:?}", base.kind);
        assert_ne!(yawed.vertices, pitched.vertices, "{:?}", base.kind);
    }
}

#[test]
fn spider_eyes_textured_mesh_uses_parent_model_geometry_and_eyes_render_type() {
    let (atlas, _) = build_entity_model_texture_atlas(&spider_texture_images()).unwrap();

    let meshes = entity_model_textured_meshes(
        &[
            EntityModelInstance::spider(912, [0.0, 64.0, 0.0], 0.0),
            EntityModelInstance::cave_spider(913, [0.0, 64.0, 0.0], 0.0),
        ],
        &atlas,
    );

    assert_eq!(meshes.cutout.cutout_faces, 132);
    assert_eq!(meshes.eyes.cutout_faces, 132);
    assert_eq!(meshes.cutout.vertices.len(), 528);
    assert_eq!(meshes.eyes.vertices.len(), 528);
    assert_close2(meshes.eyes.vertices[0].uv, [48.0 / 64.0, 68.0 / 96.0]);
    assert!(meshes
        .eyes
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert_eq!(
        textured_mesh_extents(&meshes.eyes),
        textured_mesh_extents(&meshes.cutout)
    );
}
