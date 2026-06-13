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
            face_cull: [true; 6],
        }
    );
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

    let shape = texture_state.terrain_render_shape_for_block(
        "minecraft:grass_block",
        &BTreeMap::new(),
        bbb_world::TerrainMaterialClass::Opaque,
        BlockModelShape::Boxes(vec![base, overlay]),
        [0; 6],
        [TerrainTint::WHITE; 6],
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
    assert_eq!(
        boxes[1].tint[north],
        TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59)
    );
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
            Some(BlockRenderPosition { x: 0, z: 0 })
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
        face_cull: [false; 6],
        face_tint_indices,
        face_textures,
    }
}
