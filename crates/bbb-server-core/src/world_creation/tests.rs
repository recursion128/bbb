use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use bbb_server_datapack::dimension_type::builtin_dimension_type_shape;
use bbb_server_datapack::worldgen_codec::validate_worldgen_element;
use bbb_server_persistence::level_schema::Java26LevelData;
use bbb_server_persistence::world_gen_settings_schema::parse_java_26_1_world_gen_settings;
use bbb_server_persistence::{Java26ChunkContext, Java26ChunkData};
use bbb_server_types::chunk_data::ChunkColumn;
use bbb_server_types::fixture::FlatFixture;
use bbb_server_types::save::{Java26LevelKey, JAVA_26_1_DATA_VERSION};

use super::{
    created_chunk_document, created_level_document, created_world_gen_settings_document,
    Java26WorldCreation, CREATED_WORLD_SEED, END_STEM, FLAT_PRESET_PATH, NETHER_STEM,
    NORMAL_PRESET_PATH, OVERWORLD_STEM,
};

fn level_key(raw: &str) -> Java26LevelKey {
    Java26LevelKey::parse(raw).unwrap()
}

#[test]
fn created_chunk_writes_each_sections_real_biome_palette() {
    let air = bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:air")
        .and_then(|block| bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id))
        .unwrap();
    let mut column = ChunkColumn::new(0, 1, air, 1);
    column.set_noise_biome(1, 0, 0, 2).unwrap();
    column.set_noise_biome(0, 1, 0, 3).unwrap();
    column.set_noise_biome(0, 0, 1, 4).unwrap();
    let names = [
        "minecraft:the_void",
        "minecraft:nether_wastes",
        "minecraft:soul_sand_valley",
        "minecraft:crimson_forest",
        "minecraft:warped_forest",
    ];
    let document = created_chunk_document(
        &column,
        0,
        0,
        &|id| names.get(id as usize).map(|name| (*name).to_string()),
        0,
    )
    .unwrap();
    let parsed = Java26ChunkData::parse(
        document,
        Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: 0,
            section_count: 1,
        },
    )
    .unwrap();
    let biomes = parsed.sections()[0].biomes.as_ref().unwrap();
    assert_eq!(biomes.palette.len(), 4);
    assert_eq!(biomes.bits_per_entry, 2);
    assert_eq!(biomes.value_at(0, 0, 0).map(String::as_str), Some(names[1]));
    assert_eq!(biomes.value_at(1, 0, 0).map(String::as_str), Some(names[2]));
    assert_eq!(biomes.value_at(0, 1, 0).map(String::as_str), Some(names[3]));
    assert_eq!(biomes.value_at(0, 0, 1).map(String::as_str), Some(names[4]));

    let error = created_chunk_document(&column, 0, 0, &|_| None, 0)
        .unwrap_err()
        .to_string();
    assert!(error.contains("absent from the frozen registry"), "{error}");
}

fn request() -> Java26WorldCreation {
    Java26WorldCreation {
        level_name: "bbb-created".to_string(),
        game_type: 1,
        difficulty: "easy".to_string(),
        spawn_position: [0, 65, 0],
        last_played: 1_786_000_000_000,
    }
}

fn compound(value: &NbtSummaryValue) -> &[NbtSummaryEntry] {
    match value {
        NbtSummaryValue::Compound(entries) => entries,
        other => panic!("expected a compound, got {other:?}"),
    }
}

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> &'a NbtSummaryValue {
    &entries
        .iter()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("no {name} field"))
        .value
}

/// The created level has to be a level this server can open again, which is the
/// same reader an official save goes through.
#[test]
fn the_created_level_parses_as_the_java_26_1_level_it_claims_to_be() {
    let request = request();
    let level = Java26LevelData::parse(created_level_document(&request).unwrap()).unwrap();

    assert_eq!(level.level_name(), &request.level_name);
    assert_eq!(level.game_type(), request.game_type);
    assert_eq!(level.spawn().dimension(), OVERWORLD_STEM);
    assert_eq!(level.spawn().position(), request.spawn_position);
    assert_eq!(level.last_played(), request.last_played);
    assert_eq!(level.difficulty_settings().difficulty(), request.difficulty);
    assert!(!level.difficulty_settings().hardcore());
    assert!(!level.difficulty_settings().locked());
    // `PrimaryLevelData`'s new-world constructor (:72-86): a fresh level starts
    // at tick zero, has been modded by nobody and has no brands yet.
    assert_eq!(level.game_time(), 0);
    assert!(!level.was_modded());
    assert!(level.server_brands().is_empty());
    // The dedicated path never writes one (`Main.createNewWorldData`), and a
    // save that named a single player would be claiming an owner it has not
    // got.
    assert_eq!(level.singleplayer_uuid(), None);
    assert!(!level.allow_commands());
    assert!(level.initialized());
}

