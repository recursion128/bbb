use super::*;

#[test]
fn enderman_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ENDERMAN_HEAD[0],
        ModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            color: ENDERMAN_DARK,
        }
    );
    assert_eq!(
        ENDERMAN_HAT[0],
        ModelCubeDesc {
            min: [-3.5, -7.5, -3.5],
            size: [7.0, 7.0, 7.0],
            color: ENDERMAN_DARK,
        }
    );
    assert_eq!(
        ENDERMAN_BODY[0],
        ModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            color: ENDERMAN_DARK,
        }
    );
    assert_eq!(
        ENDERMAN_ARM[0],
        ModelCubeDesc {
            min: [-1.0, -2.0, -1.0],
            size: [2.0, 30.0, 2.0],
            color: ENDERMAN_DARK,
        }
    );
    assert_eq!(
        ENDERMAN_LEG[0],
        ModelCubeDesc {
            min: [-1.0, 0.0, -1.0],
            size: [2.0, 30.0, 2.0],
            color: ENDERMAN_DARK,
        }
    );

    assert_eq!(ENDERMAN_PARTS.len(), 6);
    assert_part_tree(
        &ENDERMAN_PARTS[0],
        [0.0, -13.0, 0.0],
        [0.0, 0.0, 0.0],
        ENDERMAN_HEAD.as_slice(),
        ENDERMAN_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ENDERMAN_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ENDERMAN_HAT.as_slice(),
    );
    assert_part(
        &ENDERMAN_PARTS[1],
        [0.0, -14.0, 0.0],
        [0.0, 0.0, 0.0],
        ENDERMAN_BODY.as_slice(),
    );

    let limb_specs = [
        ([-5.0, -12.0, 0.0], ENDERMAN_ARM.as_slice()),
        ([5.0, -12.0, 0.0], ENDERMAN_ARM.as_slice()),
        ([-2.0, -5.0, 0.0], ENDERMAN_LEG.as_slice()),
        ([2.0, -5.0, 0.0], ENDERMAN_LEG.as_slice()),
    ];
    for (part, (offset, cubes)) in ENDERMAN_PARTS[2..].iter().zip(limb_specs) {
        assert_part(part, offset, [0.0, 0.0, 0.0], cubes);
    }
}

#[test]
fn enderman_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::enderman(141, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.375, 63.9385, -0.25]);
    assert_close3(max, [0.375, 66.8135, 0.25]);
}

#[test]
fn enderman_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Enderman.model_key(), "enderman");
    assert_eq!(
        EntityModelKind::Enderman.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/enderman/enderman.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        enderman_entity_texture_refs(),
        [
            EntityModelTextureRef {
                path: "textures/entity/enderman/enderman.png",
                size: [64, 32],
            },
            EntityModelTextureRef {
                path: "textures/entity/enderman/enderman_eyes.png",
                size: [64, 32],
            },
        ]
    );
    assert_eq!(
        EntityModelKind::Enderman.vanilla_layer_texture_refs(),
        &[ENDERMAN_EYES_TEXTURE_REF]
    );
}

