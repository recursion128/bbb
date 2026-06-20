use super::*;

mod armor_stand;
mod camel;
mod chicken;
mod cow;
mod enderman;
mod equine;
mod hoglin;
mod llama;
mod pig;
mod piglin;
mod ravager;
mod sheep;
mod skeleton;
mod slime;
mod spider;
mod wolf;
mod zombie;

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
fn zombie_texture_refs_match_vanilla_renderers() {
    assert_eq!(
        EntityModelKind::Zombie { baby: false }.model_key(),
        "zombie"
    );
    assert_eq!(
        EntityModelKind::Zombie { baby: false }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie/zombie.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Zombie { baby: true }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie/zombie_baby.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: false,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie/husk.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: true,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie/husk_baby.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: false,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie/drowned.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: true,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie/drowned_baby.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: false,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie_villager/zombie_villager.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: true,
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/zombie_villager/zombie_villager_baby.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Humanoid {
            family: HumanoidModelFamily::Zombie,
            baby: false,
        }
        .vanilla_texture_ref(),
        None
    );
}

#[test]
fn runtime_colored_mesh_excludes_texture_backed_entities() {
    let chicken = EntityModelInstance::chicken(303, [-2.0, 64.0, 0.0], 0.0, false);
    let sheep = EntityModelInstance::sheep(304, [0.0, 64.0, 0.0], 0.0, false);
    let wolf = EntityModelInstance::wolf(305, [2.0, 64.0, 0.0], 0.0, false);
    let boat = EntityModelInstance::boat(306, [4.0, 64.0, 0.0], 0.0, BoatModelFamily::Oak, true);
    let pig = EntityModelInstance::pig(
        307,
        [6.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Temperate,
        false,
    );
    let cow =
        EntityModelInstance::cow_variant(308, [8.0, 64.0, 0.0], 0.0, CowModelVariant::Warm, false);
    let player = EntityModelInstance::player(309, [10.0, 64.0, 0.0], 0.0, false);
    let creeper = EntityModelInstance::new(310, EntityModelKind::Creeper, [12.0, 64.0, 0.0], 0.0);
    let spider = EntityModelInstance::spider(311, [14.0, 64.0, 0.0], 0.0);
    let cave_spider = EntityModelInstance::cave_spider(312, [16.0, 64.0, 0.0], 0.0);
    let enderman = EntityModelInstance::enderman(313, [18.0, 64.0, 0.0], 0.0);
    let colored = entity_model_colored_runtime_mesh(&[
        chicken,
        sheep,
        wolf,
        boat,
        pig,
        cow,
        player,
        creeper,
        spider,
        cave_spider,
        enderman,
    ]);
    assert!(colored.vertices.is_empty());
    assert!(colored.indices.is_empty());
    let legacy_chicken_geometry_guard = entity_model_mesh(&[chicken]);
    assert!(!legacy_chicken_geometry_guard.vertices.is_empty());
    let legacy_geometry_guard = entity_model_mesh(&[sheep]);
    assert!(!legacy_geometry_guard.vertices.is_empty());
    let legacy_wolf_geometry_guard = entity_model_mesh(&[wolf]);
    assert!(!legacy_wolf_geometry_guard.vertices.is_empty());
    let legacy_boat_geometry_guard = entity_model_mesh(&[boat]);
    assert!(!legacy_boat_geometry_guard.vertices.is_empty());
    let legacy_pig_geometry_guard = entity_model_mesh(&[pig]);
    assert!(!legacy_pig_geometry_guard.vertices.is_empty());
    let legacy_cow_geometry_guard = entity_model_mesh(&[cow]);
    assert!(!legacy_cow_geometry_guard.vertices.is_empty());
    let legacy_player_geometry_guard = entity_model_mesh(&[player]);
    assert!(!legacy_player_geometry_guard.vertices.is_empty());
    let legacy_creeper_geometry_guard = entity_model_mesh(&[creeper]);
    assert!(!legacy_creeper_geometry_guard.vertices.is_empty());
    let legacy_spider_geometry_guard = entity_model_mesh(&[spider]);
    assert!(!legacy_spider_geometry_guard.vertices.is_empty());
    let legacy_cave_spider_geometry_guard = entity_model_mesh(&[cave_spider]);
    assert!(!legacy_cave_spider_geometry_guard.vertices.is_empty());
    let legacy_enderman_geometry_guard = entity_model_mesh(&[enderman]);
    assert!(!legacy_enderman_geometry_guard.vertices.is_empty());
}

#[test]
fn entity_textured_shader_samples_bound_texture_and_discards_alpha() {
    assert!(ENTITY_MODEL_TEXTURED_SHADER
        .contains("textureSample(entity_texture_atlas, entity_sampler, input.uv)"));
    assert!(ENTITY_MODEL_TEXTURED_SHADER.contains("discard"));
    assert_eq!(
        ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES,
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4]
    );
}

#[test]
fn entity_eyes_shader_samples_bound_texture_without_alpha_cutout() {
    assert!(ENTITY_MODEL_EYES_SHADER
        .contains("textureSample(entity_texture_atlas, entity_sampler, input.uv)"));
    assert!(!ENTITY_MODEL_EYES_SHADER.contains("discard"));
    assert_eq!(
        ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES,
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4]
    );
}

