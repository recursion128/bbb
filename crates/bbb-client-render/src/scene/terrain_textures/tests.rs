use super::*;

#[test]
fn terrain_generation_retains_the_selected_mipmap_limit_for_reload() {
    let disabled = TerrainTextureState::default().with_max_mipmap_levels(0);
    let fast = TerrainTextureState::default().with_max_mipmap_levels(2);
    let clamped = TerrainTextureState::default().with_max_mipmap_levels(99);

    assert_eq!(disabled.max_mipmap_levels(), 0);
    assert_eq!(fast.max_mipmap_levels(), 2);
    assert_eq!(clamped.max_mipmap_levels(), 4);
}

#[test]
fn mod_block_content_registry_extends_terrain_table_and_breaking_particles() {
    let mut builder = bbb_protocol::content_registry::ContentRegistryBuilder::new();
    builder.register_block(bbb_protocol::content_registry::ModBlockRegistration {
        resource_id: "examplemod:ruby_ore".to_string(),
        state_count: 1,
        light_emission: 12,
        destroy_speed: 3.0,
        full_cube_collision: true,
        dynamic_light_rgb: None,
    });
    let registry = Arc::new(builder.freeze().unwrap());
    let block = registry
        .block_by_resource_id("examplemod:ruby_ore")
        .expect("mod block registered");

    let images = vec![
        sprite("minecraft:block/stone"),
        sprite("examplemod:block/ruby_ore"),
    ];
    let atlas = bbb_client_assets::AtlasPacker::new(16, 1)
        .unwrap()
        .stitch(&images)
        .unwrap();
    let mut textures = TerrainTextureState::from_layout(&atlas.layout, None, None, None);
    let prev_gen = textures.generation();
    assert!(textures.set_content_registry(Arc::clone(&registry), None));
    assert_ne!(textures.generation(), prev_gen);

    assert_eq!(
        textures
            .particle_sprite_id_for_block_state(block.first_state_id)
            .as_deref(),
        Some("examplemod:block/ruby_ore")
    );

    let context = bbb_render_types::TerrainBlockRenderContext::new([], |_| None)
        .with_content_registry(Arc::clone(&registry));
    let source = TerrainResourceSource::new(&textures, context);
    let table = source.build_table();
    assert_eq!(
        table.states.len(),
        registry.total_block_state_count() as usize
    );
    let mod_state = &table.states[block.first_state_id as usize];
    assert!(mod_state.solid_render);
    assert!(mod_state.collision_full_block);
    assert_eq!(mod_state.state_light_emission, 12);
}

#[test]
fn terrain_generation_retains_the_selected_biome_blend_radius_for_reload() {
    let fast = TerrainTextureState::default().with_biome_blend_radius(1);
    let maximum = TerrainTextureState::default().with_biome_blend_radius(99);

    assert_eq!(fast.biome_blend_radius(), 1);
    assert_eq!(maximum.biome_blend_radius(), 7);
}

#[test]
fn fluid_height_units_follow_vanilla_legacy_level_amounts() {
    assert_eq!(fluid_height_units(0), 14);
    assert_eq!(fluid_height_units(1), 12);
    assert_eq!(fluid_height_units(2), 11);
    assert_eq!(fluid_height_units(3), 9);
    assert_eq!(fluid_height_units(4), 7);
    assert_eq!(fluid_height_units(5), 5);
    assert_eq!(fluid_height_units(6), 4);
    assert_eq!(fluid_height_units(7), 2);
    assert_eq!(fluid_height_units(8), 14);
    assert_eq!(fluid_height_units(15), 14);
}

#[test]
fn water_level_shape_uses_cropped_fluid_box() {
    let shape = fluid_render_shape("minecraft:water", &properties([("level", "3")]))
        .expect("water has a fluid render shape");

    assert_eq!(
        shape,
        TerrainRenderShape::Box {
            from: [0, 0, 0],
            to: [16, 9, 16],
            face_present: [true; 6],
            face_uvs: [
                [0, 0, 16, 16],
                [0, 0, 16, 16],
                [0, 7, 16, 16],
                [0, 7, 16, 16],
                [0, 7, 16, 16],
                [0, 7, 16, 16],
            ],
            face_uv_rotations: [0; 6],
            face_shade: [true; 6],
            face_light_emission: [0; 6],
            face_cull: all_terrain_face_cull(),
            face_transparency: [TerrainTransparency::OPAQUE; 6],
        }
    );
}

#[test]
fn fluid_render_data_uses_still_top_and_flowing_sides() {
    let images = vec![
        sprite("minecraft:block/water_still"),
        sprite("minecraft:block/water_flow"),
        sprite("minecraft:block/water_overlay"),
        sprite("minecraft:block/lava_still"),
        sprite("minecraft:block/lava_flow"),
    ];
    let atlas = bbb_client_assets::AtlasPacker::new(16, 1)
        .unwrap()
        .stitch(&images)
        .unwrap();
    let textures = TerrainTextureState::from_layout(&atlas.layout, None, None, None);

    let (water, _, _, _, _) = textures.block_render_data(
        Some("minecraft:water"),
        &properties([("level", "0")]),
        bbb_render_types::TerrainMaterialClass::Fluid,
        None,
        None,
        None,
    );
    let water_still = textures.texture_index("minecraft:block/water_still");
    let water_flow = textures.texture_index("minecraft:block/water_flow");
    let water_overlay = textures.texture_index("minecraft:block/water_overlay");
    assert_eq!(water[0], water_still);
    assert_eq!(water[1], water_still);
    assert_eq!(water[2..], [water_flow; 4]);
    let water_fluid = fluid_state(&textures, TerrainFluidKind::Water);
    assert_eq!(water_fluid.texture_indices, water);
    assert_eq!(water_fluid.overlay_texture_index, Some(water_overlay));
    assert_eq!(
        textures.resolve_tint(
            TerrainTintRule::for_block("minecraft:water"),
            None,
            None,
            None
        ),
        TerrainTint::from_rgb_u8(0x3f, 0x76, 0xe4)
    );

    let (lava, _, _, _, _) = textures.block_render_data(
        Some("minecraft:lava"),
        &properties([("level", "0")]),
        bbb_render_types::TerrainMaterialClass::Fluid,
        None,
        None,
        None,
    );
    let lava_still = textures.texture_index("minecraft:block/lava_still");
    let lava_flow = textures.texture_index("minecraft:block/lava_flow");
    assert_eq!(lava[0], lava_still);
    assert_eq!(lava[1], lava_still);
    assert_eq!(lava[2..], [lava_flow; 4]);
    let lava_fluid = fluid_state(&textures, TerrainFluidKind::Lava);
    assert_eq!(lava_fluid.texture_indices, lava);
    assert_eq!(lava_fluid.overlay_texture_index, None);
    assert_eq!(
        textures.resolve_tint(
            TerrainTintRule::for_block("minecraft:lava"),
            None,
            None,
            None
        ),
        TerrainTint::WHITE
    );
}

fn fluid_state(
    textures: &TerrainTextureState,
    kind: TerrainFluidKind,
) -> state_render::TerrainStateFluid {
    textures
        .state_render(
            None,
            &BTreeMap::new(),
            bbb_render_types::TerrainMaterialClass::Fluid,
            Some(kind),
            false,
        )
        .fluid
        .expect("fluid state carries fluid textures")
}

#[test]
fn destroy_stage_uv_rect_uses_block_atlas_stage_sprite() {
    let images = vec![
        sprite("minecraft:block/stone"),
        sprite("minecraft:block/destroy_stage_3"),
    ];
    let atlas = bbb_client_assets::AtlasPacker::new(16, 1)
        .unwrap()
        .stitch(&images)
        .unwrap();
    let textures = TerrainTextureState::from_layout(&atlas.layout, None, None, None);
    let destroy_stage_index = textures.texture_index("minecraft:block/destroy_stage_3");

    assert_eq!(
        textures.destroy_stage_uv_rect(3),
        textures
            .atlas
            .rects
            .get(destroy_stage_index as usize)
            .copied()
    );
    assert_eq!(textures.destroy_stage_uv_rect(4), None);
}

#[test]
fn terrain_particle_sprite_uvs_use_block_atlas_content_rects() {
    let images = vec![
        sprite("minecraft:block/stone"),
        sprite("minecraft:block/oak_planks"),
    ];
    let atlas = bbb_client_assets::AtlasPacker::new(64, 1)
        .unwrap()
        .stitch(&images)
        .unwrap();
    let particle_uvs = terrain_particle_sprite_uvs(&atlas.layout);
    let oak_index = atlas
        .layout
        .sprites
        .iter()
        .position(|sprite| sprite.id == "minecraft:block/oak_planks")
        .unwrap();
    let terrain_uv = terrain_uv_rect(&atlas.layout, &atlas.layout.sprites[oak_index]);
    let oak_particle_uv = &particle_uvs[oak_index];

    assert_eq!(
        oak_particle_uv.uv,
        ParticleUvRect {
            min: terrain_uv.min,
            max: terrain_uv.max,
        }
    );
}

