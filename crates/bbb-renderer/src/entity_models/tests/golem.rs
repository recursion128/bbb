use super::*;

#[test]
fn iron_golem_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        IRON_GOLEM_HEAD,
        [
            ModelCubeDesc {
                min: [-4.0, -12.0, -5.5],
                size: [8.0, 10.0, 8.0],
                color: IRON_GOLEM_STONE,
            },
            ModelCubeDesc {
                min: [-1.0, -5.0, -7.5],
                size: [2.0, 4.0, 2.0],
                color: IRON_GOLEM_STONE,
            },
        ]
    );
    assert_eq!(
        IRON_GOLEM_BODY,
        [
            ModelCubeDesc {
                min: [-9.0, -2.0, -6.0],
                size: [18.0, 12.0, 11.0],
                color: IRON_GOLEM_STONE,
            },
            ModelCubeDesc {
                min: [-5.0, 9.5, -3.5],
                size: [10.0, 6.0, 7.0],
                color: IRON_GOLEM_STONE,
            },
        ]
    );
    assert_eq!(
        IRON_GOLEM_RIGHT_ARM[0],
        ModelCubeDesc {
            min: [-13.0, -2.5, -3.0],
            size: [4.0, 30.0, 6.0],
            color: IRON_GOLEM_STONE,
        }
    );
    assert_eq!(
        IRON_GOLEM_LEFT_ARM[0],
        ModelCubeDesc {
            min: [9.0, -2.5, -3.0],
            size: [4.0, 30.0, 6.0],
            color: IRON_GOLEM_STONE,
        }
    );
    assert_eq!(
        IRON_GOLEM_RIGHT_LEG[0],
        ModelCubeDesc {
            min: [-3.5, -3.0, -3.0],
            size: [6.0, 16.0, 5.0],
            color: IRON_GOLEM_STONE,
        }
    );
    assert_eq!(IRON_GOLEM_LEFT_LEG, IRON_GOLEM_RIGHT_LEG);

    assert_eq!(IRON_GOLEM_PARTS.len(), 6);
    let part_specs = [
        ([0.0, -7.0, -2.0], IRON_GOLEM_HEAD.as_slice()),
        ([0.0, -7.0, 0.0], IRON_GOLEM_BODY.as_slice()),
        ([0.0, -7.0, 0.0], IRON_GOLEM_RIGHT_ARM.as_slice()),
        ([0.0, -7.0, 0.0], IRON_GOLEM_LEFT_ARM.as_slice()),
        ([-4.0, 11.0, 0.0], IRON_GOLEM_RIGHT_LEG.as_slice()),
        ([5.0, 11.0, 0.0], IRON_GOLEM_LEFT_LEG.as_slice()),
    ];
    for (part, (offset, cubes)) in IRON_GOLEM_PARTS.iter().zip(part_specs) {
        assert_part(part, offset, [0.0, 0.0, 0.0], cubes);
    }
}

#[test]
fn iron_golem_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::iron_golem(70, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 48);
    assert_eq!(mesh.vertices.len(), 192);
    assert_eq!(mesh.indices.len(), 288);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.8125, 64.001, -0.3125]);
    assert_close3(max, [0.8125, 66.6885, 0.59375]);
}

#[test]
fn iron_golem_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::IronGolem.model_key(), "iron_golem");
    assert_eq!(
        EntityModelKind::IronGolem.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/iron_golem/iron_golem.png",
            size: [128, 128],
        })
    );
}

#[test]
fn iron_golem_textured_layer_pass_matches_vanilla_renderer_model_layer() {
    let passes = iron_golem_textured_layer_passes();

    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::IronGolemBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_IRON_GOLEM);
    assert_eq!(passes[0].texture, IRON_GOLEM_TEXTURE_REF);
    assert_eq!(passes[0].parts, IRON_GOLEM_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
}