#[test]
fn goat_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(
        ADULT_GOAT_HEAD[2],
        ModelCubeDesc {
            min: [-0.5, -3.0, -14.0],
            size: [0.0, 7.0, 5.0],
            color: GOAT_BEARD,
        }
    );
    assert_eq!(ADULT_GOAT_PARTS.len(), 6);
    assert_part_tree(
        &ADULT_GOAT_PARTS[ADULT_GOAT_HEAD_INDEX],
        [1.0, 14.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_GOAT_HEAD.as_slice(),
        ADULT_GOAT_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_GOAT_HEAD_CHILDREN[ADULT_GOAT_LEFT_HORN_CHILD_INDEX],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_GOAT_LEFT_HORN.as_slice(),
    );
    assert_part(
        &ADULT_GOAT_HEAD_CHILDREN[ADULT_GOAT_RIGHT_HORN_CHILD_INDEX],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_GOAT_RIGHT_HORN.as_slice(),
    );
    assert_part(
        &ADULT_GOAT_HEAD_CHILDREN[2],
        [0.0, -8.0, -8.0],
        [0.9599, 0.0, 0.0],
        ADULT_GOAT_NOSE.as_slice(),
    );
    assert_part(
        &ADULT_GOAT_PARTS[1],
        [0.0, 24.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_GOAT_BODY.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &ADULT_GOAT_PARTS[2],
            [1.0, 14.0, 4.0],
            ADULT_GOAT_HIND_LEG.as_slice(),
        ),
        (
            &ADULT_GOAT_PARTS[3],
            [-3.0, 14.0, 4.0],
            ADULT_GOAT_HIND_LEG.as_slice(),
        ),
        (
            &ADULT_GOAT_PARTS[4],
            [1.0, 14.0, -6.0],
            ADULT_GOAT_FRONT_LEG.as_slice(),
        ),
        (
            &ADULT_GOAT_PARTS[5],
            [-3.0, 14.0, -6.0],
            ADULT_GOAT_FRONT_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }

    assert_eq!(BABY_GOAT_PARTS.len(), 6);
    for (part, expected_offset) in [
        (&BABY_GOAT_PARTS[0], [1.5, 19.5, 3.0]),
        (&BABY_GOAT_PARTS[1], [-1.5, 19.5, 3.0]),
        (&BABY_GOAT_PARTS[2], [-1.5, 19.5, -2.0]),
        (&BABY_GOAT_PARTS[3], [1.5, 19.5, -2.0]),
    ] {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_GOAT_LEG.as_slice(),
        );
    }
    assert_part(
        &BABY_GOAT_PARTS[4],
        [0.0, 17.8, 0.0],
        [0.0, 0.0, 0.0],
        BABY_GOAT_BODY.as_slice(),
    );
    assert_part_tree(
        &BABY_GOAT_PARTS[BABY_GOAT_HEAD_INDEX],
        [0.0, 15.5, -3.0],
        [0.4363, 0.0, 0.0],
        BABY_GOAT_HEAD.as_slice(),
        BABY_GOAT_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_GOAT_HEAD_CHILDREN[BABY_GOAT_RIGHT_HORN_CHILD_INDEX],
        [-1.5, -1.5, -1.0],
        [-0.3926991, 0.0, 0.0],
        BABY_GOAT_RIGHT_HORN.as_slice(),
    );
    assert_part(
        &BABY_GOAT_HEAD_CHILDREN[BABY_GOAT_LEFT_HORN_CHILD_INDEX],
        [-1.5, -1.5, -1.0],
        [-0.3926991, 0.0, 0.0],
        BABY_GOAT_LEFT_HORN.as_slice(),
    );
    assert_part(
        &BABY_GOAT_HEAD_CHILDREN[2],
        [-1.7, -2.3126, 0.1452],
        [0.0, -0.5236, 0.0],
        BABY_GOAT_RIGHT_EAR.as_slice(),
    );
    assert_part(
        &BABY_GOAT_HEAD_CHILDREN[3],
        [1.7, -2.3126, 0.1452],
        [0.0, 0.5236, 0.0],
        BABY_GOAT_LEFT_EAR.as_slice(),
    );
    assert_part(
        &BABY_GOAT_HEAD_CHILDREN[4],
        [0.0, -1.3126, -1.1548],
        [0.0, 0.0, 0.0],
        BABY_GOAT_HEAD_MAIN.as_slice(),
    );
}

#[test]
fn goat_meshes_use_vanilla_body_layers_and_horn_visibility() {
    let adult = entity_model_mesh(&[EntityModelInstance::goat(
        200,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
    )]);
    assert_eq!(adult.opaque_faces, 72);
    assert_eq!(adult.vertices.len(), 288);
    assert_eq!(adult.indices.len(), 432);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GOAT_HORN, 0.78)));

    let adult_left_horn_only = entity_model_mesh(&[EntityModelInstance::goat(
        201,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
    )]);
    assert_eq!(adult_left_horn_only.opaque_faces, 66);
    assert_eq!(adult_left_horn_only.vertices.len(), 264);
    assert_eq!(adult_left_horn_only.indices.len(), 396);

    let adult_no_horns = entity_model_mesh(&[EntityModelInstance::goat(
        202,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        false,
    )]);
    assert_eq!(adult_no_horns.opaque_faces, 60);
    assert!(!adult_no_horns
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GOAT_HORN, 0.78)));

    let baby = entity_model_mesh(&[EntityModelInstance::goat(
        203,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        true,
        true,
    )]);
    assert_eq!(baby.opaque_faces, 72);
    assert_eq!(baby.vertices.len(), 288);
    assert_eq!(baby.indices.len(), 432);

    let baby_no_horns = entity_model_mesh(&[EntityModelInstance::goat(
        204,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
        false,
    )]);
    assert_eq!(baby_no_horns.opaque_faces, 60);
    assert!(!baby_no_horns
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(GOAT_HORN, 0.78)));

    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert!(adult_max[1] > baby_max[1]);
    assert!(adult_min[2] < baby_min[2]);
}