#[test]
fn terrain_uv_rect_uses_vanilla_sprite_content_edges_without_half_texel_shrink() {
    let glass = bbb_client_assets::SpriteImage::new(
        "minecraft:block/glass",
        16,
        16,
        [255, 255, 255, 255].repeat(16 * 16),
    )
    .unwrap();
    let atlas = bbb_client_assets::AtlasPacker::new(64, 1)
        .unwrap()
        .stitch_mips(&[glass], 4)
        .unwrap();
    let sprite = &atlas.layout.sprites[0];

    assert_eq!(sprite.content.width, 16);
    assert_eq!(sprite.content.height, 16);
    assert_eq!(atlas.layout.padding, 16);
    assert_eq!(
        terrain_uv_rect(&atlas.layout, sprite),
        TerrainUvRect {
            min: [
                sprite.content.x as f32 / atlas.layout.width as f32,
                sprite.content.y as f32 / atlas.layout.height as f32,
            ],
            max: [
                (sprite.content.x + 16) as f32 / atlas.layout.width as f32,
                (sprite.content.y + 16) as f32 / atlas.layout.height as f32,
            ],
        }
    );
}

#[test]
fn terrain_particle_sprite_id_uses_block_model_particle_texture() {
    let root = unique_temp_dir("terrain-particle-sprite-id");
    let asset_root = root
        .join("sources")
        .join(bbb_client_assets::MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("water.json"),
        r##"{
            "variants": {
                "level=0": { "model": "minecraft:block/water" }
            }
        }"##,
    );
    write_json(
        &asset_root.join("models").join("block").join("water.json"),
        r##"{
            "textures": {
                "particle": "minecraft:block/water_still"
            }
        }"##,
    );
    let images = vec![
        sprite("minecraft:block/stone"),
        sprite("minecraft:block/water_still"),
    ];
    let atlas = bbb_client_assets::AtlasPacker::new(64, 1)
        .unwrap()
        .stitch(&images)
        .unwrap();
    let block_models = bbb_client_assets::PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let textures = TerrainTextureState::from_layout(&atlas.layout, Some(block_models), None, None);
    let water_id = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:water",
        &properties([("level", "0")]),
    )
    .unwrap();

    assert_eq!(
        textures
            .particle_sprite_id_for_block_state(water_id)
            .as_deref(),
        Some("minecraft:block/water_still")
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_model_seed_matches_vanilla_position_seed() {
    assert_eq!(
        block_model_seed(BlockRenderPosition { x: 0, y: 0, z: 0 }),
        0
    );
    assert_eq!(
        block_model_seed(BlockRenderPosition { x: 1, y: 2, z: 3 }),
        -33_674_130_277_896
    );
    assert_eq!(
        block_model_seed(BlockRenderPosition { x: -5, y: 64, z: 7 }),
        100_748_096_222_042
    );
    assert_eq!(
        block_model_seed(BlockRenderPosition {
            x: 16,
            y: -64,
            z: -32,
        }),
        100_430_790_140_967
    );
}

#[test]
fn moving_block_quads_use_start_seed_and_cardinal_context() {
    let root = unique_temp_dir("moving-block-start-seed");
    let asset_root = root
        .join("sources")
        .join(bbb_client_assets::MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("test_block.json"),
        r##"{
            "variants": {
                "": [
                    { "model": "minecraft:block/red_model", "weight": 2 },
                    { "model": "minecraft:block/blue_model" }
                ]
            }
        }"##,
    );
    write_json(
        &asset_root.join("blockstates").join("oak_leaves.json"),
        r##"{
            "variants": {
                "": [
                    { "model": "minecraft:block/red_model", "weight": 2 },
                    { "model": "minecraft:block/blue_model" }
                ]
            }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("cube_all.json"),
        r##"{
            "textures": { "particle": "#all" },
            "elements": [{
                "from": [0, 0, 0],
                "to": [16, 16, 16],
                "faces": {
                    "down": { "texture": "#all", "cullface": "down" },
                    "up": { "texture": "#all", "cullface": "up" },
                    "north": { "texture": "#all", "cullface": "north" },
                    "south": { "texture": "#all", "cullface": "south" },
                    "west": { "texture": "#all", "cullface": "west" },
                    "east": { "texture": "#all", "cullface": "east" }
                }
            }]
        }"##,
    );
    for (model, texture) in [("red_model", "red"), ("blue_model", "blue")] {
        write_json(
            &asset_root
                .join("models")
                .join("block")
                .join(format!("{model}.json")),
            &format!(
                r##"{{
                    "parent": "minecraft:block/cube_all",
                    "textures": {{ "all": "minecraft:block/{texture}" }}
                }}"##
            ),
        );
    }
    let images = vec![
        bbb_client_assets::SpriteImage::new("minecraft:block/red", 1, 1, vec![255, 255, 255, 127])
            .unwrap(),
        sprite("minecraft:block/blue"),
    ];
    let atlas = bbb_client_assets::AtlasPacker::new(64, 1)
        .unwrap()
        .stitch(&images)
        .unwrap();
    let block_models = bbb_client_assets::PackRoots::from_root(&root)
        .unwrap()
        .load_block_model_catalog()
        .unwrap();
    let textures = TerrainTextureState::from_layout(&atlas.layout, Some(block_models), None, None);
    assert!(textures.block_model_has_translucent_material(
        "minecraft:test_block",
        &BTreeMap::new(),
        bbb_render_types::TerrainMaterialClass::Opaque,
    ));
    let block_position = BlockRenderPosition { x: 20, y: 70, z: 4 };
    let red = textures
        .moving_block_quads(
            "minecraft:test_block",
            &BTreeMap::new(),
            BlockRenderPosition { x: 0, y: 0, z: 0 },
            block_position,
            None,
            crate::terrain::TerrainCardinalLighting::Nether,
        )
        .unwrap();
    let red_rect = textures.atlas.rects[textures.texture_index("minecraft:block/red") as usize];
    assert_eq!(red.len(), 6);
    assert!(red.iter().all(|quad| quad.translucent));
    assert!(red[0].uvs.iter().all(|uv| {
        (red_rect.min[0]..=red_rect.max[0]).contains(&uv[0])
            && (red_rect.min[1]..=red_rect.max[1]).contains(&uv[1])
    }));
    assert_eq!(red[0].shade, 0.9, "Nether down-face shade");

    let blue_rect = textures.atlas.rects[textures.texture_index("minecraft:block/blue") as usize];
    let blue = (1..=64)
        .find_map(|x| {
            let quads = textures.moving_block_quads(
                "minecraft:test_block",
                &BTreeMap::new(),
                BlockRenderPosition { x, y: 0, z: 0 },
                block_position,
                None,
                crate::terrain::TerrainCardinalLighting::Default,
            )?;
            quads[0]
                .uvs
                .iter()
                .all(|uv| {
                    (blue_rect.min[0]..=blue_rect.max[0]).contains(&uv[0])
                        && (blue_rect.min[1]..=blue_rect.max[1]).contains(&uv[1])
                })
                .then_some(quads)
        })
        .expect("a start-position seed selects the blue weighted model");
    assert_eq!(blue.len(), 6);

    let fast_leaves = textures.clone().with_cutout_leaves(false);
    let leaves = fast_leaves
        .moving_block_quads(
            "minecraft:oak_leaves",
            &BTreeMap::new(),
            BlockRenderPosition { x: 0, y: 0, z: 0 },
            block_position,
            None,
            crate::terrain::TerrainCardinalLighting::Default,
        )
        .unwrap();
    assert!(leaves.iter().all(|quad| !quad.translucent));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires local vanilla 26.1 resources"]
fn local_vanilla_block_material_flags_distinguish_stone_and_glass() {
    let roots =
        bbb_client_assets::PackRoots::from_environment().expect("discover local Vanilla resources");
    let images = roots
        .load_block_texture_images()
        .expect("load local Vanilla block textures");
    let models = roots
        .load_block_model_catalog()
        .expect("load local Vanilla block models");
    let atlas = bbb_client_assets::AtlasPacker::new(VANILLA_26_1_BLOCK_ATLAS_WIDTH, 1)
        .expect("create Vanilla block atlas packer")
        .stitch_mips_with_max_level(&images, 4)
        .expect("stitch local Vanilla block atlas");
    let textures = TerrainTextureState::from_layout(&atlas.layout, Some(models), None, None);

    assert!(!textures.block_model_has_translucent_material(
        "minecraft:stone",
        &BTreeMap::new(),
        bbb_render_types::TerrainMaterialClass::Opaque,
    ));
    assert!(textures.block_model_has_translucent_material(
        "minecraft:glass",
        &BTreeMap::new(),
        bbb_render_types::TerrainMaterialClass::Translucent,
    ));
}