#[test]
fn enderman_textured_layer_passes_match_vanilla_renderer_model_layers() {
    let passes = enderman_textured_layer_passes();
    assert_eq!(passes.len(), 2);

    assert_eq!(passes[0].kind, EntityModelLayerKind::EndermanBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_ENDERMAN);
    assert_eq!(passes[0].texture, ENDERMAN_TEXTURE_REF);
    assert_eq!(passes[0].parts, ENDERMAN_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );

    assert_eq!(passes[1].kind, EntityModelLayerKind::EndermanEyes);
    assert_eq!(passes[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(passes[1].model_layer, MODEL_LAYER_ENDERMAN);
    assert_eq!(passes[1].texture, ENDERMAN_EYES_TEXTURE_REF);
    assert_eq!(passes[1].parts, ENDERMAN_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[1].collector_order, passes[1].submit_sequence),
        (1, 1)
    );
}

#[test]
fn enderman_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_ENDERMAN, "minecraft:enderman#main");
    assert_eq!(
        ENDERMAN_EYES_TEXTURE_REF,
        EntityModelTextureRef {
            path: "textures/entity/enderman/enderman_eyes.png",
            size: [64, 32],
        }
    );
    assert_eq!(ENDERMAN_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        ENDERMAN_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        ENDERMAN_TEXTURED_HAT[0],
        TexturedModelCubeDesc {
            min: [-3.5, -7.5, -3.5],
            size: [7.0, 7.0, 7.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [0.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(
        ENDERMAN_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            uv_size: [8.0, 12.0, 4.0],
            tex: [32.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(
        ENDERMAN_TEXTURED_RIGHT_ARM[0],
        TexturedModelCubeDesc {
            min: [-1.0, -2.0, -1.0],
            size: [2.0, 30.0, 2.0],
            uv_size: [2.0, 30.0, 2.0],
            tex: [56.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        ENDERMAN_TEXTURED_LEFT_ARM[0],
        TexturedModelCubeDesc {
            min: [-1.0, -2.0, -1.0],
            size: [2.0, 30.0, 2.0],
            uv_size: [2.0, 30.0, 2.0],
            tex: [56.0, 0.0],
            mirror: true,
        }
    );
    assert_eq!(ENDERMAN_TEXTURED_RIGHT_LEG[0].mirror, false);
    assert_eq!(ENDERMAN_TEXTURED_LEFT_LEG[0].mirror, true);
    assert_eq!(ENDERMAN_TEXTURED_PARTS[0].pose, ENDERMAN_PARTS[0].pose);
    assert_eq!(
        ENDERMAN_TEXTURED_PARTS[0].children,
        ENDERMAN_TEXTURED_HEAD_CHILDREN.as_slice()
    );
    assert_eq!(ENDERMAN_TEXTURED_PARTS[5].pose, ENDERMAN_PARTS[5].pose);
}

#[test]
fn entity_texture_atlas_stitches_official_enderman_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 64);
    assert_eq!(layout.entries.len(), 2);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/enderman/enderman.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 0.5]);
    assert_eq!(
        layout.entries[1].texture.path,
        "textures/entity/enderman/enderman_eyes.png"
    );
    assert_close2(layout.entries[1].uv.min, [0.0, 0.5]);
    assert_close2(layout.entries[1].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let eyes_start = rgba_offset(layout.width, 32, 0, "enderman eyes atlas row").unwrap();
    assert_eq!(&rgba[eyes_start..eyes_start + 4], &[1; 4]);
}

#[test]
fn enderman_textured_mesh_uses_parent_geometry_for_base_and_eyes_layers() {
    let (atlas, _) = build_entity_model_texture_atlas(&enderman_texture_images()).unwrap();

    let meshes = entity_model_textured_meshes(
        &[EntityModelInstance::enderman(142, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );

    assert_eq!(meshes.cutout.cutout_faces, 42);
    assert_eq!(meshes.cutout.vertices.len(), 168);
    assert_eq!(meshes.cutout.indices.len(), 252);
    assert_eq!(meshes.eyes.cutout_faces, 42);
    assert_eq!(meshes.eyes.vertices.len(), 168);
    assert_eq!(meshes.eyes.indices.len(), 252);
    assert_close2(meshes.cutout.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert_close2(meshes.eyes.vertices[0].uv, [16.0 / 64.0, 0.5]);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(meshes
        .eyes
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert_eq!(
        textured_mesh_extents(&meshes.eyes),
        textured_mesh_extents(&meshes.cutout)
    );
    let (min, max) = textured_mesh_extents(&meshes.cutout);
    assert_close3(min, [-0.375, 63.9385, -0.25]);
    assert_close3(max, [0.375, 66.8135, 0.25]);
}

fn enderman_texture_images() -> Vec<EntityModelTextureImage> {
    enderman_entity_texture_refs()
        .iter()
        .copied()
        .enumerate()
        .map(|(index, texture)| EntityModelTextureImage {
            texture,
            rgba: vec![
                u8::try_from(index).unwrap();
                usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap()
            ],
        })
        .collect()
}
