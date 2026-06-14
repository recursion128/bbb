use super::*;

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
        sprite("minecraft:block/lava_still"),
        sprite("minecraft:block/lava_flow"),
    ];
    let atlas = bbb_pack::AtlasPacker::new(16, 1)
        .unwrap()
        .stitch(&images)
        .unwrap();
    let textures = TerrainTextureState::from_layout(&atlas.layout, None, None, None);

    let (water, _, _, _, _) = textures.block_render_data(
        Some("minecraft:water"),
        &properties([("level", "0")]),
        bbb_world::TerrainMaterialClass::Fluid,
        None,
        None,
    );
    let water_still = textures.texture_index("minecraft:block/water_still");
    let water_flow = textures.texture_index("minecraft:block/water_flow");
    assert_eq!(water[0], water_still);
    assert_eq!(water[1], water_still);
    assert_eq!(water[2..], [water_flow; 4]);
    let (water_layer, water_tint) = textures.fluid_render_data(TerrainFluidKind::Water, None, None);
    assert_eq!(water_layer, water);
    assert_eq!(water_tint, [TerrainTint::from_rgb_u8(0x3f, 0x76, 0xe4); 6]);

    let (lava, _, _, _, _) = textures.block_render_data(
        Some("minecraft:lava"),
        &properties([("level", "0")]),
        bbb_world::TerrainMaterialClass::Fluid,
        None,
        None,
    );
    let lava_still = textures.texture_index("minecraft:block/lava_still");
    let lava_flow = textures.texture_index("minecraft:block/lava_flow");
    assert_eq!(lava[0], lava_still);
    assert_eq!(lava[1], lava_still);
    assert_eq!(lava[2..], [lava_flow; 4]);
    let (lava_layer, lava_tint) = textures.fluid_render_data(TerrainFluidKind::Lava, None, None);
    assert_eq!(lava_layer, lava);
    assert_eq!(lava_tint, [TerrainTint::WHITE; 6]);
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
fn texture_alpha_maps_to_face_transparency() {
    let cutout =
        bbb_pack::SpriteImage::new("minecraft:block/cutout", 1, 1, vec![1, 2, 3, 0]).unwrap();
    let translucent =
        bbb_pack::SpriteImage::new("minecraft:block/translucent", 1, 1, vec![1, 2, 3, 127])
            .unwrap();
    let atlas = bbb_pack::AtlasPacker::new(8, 1)
        .unwrap()
        .stitch(&[cutout, translucent])
        .unwrap();
    let textures = TerrainTextureState::from_layout(&atlas.layout, None, None, None);
    let mut force_translucent = [false; 6];
    force_translucent[bbb_pack::BlockModelFace::East.index()] = true;
    let face_textures = bbb_pack::BlockFaceTextures {
        textures: std::array::from_fn(|index| {
            if index == bbb_pack::BlockModelFace::North.index() {
                "minecraft:block/cutout".to_string()
            } else {
                "minecraft:block/translucent".to_string()
            }
        }),
        tint_indices: [None; 6],
        force_translucent,
    };

    let transparencies = textures.face_texture_transparencies(&face_textures);

    assert!(transparencies[bbb_pack::BlockModelFace::North.index()].has_transparent);
    assert!(!transparencies[bbb_pack::BlockModelFace::North.index()].has_translucent);
    assert!(!transparencies[bbb_pack::BlockModelFace::Up.index()].has_transparent);
    assert!(transparencies[bbb_pack::BlockModelFace::Up.index()].has_translucent);
    assert!(transparencies[bbb_pack::BlockModelFace::East.index()].has_translucent);
}