#[test]
fn fast_leaves_force_root_and_nested_model_transparency_opaque() {
    let textures = TerrainTextureState::default().with_cutout_leaves(false);
    let mut shape = TerrainRenderShape::Quads(std::sync::Arc::from(vec![TerrainQuad {
        corners: [[0.0; 3]; 4],
        normal: [0.0, 1.0, 0.0],
        uvs: [[0.0; 2]; 4],
        cull: None,
        texture_index: 0,
        tint: TerrainTint::WHITE,
        transparency: TerrainTransparency::TRANSLUCENT,
        shade: true,
        light_emission: 0,
    }]));

    assert!(textures.forces_opaque_leaves("minecraft:oak_leaves"));
    shape.force_opaque();

    let TerrainRenderShape::Quads(quads) = shape else {
        panic!("expected quad model");
    };
    assert_eq!(quads[0].transparency, TerrainTransparency::OPAQUE);
}

#[test]
fn texture_alpha_maps_to_face_transparency() {
    let cutout =
        bbb_client_assets::SpriteImage::new("minecraft:block/cutout", 1, 1, vec![1, 2, 3, 0])
            .unwrap();
    let translucent = bbb_client_assets::SpriteImage::new(
        "minecraft:block/translucent",
        1,
        1,
        vec![1, 2, 3, 127],
    )
    .unwrap();
    let atlas = bbb_client_assets::AtlasPacker::new(8, 1)
        .unwrap()
        .stitch(&[cutout, translucent])
        .unwrap();
    let textures = TerrainTextureState::from_layout(&atlas.layout, None, None, None);
    let mut force_translucent = [false; 6];
    force_translucent[bbb_client_assets::BlockModelFace::East.index()] = true;
    let face_textures = bbb_client_assets::BlockFaceTextures {
        textures: std::array::from_fn(|index| {
            if index == bbb_client_assets::BlockModelFace::North.index() {
                "minecraft:block/cutout".to_string()
            } else {
                "minecraft:block/translucent".to_string()
            }
        }),
        tint_indices: [None; 6],
        force_translucent,
    };

    let transparencies = textures.face_texture_transparencies(&face_textures);

    assert!(transparencies[bbb_client_assets::BlockModelFace::North.index()].has_transparent);
    assert!(!transparencies[bbb_client_assets::BlockModelFace::North.index()].has_translucent);
    assert!(!transparencies[bbb_client_assets::BlockModelFace::Up.index()].has_transparent);
    assert!(transparencies[bbb_client_assets::BlockModelFace::Up.index()].has_translucent);
    assert!(transparencies[bbb_client_assets::BlockModelFace::East.index()].has_translucent);
}

#[test]
fn box_face_transparency_uses_model_uv_crop() {
    let image = bbb_client_assets::SpriteImage::new(
        "minecraft:block/cropped",
        2,
        2,
        vec![10, 0, 0, 255, 20, 0, 0, 0, 30, 0, 0, 255, 40, 0, 0, 255],
    )
    .unwrap();
    let atlas = bbb_client_assets::AtlasPacker::new(8, 1)
        .unwrap()
        .stitch(std::slice::from_ref(&image))
        .unwrap();
    let textures =
        TerrainTextureState::from_layout_and_images(&atlas.layout, &[image], None, None, None);
    let north = bbb_client_assets::BlockModelFace::North.index();
    let mut opaque_crop = block_model_box_with_face_texture(
        bbb_client_assets::BlockModelFace::North,
        "minecraft:block/cropped",
        None,
    );
    opaque_crop.face_uvs[north] = [0, 0, 8, 8];
    let mut transparent_crop = opaque_crop.clone();
    transparent_crop.face_uvs[north] = [8, 0, 16, 8];

    let opaque_shape = textures.terrain_render_shape_for_block(
        "minecraft:test_block",
        &BTreeMap::new(),
        bbb_render_types::TerrainMaterialClass::Opaque,
        BlockModelShape::Box(opaque_crop),
        [0; 6],
        [TerrainTint::WHITE; 6],
        [TerrainTransparency::OPAQUE; 6],
        &|_| TerrainTint::WHITE,
    );
    let transparent_shape = textures.terrain_render_shape_for_block(
        "minecraft:test_block",
        &BTreeMap::new(),
        bbb_render_types::TerrainMaterialClass::Opaque,
        BlockModelShape::Box(transparent_crop),
        [0; 6],
        [TerrainTint::WHITE; 6],
        [TerrainTransparency::OPAQUE; 6],
        &|_| TerrainTint::WHITE,
    );

    let TerrainRenderShape::Box {
        face_transparency: opaque_faces,
        ..
    } = opaque_shape
    else {
        panic!("expected opaque crop box");
    };
    let TerrainRenderShape::Box {
        face_transparency: transparent_faces,
        ..
    } = transparent_shape
    else {
        panic!("expected transparent crop box");
    };
    assert!(!opaque_faces[north].has_transparent);
    assert!(!opaque_faces[north].has_translucent);
    assert!(transparent_faces[north].has_transparent);
    assert!(!transparent_faces[north].has_translucent);
}

#[test]
fn box_face_transparency_uses_all_animation_frames_for_model_uv_crop() {
    let frame_zero = vec![10, 0, 0, 255, 20, 0, 0, 255, 30, 0, 0, 255, 40, 0, 0, 255];
    let frame_one = vec![10, 0, 0, 255, 20, 0, 0, 0, 30, 0, 0, 255, 40, 0, 0, 255];
    let mut image = bbb_client_assets::SpriteImage::new(
        "minecraft:block/animated_crop",
        2,
        2,
        frame_zero.clone(),
    )
    .unwrap();
    image.animation = Some(bbb_client_assets::SpriteAnimation {
        frame_count: 2,
        default_frame_time: 1,
        interpolate: false,
        frames: vec![
            bbb_client_assets::SpriteAnimationFrame { index: 0, time: 1 },
            bbb_client_assets::SpriteAnimationFrame { index: 1, time: 1 },
        ],
    });
    image.animation_frames_rgba = vec![frame_zero, frame_one];
    let atlas = bbb_client_assets::AtlasPacker::new(8, 1)
        .unwrap()
        .stitch(std::slice::from_ref(&image))
        .unwrap();
    let textures =
        TerrainTextureState::from_layout_and_images(&atlas.layout, &[image], None, None, None);
    let north = bbb_client_assets::BlockModelFace::North.index();
    let mut animated_crop = block_model_box_with_face_texture(
        bbb_client_assets::BlockModelFace::North,
        "minecraft:block/animated_crop",
        None,
    );
    animated_crop.face_uvs[north] = [8, 0, 16, 8];

    let shape = textures.terrain_render_shape_for_block(
        "minecraft:test_block",
        &BTreeMap::new(),
        bbb_render_types::TerrainMaterialClass::Opaque,
        BlockModelShape::Box(animated_crop),
        [0; 6],
        [TerrainTint::WHITE; 6],
        [TerrainTransparency::OPAQUE; 6],
        &|_| TerrainTint::WHITE,
    );

    let TerrainRenderShape::Box {
        face_transparency, ..
    } = shape
    else {
        panic!("expected animated crop box");
    };
    assert!(face_transparency[north].has_transparent);
    assert!(!face_transparency[north].has_translucent);
}

#[test]
fn fluid_material_overrides_particle_only_model_shape() {
    let textures = TerrainTextureState::default();
    let shape = textures.terrain_render_shape_for_block(
        "minecraft:lava",
        &properties([("level", "8")]),
        bbb_render_types::TerrainMaterialClass::Fluid,
        BlockModelShape::Boxes(Vec::new()),
        [0; 6],
        [TerrainTint::WHITE; 6],
        [TerrainTransparency::OPAQUE; 6],
        &|_| TerrainTint::WHITE,
    );

    assert!(matches!(
        shape,
        TerrainRenderShape::Box {
            to: [16, 14, 16],
            ..
        }
    ));

    let non_fluid_shape = textures.terrain_render_shape_for_block(
        "minecraft:lava",
        &properties([("level", "8")]),
        bbb_render_types::TerrainMaterialClass::Opaque,
        BlockModelShape::Boxes(Vec::new()),
        [0; 6],
        [TerrainTint::WHITE; 6],
        [TerrainTransparency::OPAQUE; 6],
        &|_| TerrainTint::WHITE,
    );
    assert!(matches!(non_fluid_shape, TerrainRenderShape::Boxes(boxes) if boxes.is_empty()));
}

#[test]
fn model_boxes_preserve_per_element_textures_and_tints() {
    let mut texture_state = TerrainTextureState::default();
    std::sync::Arc::make_mut(&mut texture_state.core)
        .indices
        .insert("minecraft:block/base".to_string(), 1);
    std::sync::Arc::make_mut(&mut texture_state.core)
        .indices
        .insert("minecraft:block/overlay".to_string(), 2);
    let base = block_model_box_with_face_texture(
        bbb_client_assets::BlockModelFace::North,
        "minecraft:block/base",
        None,
    );
    let mut overlay = block_model_box_with_face_texture(
        bbb_client_assets::BlockModelFace::North,
        "minecraft:block/overlay",
        Some(0),
    );
    overlay.face_shade[bbb_client_assets::BlockModelFace::North.index()] = false;
    overlay.face_force_translucent[bbb_client_assets::BlockModelFace::North.index()] = true;

    let shape = texture_state.terrain_render_shape_for_block(
        "minecraft:grass_block",
        &BTreeMap::new(),
        bbb_render_types::TerrainMaterialClass::Opaque,
        BlockModelShape::Boxes(vec![base, overlay]),
        [0; 6],
        [TerrainTint::WHITE; 6],
        [TerrainTransparency::OPAQUE; 6],
        &|tint_index| {
            texture_state.block_tint(
                "minecraft:grass_block",
                bbb_render_types::TerrainMaterialClass::Opaque,
                tint_index,
                Some(4),
                None,
            )
        },
    );

    let TerrainRenderShape::Boxes(boxes) = shape else {
        panic!("expected boxes render shape");
    };
    let north = bbb_client_assets::BlockModelFace::North.index();
    assert_eq!(boxes[0].texture_indices[north], 1);
    assert_eq!(boxes[0].tint[north], TerrainTint::WHITE);
    assert_eq!(boxes[1].texture_indices[north], 2);
    assert!(!boxes[1].face_shade[north]);
    assert!(boxes[1].face_transparency[north].has_translucent);
    assert_eq!(
        boxes[1].tint[north],
        TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59)
    );
}

