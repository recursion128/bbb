use bbb_protocol::{
    block_state_types::vanilla_block_default_state_id,
    block_types::{
        vanilla_block_type_for_resource_id, VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_BAMBOO_SAPLING_ID,
        VANILLA_BLOCK_CAVE_AIR_ID, VANILLA_BLOCK_COBWEB_ID, VANILLA_BLOCK_VOID_AIR_ID,
    },
    heightmap_types::{
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
        VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID, VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID,
    },
    NbtSummaryEntry, NbtSummaryValue,
};

use crate::dimension_type::builtin_dimension_type_shape;
use bbb_server_types::fixture::FlatFixture;
use bbb_server_types::save::Java26LevelStem;

use super::{
    block_state_blocks_motion, block_state_is_air, FlatLevelGeneratorSettings, FlatLevelSource,
    FlatStructureOverrides, DEFAULT_FLAT_BIOME, FLAT_GENERATOR_ID,
};

/// The overworld shape every test binds settings to
/// (`data/minecraft/dimension_type/overworld.json`).
const MIN_Y: i32 = -64;
const HEIGHT: i32 = 384;

const PATH: &str = "world_gen_settings.data.dimensions.minecraft:overworld.generator";

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn layer(block: &str, height: i32) -> NbtSummaryValue {
    compound(vec![
        entry("block", NbtSummaryValue::string(block)),
        entry("height", NbtSummaryValue::Int(height)),
    ])
}

fn strings(values: &[&str]) -> NbtSummaryValue {
    NbtSummaryValue::List(
        values
            .iter()
            .map(|value| NbtSummaryValue::string(*value))
            .collect(),
    )
}

/// The exact settings compound `data/minecraft/worldgen/world_preset/flat.json`
/// declares, in the NBT shape the writer stores it as.
fn preset_settings() -> NbtSummaryValue {
    compound(vec![
        entry("biome", NbtSummaryValue::string("minecraft:plains")),
        entry("features", NbtSummaryValue::Byte(0)),
        entry("lakes", NbtSummaryValue::Byte(0)),
        entry(
            "layers",
            NbtSummaryValue::List(vec![
                layer("minecraft:bedrock", 1),
                layer("minecraft:dirt", 2),
                layer("minecraft:grass_block", 1),
            ]),
        ),
        entry(
            "structure_overrides",
            strings(&["minecraft:strongholds", "minecraft:villages"]),
        ),
    ])
}

fn settings_from(layers: Vec<NbtSummaryValue>) -> FlatLevelGeneratorSettings {
    FlatLevelGeneratorSettings::parse(
        &compound(vec![entry("layers", NbtSummaryValue::List(layers))]),
        PATH,
    )
    .unwrap()
}

fn source_from(layers: Vec<NbtSummaryValue>) -> FlatLevelSource {
    FlatLevelSource::new(settings_from(layers), MIN_Y, HEIGHT).unwrap()
}

fn default_state(block: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(block).unwrap().id).unwrap()
}

#[test]
fn official_flat_preset_settings_parse_into_every_codec_field() {
    let settings = FlatLevelGeneratorSettings::parse(&preset_settings(), PATH).unwrap();

    assert_eq!(settings.biome(), "minecraft:plains");
    assert!(!settings.add_lakes());
    assert!(!settings.decoration());
    assert_eq!(
        settings.structure_overrides(),
        Some(&FlatStructureOverrides::Sets(vec![
            "minecraft:strongholds".to_string(),
            "minecraft:villages".to_string(),
        ]))
    );

    let layers_info = settings.layers_info();
    assert_eq!(layers_info.len(), 3);
    assert_eq!(layers_info[0].block(), "minecraft:bedrock");
    assert_eq!(layers_info[0].height(), 1);
    assert_eq!(layers_info[1].block(), "minecraft:dirt");
    assert_eq!(layers_info[1].height(), 2);
    assert_eq!(layers_info[2].block(), "minecraft:grass_block");
    assert_eq!(layers_info[2].height(), 1);

    // `updateLayers` repeats each layer's default state `height` times, so a
    // block with properties still contributes one id and never a placeholder.
    assert_eq!(
        settings.layers(),
        [
            default_state("minecraft:bedrock"),
            default_state("minecraft:dirt"),
            default_state("minecraft:dirt"),
            default_state("minecraft:grass_block"),
        ]
    );
    assert!(!settings.is_void_gen());
}