#[test]
fn iron_golem_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_IRON_GOLEM, "minecraft:iron_golem#main");
    assert_eq!(IRON_GOLEM_TEXTURE_REF.size, [128, 128]);
    assert_eq!(IRON_GOLEM_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        IRON_GOLEM_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-4.0, -12.0, -5.5],
            size: [8.0, 10.0, 8.0],
            uv_size: [8.0, 10.0, 8.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        IRON_GOLEM_TEXTURED_BODY[1],
        TexturedModelCubeDesc {
            min: [-5.0, 9.5, -3.5],
            size: [10.0, 6.0, 7.0],
            uv_size: [9.0, 5.0, 6.0],
            tex: [0.0, 70.0],
            mirror: false,
        }
    );
    assert_eq!(
        IRON_GOLEM_TEXTURED_LEFT_LEG[0],
        TexturedModelCubeDesc {
            min: [-3.5, -3.0, -3.0],
            size: [6.0, 16.0, 5.0],
            uv_size: [6.0, 16.0, 5.0],
            tex: [60.0, 0.0],
            mirror: true,
        }
    );
    assert_eq!(IRON_GOLEM_TEXTURED_PARTS[0].pose, IRON_GOLEM_PARTS[0].pose);
    assert_eq!(IRON_GOLEM_TEXTURED_PARTS[5].pose, IRON_GOLEM_PARTS[5].pose);
}

#[test]
fn snow_golem_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        SNOW_GOLEM_HEAD[0],
        ModelCubeDesc {
            min: [-3.5, -7.5, -3.5],
            size: [7.0, 7.0, 7.0],
            color: SNOW_GOLEM_WHITE,
        }
    );
    assert_eq!(
        SNOW_GOLEM_ARM[0],
        ModelCubeDesc {
            min: [-0.5, 0.5, -0.5],
            size: [11.0, 1.0, 1.0],
            color: SNOW_GOLEM_WHITE,
        }
    );
    assert_eq!(
        SNOW_GOLEM_UPPER_BODY[0],
        ModelCubeDesc {
            min: [-4.5, -9.5, -4.5],
            size: [9.0, 9.0, 9.0],
            color: SNOW_GOLEM_WHITE,
        }
    );
    assert_eq!(
        SNOW_GOLEM_LOWER_BODY[0],
        ModelCubeDesc {
            min: [-5.5, -11.5, -5.5],
            size: [11.0, 11.0, 11.0],
            color: SNOW_GOLEM_WHITE,
        }
    );

    assert_eq!(SNOW_GOLEM_PARTS.len(), 5);
    let part_specs = [
        ([0.0, 4.0, 0.0], [0.0, 0.0, 0.0], SNOW_GOLEM_HEAD.as_slice()),
        ([5.0, 6.0, 1.0], [0.0, 0.0, 1.0], SNOW_GOLEM_ARM.as_slice()),
        (
            [-5.0, 6.0, -1.0],
            [0.0, std::f32::consts::PI, -1.0],
            SNOW_GOLEM_ARM.as_slice(),
        ),
        (
            [0.0, 13.0, 0.0],
            [0.0, 0.0, 0.0],
            SNOW_GOLEM_UPPER_BODY.as_slice(),
        ),
        (
            [0.0, 24.0, 0.0],
            [0.0, 0.0, 0.0],
            SNOW_GOLEM_LOWER_BODY.as_slice(),
        ),
    ];
    for (part, (offset, rotation, cubes)) in SNOW_GOLEM_PARTS.iter().zip(part_specs) {
        assert_part(part, offset, rotation, cubes);
    }
}

#[test]
fn snow_golem_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 30);
    assert_eq!(mesh.vertices.len(), 120);
    assert_eq!(mesh.indices.len(), 180);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.6407774, 64.03225, -0.34375]);
    assert_close3(max, [0.6407774, 65.71975, 0.34375]);
}

#[test]
fn snow_golem_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::SnowGolem.model_key(), "snow_golem");
    assert_eq!(
        EntityModelKind::SnowGolem.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/snow_golem/snow_golem.png",
            size: [64, 64],
        })
    );
}

#[test]
fn snow_golem_textured_layer_pass_matches_vanilla_renderer_model_layer() {
    let passes = snow_golem_textured_layer_passes();

    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::SnowGolemBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_SNOW_GOLEM);
    assert_eq!(passes[0].texture, SNOW_GOLEM_TEXTURE_REF);
    assert_eq!(passes[0].parts, SNOW_GOLEM_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
}