#[test]
fn model_crosses_preserve_per_layer_textures_tints_and_light() {
    let mut texture_state = TerrainTextureState::default();
    std::sync::Arc::make_mut(&mut texture_state.core)
        .indices
        .insert("minecraft:block/base".to_string(), 1);
    std::sync::Arc::make_mut(&mut texture_state.core)
        .indices
        .insert("minecraft:block/emissive".to_string(), 2);
    let north = bbb_client_assets::BlockModelFace::North.index();
    let east = bbb_client_assets::BlockModelFace::East.index();
    let mut base_textures: [Option<String>; 6] = std::array::from_fn(|_| None);
    base_textures[north] = Some("minecraft:block/base".to_string());
    let mut emissive_textures: [Option<String>; 6] = std::array::from_fn(|_| None);
    emissive_textures[east] = Some("minecraft:block/emissive".to_string());
    let mut emissive_tints = [None; 6];
    emissive_tints[east] = Some(0);
    let mut emissive_force_translucent = [false; 6];
    emissive_force_translucent[east] = true;

    let shape = texture_state.terrain_render_shape_for_block(
        "minecraft:grass_block",
        &BTreeMap::new(),
        bbb_render_types::TerrainMaterialClass::Opaque,
        BlockModelShape::Crosses(vec![
            bbb_client_assets::BlockModelCross {
                face_textures: base_textures,
                face_tint_indices: [None; 6],
                face_force_translucent: [false; 6],
                shade: false,
                light_emission: 0,
            },
            bbb_client_assets::BlockModelCross {
                face_textures: emissive_textures,
                face_tint_indices: emissive_tints,
                face_force_translucent: emissive_force_translucent,
                shade: false,
                light_emission: 15,
            },
        ]),
        [0; 6],
        [TerrainTint::WHITE; 6],
        [TerrainTransparency::OPAQUE; 6],
        &|tint_index| {
            texture_state.block_tint(
                "minecraft:grass_block",
                bbb_render_types::TerrainMaterialClass::Opaque,
                tint_index,
                Some(4),
                None,
            )
        },
    );

    let TerrainRenderShape::Crosses(crosses) = shape else {
        panic!("expected cross render layers");
    };
    assert_eq!(crosses.len(), 2);
    assert_eq!(crosses[0].texture_indices[north], 1);
    assert_eq!(crosses[0].light_emission, 0);
    assert_eq!(crosses[1].texture_indices[east], 2);
    assert_eq!(crosses[1].light_emission, 15);
    assert!(crosses[1].face_transparency[east].has_translucent);
    assert_eq!(
        crosses[1].tint[east],
        TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59)
    );
}

#[test]
fn model_quads_preserve_texture_tint_transparency_and_light() {
    let mut texture_state = TerrainTextureState::default();
    std::sync::Arc::make_mut(&mut texture_state.core)
        .indices
        .insert("minecraft:block/rotated".to_string(), 7);

    let shape = texture_state.terrain_render_shape_for_block(
        "minecraft:grass_block",
        &BTreeMap::new(),
        bbb_render_types::TerrainMaterialClass::Opaque,
        BlockModelShape::Quads(vec![bbb_client_assets::BlockModelQuad {
            face: bbb_client_assets::BlockModelFace::North,
            corners: [
                [0.0, 0.0, 0.0],
                [16.0, 0.0, 0.0],
                [16.0, 16.0, 0.0],
                [0.0, 16.0, 0.0],
            ],
            normal: [0.0, 0.0, -1.0],
            uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            cull: Some(bbb_client_assets::BlockModelFace::North),
            tint_index: Some(0),
            texture: Some("minecraft:block/rotated".to_string()),
            force_translucent: true,
            shade: false,
            element_rotated: false,
            light_emission: 12,
        }]),
        [0; 6],
        [TerrainTint::WHITE; 6],
        [TerrainTransparency::OPAQUE; 6],
        &|tint_index| {
            texture_state.block_tint(
                "minecraft:grass_block",
                bbb_render_types::TerrainMaterialClass::Opaque,
                tint_index,
                Some(4),
                None,
            )
        },
    );

    let TerrainRenderShape::Quads(quads) = shape else {
        panic!("expected quad render shape");
    };
    assert_eq!(quads.len(), 1);
    assert_eq!(quads[0].texture_index, 7);
    assert_eq!(quads[0].cull, Some(TerrainFace::North));
    assert_eq!(quads[0].tint, TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59));
    assert!(quads[0].transparency.has_translucent);
    assert!(!quads[0].shade);
    assert_eq!(quads[0].light_emission, 12);
}

#[test]
fn block_render_data_preserves_model_ambient_occlusion() {
    let root = unique_temp_dir("block-render-ambient-occlusion");
    let asset_root = root
        .join("sources")
        .join(bbb_client_assets::MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("test_block.json"),
        r##"{
            "variants": {
                "": { "model": "minecraft:block/test_model" }
            }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("test_model.json"),
        r##"{
            "ambientocclusion": false,
            "textures": {
                "particle": "minecraft:block/stone",
                "all": "minecraft:block/stone"
            },
            "elements": [{
                "faces": {
                    "down": { "texture": "#all" },
                    "up": { "texture": "#all" },
                    "north": { "texture": "#all" },
                    "south": { "texture": "#all" },
                    "west": { "texture": "#all" },
                    "east": { "texture": "#all" }
                }
            }]
        }"##,
    );

    let mut texture_state = TerrainTextureState::default();
    std::sync::Arc::make_mut(&mut texture_state.core).block_models = Some(
        bbb_client_assets::PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog()
            .unwrap(),
    );

    let (_, _, _, _, ambient_occlusion) = texture_state.block_render_data(
        Some("minecraft:test_block"),
        &BTreeMap::new(),
        bbb_render_types::TerrainMaterialClass::Opaque,
        None,
        None,
        None,
    );

    assert!(!ambient_occlusion);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn smooth_lighting_off_disables_every_model_ambient_occlusion() {
    // Vanilla `Options.ao` builds `ModelBlockRenderer` with the option, so
    // smooth lighting off flattens even models that request AO.
    let root = unique_temp_dir("block-render-smooth-lighting");
    let asset_root = root
        .join("sources")
        .join(bbb_client_assets::MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("test_block.json"),
        r##"{
            "variants": {
                "": { "model": "minecraft:block/test_model" }
            }
        }"##,
    );
    write_json(
        &asset_root
            .join("models")
            .join("block")
            .join("test_model.json"),
        r##"{
            "textures": {
                "particle": "minecraft:block/stone",
                "all": "minecraft:block/stone"
            },
            "elements": [{
                "faces": {
                    "down": { "texture": "#all" },
                    "up": { "texture": "#all" },
                    "north": { "texture": "#all" },
                    "south": { "texture": "#all" },
                    "west": { "texture": "#all" },
                    "east": { "texture": "#all" }
                }
            }]
        }"##,
    );

    let mut texture_state = TerrainTextureState::default();
    std::sync::Arc::make_mut(&mut texture_state.core).block_models = Some(
        bbb_client_assets::PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog()
            .unwrap(),
    );

    let render_data = |texture_state: &TerrainTextureState| {
        let (_, _, _, _, ambient_occlusion) = texture_state.block_render_data(
            Some("minecraft:test_block"),
            &BTreeMap::new(),
            bbb_render_types::TerrainMaterialClass::Opaque,
            None,
            None,
            None,
        );
        ambient_occlusion
    };
    assert!(render_data(&texture_state));
    let texture_state = texture_state.with_smooth_lighting(false);
    assert!(!render_data(&texture_state));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_render_data_preserves_empty_multipart_geometry() {
    let root = unique_temp_dir("block-render-empty-multipart");
    let asset_root = root
        .join("sources")
        .join(bbb_client_assets::MC_VERSION)
        .join("assets")
        .join("minecraft");
    write_json(
        &asset_root.join("blockstates").join("cobblestone_wall.json"),
        r##"{
            "multipart": [
                {
                    "when": { "up": "true" },
                    "apply": { "model": "minecraft:block/template_wall_post" }
                },
                {
                    "when": { "north": "low" },
                    "apply": { "model": "minecraft:block/template_wall_side" }
                }
            ]
        }"##,
    );
    std::fs::create_dir_all(asset_root.join("models").join("block")).unwrap();

    let mut texture_state = TerrainTextureState::default();
    std::sync::Arc::make_mut(&mut texture_state.core).block_models = Some(
        bbb_client_assets::PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog()
            .unwrap(),
    );

    let (_, _, _, shape, _) = texture_state.block_render_data(
        Some("minecraft:cobblestone_wall"),
        &properties([("north", "none"), ("up", "false")]),
        bbb_render_types::TerrainMaterialClass::Opaque,
        None,
        None,
        None,
    );

    let TerrainRenderShape::Boxes(boxes) = shape else {
        panic!("empty multipart geometry must not fall back to a cube");
    };
    assert!(boxes.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_tint_uses_default_vanilla_color_classes() {
    let textures = TerrainTextureState::default();
    assert_eq!(
        textures.block_tint(
            "minecraft:stone",
            bbb_render_types::TerrainMaterialClass::Opaque,
            None,
            None,
            None
        ),
        TerrainTint::WHITE
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:grass_block",
            bbb_render_types::TerrainMaterialClass::Opaque,
            Some(0),
            None,
            None
        ),
        TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59)
    );
    for block_name in ["minecraft:pink_petals", "minecraft:wildflowers"] {
        assert_eq!(
            textures.block_tint(
                block_name,
                bbb_render_types::TerrainMaterialClass::Opaque,
                Some(1),
                None,
                None
            ),
            TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59),
            "{block_name}"
        );
    }
    for block_name in [
        "minecraft:cave_vines",
        "minecraft:weeping_vines",
        "minecraft:twisting_vines",
        "minecraft:divine_block",
    ] {
        assert_eq!(
            textures.block_tint(
                block_name,
                bbb_render_types::TerrainMaterialClass::Opaque,
                Some(0),
                None,
                None
            ),
            TerrainTint::WHITE,
            "{block_name}"
        );
    }
    assert_eq!(
        textures.block_tint(
            "minecraft:oak_leaves",
            bbb_render_types::TerrainMaterialClass::Cutout,
            Some(0),
            None,
            None
        ),
        TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:spruce_leaves",
            bbb_render_types::TerrainMaterialClass::Cutout,
            Some(0),
            None,
            None
        ),
        TerrainTint::from_rgb_u8(0x61, 0x99, 0x61)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:birch_leaves",
            bbb_render_types::TerrainMaterialClass::Cutout,
            Some(0),
            None,
            None
        ),
        TerrainTint::from_rgb_u8(0x80, 0xa7, 0x55)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:leaf_litter",
            bbb_render_types::TerrainMaterialClass::Cutout,
            Some(0),
            None,
            None
        ),
        TerrainTint::from_rgb_u8(0x5c, 0x3c, 0x32)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:water",
            bbb_render_types::TerrainMaterialClass::Fluid,
            None,
            None,
            None
        ),
        TerrainTint::from_rgb_u8(0x3f, 0x76, 0xe4)
    );
}