#[test]
fn absent_optional_fields_take_the_codec_defaults() {
    let settings = settings_from(vec![layer("minecraft:stone", 1)]);

    assert_eq!(settings.biome(), DEFAULT_FLAT_BIOME);
    assert!(!settings.add_lakes());
    assert!(!settings.decoration());
    // An absent `structure_overrides` is every structure set, not none.
    assert_eq!(settings.structure_overrides(), None);
}

#[test]
fn declared_optional_fields_override_the_defaults() {
    let settings = FlatLevelGeneratorSettings::parse(
        &compound(vec![
            entry("layers", NbtSummaryValue::List(vec![])),
            entry("lakes", NbtSummaryValue::Byte(1)),
            entry("features", NbtSummaryValue::Byte(1)),
            entry("biome", NbtSummaryValue::string("minecraft:desert")),
            entry(
                "structure_overrides",
                NbtSummaryValue::string("#minecraft:some_tag"),
            ),
        ]),
        PATH,
    )
    .unwrap();

    assert!(settings.add_lakes());
    assert!(settings.decoration());
    assert_eq!(settings.biome(), "minecraft:desert");
    assert_eq!(
        settings.structure_overrides(),
        Some(&FlatStructureOverrides::Tag(
            "#minecraft:some_tag".to_string()
        ))
    );
}

#[test]
fn a_bare_structure_override_id_is_the_one_element_list_form() {
    let settings = FlatLevelGeneratorSettings::parse(
        &compound(vec![
            entry("layers", NbtSummaryValue::List(vec![])),
            entry(
                "structure_overrides",
                NbtSummaryValue::string("minecraft:villages"),
            ),
        ]),
        PATH,
    )
    .unwrap();
    assert_eq!(
        settings.structure_overrides(),
        Some(&FlatStructureOverrides::Sets(vec![
            "minecraft:villages".to_string()
        ]))
    );
}

#[test]
fn empty_layers_expand_to_nothing_and_report_void_generation() {
    let settings = settings_from(Vec::new());
    assert!(settings.layers_info().is_empty());
    assert!(settings.layers().is_empty());
    assert!(settings.is_void_gen());

    let source = FlatLevelSource::new(settings, MIN_Y, HEIGHT).unwrap();
    let chunk = source.generate_chunk(0, 0, 0);
    let air = default_state("minecraft:air");
    for y in [MIN_Y, 0, MIN_Y + HEIGHT - 1] {
        assert_eq!(chunk.column().get_block_state(0, y, 0), Some(air));
    }
    // A zero-filled heightmap reads back as `minY`.
    assert_eq!(chunk.world_surface_wg(), MIN_Y);
    assert_eq!(chunk.ocean_floor_wg(), MIN_Y);
    assert_eq!(
        source.base_height(VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID),
        Some(MIN_Y)
    );
    assert_eq!(source.spawn_height(), MIN_Y);
}

#[test]
fn a_zero_height_layer_contributes_no_block_and_keeps_void_generation() {
    let settings = settings_from(vec![layer("minecraft:stone", 0)]);
    assert_eq!(settings.layers_info().len(), 1);
    assert!(settings.layers().is_empty());
    assert!(settings.is_void_gen());
}

#[test]
fn an_all_air_stack_is_void_generation_but_cave_air_is_not() {
    assert!(settings_from(vec![layer("minecraft:air", 4)]).is_void_gen());
    // `voidGen` tests `s.is(Blocks.AIR)`, which is block identity.
    assert!(!settings_from(vec![layer("minecraft:cave_air", 4)]).is_void_gen());
}

