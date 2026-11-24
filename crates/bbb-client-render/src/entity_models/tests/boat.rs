use super::*;

use crate::entity_models::textured::boat_model_layer;
use crate::entity_models::transforms::boat_model_root_transform;

#[test]
fn boat_textured_layer_passes_match_vanilla_renderer_model_layers() {
    let oak_boat = boat_textured_layer_passes(BoatModelFamily::Oak, false);
    assert_eq!(oak_boat.len(), 2);
    assert_eq!(
        oak_boat[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(oak_boat[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(oak_boat[0].texture, BOAT_OAK_TEXTURE_REF);
    assert_eq!(oak_boat[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(oak_boat[0].order, 0);
    assert_eq!(
        oak_boat[1].render_type,
        EntityModelLayerRenderType::WaterMask
    );
    assert_eq!(oak_boat[1].render_type.vanilla_name(), "waterMask");
    assert_eq!(oak_boat[1].texture, BOAT_OAK_TEXTURE_REF);
    assert_eq!(oak_boat[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(oak_boat[1].order, 0);

    let oak_chest_boat = boat_textured_layer_passes(BoatModelFamily::Oak, true);
    assert_eq!(oak_chest_boat[0].texture, CHEST_BOAT_OAK_TEXTURE_REF);
    assert_eq!(oak_chest_boat[1].texture, CHEST_BOAT_OAK_TEXTURE_REF);

    let bamboo_raft = boat_textured_layer_passes(BoatModelFamily::Bamboo, false);
    assert_eq!(bamboo_raft.len(), 1);
    assert_eq!(bamboo_raft[0].texture, BOAT_BAMBOO_TEXTURE_REF);

    let bamboo_chest_raft = boat_textured_layer_passes(BoatModelFamily::Bamboo, true);
    assert_eq!(bamboo_chest_raft.len(), 1);
    assert_eq!(bamboo_chest_raft[0].texture, CHEST_BOAT_BAMBOO_TEXTURE_REF);
}

#[test]
fn entity_texture_atlas_stitches_official_boat_png_slots() {
    let images = texture_refs_matching(|path| {
        path.starts_with("textures/entity/boat/") || path.starts_with("textures/entity/chest_boat/")
    })
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
        instance.with(|s| {
            s.light_coords = (7_u32 << 4) | (12_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        })
    });
    let meshes = entity_model_textured_meshes(&instances, &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());

    let water_mask_mesh = entity_model_water_mask_mesh(&instances);
    assert_eq!(
        water_mask_mesh.vertices.len(),
        48,
        "two wooden boats submit one vanilla BOAT_WATER_PATCH cube each"
    );
    assert_eq!(water_mask_mesh.indices.len(), 72);
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
        assert!(mesh.vertices[start..end].iter().all(|vertex| vertex.light
            == instances[index].render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]));
    }
    for ((start, family, chest), uv) in [
        (0, BoatModelFamily::Oak, false),
        (216, BoatModelFamily::Oak, true),
        (504, BoatModelFamily::Bamboo, false),
        (648, BoatModelFamily::Bamboo, true),
    ]
    .into_iter()
    .zip([
        [31.0 / 128.0, 0.0],
        [31.0 / 128.0, 64.0 / 384.0],
        [32.0 / 128.0, 192.0 / 384.0],
        [32.0 / 128.0, 256.0 / 384.0],
    ]) {
        let bottom = boat_model_layer(family, chest)
            .bake()
            .part_vertex_range("bottom")
            .unwrap()
            .start;
        assert_close2(mesh.vertices[start + bottom].uv, uv);
    }
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(mesh);
    assert!(max[0] - min[0] > 9.0);
    assert!(max[2] - min[2] > 1.0);
}

#[test]
fn boat_underwater_skips_water_mask_submission_and_bubble_roll() {
    let image = EntityModelTextureImage::new(
        BOAT_OAK_TEXTURE_REF,
        vec![
            7;
            usize::try_from(BOAT_OAK_TEXTURE_REF.size[0] * BOAT_OAK_TEXTURE_REF.size[1] * 4)
                .unwrap()
        ],
    );
    let (atlas, _) = build_entity_model_texture_atlas(&[image]).unwrap();
    let underwater =
        EntityModelInstance::boat(205, [0.0, 64.0, 0.0], 30.0, BoatModelFamily::Oak, false).with(
            |s| {
                s.kind.boat_mut().bubble_angle = 6.0;
                s.kind.boat_mut().underwater = true;
                s.light_coords = (3_u32 << 4) | (11_u32 << 20);
                s.has_red_overlay = true;
                s.white_overlay_progress = 0.4;
            },
        );

    let meshes = entity_model_textured_meshes(&[underwater], &atlas);

    assert_eq!(meshes.cutout.vertices.len(), 216);
    let water_mask_mesh = entity_model_water_mask_mesh(&[underwater]);
    assert!(water_mask_mesh.vertices.is_empty());
    assert!(water_mask_mesh.indices.is_empty());
}

#[test]
fn boat_paddles_use_vanilla_rowing_time_rotations() {
    let assert_close = |actual: f32, expected: f32| {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "expected {expected}, got {actual}"
        );
    };
    let left_rest = boat_paddle_rotations(0.0, 0);
    assert_close(left_rest[0], -5.0 * std::f32::consts::PI / 24.0);

    let left_mid = boat_paddle_rotations(1.0, 0);
    let right_mid = boat_paddle_rotations(1.0, 1);
    assert_close(left_mid[1], 0.0);
    assert_close(right_mid[0], left_mid[0]);
    assert_close(right_mid[1], std::f32::consts::PI);

    let image = EntityModelTextureImage::new(
        BOAT_OAK_TEXTURE_REF,
        vec![
            7;
            usize::try_from(BOAT_OAK_TEXTURE_REF.size[0] * BOAT_OAK_TEXTURE_REF.size[1] * 4)
                .unwrap()
        ],
    );
    let (atlas, _) = build_entity_model_texture_atlas(&[image]).unwrap();
    let rest = EntityModelInstance::boat(301, [0.0, 64.0, 0.0], 0.0, BoatModelFamily::Oak, false)
        .with(|s| {
            s.light_coords = (5_u32 << 4) | (9_u32 << 20);
            s.has_red_overlay = true;
            s.white_overlay_progress = 0.6;
        });
    let rowing = rest.with(|s| {
        s.kind.boat_mut().rowing_time_left = std::f32::consts::PI / 8.0;
        s.kind.boat_mut().rowing_time_right = std::f32::consts::PI / 4.0;
    });

    let rest_meshes = entity_model_textured_meshes(&[rest], &atlas);
    let rowing_meshes = entity_model_textured_meshes(&[rowing], &atlas);

    let rest_water_mask = entity_model_water_mask_mesh(&[rest]);
    let rowing_water_mask = entity_model_water_mask_mesh(&[rowing]);
    assert_eq!(rest_water_mask.vertices.len(), 24);
    assert_eq!(rowing_water_mask.vertices.len(), 24);
    assert_eq!(
        rest_water_mask.vertices, rowing_water_mask.vertices,
        "BoatModel.createWaterPatch is independent from paddle animation"
    );

    assert_eq!(rest_meshes.cutout.vertices.len(), 216);
    assert_eq!(rowing_meshes.cutout.vertices.len(), 216);
    let model = boat_model_layer(BoatModelFamily::Oak, false).bake();
    let hull = ["bottom", "back", "front", "right", "left"];
    assert_eq!(
        parts_vertices(&rest_meshes.cutout.vertices, &model, &hull),
        parts_vertices(&rowing_meshes.cutout.vertices, &model, &hull)
    );
    for paddle in ["left_paddle", "right_paddle"] {
        assert_ne!(
            parts_vertices(&rest_meshes.cutout.vertices, &model, &[paddle]),
            parts_vertices(&rowing_meshes.cutout.vertices, &model, &[paddle])
        );
    }
}

#[test]
fn boat_water_mask_geometry_survives_missing_base_texture_atlas_entry() {
    let image = EntityModelTextureImage::new(
        ZOMBIE_TEXTURE_REF,
        vec![
            3;
            usize::try_from(ZOMBIE_TEXTURE_REF.size[0] * ZOMBIE_TEXTURE_REF.size[1] * 4).unwrap()
        ],
    );
    let (atlas, _) = build_entity_model_texture_atlas(&[image]).unwrap();
    let instance =
        EntityModelInstance::boat(302, [0.0, 64.0, 0.0], 0.0, BoatModelFamily::Oak, false);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.cutout.indices.is_empty());
    let water_mask_mesh = entity_model_water_mask_mesh(&[instance]);
    assert_eq!(
        water_mask_mesh.vertices.len(),
        24,
        "RenderTypes.waterMask has no texture sampler, so missing boat.png only suppresses the base"
    );
    assert_eq!(water_mask_mesh.indices.len(), 36);
}

#[test]
fn boat_damage_and_bubble_roll_use_vanilla_root_transform_order() {
    let assert_close = |actual: f32, expected: f32| {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "expected {expected}, got {actual}"
        );
    };
    let assert_close_transform = |actual: Mat4, expected: Mat4| {
        for (actual, expected) in actual
            .to_cols_array()
            .into_iter()
            .zip(expected.to_cols_array())
        {
            assert!(
                (actual - expected).abs() < 1.0e-5,
                "expected {expected}, got {actual}"
            );
        }
    };

    let damaged =
        EntityModelInstance::boat(302, [1.0, 64.0, -2.0], 35.0, BoatModelFamily::Oak, false).with(
            |s| {
                s.kind.boat_mut().hurt.time = 7.5;
                s.kind.boat_mut().hurt.dir = -1;
                s.kind.boat_mut().hurt.damage_time = 17.5;
            },
        );
    let roll = -(7.5_f32.sin() * 7.5 * 17.5 / 10.0);
    assert_close(boat_damage_roll_degrees(&damaged), roll);

    let expected = Mat4::from_translation(Vec3::from_array(damaged.position))
        * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
        * Mat4::from_rotation_y((180.0_f32 - damaged.render_state.body_rot).to_radians())
        * Mat4::from_rotation_x(roll.to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2);
    assert_close_transform(boat_model_root_transform(&damaged), expected);

    let bubbling = damaged.with(|s| s.kind.boat_mut().bubble_angle = 6.0);
    let expected_bubbling = Mat4::from_translation(Vec3::from_array(bubbling.position))
        * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
        * Mat4::from_rotation_y((180.0_f32 - bubbling.render_state.body_rot).to_radians())
        * Mat4::from_rotation_x(roll.to_radians())
        * boat_bubble_transform(&bubbling)
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2);
    assert_close_transform(boat_model_root_transform(&bubbling), expected_bubbling);

    let underwater = bubbling.with(|s| s.kind.boat_mut().underwater = true);
    assert_close_transform(boat_bubble_transform(&underwater), Mat4::IDENTITY);
    assert_close_transform(boat_model_root_transform(&underwater), expected);

    let settled = damaged.with(|s| s.kind.boat_mut().hurt.time = 0.0);
    assert_close(boat_damage_roll_degrees(&settled), 0.0);
}
