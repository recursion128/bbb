use super::*;

#[test]
fn witch_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        WITCH_HEAD[0],
        ModelCubeDesc {
            min: [-4.0, -10.0, -4.0],
            size: [8.0, 10.0, 8.0],
            color: WITCH_ROBE,
        }
    );
    assert_eq!(
        WITCH_HAT_4[0],
        ModelCubeDesc {
            min: [-0.25, -0.25, -0.25],
            size: [1.5, 2.5, 1.5],
            color: WITCH_HAT_COLOR,
        }
    );
    assert_eq!(
        WITCH_MOLE[0],
        ModelCubeDesc {
            min: [0.25, 3.25, -6.5],
            size: [0.5, 0.5, 0.5],
            color: WITCH_ROBE,
        }
    );

    assert_eq!(WITCH_PARTS.len(), 5);
    assert_part_tree(
        &WITCH_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_HEAD.as_slice(),
        WITCH_HEAD_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &WITCH_HEAD_CHILDREN[0],
        [-5.0, -10.03125, -5.0],
        [0.0, 0.0, 0.0],
        WITCH_HAT.as_slice(),
        WITCH_HAT_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &WITCH_HAT_CHILDREN[0],
        [1.75, -4.0, 2.0],
        [-0.05235988, 0.0, 0.02617994],
        WITCH_HAT_2.as_slice(),
        WITCH_HAT_2_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &WITCH_HAT_2_CHILDREN[0],
        [1.75, -4.0, 2.0],
        [-0.10471976, 0.0, 0.05235988],
        WITCH_HAT_3.as_slice(),
        WITCH_HAT_3_CHILDREN.as_slice(),
    );
    assert_part(
        &WITCH_HAT_3_CHILDREN[0],
        [1.75, -2.0, 2.0],
        [-(std::f32::consts::PI / 15.0), 0.0, 0.10471976],
        WITCH_HAT_4.as_slice(),
    );
    assert_part_tree(
        &WITCH_HEAD_CHILDREN[1],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_NOSE.as_slice(),
        WITCH_NOSE_CHILDREN.as_slice(),
    );
    assert_part(
        &WITCH_NOSE_CHILDREN[0],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_MOLE.as_slice(),
    );
    assert_part_tree(
        &WITCH_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_BODY.as_slice(),
        WITCH_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &WITCH_BODY_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_JACKET.as_slice(),
    );
    assert_part(
        &WITCH_PARTS[2],
        [0.0, 3.0, -1.0],
        [-0.75, 0.0, 0.0],
        WITCH_ARMS.as_slice(),
    );
    assert_part(
        &WITCH_PARTS[3],
        [-2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_LEG.as_slice(),
    );
    assert_part(
        &WITCH_PARTS[4],
        [2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        WITCH_LEG.as_slice(),
    );
}

#[test]
fn witch_model_mesh_uses_vanilla_scaled_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::witch(66, [0.0, 64.0, 0.0], 0.0)]);

    assert_eq!(mesh.opaque_faces, 84);
    assert_eq!(mesh.vertices.len(), 336);
    assert_eq!(mesh.indices.len(), 504);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.46875, 64.00094, -0.29296878]);
    assert_close3(max, [0.46875003, 66.56483, 0.3839772]);
}

#[test]
fn witch_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Witch.model_key(), "witch");
    assert_eq!(
        EntityModelKind::Witch.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/witch/witch.png",
            size: [64, 128],
        })
    );
}

#[test]
fn witch_textured_layer_pass_matches_vanilla_renderer_model_layer() {
    let passes = witch_textured_layer_passes();

    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::WitchBase);
    assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_WITCH);
    assert_eq!(passes[0].texture, WITCH_TEXTURE_REF);
    assert_eq!(passes[0].parts, WITCH_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
}

#[test]
fn witch_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_WITCH, "minecraft:witch#main");
    assert_eq!(WITCH_TEXTURE_REF.size, [64, 128]);
    assert_eq!(WITCH_TEXTURED_PARTS.len(), 5);
    assert_eq!(
        WITCH_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-4.0, -10.0, -4.0],
            size: [8.0, 10.0, 8.0],
            uv_size: [8.0, 10.0, 8.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        WITCH_TEXTURED_HAT_4[0],
        TexturedModelCubeDesc {
            min: [-0.25, -0.25, -0.25],
            size: [1.5, 2.5, 1.5],
            uv_size: [1.0, 2.0, 1.0],
            tex: [0.0, 95.0],
            mirror: false,
        }
    );
    assert_eq!(
        WITCH_TEXTURED_MOLE[0],
        TexturedModelCubeDesc {
            min: [0.25, 3.25, -6.5],
            size: [0.5, 0.5, 0.5],
            uv_size: [1.0, 1.0, 1.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        WITCH_TEXTURED_LEFT_LEG[0],
        TexturedModelCubeDesc {
            min: [-2.0, 0.0, -2.0],
            size: [4.0, 12.0, 4.0],
            uv_size: [4.0, 12.0, 4.0],
            tex: [0.0, 22.0],
            mirror: true,
        }
    );
    assert_eq!(WITCH_TEXTURED_PARTS[0].pose, WITCH_PARTS[0].pose);
    assert_eq!(
        WITCH_TEXTURED_HEAD_CHILDREN[0].pose,
        WITCH_HEAD_CHILDREN[0].pose
    );
    assert_eq!(
        WITCH_TEXTURED_HAT_3_CHILDREN[0].pose,
        WITCH_HAT_3_CHILDREN[0].pose
    );
    assert_eq!(WITCH_TEXTURED_PARTS[4].pose, WITCH_PARTS[4].pose);
}

#[test]
fn witch_texture_atlas_stitches_official_png_slot() {
    let (layout, rgba) = build_entity_model_texture_atlas(&witch_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 128);
    assert_eq!(layout.entries.len(), 1);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/witch/witch.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
}

#[test]
fn witch_textured_mesh_uses_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&witch_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::witch(66, [0.0, 64.0, 0.0], 0.0)],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 84);
    assert_eq!(mesh.vertices.len(), 336);
    assert_eq!(mesh.indices.len(), 504);
    assert_close2(mesh.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-0.46875, 64.00094, -0.29296878]);
    assert_close3(max, [0.46875003, 66.56483, 0.3839772]);
}

fn witch_texture_images() -> Vec<EntityModelTextureImage> {
    witch_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