/// A world this server serves is a built-in-only world, and the loader refuses
/// anything else. The created save has to satisfy its own reader's guard rather
/// than merely parse.
#[test]
fn the_created_level_declares_the_builtin_only_inventory_its_loader_requires() {
    let level = Java26LevelData::parse(created_level_document(&request()).unwrap()).unwrap();
    let packs = level.data_packs().expect("a created level names its packs");
    assert_eq!(
        packs.enabled(),
        bbb_server_types::save::BUILTIN_ONLY_ENABLED_PACKS
    );
    assert_eq!(
        packs.disabled(),
        bbb_server_types::save::BUILTIN_ONLY_DISABLED_PACKS
    );

    assert_eq!(
        level.enabled_features().unwrap_or_default(),
        bbb_server_types::save::BUILTIN_ONLY_ENABLED_FEATURES
    );
}

/// The created world gen settings have to parse through the same reader an
/// official save's do, and describe the three built-in dimensions.
#[test]
fn the_created_world_gen_settings_parse_as_the_document_they_claim_to_be() {
    let fixture = FlatFixture::new().unwrap();
    let document = created_world_gen_settings_document(&fixture).unwrap();
    let settings = parse_java_26_1_world_gen_settings(document, 0).unwrap();

    assert_eq!(settings.seed, CREATED_WORLD_SEED);
    // This server generates neither, so the save says neither.
    assert!(!settings.generate_structures);
    assert!(!settings.generate_bonus_chest);
    assert_eq!(
        settings.dimensions.keys().cloned().collect::<Vec<_>>(),
        [
            level_key("minecraft:overworld"),
            level_key("minecraft:the_nether"),
            level_key("minecraft:the_end"),
        ],
    );
    assert_eq!(
        settings.dimensions[&level_key(OVERWORLD_STEM)].generator,
        "minecraft:flat"
    );
    assert_eq!(
        settings.dimensions[&level_key(NETHER_STEM)].generator,
        "minecraft:noise"
    );
    assert_eq!(
        settings.dimensions[&level_key(END_STEM)].generator,
        "minecraft:noise"
    );
}

/// The fixture-owned Overworld runs the exact column this server serves.
/// Nether and End deliberately have no flat settings because their complete
/// built-in noise stems are retained.
#[test]
fn created_overworld_is_flat_and_other_builtin_stems_retain_noise() {
    let fixture = FlatFixture::new().unwrap();
    let document = created_world_gen_settings_document(&fixture).unwrap();
    let settings = parse_java_26_1_world_gen_settings(document, 0).unwrap();

    {
        let name = OVERWORLD_STEM;
        let stem = &settings.dimensions[&level_key(name)];
        assert_eq!(stem.generator, "minecraft:flat", "{name}");
        let generator = compound(&stem.raw_generator);
        let flat = compound(field(generator, "settings"));

        let NbtSummaryValue::List(layers) = field(flat, "layers") else {
            panic!("{name}: the flat settings carry no layer list");
        };
        assert_eq!(layers.len(), 1, "{name}: the fixture is one solid column");
        let layer = compound(&layers[0]);
        assert_eq!(
            field(layer, "block"),
            &NbtSummaryValue::string(fixture.floor_block_name()),
            "{name}",
        );
        // Flat layers stack from the dimension's declared floor.
        let shape = builtin_dimension_type_shape(&stem.dimension_type).unwrap();
        assert_eq!(
            field(layer, "height"),
            &NbtSummaryValue::Int(fixture.floor_y() - shape.min_y + 1),
            "{name}",
        );

        // An absent override list means every structure set, so the created
        // world has to carry an empty one rather than leave the field out.
        assert_eq!(
            field(flat, "structure_overrides"),
            &NbtSummaryValue::List(Vec::new()),
            "{name}",
        );
    }

    assert_eq!(
        settings.dimensions[&level_key(NETHER_STEM)].generator,
        "minecraft:noise"
    );
    assert_eq!(
        settings.dimensions[&level_key(END_STEM)].generator,
        "minecraft:noise"
    );

    let height = |name: &str| {
        let generator = compound(&settings.dimensions[&level_key(name)].raw_generator);
        let flat = compound(field(generator, "settings"));
        let NbtSummaryValue::List(layers) = field(flat, "layers") else {
            unreachable!("checked above")
        };
        field(compound(&layers[0]), "height").clone()
    };
    assert_eq!(height("minecraft:overworld"), NbtSummaryValue::Int(128));
}