#[test]
fn block_tint_samples_loaded_colormaps() {
    let textures = TerrainTextureState {
        colormaps: Some(TerrainColorMaps {
            grass: flat_colormap([10, 20, 30]),
            foliage: flat_colormap([40, 50, 60]),
            dry_foliage: Some(flat_colormap([70, 80, 90])),
        }),
        ..TerrainTextureState::default()
    };

    assert_eq!(
        textures.block_tint(
            "minecraft:grass_block",
            bbb_render_types::TerrainMaterialClass::Opaque,
            Some(0),
            Some(4),
            None
        ),
        TerrainTint::from_rgb_u8(10, 20, 30)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:oak_leaves",
            bbb_render_types::TerrainMaterialClass::Cutout,
            Some(0),
            Some(4),
            None
        ),
        TerrainTint::from_rgb_u8(40, 50, 60)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:leaf_litter",
            bbb_render_types::TerrainMaterialClass::Cutout,
            Some(0),
            Some(4),
            None
        ),
        TerrainTint::from_rgb_u8(70, 80, 90)
    );
}

#[test]
fn block_tint_uses_loaded_biome_color_profiles() {
    let textures = TerrainTextureState {
        colormaps: Some(TerrainColorMaps {
            grass: flat_colormap([10, 20, 30]),
            foliage: flat_colormap([40, 50, 60]),
            dry_foliage: Some(flat_colormap([70, 80, 90])),
        }),
        biome_colors: Some(BiomeColorCatalog::new([BiomeColorProfile {
            id: 42,
            name: "minecraft:test_biome".to_string(),
            temperature: 0.2,
            temperature_modifier: bbb_client_assets::BiomeTemperatureModifier::None,
            downfall: 0.3,
            has_precipitation: true,
            grass_color: Some([1, 2, 3]),
            foliage_color: Some([4, 5, 6]),
            dry_foliage_color: Some([7, 8, 9]),
            water_color: Some([10, 11, 12]),
            grass_color_modifier: GrassColorModifier::None,
        }])),
        ..TerrainTextureState::default()
    };

    assert_eq!(
        textures.block_tint(
            "minecraft:grass_block",
            bbb_render_types::TerrainMaterialClass::Opaque,
            Some(0),
            Some(42),
            Some(BlockRenderPosition { x: 0, y: 0, z: 0 })
        ),
        TerrainTint::from_rgb_u8(1, 2, 3)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:oak_leaves",
            bbb_render_types::TerrainMaterialClass::Cutout,
            Some(0),
            Some(42),
            None
        ),
        TerrainTint::from_rgb_u8(4, 5, 6)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:leaf_litter",
            bbb_render_types::TerrainMaterialClass::Cutout,
            Some(0),
            Some(42),
            None
        ),
        TerrainTint::from_rgb_u8(7, 8, 9)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:water",
            bbb_render_types::TerrainMaterialClass::Fluid,
            None,
            Some(42),
            None
        ),
        TerrainTint::from_rgb_u8(10, 11, 12)
    );
}

#[test]
fn biome_climate_changes_colormap_sample() {
    let textures = TerrainTextureState {
        colormaps: Some(TerrainColorMaps {
            grass: coordinate_colormap(),
            foliage: flat_colormap([40, 50, 60]),
            dry_foliage: Some(flat_colormap([70, 80, 90])),
        }),
        biome_colors: Some(BiomeColorCatalog::new([BiomeColorProfile {
            id: 7,
            name: "minecraft:dry_biome".to_string(),
            temperature: 0.0,
            temperature_modifier: bbb_client_assets::BiomeTemperatureModifier::None,
            downfall: 1.0,
            has_precipitation: false,
            grass_color: None,
            foliage_color: None,
            dry_foliage_color: None,
            water_color: None,
            grass_color_modifier: GrassColorModifier::None,
        }])),
        ..TerrainTextureState::default()
    };

    assert_eq!(
        textures.block_tint(
            "minecraft:grass_block",
            bbb_render_types::TerrainMaterialClass::Opaque,
            Some(0),
            Some(7),
            None
        ),
        TerrainTint::from_rgb_u8(30, 60, 6)
    );
}

#[test]
fn particle_tint_uses_vanilla_block_colors_layer_zero() {
    let textures = TerrainTextureState::default();
    let grass_block = block_state_id("minecraft:grass_block", [("snowy", "false")]);
    let redstone = block_state_id(
        "minecraft:redstone_wire",
        [
            ("east", "up"),
            ("north", "up"),
            ("power", "15"),
            ("south", "up"),
            ("west", "up"),
        ],
    );
    let stem = block_state_id("minecraft:melon_stem", [("age", "7")]);
    let lily_pad = block_state_id("minecraft:lily_pad", []);

    assert_eq!(
        textures.terrain_particle_tint_color_for_block_state(grass_block),
        Some(rgb_06(255, 255, 255))
    );
    assert_eq!(
        textures.terrain_particle_tint_color_for_block_state(redstone),
        Some(rgb_06(255, 50, 0))
    );
    assert_eq!(
        textures.terrain_particle_tint_color_for_block_state(stem),
        Some(rgb_06(224, 199, 28))
    );
    assert_eq!(
        textures.terrain_particle_tint_color_for_block_state(lily_pad),
        Some(rgb_06(0x20, 0x80, 0x30))
    );
    assert_eq!(
        textures.falling_dust_block_tint_color_for_block_state(lily_pad),
        Some(rgb(0x20, 0x80, 0x30))
    );
}

