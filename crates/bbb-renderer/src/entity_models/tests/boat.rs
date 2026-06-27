use super::*;

use crate::entity_models::colored::boat_model_root_transform;

#[test]
fn boat_model_parts_match_vanilla_26_1_layers() {
    assert_eq!(BOAT_COMMON_PARTS.len(), 7);
    assert_part(
        &BOAT_COMMON_PARTS[0],
        [0.0, 3.0, 1.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        BOAT_BOTTOM.as_slice(),
    );
    assert_part(
        &BOAT_COMMON_PARTS[1],
        [-15.0, 4.0, 4.0],
        [0.0, std::f32::consts::PI * 1.5, 0.0],
        BOAT_BACK.as_slice(),
    );
    assert_part(
        &BOAT_COMMON_PARTS[2],
        [15.0, 4.0, 0.0],
        [0.0, std::f32::consts::FRAC_PI_2, 0.0],
        BOAT_FRONT.as_slice(),
    );
    assert_part(
        &BOAT_COMMON_PARTS[3],
        [0.0, 4.0, -9.0],
        [0.0, std::f32::consts::PI, 0.0],
        BOAT_SIDE.as_slice(),
    );
    assert_part(
        &BOAT_COMMON_PARTS[4],
        [0.0, 4.0, 9.0],
        [0.0, 0.0, 0.0],
        BOAT_SIDE.as_slice(),
    );
    assert_part(
        &BOAT_COMMON_PARTS[5],
        [3.0, -5.0, 9.0],
        [0.0, 0.0, std::f32::consts::PI / 16.0],
        BOAT_LEFT_PADDLE.as_slice(),
    );
    assert_part(
        &BOAT_COMMON_PARTS[6],
        [3.0, -5.0, -9.0],
        [0.0, std::f32::consts::PI, std::f32::consts::PI / 16.0],
        BOAT_RIGHT_PADDLE.as_slice(),
    );

    assert_eq!(BOAT_CHEST_PARTS.len(), 3);
    assert_part(
        &BOAT_CHEST_PARTS[0],
        [-2.0, -5.0, -6.0],
        [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        BOAT_CHEST_BOTTOM.as_slice(),
    );
    assert_part(
        &BOAT_CHEST_PARTS[1],
        [-2.0, -9.0, -6.0],
        [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        BOAT_CHEST_LID.as_slice(),
    );
    assert_part(
        &BOAT_CHEST_PARTS[2],
        [-1.0, -6.0, -1.0],
        [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        BOAT_CHEST_LOCK.as_slice(),
    );

    assert_eq!(RAFT_COMMON_PARTS.len(), 3);
    assert_part(
        &RAFT_COMMON_PARTS[0],
        [0.0, -2.1, 1.0],
        [1.5708, 0.0, 0.0],
        RAFT_BOTTOM.as_slice(),
    );
    assert_part(
        &RAFT_COMMON_PARTS[1],
        [3.0, -4.0, 9.0],
        [0.0, 0.0, std::f32::consts::PI / 16.0],
        BOAT_LEFT_PADDLE.as_slice(),
    );
    assert_part(
        &RAFT_COMMON_PARTS[2],
        [3.0, -4.0, -9.0],
        [0.0, std::f32::consts::PI, std::f32::consts::PI / 16.0],
        BOAT_RIGHT_PADDLE.as_slice(),
    );

    assert_eq!(RAFT_CHEST_PARTS.len(), 3);
    assert_part(
        &RAFT_CHEST_PARTS[0],
        [-2.0, -10.1, -6.0],
        [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        BOAT_CHEST_BOTTOM.as_slice(),
    );
    assert_part(
        &RAFT_CHEST_PARTS[1],
        [-2.0, -14.1, -6.0],
        [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        BOAT_CHEST_LID.as_slice(),
    );
    assert_part(
        &RAFT_CHEST_PARTS[2],
        [-1.0, -11.1, -1.0],
        [0.0, -std::f32::consts::FRAC_PI_2, 0.0],
        BOAT_CHEST_LOCK.as_slice(),
    );
}

#[test]
fn boat_meshes_use_vanilla_body_layer_geometry() {
    let oak_boat = entity_model_mesh(&[EntityModelInstance::boat(
        89,
        [0.0, 64.0, 0.0],
        0.0,
        BoatModelFamily::Oak,
        false,
    )]);
    let oak_chest_boat = entity_model_mesh(&[EntityModelInstance::boat(
        90,
        [0.0, 64.0, 0.0],
        0.0,
        BoatModelFamily::Oak,
        true,
    )]);
    let bamboo_raft = entity_model_mesh(&[EntityModelInstance::boat(
        9,
        [0.0, 64.0, 0.0],
        0.0,
        BoatModelFamily::Bamboo,
        false,
    )]);
    let bamboo_chest_raft = entity_model_mesh(&[EntityModelInstance::boat(
        8,
        [0.0, 64.0, 0.0],
        0.0,
        BoatModelFamily::Bamboo,
        true,
    )]);

    assert_eq!(oak_boat.opaque_faces, 54);
    assert_eq!(oak_boat.vertices.len(), 216);
    assert_eq!(oak_boat.indices.len(), 324);
    assert_eq!(oak_chest_boat.opaque_faces, 72);
    assert_eq!(oak_chest_boat.vertices.len(), 288);
    assert_eq!(oak_chest_boat.indices.len(), 432);
    assert_eq!(bamboo_raft.opaque_faces, 36);
    assert_eq!(bamboo_raft.vertices.len(), 144);
    assert_eq!(bamboo_raft.indices.len(), 216);
    assert_eq!(bamboo_chest_raft.opaque_faces, 54);
    assert_eq!(bamboo_chest_raft.vertices.len(), 216);
    assert_eq!(bamboo_chest_raft.indices.len(), 324);
    assert_ne!(oak_boat.vertices, bamboo_raft.vertices);

    let (min, max) = mesh_extents(&oak_boat);
    assert!(max[0] - min[0] > 1.0);
    assert!(max[2] - min[2] > 1.0);
}

#[test]
fn boat_texture_refs_match_vanilla_model_layer_paths() {
    let cases = [
        (
            BoatModelFamily::Acacia,
            false,
            "boat_acacia",
            "textures/entity/boat/acacia.png",
            [128, 64],
        ),
        (
            BoatModelFamily::Bamboo,
            true,
            "chest_boat_bamboo",
            "textures/entity/chest_boat/bamboo.png",
            [128, 128],
        ),
        (
            BoatModelFamily::DarkOak,
            false,
            "boat_dark_oak",
            "textures/entity/boat/dark_oak.png",
            [128, 64],
        ),
        (
            BoatModelFamily::Mangrove,
            true,
            "chest_boat_mangrove",
            "textures/entity/chest_boat/mangrove.png",
            [128, 128],
        ),
        (
            BoatModelFamily::PaleOak,
            false,
            "boat_pale_oak",
            "textures/entity/boat/pale_oak.png",
            [128, 64],
        ),
        (
            BoatModelFamily::Spruce,
            true,
            "chest_boat_spruce",
            "textures/entity/chest_boat/spruce.png",
            [128, 128],
        ),
    ];

    for (family, chest, model_key, path, size) in cases {
        let kind = EntityModelKind::Boat { family, chest };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(
            kind.vanilla_texture_ref(),
            Some(EntityModelTextureRef { path, size })
        );
    }
}

#[test]
fn boat_textured_layer_passes_match_vanilla_renderer_model_layers() {
    let oak_boat = boat_textured_layer_passes(BoatModelFamily::Oak, false);
    assert_eq!(oak_boat.len(), 1);
    assert_eq!(oak_boat[0].kind, EntityModelLayerKind::BoatBase);
    assert_eq!(
        oak_boat[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(oak_boat[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(oak_boat[0].model_layer, MODEL_LAYER_OAK_BOAT);
    assert_eq!(oak_boat[0].texture, BOAT_OAK_TEXTURE_REF);
    assert_eq!(oak_boat[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((oak_boat[0].order, oak_boat[0].submit_sequence), (0, 0));

    let oak_chest_boat = boat_textured_layer_passes(BoatModelFamily::Oak, true);
    assert_eq!(oak_chest_boat[0].kind, EntityModelLayerKind::BoatBase);
    assert_eq!(oak_chest_boat[0].model_layer, MODEL_LAYER_OAK_CHEST_BOAT);
    assert_eq!(oak_chest_boat[0].texture, CHEST_BOAT_OAK_TEXTURE_REF);

    let bamboo_raft = boat_textured_layer_passes(BoatModelFamily::Bamboo, false);
    assert_eq!(bamboo_raft[0].model_layer, MODEL_LAYER_BAMBOO_RAFT);
    assert_eq!(bamboo_raft[0].texture, BOAT_BAMBOO_TEXTURE_REF);

    let bamboo_chest_raft = boat_textured_layer_passes(BoatModelFamily::Bamboo, true);
    assert_eq!(
        bamboo_chest_raft[0].model_layer,
        MODEL_LAYER_BAMBOO_CHEST_RAFT
    );
    assert_eq!(bamboo_chest_raft[0].texture, CHEST_BOAT_BAMBOO_TEXTURE_REF);
}

#[test]
fn boat_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_OAK_BOAT, "minecraft:boat/oak#main");
    assert_eq!(MODEL_LAYER_OAK_CHEST_BOAT, "minecraft:chest_boat/oak#main");
    assert_eq!(MODEL_LAYER_BAMBOO_RAFT, "minecraft:boat/bamboo#main");
    assert_eq!(
        MODEL_LAYER_BAMBOO_CHEST_RAFT,
        "minecraft:chest_boat/bamboo#main"
    );
    assert_eq!(BOAT_TEXTURED_PARTS.len(), 7);
    assert_eq!(BOAT_CHEST_TEXTURED_PARTS.len(), 10);
    assert_eq!(RAFT_TEXTURED_PARTS.len(), 3);
    assert_eq!(RAFT_CHEST_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        BOAT_TEXTURED_BOTTOM[0],
        TexturedModelCubeDesc {
            min: [-14.0, -9.0, -3.0],
            size: [28.0, 16.0, 3.0],
            uv_size: [28.0, 16.0, 3.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        BOAT_TEXTURED_RIGHT_SIDE[0],
        TexturedModelCubeDesc {
            min: [-14.0, -7.0, -1.0],
            size: [28.0, 6.0, 2.0],
            uv_size: [28.0, 6.0, 2.0],
            tex: [0.0, 35.0],
            mirror: false,
        }
    );
    assert_eq!(BOAT_TEXTURED_LEFT_SIDE[0].tex, [0.0, 43.0]);
    assert_eq!(
        BOAT_TEXTURED_LEFT_PADDLE[1],
        TexturedModelCubeDesc {
            min: [-1.001, -3.0, 8.0],
            size: [1.0, 6.0, 7.0],
            uv_size: [1.0, 6.0, 7.0],
            tex: [62.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        RAFT_TEXTURED_BOTTOM[1],
        TexturedModelCubeDesc {
            min: [-14.0, -9.0, -8.0],
            size: [28.0, 16.0, 4.0],
            uv_size: [28.0, 16.0, 4.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(RAFT_TEXTURED_LEFT_PADDLE[0].tex, [0.0, 24.0]);
    assert_eq!(RAFT_TEXTURED_RIGHT_PADDLE[0].tex, [40.0, 24.0]);
    assert_eq!(
        BOAT_TEXTURED_CHEST_BOTTOM[0],
        TexturedModelCubeDesc {
            min: [0.0, 0.0, 0.0],
            size: [12.0, 8.0, 12.0],
            uv_size: [12.0, 8.0, 12.0],
            tex: [0.0, 76.0],
            mirror: false,
        }
    );
    assert_eq!(BOAT_TEXTURED_CHEST_LID[0].tex, [0.0, 59.0]);
    assert_eq!(BOAT_TEXTURED_CHEST_LOCK[0].tex, [0.0, 59.0]);
    assert_eq!(BOAT_CHEST_TEXTURED_PARTS[7].pose, BOAT_CHEST_PARTS[0].pose);
    assert_eq!(RAFT_CHEST_TEXTURED_PARTS[3].pose, RAFT_CHEST_PARTS[0].pose);
}

#[test]
fn entity_texture_atlas_stitches_official_boat_png_slots() {
    let images = boat_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect::<Vec<_>>();

    let (layout, rgba) = build_entity_model_texture_atlas(&images).unwrap();

    assert_eq!(layout.width, 128);
    assert_eq!(layout.height, 1920);
    assert_eq!(layout.entries.len(), 20);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/boat/acacia.png"
    );
    assert_eq!(
        layout.entries[1].texture.path,
        "textures/entity/chest_boat/acacia.png"
    );
    assert_eq!(
        layout.entries[14].texture.path,
        "textures/entity/boat/oak.png"
    );
    assert_eq!(
        layout.entries[19].texture.path,
        "textures/entity/chest_boat/spruce.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 64.0 / 1920.0]);
    assert_close2(layout.entries[1].uv.min, [0.0, 64.0 / 1920.0]);
    assert_close2(layout.entries[1].uv.max, [1.0, 192.0 / 1920.0]);
    assert_close2(layout.entries[14].uv.min, [0.0, 1344.0 / 1920.0]);
    assert_close2(layout.entries[14].uv.max, [1.0, 1408.0 / 1920.0]);

    let acacia_chest_first_pixel = rgba_offset(layout.width, 64, 0, "test").unwrap();
    assert_eq!(
        &rgba[acacia_chest_first_pixel..acacia_chest_first_pixel + 4],
        &[1; 4]
    );
    let oak_first_pixel = rgba_offset(layout.width, 1344, 0, "test").unwrap();
    assert_eq!(&rgba[oak_first_pixel..oak_first_pixel + 4], &[14; 4]);
    let spruce_chest_first_pixel = rgba_offset(layout.width, 1792, 0, "test").unwrap();
    assert_eq!(
        &rgba[spruce_chest_first_pixel..spruce_chest_first_pixel + 4],
        &[19; 4]
    );
}

#[test]
fn boat_textured_mesh_uses_vanilla_uvs_tints_and_root_transform() {
    let images = [
        BOAT_OAK_TEXTURE_REF,
        CHEST_BOAT_OAK_TEXTURE_REF,
        BOAT_BAMBOO_TEXTURE_REF,
        CHEST_BOAT_BAMBOO_TEXTURE_REF,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(texture, vec![index as u8; len])
    })
    .collect::<Vec<_>>();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instances = [
        EntityModelInstance::boat(201, [0.0, 64.0, 0.0], 0.0, BoatModelFamily::Oak, false),
        EntityModelInstance::boat(202, [3.0, 64.0, 0.0], 0.0, BoatModelFamily::Oak, true),
        EntityModelInstance::boat(203, [6.0, 64.0, 0.0], 0.0, BoatModelFamily::Bamboo, false),
        EntityModelInstance::boat(204, [9.0, 64.0, 0.0], 0.0, BoatModelFamily::Bamboo, true),
    ];
    let instances = instances.map(|instance| {
        instance
            .with_light_coords((7_u32 << 4) | (12_u32 << 20))
            .with_white_overlay_progress(0.8)
            .with_has_red_overlay(true)
    });
    let meshes = entity_model_textured_meshes(&instances, &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.submissions.len(), 8);
    for (index, texture) in [
        BOAT_OAK_TEXTURE_REF,
        CHEST_BOAT_OAK_TEXTURE_REF,
        BOAT_BAMBOO_TEXTURE_REF,
        CHEST_BOAT_BAMBOO_TEXTURE_REF,
    ]
    .into_iter()
    .enumerate()
    {
        let base = meshes.submissions[index * 2];
        assert_eq!(base.texture, texture);
        assert_eq!(base.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(base.render_type.vanilla_name(), "entityCutout");
        assert_eq!(base.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(base.transform, boat_model_root_transform(instances[index]));
        assert_eq!(base.light, instances[index].render_state.shader_light());
        assert_eq!(base.overlay, [0.0, 10.0]);
        assert_ne!(base.overlay, instances[index].render_state.overlay_coords());
        assert_eq!((base.order, base.submit_sequence), (0, 0));

        let water_mask = meshes.submissions[index * 2 + 1];
        assert_eq!(water_mask.texture, texture);
        assert_eq!(
            water_mask.render_type,
            EntityModelLayerRenderType::WaterMask
        );
        assert_eq!(water_mask.render_type.vanilla_name(), "waterMask");
        assert_eq!(water_mask.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(water_mask.transform, base.transform);
        assert_eq!(water_mask.light, base.light);
        assert_eq!(water_mask.overlay, [0.0, 10.0]);
        assert_eq!((water_mask.order, water_mask.submit_sequence), (0, 1));
    }
    let mesh = &meshes.cutout;

    assert_eq!(atlas.width, 128);
    assert_eq!(atlas.height, 384);
    assert_eq!(mesh.cutout_faces, 216);
    assert_eq!(mesh.vertices.len(), 864);
    assert_eq!(mesh.indices.len(), 1296);
    for (index, (start, end)) in [(0, 216), (216, 504), (504, 648), (648, 864)]
        .into_iter()
        .enumerate()
    {
        let base = meshes.submissions[index * 2];
        assert!(mesh.vertices[start..end]
            .iter()
            .all(|vertex| vertex.light == base.light && vertex.overlay == base.overlay));
    }
    assert_close2(mesh.vertices[0].uv, [31.0 / 128.0, 0.0]);
    assert_close2(mesh.vertices[216].uv, [31.0 / 128.0, 64.0 / 384.0]);
    assert_close2(mesh.vertices[504].uv, [32.0 / 128.0, 192.0 / 384.0]);
    assert_close2(mesh.vertices[648].uv, [32.0 / 128.0, 256.0 / 384.0]);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(mesh);
    assert!(max[0] - min[0] > 9.0);
    assert!(max[2] - min[2] > 1.0);
}