#[test]
fn layer_block_ids_resolve_or_fall_back_to_air() {
    let settings = settings_from(vec![layer("stone", 1)]);
    assert_eq!(settings.layers(), [default_state("minecraft:stone")]);
    for block in ["example:unobtainium", "minecraft:not_a_block"] {
        let settings = settings_from(vec![layer(block, 2)]);
        assert_eq!(settings.layers_info()[0].block(), "minecraft:air");
        assert_eq!(settings.layers(), [default_state("minecraft:air"); 2]);
    }

    // A layer with no `block` field at all is the same default.
    let settings = settings_from(vec![compound(vec![entry(
        "height",
        NbtSummaryValue::Int(1),
    )])]);
    assert_eq!(settings.layers_info()[0].block(), "minecraft:air");
}

#[test]
fn a_level_stem_is_read_through_its_generator_dispatch_key() {
    let raw_generator = compound(vec![
        entry("type", NbtSummaryValue::string(FLAT_GENERATOR_ID)),
        entry("settings", preset_settings()),
    ]);
    let stem = Java26LevelStem {
        dimension_type: "minecraft:overworld".to_string(),
        generator: FLAT_GENERATOR_ID.to_string(),
        raw_generator: raw_generator.clone(),
    };
    let settings = FlatLevelGeneratorSettings::from_level_stem(&stem, PATH).unwrap();
    assert_eq!(settings.layers().len(), 4);
}

#[test]
fn every_chunk_position_produces_the_identical_column() {
    let source = source_from(vec![
        layer("minecraft:bedrock", 1),
        layer("minecraft:dirt", 2),
        layer("minecraft:grass_block", 1),
    ]);
    let origin = source.generate_chunk(0, 0, 7);

    for (chunk_x, chunk_z) in [(0, 0), (1, 0), (0, -1), (31, -47), (i32::MIN, i32::MAX)] {
        let chunk = source.generate_chunk(chunk_x, chunk_z, 7);
        assert_eq!(chunk.chunk_x(), chunk_x);
        assert_eq!(chunk.chunk_z(), chunk_z);
        assert_eq!(chunk.column(), origin.column());
        assert_eq!(
            chunk.column().encode_section_data(),
            origin.column().encode_section_data()
        );
        assert_eq!(chunk.world_surface_wg(), origin.world_surface_wg());
        assert_eq!(chunk.ocean_floor_wg(), origin.ocean_floor_wg());
    }
}

#[test]
fn the_column_stacks_the_layers_from_min_y_and_is_air_above_them() {
    let source = source_from(vec![
        layer("minecraft:bedrock", 1),
        layer("minecraft:dirt", 2),
        layer("minecraft:grass_block", 1),
    ]);
    let chunk = source.generate_chunk(3, -5, 0);
    let column = chunk.column();

    for (x, z) in [(0, 0), (15, 15), (7, 9)] {
        assert_eq!(
            column.get_block_state(x, MIN_Y, z),
            Some(default_state("minecraft:bedrock"))
        );
        assert_eq!(
            column.get_block_state(x, MIN_Y + 1, z),
            Some(default_state("minecraft:dirt"))
        );
        assert_eq!(
            column.get_block_state(x, MIN_Y + 2, z),
            Some(default_state("minecraft:dirt"))
        );
        assert_eq!(
            column.get_block_state(x, MIN_Y + 3, z),
            Some(default_state("minecraft:grass_block"))
        );
        assert_eq!(
            column.get_block_state(x, MIN_Y + 4, z),
            Some(default_state("minecraft:air"))
        );
    }
    assert_eq!(column.min_y(), MIN_Y);
    assert_eq!(column.section_count(), (HEIGHT / 16) as usize);
    assert_eq!(column.get_block_state(0, MIN_Y - 1, 0), None);
    assert_eq!(column.get_block_state(0, MIN_Y + HEIGHT, 0), None);
}