#[test]
fn particle_tint_catalog_samples_biome_profiles() {
    let textures = TerrainTextureState::with_biome_colors_for_tests(BiomeColorCatalog::new([
        BiomeColorProfile {
            id: 42,
            name: "minecraft:test_biome".to_string(),
            temperature: 0.5,
            temperature_modifier: bbb_client_assets::BiomeTemperatureModifier::None,
            downfall: 0.5,
            has_precipitation: true,
            grass_color: Some([9, 18, 27]),
            foliage_color: Some([36, 45, 54]),
            dry_foliage_color: Some([63, 72, 81]),
            water_color: Some([90, 99, 108]),
            grass_color_modifier: GrassColorModifier::None,
        },
    ]));
    let catalog = textures.particle_tint_catalog();
    let short_grass = block_state_id("minecraft:short_grass", []);
    let water = block_state_id("minecraft:water", [("level", "0")]);
    let position = Some(BlockRenderPosition { x: 4, y: 64, z: 8 });

    assert_eq!(
        catalog.terrain_particle_tint_color_for_block_state(short_grass, Some(42), position, None,),
        Some(rgb_06(9, 18, 27))
    );
    assert_eq!(
        textures.terrain_particle_tint_color_for_block_state_at(short_grass, Some(42), position),
        Some(rgb_06(9, 18, 27))
    );
    assert_eq!(
        catalog.falling_dust_block_tint_color_for_block_state(water, Some(42), position, None,),
        Some(rgb(90, 99, 108))
    );
}

#[test]
fn terrain_particle_biome_blend_center_uses_lower_half_for_double_tall_grass() {
    let upper = block_state_id("minecraft:tall_grass", [("half", "upper")]);
    let lower = block_state_id("minecraft:tall_grass", [("half", "lower")]);
    let short = block_state_id("minecraft:short_grass", []);
    let stone = block_state_id("minecraft:stone", []);
    let position = BlockRenderPosition { x: 4, y: 64, z: 8 };

    assert_eq!(
        terrain_particle_biome_blend_center(upper, position),
        Some(BlockRenderPosition { y: 63, ..position })
    );
    assert_eq!(
        terrain_particle_biome_blend_center(lower, position),
        Some(position)
    );
    assert_eq!(
        terrain_particle_biome_blend_center(short, position),
        Some(position)
    );
    assert_eq!(terrain_particle_biome_blend_center(stone, position), None);
}

// Vanilla `ClientLevel.calculateBlockTint` averages a biome `ColorResolver`
// over the `biomeBlendRadius` (default 2 -> 5x5) window. These tests exercise
// that averaging: no-op on a uniform window, exact arithmetic mean across a
// biome boundary, per-sample application of the swamp grass modifier, and
// honest truncation when the window overhangs unloaded chunks.

fn blend_biome_profile(id: i32, water: [u8; 3], modifier: GrassColorModifier) -> BiomeColorProfile {
    BiomeColorProfile {
        id,
        name: format!("minecraft:blend_biome_{id}"),
        temperature: 0.5,
        temperature_modifier: bbb_client_assets::BiomeTemperatureModifier::None,
        downfall: 0.5,
        has_precipitation: true,
        grass_color: Some([100, 100, 100]),
        foliage_color: Some([40, 50, 60]),
        dry_foliage_color: Some([70, 80, 90]),
        water_color: Some(water),
        grass_color_modifier: modifier,
    }
}

/// Builds a blend window centred on `center`; `fill(dcol, drow)` returns the
/// biome id for the column offset `(dcol, drow)` from the centre (or `None` for
/// an unavailable / unloaded column), matching `BiomeBlend::samples` geometry.
fn blend_window(
    center: BlockRenderPosition,
    mut fill: impl FnMut(i32, i32) -> Option<i32>,
) -> BiomeBlend {
    use crate::scene::biome_tint::{BIOME_BLEND_DIAMETER, BIOME_BLEND_RADIUS, BIOME_BLEND_SAMPLES};
    let mut samples = [None; BIOME_BLEND_SAMPLES];
    for row in 0..BIOME_BLEND_DIAMETER {
        for col in 0..BIOME_BLEND_DIAMETER {
            samples[(row * BIOME_BLEND_DIAMETER + col) as usize] =
                fill(col - BIOME_BLEND_RADIUS, row - BIOME_BLEND_RADIUS);
        }
    }
    BiomeBlend::new(center, samples)
}

#[test]
fn biome_blend_uniform_window_matches_single_biome() {
    let textures = TerrainTextureState {
        biome_colors: Some(BiomeColorCatalog::new([blend_biome_profile(
            1,
            [70, 80, 90],
            GrassColorModifier::None,
        )])),
        ..TerrainTextureState::default()
    };
    let center = BlockRenderPosition { x: 8, y: 64, z: 8 };
    let blend = blend_window(center, |_, _| Some(1));

    // A window that is entirely one biome must reproduce the single-biome
    // colour exactly (no regression versus `biomeBlendRadius == 0`).
    let single = textures.block_tint(
        "minecraft:water",
        bbb_render_types::TerrainMaterialClass::Fluid,
        None,
        Some(1),
        Some(center),
    );
    let blended = textures.block_tint_blended(
        "minecraft:water",
        bbb_render_types::TerrainMaterialClass::Fluid,
        None,
        Some(1),
        Some(center),
        Some(&blend),
    );
    assert_eq!(blended, single);
    assert_eq!(blended, TerrainTint::from_rgb_u8(70, 80, 90));
}

#[test]
fn biome_blend_averages_two_biomes_across_boundary() {
    let textures = TerrainTextureState {
        biome_colors: Some(BiomeColorCatalog::new([
            blend_biome_profile(1, [10, 20, 30], GrassColorModifier::None),
            blend_biome_profile(2, [20, 40, 60], GrassColorModifier::None),
        ])),
        ..TerrainTextureState::default()
    };
    let center = BlockRenderPosition { x: 0, y: 64, z: 0 };
    // A vertical boundary: the two left columns (2 * 5 = 10 samples) are biome
    // 1, the three right columns (15 samples) are biome 2.
    let blend = blend_window(center, |dcol, _| Some(if dcol < 0 { 1 } else { 2 }));

    let tint = textures.block_tint_blended(
        "minecraft:water",
        bbb_render_types::TerrainMaterialClass::Fluid,
        None,
        Some(1),
        Some(center),
        Some(&blend),
    );
    // R = (10*10 + 15*20) / 25 = 16, G = (10*20 + 15*40) / 25 = 32,
    // B = (10*30 + 15*60) / 25 = 48 (integer truncation, matching vanilla).
    assert_eq!(tint, TerrainTint::from_rgb_u8(16, 32, 48));
    assert_ne!(tint, TerrainTint::from_rgb_u8(10, 20, 30));
    assert_ne!(tint, TerrainTint::from_rgb_u8(20, 40, 60));
}

#[test]
fn biome_blend_truncates_window_to_available_samples() {
    let textures = TerrainTextureState {
        biome_colors: Some(BiomeColorCatalog::new([
            blend_biome_profile(1, [10, 20, 30], GrassColorModifier::None),
            blend_biome_profile(2, [20, 40, 60], GrassColorModifier::None),
        ])),
        ..TerrainTextureState::default()
    };
    let center = BlockRenderPosition { x: 0, y: 64, z: 0 };
    // Only the centre row is loaded (5 samples): one biome 1 plus four biome 2;
    // the other 20 columns overhang unloaded chunks and are dropped.
    let blend = blend_window(center, |dcol, drow| {
        if drow == 0 {
            Some(if dcol == -2 { 1 } else { 2 })
        } else {
            None
        }
    });
    let tint = textures.block_tint_blended(
        "minecraft:water",
        bbb_render_types::TerrainMaterialClass::Fluid,
        None,
        Some(1),
        Some(center),
        Some(&blend),
    );
    // Divided by the available count (5), not the full 25:
    // R = (10 + 4*20) / 5 = 18, G = (20 + 4*40) / 5 = 36, B = (30 + 4*60) / 5 = 54.
    assert_eq!(tint, TerrainTint::from_rgb_u8(18, 36, 54));

    // A fully-unavailable window degrades to the single centre biome.
    let empty = blend_window(center, |_, _| None);
    let tint_empty = textures.block_tint_blended(
        "minecraft:water",
        bbb_render_types::TerrainMaterialClass::Fluid,
        None,
        Some(2),
        Some(center),
        Some(&empty),
    );
    assert_eq!(tint_empty, TerrainTint::from_rgb_u8(20, 40, 60));
}