#[test]
fn box_face_transparency_uses_model_uv_crop() {
    let image = bbb_pack::SpriteImage::new(
        "minecraft:block/cropped",
        2,
        2,
        vec![10, 0, 0, 255, 20, 0, 0, 0, 30, 0, 0, 255, 40, 0, 0, 255],
    )
    .unwrap();
    let atlas = bbb_pack::AtlasPacker::new(8, 1)
        .unwrap()
        .stitch(std::slice::from_ref(&image))
        .unwrap();
    let textures =
        TerrainTextureState::from_layout_and_images(&atlas.layout, &[image], None, None, None);
    let north = bbb_pack::BlockModelFace::North.index();
    let mut opaque_crop = block_model_box_with_face_texture(
        bbb_pack::BlockModelFace::North,
        "minecraft:block/cropped",
        None,
    );
    opaque_crop.face_uvs[north] = [0, 0, 8, 8];
    let mut transparent_crop = opaque_crop.clone();
    transparent_crop.face_uvs[north] = [8, 0, 16, 8];

    let opaque_shape = textures.terrain_render_shape_for_block(
        "minecraft:test_block",
        &BTreeMap::new(),
        bbb_world::TerrainMaterialClass::Opaque,
        BlockModelShape::Box(opaque_crop),
        [0; 6],
        [TerrainTint::WHITE; 6],
        [TerrainTransparency::OPAQUE; 6],
        None,
        None,
    );
    let transparent_shape = textures.terrain_render_shape_for_block(
        "minecraft:test_block",
        &BTreeMap::new(),
        bbb_world::TerrainMaterialClass::Opaque,
        BlockModelShape::Box(transparent_crop),
        [0; 6],
        [TerrainTint::WHITE; 6],
        [TerrainTransparency::OPAQUE; 6],
        None,
        None,
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
fn fluid_material_overrides_particle_only_model_shape() {
    let textures = TerrainTextureState::default();
    let shape = textures.terrain_render_shape_for_block(
        "minecraft:lava",
        &properties([("level", "8")]),
        bbb_world::TerrainMaterialClass::Fluid,
        BlockModelShape::Custom,
        [0; 6],
        [TerrainTint::WHITE; 6],
        [TerrainTransparency::OPAQUE; 6],
        None,
        None,
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
        bbb_world::TerrainMaterialClass::Opaque,
        BlockModelShape::Custom,
        [0; 6],
        [TerrainTint::WHITE; 6],
        [TerrainTransparency::OPAQUE; 6],
        None,
        None,
    );
    assert_eq!(non_fluid_shape, TerrainRenderShape::Cube);
}

#[test]
fn model_boxes_preserve_per_element_textures_and_tints() {
    let mut texture_state = TerrainTextureState::default();
    texture_state
        .indices
        .insert("minecraft:block/base".to_string(), 1);
    texture_state
        .indices
        .insert("minecraft:block/overlay".to_string(), 2);
    let base = block_model_box_with_face_texture(
        bbb_pack::BlockModelFace::North,
        "minecraft:block/base",
        None,
    );
    let mut overlay = block_model_box_with_face_texture(
        bbb_pack::BlockModelFace::North,
        "minecraft:block/overlay",
        Some(0),
    );
    overlay.face_shade[bbb_pack::BlockModelFace::North.index()] = false;
    overlay.face_force_translucent[bbb_pack::BlockModelFace::North.index()] = true;

    let shape = texture_state.terrain_render_shape_for_block(
        "minecraft:grass_block",
        &BTreeMap::new(),
        bbb_world::TerrainMaterialClass::Opaque,
        BlockModelShape::Boxes(vec![base, overlay]),
        [0; 6],
        [TerrainTint::WHITE; 6],
        [TerrainTransparency::OPAQUE; 6],
        Some(4),
        None,
    );

    let TerrainRenderShape::Boxes(boxes) = shape else {
        panic!("expected boxes render shape");
    };
    let north = bbb_pack::BlockModelFace::North.index();
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
    texture_state
        .indices
        .insert("minecraft:block/base".to_string(), 1);
    texture_state
        .indices
        .insert("minecraft:block/emissive".to_string(), 2);
    let north = bbb_pack::BlockModelFace::North.index();
    let east = bbb_pack::BlockModelFace::East.index();
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
        bbb_world::TerrainMaterialClass::Opaque,
        BlockModelShape::Crosses(vec![
            bbb_pack::BlockModelCross {
                face_textures: base_textures,
                face_tint_indices: [None; 6],
                face_force_translucent: [false; 6],
                shade: false,
                light_emission: 0,
            },
            bbb_pack::BlockModelCross {
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
        Some(4),
        None,
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
    texture_state
        .indices
        .insert("minecraft:block/rotated".to_string(), 7);

    let shape = texture_state.terrain_render_shape_for_block(
        "minecraft:grass_block",
        &BTreeMap::new(),
        bbb_world::TerrainMaterialClass::Opaque,
        BlockModelShape::Quads(vec![bbb_pack::BlockModelQuad {
            face: bbb_pack::BlockModelFace::North,
            corners: [
                [0.0, 0.0, 0.0],
                [16.0, 0.0, 0.0],
                [16.0, 16.0, 0.0],
                [0.0, 16.0, 0.0],
            ],
            normal: [0.0, 0.0, -1.0],
            uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            cull: Some(bbb_pack::BlockModelFace::North),
            tint_index: Some(0),
            texture: Some("minecraft:block/rotated".to_string()),
            force_translucent: true,
            shade: false,
            light_emission: 12,
        }]),
        [0; 6],
        [TerrainTint::WHITE; 6],
        [TerrainTransparency::OPAQUE; 6],
        Some(4),
        None,
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
        .join(bbb_pack::MC_VERSION)
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
    texture_state.block_models = Some(
        bbb_pack::PackRoots::from_root(&root)
            .unwrap()
            .load_block_model_catalog()
            .unwrap(),
    );

    let (_, _, _, _, ambient_occlusion) = texture_state.block_render_data(
        Some("minecraft:test_block"),
        &BTreeMap::new(),
        bbb_world::TerrainMaterialClass::Opaque,
        None,
        None,
    );

    assert!(!ambient_occlusion);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn block_tint_uses_default_vanilla_color_classes() {
    let textures = TerrainTextureState::default();
    assert_eq!(
        textures.block_tint(
            "minecraft:stone",
            bbb_world::TerrainMaterialClass::Opaque,
            None,
            None,
            None
        ),
        TerrainTint::WHITE
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:grass_block",
            bbb_world::TerrainMaterialClass::Opaque,
            Some(0),
            None,
            None
        ),
        TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:oak_leaves",
            bbb_world::TerrainMaterialClass::Cutout,
            Some(0),
            None,
            None
        ),
        TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:spruce_leaves",
            bbb_world::TerrainMaterialClass::Cutout,
            Some(0),
            None,
            None
        ),
        TerrainTint::from_rgb_u8(0x61, 0x99, 0x61)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:birch_leaves",
            bbb_world::TerrainMaterialClass::Cutout,
            Some(0),
            None,
            None
        ),
        TerrainTint::from_rgb_u8(0x80, 0xa7, 0x55)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:leaf_litter",
            bbb_world::TerrainMaterialClass::Cutout,
            Some(0),
            None,
            None
        ),
        TerrainTint::from_rgb_u8(0x5c, 0x3c, 0x32)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:water",
            bbb_world::TerrainMaterialClass::Fluid,
            None,
            None,
            None
        ),
        TerrainTint::from_rgb_u8(0x3f, 0x76, 0xe4)
    );
}

#[test]
fn block_tint_samples_loaded_colormaps() {
    let mut textures = TerrainTextureState::default();
    textures.colormaps = Some(TerrainColorMaps {
        grass: flat_colormap([10, 20, 30]),
        foliage: flat_colormap([40, 50, 60]),
        dry_foliage: Some(flat_colormap([70, 80, 90])),
    });

    assert_eq!(
        textures.block_tint(
            "minecraft:grass_block",
            bbb_world::TerrainMaterialClass::Opaque,
            Some(0),
            Some(4),
            None
        ),
        TerrainTint::from_rgb_u8(10, 20, 30)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:oak_leaves",
            bbb_world::TerrainMaterialClass::Cutout,
            Some(0),
            Some(4),
            None
        ),
        TerrainTint::from_rgb_u8(40, 50, 60)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:leaf_litter",
            bbb_world::TerrainMaterialClass::Cutout,
            Some(0),
            Some(4),
            None
        ),
        TerrainTint::from_rgb_u8(70, 80, 90)
    );
}