#[test]
fn goat_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            false,
            "goat",
            EntityModelTextureRef {
                path: "textures/entity/goat/goat.png",
                size: [64, 64],
            },
        ),
        (
            true,
            "goat_baby",
            EntityModelTextureRef {
                path: "textures/entity/goat/goat_baby.png",
                size: [64, 64],
            },
        ),
    ];

    for (baby, model_key, texture) in cases {
        let kind = EntityModelKind::Goat {
            baby,
            left_horn: false,
            right_horn: true,
        };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn polar_bear_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(ADULT_POLAR_BEAR_PARTS.len(), 6);
    assert_part(
        &ADULT_POLAR_BEAR_PARTS[0],
        [0.0, 10.0, -16.0],
        [0.0, 0.0, 0.0],
        ADULT_POLAR_BEAR_HEAD.as_slice(),
    );
    assert_eq!(
        ADULT_POLAR_BEAR_HEAD[1],
        ModelCubeDesc {
            min: [-2.5, 1.0, -6.0],
            size: [5.0, 3.0, 3.0],
            color: POLAR_BEAR_WHITE,
        }
    );
    assert_part(
        &ADULT_POLAR_BEAR_PARTS[1],
        [-2.0, 9.0, 12.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_POLAR_BEAR_BODY.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &ADULT_POLAR_BEAR_PARTS[2],
            [-4.5, 14.0, 6.0],
            ADULT_POLAR_BEAR_HIND_LEG.as_slice(),
        ),
        (
            &ADULT_POLAR_BEAR_PARTS[3],
            [4.5, 14.0, 6.0],
            ADULT_POLAR_BEAR_HIND_LEG.as_slice(),
        ),
        (
            &ADULT_POLAR_BEAR_PARTS[4],
            [-3.5, 14.0, -8.0],
            ADULT_POLAR_BEAR_FRONT_LEG.as_slice(),
        ),
        (
            &ADULT_POLAR_BEAR_PARTS[5],
            [3.5, 14.0, -8.0],
            ADULT_POLAR_BEAR_FRONT_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }

    assert_eq!(BABY_POLAR_BEAR_PARTS.len(), 6);
    assert_part(
        &BABY_POLAR_BEAR_PARTS[0],
        [0.0, 17.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_POLAR_BEAR_BODY.as_slice(),
    );
    assert_part(
        &BABY_POLAR_BEAR_PARTS[1],
        [0.0, 18.625, -5.75],
        [0.0, 0.0, 0.0],
        BABY_POLAR_BEAR_HEAD.as_slice(),
    );
    assert_eq!(
        BABY_POLAR_BEAR_HEAD[1],
        ModelCubeDesc {
            min: [-2.0, 0.375, -6.25],
            size: [4.0, 2.0, 2.0],
            color: POLAR_BEAR_WHITE,
        }
    );
    for (part, expected_offset) in [
        (&BABY_POLAR_BEAR_PARTS[2], [-2.5, 21.5, 4.5]),
        (&BABY_POLAR_BEAR_PARTS[3], [2.5, 21.5, 4.5]),
        (&BABY_POLAR_BEAR_PARTS[4], [-2.5, 21.5, -4.5]),
        (&BABY_POLAR_BEAR_PARTS[5], [2.5, 21.5, -4.5]),
    ] {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_POLAR_BEAR_LEG.as_slice(),
        );
    }
}

#[test]
fn polar_bear_meshes_use_vanilla_body_layers() {
    let adult = entity_model_mesh(&[EntityModelInstance::polar_bear(
        210,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_eq!(adult.opaque_faces, 60);
    assert_eq!(adult.vertices.len(), 240);
    assert_eq!(adult.indices.len(), 360);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(POLAR_BEAR_WHITE, 0.78)));

    let baby = entity_model_mesh(&[EntityModelInstance::polar_bear(
        211,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    assert_eq!(baby.opaque_faces, 54);
    assert_eq!(baby.vertices.len(), 216);
    assert_eq!(baby.indices.len(), 324);

    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert!(adult_max[1] > baby_max[1]);
    assert!(adult_min[2] < baby_min[2]);
}

#[test]
fn polar_bear_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            false,
            "polar_bear",
            EntityModelTextureRef {
                path: "textures/entity/bear/polarbear.png",
                size: [128, 64],
            },
        ),
        (
            true,
            "polar_bear_baby",
            EntityModelTextureRef {
                path: "textures/entity/bear/polarbear_baby.png",
                size: [64, 64],
            },
        ),
    ];

    for (baby, model_key, texture) in cases {
        let kind = EntityModelKind::PolarBear { baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn villager_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ADULT_VILLAGER_HAT[0],
        ModelCubeDesc {
            min: [-4.51, -10.51, -4.51],
            size: [9.02, 11.02, 9.02],
            color: VILLAGER_ROBE,
        }
    );
    assert_eq!(
        ADULT_VILLAGER_JACKET[0],
        ModelCubeDesc {
            min: [-4.5, -0.5, -3.5],
            size: [9.0, 21.0, 7.0],
            color: VILLAGER_ROBE,
        }
    );
    assert_eq!(ADULT_VILLAGER_PARTS.len(), 5);
    assert_part_tree(
        &ADULT_VILLAGER_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_HEAD.as_slice(),
        ADULT_VILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part_tree(
        &ADULT_VILLAGER_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_HAT.as_slice(),
        ADULT_VILLAGER_HAT_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_HAT_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [-std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_VILLAGER_HAT_RIM.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_HEAD_CHILDREN[1],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_NOSE.as_slice(),
    );
    assert_part_tree(
        &ADULT_VILLAGER_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_BODY.as_slice(),
        ADULT_VILLAGER_BODY_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_BODY_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_JACKET.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_PARTS[2],
        [0.0, 3.0, -1.0],
        [-0.75, 0.0, 0.0],
        ADULT_VILLAGER_ARMS.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_PARTS[3],
        [-2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_LEG.as_slice(),
    );
    assert_part(
        &ADULT_VILLAGER_PARTS[4],
        [2.0, 12.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_VILLAGER_LEG.as_slice(),
    );
}

#[test]
fn villager_adult_model_mesh_uses_vanilla_scaled_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::villager(
        139,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.46875003, 64.00094, -0.46875006]);
    assert_close3(max, [0.46875003, 66.02301, 0.46875003]);

    let wandering_trader_mesh = entity_model_mesh(&[EntityModelInstance::wandering_trader(
        141,
        [0.0, 64.0, 0.0],
        0.0,
    )]);
    assert_eq!(wandering_trader_mesh.opaque_faces, mesh.opaque_faces);
    assert_eq!(wandering_trader_mesh.vertices, mesh.vertices);
    assert_eq!(wandering_trader_mesh.indices, mesh.indices);
}

#[test]
fn villager_baby_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        BABY_VILLAGER_RIGHT_HAND,
        [
            ModelCubeDesc {
                min: [-1.0, -2.4925, -1.8401],
                size: [2.0, 4.0, 2.0],
                color: VILLAGER_ROBE,
            },
            ModelCubeDesc {
                min: [5.0, -2.4925, -1.8401],
                size: [2.0, 4.0, 2.0],
                color: VILLAGER_ROBE,
            },
        ]
    );
    assert_eq!(
        BABY_VILLAGER_BB_MAIN[0],
        ModelCubeDesc {
            min: [-2.7, -8.2, -1.7],
            size: [4.4, 6.4, 3.4],
            color: VILLAGER_ROBE,
        }
    );
    assert_eq!(BABY_VILLAGER_PARTS.len(), 6);
    assert_part_tree(
        &BABY_VILLAGER_PARTS[0],
        [0.0, 17.5, 0.0],
        [0.0, 0.0, 0.0],
        &[],
        BABY_VILLAGER_ARMS_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_ARMS_CHILDREN[0],
        [-3.0, 1.4025, -0.9599],
        [-1.0472, 0.0, 0.0],
        BABY_VILLAGER_RIGHT_HAND.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_ARMS_CHILDREN[1],
        [0.0, 0.9024, -1.8175],
        [-1.0472, 0.0, 0.0],
        BABY_VILLAGER_MIDDLE_ARM.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[1],
        [-1.0, 21.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_LEG.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[2],
        [1.0, 21.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_LEG.as_slice(),
    );
    assert_part_tree(
        &BABY_VILLAGER_PARTS[3],
        [0.0, 16.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_HEAD.as_slice(),
        BABY_VILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_HEAD_CHILDREN[0],
        [0.0, -4.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_HAT.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_HEAD_CHILDREN[1],
        [0.0, -4.5, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_HAT_RIM.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_HEAD_CHILDREN[2],
        [0.0, -2.0, -4.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_NOSE.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[4],
        [0.0, 18.75, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_BODY.as_slice(),
    );
    assert_part(
        &BABY_VILLAGER_PARTS[5],
        [0.5, 24.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_VILLAGER_BB_MAIN.as_slice(),
    );
}

#[test]
fn villager_baby_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::villager(
        140,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.43750003, 64.001, -0.37500003]);
    assert_close3(max, [0.43750003, 65.01975, 0.37500003]);
}

#[test]
fn villager_and_wandering_trader_texture_refs_match_vanilla_renderers() {
    assert_eq!(
        EntityModelKind::Villager { baby: false }.model_key(),
        "villager"
    );
    assert_eq!(
        EntityModelKind::Villager { baby: false }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/villager/villager.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Villager { baby: true }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/villager/villager_baby.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::WanderingTrader.model_key(),
        "wandering_trader"
    );
    assert_eq!(
        EntityModelKind::WanderingTrader.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/wandering_trader/wandering_trader.png",
            size: [64, 64],
        })
    );
}

#[test]
fn creeper_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        CREEPER_HEAD[0],
        ModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            color: CREEPER_GREEN
        }
    );
    assert_eq!(
        CREEPER_BODY[0],
        ModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            color: CREEPER_GREEN
        }
    );
    assert_eq!(
        CREEPER_LEG[0],
        ModelCubeDesc {
            min: [-2.0, 0.0, -2.0],
            size: [4.0, 6.0, 4.0],
            color: CREEPER_GREEN
        }
    );

    assert_eq!(CREEPER_PARTS.len(), 6);
    assert_eq!(CREEPER_PARTS[0].pose.offset, [0.0, 6.0, 0.0]);
    assert_eq!(CREEPER_PARTS[0].cubes, CREEPER_HEAD.as_slice());
    assert_eq!(CREEPER_PARTS[1].pose.offset, [0.0, 6.0, 0.0]);
    assert_eq!(CREEPER_PARTS[1].cubes, CREEPER_BODY.as_slice());

    let leg_offsets = [
        [-2.0, 18.0, 4.0],
        [2.0, 18.0, 4.0],
        [-2.0, 18.0, -4.0],
        [2.0, 18.0, -4.0],
    ];
    for (part, expected_offset) in CREEPER_PARTS[2..].iter().zip(leg_offsets) {
        assert_eq!(part.pose.offset, expected_offset);
        assert_eq!(part.pose.rotation, [0.0, 0.0, 0.0]);
        assert_eq!(part.cubes, CREEPER_LEG.as_slice());
        assert!(part.children.is_empty());
    }
}

#[test]
fn creeper_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::new(
        50,
        EntityModelKind::Creeper,
        [0.0, 64.0, 0.0],
        0.0,
    )]);

    assert_eq!(mesh.opaque_faces, 36);
    assert_eq!(mesh.vertices.len(), 144);
    assert_eq!(mesh.indices.len(), 216);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.375]);
    assert_close3(max, [0.25, 65.626, 0.375]);
}

