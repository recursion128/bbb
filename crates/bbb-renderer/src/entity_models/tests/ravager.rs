use super::*;

#[test]
fn ravager_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(RAVAGER_PARTS.len(), 6);
    assert_part_tree(
        &RAVAGER_PARTS[0],
        [0.0, -7.0, 5.5],
        [0.0, 0.0, 0.0],
        RAVAGER_NECK.as_slice(),
        RAVAGER_NECK_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &RAVAGER_NECK_CHILDREN[0],
        [0.0, 16.0, -17.0],
        [0.0, 0.0, 0.0],
        RAVAGER_HEAD.as_slice(),
        RAVAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &RAVAGER_HEAD_CHILDREN[0],
        [-10.0, -14.0, -8.0],
        [1.0995574, 0.0, 0.0],
        RAVAGER_HORN.as_slice(),
    );
    assert_part(
        &RAVAGER_HEAD_CHILDREN[1],
        [8.0, -14.0, -8.0],
        [1.0995574, 0.0, 0.0],
        RAVAGER_HORN.as_slice(),
    );
    assert_part(
        &RAVAGER_HEAD_CHILDREN[2],
        [0.0, -2.0, 2.0],
        [0.0, 0.0, 0.0],
        RAVAGER_MOUTH.as_slice(),
    );
    assert_part(
        &RAVAGER_PARTS[1],
        [0.0, 1.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        RAVAGER_BODY.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &RAVAGER_PARTS[2],
            [-8.0, -13.0, 18.0],
            RAVAGER_HIND_LEG.as_slice(),
        ),
        (
            &RAVAGER_PARTS[3],
            [8.0, -13.0, 18.0],
            RAVAGER_HIND_LEG.as_slice(),
        ),
        (
            &RAVAGER_PARTS[4],
            [-8.0, -13.0, -5.0],
            RAVAGER_FRONT_LEG.as_slice(),
        ),
        (
            &RAVAGER_PARTS[5],
            [8.0, -13.0, -5.0],
            RAVAGER_FRONT_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }
}

#[test]
fn ravager_mesh_uses_vanilla_body_layer_geometry() {
    let ravager = entity_model_mesh(&[EntityModelInstance::ravager(224, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(ravager.opaque_faces, 72);
    assert_eq!(ravager.vertices.len(), 288);
    assert_eq!(ravager.indices.len(), 432);
    assert!(ravager
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(RAVAGER_GRAY, 0.78)));

    let (min, max) = mesh_extents(&ravager);
    assert!(max[1] - min[1] > 2.0);
    assert!(max[2] - min[2] > 2.0);
}

#[test]
fn ravager_texture_ref_matches_vanilla_renderer() {
    let kind = EntityModelKind::Ravager;
    assert_eq!(kind.model_key(), "ravager");
    assert_eq!(
        kind.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/illager/ravager.png",
            size: [128, 128],
        })
    );
    assert_eq!(
        ravager_entity_texture_refs(),
        [EntityModelTextureRef {
            path: "textures/entity/illager/ravager.png",
            size: [128, 128],
        }]
    );
    assert!(entity_model_texture_refs().contains(&RAVAGER_TEXTURE_REF));
}

#[test]
fn ravager_textured_layer_pass_matches_vanilla_renderer_model_layer() {
    let passes = ravager_textured_layer_passes();

    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::RavagerBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_RAVAGER);
    assert_eq!(passes[0].texture, RAVAGER_TEXTURE_REF);
    assert_eq!(passes[0].parts, RAVAGER_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
}