#[test]
fn block_tint_uses_loaded_biome_color_profiles() {
    let mut textures = TerrainTextureState::default();
    textures.colormaps = Some(TerrainColorMaps {
        grass: flat_colormap([10, 20, 30]),
        foliage: flat_colormap([40, 50, 60]),
        dry_foliage: Some(flat_colormap([70, 80, 90])),
    });
    textures.biome_colors = Some(BiomeColorCatalog::new([BiomeColorProfile {
        id: 42,
        name: "minecraft:test_biome".to_string(),
        temperature: 0.2,
        downfall: 0.3,
        grass_color: Some([1, 2, 3]),
        foliage_color: Some([4, 5, 6]),
        dry_foliage_color: Some([7, 8, 9]),
        water_color: Some([10, 11, 12]),
        grass_color_modifier: GrassColorModifier::None,
    }]));

    assert_eq!(
        textures.block_tint(
            "minecraft:grass_block",
            bbb_world::TerrainMaterialClass::Opaque,
            Some(0),
            Some(42),
            Some(BlockRenderPosition { x: 0, y: 0, z: 0 })
        ),
        TerrainTint::from_rgb_u8(1, 2, 3)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:oak_leaves",
            bbb_world::TerrainMaterialClass::Cutout,
            Some(0),
            Some(42),
            None
        ),
        TerrainTint::from_rgb_u8(4, 5, 6)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:leaf_litter",
            bbb_world::TerrainMaterialClass::Cutout,
            Some(0),
            Some(42),
            None
        ),
        TerrainTint::from_rgb_u8(7, 8, 9)
    );
    assert_eq!(
        textures.block_tint(
            "minecraft:water",
            bbb_world::TerrainMaterialClass::Fluid,
            None,
            Some(42),
            None
        ),
        TerrainTint::from_rgb_u8(10, 11, 12)
    );
}