#[test]
fn creeper_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Creeper.model_key(), "creeper");
    assert_eq!(
        EntityModelKind::Creeper.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/creeper/creeper.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Temperate,
            baby: false
        }
        .vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/chicken/chicken_temperate.png",
            size: [64, 32],
        })
    );
}

#[test]
fn creeper_textured_layer_passes_match_vanilla_renderer_model_layer() {
    let passes = creeper_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::CreeperBase);
    assert_eq!(passes[0].model_layer, MODEL_LAYER_CREEPER);
    assert_eq!(passes[0].texture, CREEPER_TEXTURE_REF);
    assert_eq!(passes[0].parts, CREEPER_TEXTURED_PARTS.as_slice());
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (passes[0].collector_order, passes[0].submit_sequence),
        (0, 0)
    );
}

#[test]
fn creeper_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_CREEPER, "minecraft:creeper#main");
    assert_eq!(CREEPER_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        CREEPER_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-4.0, -8.0, -4.0],
            size: [8.0, 8.0, 8.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        CREEPER_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-4.0, 0.0, -2.0],
            size: [8.0, 12.0, 4.0],
            uv_size: [8.0, 12.0, 4.0],
            tex: [16.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(
        CREEPER_TEXTURED_LEG[0],
        TexturedModelCubeDesc {
            min: [-2.0, 0.0, -2.0],
            size: [4.0, 6.0, 4.0],
            uv_size: [4.0, 6.0, 4.0],
            tex: [0.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(CREEPER_TEXTURED_PARTS[0].pose, CREEPER_PARTS[0].pose);
    assert_eq!(CREEPER_TEXTURED_PARTS[1].pose, CREEPER_PARTS[1].pose);
    assert_eq!(CREEPER_TEXTURED_PARTS[5].pose, CREEPER_PARTS[5].pose);
}

#[test]
fn entity_texture_atlas_stitches_official_creeper_png_slot() {
    let (layout, rgba) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 32);
    assert_eq!(layout.entries.len(), 1);
    assert_eq!(
        layout.entries[0].texture.path,
        "textures/entity/creeper/creeper.png"
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
}

#[test]
fn creeper_textured_mesh_uses_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::new(
            910,
            EntityModelKind::Creeper,
            [0.0, 64.0, 0.0],
            0.0,
        )],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 36);
    assert_eq!(mesh.vertices.len(), 144);
    assert_eq!(mesh.indices.len(), 216);
    assert_close2(mesh.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.375]);
    assert_close3(max, [0.25, 65.626, 0.375]);
}

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
fn illager_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ILLAGER_HEAD[0],
        ModelCubeDesc {
            min: [-4.0, -10.0, -4.0],
            size: [8.0, 10.0, 8.0],
            color: ILLAGER_ROBE,
        }
    );
    assert_eq!(
        ILLAGER_HAT[0],
        ModelCubeDesc {
            min: [-4.45, -10.45, -4.45],
            size: [8.9, 12.9, 8.9],
            color: ILLAGER_HAT_COLOR,
        }
    );
    assert_eq!(
        ILLAGER_BODY[1],
        ModelCubeDesc {
            min: [-4.5, -0.5, -3.5],
            size: [9.0, 21.0, 7.0],
            color: ILLAGER_ROBE,
        }
    );

    assert_eq!(ILLAGER_SHARED_CROSSED_PARTS.len(), 5);
    assert_part_tree(
        &ILLAGER_SHARED_CROSSED_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_HEAD.as_slice(),
        ILLAGER_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ILLAGER_HEAD_CHILDREN[0],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_NOSE.as_slice(),
    );
    assert_part(
        &ILLAGER_SHARED_CROSSED_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_BODY.as_slice(),
    );
    assert_part_tree(
        &ILLAGER_SHARED_CROSSED_PARTS[2],
        [0.0, 3.0, -1.0],
        [-0.75, 0.0, 0.0],
        ILLAGER_CROSSED_ARMS.as_slice(),
        ILLAGER_CROSSED_ARM_CHILDREN.as_slice(),
    );
    assert_part(
        &ILLAGER_CROSSED_ARM_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_LEFT_SHOULDER.as_slice(),
    );

    assert_eq!(ILLAGER_SHARED_UNCROSSED_PARTS.len(), 6);
    assert_part(
        &ILLAGER_SHARED_UNCROSSED_PARTS[4],
        [-5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_RIGHT_ARM.as_slice(),
    );
    assert_part(
        &ILLAGER_SHARED_UNCROSSED_PARTS[5],
        [5.0, 2.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_LEFT_ARM.as_slice(),
    );

    assert_part_tree(
        &ILLAGER_ILLUSIONER_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_HEAD.as_slice(),
        ILLAGER_HEAD_WITH_HAT_CHILDREN.as_slice(),
    );
    assert_part(
        &ILLAGER_HEAD_WITH_HAT_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_HAT.as_slice(),
    );
    assert_part(
        &ILLAGER_HEAD_WITH_HAT_CHILDREN[1],
        [0.0, -2.0, 0.0],
        [0.0, 0.0, 0.0],
        ILLAGER_NOSE.as_slice(),
    );
}

#[test]
fn illager_model_meshes_use_vanilla_scaled_body_layer_geometry() {
    let evoker = entity_model_mesh(&[EntityModelInstance::illager(
        46,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Evoker,
    )]);
    assert_eq!(evoker.opaque_faces, 54);
    assert_eq!(evoker.vertices.len(), 216);
    assert_eq!(evoker.indices.len(), 324);
    let (evoker_min, evoker_max) = mesh_extents(&evoker);
    assert_close3(evoker_min, [-0.46875, 64.00094, -0.23437501]);
    assert_close3(evoker_max, [0.46875003, 65.993126, 0.3839772]);

    let illusioner = entity_model_mesh(&[EntityModelInstance::illager(
        68,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Illusioner,
    )]);
    assert_eq!(illusioner.opaque_faces, 60);
    assert_eq!(illusioner.vertices.len(), 240);
    assert_eq!(illusioner.indices.len(), 360);
    let (illusioner_min, illusioner_max) = mesh_extents(&illusioner);
    assert_close3(illusioner_min, [-0.46875, 64.00094, -0.26074222]);
    assert_close3(illusioner_max, [0.46875003, 66.01949, 0.3839772]);

    let pillager = entity_model_mesh(&[EntityModelInstance::illager(
        103,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Pillager,
    )]);
    assert_eq!(pillager.opaque_faces, 48);
    assert_eq!(pillager.vertices.len(), 192);
    assert_eq!(pillager.indices.len(), 288);
    let (pillager_min, pillager_max) = mesh_extents(&pillager);
    assert_close3(pillager_min, [-0.46875, 64.00094, -0.23437501]);
    assert_close3(pillager_max, [0.46875, 65.993126, 0.3515625]);

    let vindicator = entity_model_mesh(&[EntityModelInstance::illager(
        140,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Vindicator,
    )]);
    assert_eq!(vindicator.vertices, evoker.vertices);
    assert_eq!(vindicator.indices, evoker.indices);
}

#[test]
fn illager_texture_refs_match_vanilla_renderers() {
    let cases = [
        (
            IllagerModelFamily::Evoker,
            "evoker",
            EntityModelTextureRef {
                path: "textures/entity/illager/evoker.png",
                size: [64, 64],
            },
        ),
        (
            IllagerModelFamily::Illusioner,
            "illusioner",
            EntityModelTextureRef {
                path: "textures/entity/illager/illusioner.png",
                size: [64, 64],
            },
        ),
        (
            IllagerModelFamily::Pillager,
            "pillager",
            EntityModelTextureRef {
                path: "textures/entity/illager/pillager.png",
                size: [64, 64],
            },
        ),
        (
            IllagerModelFamily::Vindicator,
            "vindicator",
            EntityModelTextureRef {
                path: "textures/entity/illager/vindicator.png",
                size: [64, 64],
            },
        ),
    ];

    for (family, model_key, texture) in cases {
        let kind = EntityModelKind::Illager { family };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn entity_model_root_transform_rotates_instances_by_body_yaw() {
    let mesh = entity_model_mesh(&[EntityModelInstance::chicken(
        26,
        [10.0, 64.0, -3.0],
        90.0,
        false,
    )]);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [9.5, 64.001, -3.25]);
    assert_close3(max, [10.25, 64.9385, -2.75]);
}

#[test]
fn humanoid_model_families_emit_deterministic_non_empty_meshes() {
    for family in [
        HumanoidModelFamily::Player,
        HumanoidModelFamily::Zombie,
        HumanoidModelFamily::Skeleton,
        HumanoidModelFamily::Villager,
        HumanoidModelFamily::Illager,
        HumanoidModelFamily::ArmorStand,
    ] {
        let instance = EntityModelInstance::humanoid(1, [0.0, 64.0, 0.0], 0.0, family, false);
        let mesh = entity_model_mesh(&[instance]);
        let repeat = entity_model_mesh(&[instance]);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices, repeat.vertices);
        assert_eq!(mesh.indices, repeat.indices);
        let (min, max) = mesh_extents(&mesh);
        assert!(max[0] > min[0]);
        assert!(max[1] > min[1]);
        assert!(max[2] > min[2]);
    }
}

#[test]
fn quadruped_model_families_emit_deterministic_non_empty_meshes() {
    for family in [
        QuadrupedModelFamily::Pig,
        QuadrupedModelFamily::Cow,
        QuadrupedModelFamily::Sheep,
        QuadrupedModelFamily::Horse,
        QuadrupedModelFamily::Wolf,
    ] {
        let instance = EntityModelInstance::quadruped(1, [0.0, 64.0, 0.0], 0.0, family, false);
        let mesh = entity_model_mesh(&[instance]);
        let repeat = entity_model_mesh(&[instance]);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices, repeat.vertices);
        assert_eq!(mesh.indices, repeat.indices);
        let (min, max) = mesh_extents(&mesh);
        assert!(max[0] > min[0]);
        assert!(max[1] > min[1]);
        assert!(max[2] > min[2]);
    }
}

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
    assert_eq!(oak_boat[0].model_layer, MODEL_LAYER_OAK_BOAT);
    assert_eq!(oak_boat[0].texture, BOAT_OAK_TEXTURE_REF);
    assert_eq!(oak_boat[0].parts, BOAT_TEXTURED_PARTS.as_slice());
    assert_eq!(oak_boat[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (oak_boat[0].collector_order, oak_boat[0].submit_sequence),
        (0, 0)
    );

    let oak_chest_boat = boat_textured_layer_passes(BoatModelFamily::Oak, true);
    assert_eq!(oak_chest_boat[0].kind, EntityModelLayerKind::BoatBase);
    assert_eq!(oak_chest_boat[0].model_layer, MODEL_LAYER_OAK_CHEST_BOAT);
    assert_eq!(oak_chest_boat[0].texture, CHEST_BOAT_OAK_TEXTURE_REF);
    assert_eq!(
        oak_chest_boat[0].parts,
        BOAT_CHEST_TEXTURED_PARTS.as_slice()
    );

    let bamboo_raft = boat_textured_layer_passes(BoatModelFamily::Bamboo, false);
    assert_eq!(bamboo_raft[0].model_layer, MODEL_LAYER_BAMBOO_RAFT);
    assert_eq!(bamboo_raft[0].texture, BOAT_BAMBOO_TEXTURE_REF);
    assert_eq!(bamboo_raft[0].parts, RAFT_TEXTURED_PARTS.as_slice());

    let bamboo_chest_raft = boat_textured_layer_passes(BoatModelFamily::Bamboo, true);
    assert_eq!(
        bamboo_chest_raft[0].model_layer,
        MODEL_LAYER_BAMBOO_CHEST_RAFT
    );
    assert_eq!(bamboo_chest_raft[0].texture, CHEST_BOAT_BAMBOO_TEXTURE_REF);
    assert_eq!(
        bamboo_chest_raft[0].parts,
        RAFT_CHEST_TEXTURED_PARTS.as_slice()
    );
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
    let mesh = entity_model_textured_mesh(
        &[
            EntityModelInstance::boat(201, [0.0, 64.0, 0.0], 0.0, BoatModelFamily::Oak, false),
            EntityModelInstance::boat(202, [3.0, 64.0, 0.0], 0.0, BoatModelFamily::Oak, true),
            EntityModelInstance::boat(203, [6.0, 64.0, 0.0], 0.0, BoatModelFamily::Bamboo, false),
            EntityModelInstance::boat(204, [9.0, 64.0, 0.0], 0.0, BoatModelFamily::Bamboo, true),
        ],
        &atlas,
    );

    assert_eq!(atlas.width, 128);
    assert_eq!(atlas.height, 384);
    assert_eq!(mesh.cutout_faces, 216);
    assert_eq!(mesh.vertices.len(), 864);
    assert_eq!(mesh.indices.len(), 1296);
    assert_close2(mesh.vertices[0].uv, [31.0 / 128.0, 0.0]);
    assert_close2(mesh.vertices[216].uv, [31.0 / 128.0, 64.0 / 384.0]);
    assert_close2(mesh.vertices[504].uv, [32.0 / 128.0, 192.0 / 384.0]);
    assert_close2(mesh.vertices[648].uv, [32.0 / 128.0, 256.0 / 384.0]);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(&mesh);
    assert!(max[0] - min[0] > 9.0);
    assert!(max[2] - min[2] > 1.0);
}