#[test]
fn ravager_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_RAVAGER, "minecraft:ravager#main");
    assert_eq!(RAVAGER_TEXTURE_REF.size, [128, 128]);
    assert_eq!(RAVAGER_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        RAVAGER_TEXTURED_NECK[0],
        TexturedModelCubeDesc {
            min: [-5.0, -1.0, -18.0],
            size: [10.0, 10.0, 18.0],
            uv_size: [10.0, 10.0, 18.0],
            tex: [68.0, 73.0],
            mirror: false,
        }
    );
    assert_eq!(
        RAVAGER_TEXTURED_HEAD,
        [
            TexturedModelCubeDesc {
                min: [-8.0, -20.0, -14.0],
                size: [16.0, 20.0, 16.0],
                uv_size: [16.0, 20.0, 16.0],
                tex: [0.0, 0.0],
                mirror: false,
            },
            TexturedModelCubeDesc {
                min: [-2.0, -6.0, -18.0],
                size: [4.0, 8.0, 4.0],
                uv_size: [4.0, 8.0, 4.0],
                tex: [0.0, 0.0],
                mirror: false,
            },
        ]
    );
    assert_eq!(
        RAVAGER_TEXTURED_LEFT_HORN[0],
        TexturedModelCubeDesc {
            min: [0.0, -14.0, -2.0],
            size: [2.0, 14.0, 4.0],
            uv_size: [2.0, 14.0, 4.0],
            tex: [74.0, 55.0],
            mirror: true,
        }
    );
    assert_eq!(
        RAVAGER_TEXTURED_BODY,
        [
            TexturedModelCubeDesc {
                min: [-7.0, -10.0, -7.0],
                size: [14.0, 16.0, 20.0],
                uv_size: [14.0, 16.0, 20.0],
                tex: [0.0, 55.0],
                mirror: false,
            },
            TexturedModelCubeDesc {
                min: [-6.0, 6.0, -7.0],
                size: [12.0, 13.0, 18.0],
                uv_size: [12.0, 13.0, 18.0],
                tex: [0.0, 91.0],
                mirror: false,
            },
        ]
    );
    assert_eq!(RAVAGER_TEXTURED_PARTS[0].pose, RAVAGER_PARTS[0].pose);
    assert_eq!(
        RAVAGER_TEXTURED_PARTS[0].children,
        RAVAGER_TEXTURED_NECK_CHILDREN.as_slice()
    );
    assert_eq!(
        RAVAGER_TEXTURED_NECK_CHILDREN[0].children,
        RAVAGER_TEXTURED_HEAD_CHILDREN.as_slice()
    );
    assert_eq!(RAVAGER_TEXTURED_PARTS[5].pose, RAVAGER_PARTS[5].pose);
}

#[test]
fn entity_texture_atlas_stitches_official_ravager_png_slot() {
    let (layout, rgba) = build_entity_model_texture_atlas(&ravager_texture_images()).unwrap();

    assert_eq!(layout.width, 128);
    assert_eq!(layout.height, 128);
    assert_eq!(layout.entries.len(), 1);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/illager/ravager.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
}

#[test]
fn ravager_textured_mesh_uses_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&ravager_texture_images()).unwrap();
    let instance = EntityModelInstance::ravager(109, [0.0, 64.0, 0.0], 0.0);
    let mesh = entity_model_textured_mesh(&[instance], &atlas);

    assert_eq!(mesh.cutout_faces, 72);
    assert_eq!(mesh.vertices.len(), 288);
    assert_eq!(mesh.indices.len(), 432);
    assert_close2(mesh.vertices[0].uv, [96.0 / 128.0, 73.0 / 128.0]);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    let colored = entity_model_mesh(&[instance]);
    let (expected_min, expected_max) = mesh_extents(&colored);
    let (actual_min, actual_max) = textured_mesh_extents(&mesh);
    assert_close3(actual_min, expected_min);
    assert_close3(actual_max, expected_max);
}

#[test]
fn ravager_textured_mesh_turns_nested_head_not_neck_or_body() {
    let (atlas, _) = build_entity_model_texture_atlas(&ravager_texture_images()).unwrap();
    let base = EntityModelInstance::ravager(110, [0.0, 64.0, 0.0], 0.0);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let yawed = entity_model_textured_mesh(&[base.with_head_look(50.0, 0.0)], &atlas);
    let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -20.0)], &atlas);

    // Emit order matches the colored path: neck cube (verts 0..24), head + horn/
    // mouth children (24..144), then body and legs (144..). The vanilla look turns
    // the nested head only; the neck cube and the body/legs stay put.
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_eq!(resting.vertices[0..24], yawed.vertices[0..24]);
    assert_ne!(resting.vertices[24..144], yawed.vertices[24..144]);
    assert_ne!(yawed.vertices[24..144], pitched.vertices[24..144]);
    assert_eq!(resting.vertices[144..], yawed.vertices[144..]);
}

fn ravager_texture_images() -> Vec<EntityModelTextureImage> {
    ravager_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