#[test]
fn snow_golem_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_SNOW_GOLEM, "minecraft:snow_golem#main");
    assert_eq!(SNOW_GOLEM_TEXTURE_REF.size, [64, 64]);
    assert_eq!(SNOW_GOLEM_TEXTURED_PARTS.len(), 5);
    assert_eq!(
        SNOW_GOLEM_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-3.5, -7.5, -3.5],
            size: [7.0, 7.0, 7.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        SNOW_GOLEM_TEXTURED_ARM[0],
        TexturedModelCubeDesc {
            min: [-0.5, 0.5, -0.5],
            size: [11.0, 1.0, 1.0],
            uv_size: [12.0, 2.0, 2.0],
            tex: [32.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        SNOW_GOLEM_TEXTURED_UPPER_BODY[0],
        TexturedModelCubeDesc {
            min: [-4.5, -9.5, -4.5],
            size: [9.0, 9.0, 9.0],
            uv_size: [10.0, 10.0, 10.0],
            tex: [0.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(
        SNOW_GOLEM_TEXTURED_LOWER_BODY[0],
        TexturedModelCubeDesc {
            min: [-5.5, -11.5, -5.5],
            size: [11.0, 11.0, 11.0],
            uv_size: [12.0, 12.0, 12.0],
            tex: [0.0, 36.0],
            mirror: false,
        }
    );
    assert_eq!(SNOW_GOLEM_TEXTURED_PARTS[0].pose, SNOW_GOLEM_PARTS[0].pose);
    assert_eq!(SNOW_GOLEM_TEXTURED_PARTS[4].pose, SNOW_GOLEM_PARTS[4].pose);
}

#[test]
fn golem_texture_atlas_stitches_official_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&golem_texture_images()).unwrap();

    assert_eq!(layout.width, 128);
    assert_eq!(layout.height, 192);
    assert_eq!(layout.entries.len(), 2);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/iron_golem/iron_golem.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 128.0 / 192.0]);
    assert_eq!(
        layout.entries[1].texture.path,
        "textures/entity/snow_golem/snow_golem.png"
    );
    assert_close2(layout.entries[1].uv.min, [0.0, 128.0 / 192.0]);
    assert_close2(layout.entries[1].uv.max, [0.5, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let snow_start = rgba_offset(layout.width, 128, 0, "snow golem atlas row").unwrap();
    assert_eq!(&rgba[snow_start..snow_start + 4], &[1; 4]);
}

#[test]
fn golem_textured_meshes_use_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&golem_texture_images()).unwrap();

    let iron = entity_model_textured_mesh(
        &[EntityModelInstance::iron_golem(70, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert_eq!(iron.cutout_faces, 48);
    assert_eq!(iron.vertices.len(), 192);
    assert_eq!(iron.indices.len(), 288);
    assert_close2(iron.vertices[0].uv, [16.0 / 128.0, 0.0]);
    assert!(iron
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (iron_min, iron_max) = textured_mesh_extents(&iron);
    assert_close3(iron_min, [-0.8125, 64.001, -0.3125]);
    assert_close3(iron_max, [0.8125, 66.6885, 0.59375]);

    let snow = entity_model_textured_mesh(
        &[EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );
    assert_eq!(snow.cutout_faces, 30);
    assert_eq!(snow.vertices.len(), 120);
    assert_eq!(snow.indices.len(), 180);
    assert_close2(snow.vertices[0].uv, [16.0 / 128.0, 128.0 / 192.0]);
    assert!(snow
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (snow_min, snow_max) = textured_mesh_extents(&snow);
    assert_close3(snow_min, [-0.6407774, 64.03225, -0.34375]);
    assert_close3(snow_max, [0.6407774, 65.71975, 0.34375]);
}

#[test]
fn golem_textured_meshes_apply_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&golem_texture_images()).unwrap();
    for base in [
        EntityModelInstance::iron_golem(71, [0.0, 64.0, 0.0], 0.0),
        EntityModelInstance::snow_golem(122, [0.0, 64.0, 0.0], 0.0),
    ] {
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let yawed = entity_model_textured_mesh(&[base.with_head_look(45.0, 0.0)], &atlas);
        let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -20.0)], &atlas);
        assert_eq!(resting.vertices.len(), yawed.vertices.len());
        assert_ne!(resting.vertices, yawed.vertices, "{:?}", base.kind);
        assert_ne!(yawed.vertices, pitched.vertices, "{:?}", base.kind);
    }
}

fn golem_texture_images() -> Vec<EntityModelTextureImage> {
    golem_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