#[test]
fn biome_climate_changes_colormap_sample() {
    let mut textures = TerrainTextureState::default();
    textures.colormaps = Some(TerrainColorMaps {
        grass: coordinate_colormap(),
        foliage: flat_colormap([40, 50, 60]),
        dry_foliage: Some(flat_colormap([70, 80, 90])),
    });
    textures.biome_colors = Some(BiomeColorCatalog::new([BiomeColorProfile {
        id: 7,
        name: "minecraft:dry_biome".to_string(),
        temperature: 0.0,
        downfall: 1.0,
        grass_color: None,
        foliage_color: None,
        dry_foliage_color: None,
        water_color: None,
        grass_color_modifier: GrassColorModifier::None,
    }]));

    assert_eq!(
        textures.block_tint(
            "minecraft:grass_block",
            bbb_world::TerrainMaterialClass::Opaque,
            Some(0),
            Some(7),
            None
        ),
        TerrainTint::from_rgb_u8(30, 60, 6)
    );
}

fn properties<const N: usize>(entries: [(&str, &str); N]) -> BTreeMap<String, String> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn sprite(id: &str) -> bbb_pack::SpriteImage {
    bbb_pack::SpriteImage::new(id, 1, 1, vec![255, 255, 255, 255]).unwrap()
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
    std::env::temp_dir().join(format!("bbb-native-{name}-{}-{nonce}", std::process::id()))
}

fn flat_colormap(rgb: [u8; 3]) -> bbb_pack::ColorMapImage {
    bbb_pack::ColorMapImage::new(
        2,
        2,
        [rgb, rgb, rgb, rgb]
            .into_iter()
            .flat_map(|[r, g, b]| [r, g, b, 255])
            .collect(),
    )
    .unwrap()
}

fn coordinate_colormap() -> bbb_pack::ColorMapImage {
    let mut rgba = Vec::new();
    for y in 0u8..4 {
        for x in 0u8..4 {
            rgba.extend([x * 10, y * 20, x + y, 255]);
        }
    }
    bbb_pack::ColorMapImage::new(4, 4, rgba).unwrap()
}

fn block_model_box_with_face_texture(
    face: bbb_pack::BlockModelFace,
    texture: &str,
    tint_index: Option<i32>,
) -> bbb_pack::BlockModelBox {
    let mut face_present = [false; 6];
    let mut face_textures: [Option<String>; 6] = std::array::from_fn(|_| None);
    let mut face_tint_indices = [None; 6];
    face_present[face.index()] = true;
    face_textures[face.index()] = Some(texture.to_string());
    face_tint_indices[face.index()] = tint_index;
    bbb_pack::BlockModelBox {
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