#[test]
fn the_two_worldgen_heightmaps_follow_their_own_predicates() {
    // A torch is not air and does not block motion, so it moves one heightmap
    // and not the other.
    let source = source_from(vec![
        layer("minecraft:stone", 2),
        layer("minecraft:torch", 1),
    ]);
    let chunk = source.generate_chunk(0, 0, 0);

    assert_eq!(chunk.world_surface_wg(), MIN_Y + 3);
    assert_eq!(chunk.ocean_floor_wg(), MIN_Y + 2);
    assert_eq!(
        chunk.heightmap_first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID),
        Some(MIN_Y + 3)
    );
    assert_eq!(
        chunk.heightmap_first_available(VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID),
        Some(MIN_Y + 2)
    );
    // A flat generator primes no other kind.
    assert_eq!(
        chunk.heightmap_first_available(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID),
        None
    );
}

#[test]
fn an_air_gap_under_a_solid_layer_keeps_the_topmost_opaque_height() {
    let source = source_from(vec![
        layer("minecraft:stone", 1),
        layer("minecraft:air", 3),
        layer("minecraft:stone", 1),
    ]);
    let chunk = source.generate_chunk(0, 0, 0);
    assert_eq!(chunk.world_surface_wg(), MIN_Y + 5);
    assert_eq!(chunk.ocean_floor_wg(), MIN_Y + 5);

    // Air on top leaves both maps at the layer below it.
    let source = source_from(vec![layer("minecraft:stone", 1), layer("minecraft:air", 3)]);
    let chunk = source.generate_chunk(0, 0, 0);
    assert_eq!(chunk.world_surface_wg(), MIN_Y + 1);
    assert_eq!(chunk.ocean_floor_wg(), MIN_Y + 1);
}

#[test]
fn base_height_answers_what_the_generated_heightmaps_hold() {
    for layers in [
        vec![layer("minecraft:stone", 2), layer("minecraft:torch", 1)],
        vec![layer("minecraft:bedrock", 1), layer("minecraft:air", 5)],
        vec![layer("minecraft:cobweb", 3)],
        Vec::new(),
    ] {
        let source = source_from(layers);
        let chunk = source.generate_chunk(0, 0, 0);
        assert_eq!(
            source.base_height(VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID),
            Some(chunk.world_surface_wg())
        );
        assert_eq!(
            source.base_height(VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID),
            Some(chunk.ocean_floor_wg())
        );
    }
}

#[test]
fn base_height_declines_the_kinds_whose_predicate_reads_fluid_state() {
    let source = source_from(vec![layer("minecraft:stone", 1)]);
    for kind in [
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
        -1,
        99,
    ] {
        assert_eq!(source.base_height(kind), None);
    }
}

#[test]
fn layers_taller_than_the_dimension_are_declared_but_not_placed() {
    let source = source_from(vec![layer("minecraft:stone", HEIGHT + 32)]);
    assert_eq!(source.settings().layers().len(), (HEIGHT + 32) as usize);

    let chunk = source.generate_chunk(0, 0, 0);
    assert_eq!(
        chunk.column().get_block_state(0, MIN_Y + HEIGHT - 1, 0),
        Some(default_state("minecraft:stone"))
    );
    assert_eq!(chunk.column().get_block_state(0, MIN_Y + HEIGHT, 0), None);
    // `fillFromNoise` stops at the dimension height, so the heightmap does too.
    assert_eq!(chunk.world_surface_wg(), MIN_Y + HEIGHT);

    // `getSpawnHeight` is `minY + min(height, layers.size())`.
    assert_eq!(source.spawn_height(), MIN_Y + HEIGHT);
}

#[test]
fn the_motion_blocking_predicate_keeps_vanillas_two_named_exceptions() {
    // `blocksMotion` is solidity minus cobweb and bamboo sapling by identity,
    // and both of those are solid enough for the state cache to say so.
    for block_type_id in [VANILLA_BLOCK_COBWEB_ID, VANILLA_BLOCK_BAMBOO_SAPLING_ID] {
        let state = vanilla_block_default_state_id(block_type_id).unwrap();
        assert!(!block_state_blocks_motion(state));
    }
    assert!(block_state_blocks_motion(default_state("minecraft:stone")));
    assert!(!block_state_blocks_motion(default_state("minecraft:air")));
}