#[test]
fn vehicle_and_placeholder_models_emit_sane_bounds() {
    let cases = [
        EntityModelInstance::new(1, EntityModelKind::Minecart, [0.0, 64.0, 0.0], 0.0),
        EntityModelInstance::new(
            2,
            EntityModelKind::Boat {
                family: BoatModelFamily::Oak,
                chest: true,
            },
            [3.0, 64.0, 0.0],
            0.0,
        ),
        EntityModelInstance::placeholder(
            3,
            [6.0, 64.0, 0.0],
            0.0,
            "todo_test_bounds",
            1.0,
            2.0,
            0.5,
        ),
    ];

    for instance in cases {
        let mesh = entity_model_mesh(&[instance]);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        let (min, max) = mesh_extents(&mesh);
        assert!(max[0] > min[0]);
        assert!(max[1] > min[1]);
        assert!(max[2] > min[2]);
    }
}

#[test]
fn entity_model_kind_exposes_stable_model_keys() {
    assert_eq!(
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Temperate,
            baby: false
        }
        .model_key(),
        "chicken_temperate"
    );
    assert_eq!(
        EntityModelKind::Pig {
            variant: PigModelVariant::Cold,
            baby: false
        }
        .model_key(),
        "pig_cold"
    );
    assert_eq!(
        EntityModelKind::Pig {
            variant: PigModelVariant::Warm,
            baby: true
        }
        .model_key(),
        "pig_warm_baby"
    );
    assert_eq!(
        EntityModelKind::Humanoid {
            family: HumanoidModelFamily::Zombie,
            baby: true
        }
        .model_key(),
        "humanoid_zombie_baby"
    );
    assert_eq!(
        EntityModelKind::ArmorStand {
            small: true,
            show_arms: true,
            show_base_plate: false,
            pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
        }
        .model_key(),
        "armor_stand_small"
    );
    assert_eq!(EntityModelKind::Slime { size: 4 }.model_key(), "slime");
    assert_eq!(
        EntityModelKind::MagmaCube { size: 3 }.model_key(),
        "magma_cube"
    );
    assert_eq!(
        EntityModelKind::Zombie { baby: true }.model_key(),
        "zombie_baby"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: false
        }
        .model_key(),
        "husk"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: true
        }
        .model_key(),
        "husk_baby"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: false
        }
        .model_key(),
        "drowned"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: true
        }
        .model_key(),
        "drowned_baby"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: false
        }
        .model_key(),
        "zombie_villager"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: true
        }
        .model_key(),
        "zombie_villager_baby"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: false
        }
        .model_key(),
        "piglin"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: true
        }
        .model_key(),
        "piglin_baby"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::PiglinBrute,
            baby: false
        }
        .model_key(),
        "piglin_brute"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: false
        }
        .model_key(),
        "zombified_piglin"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: true
        }
        .model_key(),
        "zombified_piglin_baby"
    );
    assert_eq!(EntityModelKind::Skeleton.model_key(), "skeleton");
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Stray
        }
        .model_key(),
        "stray"
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Parched
        }
        .model_key(),
        "parched"
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::WitherSkeleton
        }
        .model_key(),
        "wither_skeleton"
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Bogged { sheared: true }
        }
        .model_key(),
        "bogged"
    );
    assert_eq!(
        EntityModelKind::Cow {
            variant: CowModelVariant::Warm,
            baby: false
        }
        .model_key(),
        "cow_warm"
    );
    assert_eq!(
        EntityModelKind::Cow {
            variant: CowModelVariant::Cold,
            baby: true
        }
        .model_key(),
        "cow_cold_baby"
    );
    assert_eq!(
        EntityModelKind::Sheep {
            baby: true,
            sheared: false,
            wool_color: SheepWoolColor::White,
        }
        .model_key(),
        "sheep_baby"
    );
    assert_eq!(
        EntityModelKind::Villager { baby: true }.model_key(),
        "villager_baby"
    );
    assert_eq!(
        EntityModelKind::WanderingTrader.model_key(),
        "wandering_trader"
    );
    assert_eq!(
        EntityModelInstance::wolf(0, [0.0, 0.0, 0.0], 0.0, true)
            .kind
            .model_key(),
        "wolf_baby"
    );
    assert_eq!(
        EntityModelKind::Horse { baby: true }.model_key(),
        "horse_baby"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Donkey,
            baby: false,
            has_chest: false
        }
        .model_key(),
        "donkey"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Donkey,
            baby: true,
            has_chest: true
        }
        .model_key(),
        "donkey_baby"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Mule,
            baby: false,
            has_chest: false
        }
        .model_key(),
        "mule"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Mule,
            baby: true,
            has_chest: true
        }
        .model_key(),
        "mule_baby"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Skeleton,
            baby: false
        }
        .model_key(),
        "skeleton_horse"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Skeleton,
            baby: true
        }
        .model_key(),
        "skeleton_horse_baby"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Zombie,
            baby: false
        }
        .model_key(),
        "zombie_horse"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Zombie,
            baby: true
        }
        .model_key(),
        "zombie_horse_baby"
    );
    assert_eq!(
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: false
        }
        .model_key(),
        "camel"
    );
    assert_eq!(
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: true
        }
        .model_key(),
        "camel_baby"
    );
    assert_eq!(
        EntityModelKind::Camel {
            family: CamelModelFamily::CamelHusk,
            baby: true
        }
        .model_key(),
        "camel_husk"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant: LlamaVariant::Creamy,
            baby: false,
            has_chest: true
        }
        .model_key(),
        "llama_creamy"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant: LlamaVariant::White,
            baby: true,
            has_chest: false
        }
        .model_key(),
        "llama_white_baby"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::TraderLlama,
            variant: LlamaVariant::Brown,
            baby: false,
            has_chest: false
        }
        .model_key(),
        "trader_llama_brown"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::TraderLlama,
            variant: LlamaVariant::Gray,
            baby: true,
            has_chest: true
        }
        .model_key(),
        "trader_llama_gray_baby"
    );
    assert_eq!(
        EntityModelKind::Goat {
            baby: true,
            left_horn: false,
            right_horn: true
        }
        .model_key(),
        "goat_baby"
    );
    assert_eq!(EntityModelKind::Spider.model_key(), "spider");
    assert_eq!(EntityModelKind::CaveSpider.model_key(), "cave_spider");
    assert_eq!(EntityModelKind::Enderman.model_key(), "enderman");
    assert_eq!(EntityModelKind::IronGolem.model_key(), "iron_golem");
    assert_eq!(EntityModelKind::SnowGolem.model_key(), "snow_golem");
    assert_eq!(EntityModelKind::Witch.model_key(), "witch");
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Evoker
        }
        .model_key(),
        "evoker"
    );
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Illusioner
        }
        .model_key(),
        "illusioner"
    );
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Pillager
        }
        .model_key(),
        "pillager"
    );
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Vindicator
        }
        .model_key(),
        "vindicator"
    );
    assert_eq!(
        EntityModelKind::Placeholder {
            name: "todo_test_bounds",
            bounds: EntityModelBounds {
                width: 1.0,
                height: 1.0,
                depth: 1.0
            }
        }
        .model_key(),
        "todo_test_bounds"
    );
}