#[test]
fn biome_blend_applies_swamp_modifier_per_sample() {
    use crate::scene::biome_tint::BIOME_BLEND_RADIUS;
    let textures = TerrainTextureState {
        biome_colors: Some(BiomeColorCatalog::new([blend_biome_profile(
            3,
            [0, 0, 0],
            GrassColorModifier::Swamp,
        )])),
        ..TerrainTextureState::default()
    };
    // The swamp modifier ignores the base colour and picks a dark/light constant
    // from `biome_info_noise(x, z)`, so the resolved colour is purely a function
    // of each sample's own position.
    let base = [100u8, 100, 100];
    let window_positions = |center: BlockRenderPosition| {
        let mut positions = Vec::new();
        for drow in -BIOME_BLEND_RADIUS..=BIOME_BLEND_RADIUS {
            for dcol in -BIOME_BLEND_RADIUS..=BIOME_BLEND_RADIUS {
                positions.push(BlockRenderPosition {
                    x: center.x + dcol,
                    y: center.y,
                    z: center.z + drow,
                });
            }
        }
        positions
    };

    // Find a centre whose 5x5 footprint straddles the swamp noise boundary so
    // the window mixes dark and light samples (the region around the vanilla
    // `biome_info_noise` dark sample at x=-496, z=-512).
    let mut chosen = None;
    'scan: for cz in -560..-440 {
        for cx in -560..-440 {
            let center = BlockRenderPosition {
                x: cx,
                y: 64,
                z: cz,
            };
            let colors: Vec<[u8; 3]> = window_positions(center)
                .into_iter()
                .map(|pos| apply_grass_color_modifier(GrassColorModifier::Swamp, base, Some(pos)))
                .collect();
            if colors.iter().any(|color| *color != colors[0]) {
                chosen = Some(center);
                break 'scan;
            }
        }
    }
    let center = chosen.expect("expected a swamp noise boundary inside the scan range");

    // Expected = per-sample modifier, then integer arithmetic mean.
    let positions = window_positions(center);
    let mut sum = [0u32; 3];
    for pos in &positions {
        let color = apply_grass_color_modifier(GrassColorModifier::Swamp, base, Some(*pos));
        sum[0] += u32::from(color[0]);
        sum[1] += u32::from(color[1]);
        sum[2] += u32::from(color[2]);
    }
    let count = positions.len() as u32;
    let expected = TerrainTint::from_rgb_u8(
        (sum[0] / count) as u8,
        (sum[1] / count) as u8,
        (sum[2] / count) as u8,
    );
    // The wrong "average first, apply modifier once at the centre" collapses to a
    // single dark/light constant, which a mixed window cannot equal.
    let center_only = apply_grass_color_modifier(GrassColorModifier::Swamp, base, Some(center));
    let center_only = TerrainTint::from_rgb_u8(center_only[0], center_only[1], center_only[2]);

    let blend = blend_window(center, |_, _| Some(3));
    let blended = textures.block_tint_blended(
        "minecraft:grass_block",
        bbb_render_types::TerrainMaterialClass::Opaque,
        Some(0),
        Some(3),
        Some(center),
        Some(&blend),
    );
    assert_eq!(blended, expected);
    assert_ne!(
        blended, center_only,
        "swamp grass modifier must run per sample, before averaging",
    );
}

fn properties<const N: usize>(entries: [(&str, &str); N]) -> BTreeMap<String, String> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn block_state_id<const N: usize>(name: &str, props: [(&str, &str); N]) -> i32 {
    let properties = properties(props);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        name,
        &properties,
    )
    .unwrap_or_else(|| panic!("missing vanilla block state {name} {properties:?}"))
}

fn rgb(r: u8, g: u8, b: u8) -> [f32; 4] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
}

fn rgb_06(r: u8, g: u8, b: u8) -> [f32; 4] {
    [
        r as f32 / 255.0 * 0.6,
        g as f32 / 255.0 * 0.6,
        b as f32 / 255.0 * 0.6,
        1.0,
    ]
}

fn sprite(id: &str) -> bbb_client_assets::SpriteImage {
    bbb_client_assets::SpriteImage::new(id, 1, 1, vec![255, 255, 255, 255]).unwrap()
}

fn write_json(path: &std::path::Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("bbb-client-{name}-{}-{nonce}", std::process::id()))
}

fn flat_colormap(rgb: [u8; 3]) -> bbb_client_assets::ColorMapImage {
    bbb_client_assets::ColorMapImage::new(
        2,
        2,
        [rgb, rgb, rgb, rgb]
            .into_iter()
            .flat_map(|[r, g, b]| [r, g, b, 255])
            .collect(),
    )
    .unwrap()
}

fn coordinate_colormap() -> bbb_client_assets::ColorMapImage {
    let mut rgba = Vec::new();
    for y in 0u8..4 {
        for x in 0u8..4 {
            rgba.extend([x * 10, y * 20, x + y, 255]);
        }
    }
    bbb_client_assets::ColorMapImage::new(4, 4, rgba).unwrap()
}

fn block_model_box_with_face_texture(
    face: bbb_client_assets::BlockModelFace,
    texture: &str,
    tint_index: Option<i32>,
) -> bbb_client_assets::BlockModelBox {
    let mut face_present = [false; 6];
    let mut face_textures: [Option<String>; 6] = std::array::from_fn(|_| None);
    let mut face_tint_indices = [None; 6];
    face_present[face.index()] = true;
    face_textures[face.index()] = Some(texture.to_string());
    face_tint_indices[face.index()] = tint_index;
    bbb_client_assets::BlockModelBox {
        from: [0, 0, 0],
        to: [16, 16, 16],
        face_present,
        face_uvs: [[0, 0, 16, 16]; 6],
        face_uv_rotations: [0; 6],
        face_shade: [true; 6],
        face_light_emission: [0; 6],
        face_cull: [None; 6],
        face_tint_indices,
        face_textures,
        face_force_translucent: [false; 6],
    }
}

#[test]
fn item_frame_border_bakes_the_five_template_elements() {
    // Vanilla `block/template_item_frame`: a back panel (north + south faces) plus four wood bars —
    // the bottom and top bars show all six faces, the left and right bars omit their up/down faces.
    // 2 + 6 + 6 + 4 + 4 = 22 quads. The wood bars sit at z=15..16, while the
    // back panel preserves vanilla's fractional `from.z = 15.5`.
    let textures = TerrainTextureState::default();
    let quads = textures.item_frame_border_quads(false, false);
    let cached = textures.item_frame_border_quads(false, false);
    assert!(std::sync::Arc::ptr_eq(&quads, &cached));
    assert_eq!(quads.len(), 22);
    let back_panel = quads
        .iter()
        .filter(|quad| quad_x_bounds(quad) == (3.0, 13.0) && quad_y_bounds(quad) == (3.0, 13.0))
        .collect::<Vec<_>>();
    assert_eq!(back_panel.len(), 2);
    assert_eq!(
        quad_z_bounds(back_panel[0])
            .0
            .min(quad_z_bounds(back_panel[1]).0),
        15.5
    );
    assert_eq!(
        quad_z_bounds(back_panel[0])
            .1
            .max(quad_z_bounds(back_panel[1]).1),
        16.0
    );
    // The bars span the full 2..=14 frame footprint in X and Y.
    let min_x = quads
        .iter()
        .flat_map(|quad| quad.corners.iter().map(|corner| corner[0]))
        .fold(f32::INFINITY, f32::min);
    let max_x = quads
        .iter()
        .flat_map(|quad| quad.corners.iter().map(|corner| corner[0]))
        .fold(f32::NEG_INFINITY, f32::max);
    assert_eq!(min_x, 2.0);
    assert_eq!(max_x, 14.0);
}

#[test]
fn item_frame_map_border_uses_the_full_template_footprint() {
    // Vanilla `block/template_item_frame_map`: the same five elements as the normal frame, but the bars
    // span the full 0..=16 footprint and the back panel spans 1..=15 for the full-frame map render.
    let textures = TerrainTextureState::default();
    let quads = textures.item_frame_border_quads(false, true);
    assert_eq!(quads.len(), 22);
    let min_x = quads
        .iter()
        .flat_map(|quad| quad.corners.iter().map(|corner| corner[0]))
        .fold(f32::INFINITY, f32::min);
    let max_x = quads
        .iter()
        .flat_map(|quad| quad.corners.iter().map(|corner| corner[0]))
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = quads
        .iter()
        .flat_map(|quad| quad.corners.iter().map(|corner| corner[1]))
        .fold(f32::INFINITY, f32::min);
    let max_y = quads
        .iter()
        .flat_map(|quad| quad.corners.iter().map(|corner| corner[1]))
        .fold(f32::NEG_INFINITY, f32::max);
    assert_eq!((min_x, max_x), (0.0, 16.0));
    assert_eq!((min_y, max_y), (0.0, 16.0));
    let min_z = quads
        .iter()
        .flat_map(|quad| quad.corners.iter().map(|corner| corner[2]))
        .fold(f32::INFINITY, f32::min);
    let max_z = quads
        .iter()
        .flat_map(|quad| quad.corners.iter().map(|corner| corner[2]))
        .fold(f32::NEG_INFINITY, f32::max);
    assert!((min_z - 15.001).abs() < 1e-6);
    assert_eq!(max_z, 16.0);
}

fn quad_x_bounds(quad: &crate::ItemModelQuad) -> (f32, f32) {
    quad.corners
        .iter()
        .map(|corner| corner[0])
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), x| {
            (min.min(x), max.max(x))
        })
}

fn quad_y_bounds(quad: &crate::ItemModelQuad) -> (f32, f32) {
    quad.corners
        .iter()
        .map(|corner| corner[1])
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), y| {
            (min.min(y), max.max(y))
        })
}

fn quad_z_bounds(quad: &crate::ItemModelQuad) -> (f32, f32) {
    quad.corners
        .iter()
        .map(|corner| corner[2])
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), z| {
            (min.min(z), max.max(z))
        })
}