#[test]
fn flat_stems_regenerate_the_column_the_runtime_serves_for_each_builtin_shape() {
    let fixture = FlatFixture::new().unwrap();
    // The type and generator are independent `LevelStem` fields (`LevelStem.java:11-15`).
    for (name, dimension_type) in [
        ("minecraft:overworld", "minecraft:overworld"),
        ("minecraft:the_end", "minecraft:the_end"),
        ("minecraft:the_nether", "minecraft:the_nether"),
    ] {
        let shape = builtin_dimension_type_shape(dimension_type).unwrap();
        let settings = compound(vec![
            entry("biome", NbtSummaryValue::string("minecraft:plains")),
            entry("features", NbtSummaryValue::Byte(0)),
            entry("lakes", NbtSummaryValue::Byte(0)),
            entry(
                "layers",
                NbtSummaryValue::List(vec![layer(
                    fixture.floor_block_name(),
                    fixture.floor_y() - shape.min_y + 1,
                )]),
            ),
            entry("structure_overrides", NbtSummaryValue::List(Vec::new())),
        ]);
        let stem = Java26LevelStem {
            dimension_type: dimension_type.to_string(),
            generator: FLAT_GENERATOR_ID.to_string(),
            raw_generator: compound(vec![
                entry("type", NbtSummaryValue::string(FLAT_GENERATOR_ID)),
                entry("settings", settings),
            ]),
        };
        let path = format!("world_gen_settings.data.dimensions.{name}.generator");
        let settings = FlatLevelGeneratorSettings::from_level_stem(&stem, &path).unwrap();
        assert_eq!(settings.layers_info().len(), 1, "{name}");
        assert_eq!(
            settings.layers_info()[0].block(),
            fixture.floor_block_name(),
            "{name}"
        );
        assert_eq!(
            settings.layers().len(),
            (fixture.floor_y() - shape.min_y + 1) as usize,
            "{name}"
        );
        // The created save writes an empty override list, which is *no*
        // structure set, unlike the absent field that would mean all of them.
        assert_eq!(
            settings.structure_overrides(),
            Some(&FlatStructureOverrides::Sets(Vec::new())),
            "{name}"
        );

        let source = FlatLevelSource::from_level_stem(&stem, None, &path).unwrap();
        assert_eq!(
            (source.min_y(), source.height()),
            (shape.min_y, shape.height)
        );
        let chunk = source.generate_chunk(0, 0, fixture.biome_id());
        let air = vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).unwrap();
        let floor = default_state(fixture.floor_block_name());
        for y in shape.min_y..shape.max_y_exclusive() {
            let expected = if y <= fixture.floor_y() { floor } else { air };
            for x in [0, 15] {
                for z in [0, 15] {
                    assert_eq!(
                        chunk.column().get_block_state(x, y, z),
                        Some(expected),
                        "{name} ({x}, {y}, {z})"
                    );
                }
            }
        }
        assert_eq!(chunk.world_surface_wg(), fixture.floor_y() + 1, "{name}");
    }
}

#[test]
fn the_air_predicate_resolves_to_the_three_generated_air_blocks() {
    for block_type_id in [
        VANILLA_BLOCK_AIR_ID,
        VANILLA_BLOCK_CAVE_AIR_ID,
        VANILLA_BLOCK_VOID_AIR_ID,
    ] {
        let state = vanilla_block_default_state_id(block_type_id).expect("a built-in block");
        assert!(block_state_is_air(state));
    }
    for resource_id in [
        "minecraft:stone",
        "minecraft:water",
        "minecraft:short_grass",
    ] {
        let block = vanilla_block_type_for_resource_id(resource_id).expect("a built-in block");
        let state = vanilla_block_default_state_id(block.id).expect("a built-in block");
        assert!(!block_state_is_air(state), "{resource_id} is not air");
    }
}