#[test]
fn sanitize_entity_model_instances_drops_non_finite_instances() {
    assert_eq!(
        sanitize_entity_model_instances(vec![
            EntityModelInstance::chicken(1, [0.0, 0.0, 0.0], 0.0, false),
            EntityModelInstance::chicken(2, [0.0, f32::NAN, 0.0], 0.0, false),
            EntityModelInstance::chicken(3, [0.0, 0.0, 0.0], f32::INFINITY, false),
        ]),
        vec![EntityModelInstance::chicken(1, [0.0, 0.0, 0.0], 0.0, false)]
    );
}

#[test]
fn entity_model_vertex_layout_matches_shader_inputs() {
    let layout = entity_model_vertex_layout();

    assert_eq!(
        layout.array_stride,
        std::mem::size_of::<EntityModelVertex>() as wgpu::BufferAddress
    );
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES.len(), 2);
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[0].shader_location, 0);
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[1].shader_location, 1);
}

fn mesh_extents(mesh: &EntityModelMesh) -> ([f32; 3], [f32; 3]) {
    let mut vertices = mesh.vertices.iter();
    let first = vertices.next().expect("mesh has vertices").position;
    let mut min = Vec3::from_array(first);
    let mut max = Vec3::from_array(first);
    for vertex in vertices {
        let position = Vec3::from_array(vertex.position);
        min = min.min(position);
        max = max.max(position);
    }
    (min.to_array(), max.to_array())
}

