use super::*;

#[test]
fn player_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(PLAYER_WIDE_PARTS.len(), 6);
    assert_part_tree(
        &PLAYER_WIDE_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_HEAD.as_slice(),
        PLAYER_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &PLAYER_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_HAT.as_slice(),
    );
    assert_part_tree(
        &PLAYER_WIDE_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_BODY.as_slice(),
        PLAYER_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &PLAYER_BODY_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_JACKET.as_slice(),
    );
    assert_part_tree(
        &PLAYER_WIDE_PARTS[2],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_WIDE_RIGHT_ARM.as_slice(),
        PLAYER_WIDE_RIGHT_ARM_CHILDREN.as_slice(),
    );
    assert_part(
        &PLAYER_WIDE_RIGHT_ARM_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_WIDE_RIGHT_SLEEVE.as_slice(),
    );
    assert_part_tree(
        &PLAYER_WIDE_PARTS[3],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_WIDE_LEFT_ARM.as_slice(),
        PLAYER_WIDE_LEFT_ARM_CHILDREN.as_slice(),
    );
    assert_part(
        &PLAYER_WIDE_LEFT_ARM_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_WIDE_LEFT_SLEEVE.as_slice(),
    );
    assert_part_tree(
        &PLAYER_WIDE_PARTS[4],
        [-1.9, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_LEG.as_slice(),
        PLAYER_RIGHT_PANTS_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &PLAYER_WIDE_PARTS[5],
        [1.9, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_LEG.as_slice(),
        PLAYER_LEFT_PANTS_CHILDREN.as_slice(),
    );
    assert_part(
        &PLAYER_RIGHT_PANTS_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_PANTS.as_slice(),
    );
    assert_part(
        &PLAYER_LEFT_PANTS_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_PANTS.as_slice(),
    );

    assert_eq!(PLAYER_SLIM_PARTS.len(), 6);
    assert_part_tree(
        &PLAYER_SLIM_PARTS[2],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_SLIM_RIGHT_ARM.as_slice(),
        PLAYER_SLIM_RIGHT_ARM_CHILDREN.as_slice(),
    );
    assert_part(
        &PLAYER_SLIM_RIGHT_ARM_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_SLIM_RIGHT_SLEEVE.as_slice(),
    );
    assert_part_tree(
        &PLAYER_SLIM_PARTS[3],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_SLIM_LEFT_ARM.as_slice(),
        PLAYER_SLIM_LEFT_ARM_CHILDREN.as_slice(),
    );
    assert_part(
        &PLAYER_SLIM_LEFT_ARM_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        PLAYER_SLIM_LEFT_SLEEVE.as_slice(),
    );
}

#[test]
fn player_mesh_uses_vanilla_body_layer_geometry_and_avatar_scale() {
    let wide = entity_model_mesh(&[EntityModelInstance::player(
        155,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let slim = entity_model_mesh(&[EntityModelInstance::player(
        156,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);

    for mesh in [&wide, &slim] {
        assert_eq!(mesh.opaque_faces, 72);
        assert_eq!(mesh.vertices.len(), 288);
        assert_eq!(mesh.indices.len(), 432);
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.color == shade_color(PLAYER_BLUE, 0.78)));
    }

    let (wide_min, wide_max) = mesh_extents(&wide);
    let (slim_min, slim_max) = mesh_extents(&slim);
    assert!(wide_max[1] - wide_min[1] > 1.8);
    assert!(wide_max[1] - wide_min[1] < 2.0);
    assert!(wide_max[0] - wide_min[0] > slim_max[0] - slim_min[0]);
    assert_ne!(wide.vertices, slim.vertices);
}

#[test]
fn player_texture_refs_match_vanilla_default_assets() {
    let cases = [
        (
            false,
            "player",
            EntityModelTextureRef {
                path: "textures/entity/player/wide/steve.png",
                size: [64, 64],
            },
        ),
        (
            true,
            "player_slim",
            EntityModelTextureRef {
                path: "textures/entity/player/slim/steve.png",
                size: [64, 64],
            },
        ),
    ];

    for (slim, model_key, texture) in cases {
        let kind = EntityModelKind::Player {
            slim,
            parts: PLAYER_MODEL_PARTS_ALL_VISIBLE,
        };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn player_model_part_visibility_masks_match_vanilla_player_model_part_bits() {
    assert_eq!(PlayerModelPartVisibility::CAPE_MASK, 1 << 0);
    assert_eq!(PlayerModelPartVisibility::JACKET_MASK, 1 << 1);
    assert_eq!(PlayerModelPartVisibility::LEFT_SLEEVE_MASK, 1 << 2);
    assert_eq!(PlayerModelPartVisibility::RIGHT_SLEEVE_MASK, 1 << 3);
    assert_eq!(PlayerModelPartVisibility::LEFT_PANTS_MASK, 1 << 4);
    assert_eq!(PlayerModelPartVisibility::RIGHT_PANTS_MASK, 1 << 5);
    assert_eq!(PlayerModelPartVisibility::HAT_MASK, 1 << 6);
    assert_eq!(PlayerModelPartVisibility::ALL_MASK, 0x7f);
    assert_eq!(
        PLAYER_MODEL_PARTS_ALL_VISIBLE.vanilla_mask(),
        PlayerModelPartVisibility::ALL_MASK
    );
    assert_eq!(PLAYER_MODEL_PARTS_ALL_HIDDEN.vanilla_mask(), 0);

    let mask = PlayerModelPartVisibility::HAT_MASK
        | PlayerModelPartVisibility::JACKET_MASK
        | PlayerModelPartVisibility::LEFT_SLEEVE_MASK
        | PlayerModelPartVisibility::RIGHT_PANTS_MASK;
    let parts = PlayerModelPartVisibility::from_vanilla_mask(mask);
    assert!(parts.hat);
    assert!(parts.jacket);
    assert!(parts.left_sleeve);
    assert!(!parts.right_sleeve);
    assert!(!parts.left_pants);
    assert!(parts.right_pants);
    assert!(!parts.cape);
    assert_eq!(parts.vanilla_mask(), mask);
}

#[test]
fn player_textured_layer_passes_match_vanilla_avatar_renderer_model_layers() {
    let wide = player_textured_layer_passes(false, PLAYER_MODEL_PARTS_ALL_VISIBLE);
    assert_eq!(wide.len(), 1);
    assert_eq!(wide[0].kind, EntityModelLayerKind::PlayerBase);
    assert_eq!(wide[0].model_layer, MODEL_LAYER_PLAYER);
    assert_eq!(wide[0].texture, PLAYER_WIDE_STEVE_TEXTURE_REF);
    assert_eq!(wide[0].parts, PLAYER_WIDE_TEXTURED_PARTS.as_slice());
    assert_eq!(
        wide[0].visibility,
        EntityModelLayerVisibility::PlayerParts(PLAYER_MODEL_PARTS_ALL_VISIBLE)
    );
    assert_eq!(wide[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((wide[0].collector_order, wide[0].submit_sequence), (0, 0));

    let slim_parts = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::HAT_MASK | PlayerModelPartVisibility::LEFT_SLEEVE_MASK,
    );
    let slim = player_textured_layer_passes(true, slim_parts);
    assert_eq!(slim.len(), 1);
    assert_eq!(slim[0].kind, EntityModelLayerKind::PlayerBase);
    assert_eq!(slim[0].model_layer, MODEL_LAYER_PLAYER_SLIM);
    assert_eq!(slim[0].texture, PLAYER_SLIM_STEVE_TEXTURE_REF);
    assert_eq!(slim[0].parts, PLAYER_SLIM_TEXTURED_PARTS.as_slice());
    assert_eq!(
        slim[0].visibility,
        EntityModelLayerVisibility::PlayerParts(slim_parts)
    );
    assert_eq!(slim[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((slim[0].collector_order, slim[0].submit_sequence), (0, 0));
}

#[test]
fn player_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_PLAYER, "minecraft:player#main");
    assert_eq!(MODEL_LAYER_PLAYER_SLIM, "minecraft:player_slim#main");
    assert_eq!(PLAYER_WIDE_TEXTURED_PARTS.len(), 6);
    assert_eq!(PLAYER_SLIM_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        PLAYER_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        PLAYER_TEXTURED_HAT[0],
        TexturedModelCubeDesc {
            min: [-4.5, -8.5, -4.5],
            size: [9.0, 9.0, 9.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [32.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        PLAYER_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            uv_size: [8.0, 12.0, 4.0],
            tex: [16.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(
        PLAYER_TEXTURED_JACKET[0],
        TexturedModelCubeDesc {
            min: [-4.25, -0.25, -2.25],
            size: [8.5, 12.5, 4.5],
            uv_size: [8.0, 12.0, 4.0],
            tex: [16.0, 32.0],
            mirror: false,
        }
    );
    assert_eq!(PLAYER_WIDE_TEXTURED_RIGHT_ARM[0].tex, [40.0, 16.0]);
    assert_eq!(PLAYER_WIDE_TEXTURED_LEFT_ARM[0].tex, [32.0, 48.0]);
    assert_eq!(PLAYER_WIDE_TEXTURED_RIGHT_SLEEVE[0].tex, [40.0, 32.0]);
    assert_eq!(PLAYER_WIDE_TEXTURED_LEFT_SLEEVE[0].tex, [48.0, 48.0]);
    assert_eq!(PLAYER_SLIM_TEXTURED_RIGHT_ARM[0].size, [3.0, 12.0, 4.0]);
    assert_eq!(PLAYER_SLIM_TEXTURED_LEFT_ARM[0].size, [3.0, 12.0, 4.0]);
    assert_eq!(
        PLAYER_SLIM_TEXTURED_RIGHT_SLEEVE[0].uv_size,
        [3.0, 12.0, 4.0]
    );
    assert_eq!(
        PLAYER_SLIM_TEXTURED_LEFT_SLEEVE[0].uv_size,
        [3.0, 12.0, 4.0]
    );
    assert_eq!(PLAYER_TEXTURED_RIGHT_LEG[0].tex, [0.0, 16.0]);
    assert_eq!(PLAYER_TEXTURED_LEFT_LEG[0].tex, [16.0, 48.0]);
    assert_eq!(PLAYER_TEXTURED_RIGHT_PANTS[0].tex, [0.0, 32.0]);
    assert_eq!(PLAYER_TEXTURED_LEFT_PANTS[0].tex, [0.0, 48.0]);
    assert_eq!(
        PLAYER_WIDE_TEXTURED_PARTS[0].pose,
        PLAYER_WIDE_PARTS[0].pose
    );
    assert_eq!(
        PLAYER_SLIM_TEXTURED_PARTS[2].pose,
        PLAYER_SLIM_PARTS[2].pose
    );
}

#[test]
fn entity_texture_atlas_stitches_official_player_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 128);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/player/wide/steve.png",
            "textures/entity/player/slim/steve.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 0.5]);
    assert_close2(layout.entries[1].uv.min, [0.0, 0.5]);
    assert_close2(layout.entries[1].uv.max, [1.0, 1.0]);
    let slim_first_pixel = rgba_offset(layout.width, 64, 0, "test").unwrap();
    assert_eq!(&rgba[0..4], &[0; 4]);
    assert_eq!(&rgba[slim_first_pixel..slim_first_pixel + 4], &[1; 4]);
}

#[test]
fn player_textured_mesh_uses_vanilla_uvs_tints_and_avatar_scale() {
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    let wide = entity_model_textured_mesh(
        &[EntityModelInstance::player(
            901,
            [0.0, 64.0, 0.0],
            0.0,
            false,
        )],
        &atlas,
    );
    let slim = entity_model_textured_mesh(
        &[EntityModelInstance::player(
            902,
            [0.0, 64.0, 0.0],
            0.0,
            true,
        )],
        &atlas,
    );

    for mesh in [&wide, &slim] {
        assert_eq!(mesh.cutout_faces, 72);
        assert_eq!(mesh.vertices.len(), 288);
        assert_eq!(mesh.indices.len(), 432);
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    }
    assert_close2(wide.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert_close2(slim.vertices[0].uv, [16.0 / 64.0, 0.5]);

    let (wide_min, wide_max) = textured_mesh_extents(&wide);
    let (slim_min, slim_max) = textured_mesh_extents(&slim);
    assert!(wide_max[1] - wide_min[1] > 1.8);
    assert!(wide_max[1] - wide_min[1] < 2.0);
    assert!(wide_max[0] - wide_min[0] > slim_max[0] - slim_min[0]);
    assert_ne!(wide.vertices, slim.vertices);
}

#[test]
fn player_textured_mesh_applies_vanilla_model_part_visibility_to_overlay_parts() {
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    let hidden = entity_model_textured_mesh(
        &[EntityModelInstance::player_with_parts(
            903,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            PLAYER_MODEL_PARTS_ALL_HIDDEN,
        )],
        &atlas,
    );
    assert_eq!(hidden.cutout_faces, 36);
    assert_eq!(hidden.vertices.len(), 144);
    assert_eq!(hidden.indices.len(), 216);

    let partial_parts = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::HAT_MASK | PlayerModelPartVisibility::RIGHT_SLEEVE_MASK,
    );
    let partial = entity_model_textured_mesh(
        &[EntityModelInstance::player_with_parts(
            904,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            partial_parts,
        )],
        &atlas,
    );
    assert_eq!(partial.cutout_faces, 48);
    assert_eq!(partial.vertices.len(), 192);
    assert_eq!(partial.indices.len(), 288);
    assert!(partial
        .vertices
        .iter()
        .any(|vertex| vertex.uv[1] >= 32.0 / 64.0));
}

#[test]
fn player_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    for slim in [false, true] {
        let base = EntityModelInstance::player(903, [0.0, 64.0, 0.0], 0.0, slim);
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let yawed = entity_model_textured_mesh(&[base.with_head_look(45.0, 0.0)], &atlas);
        let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -20.0)], &atlas);

        // Head look turns the head part (index 0, shared across all passes)
        // without changing the vertex count.
        assert_eq!(resting.vertices.len(), yawed.vertices.len());
        assert_ne!(resting.vertices, yawed.vertices, "slim={slim}");
        assert_ne!(yawed.vertices, pitched.vertices, "slim={slim}");
    }
}

fn player_texture_images() -> Vec<EntityModelTextureImage> {
    player_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