#[test]
fn renderer_skip_rendering_preserves_world_flags() {
    let skip = bbb_render_types::TerrainSkipRendering {
        same_block_key: 42,
        same_block_culls_all_faces: true,
        same_block_culls_vertical_faces: true,
        leaves_block: true,
        iron_bars_block: true,
        bars_tag: true,
        north: true,
        south: false,
        west: true,
        east: false,
        connection_property_mask: 0b1111,
    };

    assert_eq!(
        super::state_render::renderer_skip_rendering(skip, false),
        crate::terrain::TerrainSkipRendering {
            same_block_key: 42,
            same_block_culls_all_faces: true,
            same_block_culls_vertical_faces: true,
            culls_with_leaves: true,
            iron_bars_block: true,
            bars_tag: true,
            north: true,
            south: false,
            west: true,
            east: false,
            connection_property_mask: 0b1111,
        }
    );
    assert!(!super::state_render::renderer_skip_rendering(skip, true).culls_with_leaves);
}

#[test]
fn sprite_alpha_bitpacks_words_across_boundaries_and_skips_uniform_opaque_sprites() {
    let mut rgba = vec![0_u8; 64 * 2 * 4];
    for y in 0..2_usize {
        for x in 0..64_usize {
            let pixel = (y * 64 + x) * 4;
            rgba[pixel] = 200;
            rgba[pixel + 1] = 100;
            rgba[pixel + 2] = 50;
            rgba[pixel + 3] = if y == 0 && (30..34).contains(&x) {
                0
            } else if y == 1 && x >= 48 {
                128
            } else {
                255
            };
        }
    }
    let wide =
        bbb_client_assets::SpriteImage::new("minecraft:block/wide_crop", 64, 2, rgba).unwrap();
    let opaque = sprite("minecraft:block/stone");
    let images = vec![wide, opaque];
    let atlas = bbb_client_assets::AtlasPacker::new(128, 1)
        .unwrap()
        .stitch(&images)
        .unwrap();
    let textures =
        TerrainTextureState::from_layout_and_images(&atlas.layout, &images, None, None, None);

    assert!(
        !textures
            .core
            .sprite_alphas
            .contains_key("minecraft:block/stone"),
        "uniform opaque sprites should not allocate per-texel alpha bitsets"
    );
    let wide_alpha = textures
        .core
        .sprite_alphas
        .get("minecraft:block/wide_crop")
        .expect("mixed-alpha sprite retains bitpacked alpha words");
    assert_eq!(wide_alpha.words.len(), 4);

    let crop_flags = |uv: [u8; 4]| {
        let north = bbb_client_assets::BlockModelFace::North.index();
        let mut model_box = block_model_box_with_face_texture(
            bbb_client_assets::BlockModelFace::North,
            "minecraft:block/wide_crop",
            None,
        );
        model_box.face_uvs[north] = uv;
        let shape = textures.terrain_render_shape_for_block(
            "minecraft:test_block",
            &BTreeMap::new(),
            bbb_render_types::TerrainMaterialClass::Opaque,
            BlockModelShape::Box(model_box),
            [0; 6],
            [TerrainTint::WHITE; 6],
            [TerrainTransparency::OPAQUE; 6],
            &|_| TerrainTint::WHITE,
        );
        let TerrainRenderShape::Box {
            face_transparency, ..
        } = shape
        else {
            panic!("expected crop box");
        };
        face_transparency[north]
    };

    assert_eq!(crop_flags([0, 0, 7, 16]), TerrainTransparency::OPAQUE);
    assert_eq!(
        crop_flags([7, 0, 9, 8]),
        TerrainTransparency {
            has_transparent: true,
            has_translucent: false,
        }
    );
    assert_eq!(
        crop_flags([12, 8, 16, 16]),
        TerrainTransparency {
            has_transparent: false,
            has_translucent: true,
        }
    );
}

#[test]
fn terrain_tint_rule_matches_vanilla_block_colors_for_redstone_stems_lily_pad_cauldron_and_petals()
{
    let unpowered =
        TerrainTintRule::for_block_state("minecraft:redstone_wire", &properties([("power", "0")]));
    let powered =
        TerrainTintRule::for_block_state("minecraft:redstone_wire", &properties([("power", "15")]));
    assert_eq!(
        unpowered.source,
        TerrainTintSource::Fixed(redstone_wire_tint(0))
    );
    assert_eq!(
        powered.source,
        TerrainTintSource::Fixed(redstone_wire_tint(15))
    );
    assert_ne!(unpowered.source, powered.source);

    let lily_pad = TerrainTintRule::for_block("minecraft:lily_pad");
    assert_eq!(
        lily_pad.source,
        TerrainTintSource::Fixed(TerrainTint::from_rgb_u8(0x20, 0x80, 0x30))
    );

    let stem =
        TerrainTintRule::for_block_state("minecraft:melon_stem", &properties([("age", "7")]));
    assert_eq!(stem.source, TerrainTintSource::Fixed(stem_tint(7)));

    let attached = TerrainTintRule::for_block("minecraft:attached_pumpkin_stem");
    assert_eq!(
        attached.source,
        TerrainTintSource::Fixed(TerrainTint::from_rgb_u8(0xe0, 0xc7, 0x1c))
    );

    let cauldron = TerrainTintRule::for_block("minecraft:water_cauldron");
    assert_eq!(cauldron.source, TerrainTintSource::Water);
    assert!(
        !cauldron.tints(None),
        "cauldron body faces must stay untinted"
    );
    assert!(
        cauldron.tints(Some(0)),
        "cauldron water surface face takes water tint"
    );

    let water = TerrainTintRule::for_block("minecraft:water");
    assert!(water.tints(None));

    for block_name in ["minecraft:pink_petals", "minecraft:wildflowers"] {
        let rule = TerrainTintRule::for_block(block_name);
        assert_eq!(rule.source, TerrainTintSource::Grass);
        assert!(
            !rule.tints(Some(0)),
            "{block_name} petal layer 0 is untinted"
        );
        assert!(
            rule.tints(Some(1)),
            "{block_name} stem layer 1 takes grass tint"
        );
    }
}

#[test]
fn terrain_block_table_interns_shared_weighted_models_and_caches_translucent_material() {
    let temp = unique_temp_dir("r22-01-terrain-dedup");
    let asset_root = temp
        .join("sources")
        .join(bbb_client_assets::MC_VERSION)
        .join("assets")
        .join("minecraft");
    let bs_dir = asset_root.join("blockstates");
    let models_dir = asset_root.join("models/block");
    std::fs::create_dir_all(&bs_dir).unwrap();
    std::fs::create_dir_all(&models_dir).unwrap();
    std::fs::write(
        models_dir.join("shared_box.json"),
        r##"{
            "textures": { "all": "minecraft:block/stone" },
            "elements": [
                { "from": [0, 0, 0], "to": [16, 16, 16], "faces": {
                    "down": { "texture": "#all", "tintindex": 0 },
                    "up": { "texture": "#all", "tintindex": 0 },
                    "north": { "texture": "#all", "tintindex": 0 },
                    "south": { "texture": "#all", "tintindex": 0 },
                    "west": { "texture": "#all", "tintindex": 0 },
                    "east": { "texture": "#all", "tintindex": 0 }
                } }
            ]
        }"##,
    )
    .unwrap();
    let variants: Vec<serde_json::Value> = (0..16)
        .map(|_| serde_json::json!({ "model": "minecraft:block/shared_box", "weight": 1 }))
        .collect();
    std::fs::write(
        bs_dir.join("redstone_wire.json"),
        serde_json::json!({ "variants": { "": variants } }).to_string(),
    )
    .unwrap();
    std::fs::write(
        bs_dir.join("stone.json"),
        serde_json::json!({ "variants": { "": { "model": "minecraft:block/shared_box" } } })
            .to_string(),
    )
    .unwrap();

    let roots = bbb_client_assets::PackRoots::from_root(&temp).unwrap();
    let block_models = bbb_client_assets::BlockModelCatalog::load(&roots).unwrap();
    let stone_sprite = sprite("minecraft:block/stone");
    let atlas = bbb_client_assets::AtlasPacker::new(64, 1)
        .unwrap()
        .stitch(std::slice::from_ref(&stone_sprite))
        .unwrap();
    let textures = TerrainTextureState::from_layout_and_images(
        &atlas.layout,
        &[stone_sprite],
        Some(block_models),
        None,
        None,
    );
    let source = TerrainResourceSource::new(
        &textures,
        bbb_render_types::TerrainBlockRenderContext::new([], |_| None),
    );
    let parts = source.build_table();
    // 1,296 redstone_wire states x 16 weighted variants all collapse to the shared model instead
    // of retaining 20,736 duplicate `TerrainStateModel` entries.
    assert!(
        parts.models.len() < 128,
        "expected deduplicated model table, got {} models",
        parts.models.len()
    );

    // Translucent material queries populate the per-(block, properties) cache cleanly.
    let props = BTreeMap::new();
    assert!(!textures.block_model_has_translucent_material(
        "minecraft:stone",
        &props,
        bbb_render_types::TerrainMaterialClass::Opaque,
    ));
    assert_eq!(textures.block_translucent_material_cache.borrow().len(), 1);
    assert!(!textures.block_model_has_translucent_material(
        "minecraft:stone",
        &props,
        bbb_render_types::TerrainMaterialClass::Opaque,
    ));

    let _ = std::fs::remove_dir_all(&temp);
}