fn textured_mesh_extents(mesh: &EntityModelTexturedMesh) -> ([f32; 3], [f32; 3]) {
    let mut vertices = mesh.vertices.iter();
    let first = vertices.next().expect("mesh has vertices").position;
    let mut min = Vec3::from_array(first);
    let mut max = Vec3::from_array(first);
    for vertex in vertices {
        let position = Vec3::from_array(vertex.position);
        min = min.min(position);
        max = max.max(position);
    }
    (min.to_array(), max.to_array())
}

fn assert_close3(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 1.0e-4,
            "expected {expected}, got {actual}"
        );
    }
}

fn assert_close2(actual: [f32; 2], expected: [f32; 2]) {
    for (actual, expected) in actual.iter().copied().zip(expected.iter().copied()) {
        assert!(
            (actual - expected).abs() < 1.0e-4,
            "expected {expected}, got {actual}"
        );
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

fn creeper_texture_images() -> Vec<EntityModelTextureImage> {
    creeper_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn spider_texture_images() -> Vec<EntityModelTextureImage> {
    spider_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn assert_same_geometry(actual: &EntityModelMesh, expected: &EntityModelMesh) {
    assert_eq!(actual.opaque_faces, expected.opaque_faces);
    assert_eq!(actual.indices, expected.indices);
    assert_eq!(actual.vertices.len(), expected.vertices.len());
    for (actual, expected) in actual.vertices.iter().zip(expected.vertices.iter()) {
        assert_eq!(actual.position, expected.position);
    }
}

fn assert_part(
    part: &ModelPartDesc,
    offset: [f32; 3],
    rotation: [f32; 3],
    cubes: &[ModelCubeDesc],
) {
    assert_eq!(part.pose.offset, offset);
    assert_eq!(part.pose.rotation, rotation);
    assert_eq!(part.cubes, cubes);
    assert!(part.children.is_empty());
}

fn assert_part_tree(
    part: &ModelPartDesc,
    offset: [f32; 3],
    rotation: [f32; 3],
    cubes: &[ModelCubeDesc],
    children: &[ModelPartDesc],
) {
    assert_eq!(part.pose.offset, offset);
    assert_eq!(part.pose.rotation, rotation);
    assert_eq!(part.cubes, cubes);
    assert_eq!(part.children, children);
}