/// The stem *set* and every stem's dimension type come from vanilla's preset
/// untouched, and so does everything in the generator this server did not have
/// to change. This is the alignment that keeps the created save from being a
/// second, hand-written copy of what the built-in pack already says.
///
/// Nether and End are selected whole from Vanilla's normal preset. Overworld
/// uses the flat preset generator with only fixture-owned fields changed.
#[test]
fn each_created_stem_comes_from_its_frozen_builtin_source() {
    let snapshot =
        bbb_server_datapack::vanilla_snapshot::VanillaDatapackSnapshot::embedded().unwrap();
    let flat: serde_json::Value =
        serde_json::from_slice(snapshot.read_path(FLAT_PRESET_PATH).unwrap()).unwrap();
    let flat = bbb_protocol::json_value_to_nbt_summary(flat.get("dimensions").unwrap()).unwrap();
    let flat = compound(&flat);
    let normal: serde_json::Value =
        serde_json::from_slice(snapshot.read_path(NORMAL_PRESET_PATH).unwrap()).unwrap();
    let normal =
        bbb_protocol::json_value_to_nbt_summary(normal.get("dimensions").unwrap()).unwrap();
    let normal = compound(&normal);

    let fixture = FlatFixture::new().unwrap();
    let document = created_world_gen_settings_document(&fixture).unwrap();
    let settings = parse_java_26_1_world_gen_settings(document, 0).unwrap();
    assert_eq!(settings.dimensions.len(), flat.len());

    let overworld_settings = {
        let overworld = compound(field(
            compound(
                &flat
                    .iter()
                    .find(|s| s.name == OVERWORLD_STEM)
                    .unwrap()
                    .value,
            ),
            "generator",
        ));
        compound(field(overworld, "settings")).to_vec()
    };

    for stem in flat {
        let created = settings
            .dimensions
            .get(&level_key(&stem.name.to_jdk_string()))
            .unwrap_or_else(|| {
                panic!("the created save dropped {}", stem.name.to_display_string())
            });
        let expected = compound(&stem.value);
        assert_eq!(
            created.dimension_type,
            match field(expected, "type") {
                NbtSummaryValue::String(value) => value.try_to_string().unwrap(),
                other => panic!("preset stem type is {other:?}"),
            },
            "{} keeps the preset dimension type",
            stem.name.to_display_string(),
        );
        if stem.name == NETHER_STEM || stem.name == END_STEM {
            let expected = compound(
                &normal
                    .iter()
                    .find(|entry| entry.name == stem.name)
                    .unwrap_or_else(|| {
                        panic!("normal preset has no {}", stem.name.to_display_string())
                    })
                    .value,
            );
            assert_eq!(
                &created.raw_generator,
                field(expected, "generator"),
                "{} keeps the complete normal-preset generator",
                stem.name.to_display_string(),
            );
            continue;
        }
        let flat = compound(field(compound(&created.raw_generator), "settings"));
        for entry in &overworld_settings {
            if entry.name == "layers" || entry.name == "structure_overrides" {
                continue;
            }
            assert_eq!(
                field(flat, &entry.name.to_jdk_string()),
                &entry.value,
                "{} changed the preset's {}",
                stem.name.to_display_string(),
                entry.name.to_display_string(),
            );
        }
    }
}

/// The dispatch envelope of the generator this server writes has to be one the
/// 26.1 codec accepts, checked by the repository's own worldgen validator
/// rather than by reading the payload and agreeing with it.
#[test]
fn the_created_dimensions_pass_the_worldgen_codec() {
    let fixture = FlatFixture::new().unwrap();
    let document = created_world_gen_settings_document(&fixture).unwrap();
    let settings = parse_java_26_1_world_gen_settings(document, 0).unwrap();

    for (id, stem) in &settings.dimensions {
        let element = serde_json::json!({
            "type": stem.dimension_type,
            "generator": nbt_to_json(&stem.raw_generator),
        });
        validate_worldgen_element(
            "minecraft:dimension",
            id.as_str(),
            &bbb_registry_codec::parse_registry_json(&serde_json::to_vec(&element).unwrap())
                .unwrap(),
        )
        .unwrap_or_else(|error| {
            panic!(
                "{} is not a dimension the codec accepts: {error}",
                id.as_str()
            )
        });
    }
}

/// The validator reads JSON, and the document is NBT. Only the shapes the
/// preset actually uses are converted; anything else is a preset this test
/// would have to be taught about rather than something to guess at.
fn nbt_to_json(value: &NbtSummaryValue) -> serde_json::Value {
    match value {
        NbtSummaryValue::Byte(value) => serde_json::Value::Bool(*value != 0),
        NbtSummaryValue::Int(value) => serde_json::json!(*value),
        NbtSummaryValue::Long(value) => serde_json::json!(*value),
        NbtSummaryValue::String(value) => serde_json::Value::String(value.try_to_string().unwrap()),
        NbtSummaryValue::List(values) => {
            serde_json::Value::Array(values.iter().map(nbt_to_json).collect())
        }
        NbtSummaryValue::Compound(entries) => serde_json::Value::Object(
            entries
                .iter()
                .map(|entry| (entry.name.to_jdk_string(), nbt_to_json(&entry.value)))
                .collect(),
        ),
        other => panic!("the created preset does not use {other:?}"),
    }
}

#[test]
fn the_created_documents_carry_the_java_26_1_data_version() {
    let level = created_level_document(&request()).unwrap();
    let data = compound(&level.entries()[0].value);
    assert_eq!(
        field(data, "DataVersion"),
        &NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)
    );

    let fixture = FlatFixture::new().unwrap();
    let settings = created_world_gen_settings_document(&fixture).unwrap();
    assert_eq!(
        field(settings.entries(), "DataVersion"),
        &NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)
    );
}
